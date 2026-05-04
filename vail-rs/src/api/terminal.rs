use std::{
    collections::HashMap,
    io::{Read, Write},
    path::Path as FsPath,
    time::{Duration, Instant},
};

use axum::{
    extract::{
        ws::{Message, WebSocket},
        Multipart, Path, Query, State, WebSocketUpgrade,
    },
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use encoding_rs::Encoding;
use futures_util::StreamExt;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use tokio::sync::mpsc;

use crate::{
    api::guard,
    application::{
        orion::compat_service,
        terminal::{access_service, audit_service},
    },
    domain::terminal::command_audit::SshCommandAuditSnapshot,
    domain::orion::compat::OrionCompatModule,
    error::{AppError, AppResult},
    ssh_client::{self, HostSshConfig},
};

use super::AppState;

const TERMINAL_CLOSE_FORCE: i32 = 10000;
const TERMINAL_CLOSE_NETWORK: i32 = 10011;
const TOKEN_EXPIRE_MS: i64 = 5 * 60 * 1000;

static SFTP_CONTENT_TOKENS: Lazy<std::sync::Mutex<HashMap<String, SftpContentToken>>> =
    Lazy::new(|| std::sync::Mutex::new(HashMap::new()));
static SFTP_DOWNLOAD_TOKENS: Lazy<std::sync::Mutex<HashMap<String, SftpDownloadToken>>> =
    Lazy::new(|| std::sync::Mutex::new(HashMap::new()));

#[derive(Debug, Serialize, Deserialize)]
pub struct TerminalTheme {
    pub name: String,
    pub dark: bool,
    pub schema: JsonValue,
}

#[derive(Debug)]
enum SshWorkerCommand {
    Connect {
        config: HostSshConfig,
        width: u32,
        height: u32,
        terminal_type: String,
        charset: Option<String>,
    },
    Input(String),
    Resize {
        width: u32,
        height: u32,
    },
    Close,
}

#[derive(Debug)]
enum SshWorkerEvent {
    Connected,
    Output(String),
    Closed(SshCloseNotice),
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct SshCloseNotice {
    code: i32,
    msg: String,
    retryable: bool,
    reason: &'static str,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SshConnectPayload {
    width: Option<u32>,
    height: Option<u32>,
    terminal_type: Option<String>,
    charset: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TerminalWsAuthFrame {
    #[serde(rename = "type")]
    frame_type: String,
    ticket: String,
    session_hint: String,
}


#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TransferMessage {
    operator: String,
    #[serde(rename = "type")]
    transfer_type: Option<String>,
    host_id: Option<i64>,
    path: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TerminalSftpContentQuery {
    token: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TerminalSftpDownloadQuery {
    channel_id: Option<String>,
    transfer_token: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct SftpFile {
    name: String,
    path: String,
    suffix: String,
    size: i64,
    attr: String,
    is_dir: bool,
    permission: i64,
    uid: i64,
    gid: i64,
    modify_time: i64,
    can_preview: bool,
}

#[derive(Debug, Clone)]
struct SftpContentToken {
    user_id: i64,
    host_id: i64,
    path: String,
    mode: SftpContentMode,
    expires_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum SftpContentMode {
    Read,
    Write,
}

#[derive(Debug, Clone)]
struct SftpDownloadToken {
    user_id: i64,
    host_id: i64,
    path: String,
    expires_at_ms: i64,
}

struct SftpListResult {
    path: String,
    list: Vec<SftpFile>,
}

struct UploadState {
    host_id: i64,
    path: String,
    content: Vec<u8>,
}

#[derive(Debug, Clone)]
struct TerminalAuditContext {
    username: String,
    host_name: String,
    host_address: String,
}

struct TerminalConnectLogParams<'a> {
    pub user_id: i64,
    pub host_id: i64,
    pub context: &'a TerminalAuditContext,
    pub connect_type: &'a str,
    pub session_id: &'a str,
    pub status: &'a str,
    pub start_time: i64,
    pub end_time: i64,
    pub error_message: Option<&'a str>,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/terminal/themes", get(get_terminal_themes))
        .route("/keep-alive/terminal/access/:protocol", get(open_terminal_access_ws))
        .route(
            "/keep-alive/terminal/transfer/:token",
            get(open_terminal_transfer_ws),
        )
        .route(
            "/terminal/terminal-sftp/get-content",
            get(terminal_sftp_get_content),
        )
        .route(
            "/terminal/terminal-sftp/set-content",
            post(terminal_sftp_set_content),
        )
        .route(
            "/terminal/terminal-sftp/download",
            get(terminal_sftp_download),
        )
}

pub async fn get_terminal_themes(
    State(state): State<AppState>,
) -> Result<Json<Vec<TerminalTheme>>, AppError> {
    let themes = fetch_terminal_themes(&state.db).await?;
    Ok(Json(themes))
}

async fn fetch_terminal_themes(db: &PgPool) -> Result<Vec<TerminalTheme>, AppError> {
    let rows = sqlx::query_as::<_, (String, String, Option<String>)>(
        r#"
        SELECT
            dv.label as name,
            dv.value::text as schema_json,
            dv.extra::text as extra_json
        FROM sys_dict_value dv
        JOIN sys_dict_key dk ON dv.key_id = dk.id
        WHERE dk.key_name = 'terminalTheme'
          AND dv.deleted = 0
        ORDER BY dv.sort, dv.id
        "#,
    )
    .fetch_all(db)
    .await
    .map_err(AppError::Database)?;

    let mut themes = Vec::new();
    for (name, schema_json, extra_json) in rows {
        let schema: JsonValue = serde_json::from_str(&schema_json).unwrap_or(JsonValue::Null);
        let extra: JsonValue = extra_json
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or(JsonValue::Null);
        let dark = extra
            .get("dark")
            .and_then(|v: &JsonValue| v.as_bool())
            .unwrap_or(false);

        themes.push(TerminalTheme { name, dark, schema });
    }
    Ok(themes)
}

async fn open_terminal_access_ws(
    State(state): State<AppState>,
    Path(protocol): Path<String>,
    ws: WebSocketUpgrade,
) -> AppResult<impl axum::response::IntoResponse> {
    let protocol = protocol.to_ascii_lowercase();
    if !matches!(protocol.as_str(), "ssh" | "sftp") {
        return Err(AppError::BadRequest(
            "unsupported terminal protocol".to_string(),
        ));
    }

    Ok(ws.on_upgrade(move |socket| async move {
        handle_v2_terminal_access_socket(state, socket, protocol).await;
    }))
}

async fn open_terminal_transfer_ws(
    State(state): State<AppState>,
    Path(token): Path<String>,
    ws: WebSocketUpgrade,
) -> AppResult<impl axum::response::IntoResponse> {
    let token_user_id = parse_transfer_token(&token, &state.config.secrets.data_encryption_key)?;
    Ok(ws.on_upgrade(move |socket| async move {
        handle_transfer_socket(state, socket, token_user_id).await;
    }))
}

async fn terminal_sftp_get_content(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<TerminalSftpContentQuery>,
) -> AppResult<impl IntoResponse> {
    let user_id = guard::current_user_id(&headers, &state.config.jwt)?;
    let token = query
        .token
        .ok_or_else(|| AppError::BadRequest("token is required".to_string()))?;

    let ctx = consume_content_token(&token, user_id, SftpContentMode::Read)?;
    let content = sftp_read_file_content(&state, ctx.host_id, &ctx.path).await?;
    Ok(content)
}

async fn terminal_sftp_set_content(
    State(state): State<AppState>,
    headers: HeaderMap,
    mut multipart: Multipart,
) -> AppResult<impl IntoResponse> {
    let user_id = guard::current_user_id(&headers, &state.config.jwt)?;
    let mut token: Option<String> = None;
    let mut content: Option<Vec<u8>> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::BadRequest(e.to_string()))?
    {
        let name = field.name().unwrap_or("").to_string();
        if name == "token" {
            token = Some(
                field
                    .text()
                    .await
                    .map_err(|e| AppError::BadRequest(e.to_string()))?,
            );
        } else if name == "file" {
            content = Some(
                field
                    .bytes()
                    .await
                    .map_err(|e| AppError::BadRequest(e.to_string()))?
                    .to_vec(),
            );
        }
    }

    let token = token.ok_or_else(|| AppError::BadRequest("token is required".to_string()))?;
    let content = content.ok_or_else(|| AppError::BadRequest("file is required".to_string()))?;
    let ctx = consume_content_token(&token, user_id, SftpContentMode::Write)?;
    sftp_write_file_content(&state, ctx.host_id, &ctx.path, content).await?;
    Ok(Json(
        serde_json::json!({ "code": 200, "msg": "ok", "data": true }),
    ))
}

async fn terminal_sftp_download(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<TerminalSftpDownloadQuery>,
) -> AppResult<impl IntoResponse> {
    let _channel_id = query
        .channel_id
        .ok_or_else(|| AppError::BadRequest("channelId is required".to_string()))?;
    let transfer_token = query
        .transfer_token
        .ok_or_else(|| AppError::BadRequest("transferToken is required".to_string()))?;
    let user_id = guard::current_user_id(&headers, &state.config.jwt)?;
    let ctx = consume_download_token(&transfer_token, user_id)?;
    let data = match sftp_read_file_bytes(&state, ctx.host_id, &ctx.path).await {
        Ok(bytes) => {
            if let Some(operator_type) = sftp_operator_audit_type("download") {
                let context = load_terminal_audit_context(&state, user_id, ctx.host_id).await;
                append_terminal_file_log(
                    &state,
                    user_id,
                    ctx.host_id,
                    &context,
                    operator_type,
                    1,
                    vec![ctx.path.clone()],
                )
                .await;
            }
            bytes
        }
        Err(err) => {
            if let Some(operator_type) = sftp_operator_audit_type("download") {
                let context = load_terminal_audit_context(&state, user_id, ctx.host_id).await;
                append_terminal_file_log(
                    &state,
                    user_id,
                    ctx.host_id,
                    &context,
                    operator_type,
                    0,
                    vec![ctx.path.clone()],
                )
                .await;
            }
            return Err(err);
        }
    };

    let filename = FsPath::new(&ctx.path)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("download.bin")
        .to_string();

    let mut response = (StatusCode::OK, data).into_response();
    let disposition = format!("attachment; filename=\"{}\"", filename.replace('"', "_"));
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/octet-stream"),
    );
    response.headers_mut().insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_str(&disposition)
            .unwrap_or_else(|_| HeaderValue::from_static("attachment")),
    );
    Ok(response)
}

async fn handle_ssh_socket(state: AppState, mut socket: WebSocket, user_id: i64, host_id: i64) {
    let (cmd_tx, cmd_rx) = std::sync::mpsc::channel::<SshWorkerCommand>();
    let (event_tx, mut event_rx) = mpsc::unbounded_channel::<SshWorkerEvent>();
    let timeout_secs = state.config.ssh.connection_timeout;
    let keepalive_interval_secs = state.config.ssh.keepalive_interval;
    let max_consecutive_retryable_read_errors =
        state.config.ssh.max_consecutive_retryable_read_errors;
    let max_consecutive_keepalive_errors = state.config.ssh.max_consecutive_keepalive_errors;
    let network_silence_multiplier = state.config.ssh.network_silence_multiplier;
    let transient_read_error_backoff_ms = state.config.ssh.transient_read_error_backoff_ms;

    std::thread::spawn(move || {
        run_ssh_worker(
            cmd_rx,
            event_tx,
            timeout_secs,
            keepalive_interval_secs,
            max_consecutive_retryable_read_errors,
            max_consecutive_keepalive_errors,
            network_silence_multiplier,
            transient_read_error_backoff_ms,
        )
    });

    let session_id = format!("ssh-{}", uuid::Uuid::new_v4());
    let session_start = now_ms();
    let context = load_terminal_audit_context(&state, user_id, host_id).await;
    let mut connect_log_id: Option<i64> = None;
    let mut close_error: Option<String> = None;
    let mut command_snapshot = SshCommandAuditSnapshot::default();
    if socket
        .send(Message::Text(format!("id|{session_id}")))
        .await
        .is_err()
    {
        return;
    }

    let mut connected = false;
    loop {
        tokio::select! {
            Some(event) = event_rx.recv() => {
                let send_res = match event {
                    SshWorkerEvent::Connected => {
                        connected = true;
                        if connect_log_id.is_none() {
                            connect_log_id = create_terminal_connect_log(
                                &state,
                                TerminalConnectLogParams {
                                    user_id,
                                    host_id,
                                    context: &context,
                                    connect_type: "SSH",
                                    session_id: &session_id,
                                    status: "CONNECTING",
                                    start_time: session_start,
                                    end_time: 0,
                                    error_message: None,
                                },
                            )
                            .await;
                        }
                        socket.send(Message::Text("co".to_string())).await
                    }
                    SshWorkerEvent::Output(body) => {
                        socket.send(Message::Text(format!("o|{body}"))).await
                    }
                    SshWorkerEvent::Closed(close) => {
                        if close.code != 0 {
                            close_error = Some(close.msg.clone());
                        }
                        let primary = socket
                            .send(Message::Text(format!(
                                "cl|{}|{}",
                                close.code,
                                safe_field(&close.msg)
                            )))
                            .await;
                        if primary.is_ok() && close.code != 0 {
                            // Optional structured metadata for newer clients.
                            // Legacy clients can safely ignore this message type.
                            let payload = serde_json::to_string(&close).unwrap_or_else(|_| {
                                format!(
                                    "{{\"code\":{},\"retryable\":{},\"reason\":\"{}\"}}",
                                    close.code, close.retryable, close.reason
                                )
                            });
                            let _ = socket.send(Message::Text(format!("clmeta|{payload}"))).await;
                        }
                        primary
                    }
                };
                if send_res.is_err() {
                    break;
                }
            }
            msg = socket.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        if text == "p" {
                            if socket.send(Message::Text("p".to_string())).await.is_err() {
                                break;
                            }
                            continue;
                        }

                        if text == "cl" {
                            let _ = cmd_tx.send(SshWorkerCommand::Close);
                            break;
                        }

                        if let Some(body) = text.strip_prefix("co|") {
                            if connected {
                                continue;
                            }
                            let payload: SshConnectPayload = serde_json::from_str(body)
                                .unwrap_or(SshConnectPayload { width: None, height: None, terminal_type: None, charset: None });

                            // 获取主机配置中的编码
                            let mut charset = payload.charset;
                            if charset.is_none() {
                                let cache_key = format!("orion:host-config:host:{host_id}:type:SSH");
                                if let Ok(Some(raw)) = sqlx::query_scalar::<_, String>(
                                    "SELECT cache_value FROM cache
                                     WHERE cache_key = $1
                                       AND (expire_time IS NULL OR expire_time > NOW())",
                                )
                                .bind(cache_key)
                                .fetch_optional(&state.db)
                                .await
                                {
                                    if let Ok(config) = serde_json::from_str::<serde_json::Value>(&raw) {
                                        charset = config.get("charset").and_then(|v| v.as_str()).map(|v| v.to_string());
                                    }
                                }
                            }

                            let cfg = match ssh_client::resolve_host_ssh_config(
                                &state.db,
                                &state.config.secrets.data_encryption_key,
                                Some(user_id),
                                host_id,
                            ).await {
                                Ok(v) => v,
                                Err(err) => {
                                    let _ = socket.send(Message::Text(format!("cl|{TERMINAL_CLOSE_FORCE}|{}", safe_field(&err.to_string())))).await;
                                    break;
                                }
                            };
                            let width = payload.width.unwrap_or(120).max(1);
                            let height = payload.height.unwrap_or(40).max(1);
                            let terminal_type = payload.terminal_type.unwrap_or_else(|| "xterm".to_string());
                            let _ = cmd_tx.send(SshWorkerCommand::Connect { config: cfg, width, height, terminal_type, charset });
                            continue;
                        }

                        if let Some(command) = text.strip_prefix("i|") {
                            command_snapshot.ingest(command);
                            let _ = cmd_tx.send(SshWorkerCommand::Input(command.to_string()));
                            continue;
                        }

                        if let Some(body) = text.strip_prefix("rs|") {
                            let mut parts = body.split('|');
                            let width = parts.next().and_then(|v| v.parse::<u32>().ok()).unwrap_or(120).max(1);
                            let height = parts.next().and_then(|v| v.parse::<u32>().ok()).unwrap_or(40).max(1);
                            let _ = cmd_tx.send(SshWorkerCommand::Resize { width, height });
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Ok(_)) => {}
                    Some(Err(_)) => break,
                }
            }
        }
    }

    let _ = cmd_tx.send(SshWorkerCommand::Close);
    command_snapshot.finish_line();
    audit_service::append_terminal_command_snapshot(
        &state.db,
        audit_service::TerminalCommandSnapshotRecordInput {
            user_id,
            host_id,
            username: &context.username,
            host_name: &context.host_name,
            host_address: &context.host_address,
            session_id: &session_id,
            start_time: session_start,
            end_time: now_ms(),
            snapshot: command_snapshot,
        },
    )
    .await;

    let status = if connected { "COMPLETE" } else { "FAILED" };
    if let Some(log_id) = connect_log_id {
        update_terminal_connect_log(&state, user_id, log_id, status, now_ms()).await;
    } else {
        let _ = create_terminal_connect_log(
            &state,
            TerminalConnectLogParams {
                user_id,
                host_id,
                context: &context,
                connect_type: "SSH",
                session_id: &session_id,
                status,
                start_time: session_start,
                end_time: now_ms(),
                error_message: close_error.as_deref(),
            },
        )
        .await;
    }
    tracing::info!(user_id, host_id, "ssh websocket closed");
}

async fn handle_sftp_socket(state: AppState, mut socket: WebSocket, user_id: i64, host_id: i64) {
    let session_id = format!("sftp-{}", uuid::Uuid::new_v4());
    let session_start = now_ms();
    let context = load_terminal_audit_context(&state, user_id, host_id).await;
    let mut connect_log_id: Option<i64> = None;
    let mut close_error: Option<String> = None;
    if socket
        .send(Message::Text(format!("id|{session_id}")))
        .await
        .is_err()
    {
        return;
    }

    let mut connected = false;
    loop {
        let msg = socket.next().await;
        let Some(msg) = msg else {
            break;
        };

        let text = match msg {
            Ok(Message::Text(text)) => text,
            Ok(Message::Close(_)) => break,
            Ok(_) => continue,
            Err(_) => break,
        };

        if text == "p" {
            if socket.send(Message::Text("p".to_string())).await.is_err() {
                break;
            }
            continue;
        }

        if text == "cl" {
            let _ = socket
                .send(Message::Text("cl|0|会话已结束...".to_string()))
                .await;
            break;
        }

        if text.strip_prefix("co|").is_some() {
            if connected {
                continue;
            }
            let verify = match ssh_client::resolve_host_ssh_config(
                &state.db,
                &state.config.secrets.data_encryption_key,
                Some(user_id),
                host_id,
            )
            .await
            {
                Ok(cfg) => ssh_client::verify_login(cfg, state.config.ssh.connection_timeout).await,
                Err(err) => Err(err),
            };
            match verify {
                Ok(_) => {
                    connected = true;
                    if connect_log_id.is_none() {
                        connect_log_id = create_terminal_connect_log(
                            &state,
                            TerminalConnectLogParams {
                                user_id,
                                host_id,
                                context: &context,
                                connect_type: "SFTP",
                                session_id: &session_id,
                                status: "CONNECTING",
                                start_time: session_start,
                                end_time: 0,
                                error_message: None,
                            },
                        )
                        .await;
                    }
                    if socket.send(Message::Text("co".to_string())).await.is_err() {
                        break;
                    }
                }
                Err(err) => {
                    close_error = Some(err.to_string());
                    let _ = socket
                        .send(Message::Text(format!(
                            "cl|{TERMINAL_CLOSE_FORCE}|{}",
                            safe_field(&err.to_string())
                        )))
                        .await;
                    break;
                }
            }
            continue;
        }

        if !connected {
            let _ = socket
                .send(Message::Text(
                    "cl|10000|sftp session not connected".to_string(),
                ))
                .await;
            break;
        }

        let send_res = if let Some(body) = text.strip_prefix("ls|") {
            let mut parts = body.splitn(2, '|');
            let show_hidden = parts.next().unwrap_or("0") == "1";
            let path = parts.next().unwrap_or("/");
            match sftp_list(&state, host_id, path, show_hidden).await {
                Ok(result) => {
                    let body =
                        serde_json::to_string(&result.list).unwrap_or_else(|_| "[]".to_string());
                    socket
                        .send(Message::Text(format!(
                            "ls|{}|1||{body}",
                            safe_field(&result.path)
                        )))
                        .await
                }
                Err(err) => {
                    socket
                        .send(Message::Text(format!(
                            "ls|{}|0|{}|[]",
                            safe_field(path),
                            safe_field(&err.to_string())
                        )))
                        .await
                }
            }
        } else if let Some(path) = text.strip_prefix("mk|") {
            match sftp_mkdir(&state, host_id, path).await {
                Ok(_) => {
                    if let Some(operator_type) = sftp_operator_audit_type("mk") {
                        append_terminal_file_log(
                            &state,
                            user_id,
                            host_id,
                            &context,
                            operator_type,
                            1,
                            vec![path.to_string()],
                        )
                        .await;
                    }
                    socket.send(Message::Text("mk|1|".to_string())).await
                }
                Err(err) => {
                    if let Some(operator_type) = sftp_operator_audit_type("mk") {
                        append_terminal_file_log(
                            &state,
                            user_id,
                            host_id,
                            &context,
                            operator_type,
                            0,
                            vec![path.to_string()],
                        )
                        .await;
                    }
                    socket
                        .send(Message::Text(format!(
                            "mk|0|{}",
                            safe_field(&err.to_string())
                        )))
                        .await
                }
            }
        } else if let Some(path) = text.strip_prefix("to|") {
            match sftp_touch(&state, host_id, path).await {
                Ok(_) => {
                    if let Some(operator_type) = sftp_operator_audit_type("to") {
                        append_terminal_file_log(
                            &state,
                            user_id,
                            host_id,
                            &context,
                            operator_type,
                            1,
                            vec![path.to_string()],
                        )
                        .await;
                    }
                    socket.send(Message::Text("to|1|".to_string())).await
                }
                Err(err) => {
                    if let Some(operator_type) = sftp_operator_audit_type("to") {
                        append_terminal_file_log(
                            &state,
                            user_id,
                            host_id,
                            &context,
                            operator_type,
                            0,
                            vec![path.to_string()],
                        )
                        .await;
                    }
                    socket
                        .send(Message::Text(format!(
                            "to|0|{}",
                            safe_field(&err.to_string())
                        )))
                        .await
                }
            }
        } else if let Some(body) = text.strip_prefix("mv|") {
            let mut parts = body.splitn(2, '|');
            let source = parts.next().unwrap_or("");
            let target = parts.next().unwrap_or("");
            match sftp_move(&state, host_id, source, target).await {
                Ok(_) => {
                    append_terminal_file_log(
                        &state,
                        user_id,
                        host_id,
                        &context,
                        "terminal:sftp-move",
                        1,
                        vec![source.to_string(), target.to_string()],
                    )
                    .await;
                    socket.send(Message::Text("mv|1|".to_string())).await
                }
                Err(err) => {
                    append_terminal_file_log(
                        &state,
                        user_id,
                        host_id,
                        &context,
                        "terminal:sftp-move",
                        0,
                        vec![source.to_string(), target.to_string()],
                    )
                    .await;
                    socket
                        .send(Message::Text(format!(
                            "mv|0|{}",
                            safe_field(&err.to_string())
                        )))
                        .await
                }
            }
        } else if let Some(paths) = text.strip_prefix("rm|") {
            let paths = split_path_list(paths);
            match sftp_remove(&state, host_id, &paths).await {
                Ok(_) => {
                    if let Some(operator_type) = sftp_operator_audit_type("rm") {
                        append_terminal_file_log(
                            &state,
                            user_id,
                            host_id,
                            &context,
                            operator_type,
                            1,
                            paths.clone(),
                        )
                        .await;
                    }
                    socket.send(Message::Text("rm|1|".to_string())).await
                }
                Err(err) => {
                    if let Some(operator_type) = sftp_operator_audit_type("rm") {
                        append_terminal_file_log(
                            &state,
                            user_id,
                            host_id,
                            &context,
                            operator_type,
                            0,
                            paths.clone(),
                        )
                        .await;
                    }
                    socket
                        .send(Message::Text(format!(
                            "rm|0|{}",
                            safe_field(&err.to_string())
                        )))
                        .await
                }
            }
        } else if let Some(body) = text.strip_prefix("chm|") {
            let mut parts = body.splitn(2, '|');
            let path = parts.next().unwrap_or("");
            let mod_value = parts
                .next()
                .and_then(|v| v.parse::<u32>().ok())
                .unwrap_or(0);
            match sftp_chmod(&state, host_id, path, mod_value).await {
                Ok(_) => {
                    append_terminal_file_log(
                        &state,
                        user_id,
                        host_id,
                        &context,
                        "terminal:sftp-chmod",
                        1,
                        vec![path.to_string()],
                    )
                    .await;
                    socket.send(Message::Text("chm|1|".to_string())).await
                }
                Err(err) => {
                    append_terminal_file_log(
                        &state,
                        user_id,
                        host_id,
                        &context,
                        "terminal:sftp-chmod",
                        0,
                        vec![path.to_string()],
                    )
                    .await;
                    socket
                        .send(Message::Text(format!(
                            "chm|0|{}",
                            safe_field(&err.to_string())
                        )))
                        .await
                }
            }
        } else if let Some(body) = text.strip_prefix("df|") {
            let mut parts = body.splitn(2, '|');
            let current_path = parts.next().unwrap_or("/");
            let paths = split_path_list(parts.next().unwrap_or(""));
            match sftp_flatten_download(&state, host_id, &paths).await {
                Ok(list) => {
                    if let Some(operator_type) = sftp_operator_audit_type("df") {
                        append_terminal_file_log(
                            &state,
                            user_id,
                            host_id,
                            &context,
                            operator_type,
                            1,
                            paths.clone(),
                        )
                        .await;
                    }
                    let body = serde_json::to_string(&list).unwrap_or_else(|_| "[]".to_string());
                    socket
                        .send(Message::Text(format!(
                            "df|{}|1||{body}",
                            safe_field(current_path)
                        )))
                        .await
                }
                Err(err) => {
                    if let Some(operator_type) = sftp_operator_audit_type("df") {
                        append_terminal_file_log(
                            &state,
                            user_id,
                            host_id,
                            &context,
                            operator_type,
                            0,
                            paths.clone(),
                        )
                        .await;
                    }
                    socket
                        .send(Message::Text(format!(
                            "df|{}|0|{}|[]",
                            safe_field(current_path),
                            safe_field(&err.to_string())
                        )))
                        .await
                }
            }
        } else if let Some(path) = text.strip_prefix("gc|") {
            let token = put_content_token(SftpContentToken {
                user_id,
                host_id,
                path: path.to_string(),
                mode: SftpContentMode::Read,
                expires_at_ms: now_ms() + TOKEN_EXPIRE_MS,
            });
            socket
                .send(Message::Text(format!("gc|1||{}", safe_field(&token))))
                .await
        } else if let Some(path) = text.strip_prefix("sc|") {
            let token = put_content_token(SftpContentToken {
                user_id,
                host_id,
                path: path.to_string(),
                mode: SftpContentMode::Write,
                expires_at_ms: now_ms() + TOKEN_EXPIRE_MS,
            });
            socket
                .send(Message::Text(format!("sc|1||{}", safe_field(&token))))
                .await
        } else {
            socket
                .send(Message::Text(
                    "cl|10000|unsupported sftp protocol".to_string(),
                ))
                .await
        };

        if send_res.is_err() {
            break;
        }
    }

    let status = if connected { "COMPLETE" } else { "FAILED" };
    if let Some(log_id) = connect_log_id {
        update_terminal_connect_log(&state, user_id, log_id, status, now_ms()).await;
    } else {
        let _ = create_terminal_connect_log(
            &state,
            TerminalConnectLogParams {
                user_id,
                host_id,
                context: &context,
                connect_type: "SFTP",
                session_id: &session_id,
                status,
                start_time: session_start,
                end_time: now_ms(),
                error_message: close_error.as_deref(),
            },
        )
        .await;
    }
}

fn run_ssh_worker(
    cmd_rx: std::sync::mpsc::Receiver<SshWorkerCommand>,
    event_tx: mpsc::UnboundedSender<SshWorkerEvent>,
    timeout_secs: u64,
    keepalive_interval_secs: u64,
    max_consecutive_retryable_read_errors: u32,
    max_consecutive_keepalive_errors: u32,
    network_silence_multiplier: u32,
    transient_read_error_backoff_ms: u64,
) {
    let transient_read_error_backoff = Duration::from_millis(transient_read_error_backoff_ms.max(1));

    let mut session: Option<ssh2::Session> = None;
    let mut channel: Option<ssh2::Channel> = None;
    let mut encoding: &'static Encoding = encoding_rs::UTF_8;
    let keepalive_interval = if keepalive_interval_secs > 0 {
        Some(Duration::from_secs(keepalive_interval_secs.max(1)))
    } else {
        None
    };
    let mut last_keepalive = Instant::now();
    let mut last_network_progress = Instant::now();
    let mut consecutive_transient_read_errors: u32 = 0;
    let mut consecutive_transient_write_errors: u32 = 0;
    let mut consecutive_keepalive_errors: u32 = 0;

    loop {
        match cmd_rx.recv_timeout(Duration::from_millis(10)) {
            Ok(cmd) => match cmd {
                SshWorkerCommand::Connect {
                    config,
                    width,
                    height,
                    terminal_type,
                    charset,
                } => {
                    if let Some(label) = charset {
                        if let Some(enc) = Encoding::for_label(label.as_bytes()) {
                            encoding = enc;
                        }
                    }
                    match ssh_client::connect_session_with_keepalive(
                        &config,
                        timeout_secs,
                        keepalive_interval_secs,
                    )
                    .and_then(|sess| {
                        if keepalive_interval_secs > 0 {
                            sess.set_keepalive(true, keepalive_interval_secs as u32);
                        }
                        let mut ch = sess.channel_session().map_err(|e| {
                            AppError::Ssh(format!("open shell channel failed: {e}"))
                        })?;
                        ch.request_pty(&terminal_type, None, Some((width, height, 0, 0)))
                            .map_err(|e| AppError::Ssh(format!("request pty failed: {e}")))?;
                        ch.shell()
                            .map_err(|e| AppError::Ssh(format!("open shell failed: {e}")))?;
                        sess.set_blocking(false);
                        Ok((sess, ch))
                    }) {
                        Ok((sess, ch)) => {
                            session = Some(sess);
                            channel = Some(ch);
                            last_keepalive = Instant::now();
                            last_network_progress = Instant::now();
                            consecutive_transient_read_errors = 0;
                            consecutive_keepalive_errors = 0;
                            let _ = event_tx.send(SshWorkerEvent::Connected);
                        }
                        Err(err) => {
                            let _ = event_tx.send(SshWorkerEvent::Closed(SshCloseNotice {
                                code: TERMINAL_CLOSE_FORCE,
                                msg: err.to_string(),
                                retryable: false,
                                reason: "connect-failed",
                            }));
                            break;
                        }
                    }
                }
                SshWorkerCommand::Input(command) => {
                    if let Some(ch) = channel.as_mut() {
                        if let Err(err) = write_ssh_command_with_retry(
                            ch,
                            command.as_bytes(),
                            max_consecutive_retryable_read_errors,
                            transient_read_error_backoff,
                        ) {
                            let close = classify_ssh_write_error(&err);
                            if should_tolerate_retryable_write_error(
                                &close,
                                &mut consecutive_transient_write_errors,
                                max_consecutive_retryable_read_errors,
                            ) {
                                continue;
                            }
                            let _ = event_tx.send(SshWorkerEvent::Closed(close));
                            break;
                        }
                        consecutive_transient_write_errors = 0;
                        last_network_progress = Instant::now();
                        let _ = ch.flush();
                    }
                }
                SshWorkerCommand::Resize { width, height } => {
                    if let Some(ch) = channel.as_mut() {
                        let _ = ch.request_pty_size(width, height, None, None);
                    }
                }
                SshWorkerCommand::Close => {
                    if let Some(mut ch) = channel.take() {
                        let _ = ch.send_eof();
                        let _ = ch.close();
                    }
                    if let Some(sess) = session.take() {
                        let _ = sess.disconnect(None, "closed", None);
                    }
                    let _ = event_tx.send(SshWorkerEvent::Closed(SshCloseNotice {
                        code: 0,
                        msg: "会话已结束...".to_string(),
                        retryable: false,
                        reason: "closed-by-client",
                    }));
                    break;
                }
            },
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
        }

        if let Some(ch) = channel.as_mut() {
            let mut buf = [0_u8; 8192];
            match ch.read(&mut buf) {
                Ok(read) if read > 0 => {
                    let (out, _, _) = encoding.decode(&buf[..read]);
                    consecutive_transient_read_errors = 0;
                    consecutive_transient_write_errors = 0;
                    consecutive_keepalive_errors = 0;
                    last_network_progress = Instant::now();
                    let _ = event_tx.send(SshWorkerEvent::Output(out.into_owned()));
                }
                Ok(_) => {
                    if ch.eof() {
                        let _ = event_tx.send(SshWorkerEvent::Closed(SshCloseNotice {
                            code: 0,
                            msg: "会话已结束...".to_string(),
                            retryable: false,
                            reason: "eof",
                        }));
                        break;
                    }
                }
                Err(err) => {
                    if err.kind() != std::io::ErrorKind::WouldBlock {
                        let close = classify_ssh_read_error(&err);
                        if close.retryable {
                            consecutive_transient_read_errors += 1;
                            if should_tolerate_retryable_read_error(
                                &close,
                                consecutive_transient_read_errors,
                                max_consecutive_retryable_read_errors,
                            ) {
                                std::thread::sleep(transient_read_error_backoff);
                                continue;
                            }
                        }

                        let _ = event_tx.send(SshWorkerEvent::Closed(close));
                        break;
                    }
                }
            }
        }

        if let (Some(interval), Some(sess)) = (keepalive_interval, session.as_mut()) {
            if last_keepalive.elapsed() >= interval {
                if let Err(err) = sess.keepalive_send() {
                    if err.message().to_ascii_lowercase().contains("would block") {
                        last_keepalive = Instant::now();
                        continue;
                    }
                    let lower = err.message().to_ascii_lowercase();
                    let retryable = !lower.contains("auth")
                        && !lower.contains("permission")
                        && !lower.contains("protocol");
                    if retryable {
                        consecutive_keepalive_errors += 1;
                        if consecutive_keepalive_errors < max_consecutive_keepalive_errors {
                            last_keepalive = Instant::now();
                            continue;
                        }
                    }
                    let _ = event_tx.send(SshWorkerEvent::Closed(SshCloseNotice {
                        code: if retryable {
                            TERMINAL_CLOSE_NETWORK
                        } else {
                            TERMINAL_CLOSE_FORCE
                        },
                        msg: format!("SSH keepalive failed, 网络连接可能已断开: {err}"),
                        retryable,
                        reason: if retryable {
                            "keepalive-failed"
                        } else {
                            "keepalive-non-retryable"
                        },
                    }));
                    break;
                }
                consecutive_keepalive_errors = 0;
                last_keepalive = Instant::now();
                last_network_progress = Instant::now();
            }
        }

        if let Some(interval) = keepalive_interval {
            let silence_limit = interval.saturating_mul(network_silence_multiplier);
            if silence_limit.as_secs() > 0 && last_network_progress.elapsed() >= silence_limit {
                let _ = event_tx.send(SshWorkerEvent::Closed(SshCloseNotice {
                    code: TERMINAL_CLOSE_NETWORK,
                    msg: "网络长时间无响应，SSH 会话已断开，正在等待重连...".to_string(),
                    retryable: true,
                    reason: "network-silence-timeout",
                }));
                break;
            }
        }
    }
}

fn should_tolerate_retryable_read_error(
    close: &SshCloseNotice,
    consecutive_transient_read_errors: u32,
    max_consecutive_retryable_read_errors: u32,
) -> bool {
    close.retryable
        && consecutive_transient_read_errors > 0
        && consecutive_transient_read_errors < max_consecutive_retryable_read_errors
}

fn write_ssh_command_with_retry(
    ch: &mut ssh2::Channel,
    mut pending: &[u8],
    max_retryable_errors: u32,
    backoff: Duration,
) -> std::io::Result<()> {
    let mut retryable_errors: u32 = 0;
    while !pending.is_empty() {
        match ch.write(pending) {
            Ok(0) => {
                retryable_errors += 1;
                if retryable_errors >= max_retryable_errors {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::WriteZero,
                        "ssh write returned zero bytes",
                    ));
                }
                std::thread::sleep(backoff);
            }
            Ok(written) => {
                pending = &pending[written..];
                retryable_errors = 0;
            }
            Err(err) => {
                if err.kind() == std::io::ErrorKind::WouldBlock {
                    retryable_errors += 1;
                    if retryable_errors >= max_retryable_errors {
                        return Err(err);
                    }
                    std::thread::sleep(backoff);
                    continue;
                }
                let close = classify_ssh_write_error(&err);
                if close.retryable {
                    retryable_errors += 1;
                    if retryable_errors >= max_retryable_errors {
                        return Err(err);
                    }
                    std::thread::sleep(backoff);
                    continue;
                }
                return Err(err);
            }
        }
    }

    Ok(())
}

fn should_tolerate_retryable_write_error(
    close: &SshCloseNotice,
    consecutive_transient_write_errors: &mut u32,
    max_consecutive_retryable_write_errors: u32,
) -> bool {
    if close.retryable && *consecutive_transient_write_errors < max_consecutive_retryable_write_errors {
        *consecutive_transient_write_errors += 1;
        return true;
    }
    false
}

async fn handle_transfer_socket(state: AppState, mut socket: WebSocket, user_id: i64) {
    let mut upload: Option<UploadState> = None;

    loop {
        let msg = socket.next().await;
        match msg {
            Some(Ok(Message::Text(text))) => {
                let payload = match serde_json::from_str::<TransferMessage>(&text) {
                    Ok(v) => v,
                    Err(_) => {
                        if send_transfer(
                            &mut socket,
                            serde_json::json!({
                                "type": "error",
                                "success": false,
                                "msg": "invalid transfer payload"
                            }),
                        )
                        .await
                        .is_err()
                        {
                            break;
                        }
                        continue;
                    }
                };

                match payload.operator.as_str() {
                    "start" => {
                        let host_id = match payload.host_id {
                            Some(v) if v > 0 => v,
                            _ => {
                                if send_transfer(
                                    &mut socket,
                                    serde_json::json!({
                                        "type": "error",
                                        "success": false,
                                        "msg": "hostId is required"
                                    }),
                                )
                                .await
                                .is_err()
                                {
                                    break;
                                }
                                continue;
                            }
                        };

                        let allowed = guard::require_host_permission(&state, user_id, host_id)
                            .await
                            .is_ok();
                        if !allowed {
                            if send_transfer(
                                &mut socket,
                                serde_json::json!({
                                    "type": "error",
                                    "success": false,
                                    "msg": "Host access denied"
                                }),
                            )
                            .await
                            .is_err()
                            {
                                break;
                            }
                            continue;
                        }

                        match payload.transfer_type.as_deref() {
                            Some("upload") => {
                                let path = payload
                                    .path
                                    .as_deref()
                                    .map(str::trim)
                                    .filter(|v| !v.is_empty())
                                    .map(ToOwned::to_owned);

                                let path = match path {
                                    Some(v) => v,
                                    None => {
                                        if send_transfer(
                                            &mut socket,
                                            serde_json::json!({
                                                "type": "error",
                                                "success": false,
                                                "msg": "path is required"
                                            }),
                                        )
                                        .await
                                        .is_err()
                                        {
                                            break;
                                        }
                                        continue;
                                    }
                                };

                                upload = Some(UploadState {
                                    host_id,
                                    path,
                                    content: Vec::new(),
                                });

                                if send_transfer(
                                    &mut socket,
                                    serde_json::json!({
                                        "type": "nextPart",
                                        "success": true,
                                        "msg": null
                                    }),
                                )
                                .await
                                .is_err()
                                {
                                    break;
                                }
                            }
                            Some("download") => {
                                let path = payload
                                    .path
                                    .as_deref()
                                    .map(str::trim)
                                    .filter(|v| !v.is_empty())
                                    .map(ToOwned::to_owned);
                                let Some(path) = path else {
                                    if send_transfer(
                                        &mut socket,
                                        serde_json::json!({
                                            "type": "error",
                                            "success": false,
                                            "msg": "path is required"
                                        }),
                                    )
                                    .await
                                    .is_err()
                                    {
                                        break;
                                    }
                                    continue;
                                };

                                let channel_id = format!("dl-{}", uuid::Uuid::new_v4());
                                let transfer_token = put_download_token(SftpDownloadToken {
                                    user_id,
                                    host_id,
                                    path,
                                    expires_at_ms: now_ms() + TOKEN_EXPIRE_MS,
                                });

                                if send_transfer(
                                    &mut socket,
                                    serde_json::json!({
                                        "type": "start",
                                        "success": true,
                                        "channelId": channel_id,
                                        "transferToken": transfer_token
                                    }),
                                )
                                .await
                                .is_err()
                                {
                                    break;
                                }
                                if send_transfer(
                                    &mut socket,
                                    serde_json::json!({
                                        "type": "finish",
                                        "success": true
                                    }),
                                )
                                .await
                                .is_err()
                                {
                                    break;
                                }
                            }
                            _ => {
                                if send_transfer(
                                    &mut socket,
                                    serde_json::json!({
                                        "type": "error",
                                        "success": false,
                                        "msg": "unsupported transfer type"
                                    }),
                                )
                                .await
                                .is_err()
                                {
                                    break;
                                }
                            }
                        }
                    }
                    "finish" => {
                        let current = match upload.take() {
                            Some(v) => v,
                            None => {
                                if send_transfer(
                                    &mut socket,
                                    serde_json::json!({
                                        "type": "error",
                                        "success": false,
                                        "msg": "upload task not started"
                                    }),
                                )
                                .await
                                .is_err()
                                {
                                    break;
                                }
                                continue;
                            }
                        };
                        let upload_path = current.path.clone();

                        let cfg = match ssh_client::resolve_host_ssh_config(
                            &state.db,
                            &state.config.secrets.data_encryption_key,
                            Some(user_id),
                            current.host_id,
                        )
                        .await
                        {
                            Ok(v) => v,
                            Err(err) => {
                                if send_transfer(
                                    &mut socket,
                                    serde_json::json!({
                                        "type": "error",
                                        "success": false,
                                        "msg": err.to_string(),
                                    }),
                                )
                                .await
                                .is_err()
                                {
                                    break;
                                }
                                continue;
                            }
                        };

                        match ssh_client::upload_files(
                            cfg,
                            state.config.ssh.connection_timeout,
                            vec![(upload_path.clone(), current.content)],
                        )
                        .await
                        {
                            Ok(_) => {
                                if let Some(operator_type) = sftp_operator_audit_type("upload") {
                                    let context = load_terminal_audit_context(
                                        &state,
                                        user_id,
                                        current.host_id,
                                    )
                                    .await;
                                    append_terminal_file_log(
                                        &state,
                                        user_id,
                                        current.host_id,
                                        &context,
                                        operator_type,
                                        1,
                                        vec![upload_path.clone()],
                                    )
                                    .await;
                                }
                                if send_transfer(
                                    &mut socket,
                                    serde_json::json!({
                                        "type": "finish",
                                        "success": true
                                    }),
                                )
                                .await
                                .is_err()
                                {
                                    break;
                                }
                            }
                            Err(err) => {
                                if let Some(operator_type) = sftp_operator_audit_type("upload") {
                                    let context = load_terminal_audit_context(
                                        &state,
                                        user_id,
                                        current.host_id,
                                    )
                                    .await;
                                    append_terminal_file_log(
                                        &state,
                                        user_id,
                                        current.host_id,
                                        &context,
                                        operator_type,
                                        0,
                                        vec![upload_path.clone()],
                                    )
                                    .await;
                                }
                                if send_transfer(
                                    &mut socket,
                                    serde_json::json!({
                                        "type": "error",
                                        "success": false,
                                        "msg": err.to_string(),
                                    }),
                                )
                                .await
                                .is_err()
                                {
                                    break;
                                }
                            }
                        }
                    }
                    "abort" => {
                        upload = None;
                        if send_transfer(
                            &mut socket,
                            serde_json::json!({
                                "type": "abort",
                                "success": true
                            }),
                        )
                        .await
                        .is_err()
                        {
                            break;
                        }
                    }
                    _ => {
                        if send_transfer(
                            &mut socket,
                            serde_json::json!({
                                "type": "error",
                                "success": false,
                                "msg": "unsupported transfer operator"
                            }),
                        )
                        .await
                        .is_err()
                        {
                            break;
                        }
                    }
                }
            }
            Some(Ok(Message::Binary(binary))) => {
                if let Some(task) = upload.as_mut() {
                    task.content.extend_from_slice(&binary);
                    if send_transfer(
                        &mut socket,
                        serde_json::json!({
                            "type": "nextPart",
                            "success": true
                        }),
                    )
                    .await
                    .is_err()
                    {
                        break;
                    }
                } else if send_transfer(
                    &mut socket,
                    serde_json::json!({
                        "type": "error",
                        "success": false,
                        "msg": "upload task not started"
                    }),
                )
                .await
                .is_err()
                {
                    break;
                }
            }
            Some(Ok(Message::Close(_))) | None => break,
            Some(Ok(_)) => {}
            Some(Err(_)) => break,
        }
    }
}

async fn handle_v2_terminal_access_socket(state: AppState, mut socket: WebSocket, protocol: String) {
    let auth_text = match tokio::time::timeout(Duration::from_secs(10), socket.next()).await {
        Ok(Some(Ok(Message::Text(text)))) => text,
        _ => {
            let _ = socket
                .send(Message::Text(format!(
                    "cl|{}|{}",
                    TERMINAL_CLOSE_FORCE,
                    safe_field("missing or invalid auth frame")
                )))
                .await;
            return;
        }
    };

    let frame = match serde_json::from_str::<TerminalWsAuthFrame>(&auth_text) {
        Ok(v) if v.frame_type == "auth" => v,
        _ => {
            let _ = socket
                .send(Message::Text(format!(
                    "cl|{}|{}",
                    TERMINAL_CLOSE_FORCE,
                    safe_field("invalid auth frame")
                )))
                .await;
            return;
        }
    };

    let ticket = match access_service::consume_terminal_access_v2_ticket(
        &state.db,
        &frame.ticket,
        &frame.session_hint,
        &state.config.secrets.data_encryption_key,
    )
    .await
    {
        Ok(v) => v,
        Err(err) => {
            let _ = socket
                .send(Message::Text(format!(
                    "cl|{}|{}",
                    TERMINAL_CLOSE_FORCE,
                    safe_field(&err.to_string())
                )))
                .await;
            return;
        }
    };

    if ticket.connect_type != protocol {
        let _ = socket
            .send(Message::Text(format!(
                "cl|{}|{}",
                TERMINAL_CLOSE_FORCE,
                safe_field("protocol mismatch")
            )))
            .await;
        return;
    }

    if guard::require_host_permission(&state, ticket.user_id, ticket.host_id)
        .await
        .is_err()
    {
        let _ = socket
            .send(Message::Text(format!(
                "cl|{}|{}",
                TERMINAL_CLOSE_FORCE,
                safe_field("host permission denied")
            )))
            .await;
        return;
    }

    match protocol.as_str() {
        "ssh" => handle_ssh_socket(state, socket, ticket.user_id, ticket.host_id).await,
        "sftp" => handle_sftp_socket(state, socket, ticket.user_id, ticket.host_id).await,
        _ => {
            let _ = socket
                .send(Message::Text(format!(
                    "cl|{}|{}",
                    TERMINAL_CLOSE_FORCE,
                    safe_field("unsupported terminal protocol")
                )))
                .await;
        }
    }
}

async fn send_transfer(
    socket: &mut WebSocket,
    payload: serde_json::Value,
) -> Result<(), axum::Error> {
    socket.send(Message::Text(payload.to_string())).await
}

async fn load_terminal_audit_context(
    state: &AppState,
    user_id: i64,
    host_id: i64,
) -> TerminalAuditContext {
    let username = sqlx::query_scalar::<_, Option<String>>(
        "SELECT username FROM sys_user WHERE id = $1 AND deleted = 0",
    )
    .bind(user_id)
    .fetch_optional(&state.db)
    .await
    .ok()
    .flatten()
    .flatten()
    .unwrap_or_else(|| format!("user-{user_id}"));

    let host = sqlx::query_as::<_, (Option<String>, Option<String>)>(
        "SELECT name, hostname FROM host WHERE id = $1 AND deleted = 0",
    )
    .bind(host_id)
    .fetch_optional(&state.db)
    .await
    .ok()
    .flatten();

    let (host_name, host_address) = host
        .map(|(name, hostname)| {
            (
                name.unwrap_or_else(|| format!("host-{host_id}")),
                hostname.unwrap_or_default(),
            )
        })
        .unwrap_or_else(|| (format!("host-{host_id}"), String::new()));

    TerminalAuditContext {
        username,
        host_name,
        host_address,
    }
}

async fn create_terminal_connect_log(
    state: &AppState,
    params: TerminalConnectLogParams<'_>,
) -> Option<i64> {
    let payload = serde_json::json!({
        "userId": params.user_id,
        "username": params.context.username,
        "hostId": params.host_id,
        "hostName": params.context.host_name,
        "hostAddress": params.context.host_address,
        "type": params.connect_type,
        "sessionId": params.session_id,
        "status": params.status,
        "startTime": params.start_time,
        "endTime": params.end_time,
        "extra": {
            "traceId": uuid::Uuid::new_v4().to_string(),
            "channel": params.connect_type,
            "channelId": params.session_id,
            "sessionId": params.session_id,
            "address": "",
            "location": "",
            "userAgent": "",
            "errorMessage": params.error_message.unwrap_or("")
        }
    });

    compat_service::create_record(
        &state.db,
        OrionCompatModule::TerminalConnectLog,
        payload,
        &format!("user-{}", params.user_id),
    )
    .await
    .ok()
    .and_then(|v| v.get("id").and_then(serde_json::Value::as_i64))
}

async fn update_terminal_connect_log(
    state: &AppState,
    user_id: i64,
    id: i64,
    status: &str,
    end_time: i64,
) {
    let mut patch = serde_json::Map::new();
    patch.insert("status".to_string(), serde_json::json!(status));
    patch.insert("endTime".to_string(), serde_json::json!(end_time));

    let _ = compat_service::update_record(
        &state.db,
        OrionCompatModule::TerminalConnectLog,
        id,
        serde_json::Value::Object(patch),
        &format!("user-{user_id}"),
    )
    .await;
}

async fn append_terminal_file_log(
    state: &AppState,
    user_id: i64,
    host_id: i64,
    context: &TerminalAuditContext,
    operator_type: &str,
    result: i32,
    paths: Vec<String>,
) {
    let payload = serde_json::json!({
        "userId": user_id,
        "username": context.username,
        "hostId": host_id,
        "hostName": context.host_name,
        "hostAddress": context.host_address,
        "address": "",
        "location": "",
        "userAgent": "",
        "paths": paths,
        "type": operator_type,
        "result": result,
        "startTime": now_ms(),
        "extra": {
            "maxCount": 0
        }
    });

    let _ = compat_service::create_record(
        &state.db,
        OrionCompatModule::TerminalFileLog,
        payload,
        &format!("user-{user_id}"),
    )
    .await;
}

fn parse_transfer_token(token: &str, signing_key: &str) -> AppResult<i64> {
    let parts = token.split(':').collect::<Vec<_>>();
    if parts.len() != 4 || parts[0] != "transfer" {
        return Err(AppError::BadRequest("invalid transfer token".to_string()));
    }
    let unsigned = parts[0..3].join(":");
    ensure_token_signature(&unsigned, parts[3], signing_key)?;
    let user_id = parts[1]
        .parse::<i64>()
        .map_err(|_| AppError::BadRequest("invalid user in transfer token".to_string()))?;
    let issued_at_ms = parts[2]
        .parse::<i64>()
        .map_err(|_| AppError::BadRequest("invalid timestamp in transfer token".to_string()))?;
    ensure_token_freshness_ms(issued_at_ms, "transfer token expired")?;
    Ok(user_id)
}

fn ensure_token_freshness_ms(issued_at_ms: i64, expired_message: &str) -> AppResult<()> {
    let now = now_ms();
    if issued_at_ms > now || now - issued_at_ms > TOKEN_EXPIRE_MS {
        return Err(AppError::BadRequest(expired_message.to_string()));
    }
    Ok(())
}

fn token_signature(payload: &str, signing_key: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(signing_key.as_bytes());
    hasher.update(b":");
    hasher.update(payload.as_bytes());
    URL_SAFE_NO_PAD.encode(hasher.finalize())
}

fn ensure_token_signature(payload: &str, signature: &str, signing_key: &str) -> AppResult<()> {
    let expected = token_signature(payload, signing_key);
    if signature != expected {
        return Err(AppError::BadRequest(
            "invalid terminal token signature".to_string(),
        ));
    }
    Ok(())
}

async fn with_sftp<R, F>(state: &AppState, host_id: i64, action: F) -> AppResult<R>
where
    R: Send + 'static,
    F: FnOnce(&ssh2::Sftp) -> AppResult<R> + Send + 'static,
{
    let cfg = ssh_client::resolve_host_ssh_config(
        &state.db,
        &state.config.secrets.data_encryption_key,
        None,
        host_id,
    )
    .await?;
    let timeout_secs = state.config.ssh.connection_timeout;

    tokio::task::spawn_blocking(move || {
        let session = ssh_client::connect_session(&cfg, timeout_secs)?;
        let sftp = session
            .sftp()
            .map_err(|e| AppError::Sftp(format!("failed to initialize sftp channel: {e}")))?;
        let result = action(&sftp)?;
        let _ = session.disconnect(None, "sftp-op-complete", None);
        Ok(result)
    })
    .await
    .map_err(|e| AppError::Internal(format!("sftp task join error: {e}")))?
}

async fn sftp_list(
    state: &AppState,
    host_id: i64,
    path: &str,
    show_hidden: bool,
) -> AppResult<SftpListResult> {
    let path = normalize_remote_path(path)?;
    with_sftp(state, host_id, move |sftp| {
        let resolved_path = if path == "." {
            sftp.realpath(FsPath::new("."))
                .ok()
                .and_then(|p| p.to_str().map(ToOwned::to_owned))
                .unwrap_or_else(|| "/".to_string())
        } else {
            path.clone()
        };
        let mut list = Vec::new();
        let rows = sftp
            .readdir(FsPath::new(&path))
            .map_err(|e| AppError::Sftp(format!("list directory failed: {e}")))?;
        for (entry_path, stat) in rows {
            let name = entry_path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_string();
            if name.is_empty() || name == "." || name == ".." {
                continue;
            }
            if !show_hidden && name.starts_with('.') {
                continue;
            }
            let full = join_remote_path(&resolved_path, &name);
            list.push(map_sftp_file(full, name, stat));
        }
        list.sort_by(|a, b| {
            if a.is_dir == b.is_dir {
                a.name.to_lowercase().cmp(&b.name.to_lowercase())
            } else if a.is_dir {
                std::cmp::Ordering::Less
            } else {
                std::cmp::Ordering::Greater
            }
        });
        Ok(SftpListResult {
            path: resolved_path,
            list,
        })
    })
    .await
}

async fn sftp_mkdir(state: &AppState, host_id: i64, path: &str) -> AppResult<()> {
    let path = normalize_remote_path(path)?;
    with_sftp(state, host_id, move |sftp| {
        sftp.mkdir(FsPath::new(&path), 0o755)
            .map_err(|e| AppError::Sftp(format!("create directory failed: {e}")))?;
        Ok(())
    })
    .await
}

async fn sftp_touch(state: &AppState, host_id: i64, path: &str) -> AppResult<()> {
    let path = normalize_remote_path(path)?;
    with_sftp(state, host_id, move |sftp| {
        let mut file = sftp
            .create(FsPath::new(&path))
            .map_err(|e| AppError::Sftp(format!("create file failed: {e}")))?;
        file.flush()
            .map_err(|e| AppError::Sftp(format!("flush file failed: {e}")))?;
        Ok(())
    })
    .await
}

async fn sftp_move(state: &AppState, host_id: i64, source: &str, target: &str) -> AppResult<()> {
    let source = normalize_remote_path(source)?;
    let target = normalize_remote_path(target)?;
    with_sftp(state, host_id, move |sftp| {
        sftp.rename(FsPath::new(&source), FsPath::new(&target), None)
            .map_err(|e| AppError::Sftp(format!("move file failed: {e}")))?;
        Ok(())
    })
    .await
}

async fn sftp_remove(state: &AppState, host_id: i64, paths: &[String]) -> AppResult<()> {
    let paths = paths
        .iter()
        .map(|p| normalize_remote_path(p))
        .collect::<AppResult<Vec<_>>>()?;
    with_sftp(state, host_id, move |sftp| {
        for path in paths {
            remove_path_recursive(sftp, FsPath::new(&path))?;
        }
        Ok(())
    })
    .await
}

async fn sftp_chmod(state: &AppState, host_id: i64, path: &str, mod_value: u32) -> AppResult<()> {
    let path = normalize_remote_path(path)?;
    with_sftp(state, host_id, move |sftp| {
        let stat = ssh2::FileStat {
            size: None,
            uid: None,
            gid: None,
            perm: Some(mod_value),
            atime: None,
            mtime: None,
        };
        sftp.setstat(FsPath::new(&path), stat)
            .map_err(|e| AppError::Sftp(format!("chmod failed: {e}")))?;
        Ok(())
    })
    .await
}

async fn sftp_flatten_download(
    state: &AppState,
    host_id: i64,
    paths: &[String],
) -> AppResult<Vec<SftpFile>> {
    let paths = paths
        .iter()
        .map(|p| normalize_remote_path(p))
        .collect::<AppResult<Vec<_>>>()?;
    with_sftp(state, host_id, move |sftp| {
        let mut files = Vec::new();
        for path in paths {
            flatten_path(sftp, FsPath::new(&path), &mut files)?;
        }
        Ok(files)
    })
    .await
}

async fn sftp_read_file_content(state: &AppState, host_id: i64, path: &str) -> AppResult<String> {
    let path = normalize_remote_path(path)?;
    with_sftp(state, host_id, move |sftp| {
        let mut file = sftp
            .open(FsPath::new(&path))
            .map_err(|e| AppError::Sftp(format!("open file failed: {e}")))?;
        let mut buf = Vec::new();
        file.read_to_end(&mut buf)
            .map_err(|e| AppError::Sftp(format!("read file failed: {e}")))?;
        Ok(String::from_utf8_lossy(&buf).to_string())
    })
    .await
}

async fn sftp_write_file_content(
    state: &AppState,
    host_id: i64,
    path: &str,
    content: Vec<u8>,
) -> AppResult<()> {
    let path = normalize_remote_path(path)?;
    with_sftp(state, host_id, move |sftp| {
        let mut file = sftp
            .create(FsPath::new(&path))
            .map_err(|e| AppError::Sftp(format!("open file failed: {e}")))?;
        file.write_all(&content)
            .map_err(|e| AppError::Sftp(format!("write file failed: {e}")))?;
        file.flush()
            .map_err(|e| AppError::Sftp(format!("flush file failed: {e}")))?;
        Ok(())
    })
    .await
}

async fn sftp_read_file_bytes(state: &AppState, host_id: i64, path: &str) -> AppResult<Vec<u8>> {
    let path = normalize_remote_path(path)?;
    with_sftp(state, host_id, move |sftp| {
        let mut file = sftp
            .open(FsPath::new(&path))
            .map_err(|e| AppError::Sftp(format!("open file failed: {e}")))?;
        let mut buf = Vec::new();
        file.read_to_end(&mut buf)
            .map_err(|e| AppError::Sftp(format!("read file failed: {e}")))?;
        Ok(buf)
    })
    .await
}

fn flatten_path(sftp: &ssh2::Sftp, path: &FsPath, out: &mut Vec<SftpFile>) -> AppResult<()> {
    let stat = sftp
        .stat(path)
        .map_err(|e| AppError::Sftp(format!("stat file failed: {e}")))?;
    let perm = stat.perm.unwrap_or(0);
    if is_dir(perm) {
        let entries = sftp
            .readdir(path)
            .map_err(|e| AppError::Sftp(format!("list directory failed: {e}")))?;
        for (entry, _) in entries {
            let name = entry.file_name().and_then(|s| s.to_str()).unwrap_or("");
            if name.is_empty() || name == "." || name == ".." {
                continue;
            }
            flatten_path(sftp, &entry, out)?;
        }
    } else {
        let path_str = path.to_string_lossy().to_string();
        let name = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();
        out.push(map_sftp_file(path_str, name, stat));
    }
    Ok(())
}

fn remove_path_recursive(sftp: &ssh2::Sftp, path: &FsPath) -> AppResult<()> {
    let stat = sftp
        .stat(path)
        .map_err(|e| AppError::Sftp(format!("stat file failed: {e}")))?;
    let perm = stat.perm.unwrap_or(0);
    if is_dir(perm) {
        let entries = sftp
            .readdir(path)
            .map_err(|e| AppError::Sftp(format!("list directory failed: {e}")))?;
        for (entry, _) in entries {
            let name = entry.file_name().and_then(|s| s.to_str()).unwrap_or("");
            if name.is_empty() || name == "." || name == ".." {
                continue;
            }
            remove_path_recursive(sftp, &entry)?;
        }
        sftp.rmdir(path)
            .map_err(|e| AppError::Sftp(format!("remove directory failed: {e}")))?;
    } else {
        sftp.unlink(path)
            .map_err(|e| AppError::Sftp(format!("remove file failed: {e}")))?;
    }
    Ok(())
}

fn map_sftp_file(path: String, name: String, stat: ssh2::FileStat) -> SftpFile {
    let perm = stat.perm.unwrap_or(0);
    let attr = file_attr(perm);
    let suffix = file_suffix(&name);
    let size = stat.size.unwrap_or(0) as i64;
    let mtime = stat.mtime.unwrap_or(0) as i64 * 1000;
    SftpFile {
        name,
        path,
        suffix: suffix.clone(),
        size,
        attr,
        is_dir: is_dir(perm),
        permission: (perm & 0o7777) as i64,
        uid: stat.uid.unwrap_or(0) as i64,
        gid: stat.gid.unwrap_or(0) as i64,
        modify_time: mtime,
        can_preview: can_preview_file(&suffix, size),
    }
}

fn file_attr(perm: u32) -> String {
    let ftype = perm & 0o170000;
    let prefix = match ftype {
        0o040000 => 'd',
        0o120000 => 'l',
        0o010000 => 'p',
        0o060000 => 'b',
        0o020000 => 'c',
        _ => '-',
    };
    format!("{}{}", prefix, permission_to_string(perm & 0o777))
}

fn permission_to_string(perm: u32) -> String {
    let mut out = String::with_capacity(9);
    let masks = [
        0o400, 0o200, 0o100, 0o040, 0o020, 0o010, 0o004, 0o002, 0o001,
    ];
    for (idx, mask) in masks.into_iter().enumerate() {
        let c = match idx % 3 {
            0 => 'r',
            1 => 'w',
            _ => 'x',
        };
        if perm & mask != 0 {
            out.push(c);
        } else {
            out.push('-');
        }
    }
    out
}

fn file_suffix(name: &str) -> String {
    name.rsplit_once('.')
        .map(|(_, suffix)| suffix.to_ascii_lowercase())
        .unwrap_or_default()
}

fn can_preview_file(suffix: &str, size: i64) -> bool {
    if size > 2 * 1024 * 1024 {
        return false;
    }
    matches!(
        suffix,
        "txt"
            | "log"
            | "md"
            | "json"
            | "yaml"
            | "yml"
            | "toml"
            | "ini"
            | "xml"
            | "html"
            | "htm"
            | "css"
            | "js"
            | "ts"
            | "rs"
            | "py"
            | "sh"
            | "sql"
            | "conf"
    )
}

fn is_dir(perm: u32) -> bool {
    (perm & 0o170000) == 0o040000
}

fn normalize_remote_path(raw: &str) -> AppResult<String> {
    let trimmed = raw.trim().replace('\\', "/");
    if trimmed.is_empty() || trimmed == "~" || trimmed == "." {
        return Ok(".".to_string());
    }
    if !trimmed.starts_with('/') {
        return Err(AppError::BadRequest(
            "path must be absolute unix path".to_string(),
        ));
    }

    let mut parts = Vec::new();
    for seg in trimmed.split('/') {
        if seg.is_empty() || seg == "." {
            continue;
        }
        if seg == ".." {
            return Err(AppError::BadRequest(
                "path cannot contain parent directory traversal".to_string(),
            ));
        }
        parts.push(seg);
    }

    if parts.is_empty() {
        Ok("/".to_string())
    } else {
        Ok(format!("/{}", parts.join("/")))
    }
}

fn join_remote_path(parent: &str, name: &str) -> String {
    if parent == "/" {
        format!("/{name}")
    } else {
        format!("{parent}/{name}")
    }
}

fn split_path_list(paths: &str) -> Vec<String> {
    paths
        .split('|')
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn sftp_operator_audit_type(op: &str) -> Option<&'static str> {
    match op {
        "mk" => Some("terminal:sftp-mkdir"),
        "to" => Some("terminal:sftp-touch"),
        "mv" => Some("terminal:sftp-move"),
        "rm" => Some("terminal:sftp-remove"),
        "chm" => Some("terminal:sftp-chmod"),
        "df" | "download" => Some("terminal:sftp-download"),
        "upload" => Some("terminal:sftp-upload"),
        _ => None,
    }
}

fn classify_ssh_read_error(err: &std::io::Error) -> SshCloseNotice {
    let raw = err.to_string();
    let lower = raw.to_ascii_lowercase();
    let retryable_kind = matches!(
        err.kind(),
        std::io::ErrorKind::ConnectionAborted
            | std::io::ErrorKind::ConnectionReset
            | std::io::ErrorKind::TimedOut
            | std::io::ErrorKind::UnexpectedEof
            | std::io::ErrorKind::BrokenPipe
            | std::io::ErrorKind::NotConnected
    );
    let retryable_text = lower.contains("transport read")
        || lower.contains("connection reset")
        || lower.contains("connection timed out")
        || lower.contains("timed out")
        || lower.contains("broken pipe")
        || lower.contains("unexpected eof")
        || lower.contains("network is unreachable")
        || lower.contains("connection aborted")
        || lower.contains("connection closed by remote host")
        || lower.contains("connection closed")
        || lower.contains("connection lost")
        || lower.contains("socket closed");

    if retryable_kind || retryable_text {
        return SshCloseNotice {
            code: TERMINAL_CLOSE_NETWORK,
            msg: "网络连接不稳定，SSH 会话已断开，正在等待重连...".to_string(),
            retryable: true,
            reason: "network-read-failed",
        };
    }

    SshCloseNotice {
        code: TERMINAL_CLOSE_FORCE,
        msg: format!("ssh read failed: {raw}"),
        retryable: false,
        reason: "read-failed",
    }
}

fn classify_ssh_write_error(err: &std::io::Error) -> SshCloseNotice {
    let raw = err.to_string();
    let lower = raw.to_ascii_lowercase();
    let retryable_kind = matches!(
        err.kind(),
        std::io::ErrorKind::ConnectionAborted
            | std::io::ErrorKind::ConnectionReset
            | std::io::ErrorKind::TimedOut
            | std::io::ErrorKind::UnexpectedEof
            | std::io::ErrorKind::BrokenPipe
            | std::io::ErrorKind::NotConnected
            | std::io::ErrorKind::WouldBlock
    );
    let retryable_text = lower.contains("transport write")
        || lower.contains("draining incoming flow")
        || lower.contains("connection reset")
        || lower.contains("connection timed out")
        || lower.contains("timed out")
        || lower.contains("broken pipe")
        || lower.contains("unexpected eof")
        || lower.contains("network is unreachable")
        || lower.contains("connection aborted")
        || lower.contains("connection closed by remote host")
        || lower.contains("connection closed")
        || lower.contains("connection lost")
        || lower.contains("socket closed")
        || lower.contains("would block");

    if retryable_kind || retryable_text {
        return SshCloseNotice {
            code: TERMINAL_CLOSE_NETWORK,
            msg: "网络连接不稳定，SSH 会话写入失败，正在等待重连...".to_string(),
            retryable: true,
            reason: "network-write-failed",
        };
    }

    SshCloseNotice {
        code: TERMINAL_CLOSE_FORCE,
        msg: format!("ssh write failed: {raw}"),
        retryable: false,
        reason: "write-failed",
    }
}

fn safe_field(value: &str) -> String {
    value.replace('|', " ")
}

fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

fn put_content_token(token: SftpContentToken) -> String {
    let id = format!("sc-{}", uuid::Uuid::new_v4());
    if let Ok(mut map) = SFTP_CONTENT_TOKENS.lock() {
        map.insert(id.clone(), token);
    }
    id
}

fn put_download_token(token: SftpDownloadToken) -> String {
    let id = format!("sd-{}", uuid::Uuid::new_v4());
    if let Ok(mut map) = SFTP_DOWNLOAD_TOKENS.lock() {
        map.insert(id.clone(), token);
    }
    id
}

fn consume_content_token(
    token: &str,
    user_id: i64,
    mode: SftpContentMode,
) -> AppResult<SftpContentToken> {
    let mut map = SFTP_CONTENT_TOKENS
        .lock()
        .map_err(|_| AppError::Internal("token lock poisoned".to_string()))?;
    let value = map
        .remove(token)
        .ok_or_else(|| AppError::BadRequest("invalid content token".to_string()))?;
    if value.user_id != user_id {
        return Err(AppError::Auth(
            "token not owned by current user".to_string(),
        ));
    }
    if value.mode != mode {
        return Err(AppError::BadRequest(
            "content token mode mismatch".to_string(),
        ));
    }
    if value.expires_at_ms < now_ms() {
        return Err(AppError::BadRequest("content token expired".to_string()));
    }
    Ok(value)
}

fn consume_download_token(token: &str, user_id: i64) -> AppResult<SftpDownloadToken> {
    let mut map = SFTP_DOWNLOAD_TOKENS
        .lock()
        .map_err(|_| AppError::Internal("token lock poisoned".to_string()))?;
    let value = map
        .remove(token)
        .ok_or_else(|| AppError::BadRequest("invalid download token".to_string()))?;
    if value.user_id != user_id {
        return Err(AppError::Auth(
            "token not owned by current user".to_string(),
        ));
    }
    if value.expires_at_ms < now_ms() {
        return Err(AppError::BadRequest("download token expired".to_string()));
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_path_list_skips_empty() {
        let list = split_path_list("/a|| /b | ");
        assert_eq!(list, vec!["/a", "/b"]);
    }

    #[test]
    fn normalize_remote_path_supports_home_and_current() {
        assert_eq!(normalize_remote_path("~").unwrap(), ".");
        assert_eq!(normalize_remote_path(".").unwrap(), ".");
        assert_eq!(normalize_remote_path("  ").unwrap(), ".");
        assert_eq!(normalize_remote_path("/tmp/a").unwrap(), "/tmp/a");
    }

    #[test]
    fn normalize_remote_path_rejects_relative() {
        let err = normalize_remote_path("tmp/a").unwrap_err();
        assert!(err.to_string().contains("absolute unix path"));
    }

    #[test]
    fn file_attr_marks_directory() {
        assert!(file_attr(0o040755).starts_with('d'));
        assert!(file_attr(0o100644).starts_with('-'));
    }

    #[test]
    fn sftp_operator_audit_type_maps_supported_ops() {
        assert_eq!(sftp_operator_audit_type("mk"), Some("terminal:sftp-mkdir"));
        assert_eq!(sftp_operator_audit_type("to"), Some("terminal:sftp-touch"));
        assert_eq!(sftp_operator_audit_type("mv"), Some("terminal:sftp-move"));
        assert_eq!(sftp_operator_audit_type("rm"), Some("terminal:sftp-remove"));
        assert_eq!(sftp_operator_audit_type("chm"), Some("terminal:sftp-chmod"));
        assert_eq!(
            sftp_operator_audit_type("df"),
            Some("terminal:sftp-download")
        );
        assert_eq!(
            sftp_operator_audit_type("upload"),
            Some("terminal:sftp-upload")
        );
        assert_eq!(
            sftp_operator_audit_type("download"),
            Some("terminal:sftp-download")
        );
        assert_eq!(sftp_operator_audit_type("ls"), None);
    }

    #[test]
    fn classify_ssh_read_error_marks_transport_read_as_reconnectable() {
        let err = std::io::Error::new(std::io::ErrorKind::Other, "transport read failed");
        let close = classify_ssh_read_error(&err);
        assert_eq!(close.code, TERMINAL_CLOSE_NETWORK);
        assert!(close.retryable);
        assert_eq!(close.reason, "network-read-failed");
        assert!(close.msg.contains("等待重连"));
    }

    #[test]
    fn classify_ssh_read_error_keeps_unknown_errors_force_closed() {
        let err = std::io::Error::new(std::io::ErrorKind::Other, "unexpected protocol error");
        let close = classify_ssh_read_error(&err);
        assert_eq!(close.code, TERMINAL_CLOSE_FORCE);
        assert!(!close.retryable);
        assert_eq!(close.reason, "read-failed");
        assert!(close.msg.contains("ssh read failed"));
    }

    #[test]
    fn classify_ssh_read_error_marks_timeout_kind_reconnectable() {
        let err = std::io::Error::new(std::io::ErrorKind::TimedOut, "read timeout");
        let close = classify_ssh_read_error(&err);
        assert_eq!(close.code, TERMINAL_CLOSE_NETWORK);
        assert!(close.retryable);
    }

    #[test]
    fn classify_ssh_read_error_marks_unexpected_eof_reconnectable() {
        let err = std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "unexpected eof");
        let close = classify_ssh_read_error(&err);
        assert_eq!(close.code, TERMINAL_CLOSE_NETWORK);
        assert!(close.retryable);
    }

    #[test]
    fn classify_ssh_write_error_marks_drain_flow_as_reconnectable() {
        let err = std::io::Error::other("Failure while draining incoming flow");
        let close = classify_ssh_write_error(&err);
        assert_eq!(close.code, TERMINAL_CLOSE_NETWORK);
        assert!(close.retryable);
        assert_eq!(close.reason, "network-write-failed");
    }

    #[test]
    fn classify_ssh_write_error_keeps_unknown_errors_force_closed() {
        let err = std::io::Error::other("unexpected protocol write error");
        let close = classify_ssh_write_error(&err);
        assert_eq!(close.code, TERMINAL_CLOSE_FORCE);
        assert!(!close.retryable);
        assert_eq!(close.reason, "write-failed");
    }

    #[test]
    fn tolerate_retryable_read_error_within_transient_budget() {
        let close = SshCloseNotice {
            code: TERMINAL_CLOSE_NETWORK,
            msg: "transient".to_string(),
            retryable: true,
            reason: "network-read-failed",
        };

        assert!(should_tolerate_retryable_read_error(&close, 1, 12));
        assert!(should_tolerate_retryable_read_error(&close, 11, 12));
        assert!(!should_tolerate_retryable_read_error(&close, 12, 12));
    }

    #[test]
    fn do_not_tolerate_non_retryable_read_error() {
        let close = SshCloseNotice {
            code: TERMINAL_CLOSE_FORCE,
            msg: "protocol".to_string(),
            retryable: false,
            reason: "read-failed",
        };

        assert!(!should_tolerate_retryable_read_error(&close, 1, 12));
    }

    #[test]
    fn tolerate_retryable_read_error_uses_configured_budget() {
        let close = SshCloseNotice {
            code: TERMINAL_CLOSE_NETWORK,
            msg: "transient".to_string(),
            retryable: true,
            reason: "network-read-failed",
        };

        assert!(should_tolerate_retryable_read_error(&close, 2, 3));
        assert!(!should_tolerate_retryable_read_error(&close, 3, 3));
    }

    #[test]
    fn parse_transfer_token_rejects_future_timestamp() {
        let future = now_ms() + TOKEN_EXPIRE_MS + 1000;
        let payload = format!("transfer:1:{future}");
        let token = format!("{payload}:{}", token_signature(&payload, "k"));
        let err = parse_transfer_token(&token, "k").unwrap_err();
        assert!(err.to_string().contains("transfer token expired"));
    }

    #[test]
    fn mask_command_for_audit_masks_mysql_inline_password() {
        let masked = crate::domain::terminal::command_audit::mask_command_for_audit(
            "mysql -h 127.0.0.1 -uroot -pMySecret",
        );
        assert_eq!(masked, "mysql -h 127.0.0.1 -uroot -p***");
    }

    #[test]
    fn mask_command_for_audit_masks_key_value_secrets() {
        let masked = crate::domain::terminal::command_audit::mask_command_for_audit(
            "export token=abc123 password='abc'",
        );
        assert_eq!(masked, "export token=*** password=***");
    }
}

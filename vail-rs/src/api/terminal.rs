use std::{
    collections::HashMap,
    io::{Read, Write},
    path::Path as FsPath,
    time::Duration,
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
use futures_util::StreamExt;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use sqlx::PgPool;
use tokio::sync::mpsc;

use crate::{
    api::guard,
    application::orion::compat_service,
    domain::orion::compat::OrionCompatModule,
    error::{AppError, AppResult},
    ssh_client::{self, HostSshConfig},
};

use super::AppState;

const TERMINAL_CLOSE_FORCE: i32 = 10000;
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
    Closed { code: i32, msg: String },
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SshConnectPayload {
    width: Option<u32>,
    height: Option<u32>,
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

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/terminal/themes", get(get_terminal_themes))
        .route(
            "/keep-alive/terminal/access/:protocol/:token",
            get(open_terminal_access_ws),
        )
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
    Path((protocol, token)): Path<(String, String)>,
    ws: WebSocketUpgrade,
) -> AppResult<impl axum::response::IntoResponse> {
    let (user_id, host_id, connect_type) = parse_access_token(&token)?;
    let protocol = protocol.to_ascii_lowercase();
    let connect_type = connect_type.to_ascii_lowercase();

    let is_admin = has_host_read_permission(&state, user_id).await?;
    if !can_access_host(&state, user_id, host_id, is_admin).await? {
        return Err(AppError::Auth("Host access denied".to_string()));
    }

    match (protocol.as_str(), connect_type.as_str()) {
        ("ssh", "ssh") => Ok(ws.on_upgrade(move |socket| async move {
            handle_ssh_socket(state, socket, user_id, host_id).await;
        })),
        ("sftp", "sftp") => Ok(ws.on_upgrade(move |socket| async move {
            handle_sftp_socket(state, socket, user_id, host_id).await;
        })),
        _ => Err(AppError::BadRequest(
            "unsupported terminal protocol".to_string(),
        )),
    }
}

async fn open_terminal_transfer_ws(
    State(state): State<AppState>,
    Path(token): Path<String>,
    ws: WebSocketUpgrade,
) -> AppResult<impl axum::response::IntoResponse> {
    let user_id = parse_transfer_token(&token)?;
    Ok(ws.on_upgrade(move |socket| async move {
        handle_transfer_socket(state, socket, user_id).await;
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

    std::thread::spawn(move || run_ssh_worker(cmd_rx, event_tx, timeout_secs));

    let session_id = format!("ssh-{}", uuid::Uuid::new_v4());
    let session_start = now_ms();
    let context = load_terminal_audit_context(&state, user_id, host_id).await;
    let mut connect_log_id: Option<i64> = None;
    let mut close_error: Option<String> = None;
    if socket
        .send(Message::Text(format!("id|{session_id}").into()))
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
                                user_id,
                                host_id,
                                &context,
                                "SSH",
                                &session_id,
                                "CONNECTING",
                                session_start,
                                0,
                                None,
                            )
                            .await;
                        }
                        socket.send(Message::Text("co".into())).await
                    }
                    SshWorkerEvent::Output(body) => {
                        socket.send(Message::Text(format!("o|{body}").into())).await
                    }
                    SshWorkerEvent::Closed { code, msg } => {
                        if code != 0 {
                            close_error = Some(msg.clone());
                        }
                        socket.send(Message::Text(format!("cl|{code}|{}", safe_field(&msg)).into())).await
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
                            if socket.send(Message::Text("p".into())).await.is_err() {
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
                                .unwrap_or(SshConnectPayload { width: None, height: None });
                            let cfg = match ssh_client::resolve_host_ssh_config(
                                &state.db,
                                &state.config.secrets.data_encryption_key,
                                Some(user_id),
                                host_id,
                            ).await {
                                Ok(v) => v,
                                Err(err) => {
                                    let _ = socket.send(Message::Text(format!("cl|{TERMINAL_CLOSE_FORCE}|{}", safe_field(&err.to_string())).into())).await;
                                    break;
                                }
                            };
                            let width = payload.width.unwrap_or(120).max(1);
                            let height = payload.height.unwrap_or(40).max(1);
                            let _ = cmd_tx.send(SshWorkerCommand::Connect { config: cfg, width, height });
                            continue;
                        }

                        if let Some(command) = text.strip_prefix("i|") {
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
    let status = if connected { "COMPLETE" } else { "FAILED" };
    if let Some(log_id) = connect_log_id {
        update_terminal_connect_log(&state, user_id, log_id, status, now_ms()).await;
    } else {
        let _ = create_terminal_connect_log(
            &state,
            user_id,
            host_id,
            &context,
            "SSH",
            &session_id,
            status,
            session_start,
            now_ms(),
            close_error.as_deref(),
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
        .send(Message::Text(format!("id|{session_id}").into()))
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
            if socket.send(Message::Text("p".into())).await.is_err() {
                break;
            }
            continue;
        }

        if text == "cl" {
            let _ = socket
                .send(Message::Text("cl|0|会话已结束...".into()))
                .await;
            break;
        }

        if let Some(_) = text.strip_prefix("co|") {
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
                            user_id,
                            host_id,
                            &context,
                            "SFTP",
                            &session_id,
                            "CONNECTING",
                            session_start,
                            0,
                            None,
                        )
                        .await;
                    }
                    if socket.send(Message::Text("co".into())).await.is_err() {
                        break;
                    }
                }
                Err(err) => {
                    close_error = Some(err.to_string());
                    let _ = socket
                        .send(Message::Text(
                            format!("cl|{TERMINAL_CLOSE_FORCE}|{}", safe_field(&err.to_string()))
                                .into(),
                        ))
                        .await;
                    break;
                }
            }
            continue;
        }

        if !connected {
            let _ = socket
                .send(Message::Text("cl|10000|sftp session not connected".into()))
                .await;
            break;
        }

        let send_res = if let Some(body) = text.strip_prefix("ls|") {
            let mut parts = body.splitn(2, '|');
            let show_hidden = parts.next().unwrap_or("0") == "1";
            let path = parts.next().unwrap_or("/");
            match sftp_list(&state, host_id, path, show_hidden).await {
                Ok(list) => {
                    let body = serde_json::to_string(&list).unwrap_or_else(|_| "[]".to_string());
                    socket
                        .send(Message::Text(
                            format!("ls|{}|1||{body}", safe_field(path)).into(),
                        ))
                        .await
                }
                Err(err) => {
                    socket
                        .send(Message::Text(
                            format!(
                                "ls|{}|0|{}|[]",
                                safe_field(path),
                                safe_field(&err.to_string())
                            )
                            .into(),
                        ))
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
                    socket.send(Message::Text("mk|1|".into())).await
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
                        .send(Message::Text(
                            format!("mk|0|{}", safe_field(&err.to_string())).into(),
                        ))
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
                    socket.send(Message::Text("to|1|".into())).await
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
                        .send(Message::Text(
                            format!("to|0|{}", safe_field(&err.to_string())).into(),
                        ))
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
                    socket.send(Message::Text("mv|1|".into())).await
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
                        .send(Message::Text(
                            format!("mv|0|{}", safe_field(&err.to_string())).into(),
                        ))
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
                    socket.send(Message::Text("rm|1|".into())).await
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
                        .send(Message::Text(
                            format!("rm|0|{}", safe_field(&err.to_string())).into(),
                        ))
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
                    socket.send(Message::Text("chm|1|".into())).await
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
                        .send(Message::Text(
                            format!("chm|0|{}", safe_field(&err.to_string())).into(),
                        ))
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
                        .send(Message::Text(
                            format!("df|{}|1||{body}", safe_field(current_path)).into(),
                        ))
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
                        .send(Message::Text(
                            format!(
                                "df|{}|0|{}|[]",
                                safe_field(current_path),
                                safe_field(&err.to_string())
                            )
                            .into(),
                        ))
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
                .send(Message::Text(
                    format!("gc|1||{}", safe_field(&token)).into(),
                ))
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
                .send(Message::Text(
                    format!("sc|1||{}", safe_field(&token)).into(),
                ))
                .await
        } else {
            socket
                .send(Message::Text("cl|10000|unsupported sftp protocol".into()))
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
            user_id,
            host_id,
            &context,
            "SFTP",
            &session_id,
            status,
            session_start,
            now_ms(),
            close_error.as_deref(),
        )
        .await;
    }
}

fn run_ssh_worker(
    cmd_rx: std::sync::mpsc::Receiver<SshWorkerCommand>,
    event_tx: mpsc::UnboundedSender<SshWorkerEvent>,
    timeout_secs: u64,
) {
    let mut session: Option<ssh2::Session> = None;
    let mut channel: Option<ssh2::Channel> = None;

    loop {
        match cmd_rx.recv_timeout(Duration::from_millis(10)) {
            Ok(cmd) => match cmd {
                SshWorkerCommand::Connect {
                    config,
                    width,
                    height,
                } => {
                    match ssh_client::connect_session(&config, timeout_secs).and_then(|sess| {
                        let mut ch = sess.channel_session().map_err(|e| {
                            AppError::Ssh(format!("open shell channel failed: {e}"))
                        })?;
                        ch.request_pty("xterm", None, Some((width, height, 0, 0)))
                            .map_err(|e| AppError::Ssh(format!("request pty failed: {e}")))?;
                        ch.shell()
                            .map_err(|e| AppError::Ssh(format!("open shell failed: {e}")))?;
                        sess.set_blocking(false);
                        Ok((sess, ch))
                    }) {
                        Ok((sess, ch)) => {
                            session = Some(sess);
                            channel = Some(ch);
                            let _ = event_tx.send(SshWorkerEvent::Connected);
                        }
                        Err(err) => {
                            let _ = event_tx.send(SshWorkerEvent::Closed {
                                code: TERMINAL_CLOSE_FORCE,
                                msg: err.to_string(),
                            });
                            break;
                        }
                    }
                }
                SshWorkerCommand::Input(command) => {
                    if let Some(ch) = channel.as_mut() {
                        if let Err(err) = ch.write_all(command.as_bytes()) {
                            let _ = event_tx.send(SshWorkerEvent::Closed {
                                code: TERMINAL_CLOSE_FORCE,
                                msg: format!("ssh write failed: {err}"),
                            });
                            break;
                        }
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
                    let _ = event_tx.send(SshWorkerEvent::Closed {
                        code: 0,
                        msg: "会话已结束...".to_string(),
                    });
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
                    let out = String::from_utf8_lossy(&buf[..read]).to_string();
                    let _ = event_tx.send(SshWorkerEvent::Output(out));
                }
                Ok(_) => {
                    if ch.eof() {
                        let _ = event_tx.send(SshWorkerEvent::Closed {
                            code: 0,
                            msg: "会话已结束...".to_string(),
                        });
                        break;
                    }
                }
                Err(err) => {
                    if err.kind() != std::io::ErrorKind::WouldBlock {
                        let _ = event_tx.send(SshWorkerEvent::Closed {
                            code: TERMINAL_CLOSE_FORCE,
                            msg: format!("ssh read failed: {err}"),
                        });
                        break;
                    }
                }
            }
        }
    }
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

                        let is_admin = has_host_read_permission(&state, user_id)
                            .await
                            .unwrap_or(false);
                        let allowed = can_access_host(&state, user_id, host_id, is_admin)
                            .await
                            .unwrap_or(false);
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

async fn send_transfer(
    socket: &mut WebSocket,
    payload: serde_json::Value,
) -> Result<(), axum::Error> {
    socket.send(Message::Text(payload.to_string().into())).await
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
    user_id: i64,
    host_id: i64,
    context: &TerminalAuditContext,
    connect_type: &str,
    session_id: &str,
    status: &str,
    start_time: i64,
    end_time: i64,
    error_message: Option<&str>,
) -> Option<i64> {
    let payload = serde_json::json!({
        "userId": user_id,
        "username": context.username,
        "hostId": host_id,
        "hostName": context.host_name,
        "hostAddress": context.host_address,
        "type": connect_type,
        "sessionId": session_id,
        "status": status,
        "startTime": start_time,
        "endTime": end_time,
        "extra": {
            "traceId": uuid::Uuid::new_v4().to_string(),
            "channel": connect_type,
            "channelId": session_id,
            "sessionId": session_id,
            "address": "",
            "location": "",
            "userAgent": "",
            "errorMessage": error_message.unwrap_or("")
        }
    });

    compat_service::create_record(
        &state.db,
        OrionCompatModule::TerminalConnectLog,
        payload,
        &format!("user-{user_id}"),
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

fn parse_access_token(token: &str) -> AppResult<(i64, i64, String)> {
    let parts = token.split(':').collect::<Vec<_>>();
    if parts.len() < 5 || parts[0] != "term" {
        return Err(AppError::BadRequest(
            "invalid terminal access token".to_string(),
        ));
    }
    let user_id = parts[1]
        .parse::<i64>()
        .map_err(|_| AppError::BadRequest("invalid user in terminal access token".to_string()))?;
    let host_id = parts[2]
        .parse::<i64>()
        .map_err(|_| AppError::BadRequest("invalid host in terminal access token".to_string()))?;
    Ok((user_id, host_id, parts[3].to_string()))
}

fn parse_transfer_token(token: &str) -> AppResult<i64> {
    let parts = token.split(':').collect::<Vec<_>>();
    if parts.len() < 3 || parts[0] != "transfer" {
        return Err(AppError::BadRequest("invalid transfer token".to_string()));
    }
    parts[1]
        .parse::<i64>()
        .map_err(|_| AppError::BadRequest("invalid user in transfer token".to_string()))
}

async fn has_host_read_permission(state: &AppState, user_id: i64) -> AppResult<bool> {
    let allowed = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(
            SELECT 1
            FROM sys_user_role ur
            JOIN sys_role r ON r.id = ur.role_id AND r.deleted = 0 AND r.status = 1
            JOIN sys_role_permission rp ON rp.role_id = ur.role_id
            JOIN sys_permission p ON p.id = rp.permission_id
            WHERE ur.user_id = $1 AND p.code = 'host.read'
        )",
    )
    .bind(user_id)
    .fetch_one(&state.db)
    .await?;
    Ok(allowed)
}

async fn can_access_host(
    state: &AppState,
    user_id: i64,
    host_id: i64,
    is_admin: bool,
) -> AppResult<bool> {
    if is_admin {
        let exists = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM host WHERE id = $1 AND deleted = 0 AND status = 1)",
        )
        .bind(host_id)
        .fetch_one(&state.db)
        .await?;
        return Ok(exists);
    }

    let exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(
            SELECT 1
            FROM user_host_access uha
            JOIN host h ON h.id = uha.host_id
            WHERE uha.user_id = $1
              AND uha.host_id = $2
              AND h.deleted = 0
              AND h.status = 1
        )",
    )
    .bind(user_id)
    .bind(host_id)
    .fetch_one(&state.db)
    .await?;

    Ok(exists)
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
) -> AppResult<Vec<SftpFile>> {
    let path = normalize_remote_path(path)?;
    with_sftp(state, host_id, move |sftp| {
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
            let full = join_remote_path(&path, &name);
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
        Ok(list)
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
}

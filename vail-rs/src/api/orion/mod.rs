use axum::{
    extract::{ConnectInfo, Path, Query, State},
    http::{HeaderMap, Method},
    routing::{any, delete, get, post, put},
    Json, Router,
};
use base64::{
    engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD},
    Engine as _,
};
use chrono::{DateTime, NaiveDateTime, Utc};
use rsa::{
    pkcs1::DecodeRsaPrivateKey,
    pkcs8::{DecodePrivateKey, EncodePrivateKey, EncodePublicKey, LineEnding},
    rand_core::OsRng,
    Oaep, RsaPrivateKey, RsaPublicKey,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;
use std::net::SocketAddr;

use crate::{
    api::{auth, guard, AppState},
    application::orion::{
        asset_service, audit_service, compat_service, dict_key_service, dict_value_service,
        host_service, iam_service, infra_statistics_service, menu_service, operator_log_service,
        role_service, system_message_service, system_user_service,
    },
    domain::orion::{
        asset::{OrionHostGroupAggregate, OrionHostIdentityAggregate, OrionHostKeyAggregate},
        compat::OrionCompatModule,
        dict_key::{OrionDictKeyAggregate, OrionDictKeyQueryFilters},
        dict_value::{
            OrionDictValueAggregate, OrionDictValueOptionAggregate, OrionDictValueQueryFilters,
        },
        host::OrionHostAggregate,
        infra_statistics::{OrionInfraWorkplaceAggregate, OrionLoginHistoryAggregate},
        menu::OrionMenuAggregate,
        operator_log::OrionOperatorLogAggregate,
        role::OrionRoleAggregate,
        system_message::{
            OrionSystemMessageAggregate, OrionSystemMessageCountFilters,
            OrionSystemMessageListFilters,
        },
        system_user::OrionSystemUserAggregate,
    },
    error::{AppError, AppResult},
    security,
};

const ORION_MINE_OPERATOR_LOG_QUERY: &str = "/infra/mine/query-operator-log";
const ORION_OPERATOR_LOG_QUERY: &str = "/infra/operator-log/query";
const ORION_OPERATOR_LOG_COUNT: &str = "/infra/operator-log/count";
const ORION_OPERATOR_LOG_DELETE: &str = "/infra/operator-log/delete";
const ORION_OPERATOR_LOG_CLEAR: &str = "/infra/operator-log/clear";

const ORION_TERMINAL_SESSION_QUERY: &str = "/terminal/session/query";
const ORION_TERMINAL_SESSION_FORCE_OFFLINE: &str = "/terminal/session/force-offline";

const ORION_TERMINAL_CONNECT_LOG_QUERY: &str = "/terminal/connect-log/query";
const ORION_TERMINAL_CONNECT_LOG_LATEST: &str = "/terminal/connect-log/latest-connect";
const ORION_TERMINAL_CONNECT_LOG_DELETE: &str = "/terminal/connect-log/delete";
const ORION_TERMINAL_CONNECT_LOG_COUNT: &str = "/terminal/connect-log/count";
const ORION_TERMINAL_CONNECT_LOG_CLEAR: &str = "/terminal/connect-log/clear";

const ORION_TERMINAL_FILE_LOG_QUERY: &str = "/terminal/file-log/query";
const ORION_TERMINAL_FILE_LOG_COUNT: &str = "/terminal/file-log/count";
const ORION_TERMINAL_FILE_LOG_DELETE: &str = "/terminal/file-log/delete";
#[derive(Debug, Serialize)]
struct OrionResponse<T> {
    code: u16,
    msg: String,
    data: T,
}

impl<T> OrionResponse<T> {
    fn ok(data: T) -> Json<Self> {
        Json(Self {
            code: 200,
            msg: "success".to_string(),
            data,
        })
    }
}

fn orion_ok<T: Serialize>(data: T) -> Json<OrionResponse<serde_json::Value>> {
    OrionResponse::ok(serde_json::to_value(data).unwrap_or(serde_json::Value::Null))
}

#[derive(Debug, Deserialize)]
struct OrionLoginRequest {
    username: String,
    password: String,
}

#[derive(Debug, Serialize)]
struct OrionLoginResponse {
    token: String,
}

#[derive(Debug, Serialize)]
struct OrionTagItem {
    id: i64,
    name: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct OrionHostResponse {
    id: i64,
    types: Vec<String>,
    os_type: String,
    arch_type: String,
    name: String,
    code: String,
    address: String,
    status: String,
    agent_key: String,
    agent_version: String,
    agent_install_status: i32,
    agent_online_status: i32,
    agent_online_change_time: i64,
    description: String,
    create_time: i64,
    update_time: i64,
    creator: String,
    updater: String,
    alias: String,
    color: String,
    tags: Vec<OrionTagItem>,
    group_id_list: Vec<i64>,
    spec: serde_json::Value,
    favorite: bool,
    editable: bool,
    loading: bool,
    mod_count: i32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OrionHostQueryRequest {
    page: Option<i64>,
    limit: Option<i64>,
    search_value: Option<String>,
    id: Option<i64>,
    name: Option<String>,
    address: Option<String>,
    status: Option<String>,
}

#[derive(Debug, Serialize)]
struct OrionHostDataGrid {
    page: i64,
    limit: i64,
    total: i64,
    rows: Vec<OrionHostResponse>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OrionHostCreateRequest {
    name: Option<String>,
    address: Option<String>,
    description: Option<String>,
    group_id_list: Option<Vec<i64>>,
    tags: Option<Vec<i64>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OrionHostUpdateRequest {
    id: Option<i64>,
    name: Option<String>,
    address: Option<String>,
    description: Option<String>,
    group_id_list: Option<Vec<i64>>,
    tags: Option<Vec<i64>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OrionHostUpdateStatusRequest {
    id: Option<i64>,
    status: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OrionHostExtraQuery {
    host_id: Option<i64>,
    item: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OrionHostExtraUpdateRequest {
    host_id: Option<i64>,
    item: Option<String>,
    extra: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OrionHostConfigGetRequest {
    host_id: Option<i64>,
    r#type: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OrionHostConfigUpdateRequest {
    host_id: Option<i64>,
    r#type: Option<String>,
    config: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct OrionUserBaseResponse {
    id: i64,
    username: String,
    nickname: String,
    avatar: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct OrionUserAggregateResponse {
    user: OrionUserBaseResponse,
    roles: Vec<String>,
    permissions: Vec<String>,
    system_preference: serde_json::Value,
    tipped_keys: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct OrionMenuItem {
    id: i64,
    parent_id: i64,
    name: String,
    permission: String,
    r#type: i16,
    sort: i32,
    visible: i16,
    status: i16,
    cache: i16,
    new_window: i16,
    icon: String,
    path: String,
    component: String,
    children: Vec<OrionMenuItem>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct OrionHostGroupTreeNode {
    key: i64,
    parent_id: i64,
    title: String,
    children: Vec<OrionHostGroupTreeNode>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OrionHostGroupCreateRequest {
    parent_id: Option<i64>,
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OrionHostGroupRenameRequest {
    id: Option<i64>,
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OrionHostGroupMoveRequest {
    id: Option<i64>,
    target_id: Option<i64>,
    position: Option<i32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OrionHostGroupRelListQuery {
    group_id: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OrionHostGroupUpdateRelRequest {
    group_id: Option<i64>,
    host_id_list: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct OrionHostIdQuery {
    id: i64,
}

#[derive(Debug, Deserialize)]
struct OrionHostListQuery {
    #[serde(rename = "type")]
    host_type: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OrionHostKeyUpsertRequest {
    id: Option<i64>,
    name: Option<String>,
    #[serde(rename = "publicKey")]
    _public_key: Option<String>,
    #[serde(alias = "private_key")]
    private_key: Option<String>,
    password: Option<String>,
    description: Option<String>,
    #[serde(alias = "use_new_password")]
    use_new_password: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OrionHostKeyQueryRequest {
    page: Option<i64>,
    limit: Option<i64>,
    search_value: Option<String>,
    id: Option<i64>,
    name: Option<String>,
    description: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OrionHostIdentityUpsertRequest {
    id: Option<i64>,
    name: Option<String>,
    r#type: Option<String>,
    username: Option<String>,
    password: Option<String>,
    key_id: Option<i64>,
    description: Option<String>,
    use_new_password: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OrionHostIdentityQueryRequest {
    page: Option<i64>,
    limit: Option<i64>,
    search_value: Option<String>,
    id: Option<i64>,
    name: Option<String>,
    r#type: Option<String>,
    username: Option<String>,
    key_id: Option<i64>,
    description: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OrionAssetDataGrantRequest {
    user_id: Option<i64>,
    role_id: Option<i64>,
    id_list: Option<Vec<i64>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OrionAssetAuthorizedDataQuery {
    user_id: Option<i64>,
    role_id: Option<i64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct OrionHostKeyItem {
    id: i64,
    name: String,
    public_key: String,
    private_key: String,
    password: String,
    description: String,
    create_time: i64,
    update_time: i64,
    creator: String,
    updater: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct OrionHostIdentityItem {
    id: i64,
    name: String,
    r#type: String,
    username: String,
    password: String,
    key_id: i64,
    description: String,
    create_time: i64,
    update_time: i64,
    creator: String,
    updater: String,
}

#[derive(Debug, Deserialize)]
struct OrionDictValueListQuery {
    keys: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OrionDictKeyUpsertRequest {
    id: Option<i64>,
    key_name: Option<String>,
    value_type: Option<String>,
    extra_schema: Option<String>,
    description: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OrionDictKeyQueryRequest {
    page: Option<i64>,
    limit: Option<i64>,
    search_value: Option<String>,
    id: Option<i64>,
    key_name: Option<String>,
    description: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OrionDictValueUpsertRequest {
    id: Option<i64>,
    key_id: Option<i64>,
    name: Option<String>,
    value: Option<String>,
    label: Option<String>,
    extra: Option<String>,
    sort: Option<i32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OrionDictValueQueryRequest {
    page: Option<i64>,
    limit: Option<i64>,
    key_id: Option<i64>,
    key_name: Option<String>,
    value: Option<String>,
    label: Option<String>,
    extra: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OrionDictValueRollbackRequest {
    id: Option<i64>,
    value_id: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OrionSystemMessageQueryRequest {
    page: Option<i64>,
    limit: Option<i64>,
    max_id: Option<i64>,
    classify: Option<String>,
    query_unread: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OrionSystemMessageCountQuery {
    query_unread: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct OrionIdQuery {
    id: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct OrionKeyQuery {
    key: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OrionSystemMessageReadAllQuery {
    classify: Option<String>,
}

#[derive(Debug, Serialize)]
struct OrionDictOption {
    label: String,
    value: serde_json::Value,
    #[serde(flatten)]
    extra: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct OrionDictKeyItem {
    id: i64,
    key_name: String,
    value_type: String,
    extra_schema: String,
    description: String,
    create_time: i64,
    update_time: i64,
    creator: String,
    updater: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct OrionDictValueItem {
    id: i64,
    key_id: i64,
    key_name: String,
    key_description: String,
    value: String,
    label: String,
    extra: String,
    sort: i32,
    create_time: i64,
    update_time: i64,
    creator: String,
    updater: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct OrionSystemMessageItem {
    id: i64,
    classify: String,
    r#type: String,
    status: i16,
    rel_key: String,
    title: String,
    content: String,
    content_html: String,
    create_time: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct OrionLineSingleChartData {
    x: Vec<String>,
    data: Vec<i64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct OrionLoginHistoryItem {
    id: i64,
    address: String,
    location: String,
    user_agent: String,
    result: i16,
    error_message: String,
    create_time: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct OrionInfraWorkplaceStatisticsResponse {
    user_id: i64,
    username: String,
    nickname: String,
    unread_message_count: i64,
    last_login_time: i64,
    user_session_count: i64,
    operator_chart: OrionLineSingleChartData,
    login_history_list: Vec<OrionLoginHistoryItem>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct OrionExecWorkplaceStatisticsResponse {
    exec_job_count: i64,
    today_exec_command_count: i64,
    week_exec_command_count: i64,
    exec_command_chart: OrionLineSingleChartData,
    exec_log_list: Vec<serde_json::Value>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct OrionTerminalWorkplaceStatisticsResponse {
    today_terminal_connect_count: i64,
    week_terminal_connect_count: i64,
    terminal_connect_chart: OrionLineSingleChartData,
    terminal_connect_list: Vec<serde_json::Value>,
}

fn get_source_ip(headers: &HeaderMap, connect_info: Option<&ConnectInfo<SocketAddr>>) -> String {
    headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(',').next())
        .map(|v| v.trim().to_string())
        .or_else(|| {
            headers
                .get("x-real-ip")
                .and_then(|v| v.to_str().ok())
                .map(|v| v.to_string())
        })
        .or_else(|| connect_info.map(|addr| addr.0.ip().to_string()))
        .unwrap_or_else(|| "0.0.0.0".to_string())
}

fn sanitize_search(v: Option<String>) -> Option<String> {
    v.map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
}

const CLIENT_OAEP_PREFIX: &str = "oaep-sha256:";

async fn decrypt_client_sensitive_input(
    state: &AppState,
    input: Option<String>,
    field_name: &str,
) -> AppResult<Option<String>> {
    let Some(raw) = input else {
        return Ok(None);
    };

    if !raw.starts_with(CLIENT_OAEP_PREFIX) {
        return Ok(Some(raw));
    }

    let payload = raw[CLIENT_OAEP_PREFIX.len()..].trim();
    if payload.is_empty() {
        return Err(AppError::BadRequest(format!(
            "{field_name} encrypted payload is empty"
        )));
    }

    let key = OrionCompatModule::SystemSetting.store_key();
    let map = compat_service::get_config_map(&state.db, key).await?;
    let private_key_pem = map
        .get("encrypt.private-key")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .ok_or_else(|| {
            AppError::BadRequest(
                "server RSA private key is not configured in system settings".to_string(),
            )
        })?;

    let private_key = RsaPrivateKey::from_pkcs8_pem(private_key_pem)
        .or_else(|_| RsaPrivateKey::from_pkcs1_pem(private_key_pem))
        .map_err(|_| AppError::Internal("server RSA private key format is invalid".to_string()))?;

    let mut plain = Vec::new();
    for chunk in payload.split('|').filter(|v| !v.is_empty()) {
        let encrypted_chunk = STANDARD.decode(chunk).map_err(|_| {
            AppError::BadRequest(format!(
                "{field_name} encrypted payload contains invalid base64 chunk"
            ))
        })?;

        let decrypted_chunk = private_key
            .decrypt(Oaep::new::<Sha256>(), &encrypted_chunk)
            .map_err(|_| {
                AppError::BadRequest(format!(
                    "{field_name} encrypted payload cannot be decrypted"
                ))
            })?;
        plain.extend_from_slice(&decrypted_chunk);
    }

    let plain_text = String::from_utf8(plain).map_err(|_| {
        AppError::BadRequest(format!("{field_name} decrypted payload is not valid UTF-8"))
    })?;

    Ok(Some(plain_text))
}

fn log_private_key_validation_failure(stage: &str, key: &str, detail: &str) {
    let mut hasher = Sha256::new();
    hasher.update(key.as_bytes());
    let digest = hasher.finalize();

    let mut key_fingerprint = String::with_capacity(16);
    for byte in digest.iter().take(8) {
        let _ = write!(&mut key_fingerprint, "{byte:02x}");
    }

    tracing::warn!(
        target: "security::ssh_key_validation",
        stage,
        detail,
        key_len = key.len(),
        line_count = key.lines().count(),
        has_begin_openssh = key.contains("-----BEGIN OPENSSH PRIVATE KEY-----"),
        has_end_openssh = key.contains("-----END OPENSSH PRIVATE KEY-----"),
        has_begin_rsa = key.contains("-----BEGIN RSA PRIVATE KEY-----"),
        has_end_rsa = key.contains("-----END RSA PRIVATE KEY-----"),
        has_begin_pkcs8 = key.contains("-----BEGIN PRIVATE KEY-----"),
        has_end_pkcs8 = key.contains("-----END PRIVATE KEY-----"),
        key_fingerprint = %key_fingerprint,
        "ssh private key validation failed"
    );
}

fn normalize_and_validate_private_key(private_key: &str) -> AppResult<String> {
    let mut normalized = private_key.trim().to_string();
    if normalized.is_empty() {
        log_private_key_validation_failure("empty", private_key, "private key is empty");
        return Err(AppError::BadRequest("privateKey is required".to_string()));
    }

    normalized = normalized.trim_start_matches('\u{feff}').to_string();

    if !normalized.contains('\n') && normalized.contains("\\n") {
        normalized = normalized.replace("\\n", "\n");
    }
    normalized = normalized.replace("\r\n", "\n");
    normalized = normalized.replace('\r', "\n");
    normalized = normalized.trim().to_string();

    if normalized.contains("-----BEGIN OPENSSH PRIVATE KEY-----") {
        if let Err(err) = ssh_key::PrivateKey::from_openssh(&normalized) {
            log_private_key_validation_failure(
                "openssh-parse",
                &normalized,
                &format!("openssh parser rejected key: {err}"),
            );
            return Err(AppError::BadRequest(
                "unsupported private key file format".to_string(),
            ));
        }
        return Ok(normalized);
    }

    const LEGACY_ALLOWED_MARKERS: [(&str, &str); 2] = [
        (
            "-----BEGIN RSA PRIVATE KEY-----",
            "-----END RSA PRIVATE KEY-----",
        ),
        ("-----BEGIN PRIVATE KEY-----", "-----END PRIVATE KEY-----"),
    ];

    let marker = LEGACY_ALLOWED_MARKERS
        .iter()
        .find(|(begin, end)| normalized.contains(begin) && normalized.contains(end));

    if marker.is_none() {
        log_private_key_validation_failure("legacy-marker", &normalized, "legacy marker not found");
        return Err(AppError::BadRequest(
            "unsupported private key file format".to_string(),
        ));
    }

    let non_empty_lines = normalized
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .count();
    if non_empty_lines < 3 {
        log_private_key_validation_failure(
            "legacy-lines",
            &normalized,
            "legacy key has fewer than 3 non-empty lines",
        );
        return Err(AppError::BadRequest(
            "unsupported private key file format".to_string(),
        ));
    }

    Ok(normalized)
}

async fn get_tipped_keys(state: &AppState, user_id: i64) -> AppResult<Vec<String>> {
    compat_service::get_user_tipped_keys(&state.db, user_id).await
}

async fn save_tipped_keys(state: &AppState, user_id: i64, tipped_keys: &[String]) -> AppResult<()> {
    compat_service::save_user_tipped_keys(&state.db, user_id, tipped_keys).await
}

fn map_host_key_item(v: OrionHostKeyAggregate) -> OrionHostKeyItem {
    OrionHostKeyItem {
        id: v.id,
        name: v.name,
        public_key: String::new(),
        private_key: String::new(),
        password: String::new(),
        description: v.description.unwrap_or_default(),
        create_time: v.create_time_ms,
        update_time: v.update_time_ms,
        creator: "system".to_string(),
        updater: "system".to_string(),
    }
}

fn map_host_identity_item(v: OrionHostIdentityAggregate) -> OrionHostIdentityItem {
    OrionHostIdentityItem {
        id: v.id,
        name: v.name,
        r#type: v.identity_type,
        username: v.username.unwrap_or_default(),
        password: String::new(),
        key_id: v.key_id.unwrap_or_default(),
        description: v.description.unwrap_or_default(),
        create_time: v.create_time_ms,
        update_time: v.update_time_ms,
        creator: "system".to_string(),
        updater: "system".to_string(),
    }
}

fn build_orion_menu_tree(rows: Vec<OrionMenuItem>) -> Vec<OrionMenuItem> {
    let items_by_id: HashMap<i64, OrionMenuItem> = rows
        .into_iter()
        .map(|item| {
            (
                item.id,
                OrionMenuItem {
                    children: Vec::new(),
                    ..item
                },
            )
        })
        .collect();

    let mut children_by_parent: HashMap<i64, Vec<i64>> = HashMap::new();
    for item in items_by_id.values() {
        children_by_parent
            .entry(item.parent_id)
            .or_default()
            .push(item.id);
    }

    fn build_node(
        id: i64,
        items_by_id: &HashMap<i64, OrionMenuItem>,
        children_by_parent: &HashMap<i64, Vec<i64>>,
    ) -> Option<OrionMenuItem> {
        let mut node = items_by_id.get(&id)?.clone();
        let mut child_ids = children_by_parent.get(&id).cloned().unwrap_or_default();
        child_ids.sort_unstable();
        node.children = child_ids
            .into_iter()
            .filter_map(|child_id| build_node(child_id, items_by_id, children_by_parent))
            .collect();
        Some(node)
    }

    let mut roots = children_by_parent.get(&0).cloned().unwrap_or_default();
    roots.sort_unstable();
    roots
        .into_iter()
        .filter_map(|id| build_node(id, &items_by_id, &children_by_parent))
        .collect()
}

fn build_host_group_tree(rows: Vec<OrionHostGroupAggregate>) -> Vec<OrionHostGroupTreeNode> {
    let mut children_by_parent: HashMap<i64, Vec<(i64, String)>> = HashMap::new();
    for row in rows {
        children_by_parent
            .entry(row.parent_id)
            .or_default()
            .push((row.id, row.name));
    }

    fn build(
        parent_id: i64,
        children_by_parent: &HashMap<i64, Vec<(i64, String)>>,
    ) -> Vec<OrionHostGroupTreeNode> {
        let mut children = children_by_parent
            .get(&parent_id)
            .cloned()
            .unwrap_or_default();
        children.sort_by_key(|(id, _)| *id);
        children
            .into_iter()
            .map(|(id, name)| OrionHostGroupTreeNode {
                key: id,
                parent_id,
                title: name,
                children: build(id, children_by_parent),
            })
            .collect()
    }

    build(0, &children_by_parent)
}

fn normalize_pagination(page: Option<i64>, limit: Option<i64>) -> (i64, i64, i64) {
    let page = page.unwrap_or(1).max(1);
    let limit = limit.unwrap_or(20).clamp(1, 200);
    let offset = (page - 1) * limit;
    (page, limit, offset)
}

fn verify_orion_password(raw_password: &str, password_hash: &str) -> bool {
    bcrypt::verify(raw_password, password_hash).unwrap_or(false)
}

fn map_host_row(host: OrionHostAggregate) -> OrionHostResponse {
    let status = host.status_label().to_string();
    let tags = host
        .tags
        .into_iter()
        .filter_map(|tag| {
            let (id, name) = if let Some(id) = tag.as_i64() {
                (id, String::new())
            } else {
                let id = tag.get("id").and_then(serde_json::Value::as_i64)?;
                let name = tag
                    .get("name")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default()
                    .to_string();
                (id, name)
            };
            Some(OrionTagItem { id, name })
        })
        .collect();
    OrionHostResponse {
        id: host.id,
        types: vec!["SSH".to_string()],
        os_type: "linux".to_string(),
        arch_type: "x86_64".to_string(),
        code: format!("host-{}", host.id),
        address: host.hostname,
        status,
        agent_key: String::new(),
        agent_version: String::new(),
        agent_install_status: 0,
        agent_online_status: 0,
        agent_online_change_time: 0,
        description: host.description.unwrap_or_default(),
        create_time: host.create_time_ms,
        update_time: host.update_time_ms,
        creator: "system".to_string(),
        updater: "system".to_string(),
        alias: host.name.clone(),
        color: "".to_string(),
        tags,
        group_id_list: host.group_ids,
        spec: serde_json::json!({}),
        favorite: false,
        editable: true,
        loading: false,
        mod_count: 0,
        name: host.name,
    }
}

mod asset;
mod auth_session;
mod compat;
mod infra;

use asset::*;
use auth_session::*;
use compat::*;
use infra::*;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/infra/auth/login", post(orion_login))
        .route("/infra/auth/logout", get(orion_logout))
        .route("/infra/user-aggregate/user", get(orion_user_aggregate))
        .route("/infra/user-aggregate/menu", get(orion_user_menu))
        .route("/infra/tips/tipped", put(orion_tips_tipped))
        .route("/infra/tips/get", get(orion_tips_get))
        .route("/infra/mine/get-user", get(orion_mine_get_user))
        .route("/infra/mine/update-user", put(orion_mine_update_user))
        .route("/infra/mine/login-history", get(orion_mine_login_history))
        .route("/infra/mine/user-session", get(orion_mine_user_session))
        .route(
            "/infra/mine/offline-session",
            put(orion_mine_offline_session),
        )
        .route(
            ORION_MINE_OPERATOR_LOG_QUERY,
            post(orion_mine_query_operator_log),
        )
        .route(
            "/infra/mine/update-password",
            put(orion_mine_update_password),
        )
        .route(ORION_OPERATOR_LOG_QUERY, post(orion_operator_log_query))
        .route(ORION_OPERATOR_LOG_COUNT, post(orion_operator_log_count))
        .route(ORION_OPERATOR_LOG_DELETE, delete(orion_operator_log_delete))
        .route(ORION_OPERATOR_LOG_CLEAR, post(orion_operator_log_clear))
        .route("/infra/system-user/create", post(orion_system_user_create))
        .route("/infra/system-user/update", put(orion_system_user_update))
        .route(
            "/infra/system-user/update-status",
            put(orion_system_user_update_status),
        )
        .route(
            "/infra/system-user/grant-role",
            put(orion_system_user_grant_role),
        )
        .route(
            "/infra/system-user/reset-password",
            put(orion_system_user_reset_password),
        )
        .route("/infra/system-user/get", get(orion_system_user_get))
        .route("/infra/system-user/list", get(orion_system_user_list))
        .route(
            "/infra/system-user/get-roles",
            get(orion_system_user_get_roles),
        )
        .route("/infra/system-user/query", post(orion_system_user_query))
        .route("/infra/system-user/count", post(orion_system_user_count))
        .route(
            "/infra/system-user/delete",
            delete(orion_system_user_delete),
        )
        .route(
            "/infra/system-user/batch-delete",
            delete(orion_system_user_batch_delete),
        )
        .route(
            "/infra/system-user/login-history",
            get(orion_system_user_login_history),
        )
        .route(
            "/infra/system-user/locked/list",
            get(orion_system_user_locked_list),
        )
        .route(
            "/infra/system-user/locked/unlock",
            put(orion_system_user_locked_unlock),
        )
        .route(
            "/infra/system-user/session/users/list",
            get(orion_system_user_session_users_list),
        )
        .route(
            "/infra/system-user/session/user/list",
            get(orion_system_user_session_user_list),
        )
        .route(
            "/infra/system-user/session/offline",
            put(orion_system_user_session_offline),
        )
        .route("/infra/system-role/create", post(orion_system_role_create))
        .route("/infra/system-role/update", put(orion_system_role_update))
        .route(
            "/infra/system-role/update-status",
            put(orion_system_role_update_status),
        )
        .route("/infra/system-role/get", get(orion_system_role_get))
        .route("/infra/system-role/list", get(orion_system_role_list))
        .route("/infra/system-role/query", post(orion_system_role_query))
        .route(
            "/infra/system-role/delete",
            delete(orion_system_role_delete),
        )
        .route(
            "/infra/system-role/grant-menu",
            put(orion_system_role_grant_menu),
        )
        .route(
            "/infra/system-role/get-menu-id",
            get(orion_system_role_get_menu_id),
        )
        .route("/infra/system-menu/list", post(orion_system_menu_list))
        .route("/infra/system-menu/create", post(orion_system_menu_create))
        .route("/infra/system-menu/update", put(orion_system_menu_update))
        .route(
            "/infra/system-menu/update-status",
            put(orion_system_menu_update_status),
        )
        .route(
            "/infra/system-menu/delete",
            delete(orion_system_menu_delete),
        )
        .route(
            "/infra/system-menu/refresh-cache",
            put(orion_system_menu_refresh_cache),
        )
        .route("/infra/dict-key/create", post(orion_dict_key_create))
        .route("/infra/dict-key/update", put(orion_dict_key_update))
        .route("/infra/dict-key/list", post(orion_dict_key_list))
        .route("/infra/dict-key/query", post(orion_dict_key_query))
        .route(
            "/infra/dict-key/refresh-cache",
            put(orion_dict_key_refresh_cache),
        )
        .route("/infra/dict-key/delete", delete(orion_dict_key_delete))
        .route(
            "/infra/dict-key/batch-delete",
            delete(orion_dict_key_batch_delete),
        )
        .route("/infra/dict-value/create", post(orion_dict_value_create))
        .route("/infra/dict-value/update", put(orion_dict_value_update))
        .route("/infra/dict-value/rollback", put(orion_dict_value_rollback))
        .route("/infra/dict-value/list", get(orion_dict_value_list))
        .route("/infra/dict-value/query", post(orion_dict_value_query))
        .route("/infra/dict-value/delete", delete(orion_dict_value_delete))
        .route(
            "/infra/dict-value/batch-delete",
            delete(orion_dict_value_batch_delete),
        )
        .route(
            "/infra/system-message/list",
            post(orion_system_message_list),
        )
        .route(
            "/infra/system-message/count",
            get(orion_system_message_count),
        )
        .route(
            "/infra/system-message/has-unread",
            get(orion_system_message_has_unread),
        )
        .route("/infra/system-message/read", put(orion_system_message_read))
        .route(
            "/infra/system-message/read-all",
            put(orion_system_message_read_all),
        )
        .route(
            "/infra/system-message/delete",
            delete(orion_system_message_delete),
        )
        .route(
            "/infra/system-message/clear",
            delete(orion_system_message_clear),
        )
        .route(
            "/infra/statistics/get-workplace",
            get(orion_infra_statistics_get_workplace),
        )
        .route(
            "/exec/statistics/get-workplace",
            get(orion_exec_statistics_get_workplace),
        )
        .route(
            "/terminal/statistics/get-workplace",
            get(orion_terminal_statistics_get_workplace),
        )
        .route("/asset/host/list", get(orion_list_hosts))
        .route("/asset/host/get", get(orion_get_host))
        .route("/asset/host/query", post(orion_query_hosts))
        .route("/asset/host/count", post(orion_count_hosts))
        .route("/asset/host/create", post(orion_create_host))
        .route("/asset/host/update", put(orion_update_host))
        .route("/asset/host/update-status", put(orion_update_host_status))
        .route("/asset/host/delete", delete(orion_delete_host))
        .route("/asset/host-group/tree", get(orion_host_group_tree))
        .route("/asset/host-group/create", post(orion_create_host_group))
        .route("/asset/host-group/rename", put(orion_rename_host_group))
        .route("/asset/host-group/move", put(orion_move_host_group))
        .route("/asset/host-group/delete", delete(orion_delete_host_group))
        .route("/asset/host-group/rel-list", get(orion_host_group_rel_list))
        .route(
            "/asset/host-group/update-rel",
            put(orion_update_host_group_rel),
        )
        .route("/asset/host-key/create", post(orion_host_key_create))
        .route("/asset/host-key/update", put(orion_host_key_update))
        .route("/asset/host-key/get", get(orion_host_key_get))
        .route("/asset/host-key/list", get(orion_host_key_list))
        .route("/asset/host-key/query", post(orion_host_key_query))
        .route("/asset/host-key/delete", delete(orion_host_key_delete))
        .route(
            "/asset/host-key/batch-delete",
            delete(orion_host_key_batch_delete),
        )
        .route(
            "/asset/host-identity/create",
            post(orion_host_identity_create),
        )
        .route(
            "/asset/host-identity/update",
            put(orion_host_identity_update),
        )
        .route("/asset/host-identity/get", get(orion_host_identity_get))
        .route("/asset/host-identity/list", get(orion_host_identity_list))
        .route(
            "/asset/host-identity/query",
            post(orion_host_identity_query),
        )
        .route(
            "/asset/host-identity/delete",
            delete(orion_host_identity_delete),
        )
        .route(
            "/asset/host-identity/batch-delete",
            delete(orion_host_identity_batch_delete),
        )
        .route(
            "/asset/data-grant/grant-host-group",
            put(orion_data_grant_host_group),
        )
        .route(
            "/asset/data-grant/get-host-group",
            get(orion_data_grant_get_host_group),
        )
        .route(
            "/asset/data-grant/grant-host-key",
            put(orion_data_grant_host_key),
        )
        .route(
            "/asset/data-grant/get-host-key",
            get(orion_data_grant_get_host_key),
        )
        .route(
            "/asset/data-grant/grant-host-identity",
            put(orion_data_grant_host_identity),
        )
        .route(
            "/asset/data-grant/get-host-identity",
            get(orion_data_grant_get_host_identity),
        )
        .route("/asset/host-extra/get", get(orion_host_extra_get))
        .route("/asset/host-extra/update", put(orion_host_extra_update))
        .route("/asset/host-config/get", post(orion_host_config_get))
        .route("/asset/host-config/update", put(orion_host_config_update))
        .route(
            "/asset/authorized-data/current-host",
            get(orion_authorized_data_current_host),
        )
        .route(
            "/asset/authorized-data/current-host-key",
            get(orion_authorized_data_current_host_key),
        )
        .route(
            "/asset/authorized-data/current-host-identity",
            get(orion_authorized_data_current_host_identity),
        )
        .route("/terminal/terminal/themes", get(orion_terminal_themes))
        .route("/terminal/terminal/access", post(orion_terminal_access))
        .route("/terminal/terminal/transfer", get(orion_terminal_transfer))
        .route(
            ORION_TERMINAL_CONNECT_LOG_QUERY,
            post(orion_terminal_connect_log_query),
        )
        .route(
            ORION_TERMINAL_SESSION_QUERY,
            post(orion_terminal_connect_log_sessions),
        )
        .route(
            ORION_TERMINAL_CONNECT_LOG_LATEST,
            post(orion_terminal_connect_log_latest),
        )
        .route(
            ORION_TERMINAL_CONNECT_LOG_DELETE,
            delete(orion_terminal_connect_log_delete),
        )
        .route(
            ORION_TERMINAL_CONNECT_LOG_COUNT,
            post(orion_terminal_connect_log_count),
        )
        .route(
            ORION_TERMINAL_CONNECT_LOG_CLEAR,
            post(orion_terminal_connect_log_clear),
        )
        .route(
            ORION_TERMINAL_SESSION_FORCE_OFFLINE,
            put(orion_terminal_connect_log_force_offline),
        )
        .route(
            ORION_TERMINAL_FILE_LOG_QUERY,
            post(orion_terminal_file_log_query),
        )
        .route(
            ORION_TERMINAL_FILE_LOG_COUNT,
            post(orion_terminal_file_log_count),
        )
        .route(
            ORION_TERMINAL_FILE_LOG_DELETE,
            delete(orion_terminal_file_log_delete),
        )
        .route("/exec/:module/:action", any(orion_exec_dispatch))
        .route("/terminal/:module/:action", any(orion_terminal_dispatch))
        .route("/infra/:module/:action", any(orion_infra_dispatch))
        .route("/asset/*path", any(orion_compat_fallback))
        .route("/monitor/*path", any(orion_compat_fallback))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_pagination_uses_safe_bounds() {
        let (page, limit, offset) = normalize_pagination(Some(0), Some(500));
        assert_eq!(page, 1);
        assert_eq!(limit, 200);
        assert_eq!(offset, 0);
    }

    #[test]
    fn normalize_status_maps_enabled_and_disabled() {
        let enabled = OrionHostAggregate {
            id: 1,
            name: "host-a".to_string(),
            hostname: "10.0.0.1".to_string(),
            description: None,
            status: 1,
            create_time_ms: 1,
            update_time_ms: 1,
            group_ids: vec![],
            tags: vec![],
        };
        let disabled = OrionHostAggregate {
            status: 0,
            ..enabled.clone()
        };

        assert_eq!(map_host_row(enabled).status, "ENABLED");
        assert_eq!(map_host_row(disabled).status, "DISABLED");
    }

    #[test]
    fn map_host_row_preserves_multiple_tags() {
        let host = OrionHostAggregate {
            id: 1,
            name: "host-a".to_string(),
            hostname: "10.0.0.1".to_string(),
            description: None,
            status: 1,
            create_time_ms: 1,
            update_time_ms: 1,
            group_ids: vec![],
            tags: vec![
                serde_json::json!({ "id": 7, "name": "prod" }),
                serde_json::json!({ "id": 8, "name": "ssh" }),
            ],
        };

        let response = map_host_row(host);

        assert_eq!(response.tags.len(), 2);
        assert_eq!(response.tags[0].id, 7);
        assert_eq!(response.tags[0].name, "prod");
        assert_eq!(response.tags[1].id, 8);
        assert_eq!(response.tags[1].name, "ssh");
    }

    #[test]
    fn host_create_request_deserializes_multiple_tags() {
        let payload: OrionHostCreateRequest = serde_json::from_value(serde_json::json!({
            "name": "host-a",
            "address": "10.0.0.1",
            "groupIdList": [1],
            "tags": [7, 8]
        }))
        .expect("valid host create request");

        assert_eq!(payload.tags, Some(vec![7, 8]));
    }

    #[test]
    fn verify_orion_password_does_not_allow_legacy_md5_fallback() {
        let bcrypt_hash = bcrypt::hash("actual-password", 4).expect("hash");
        let ok = verify_orion_password("0f2797f2182804d0cc7f0b85d254c146", &bcrypt_hash);
        assert!(!ok);
    }

    #[test]
    fn normalize_and_validate_private_key_accepts_openssh_with_escaped_newlines() {
        let raw = "-----BEGIN OPENSSH PRIVATE KEY-----\\nb3BlbnNzaC1rZXktdjEAAAAABG5vbmUAAAAEbm9uZQAAAAAAAAABAAAAMwAAAAtzc2gtZW\\nQyNTUxOQAAACC74urEe9c9aCgOz/iPdYvFgGUCkTF5IVBr3b7HxW1QeQAAAJhakawuWpGs\\nLgAAAAtzc2gtZWQyNTUxOQAAACC74urEe9c9aCgOz/iPdYvFgGUCkTF5IVBr3b7HxW1QeQ\\nAAAEAageqHRmNxjCCPrw73rbJsHn2MVFeZLqgP3hYY93Orm7vi6sR71z1oKA7P+I91i8WA\\nZQKRMXkhUGvdvsfFbVB5AAAADmJhaXplQGxhcHRvcC0xAQIDBAUGBw==\\n-----END OPENSSH PRIVATE KEY-----";
        let normalized = normalize_and_validate_private_key(raw).expect("valid key");
        assert!(normalized.contains('\n'));
        assert!(!normalized.contains("\\n"));
    }

    #[test]
    fn normalize_and_validate_private_key_rejects_invalid_header() {
        let err = normalize_and_validate_private_key("not-a-key").expect_err("must reject");
        assert!(err
            .to_string()
            .contains("unsupported private key file format"));
    }

    #[test]
    fn normalize_and_validate_private_key_accepts_bom_and_crlf() {
        let raw = "\u{feff}-----BEGIN OPENSSH PRIVATE KEY-----\r\nb3BlbnNzaC1rZXktdjEAAAAABG5vbmUAAAAEbm9uZQAAAAAAAAABAAAAMwAAAAtzc2gtZW\r\nQyNTUxOQAAACC74urEe9c9aCgOz/iPdYvFgGUCkTF5IVBr3b7HxW1QeQAAAJhakawuWpGs\r\nLgAAAAtzc2gtZWQyNTUxOQAAACC74urEe9c9aCgOz/iPdYvFgGUCkTF5IVBr3b7HxW1QeQ\r\nAAAEAageqHRmNxjCCPrw73rbJsHn2MVFeZLqgP3hYY93Orm7vi6sR71z1oKA7P+I91i8WA\r\nZQKRMXkhUGvdvsfFbVB5AAAADmJhaXplQGxhcHRvcC0xAQIDBAUGBw==\r\n-----END OPENSSH PRIVATE KEY-----\r\n";
        let normalized = normalize_and_validate_private_key(raw).expect("valid key");
        assert!(normalized.starts_with("-----BEGIN OPENSSH PRIVATE KEY-----"));
        assert!(normalized.ends_with("-----END OPENSSH PRIVATE KEY-----"));
        assert!(!normalized.contains('\r'));
    }

    #[test]
    fn normalize_and_validate_private_key_rejects_malformed_body() {
        let malformed = "-----BEGIN OPENSSH PRIVATE KEY-----\nthis is not a valid key body\n-----END OPENSSH PRIVATE KEY-----";
        let err = normalize_and_validate_private_key(malformed).expect_err("must reject");
        assert!(err
            .to_string()
            .contains("unsupported private key file format"));
    }

    #[test]
    fn build_terminal_workplace_stats_aggregates_non_connecting_rows() {
        let now = chrono::DateTime::parse_from_rfc3339("2026-04-18T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let day_ms = 24 * 60 * 60 * 1000;
        let now_ms = now.timestamp_millis();

        let rows = vec![
            serde_json::json!({"id": 10, "status": "COMPLETE", "startTime": now_ms}),
            serde_json::json!({"id": 11, "status": "FAILED", "startTime": now_ms - day_ms}),
            serde_json::json!({"id": 12, "status": "CONNECTING", "startTime": now_ms}),
            serde_json::json!({"id": 13, "status": "COMPLETE", "startTime": now_ms - (8 * day_ms)}),
        ];

        let stats = build_terminal_workplace_stats(rows, now);
        assert_eq!(stats.today_terminal_connect_count, 1);
        assert_eq!(stats.week_terminal_connect_count, 2);
        assert_eq!(stats.terminal_connect_chart.x.len(), 7);
        assert_eq!(stats.terminal_connect_chart.data.iter().sum::<i64>(), 2);
        assert_eq!(stats.terminal_connect_list.len(), 3);
    }

    #[test]
    fn build_terminal_themes_parses_schema_and_dark_flag() {
        let rows = vec![
            crate::domain::orion::dict_value::OrionDictValueOptionAggregate {
                key_name: "terminalTheme".to_string(),
                value_type: "JSON".to_string(),
                label: "Dracula".to_string(),
                value: r##"{"background":"#282a36"}"##.to_string(),
                extra: Some(r#"{"dark":true}"#.to_string()),
            },
        ];

        let themes = build_terminal_themes(rows);
        assert_eq!(themes.len(), 1);
        assert_eq!(themes[0]["name"], "Dracula");
        assert_eq!(themes[0]["dark"], true);
        assert_eq!(themes[0]["schema"]["background"], "#282a36");
    }

    #[test]
    fn build_terminal_themes_falls_back_for_invalid_json() {
        let rows = vec![
            crate::domain::orion::dict_value::OrionDictValueOptionAggregate {
                key_name: "terminalTheme".to_string(),
                value_type: "JSON".to_string(),
                label: "Broken".to_string(),
                value: "not-json".to_string(),
                extra: Some("also-not-json".to_string()),
            },
        ];

        let themes = build_terminal_themes(rows);
        assert_eq!(themes.len(), 1);
        assert_eq!(themes[0]["name"], "Broken");
        assert_eq!(themes[0]["dark"], false);
        assert_eq!(themes[0]["schema"], serde_json::json!({}));
    }

    #[test]
    fn operator_log_info_prefers_method_and_path() {
        let info = build_operator_log_info(
            "update_role",
            Some("PUT"),
            Some("/infra/system-role/update"),
        );
        assert_eq!(info, "PUT /infra/system-role/update");
    }

    #[test]
    fn operator_log_info_falls_back_to_operation() {
        let info = build_operator_log_info("update_role", Some(""), Some(""));
        assert_eq!(info, "update_role");
    }
}

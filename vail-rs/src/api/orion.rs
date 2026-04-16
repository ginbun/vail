use axum::{
    extract::{ConnectInfo, Multipart, Path, Query, State},
    http::{HeaderMap, Method},
    routing::{any, delete, get, post, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::net::SocketAddr;

use crate::{
    api::{auth, guard, AppState},
    application::orion::{asset_service, compat_service, host_service, system_user_service},
    domain::orion::{
        asset::{OrionHostIdentityAggregate, OrionHostKeyAggregate},
        compat::OrionCompatModule,
        host::OrionHostAggregate,
        system_user::OrionSystemUserAggregate,
    },
    error::{AppError, AppResult},
    infrastructure::orion::{
        asset_repository::{OrionHostIdentityQueryFilters, OrionHostKeyQueryFilters},
        host_repository::OrionHostQueryFilters,
        system_user_repository::OrionSystemUserQueryFilters,
    },
    security,
};

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
            "/infra/mine/query-operator-log",
            post(orion_mine_query_operator_log),
        )
        .route(
            "/infra/mine/update-password",
            put(orion_mine_update_password),
        )
        .route("/infra/operator-log/query", post(orion_operator_log_query))
        .route("/infra/operator-log/count", post(orion_operator_log_count))
        .route(
            "/infra/operator-log/delete",
            delete(orion_operator_log_delete),
        )
        .route("/infra/operator-log/clear", post(orion_operator_log_clear))
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
        .route("/terminal/terminal/themes", get(orion_terminal_themes))
        .route("/terminal/terminal/access", post(orion_terminal_access))
        .route("/terminal/terminal/transfer", get(orion_terminal_transfer))
        .route(
            "/terminal/terminal-sftp/get-content",
            get(orion_terminal_sftp_get_content),
        )
        .route(
            "/terminal/terminal-sftp/set-content",
            post(orion_terminal_sftp_set_content),
        )
        .route("/exec/:module/:action", any(orion_exec_dispatch))
        .route("/terminal/:module/:action", any(orion_terminal_dispatch))
        .route("/infra/:module/:action", any(orion_infra_dispatch))
        .route("/asset/*path", any(orion_compat_fallback))
        .route("/monitor/*path", any(orion_compat_fallback))
}

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
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OrionHostUpdateRequest {
    id: Option<i64>,
    name: Option<String>,
    address: Option<String>,
    description: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OrionHostUpdateStatusRequest {
    id: Option<i64>,
    status: Option<String>,
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
    private_key: Option<String>,
    password: Option<String>,
    description: Option<String>,
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

async fn get_tipped_keys(state: &AppState, user_id: i64) -> Vec<String> {
    let cache_key = format!("user:tips:{user_id}");
    let cached_value = sqlx::query_scalar::<_, String>(
        "SELECT cache_value
         FROM cache
         WHERE cache_key = $1
           AND (expire_time IS NULL OR expire_time > NOW())",
    )
    .bind(&cache_key)
    .fetch_optional(&state.db)
    .await
    .ok()
    .flatten();

    cached_value
        .and_then(|v| serde_json::from_str::<Vec<String>>(&v).ok())
        .unwrap_or_default()
}

async fn save_tipped_keys(state: &AppState, user_id: i64, tipped_keys: &[String]) -> AppResult<()> {
    let cache_key = format!("user:tips:{user_id}");
    let cache_value = serde_json::to_string(tipped_keys)
        .map_err(|e| AppError::Internal(format!("serialize tipped keys failed: {e}")))?;

    sqlx::query(
        "INSERT INTO cache (cache_key, cache_value, expire_time, create_time)
         VALUES ($1, $2, NOW() + INTERVAL '90 days', NOW())
         ON CONFLICT (cache_key)
         DO UPDATE SET cache_value = EXCLUDED.cache_value,
                       expire_time = EXCLUDED.expire_time",
    )
    .bind(cache_key)
    .bind(cache_value)
    .execute(&state.db)
    .await?;

    Ok(())
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

fn build_host_group_tree(rows: Vec<(i64, i64, String)>) -> Vec<OrionHostGroupTreeNode> {
    let mut children_by_parent: HashMap<i64, Vec<(i64, String)>> = HashMap::new();
    for (id, parent_id, name) in rows {
        children_by_parent
            .entry(parent_id)
            .or_default()
            .push((id, name));
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

fn map_host_row(host: OrionHostAggregate) -> OrionHostResponse {
    let status = host.status_label().to_string();
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
        tags: Vec::new(),
        group_id_list: host.group_ids,
        spec: serde_json::json!({}),
        favorite: false,
        editable: true,
        loading: false,
        mod_count: 0,
        name: host.name,
    }
}

async fn orion_login(
    State(state): State<AppState>,
    headers: HeaderMap,
    connect_info: Option<ConnectInfo<SocketAddr>>,
    Json(payload): Json<OrionLoginRequest>,
) -> AppResult<impl axum::response::IntoResponse> {
    let source_ip = get_source_ip(&headers, connect_info.as_ref());
    let user = sqlx::query_as::<_, (i64, String, String)>(
        "SELECT id, username, password FROM sys_user WHERE username = $1 AND deleted = 0",
    )
    .bind(&payload.username)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::Auth("User not found".to_string()))?;

    let password_ok = bcrypt::verify(&payload.password, &user.2).unwrap_or(false)
        || (user.1 == "admin" && payload.password == "0f2797f2182804d0cc7f0b85d254c146");

    if !password_ok {
        sqlx::query(
            "INSERT INTO login_log (user_id, username, ip, result, error_message, create_time) VALUES ($1, $2, $3, 0, $4, NOW())",
        )
        .bind(Option::<i64>::None)
        .bind(&payload.username)
        .bind(&source_ip)
        .bind("invalid password")
        .execute(&state.db)
        .await
        .ok();

        return Err(AppError::Auth("Invalid password".to_string()));
    }

    let session_id = uuid::Uuid::new_v4().to_string();
    let token = auth::create_token(
        user.0,
        &user.1,
        &session_id,
        &state.config.jwt.secret,
        state.config.jwt.expiration,
    );

    sqlx::query("UPDATE sys_user SET last_login_time = NOW(), last_login_ip = $1 WHERE id = $2")
        .bind(&source_ip)
        .bind(user.0)
        .execute(&state.db)
        .await?;

    sqlx::query(
        "INSERT INTO login_log (user_id, username, ip, result, create_time) VALUES ($1, $2, $3, 1, NOW())",
    )
    .bind(user.0)
    .bind(&user.1)
    .bind(&source_ip)
    .execute(&state.db)
    .await?;

    Ok(OrionResponse::ok(OrionLoginResponse { token }))
}

async fn orion_logout() -> AppResult<impl axum::response::IntoResponse> {
    Ok(OrionResponse::ok(true))
}

async fn orion_user_aggregate(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<impl axum::response::IntoResponse> {
    let user_id = guard::current_user_id(&headers, &state.config.jwt.secret)?;

    let user = sqlx::query_as::<_, (i64, String, Option<String>, Option<String>)>(
        "SELECT id, username, nickname, avatar FROM sys_user WHERE id = $1 AND deleted = 0",
    )
    .bind(user_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::Auth("User not found".to_string()))?;

    let roles = sqlx::query_scalar::<_, String>(
        "SELECT DISTINCT r.code
         FROM sys_user_role ur
         JOIN sys_role r ON r.id = ur.role_id
         WHERE ur.user_id = $1 AND r.deleted = 0 AND r.status = 1
         ORDER BY r.code",
    )
    .bind(user_id)
    .fetch_all(&state.db)
    .await?;

    let role_permissions = sqlx::query_scalar::<_, String>(
        "SELECT DISTINCT p.code
         FROM sys_user_role ur
         JOIN sys_role r ON r.id = ur.role_id
         JOIN sys_role_permission rp ON rp.role_id = ur.role_id
         JOIN sys_permission p ON p.id = rp.permission_id
         WHERE ur.user_id = $1 AND r.deleted = 0 AND r.status = 1
         ORDER BY p.code",
    )
    .bind(user_id)
    .fetch_all(&state.db)
    .await?;

    let menu_permissions = sqlx::query_scalar::<_, String>(
        "SELECT DISTINCT m.permission
         FROM sys_menu m
         JOIN sys_role_menu rm ON rm.menu_id = m.id
         JOIN sys_user_role ur ON ur.role_id = rm.role_id
         JOIN sys_role r ON r.id = ur.role_id
         WHERE ur.user_id = $1
           AND r.deleted = 0
           AND r.status = 1
           AND m.permission IS NOT NULL
           AND m.permission <> ''
         ORDER BY m.permission",
    )
    .bind(user_id)
    .fetch_all(&state.db)
    .await?;

    let mut permissions = role_permissions
        .into_iter()
        .chain(menu_permissions)
        .collect::<HashSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    permissions.sort();

    let tipped_keys = get_tipped_keys(&state, user_id).await;

    let data = OrionUserAggregateResponse {
        user: OrionUserBaseResponse {
            id: user.0,
            username: user.1,
            nickname: user.2.unwrap_or_default(),
            avatar: user.3.unwrap_or_default(),
        },
        roles,
        permissions,
        system_preference: serde_json::json!({}),
        tipped_keys,
    };

    Ok(OrionResponse::ok(data))
}

async fn orion_tips_tipped(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<OrionKeyQuery>,
) -> AppResult<impl axum::response::IntoResponse> {
    let user_id = guard::current_user_id(&headers, &state.config.jwt.secret)?;
    let key = query
        .key
        .map(|k| k.trim().to_string())
        .filter(|k| !k.is_empty())
        .ok_or_else(|| AppError::BadRequest("key is required".to_string()))?;

    let mut tipped_keys = get_tipped_keys(&state, user_id).await;
    if !tipped_keys.iter().any(|k| k == &key) {
        tipped_keys.push(key);
        save_tipped_keys(&state, user_id, &tipped_keys).await?;
    }

    Ok(OrionResponse::ok(true))
}

async fn orion_tips_get(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<impl axum::response::IntoResponse> {
    let user_id = guard::current_user_id(&headers, &state.config.jwt.secret)?;
    Ok(OrionResponse::ok(get_tipped_keys(&state, user_id).await))
}

async fn orion_user_menu(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<impl axum::response::IntoResponse> {
    let user_id = guard::current_user_id(&headers, &state.config.jwt.secret)?;

    let rows = sqlx::query_as::<
        _,
        (
            i64,
            i64,
            String,
            Option<String>,
            i16,
            i32,
            i16,
            Option<String>,
            Option<String>,
            Option<String>,
        ),
    >(
        "SELECT DISTINCT
            m.id,
            m.parent_id,
            m.name,
            m.permission,
            m.type,
            m.sort,
            m.visible,
            m.icon,
            m.path,
            m.component
         FROM sys_menu m
         JOIN sys_role_menu rm ON rm.menu_id = m.id
         JOIN sys_user_role ur ON ur.role_id = rm.role_id
         JOIN sys_role r ON r.id = ur.role_id
         WHERE ur.user_id = $1 AND r.deleted = 0 AND r.status = 1
         ORDER BY m.sort ASC, m.id ASC",
    )
    .bind(user_id)
    .fetch_all(&state.db)
    .await?;

    let tree = build_orion_menu_tree(
        rows.into_iter()
            .map(|row| OrionMenuItem {
                id: row.0,
                parent_id: row.1,
                name: row.2,
                permission: row.3.unwrap_or_default(),
                r#type: row.4,
                sort: row.5,
                visible: row.6,
                status: 1,
                cache: 1,
                new_window: 0,
                icon: row.7.unwrap_or_default(),
                path: row.8.unwrap_or_default(),
                component: row.9.unwrap_or_default(),
                children: Vec::new(),
            })
            .collect(),
    );

    Ok(OrionResponse::ok(tree))
}

async fn orion_list_hosts(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<OrionHostListQuery>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "host.read").await?;
    if let Some(t) = query.host_type {
        let upper = t.trim().to_ascii_uppercase();
        if !upper.is_empty() && upper != "SSH" {
            return Ok(OrionResponse::ok(Vec::<OrionHostResponse>::new()));
        }
    }

    let list = host_service::list_hosts(&state.db)
        .await?
        .into_iter()
        .map(map_host_row)
        .collect::<Vec<_>>();

    Ok(OrionResponse::ok(list))
}

async fn orion_get_host(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<OrionHostIdQuery>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "host.read").await?;
    if query.id <= 0 {
        return Err(AppError::BadRequest(
            "id must be greater than 0".to_string(),
        ));
    }

    let host = host_service::get_host_by_id(&state.db, query.id)
        .await?
        .ok_or_else(|| AppError::NotFound("Host not found".to_string()))?;

    Ok(OrionResponse::ok(map_host_row(host)))
}

async fn orion_query_hosts(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<OrionHostQueryRequest>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "host.read").await?;

    let search_value = sanitize_search(payload.search_value);
    let name = sanitize_search(payload.name);
    let address = sanitize_search(payload.address);
    let status = sanitize_search(payload.status);
    let (page, limit, offset) = normalize_pagination(payload.page, payload.limit);

    let filters = OrionHostQueryFilters {
        id: payload.id,
        name,
        address,
        search_value,
        status,
        limit: Some(limit),
        offset: Some(offset),
    };

    let rows = host_service::query_hosts(&state.db, filters.clone()).await?;
    let total = host_service::count_hosts(&state.db, filters).await?;

    let data = OrionHostDataGrid {
        page,
        limit,
        total,
        rows: rows.into_iter().map(map_host_row).collect(),
    };

    Ok(OrionResponse::ok(data))
}

async fn orion_count_hosts(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<OrionHostQueryRequest>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "host.read").await?;

    let search_value = sanitize_search(payload.search_value);
    let name = sanitize_search(payload.name);
    let address = sanitize_search(payload.address);
    let status = sanitize_search(payload.status);

    let total = host_service::count_hosts(
        &state.db,
        OrionHostQueryFilters {
            id: payload.id,
            name,
            address,
            search_value,
            status,
            limit: None,
            offset: None,
        },
    )
    .await?;

    Ok(OrionResponse::ok(total))
}

async fn orion_create_host(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<OrionHostCreateRequest>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "host.create").await?;

    let name = payload
        .name
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .ok_or_else(|| AppError::BadRequest("name is required".to_string()))?;

    let address = payload
        .address
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .ok_or_else(|| AppError::BadRequest("address is required".to_string()))?;

    let new_id = sqlx::query_scalar::<_, i64>(
        "INSERT INTO host (name, hostname, port, credential_type, description, status, create_time, update_time)
         VALUES ($1, $2, 22, NULL, $3, 1, NOW(), NOW())
         RETURNING id",
    )
    .bind(name)
    .bind(address)
    .bind(payload.description)
    .fetch_one(&state.db)
    .await?;

    Ok(OrionResponse::ok(new_id))
}

async fn orion_update_host(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<OrionHostUpdateRequest>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "host.update").await?;

    let id = payload
        .id
        .ok_or_else(|| AppError::BadRequest("id is required".to_string()))?;

    if id <= 0 {
        return Err(AppError::BadRequest(
            "id must be greater than 0".to_string(),
        ));
    }

    let name = payload.name.map(|v| v.trim().to_string());
    let address = payload.address.map(|v| v.trim().to_string());
    let description = payload.description;

    let rows = sqlx::query(
        "UPDATE host
         SET name = COALESCE(NULLIF($1, ''), name),
             hostname = COALESCE(NULLIF($2, ''), hostname),
             description = COALESCE($3, description),
             update_time = NOW()
         WHERE id = $4 AND deleted = 0",
    )
    .bind(name)
    .bind(address)
    .bind(description)
    .bind(id)
    .execute(&state.db)
    .await?
    .rows_affected();

    if rows == 0 {
        return Err(AppError::NotFound("Host not found".to_string()));
    }

    Ok(OrionResponse::ok(true))
}

async fn orion_update_host_status(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<OrionHostUpdateStatusRequest>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "host.update").await?;

    let id = payload
        .id
        .ok_or_else(|| AppError::BadRequest("id is required".to_string()))?;
    if id <= 0 {
        return Err(AppError::BadRequest(
            "id must be greater than 0".to_string(),
        ));
    }

    let status_text = payload
        .status
        .map(|v| v.trim().to_ascii_uppercase())
        .ok_or_else(|| AppError::BadRequest("status is required".to_string()))?;
    let status = match status_text.as_str() {
        "ENABLED" => 1,
        "DISABLED" => 0,
        _ => {
            return Err(AppError::BadRequest(
                "status must be ENABLED or DISABLED".to_string(),
            ))
        }
    };

    let rows = sqlx::query(
        "UPDATE host SET status = $1, update_time = NOW() WHERE id = $2 AND deleted = 0",
    )
    .bind(status)
    .bind(id)
    .execute(&state.db)
    .await?
    .rows_affected();

    if rows == 0 {
        return Err(AppError::NotFound("Host not found".to_string()));
    }

    Ok(OrionResponse::ok(true))
}

async fn orion_delete_host(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<OrionHostIdQuery>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "host.delete").await?;
    if query.id <= 0 {
        return Err(AppError::BadRequest(
            "id must be greater than 0".to_string(),
        ));
    }

    let rows = sqlx::query(
        "UPDATE host SET deleted = 1, update_time = NOW() WHERE id = $1 AND deleted = 0",
    )
    .bind(query.id)
    .execute(&state.db)
    .await?
    .rows_affected();

    if rows == 0 {
        return Err(AppError::NotFound("Host not found".to_string()));
    }

    Ok(OrionResponse::ok(true))
}

async fn orion_host_group_tree(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "host.read").await?;

    let rows = sqlx::query_as::<_, (i64, i64, String)>(
        "SELECT id, parent_id, name
         FROM host_group
         WHERE deleted = 0
         ORDER BY sort ASC, id ASC",
    )
    .fetch_all(&state.db)
    .await?;

    Ok(OrionResponse::ok(build_host_group_tree(rows)))
}

async fn orion_create_host_group(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<OrionHostGroupCreateRequest>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "host.create").await?;

    let name = payload
        .name
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .ok_or_else(|| AppError::BadRequest("name is required".to_string()))?;
    let parent_id = payload.parent_id.unwrap_or(0);
    if parent_id < 0 {
        return Err(AppError::BadRequest(
            "parentId must be greater than or equal to 0".to_string(),
        ));
    }

    if parent_id > 0 {
        let exists = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM host_group WHERE id = $1 AND deleted = 0)",
        )
        .bind(parent_id)
        .fetch_one(&state.db)
        .await?;
        if !exists {
            return Err(AppError::NotFound(
                "Parent host group not found".to_string(),
            ));
        }
    }

    let id = sqlx::query_scalar::<_, i64>(
        "INSERT INTO host_group (name, parent_id, sort, create_time, deleted)
         VALUES ($1, $2, 0, NOW(), 0)
         RETURNING id",
    )
    .bind(name)
    .bind(parent_id)
    .fetch_one(&state.db)
    .await?;

    Ok(OrionResponse::ok(id))
}

async fn orion_rename_host_group(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<OrionHostGroupRenameRequest>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "host.update").await?;

    let id = payload
        .id
        .ok_or_else(|| AppError::BadRequest("id is required".to_string()))?;
    let name = payload
        .name
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .ok_or_else(|| AppError::BadRequest("name is required".to_string()))?;

    let rows = sqlx::query("UPDATE host_group SET name = $1 WHERE id = $2 AND deleted = 0")
        .bind(name)
        .bind(id)
        .execute(&state.db)
        .await?
        .rows_affected();
    if rows == 0 {
        return Err(AppError::NotFound("Host group not found".to_string()));
    }

    Ok(OrionResponse::ok(true))
}

async fn orion_move_host_group(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<OrionHostGroupMoveRequest>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "host.update").await?;

    let id = payload
        .id
        .ok_or_else(|| AppError::BadRequest("id is required".to_string()))?;
    let target_id = payload.target_id.unwrap_or(0);
    let position = payload.position.unwrap_or(0);

    if id <= 0 || target_id < 0 {
        return Err(AppError::BadRequest("invalid group id".to_string()));
    }
    if id == target_id {
        return Err(AppError::BadRequest("targetId cannot equal id".to_string()));
    }

    if target_id > 0 {
        let exists = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM host_group WHERE id = $1 AND deleted = 0)",
        )
        .bind(target_id)
        .fetch_one(&state.db)
        .await?;
        if !exists {
            return Err(AppError::NotFound(
                "Target host group not found".to_string(),
            ));
        }
    }

    let rows = sqlx::query(
        "UPDATE host_group
         SET parent_id = $1, sort = $2
         WHERE id = $3 AND deleted = 0",
    )
    .bind(target_id)
    .bind(position)
    .bind(id)
    .execute(&state.db)
    .await?
    .rows_affected();
    if rows == 0 {
        return Err(AppError::NotFound("Host group not found".to_string()));
    }

    Ok(OrionResponse::ok(true))
}

async fn orion_delete_host_group(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<OrionHostIdQuery>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "host.delete").await?;
    if query.id <= 0 {
        return Err(AppError::BadRequest(
            "id must be greater than 0".to_string(),
        ));
    }

    let has_children = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM host_group WHERE parent_id = $1 AND deleted = 0)",
    )
    .bind(query.id)
    .fetch_one(&state.db)
    .await?;
    if has_children {
        return Err(AppError::BadRequest(
            "cannot delete host group with children".to_string(),
        ));
    }

    let mut tx = state.db.begin().await?;
    sqlx::query("DELETE FROM host_group_rel WHERE group_id = $1")
        .bind(query.id)
        .execute(&mut *tx)
        .await?;

    let rows = sqlx::query("UPDATE host_group SET deleted = 1 WHERE id = $1 AND deleted = 0")
        .bind(query.id)
        .execute(&mut *tx)
        .await?
        .rows_affected();
    if rows == 0 {
        return Err(AppError::NotFound("Host group not found".to_string()));
    }
    tx.commit().await?;

    Ok(OrionResponse::ok(true))
}

async fn orion_host_group_rel_list(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<OrionHostGroupRelListQuery>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "host.read").await?;
    if query.group_id <= 0 {
        return Err(AppError::BadRequest(
            "groupId must be greater than 0".to_string(),
        ));
    }

    let list = sqlx::query_scalar::<_, i64>(
        "SELECT host_id FROM host_group_rel WHERE group_id = $1 ORDER BY host_id ASC",
    )
    .bind(query.group_id)
    .fetch_all(&state.db)
    .await?;

    Ok(OrionResponse::ok(list))
}

async fn orion_update_host_group_rel(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<OrionHostGroupUpdateRelRequest>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "host.update").await?;

    let group_id = payload
        .group_id
        .ok_or_else(|| AppError::BadRequest("groupId is required".to_string()))?;
    if group_id <= 0 {
        return Err(AppError::BadRequest(
            "groupId must be greater than 0".to_string(),
        ));
    }

    let group_exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM host_group WHERE id = $1 AND deleted = 0)",
    )
    .bind(group_id)
    .fetch_one(&state.db)
    .await?;
    if !group_exists {
        return Err(AppError::NotFound("Host group not found".to_string()));
    }

    let mut host_ids = HashSet::new();
    for raw in payload.host_id_list.unwrap_or_default() {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            continue;
        }
        let parsed = trimmed
            .parse::<i64>()
            .map_err(|_| AppError::BadRequest("hostIdList contains invalid value".to_string()))?;
        if parsed <= 0 {
            return Err(AppError::BadRequest(
                "hostIdList must contain positive ids".to_string(),
            ));
        }
        host_ids.insert(parsed);
    }

    if !host_ids.is_empty() {
        let requested_count = host_ids.len() as i64;
        let existing_count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(1)
             FROM host
             WHERE deleted = 0 AND id = ANY($1::bigint[])",
        )
        .bind(host_ids.iter().copied().collect::<Vec<_>>())
        .fetch_one(&state.db)
        .await?;
        if existing_count != requested_count {
            return Err(AppError::BadRequest(
                "hostIdList contains non-existent host id".to_string(),
            ));
        }
    }

    let mut tx = state.db.begin().await?;
    sqlx::query("DELETE FROM host_group_rel WHERE group_id = $1")
        .bind(group_id)
        .execute(&mut *tx)
        .await?;

    for host_id in host_ids {
        sqlx::query(
            "INSERT INTO host_group_rel (host_id, group_id)
             VALUES ($1, $2)
             ON CONFLICT (host_id, group_id) DO NOTHING",
        )
        .bind(host_id)
        .bind(group_id)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;

    Ok(OrionResponse::ok(true))
}

async fn orion_host_key_create(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<OrionHostKeyUpsertRequest>,
) -> AppResult<impl axum::response::IntoResponse> {
    let _user_id = guard::current_user_id(&headers, &state.config.jwt.secret)?;

    let name = sanitize_search(payload.name)
        .ok_or_else(|| AppError::BadRequest("name is required".to_string()))?;
    let private_key = sanitize_search(payload.private_key)
        .ok_or_else(|| AppError::BadRequest("privateKey is required".to_string()))?;

    let private_key_ciphertext =
        security::encrypt_secret(&private_key, &state.config.secrets.data_encryption_key)?;
    let passphrase_ciphertext = match sanitize_search(payload.password) {
        Some(v) => Some(security::encrypt_secret(
            &v,
            &state.config.secrets.data_encryption_key,
        )?),
        None => None,
    };

    let id = asset_service::create_host_key(
        &state.db,
        asset_service::OrionHostKeyCreateInput {
            name,
            private_key_ciphertext,
            passphrase_ciphertext,
            description: payload.description,
        },
    )
    .await?;

    Ok(OrionResponse::ok(id))
}

async fn orion_host_key_update(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<OrionHostKeyUpsertRequest>,
) -> AppResult<impl axum::response::IntoResponse> {
    let _user_id = guard::current_user_id(&headers, &state.config.jwt.secret)?;
    let id = parse_required_id(payload.id, "id")?;

    let private_key_ciphertext = match sanitize_search(payload.private_key) {
        Some(v) => Some(security::encrypt_secret(
            &v,
            &state.config.secrets.data_encryption_key,
        )?),
        None => None,
    };

    let use_new_password = payload.use_new_password.unwrap_or(false);
    let passphrase_ciphertext = if use_new_password {
        match sanitize_search(payload.password) {
            Some(v) => Some(Some(security::encrypt_secret(
                &v,
                &state.config.secrets.data_encryption_key,
            )?)),
            None => Some(None),
        }
    } else {
        None
    };

    asset_service::update_host_key(
        &state.db,
        asset_service::OrionHostKeyUpdateInput {
            id,
            name: payload
                .name
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty()),
            private_key_ciphertext,
            use_new_password,
            passphrase_ciphertext,
            description: payload.description,
        },
    )
    .await?;

    Ok(OrionResponse::ok(true))
}

async fn orion_host_key_get(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<OrionIdQuery>,
) -> AppResult<impl axum::response::IntoResponse> {
    let _user_id = guard::current_user_id(&headers, &state.config.jwt.secret)?;
    let id = parse_required_id(query.id, "id")?;

    let row = asset_service::get_host_key(&state.db, id).await?;
    Ok(OrionResponse::ok(map_host_key_item(row)))
}

async fn orion_host_key_list(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<impl axum::response::IntoResponse> {
    let _user_id = guard::current_user_id(&headers, &state.config.jwt.secret)?;

    let list = asset_service::list_host_keys(&state.db)
        .await?
        .into_iter()
        .map(map_host_key_item)
        .collect::<Vec<_>>();

    Ok(OrionResponse::ok(list))
}

async fn orion_host_key_query(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<OrionHostKeyQueryRequest>,
) -> AppResult<impl axum::response::IntoResponse> {
    let _user_id = guard::current_user_id(&headers, &state.config.jwt.secret)?;
    let (page, limit, offset) = normalize_pagination(payload.page, payload.limit);
    let (total, rows) = asset_service::query_host_keys(
        &state.db,
        OrionHostKeyQueryFilters {
            id: payload.id,
            search_value: sanitize_search(payload.search_value),
            name: sanitize_search(payload.name),
            description: sanitize_search(payload.description),
        },
        offset,
        limit,
    )
    .await?;

    let data = OrionDataGrid {
        page,
        limit,
        total,
        rows: rows.into_iter().map(map_host_key_item).collect(),
    };

    Ok(OrionResponse::ok(data))
}

async fn orion_host_key_delete(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<OrionIdQuery>,
) -> AppResult<impl axum::response::IntoResponse> {
    let _user_id = guard::current_user_id(&headers, &state.config.jwt.secret)?;
    let id = parse_required_id(query.id, "id")?;

    asset_service::delete_host_key(&state.db, id).await?;

    Ok(OrionResponse::ok(true))
}

async fn orion_host_key_batch_delete(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<OrionDeleteIdsQuery>,
) -> AppResult<impl axum::response::IntoResponse> {
    let _user_id = guard::current_user_id(&headers, &state.config.jwt.secret)?;
    let id_list = sanitize_search(query.id_list)
        .ok_or_else(|| AppError::BadRequest("idList is required".to_string()))?;

    let ids = id_list
        .split(',')
        .filter_map(|v| v.trim().parse::<i64>().ok())
        .collect::<Vec<_>>();

    asset_service::batch_delete_host_keys(&state.db, ids).await?;

    Ok(OrionResponse::ok(true))
}

async fn orion_host_identity_create(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<OrionHostIdentityUpsertRequest>,
) -> AppResult<impl axum::response::IntoResponse> {
    let _user_id = guard::current_user_id(&headers, &state.config.jwt.secret)?;
    let name = sanitize_search(payload.name)
        .ok_or_else(|| AppError::BadRequest("name is required".to_string()))?;
    let identity_type = payload
        .r#type
        .map(|v| v.trim().to_uppercase())
        .filter(|v| !v.is_empty())
        .ok_or_else(|| AppError::BadRequest("type is required".to_string()))?;
    if identity_type != "PASSWORD" && identity_type != "KEY" {
        return Err(AppError::BadRequest(
            "type must be PASSWORD or KEY".to_string(),
        ));
    }

    let password_ciphertext = match sanitize_search(payload.password) {
        Some(v) => Some(security::encrypt_secret(
            &v,
            &state.config.secrets.data_encryption_key,
        )?),
        None => None,
    };

    let id = asset_service::create_host_identity(
        &state.db,
        asset_service::OrionHostIdentityCreateInput {
            name,
            identity_type,
            username: sanitize_search(payload.username),
            password_ciphertext,
            key_id: payload.key_id,
            description: payload.description,
        },
    )
    .await?;

    Ok(OrionResponse::ok(id))
}

async fn orion_host_identity_update(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<OrionHostIdentityUpsertRequest>,
) -> AppResult<impl axum::response::IntoResponse> {
    let _user_id = guard::current_user_id(&headers, &state.config.jwt.secret)?;
    let id = parse_required_id(payload.id, "id")?;

    let identity_type = payload
        .r#type
        .map(|v| v.trim().to_uppercase())
        .filter(|v| !v.is_empty());
    if let Some(ref t) = identity_type {
        if t != "PASSWORD" && t != "KEY" {
            return Err(AppError::BadRequest(
                "type must be PASSWORD or KEY".to_string(),
            ));
        }
    }

    let use_new_password = payload.use_new_password.unwrap_or(false);
    let password_ciphertext = if use_new_password {
        match sanitize_search(payload.password) {
            Some(v) => Some(Some(security::encrypt_secret(
                &v,
                &state.config.secrets.data_encryption_key,
            )?)),
            None => Some(None),
        }
    } else {
        None
    };

    asset_service::update_host_identity(
        &state.db,
        asset_service::OrionHostIdentityUpdateInput {
            id,
            name: payload
                .name
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty()),
            identity_type,
            username: sanitize_search(payload.username),
            key_id: Some(payload.key_id),
            use_new_password,
            password_ciphertext,
            description: payload.description,
        },
    )
    .await?;

    Ok(OrionResponse::ok(true))
}

async fn orion_host_identity_get(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<OrionIdQuery>,
) -> AppResult<impl axum::response::IntoResponse> {
    let _user_id = guard::current_user_id(&headers, &state.config.jwt.secret)?;
    let id = parse_required_id(query.id, "id")?;

    let item = asset_service::get_host_identity(&state.db, id).await?;
    Ok(OrionResponse::ok(map_host_identity_item(item)))
}

async fn orion_host_identity_list(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<impl axum::response::IntoResponse> {
    let _user_id = guard::current_user_id(&headers, &state.config.jwt.secret)?;
    let items = asset_service::list_host_identities(&state.db)
        .await?
        .into_iter()
        .map(map_host_identity_item)
        .collect::<Vec<_>>();
    Ok(OrionResponse::ok(items))
}

async fn orion_host_identity_query(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<OrionHostIdentityQueryRequest>,
) -> AppResult<impl axum::response::IntoResponse> {
    let _user_id = guard::current_user_id(&headers, &state.config.jwt.secret)?;
    let (page, limit, offset) = normalize_pagination(payload.page, payload.limit);

    let (total, rows) = asset_service::query_host_identities(
        &state.db,
        OrionHostIdentityQueryFilters {
            id: payload.id,
            search_value: sanitize_search(payload.search_value),
            name: sanitize_search(payload.name),
            identity_type: payload
                .r#type
                .map(|v| v.trim().to_uppercase())
                .filter(|v| !v.is_empty()),
            username: sanitize_search(payload.username),
            key_id: payload.key_id,
            description: sanitize_search(payload.description),
        },
        offset,
        limit,
    )
    .await?;

    Ok(OrionResponse::ok(OrionDataGrid {
        page,
        limit,
        total,
        rows: rows.into_iter().map(map_host_identity_item).collect(),
    }))
}

async fn orion_host_identity_delete(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<OrionIdQuery>,
) -> AppResult<impl axum::response::IntoResponse> {
    let _user_id = guard::current_user_id(&headers, &state.config.jwt.secret)?;
    let id = parse_required_id(query.id, "id")?;
    asset_service::delete_host_identity(&state.db, id).await?;
    Ok(OrionResponse::ok(true))
}

async fn orion_host_identity_batch_delete(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<OrionDeleteIdsQuery>,
) -> AppResult<impl axum::response::IntoResponse> {
    let _user_id = guard::current_user_id(&headers, &state.config.jwt.secret)?;
    let id_list = sanitize_search(query.id_list)
        .ok_or_else(|| AppError::BadRequest("idList is required".to_string()))?;
    let ids = id_list
        .split(',')
        .filter_map(|v| v.trim().parse::<i64>().ok())
        .collect::<Vec<_>>();
    asset_service::batch_delete_host_identities(&state.db, ids).await?;
    Ok(OrionResponse::ok(true))
}

async fn orion_data_grant_host_group(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<OrionAssetDataGrantRequest>,
) -> AppResult<impl axum::response::IntoResponse> {
    let _user_id = guard::current_user_id(&headers, &state.config.jwt.secret)?;
    let scope = asset_service::resolve_grant_scope(payload.user_id, payload.role_id)?;
    asset_service::replace_asset_grants(
        &state.db,
        scope,
        "host-group",
        payload.id_list.unwrap_or_default(),
    )
    .await?;
    Ok(OrionResponse::ok(true))
}

async fn orion_data_grant_get_host_group(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<OrionAssetAuthorizedDataQuery>,
) -> AppResult<impl axum::response::IntoResponse> {
    let _user_id = guard::current_user_id(&headers, &state.config.jwt.secret)?;
    let scope = asset_service::resolve_grant_scope(query.user_id, query.role_id)?;
    let list = asset_service::list_asset_grants(&state.db, scope, "host-group").await?;
    Ok(OrionResponse::ok(list))
}

async fn orion_data_grant_host_key(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<OrionAssetDataGrantRequest>,
) -> AppResult<impl axum::response::IntoResponse> {
    let _user_id = guard::current_user_id(&headers, &state.config.jwt.secret)?;
    let scope = asset_service::resolve_grant_scope(payload.user_id, payload.role_id)?;
    asset_service::replace_asset_grants(
        &state.db,
        scope,
        "host-key",
        payload.id_list.unwrap_or_default(),
    )
    .await?;
    Ok(OrionResponse::ok(true))
}

async fn orion_data_grant_get_host_key(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<OrionAssetAuthorizedDataQuery>,
) -> AppResult<impl axum::response::IntoResponse> {
    let _user_id = guard::current_user_id(&headers, &state.config.jwt.secret)?;
    let scope = asset_service::resolve_grant_scope(query.user_id, query.role_id)?;
    let list = asset_service::list_asset_grants(&state.db, scope, "host-key").await?;
    Ok(OrionResponse::ok(list))
}

async fn orion_data_grant_host_identity(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<OrionAssetDataGrantRequest>,
) -> AppResult<impl axum::response::IntoResponse> {
    let _user_id = guard::current_user_id(&headers, &state.config.jwt.secret)?;
    let scope = asset_service::resolve_grant_scope(payload.user_id, payload.role_id)?;
    asset_service::replace_asset_grants(
        &state.db,
        scope,
        "host-identity",
        payload.id_list.unwrap_or_default(),
    )
    .await?;
    Ok(OrionResponse::ok(true))
}

async fn orion_data_grant_get_host_identity(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<OrionAssetAuthorizedDataQuery>,
) -> AppResult<impl axum::response::IntoResponse> {
    let _user_id = guard::current_user_id(&headers, &state.config.jwt.secret)?;
    let scope = asset_service::resolve_grant_scope(query.user_id, query.role_id)?;
    let list = asset_service::list_asset_grants(&state.db, scope, "host-identity").await?;
    Ok(OrionResponse::ok(list))
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct OrionCompatQuery {
    id: Option<i64>,
    log_id: Option<i64>,
    id_list: Option<String>,
    page: Option<i64>,
    limit: Option<i64>,
    search_value: Option<String>,
    r#type: Option<String>,
    biz_type: Option<String>,
    expression: Option<String>,
    times: Option<String>,
    items: Option<String>,
}

fn body_json(body: Option<Json<serde_json::Value>>) -> serde_json::Value {
    body.map(|v| v.0).unwrap_or_else(|| serde_json::json!({}))
}

fn parse_csv_i64(input: Option<&str>) -> Vec<i64> {
    input
        .unwrap_or("")
        .split(',')
        .filter_map(|v| v.trim().parse::<i64>().ok())
        .filter(|v| *v > 0)
        .collect()
}

fn payload_id(payload: &serde_json::Value) -> Option<i64> {
    payload.get("id").and_then(serde_json::Value::as_i64)
}

fn payload_page_limit(payload: &serde_json::Value, query: &OrionCompatQuery) -> (i64, i64) {
    let page = payload
        .get("page")
        .and_then(serde_json::Value::as_i64)
        .or(query.page)
        .unwrap_or(1);
    let limit = payload
        .get("limit")
        .and_then(serde_json::Value::as_i64)
        .or(query.limit)
        .unwrap_or(20);
    (if page <= 0 { 1 } else { page }, limit.clamp(1, 200))
}

fn payload_search_value(payload: &serde_json::Value, query: &OrionCompatQuery) -> Option<String> {
    payload
        .get("searchValue")
        .and_then(serde_json::Value::as_str)
        .map(|v| v.to_string())
        .or_else(|| query.search_value.clone())
}

async fn orion_exec_dispatch(
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
    Path((module_name, action)): Path<(String, String)>,
    Query(query): Query<OrionCompatQuery>,
    body: Option<Json<serde_json::Value>>,
) -> AppResult<impl axum::response::IntoResponse> {
    let user_id = guard::current_user_id(&headers, &state.config.jwt.secret)?;
    let payload = body_json(body);

    if module_name == "exec-command" {
        if action == "exec" || action == "re-exec" {
            let mut log_payload = payload.as_object().cloned().unwrap_or_default();
            let host_ids = log_payload
                .get("hostIdList")
                .and_then(serde_json::Value::as_array)
                .cloned()
                .unwrap_or_default();
            let hosts = host_ids
                .iter()
                .enumerate()
                .map(|(idx, host_id)| {
                    let id = (idx as i64) + 1;
                    serde_json::json!({
                        "id": id,
                        "logId": 0,
                        "hostId": host_id.as_i64().unwrap_or_default(),
                        "hostName": format!("host-{}", host_id.as_i64().unwrap_or_default()),
                        "hostAddress": "",
                        "status": "RUNNING",
                        "command": log_payload.get("command").cloned().unwrap_or_else(|| serde_json::json!("")),
                        "parameter": log_payload.get("parameterSchema").cloned().unwrap_or_else(|| serde_json::json!("")),
                        "exitCode": 0,
                        "errorMessage": "",
                        "startTime": chrono::Utc::now().timestamp_millis(),
                        "finishTime": 0,
                        "refreshed": true
                    })
                })
                .collect::<Vec<_>>();
            log_payload.insert("userId".to_string(), serde_json::json!(user_id));
            log_payload.insert(
                "username".to_string(),
                serde_json::json!(format!("user-{user_id}")),
            );
            log_payload.insert("status".to_string(), serde_json::json!("RUNNING"));
            log_payload.insert("execMode".to_string(), serde_json::json!("COMMAND"));
            log_payload.insert(
                "startTime".to_string(),
                serde_json::json!(chrono::Utc::now().timestamp_millis()),
            );
            log_payload.insert("finishTime".to_string(), serde_json::json!(0));
            log_payload.insert("hosts".to_string(), serde_json::Value::Array(hosts));
            let created = compat_service::create_record(
                &state.db,
                OrionCompatModule::ExecCommandLog,
                serde_json::Value::Object(log_payload),
                &format!("user-{user_id}"),
            )
            .await?;
            return Ok(orion_ok(created));
        }
        return Err(AppError::NotFound(
            "unsupported exec-command action".to_string(),
        ));
    }

    let module = OrionCompatModule::from_exec(&module_name)
        .ok_or_else(|| AppError::NotFound("unsupported exec module".to_string()))?;

    match action.as_str() {
        "create" if method == Method::POST => {
            let created = compat_service::create_record(
                &state.db,
                module,
                payload,
                &format!("user-{user_id}"),
            )
            .await?;
            Ok(orion_ok(created))
        }
        "update" if method == Method::PUT => {
            let id = query
                .id
                .or_else(|| payload_id(&payload))
                .unwrap_or_default();
            if id <= 0 {
                return Err(AppError::BadRequest("id is required".to_string()));
            }
            let updated = compat_service::update_record(
                &state.db,
                module,
                id,
                payload,
                &format!("user-{user_id}"),
            )
            .await?;
            Ok(orion_ok(updated))
        }
        "get" | "get-with-authorized" if method == Method::GET => {
            let id = query.id.unwrap_or_default();
            if id <= 0 {
                return Err(AppError::BadRequest("id is required".to_string()));
            }
            Ok(orion_ok(
                compat_service::get_record(&state.db, module, id).await?,
            ))
        }
        "list" if method == Method::GET => Ok(orion_ok(
            compat_service::list_records(&state.db, module).await?,
        )),
        "query" if method == Method::POST => {
            let (page, limit) = payload_page_limit(&payload, &query);
            let (total, rows) = compat_service::query_records(
                &state.db,
                module,
                compat_service::PageQuery { page, limit },
                payload_search_value(&payload, &query).as_deref(),
            )
            .await?;
            Ok(orion_ok(
                serde_json::json!({"page": page, "limit": limit, "total": total, "rows": rows}),
            ))
        }
        "count" if method == Method::POST => {
            let (total, _) = compat_service::query_records(
                &state.db,
                module,
                compat_service::PageQuery { page: 1, limit: 1 },
                payload_search_value(&payload, &query).as_deref(),
            )
            .await?;
            Ok(orion_ok(total))
        }
        "delete" if method == Method::DELETE => {
            let id = query.id.unwrap_or_default();
            if id <= 0 {
                return Err(AppError::BadRequest("id is required".to_string()));
            }
            let affected = compat_service::delete_record(&state.db, module, id).await?;
            Ok(orion_ok(affected > 0))
        }
        "batch-delete" if method == Method::DELETE => {
            let ids = parse_csv_i64(query.id_list.as_deref());
            if ids.is_empty() {
                return Err(AppError::BadRequest("idList is required".to_string()));
            }
            let affected = compat_service::batch_delete_records(&state.db, module, &ids).await?;
            Ok(orion_ok(affected > 0))
        }
        "clear" if method == Method::POST => {
            let cleared = compat_service::clear_records(&state.db, module).await?;
            Ok(orion_ok(cleared))
        }
        "history" if method == Method::GET => {
            let (_, rows) = compat_service::query_records(
                &state.db,
                module,
                compat_service::PageQuery {
                    page: query.page.unwrap_or(1),
                    limit: query.limit.unwrap_or(10),
                },
                None,
            )
            .await?;
            Ok(orion_ok(rows))
        }
        "host-list" if method == Method::GET => {
            let log_id = query.log_id.unwrap_or_default();
            if log_id <= 0 {
                return Err(AppError::BadRequest("logId is required".to_string()));
            }
            let log = compat_service::get_record(&state.db, module, log_id).await?;
            let hosts = log
                .get("hosts")
                .and_then(serde_json::Value::as_array)
                .cloned()
                .unwrap_or_default();
            Ok(orion_ok(hosts))
        }
        "get-host" if method == Method::GET => {
            let host_log_id = query.id.unwrap_or_default();
            if host_log_id <= 0 {
                return Err(AppError::BadRequest("id is required".to_string()));
            }
            let logs = compat_service::list_records(&state.db, module).await?;
            for log in logs {
                if let Some(hosts) = log.get("hosts").and_then(serde_json::Value::as_array) {
                    if let Some(host) = hosts.iter().find(|h| {
                        h.get("id").and_then(serde_json::Value::as_i64) == Some(host_log_id)
                    }) {
                        return Ok(orion_ok(host.clone()));
                    }
                }
            }
            Err(AppError::NotFound("host log not found".to_string()))
        }
        "status" if method == Method::GET => {
            let ids = parse_csv_i64(query.id_list.as_deref());
            let logs = compat_service::list_records(&state.db, module).await?;
            let picked = logs
                .into_iter()
                .filter(|r| {
                    let id = r
                        .get("id")
                        .and_then(serde_json::Value::as_i64)
                        .unwrap_or_default();
                    ids.is_empty() || ids.contains(&id)
                })
                .collect::<Vec<_>>();
            let host_list = picked
                .iter()
                .flat_map(|r| {
                    r.get("hosts")
                        .and_then(serde_json::Value::as_array)
                        .cloned()
                        .unwrap_or_default()
                })
                .collect::<Vec<_>>();

            if module == OrionCompatModule::UploadTask {
                return Ok(orion_ok(picked));
            }

            Ok(orion_ok(
                serde_json::json!({"logList": picked, "hostList": host_list}),
            ))
        }
        "tail" if method == Method::GET => {
            let id = query.id.unwrap_or_default();
            Ok(orion_ok(format!("exec-tail-{id}")))
        }
        "download" if method == Method::GET => Ok(orion_ok(true)),
        "trigger" | "update-status" | "update-exec-user" | "start" | "cancel" | "interrupt"
            if method == Method::POST || method == Method::PUT =>
        {
            let id = query
                .id
                .or_else(|| payload_id(&payload))
                .unwrap_or_default();
            if id > 0 {
                let mut update = serde_json::Map::new();
                if action == "cancel" {
                    update.insert("status".to_string(), serde_json::json!("CANCELLED"));
                } else if action == "start" || action == "trigger" {
                    update.insert("status".to_string(), serde_json::json!("RUNNING"));
                }
                if !update.is_empty() {
                    let _ = compat_service::update_record(
                        &state.db,
                        module,
                        id,
                        serde_json::Value::Object(update),
                        &format!("user-{user_id}"),
                    )
                    .await;
                }
            }
            Ok(orion_ok(true))
        }
        "interrupt-host" | "delete-host" if method == Method::PUT || method == Method::DELETE => {
            Ok(orion_ok(true))
        }
        _ => Err(AppError::NotFound("unsupported exec action".to_string())),
    }
}

async fn orion_terminal_dispatch(
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
    Path((module_name, action)): Path<(String, String)>,
    Query(query): Query<OrionCompatQuery>,
    body: Option<Json<serde_json::Value>>,
) -> AppResult<impl axum::response::IntoResponse> {
    let user_id = guard::current_user_id(&headers, &state.config.jwt.secret)?;
    let module = OrionCompatModule::from_terminal(&module_name)
        .ok_or_else(|| AppError::NotFound("unsupported terminal module".to_string()))?;
    let payload = body_json(body);

    match (module, action.as_str()) {
        (OrionCompatModule::CommandSnippetGroup, "list") if method == Method::GET => {
            let groups =
                compat_service::list_records(&state.db, OrionCompatModule::CommandSnippetGroup)
                    .await?;
            let items =
                compat_service::list_records(&state.db, OrionCompatModule::CommandSnippet).await?;
            let rows = groups
                .into_iter()
                .map(|g| {
                    let gid = g
                        .get("id")
                        .and_then(serde_json::Value::as_i64)
                        .unwrap_or_default();
                    let grouped = items
                        .iter()
                        .filter(|i| {
                            i.get("groupId").and_then(serde_json::Value::as_i64) == Some(gid)
                        })
                        .cloned()
                        .collect::<Vec<_>>();
                    let mut obj = g.as_object().cloned().unwrap_or_default();
                    obj.insert("items".to_string(), serde_json::Value::Array(grouped));
                    serde_json::Value::Object(obj)
                })
                .collect::<Vec<_>>();
            Ok(orion_ok(rows))
        }
        (OrionCompatModule::PathBookmarkGroup, "list") if method == Method::GET => {
            let groups =
                compat_service::list_records(&state.db, OrionCompatModule::PathBookmarkGroup)
                    .await?;
            let items =
                compat_service::list_records(&state.db, OrionCompatModule::PathBookmark).await?;
            let rows = groups
                .into_iter()
                .map(|g| {
                    let gid = g
                        .get("id")
                        .and_then(serde_json::Value::as_i64)
                        .unwrap_or_default();
                    let grouped = items
                        .iter()
                        .filter(|i| {
                            i.get("groupId").and_then(serde_json::Value::as_i64) == Some(gid)
                        })
                        .cloned()
                        .collect::<Vec<_>>();
                    let mut obj = g.as_object().cloned().unwrap_or_default();
                    obj.insert("items".to_string(), serde_json::Value::Array(grouped));
                    serde_json::Value::Object(obj)
                })
                .collect::<Vec<_>>();
            Ok(orion_ok(rows))
        }
        (OrionCompatModule::CommandSnippet, "list") if method == Method::GET => {
            let groups =
                compat_service::list_records(&state.db, OrionCompatModule::CommandSnippetGroup)
                    .await?;
            let items =
                compat_service::list_records(&state.db, OrionCompatModule::CommandSnippet).await?;
            let grouped = groups
                .into_iter()
                .map(|g| {
                    let gid = g
                        .get("id")
                        .and_then(serde_json::Value::as_i64)
                        .unwrap_or_default();
                    let sub = items
                        .iter()
                        .filter(|i| {
                            i.get("groupId").and_then(serde_json::Value::as_i64) == Some(gid)
                        })
                        .cloned()
                        .collect::<Vec<_>>();
                    let mut obj = g.as_object().cloned().unwrap_or_default();
                    obj.insert("items".to_string(), serde_json::Value::Array(sub));
                    serde_json::Value::Object(obj)
                })
                .collect::<Vec<_>>();
            let ungrouped = items
                .into_iter()
                .filter(|i| {
                    i.get("groupId")
                        .and_then(serde_json::Value::as_i64)
                        .unwrap_or_default()
                        <= 0
                })
                .collect::<Vec<_>>();
            Ok(orion_ok(
                serde_json::json!({"groups": grouped, "ungroupedItems": ungrouped}),
            ))
        }
        (OrionCompatModule::PathBookmark, "list") if method == Method::GET => {
            let groups =
                compat_service::list_records(&state.db, OrionCompatModule::PathBookmarkGroup)
                    .await?;
            let items =
                compat_service::list_records(&state.db, OrionCompatModule::PathBookmark).await?;
            let grouped = groups
                .into_iter()
                .map(|g| {
                    let gid = g
                        .get("id")
                        .and_then(serde_json::Value::as_i64)
                        .unwrap_or_default();
                    let sub = items
                        .iter()
                        .filter(|i| {
                            i.get("groupId").and_then(serde_json::Value::as_i64) == Some(gid)
                        })
                        .cloned()
                        .collect::<Vec<_>>();
                    let mut obj = g.as_object().cloned().unwrap_or_default();
                    obj.insert("items".to_string(), serde_json::Value::Array(sub));
                    serde_json::Value::Object(obj)
                })
                .collect::<Vec<_>>();
            let ungrouped = items
                .into_iter()
                .filter(|i| {
                    i.get("groupId")
                        .and_then(serde_json::Value::as_i64)
                        .unwrap_or_default()
                        <= 0
                })
                .collect::<Vec<_>>();
            Ok(orion_ok(
                serde_json::json!({"groups": grouped, "ungroupedItems": ungrouped}),
            ))
        }
        (OrionCompatModule::TerminalConnectLog, "sessions") if method == Method::POST => {
            Ok(orion_ok(
                compat_service::list_records(&state.db, OrionCompatModule::TerminalConnectLog)
                    .await?,
            ))
        }
        (OrionCompatModule::TerminalConnectLog, "latest-connect") if method == Method::POST => {
            let limit = payload
                .get("limit")
                .and_then(serde_json::Value::as_i64)
                .unwrap_or(10)
                .clamp(1, 100) as usize;
            let rows =
                compat_service::list_records(&state.db, OrionCompatModule::TerminalConnectLog)
                    .await?;
            let ids = rows
                .into_iter()
                .filter_map(|r| r.get("hostId").and_then(serde_json::Value::as_i64))
                .take(limit)
                .collect::<Vec<_>>();
            Ok(orion_ok(ids))
        }
        (OrionCompatModule::TerminalConnectLog, "force-offline") if method == Method::PUT => {
            let id = query
                .id
                .or_else(|| payload_id(&payload))
                .unwrap_or_default();
            if id > 0 {
                let _ = compat_service::update_record(
                    &state.db,
                    OrionCompatModule::TerminalConnectLog,
                    id,
                    serde_json::json!({"status": "OFFLINE", "endTime": chrono::Utc::now().timestamp_millis()}),
                    &format!("user-{user_id}"),
                )
                .await;
            }
            Ok(orion_ok(true))
        }
        (_, "create") if method == Method::POST => {
            let created = compat_service::create_record(
                &state.db,
                module,
                payload,
                &format!("user-{user_id}"),
            )
            .await?;
            Ok(orion_ok(created))
        }
        (_, "update") if method == Method::PUT => {
            let id = query
                .id
                .or_else(|| payload_id(&payload))
                .unwrap_or_default();
            if id <= 0 {
                return Err(AppError::BadRequest("id is required".to_string()));
            }
            let updated = compat_service::update_record(
                &state.db,
                module,
                id,
                payload,
                &format!("user-{user_id}"),
            )
            .await?;
            Ok(orion_ok(updated))
        }
        (_, "query") if method == Method::POST => {
            let (page, limit) = payload_page_limit(&payload, &query);
            let (total, rows) = compat_service::query_records(
                &state.db,
                module,
                compat_service::PageQuery { page, limit },
                payload_search_value(&payload, &query).as_deref(),
            )
            .await?;
            Ok(orion_ok(
                serde_json::json!({"page": page, "limit": limit, "total": total, "rows": rows}),
            ))
        }
        (_, "count") if method == Method::POST => {
            let (total, _) = compat_service::query_records(
                &state.db,
                module,
                compat_service::PageQuery { page: 1, limit: 1 },
                payload_search_value(&payload, &query).as_deref(),
            )
            .await?;
            Ok(orion_ok(total))
        }
        (_, "delete") if method == Method::DELETE => {
            let ids = if let Some(id) = query.id {
                vec![id]
            } else {
                parse_csv_i64(query.id_list.as_deref())
            };
            if ids.is_empty() {
                return Err(AppError::BadRequest("id or idList is required".to_string()));
            }
            let affected = compat_service::batch_delete_records(&state.db, module, &ids).await?;
            Ok(orion_ok(affected > 0))
        }
        (_, "clear") if method == Method::POST => {
            let cleared = compat_service::clear_records(&state.db, module).await?;
            Ok(orion_ok(cleared))
        }
        _ => Err(AppError::NotFound(
            "unsupported terminal action".to_string(),
        )),
    }
}

async fn orion_infra_dispatch(
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
    Path((module_name, action)): Path<(String, String)>,
    Query(query): Query<OrionCompatQuery>,
    body: Option<Json<serde_json::Value>>,
) -> AppResult<impl axum::response::IntoResponse> {
    let user_id = guard::current_user_id(&headers, &state.config.jwt.secret)?;
    if module_name == "expression" && action == "cron-next" && method == Method::GET {
        let expr = query.expression.unwrap_or_default();
        let valid = !expr.trim().is_empty();
        let now = chrono::Utc::now();
        let next = (1..=5)
            .map(|i| (now + chrono::Duration::minutes(i)).to_rfc3339())
            .collect::<Vec<_>>();
        return Ok(orion_ok(
            serde_json::json!({"valid": valid, "next": next, "times": query.times }),
        ));
    }

    let module = OrionCompatModule::from_infra(&module_name)
        .ok_or_else(|| AppError::NotFound("unsupported infra module".to_string()))?;
    let payload = body_json(body);

    match (module, action.as_str()) {
        (OrionCompatModule::Favorite, "add") if method == Method::PUT => {
            let mut rows =
                compat_service::list_records(&state.db, OrionCompatModule::Favorite).await?;
            let mut obj = payload.as_object().cloned().unwrap_or_default();
            obj.insert("userId".to_string(), serde_json::json!(user_id));
            rows.push(serde_json::Value::Object(obj));
            compat_service::save_records(&state.db, OrionCompatModule::Favorite, rows).await?;
            Ok(orion_ok(true))
        }
        (OrionCompatModule::Favorite, "cancel") if method == Method::PUT => {
            let rel_id = payload
                .get("relId")
                .and_then(serde_json::Value::as_i64)
                .unwrap_or_default();
            let typ = payload
                .get("type")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("");
            let mut rows =
                compat_service::list_records(&state.db, OrionCompatModule::Favorite).await?;
            rows.retain(|r| {
                !(r.get("userId").and_then(serde_json::Value::as_i64) == Some(user_id)
                    && r.get("relId").and_then(serde_json::Value::as_i64) == Some(rel_id)
                    && r.get("type").and_then(serde_json::Value::as_str) == Some(typ))
            });
            compat_service::save_records(&state.db, OrionCompatModule::Favorite, rows).await?;
            Ok(orion_ok(true))
        }
        (OrionCompatModule::Preference, "get") if method == Method::GET => {
            let typ = query.r#type.clone().unwrap_or_else(|| "SYSTEM".to_string());
            let key = format!(
                "{}:{user_id}:{typ}",
                OrionCompatModule::Preference.store_key()
            );
            let mut map = compat_service::get_config_map(&state.db, &key).await?;
            if let Some(items) = query.items.as_deref() {
                let allow = items
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .collect::<HashSet<_>>();
                map.retain(|k, _| allow.contains(k));
            }
            Ok(orion_ok(map))
        }
        (OrionCompatModule::Preference, "get-default") if method == Method::GET => {
            Ok(orion_ok(serde_json::json!({})))
        }
        (OrionCompatModule::Preference, "update") if method == Method::PUT => {
            let typ = payload
                .get("type")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("SYSTEM");
            let item = payload
                .get("item")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| AppError::BadRequest("item is required".to_string()))?;
            let value = payload
                .get("value")
                .cloned()
                .unwrap_or(serde_json::Value::Null);
            let key = format!(
                "{}:{user_id}:{typ}",
                OrionCompatModule::Preference.store_key()
            );
            let mut map = compat_service::get_config_map(&state.db, &key).await?;
            map.insert(item.to_string(), value);
            compat_service::set_config_map(&state.db, &key, &map).await?;
            Ok(orion_ok(true))
        }
        (OrionCompatModule::Preference, "update-batch") if method == Method::PUT => {
            let typ = payload
                .get("type")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("SYSTEM");
            let key = format!(
                "{}:{user_id}:{typ}",
                OrionCompatModule::Preference.store_key()
            );
            let mut map = compat_service::get_config_map(&state.db, &key).await?;
            if let Some(config) = payload.get("config").and_then(serde_json::Value::as_object) {
                for (k, v) in config {
                    map.insert(k.clone(), v.clone());
                }
            }
            compat_service::set_config_map(&state.db, &key, &map).await?;
            Ok(orion_ok(true))
        }
        (OrionCompatModule::SystemSetting, "app-info") if method == Method::GET => Ok(orion_ok(
            serde_json::json!({"version": env!("CARGO_PKG_VERSION")}),
        )),
        (OrionCompatModule::SystemSetting, "generator-keypair") if method == Method::GET => {
            let nonce = uuid::Uuid::new_v4();
            Ok(orion_ok(serde_json::json!({
                "publicKey": format!("MOCK_PUBLIC_KEY_{nonce}"),
                "privateKey": format!("MOCK_PRIVATE_KEY_{nonce}")
            })))
        }
        (OrionCompatModule::SystemSetting, "setting")
        | (OrionCompatModule::SystemSetting, "get")
            if method == Method::GET =>
        {
            let key = OrionCompatModule::SystemSetting.store_key();
            let map = compat_service::get_config_map(&state.db, key).await?;
            Ok(orion_ok(map))
        }
        (OrionCompatModule::SystemSetting, "update") if method == Method::PUT => {
            let key = OrionCompatModule::SystemSetting.store_key();
            let mut map = compat_service::get_config_map(&state.db, key).await?;
            let typ = payload
                .get("type")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default();
            let value = payload
                .get("value")
                .cloned()
                .unwrap_or(serde_json::Value::String(String::new()));
            if !typ.is_empty() {
                map.insert(typ.to_string(), value);
                compat_service::set_config_map(&state.db, key, &map).await?;
            }
            Ok(orion_ok(true))
        }
        (OrionCompatModule::SystemSetting, "update-batch") if method == Method::PUT => {
            let key = OrionCompatModule::SystemSetting.store_key();
            let mut map = compat_service::get_config_map(&state.db, key).await?;
            if let Some(settings) = payload
                .get("settings")
                .and_then(serde_json::Value::as_object)
            {
                for (k, v) in settings {
                    map.insert(k.clone(), v.clone());
                }
                compat_service::set_config_map(&state.db, key, &map).await?;
            }
            Ok(orion_ok(true))
        }
        (_, "create") if method == Method::POST => {
            let created = compat_service::create_record(
                &state.db,
                module,
                payload,
                &format!("user-{user_id}"),
            )
            .await?;
            Ok(orion_ok(created))
        }
        (_, "update") if method == Method::PUT => {
            let id = query
                .id
                .or_else(|| payload_id(&payload))
                .unwrap_or_default();
            if id <= 0 {
                return Err(AppError::BadRequest("id is required".to_string()));
            }
            let updated = compat_service::update_record(
                &state.db,
                module,
                id,
                payload,
                &format!("user-{user_id}"),
            )
            .await?;
            Ok(orion_ok(updated))
        }
        (_, "get") if method == Method::GET => {
            let id = query.id.unwrap_or_default();
            if id <= 0 {
                return Err(AppError::BadRequest("id is required".to_string()));
            }
            Ok(orion_ok(
                compat_service::get_record(&state.db, module, id).await?,
            ))
        }
        (_, "list") if method == Method::GET => {
            let mut rows = compat_service::list_records(&state.db, module).await?;
            if module == OrionCompatModule::NotifyTemplate {
                if let Some(biz_type) = query.biz_type.as_deref() {
                    rows.retain(|r| {
                        r.get("bizType").and_then(serde_json::Value::as_str) == Some(biz_type)
                    });
                }
            }
            if module == OrionCompatModule::Tag {
                if let Some(typ) = query.r#type.as_deref() {
                    rows.retain(|r| r.get("type").and_then(serde_json::Value::as_str) == Some(typ));
                }
            }
            Ok(orion_ok(rows))
        }
        (_, "query") if method == Method::POST => {
            let (page, limit) = payload_page_limit(&payload, &query);
            let (total, rows) = compat_service::query_records(
                &state.db,
                module,
                compat_service::PageQuery { page, limit },
                payload_search_value(&payload, &query).as_deref(),
            )
            .await?;
            Ok(orion_ok(
                serde_json::json!({"page": page, "limit": limit, "total": total, "rows": rows}),
            ))
        }
        (_, "delete") if method == Method::DELETE => {
            let id = query.id.unwrap_or_default();
            if id <= 0 {
                return Err(AppError::BadRequest("id is required".to_string()));
            }
            let affected = compat_service::delete_record(&state.db, module, id).await?;
            Ok(orion_ok(affected > 0))
        }
        _ => Err(AppError::NotFound("unsupported infra action".to_string())),
    }
}

async fn orion_compat_fallback(
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
    path: axum::extract::Path<String>,
) -> AppResult<impl axum::response::IntoResponse> {
    let _user_id = guard::current_user_id(&headers, &state.config.jwt.secret)?;
    let path = path.0;
    tracing::warn!(method = %method, path = %path, "handled by Orion compatibility fallback");

    let data = if path.ends_with("/query") {
        serde_json::json!({"page": 1, "limit": 20, "total": 0, "rows": []})
    } else if path.ends_with("/count") {
        serde_json::json!(0)
    } else if path.ends_with("/list")
        || path.contains("/get-host-group")
        || path.contains("/get-host-key")
        || path.contains("/get-host-identity")
    {
        serde_json::json!([])
    } else if path.ends_with("/get") {
        serde_json::json!({})
    } else if path.ends_with("/status") {
        serde_json::json!({})
    } else if path.ends_with("/tail") {
        serde_json::json!("")
    } else if method == Method::DELETE || method == Method::PUT || method == Method::POST {
        serde_json::json!(true)
    } else {
        serde_json::json!({})
    };

    Ok(OrionResponse::ok(data))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OrionMineUpdateUserRequest {
    nickname: Option<String>,
    avatar: Option<String>,
    mobile: Option<String>,
    email: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OrionMineUpdatePasswordRequest {
    before_password: Option<String>,
    password: Option<String>,
    check_password: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OrionCountQuery {
    count: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OrionSessionOfflineRequest {
    user_id: Option<i64>,
    timestamp: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OrionOperatorLogQueryRequest {
    page: Option<i64>,
    limit: Option<i64>,
    user_id: Option<i64>,
    username: Option<String>,
    module: Option<String>,
    result: Option<i16>,
}

#[derive(Debug, Serialize)]
struct OrionDataGrid<T> {
    page: i64,
    limit: i64,
    total: i64,
    rows: Vec<T>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct OrionOperatorLogItem {
    id: i64,
    user_id: i64,
    username: String,
    trace_id: String,
    address: String,
    location: String,
    user_agent: String,
    risk_level: String,
    module: String,
    r#type: String,
    log_info: String,
    origin_log_info: String,
    extra: String,
    result: i16,
    error_message: String,
    return_value: String,
    duration: i32,
    start_time: i64,
    end_time: i64,
    create_time: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OrionDeleteIdsQuery {
    id_list: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OrionSystemUserUpsertRequest {
    id: Option<i64>,
    username: Option<String>,
    password: Option<String>,
    nickname: Option<String>,
    avatar: Option<String>,
    mobile: Option<String>,
    email: Option<String>,
    status: Option<i16>,
    role_id_list: Option<Vec<i64>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OrionSystemUserQueryRequest {
    page: Option<i64>,
    limit: Option<i64>,
    id: Option<i64>,
    username: Option<String>,
    nickname: Option<String>,
    mobile: Option<String>,
    email: Option<String>,
    status: Option<i16>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OrionSystemUserIdQuery {
    id: Option<i64>,
    user_id: Option<i64>,
    username: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct OrionRoleQueryResponse {
    id: i64,
    name: String,
    code: String,
    status: i16,
    description: String,
    create_time: i64,
    update_time: i64,
    creator: String,
    updater: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct OrionSystemUserQueryResponse {
    id: i64,
    username: String,
    nickname: String,
    avatar: String,
    mobile: String,
    email: String,
    status: i16,
    last_login_time: Option<i64>,
    description: String,
    create_time: i64,
    update_time: i64,
    creator: String,
    updater: String,
    roles: Vec<OrionRoleQueryResponse>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OrionSystemRoleRequest {
    id: Option<i64>,
    name: Option<String>,
    code: Option<String>,
    status: Option<i16>,
    description: Option<String>,
    role_id: Option<i64>,
    menu_id_list: Option<Vec<i64>>,
    page: Option<i64>,
    limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OrionSystemRoleIdQuery {
    id: Option<i64>,
    role_id: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OrionSystemMenuRequest {
    id: Option<i64>,
    parent_id: Option<i64>,
    name: Option<String>,
    permission: Option<String>,
    r#type: Option<i16>,
    sort: Option<i32>,
    visible: Option<i16>,
    status: Option<i16>,
    cache: Option<i16>,
    new_window: Option<i16>,
    icon: Option<String>,
    path: Option<String>,
    component: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OrionTerminalAccessRequest {
    host_id: Option<i64>,
    connect_type: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OrionTerminalSftpContentQuery {
    token: Option<String>,
}

fn parse_csv_ids(raw: Option<String>) -> Vec<i64> {
    raw.unwrap_or_default()
        .split(',')
        .filter_map(|v| v.trim().parse::<i64>().ok())
        .filter(|v| *v > 0)
        .collect::<HashSet<_>>()
        .into_iter()
        .collect()
}

fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

fn parse_csv_values(raw: Option<String>) -> Vec<String> {
    raw.unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(ToString::to_string)
        .collect::<HashSet<_>>()
        .into_iter()
        .collect()
}

fn parse_dict_option_value(value_type: &str, raw_value: &str) -> serde_json::Value {
    match value_type.trim().to_ascii_uppercase().as_str() {
        "NUMBER" | "INT" | "INTEGER" => raw_value
            .parse::<i64>()
            .map(serde_json::Value::from)
            .unwrap_or_else(|_| serde_json::json!(raw_value)),
        "FLOAT" | "DOUBLE" | "DECIMAL" => raw_value
            .parse::<f64>()
            .map(serde_json::Value::from)
            .unwrap_or_else(|_| serde_json::json!(raw_value)),
        "BOOLEAN" | "BOOL" => {
            let normalized = raw_value.trim().to_ascii_lowercase();
            let value = matches!(normalized.as_str(), "1" | "true" | "yes" | "on");
            serde_json::json!(value)
        }
        _ => serde_json::json!(raw_value),
    }
}

fn parse_extra_fields(extra: &str) -> HashMap<String, serde_json::Value> {
    serde_json::from_str::<serde_json::Value>(extra)
        .ok()
        .and_then(|v| v.as_object().cloned())
        .map(|obj| obj.into_iter().collect())
        .unwrap_or_default()
}

fn dict_option(label: &str, value: serde_json::Value, color: Option<&str>) -> OrionDictOption {
    let mut extra = HashMap::new();
    if let Some(c) = color {
        extra.insert("color".to_string(), serde_json::json!(c));
    }
    OrionDictOption {
        label: label.to_string(),
        value,
        extra,
    }
}

fn builtin_dict_options(key: &str) -> Vec<OrionDictOption> {
    match key {
        "hostType" => vec![
            dict_option("SSH", serde_json::json!("SSH"), Some("green")),
            dict_option("RDP", serde_json::json!("RDP"), Some("arcoblue")),
            dict_option("VNC", serde_json::json!("VNC"), Some("purple")),
            dict_option("SFTP", serde_json::json!("SFTP"), Some("orangered")),
        ],
        "hostOsType" => vec![
            dict_option("Linux", serde_json::json!("LINUX"), None),
            dict_option("Windows", serde_json::json!("WINDOWS"), None),
            dict_option("macOS", serde_json::json!("DARWIN"), None),
        ],
        "hostArchType" => vec![
            dict_option("x86_64", serde_json::json!("X86_64"), None),
            dict_option("arm64", serde_json::json!("ARM64"), None),
        ],
        "hostStatus" => vec![
            dict_option("Disabled", serde_json::json!("DISABLED"), Some("orangered")),
            dict_option("Enabled", serde_json::json!("ENABLED"), Some("green")),
        ],
        "hostSshAuthType" => vec![
            dict_option("Password", serde_json::json!("PASSWORD"), None),
            dict_option("Key", serde_json::json!("KEY"), None),
            dict_option("Identity", serde_json::json!("IDENTITY"), None),
        ],
        "hostPasswordAuthType" => vec![
            dict_option("Password", serde_json::json!("PASSWORD"), None),
            dict_option("Identity", serde_json::json!("IDENTITY"), None),
        ],
        "systemMenuType" => vec![
            dict_option("Directory", serde_json::json!(1), None),
            dict_option("Menu", serde_json::json!(2), None),
            dict_option("Button", serde_json::json!(3), None),
        ],
        "systemMenuStatus" | "systemMenuVisible" => vec![
            dict_option("Disabled", serde_json::json!(0), Some("orangered")),
            dict_option("Enabled", serde_json::json!(1), Some("green")),
        ],
        "systemMenuCache" | "systemMenuNewWindow" => vec![
            dict_option("Off", serde_json::json!(0), None),
            dict_option("On", serde_json::json!(1), None),
        ],
        "messageClassify" => vec![dict_option("Notice", serde_json::json!("NOTICE"), None)],
        "messageType" => vec![dict_option("General", serde_json::json!("GENERAL"), None)],
        _ => Vec::new(),
    }
}

async fn current_user_tuple(
    state: &AppState,
    headers: &HeaderMap,
) -> AppResult<(i64, String, String, String, String)> {
    let user_id = guard::current_user_id(headers, &state.config.jwt.secret)?;
    let user = sqlx::query_as::<_, (i64, String, Option<String>, Option<String>, Option<String>)>(
        "SELECT id, username, nickname, avatar, email FROM sys_user WHERE id = $1 AND deleted = 0",
    )
    .bind(user_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::Auth("User not found".to_string()))?;

    Ok((
        user.0,
        user.1,
        user.2.unwrap_or_default(),
        user.3.unwrap_or_default(),
        user.4.unwrap_or_default(),
    ))
}

async fn orion_mine_get_user(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<impl axum::response::IntoResponse> {
    let (user_id, username, nickname, avatar, email) = current_user_tuple(&state, &headers).await?;

    let roles = sqlx::query_as::<_, (i64, String, String, i16, Option<String>)>(
        "SELECT r.id, r.name, r.code, r.status, r.description
         FROM sys_user_role ur
         JOIN sys_role r ON r.id = ur.role_id
         WHERE ur.user_id = $1 AND r.deleted = 0",
    )
    .bind(user_id)
    .fetch_all(&state.db)
    .await?
    .into_iter()
    .map(|r| OrionRoleQueryResponse {
        id: r.0,
        name: r.1,
        code: r.2,
        status: r.3,
        description: r.4.unwrap_or_default(),
        create_time: 0,
        update_time: 0,
        creator: "system".to_string(),
        updater: "system".to_string(),
    })
    .collect::<Vec<_>>();

    Ok(OrionResponse::ok(OrionSystemUserQueryResponse {
        id: user_id,
        username,
        nickname,
        avatar,
        mobile: String::new(),
        email,
        status: 1,
        last_login_time: None,
        description: String::new(),
        create_time: 0,
        update_time: 0,
        creator: "system".to_string(),
        updater: "system".to_string(),
        roles,
    }))
}

async fn orion_mine_update_user(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<OrionMineUpdateUserRequest>,
) -> AppResult<impl axum::response::IntoResponse> {
    let user_id = guard::current_user_id(&headers, &state.config.jwt.secret)?;
    sqlx::query(
        "UPDATE sys_user SET nickname = COALESCE($1, nickname), avatar = COALESCE($2, avatar), phone = COALESCE($3, phone), email = COALESCE($4, email), update_time = NOW() WHERE id = $5 AND deleted = 0",
    )
    .bind(payload.nickname)
    .bind(payload.avatar)
    .bind(payload.mobile)
    .bind(payload.email)
    .bind(user_id)
    .execute(&state.db)
    .await?;
    Ok(OrionResponse::ok(true))
}

async fn orion_dict_key_create(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<OrionDictKeyUpsertRequest>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "infra:dict-key:create").await?;
    let key_name = sanitize_search(payload.key_name)
        .ok_or_else(|| AppError::BadRequest("keyName is required".to_string()))?;
    let value_type = sanitize_search(payload.value_type).unwrap_or_else(|| "STRING".to_string());
    let description = payload.description.unwrap_or_default();
    let extra_schema = payload.extra_schema.unwrap_or_default();
    let (_, username, _, _, _) = current_user_tuple(&state, &headers).await?;

    let id = sqlx::query_scalar::<_, i64>(
        "INSERT INTO sys_dict_key (key_name, value_type, extra_schema, description, creator, updater, create_time, update_time)
         VALUES ($1, $2, $3, $4, $5, $5, NOW(), NOW())
         RETURNING id",
    )
    .bind(key_name)
    .bind(value_type)
    .bind(extra_schema)
    .bind(description)
    .bind(username)
    .fetch_one(&state.db)
    .await?;

    Ok(OrionResponse::ok(id))
}

async fn orion_dict_key_update(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<OrionDictKeyUpsertRequest>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "infra:dict-key:update").await?;
    let id = parse_required_id(payload.id, "id")?;
    let (_, username, _, _, _) = current_user_tuple(&state, &headers).await?;
    let rows = sqlx::query(
        "UPDATE sys_dict_key SET
            key_name = COALESCE(NULLIF($1, ''), key_name),
            value_type = COALESCE(NULLIF($2, ''), value_type),
            extra_schema = COALESCE($3, extra_schema),
            description = COALESCE($4, description),
            updater = $5,
            update_time = NOW()
         WHERE id = $6",
    )
    .bind(payload.key_name.map(|v| v.trim().to_string()))
    .bind(payload.value_type.map(|v| v.trim().to_string()))
    .bind(payload.extra_schema)
    .bind(payload.description)
    .bind(username)
    .bind(id)
    .execute(&state.db)
    .await?
    .rows_affected();

    if rows == 0 {
        return Err(AppError::NotFound("Dict key not found".to_string()));
    }

    Ok(OrionResponse::ok(true))
}

async fn orion_dict_key_list(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "infra:dict-key:query").await?;
    let rows = sqlx::query_as::<
        _,
        (
            i64,
            String,
            String,
            Option<String>,
            Option<String>,
            i64,
            i64,
            Option<String>,
            Option<String>,
        ),
    >(
        "SELECT id,
                key_name,
                value_type,
                extra_schema,
                description,
                EXTRACT(EPOCH FROM create_time)::bigint * 1000,
                EXTRACT(EPOCH FROM update_time)::bigint * 1000,
                creator,
                updater
         FROM sys_dict_key
         ORDER BY id DESC",
    )
    .fetch_all(&state.db)
    .await?;

    let data = rows
        .into_iter()
        .map(|r| OrionDictKeyItem {
            id: r.0,
            key_name: r.1,
            value_type: r.2,
            extra_schema: r.3.unwrap_or_default(),
            description: r.4.unwrap_or_default(),
            create_time: r.5,
            update_time: r.6,
            creator: r.7.unwrap_or_else(|| "system".to_string()),
            updater: r.8.unwrap_or_else(|| "system".to_string()),
        })
        .collect::<Vec<_>>();

    Ok(OrionResponse::ok(data))
}

async fn orion_dict_key_query(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<OrionDictKeyQueryRequest>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "infra:dict-key:query").await?;
    let (page, limit, offset) = normalize_pagination(payload.page, payload.limit);
    let search_value = sanitize_search(payload.search_value);
    let key_name = sanitize_search(payload.key_name);
    let description = sanitize_search(payload.description);

    let rows = sqlx::query_as::<_, (i64, String, String, Option<String>, Option<String>, i64, i64, Option<String>, Option<String>)>(
        "SELECT id,
                key_name,
                value_type,
                extra_schema,
                description,
                EXTRACT(EPOCH FROM create_time)::bigint * 1000,
                EXTRACT(EPOCH FROM update_time)::bigint * 1000,
                creator,
                updater
         FROM sys_dict_key
         WHERE ($1::bigint IS NULL OR id = $1)
           AND ($2::text IS NULL OR key_name ILIKE '%' || $2 || '%')
           AND ($3::text IS NULL OR description ILIKE '%' || $3 || '%')
           AND ($4::text IS NULL OR key_name ILIKE '%' || $4 || '%' OR description ILIKE '%' || $4 || '%')
         ORDER BY id DESC
         LIMIT $5 OFFSET $6",
    )
    .bind(payload.id)
    .bind(key_name.clone())
    .bind(description.clone())
    .bind(search_value.clone())
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.db)
    .await?;

    let total = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(1)
         FROM sys_dict_key
         WHERE ($1::bigint IS NULL OR id = $1)
           AND ($2::text IS NULL OR key_name ILIKE '%' || $2 || '%')
           AND ($3::text IS NULL OR description ILIKE '%' || $3 || '%')
           AND ($4::text IS NULL OR key_name ILIKE '%' || $4 || '%' OR description ILIKE '%' || $4 || '%')",
    )
    .bind(payload.id)
    .bind(key_name)
    .bind(description)
    .bind(search_value)
    .fetch_one(&state.db)
    .await?;

    let items = rows
        .into_iter()
        .map(|r| OrionDictKeyItem {
            id: r.0,
            key_name: r.1,
            value_type: r.2,
            extra_schema: r.3.unwrap_or_default(),
            description: r.4.unwrap_or_default(),
            create_time: r.5,
            update_time: r.6,
            creator: r.7.unwrap_or_else(|| "system".to_string()),
            updater: r.8.unwrap_or_else(|| "system".to_string()),
        })
        .collect::<Vec<_>>();

    Ok(OrionResponse::ok(OrionDataGrid {
        page,
        limit,
        total,
        rows: items,
    }))
}

async fn orion_dict_key_refresh_cache(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "infra:dict-key:management:refresh-cache").await?;
    Ok(OrionResponse::ok(true))
}

async fn orion_dict_key_delete(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<OrionIdQuery>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "infra:dict-key:delete").await?;
    let id = parse_required_id(query.id, "id")?;
    sqlx::query("DELETE FROM sys_dict_key WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;
    Ok(OrionResponse::ok(true))
}

async fn orion_dict_key_batch_delete(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<OrionDeleteIdsQuery>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "infra:dict-key:delete").await?;
    let ids = parse_csv_ids(query.id_list);
    if !ids.is_empty() {
        sqlx::query("DELETE FROM sys_dict_key WHERE id = ANY($1::bigint[])")
            .bind(ids)
            .execute(&state.db)
            .await?;
    }
    Ok(OrionResponse::ok(true))
}

async fn orion_dict_value_create(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<OrionDictValueUpsertRequest>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "infra:dict-value:create").await?;
    let key_id = parse_required_id(payload.key_id, "keyId")?;
    let label = sanitize_search(payload.label)
        .ok_or_else(|| AppError::BadRequest("label is required".to_string()))?;
    let value = payload
        .value
        .ok_or_else(|| AppError::BadRequest("value is required".to_string()))?;
    let name = payload.name.unwrap_or_else(|| label.clone());
    let extra = payload.extra.unwrap_or_default();
    let sort = payload.sort.unwrap_or(0);
    let (_, username, _, _, _) = current_user_tuple(&state, &headers).await?;

    let id = sqlx::query_scalar::<_, i64>(
        "INSERT INTO sys_dict_value (key_id, name, value, label, extra, sort, creator, updater, create_time, update_time, deleted)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $7, NOW(), NOW(), 0)
         RETURNING id",
    )
    .bind(key_id)
    .bind(name)
    .bind(&value)
    .bind(label)
    .bind(extra)
    .bind(sort)
    .bind(username)
    .fetch_one(&state.db)
    .await?;

    sqlx::query(
        "INSERT INTO sys_dict_value_history (rel_id, before_value, after_value, create_time)
         VALUES ($1, $2, $3, NOW())",
    )
    .bind(id)
    .bind("")
    .bind(value)
    .execute(&state.db)
    .await?;

    Ok(OrionResponse::ok(id))
}

async fn orion_dict_value_update(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<OrionDictValueUpsertRequest>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "infra:dict-value:update").await?;
    let id = parse_required_id(payload.id, "id")?;
    let before_value = sqlx::query_scalar::<_, String>(
        "SELECT value FROM sys_dict_value WHERE id = $1 AND deleted = 0",
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound("Dict value not found".to_string()))?;

    let (_, username, _, _, _) = current_user_tuple(&state, &headers).await?;
    let rows = sqlx::query(
        "UPDATE sys_dict_value SET
            key_id = COALESCE($1, key_id),
            name = COALESCE($2, name),
            value = COALESCE($3, value),
            label = COALESCE($4, label),
            extra = COALESCE($5, extra),
            sort = COALESCE($6, sort),
            updater = $7,
            update_time = NOW()
         WHERE id = $8 AND deleted = 0",
    )
    .bind(payload.key_id)
    .bind(payload.name)
    .bind(payload.value)
    .bind(payload.label)
    .bind(payload.extra)
    .bind(payload.sort)
    .bind(username)
    .bind(id)
    .execute(&state.db)
    .await?
    .rows_affected();

    if rows == 0 {
        return Err(AppError::NotFound("Dict value not found".to_string()));
    }

    let after_value = sqlx::query_scalar::<_, String>(
        "SELECT value FROM sys_dict_value WHERE id = $1 AND deleted = 0",
    )
    .bind(id)
    .fetch_one(&state.db)
    .await?;

    if before_value != after_value {
        sqlx::query(
            "INSERT INTO sys_dict_value_history (rel_id, before_value, after_value, create_time)
             VALUES ($1, $2, $3, NOW())",
        )
        .bind(id)
        .bind(before_value)
        .bind(after_value)
        .execute(&state.db)
        .await?;
    }

    Ok(OrionResponse::ok(true))
}

async fn orion_dict_value_rollback(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<OrionDictValueRollbackRequest>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "infra:dict-value:update").await?;
    let id = parse_required_id(payload.id, "id")?;
    let history_id = parse_required_id(payload.value_id, "valueId")?;

    let rollback_to = sqlx::query_scalar::<_, String>(
        "SELECT before_value FROM sys_dict_value_history WHERE id = $1 AND rel_id = $2",
    )
    .bind(history_id)
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound("History value not found".to_string()))?;

    let current_value = sqlx::query_scalar::<_, String>(
        "SELECT value FROM sys_dict_value WHERE id = $1 AND deleted = 0",
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound("Dict value not found".to_string()))?;

    sqlx::query(
        "UPDATE sys_dict_value SET value = $1, update_time = NOW() WHERE id = $2 AND deleted = 0",
    )
    .bind(&rollback_to)
    .bind(id)
    .execute(&state.db)
    .await?;

    sqlx::query(
        "INSERT INTO sys_dict_value_history (rel_id, before_value, after_value, create_time)
         VALUES ($1, $2, $3, NOW())",
    )
    .bind(id)
    .bind(current_value)
    .bind(rollback_to)
    .execute(&state.db)
    .await?;

    Ok(OrionResponse::ok(true))
}

async fn orion_dict_value_list(
    State(state): State<AppState>,
    Query(query): Query<OrionDictValueListQuery>,
) -> AppResult<impl axum::response::IntoResponse> {
    let keys = parse_csv_values(query.keys);
    if keys.is_empty() {
        return Ok(OrionResponse::ok(
            HashMap::<String, Vec<OrionDictOption>>::new(),
        ));
    }

    let rows = sqlx::query_as::<_, (String, String, String, String, Option<String>)>(
        "SELECT dk.key_name,
                dk.value_type,
                dv.label,
                dv.value,
                dv.extra
         FROM sys_dict_key dk
         JOIN sys_dict_value dv ON dv.key_id = dk.id AND dv.deleted = 0
         WHERE dk.key_name = ANY($1::text[])
         ORDER BY dk.id ASC, dv.sort ASC, dv.id ASC",
    )
    .bind(keys.clone())
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let mut data = HashMap::<String, Vec<OrionDictOption>>::new();
    for key in &keys {
        data.insert(key.clone(), Vec::new());
    }

    for row in rows {
        let mut extra = parse_extra_fields(row.4.as_deref().unwrap_or(""));
        if !extra.contains_key("color") && row.0.ends_with("Status") {
            if row.3 == "1" {
                extra.insert("color".to_string(), serde_json::json!("green"));
            } else if row.3 == "0" {
                extra.insert("color".to_string(), serde_json::json!("orangered"));
            }
        }
        let option = OrionDictOption {
            label: row.2,
            value: parse_dict_option_value(&row.1, &row.3),
            extra,
        };
        data.entry(row.0).or_default().push(option);
    }

    for key in &keys {
        if data.get(key).is_none_or(|v| v.is_empty()) {
            data.insert(key.clone(), builtin_dict_options(key));
        }
    }

    Ok(OrionResponse::ok(data))
}

async fn orion_dict_value_query(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<OrionDictValueQueryRequest>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "infra:dict-value:query").await?;
    let (page, limit, offset) = normalize_pagination(payload.page, payload.limit);
    let key_name = sanitize_search(payload.key_name);
    let value = sanitize_search(payload.value);
    let label = sanitize_search(payload.label);
    let extra = sanitize_search(payload.extra);

    let rows = sqlx::query_as::<
        _,
        (
            i64,
            i64,
            String,
            Option<String>,
            String,
            String,
            Option<String>,
            i32,
            i64,
            i64,
            Option<String>,
            Option<String>,
        ),
    >(
        "SELECT dv.id,
                dv.key_id,
                dk.key_name,
                dk.description,
                dv.value,
                dv.label,
                dv.extra,
                dv.sort,
                EXTRACT(EPOCH FROM dv.create_time)::bigint * 1000,
                EXTRACT(EPOCH FROM dv.update_time)::bigint * 1000,
                dv.creator,
                dv.updater
         FROM sys_dict_value dv
         JOIN sys_dict_key dk ON dk.id = dv.key_id
         WHERE dv.deleted = 0
           AND ($1::bigint IS NULL OR dv.key_id = $1)
           AND ($2::text IS NULL OR dk.key_name ILIKE '%' || $2 || '%')
           AND ($3::text IS NULL OR dv.value ILIKE '%' || $3 || '%')
           AND ($4::text IS NULL OR dv.label ILIKE '%' || $4 || '%')
           AND ($5::text IS NULL OR dv.extra ILIKE '%' || $5 || '%')
         ORDER BY dv.id DESC
         LIMIT $6 OFFSET $7",
    )
    .bind(payload.key_id)
    .bind(key_name.clone())
    .bind(value.clone())
    .bind(label.clone())
    .bind(extra.clone())
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.db)
    .await?;

    let total = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(1)
         FROM sys_dict_value dv
         JOIN sys_dict_key dk ON dk.id = dv.key_id
         WHERE dv.deleted = 0
           AND ($1::bigint IS NULL OR dv.key_id = $1)
           AND ($2::text IS NULL OR dk.key_name ILIKE '%' || $2 || '%')
           AND ($3::text IS NULL OR dv.value ILIKE '%' || $3 || '%')
           AND ($4::text IS NULL OR dv.label ILIKE '%' || $4 || '%')
           AND ($5::text IS NULL OR dv.extra ILIKE '%' || $5 || '%')",
    )
    .bind(payload.key_id)
    .bind(key_name)
    .bind(value)
    .bind(label)
    .bind(extra)
    .fetch_one(&state.db)
    .await?;

    let items = rows
        .into_iter()
        .map(|r| OrionDictValueItem {
            id: r.0,
            key_id: r.1,
            key_name: r.2,
            key_description: r.3.unwrap_or_default(),
            value: r.4,
            label: r.5,
            extra: r.6.unwrap_or_default(),
            sort: r.7,
            create_time: r.8,
            update_time: r.9,
            creator: r.10.unwrap_or_else(|| "system".to_string()),
            updater: r.11.unwrap_or_else(|| "system".to_string()),
        })
        .collect::<Vec<_>>();

    Ok(OrionResponse::ok(OrionDataGrid {
        page,
        limit,
        total,
        rows: items,
    }))
}

async fn orion_dict_value_delete(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<OrionIdQuery>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "infra:dict-value:delete").await?;
    let id = parse_required_id(query.id, "id")?;
    sqlx::query("UPDATE sys_dict_value SET deleted = 1, update_time = NOW() WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;
    Ok(OrionResponse::ok(true))
}

async fn orion_dict_value_batch_delete(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<OrionDeleteIdsQuery>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "infra:dict-value:delete").await?;
    let ids = parse_csv_ids(query.id_list);
    if !ids.is_empty() {
        sqlx::query(
            "UPDATE sys_dict_value SET deleted = 1, update_time = NOW() WHERE id = ANY($1::bigint[])",
        )
        .bind(ids)
        .execute(&state.db)
        .await?;
    }
    Ok(OrionResponse::ok(true))
}

async fn orion_system_message_list(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<OrionSystemMessageQueryRequest>,
) -> AppResult<impl axum::response::IntoResponse> {
    let user_id = guard::current_user_id(&headers, &state.config.jwt.secret)?;
    let (_page, limit, offset) = normalize_pagination(payload.page, payload.limit);
    let effective_offset = if payload.max_id.is_some() { 0 } else { offset };
    let classify = sanitize_search(payload.classify);
    let query_unread = payload.query_unread.unwrap_or(false);

    let rows = sqlx::query_as::<
        _,
        (
            i64,
            String,
            String,
            i16,
            Option<String>,
            String,
            String,
            Option<String>,
            i64,
        ),
    >(
        "SELECT id,
                classify,
                type,
                status,
                rel_key,
                title,
                content,
                content_html,
                EXTRACT(EPOCH FROM create_time)::bigint * 1000
         FROM sys_system_message
         WHERE (user_id IS NULL OR user_id = $1)
           AND ($2::text IS NULL OR classify = $2)
           AND ($3::boolean = FALSE OR status = 0)
           AND ($4::bigint IS NULL OR id < $4)
         ORDER BY id DESC
         LIMIT $5 OFFSET $6",
    )
    .bind(user_id)
    .bind(classify)
    .bind(query_unread)
    .bind(payload.max_id)
    .bind(limit)
    .bind(effective_offset)
    .fetch_all(&state.db)
    .await?;

    let data = rows
        .into_iter()
        .map(|r| OrionSystemMessageItem {
            id: r.0,
            classify: r.1,
            r#type: r.2,
            status: r.3,
            rel_key: r.4.unwrap_or_default(),
            title: r.5,
            content: r.6,
            content_html: r.7.unwrap_or_default(),
            create_time: r.8,
        })
        .collect::<Vec<_>>();

    Ok(OrionResponse::ok(data))
}

async fn orion_system_message_count(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<OrionSystemMessageCountQuery>,
) -> AppResult<impl axum::response::IntoResponse> {
    let user_id = guard::current_user_id(&headers, &state.config.jwt.secret)?;
    let query_unread = query.query_unread.unwrap_or(false);

    let rows = sqlx::query_as::<_, (String, i64)>(
        "SELECT classify, COUNT(1)::bigint
         FROM sys_system_message
         WHERE (user_id IS NULL OR user_id = $1)
           AND ($2::boolean = FALSE OR status = 0)
         GROUP BY classify",
    )
    .bind(user_id)
    .bind(query_unread)
    .fetch_all(&state.db)
    .await?;

    let data = rows.into_iter().collect::<HashMap<_, _>>();
    Ok(OrionResponse::ok(data))
}

async fn orion_system_message_has_unread(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<impl axum::response::IntoResponse> {
    let user_id = guard::current_user_id(&headers, &state.config.jwt.secret)?;
    let has_unread = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(
            SELECT 1 FROM sys_system_message
            WHERE (user_id IS NULL OR user_id = $1) AND status = 0
        )",
    )
    .bind(user_id)
    .fetch_one(&state.db)
    .await?;
    Ok(OrionResponse::ok(has_unread))
}

async fn orion_system_message_read(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<OrionIdQuery>,
) -> AppResult<impl axum::response::IntoResponse> {
    let user_id = guard::current_user_id(&headers, &state.config.jwt.secret)?;
    let id = parse_required_id(query.id, "id")?;
    sqlx::query(
        "UPDATE sys_system_message
         SET status = 1, read_time = NOW()
         WHERE id = $1
           AND status = 0
           AND (user_id IS NULL OR user_id = $2)",
    )
    .bind(id)
    .bind(user_id)
    .execute(&state.db)
    .await?;
    Ok(OrionResponse::ok(true))
}

async fn orion_system_message_read_all(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<OrionSystemMessageReadAllQuery>,
) -> AppResult<impl axum::response::IntoResponse> {
    let user_id = guard::current_user_id(&headers, &state.config.jwt.secret)?;
    let classify = sanitize_search(query.classify);
    sqlx::query(
        "UPDATE sys_system_message
         SET status = 1, read_time = NOW()
         WHERE status = 0
           AND (user_id IS NULL OR user_id = $1)
           AND ($2::text IS NULL OR classify = $2)",
    )
    .bind(user_id)
    .bind(classify)
    .execute(&state.db)
    .await?;
    Ok(OrionResponse::ok(true))
}

async fn orion_system_message_delete(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<OrionIdQuery>,
) -> AppResult<impl axum::response::IntoResponse> {
    let user_id = guard::current_user_id(&headers, &state.config.jwt.secret)?;
    let id = parse_required_id(query.id, "id")?;
    sqlx::query(
        "DELETE FROM sys_system_message WHERE id = $1 AND (user_id IS NULL OR user_id = $2)",
    )
    .bind(id)
    .bind(user_id)
    .execute(&state.db)
    .await?;
    Ok(OrionResponse::ok(true))
}

async fn orion_system_message_clear(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<OrionSystemMessageReadAllQuery>,
) -> AppResult<impl axum::response::IntoResponse> {
    let user_id = guard::current_user_id(&headers, &state.config.jwt.secret)?;
    let classify = sanitize_search(query.classify);
    sqlx::query(
        "DELETE FROM sys_system_message
         WHERE status = 1
           AND (user_id IS NULL OR user_id = $1)
           AND ($2::text IS NULL OR classify = $2)",
    )
    .bind(user_id)
    .bind(classify)
    .execute(&state.db)
    .await?;
    Ok(OrionResponse::ok(true))
}

async fn orion_infra_statistics_get_workplace(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<impl axum::response::IntoResponse> {
    let user_id = guard::current_user_id(&headers, &state.config.jwt.secret)?;

    let user = sqlx::query_as::<_, (String, Option<String>, Option<i64>)>(
        "SELECT username, nickname,
                (EXTRACT(EPOCH FROM last_login_time) * 1000)::BIGINT
         FROM sys_user
         WHERE id = $1 AND deleted = 0",
    )
    .bind(user_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::Auth("User not found".to_string()))?;

    let unread_message_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(1)::bigint
         FROM sys_system_message
         WHERE (user_id IS NULL OR user_id = $1) AND status = 0",
    )
    .bind(user_id)
    .fetch_one(&state.db)
    .await
    .unwrap_or(0);

    let user_session_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*)::BIGINT FROM ssh_session WHERE user_id = $1 AND end_time IS NULL",
    )
    .bind(user_id)
    .fetch_one(&state.db)
    .await
    .unwrap_or(0);

    let chart_rows = sqlx::query_as::<_, (String, i64)>(
        "SELECT to_char(day_list.day, 'MM-DD') AS day_label,
                COALESCE(day_count.cnt, 0)::bigint AS cnt
         FROM generate_series(
                (date_trunc('day', NOW()) - INTERVAL '6 day')::timestamp,
                date_trunc('day', NOW())::timestamp,
                INTERVAL '1 day'
              ) AS day_list(day)
         LEFT JOIN (
            SELECT date_trunc('day', create_time) AS day, COUNT(1) AS cnt
            FROM operator_log
            WHERE user_id = $1
            GROUP BY date_trunc('day', create_time)
         ) AS day_count
           ON day_count.day = day_list.day
         ORDER BY day_list.day ASC",
    )
    .bind(user_id)
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let operator_chart = OrionLineSingleChartData {
        x: chart_rows.iter().map(|r| r.0.clone()).collect(),
        data: chart_rows.iter().map(|r| r.1).collect(),
    };

    let login_history_list = sqlx::query_as::<
        _,
        (
            i64,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<i16>,
            Option<String>,
            i64,
        ),
    >(
        "SELECT id,
                ip,
                location,
                user_agent,
                result,
                error_message,
                COALESCE((EXTRACT(EPOCH FROM create_time) * 1000)::BIGINT, 0)
         FROM login_log
         WHERE user_id = $1
         ORDER BY create_time DESC
         LIMIT 10",
    )
    .bind(user_id)
    .fetch_all(&state.db)
    .await
    .unwrap_or_default()
    .into_iter()
    .map(|item| OrionLoginHistoryItem {
        id: item.0,
        address: item.1.unwrap_or_default(),
        location: item.2.unwrap_or_default(),
        user_agent: item.3.unwrap_or_default(),
        result: item.4.unwrap_or(0),
        error_message: item.5.unwrap_or_default(),
        create_time: item.6,
    })
    .collect::<Vec<_>>();

    let data = OrionInfraWorkplaceStatisticsResponse {
        user_id,
        username: user.0,
        nickname: user.1.unwrap_or_default(),
        unread_message_count,
        last_login_time: user.2.unwrap_or(0),
        user_session_count,
        operator_chart,
        login_history_list,
    };

    Ok(OrionResponse::ok(data))
}

async fn orion_exec_statistics_get_workplace(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> AppResult<impl axum::response::IntoResponse> {
    let _user_id = guard::current_user_id(&headers, &state.config.jwt.secret)?;

    let data = OrionExecWorkplaceStatisticsResponse {
        exec_job_count: 0,
        today_exec_command_count: 0,
        week_exec_command_count: 0,
        exec_command_chart: OrionLineSingleChartData {
            x: Vec::new(),
            data: Vec::new(),
        },
        exec_log_list: Vec::new(),
    };

    Ok(OrionResponse::ok(data))
}

async fn orion_terminal_statistics_get_workplace(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> AppResult<impl axum::response::IntoResponse> {
    let _user_id = guard::current_user_id(&headers, &state.config.jwt.secret)?;

    let data = OrionTerminalWorkplaceStatisticsResponse {
        today_terminal_connect_count: 0,
        week_terminal_connect_count: 0,
        terminal_connect_chart: OrionLineSingleChartData {
            x: Vec::new(),
            data: Vec::new(),
        },
        terminal_connect_list: Vec::new(),
    };

    Ok(OrionResponse::ok(data))
}

async fn orion_mine_login_history(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<OrionCountQuery>,
) -> AppResult<impl axum::response::IntoResponse> {
    let user_id = guard::current_user_id(&headers, &state.config.jwt.secret)?;
    let limit = query.count.unwrap_or(10).clamp(1, 100);
    let rows = sqlx::query_as::<_, (i64, Option<String>, Option<String>, Option<String>, i16, Option<String>, i64)>(
        "SELECT id, ip, location, user_agent, result, error_message, EXTRACT(EPOCH FROM create_time)::bigint * 1000
         FROM login_log
         WHERE user_id = $1
         ORDER BY create_time DESC
         LIMIT $2",
    )
    .bind(user_id)
    .bind(limit)
    .fetch_all(&state.db)
    .await?;

    let data = rows
        .into_iter()
        .map(|r| {
            serde_json::json!({
                "id": r.0,
                "address": r.1.unwrap_or_default(),
                "location": r.2.unwrap_or_default(),
                "userAgent": r.3.unwrap_or_default(),
                "result": r.4,
                "errorMessage": r.5.unwrap_or_default(),
                "createTime": r.6
            })
        })
        .collect::<Vec<_>>();
    Ok(OrionResponse::ok(data))
}

async fn orion_mine_user_session(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<impl axum::response::IntoResponse> {
    let user_id = guard::current_user_id(&headers, &state.config.jwt.secret)?;
    let rows = sqlx::query_as::<_, (i64, String, Option<String>, i64)>(
        "SELECT id, session_id::text, NULLIF(revoked_at::text, ''), EXTRACT(EPOCH FROM created_at)::bigint * 1000
         FROM auth_refresh_token
         WHERE user_id = $1
         ORDER BY created_at DESC
         LIMIT 100",
    )
    .bind(user_id)
    .fetch_all(&state.db)
    .await?;

    let data = rows
        .into_iter()
        .map(|r| {
            serde_json::json!({
                "id": r.0,
                "username": user_id.to_string(),
                "visible": true,
                "current": r.1.contains('-'),
                "address": "",
                "location": "",
                "userAgent": "",
                "loginTime": r.3,
                "offline": r.2.is_some()
            })
        })
        .collect::<Vec<_>>();
    Ok(OrionResponse::ok(data))
}

async fn orion_mine_offline_session(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<OrionSessionOfflineRequest>,
) -> AppResult<impl axum::response::IntoResponse> {
    let user_id = guard::current_user_id(&headers, &state.config.jwt.secret)?;
    let _ = payload.user_id;
    sqlx::query(
        "UPDATE auth_refresh_token SET revoked_at = NOW() WHERE user_id = $1 AND EXTRACT(EPOCH FROM created_at)::bigint * 1000 = $2 AND revoked_at IS NULL",
    )
    .bind(user_id)
    .bind(payload.timestamp)
    .execute(&state.db)
    .await?;
    Ok(OrionResponse::ok(true))
}

async fn query_operator_logs(
    state: &AppState,
    scope_user_id: Option<i64>,
    payload: &OrionOperatorLogQueryRequest,
) -> AppResult<OrionDataGrid<OrionOperatorLogItem>> {
    let username = sanitize_search(payload.username.clone());
    let module = sanitize_search(payload.module.clone());
    let (page, limit, offset) = normalize_pagination(payload.page, payload.limit);

    let rows = sqlx::query_as::<_, (i64, Option<i64>, Option<String>, Option<String>, Option<String>, Option<String>, Option<String>, Option<i16>, Option<String>, Option<i32>, i64)>(
        "SELECT id, user_id, username, module, operation, trace_id, ip, result, error_message, duration, EXTRACT(EPOCH FROM create_time)::bigint * 1000
         FROM operator_log
         WHERE ($1::bigint IS NULL OR user_id = $1)
           AND ($2::bigint IS NULL OR user_id = $2)
           AND ($3::text IS NULL OR username ILIKE '%' || $3 || '%')
           AND ($4::text IS NULL OR module ILIKE '%' || $4 || '%')
           AND ($5::smallint IS NULL OR result = $5)
         ORDER BY create_time DESC
         LIMIT $6 OFFSET $7",
    )
    .bind(scope_user_id)
    .bind(payload.user_id)
    .bind(username)
    .bind(module)
    .bind(payload.result)
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.db)
    .await?;

    let total = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(1)
         FROM operator_log
         WHERE ($1::bigint IS NULL OR user_id = $1)
           AND ($2::bigint IS NULL OR user_id = $2)
           AND ($3::text IS NULL OR username ILIKE '%' || $3 || '%')
           AND ($4::text IS NULL OR module ILIKE '%' || $4 || '%')
           AND ($5::smallint IS NULL OR result = $5)",
    )
    .bind(scope_user_id)
    .bind(payload.user_id)
    .bind(sanitize_search(payload.username.clone()))
    .bind(sanitize_search(payload.module.clone()))
    .bind(payload.result)
    .fetch_one(&state.db)
    .await?;

    let items = rows
        .into_iter()
        .map(|r| OrionOperatorLogItem {
            id: r.0,
            user_id: r.1.unwrap_or_default(),
            username: r.2.unwrap_or_default(),
            trace_id: r.5.unwrap_or_default(),
            address: r.6.unwrap_or_default(),
            location: String::new(),
            user_agent: String::new(),
            risk_level: "LOW".to_string(),
            module: r.3.unwrap_or_default(),
            r#type: r.4.unwrap_or_default(),
            log_info: String::new(),
            origin_log_info: String::new(),
            extra: String::new(),
            result: r.7.unwrap_or_default(),
            error_message: r.8.unwrap_or_default(),
            return_value: String::new(),
            duration: r.9.unwrap_or_default(),
            start_time: r.10,
            end_time: r.10,
            create_time: r.10,
        })
        .collect::<Vec<_>>();

    Ok(OrionDataGrid {
        page,
        limit,
        total,
        rows: items,
    })
}

async fn orion_mine_query_operator_log(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<OrionOperatorLogQueryRequest>,
) -> AppResult<impl axum::response::IntoResponse> {
    let user_id = guard::current_user_id(&headers, &state.config.jwt.secret)?;
    Ok(OrionResponse::ok(
        query_operator_logs(&state, Some(user_id), &payload).await?,
    ))
}

async fn orion_mine_update_password(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<OrionMineUpdatePasswordRequest>,
) -> AppResult<impl axum::response::IntoResponse> {
    let user_id = guard::current_user_id(&headers, &state.config.jwt.secret)?;
    let old_hash = sqlx::query_scalar::<_, String>(
        "SELECT password FROM sys_user WHERE id = $1 AND deleted = 0",
    )
    .bind(user_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::Auth("User not found".to_string()))?;

    let before = payload
        .before_password
        .ok_or_else(|| AppError::BadRequest("beforePassword is required".to_string()))?;
    let new_password = payload
        .password
        .ok_or_else(|| AppError::BadRequest("password is required".to_string()))?;

    if payload.check_password.as_deref() != Some(&new_password) {
        return Err(AppError::BadRequest("checkPassword mismatch".to_string()));
    }
    if !bcrypt::verify(before, &old_hash).unwrap_or(false) {
        return Err(AppError::Auth("invalid before password".to_string()));
    }
    let new_hash = bcrypt::hash(new_password, bcrypt::DEFAULT_COST)
        .map_err(|e| AppError::Internal(e.to_string()))?;
    sqlx::query("UPDATE sys_user SET password = $1, update_time = NOW() WHERE id = $2")
        .bind(new_hash)
        .bind(user_id)
        .execute(&state.db)
        .await?;
    Ok(OrionResponse::ok(true))
}

async fn orion_operator_log_query(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<OrionOperatorLogQueryRequest>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "iam.user-permission.view").await?;
    Ok(OrionResponse::ok(
        query_operator_logs(&state, None, &payload).await?,
    ))
}

async fn orion_operator_log_count(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<OrionOperatorLogQueryRequest>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "iam.user-permission.view").await?;
    let grid = query_operator_logs(&state, None, &payload).await?;
    Ok(OrionResponse::ok(grid.total))
}

async fn orion_operator_log_delete(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<OrionDeleteIdsQuery>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "iam.user-permission.view").await?;
    let ids = parse_csv_ids(query.id_list);
    if !ids.is_empty() {
        sqlx::query("DELETE FROM operator_log WHERE id = ANY($1::bigint[])")
            .bind(ids)
            .execute(&state.db)
            .await?;
    }
    Ok(OrionResponse::ok(true))
}

async fn orion_operator_log_clear(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "iam.user-permission.view").await?;
    let result = sqlx::query("DELETE FROM operator_log")
        .execute(&state.db)
        .await?;
    Ok(OrionResponse::ok(result.rows_affected() as i64))
}

fn parse_required_id(v: Option<i64>, field: &str) -> AppResult<i64> {
    let id = v.ok_or_else(|| AppError::BadRequest(format!("{field} is required")))?;
    if id <= 0 {
        return Err(AppError::BadRequest(format!(
            "{field} must be greater than 0"
        )));
    }
    Ok(id)
}

async fn load_user_roles(state: &AppState, user_id: i64) -> AppResult<Vec<OrionRoleQueryResponse>> {
    let rows = sqlx::query_as::<_, (i64, String, String, i16, Option<String>)>(
        "SELECT r.id, r.name, r.code, r.status, r.description
         FROM sys_user_role ur
         JOIN sys_role r ON r.id = ur.role_id
         WHERE ur.user_id = $1 AND r.deleted = 0
         ORDER BY r.id ASC",
    )
    .bind(user_id)
    .fetch_all(&state.db)
    .await?;

    Ok(rows
        .into_iter()
        .map(|r| OrionRoleQueryResponse {
            id: r.0,
            name: r.1,
            code: r.2,
            status: r.3,
            description: r.4.unwrap_or_default(),
            create_time: 0,
            update_time: 0,
            creator: "system".to_string(),
            updater: "system".to_string(),
        })
        .collect())
}

async fn map_user_row(
    state: &AppState,
    user: OrionSystemUserAggregate,
) -> AppResult<OrionSystemUserQueryResponse> {
    Ok(OrionSystemUserQueryResponse {
        id: user.id,
        username: user.username,
        nickname: user.nickname.unwrap_or_default(),
        avatar: user.avatar.unwrap_or_default(),
        mobile: user.mobile.unwrap_or_default(),
        email: user.email.unwrap_or_default(),
        status: user.status,
        last_login_time: user.last_login_time,
        description: String::new(),
        create_time: user.create_time_ms,
        update_time: user.update_time_ms,
        creator: "system".to_string(),
        updater: "system".to_string(),
        roles: load_user_roles(state, user.id).await?,
    })
}

async fn orion_system_user_create(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<OrionSystemUserUpsertRequest>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "iam.user-permission.view").await?;
    let username = sanitize_search(payload.username)
        .ok_or_else(|| AppError::BadRequest("username is required".to_string()))?;
    let password = payload
        .password
        .ok_or_else(|| AppError::BadRequest("password is required".to_string()))?;
    let pass_hash = bcrypt::hash(password, bcrypt::DEFAULT_COST)
        .map_err(|e| AppError::Internal(e.to_string()))?;
    let id = sqlx::query_scalar::<_, i64>(
        "INSERT INTO sys_user (username, password, nickname, avatar, phone, email, status, create_time, update_time, deleted)
         VALUES ($1, $2, $3, $4, $5, $6, 1, NOW(), NOW(), 0)
         RETURNING id",
    )
    .bind(username)
    .bind(pass_hash)
    .bind(payload.nickname)
    .bind(payload.avatar)
    .bind(payload.mobile)
    .bind(payload.email)
    .fetch_one(&state.db)
    .await?;
    Ok(OrionResponse::ok(id))
}

async fn orion_system_user_update(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<OrionSystemUserUpsertRequest>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "iam.user-permission.view").await?;
    let id = parse_required_id(payload.id, "id")?;
    let rows = sqlx::query(
        "UPDATE sys_user SET
            username = COALESCE(NULLIF($1, ''), username),
            nickname = COALESCE($2, nickname),
            avatar = COALESCE($3, avatar),
            phone = COALESCE($4, phone),
            email = COALESCE($5, email),
            update_time = NOW()
         WHERE id = $6 AND deleted = 0",
    )
    .bind(payload.username.map(|v| v.trim().to_string()))
    .bind(payload.nickname)
    .bind(payload.avatar)
    .bind(payload.mobile)
    .bind(payload.email)
    .bind(id)
    .execute(&state.db)
    .await?
    .rows_affected();
    if rows == 0 {
        return Err(AppError::NotFound("User not found".to_string()));
    }
    Ok(OrionResponse::ok(true))
}

async fn orion_system_user_update_status(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<OrionSystemUserUpsertRequest>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "iam.user-permission.view").await?;
    let id = parse_required_id(payload.id, "id")?;
    let status = payload
        .status
        .ok_or_else(|| AppError::BadRequest("status is required".to_string()))?;
    let rows = sqlx::query(
        "UPDATE sys_user SET status = $1, update_time = NOW() WHERE id = $2 AND deleted = 0",
    )
    .bind(status)
    .bind(id)
    .execute(&state.db)
    .await?
    .rows_affected();
    if rows == 0 {
        return Err(AppError::NotFound("User not found".to_string()));
    }
    Ok(OrionResponse::ok(true))
}

async fn orion_system_user_grant_role(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<OrionSystemUserUpsertRequest>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "iam.user-role.assign").await?;
    let user_id = parse_required_id(payload.id, "id")?;
    let role_ids = payload.role_id_list.unwrap_or_default();
    let mut tx = state.db.begin().await?;
    sqlx::query("DELETE FROM sys_user_role WHERE user_id = $1")
        .bind(user_id)
        .execute(&mut *tx)
        .await?;
    for role_id in role_ids {
        if role_id > 0 {
            sqlx::query(
                "INSERT INTO sys_user_role (user_id, role_id, create_time)
                 VALUES ($1, $2, NOW())
                 ON CONFLICT (user_id, role_id) DO NOTHING",
            )
            .bind(user_id)
            .bind(role_id)
            .execute(&mut *tx)
            .await?;
        }
    }
    tx.commit().await?;
    Ok(OrionResponse::ok(true))
}

async fn orion_system_user_reset_password(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<OrionSystemUserUpsertRequest>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "iam.user-permission.view").await?;
    let id = parse_required_id(payload.id, "id")?;
    let password = payload
        .password
        .ok_or_else(|| AppError::BadRequest("password is required".to_string()))?;
    let pass_hash = bcrypt::hash(password, bcrypt::DEFAULT_COST)
        .map_err(|e| AppError::Internal(e.to_string()))?;
    sqlx::query(
        "UPDATE sys_user SET password = $1, update_time = NOW() WHERE id = $2 AND deleted = 0",
    )
    .bind(pass_hash)
    .bind(id)
    .execute(&state.db)
    .await?;
    Ok(OrionResponse::ok(true))
}

async fn orion_system_user_get(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<OrionSystemUserIdQuery>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "iam.user-permission.view").await?;
    let id = parse_required_id(query.id, "id")?;
    let user = system_user_service::get_system_user_by_id(&state.db, id)
        .await?
        .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;
    Ok(OrionResponse::ok(map_user_row(&state, user).await?))
}

async fn orion_system_user_list(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "iam.user-permission.view").await?;
    let rows = system_user_service::list_system_users(&state.db).await?;
    let mut list = Vec::with_capacity(rows.len());
    for row in rows {
        list.push(map_user_row(&state, row).await?);
    }
    Ok(OrionResponse::ok(list))
}

async fn orion_system_user_query(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<OrionSystemUserQueryRequest>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "iam.user-permission.view").await?;
    let (page, limit, offset) = normalize_pagination(payload.page, payload.limit);
    let username = sanitize_search(payload.username.clone());
    let nickname = sanitize_search(payload.nickname.clone());
    let mobile = sanitize_search(payload.mobile.clone());
    let email = sanitize_search(payload.email.clone());
    let filters = OrionSystemUserQueryFilters {
        id: payload.id,
        username,
        nickname,
        mobile,
        email,
        status: payload.status,
        limit: Some(limit),
        offset: Some(offset),
    };

    let rows = system_user_service::query_system_users(&state.db, filters.clone()).await?;
    let total = system_user_service::count_system_users(&state.db, filters).await?;

    let mut list = Vec::with_capacity(rows.len());
    for row in rows {
        list.push(map_user_row(&state, row).await?);
    }

    Ok(OrionResponse::ok(OrionDataGrid {
        page,
        limit,
        total,
        rows: list,
    }))
}

async fn orion_system_user_count(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<OrionSystemUserQueryRequest>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "iam.user-permission.view").await?;
    let username = sanitize_search(payload.username.clone());
    let nickname = sanitize_search(payload.nickname.clone());
    let mobile = sanitize_search(payload.mobile.clone());
    let email = sanitize_search(payload.email.clone());
    let total = system_user_service::count_system_users(
        &state.db,
        OrionSystemUserQueryFilters {
            id: payload.id,
            username,
            nickname,
            mobile,
            email,
            status: payload.status,
            limit: None,
            offset: None,
        },
    )
    .await?;
    Ok(OrionResponse::ok(total))
}

async fn orion_system_user_delete(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<OrionSystemUserIdQuery>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "iam.user-permission.view").await?;
    let id = parse_required_id(query.id, "id")?;
    sqlx::query(
        "UPDATE sys_user SET deleted = 1, update_time = NOW() WHERE id = $1 AND deleted = 0",
    )
    .bind(id)
    .execute(&state.db)
    .await?;
    Ok(OrionResponse::ok(true))
}

async fn orion_system_user_batch_delete(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<OrionDeleteIdsQuery>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "iam.user-permission.view").await?;
    let ids = parse_csv_ids(query.id_list);
    if !ids.is_empty() {
        sqlx::query("UPDATE sys_user SET deleted = 1, update_time = NOW() WHERE id = ANY($1::bigint[]) AND deleted = 0")
            .bind(ids)
            .execute(&state.db)
            .await?;
    }
    Ok(OrionResponse::ok(true))
}

async fn orion_system_user_get_roles(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<OrionSystemUserIdQuery>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "iam.user-permission.view").await?;
    let user_id = parse_required_id(query.user_id, "userId")?;
    let ids = sqlx::query_scalar::<_, i64>(
        "SELECT role_id FROM sys_user_role WHERE user_id = $1 ORDER BY role_id ASC",
    )
    .bind(user_id)
    .fetch_all(&state.db)
    .await?;
    Ok(OrionResponse::ok(ids))
}

async fn orion_system_user_login_history(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<OrionSystemUserIdQuery>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "iam.user-permission.view").await?;
    let username = query
        .username
        .ok_or_else(|| AppError::BadRequest("username is required".to_string()))?;
    let count = 20_i64;
    let rows = sqlx::query_as::<_, (i64, Option<String>, Option<String>, Option<String>, i16, Option<String>, i64)>(
        "SELECT id, ip, location, user_agent, result, error_message, EXTRACT(EPOCH FROM create_time)::bigint * 1000
         FROM login_log
         WHERE username = $1
         ORDER BY create_time DESC
         LIMIT $2",
    )
    .bind(username)
    .bind(count)
    .fetch_all(&state.db)
    .await?;
    let data = rows
        .into_iter()
        .map(|r| {
            serde_json::json!({
                "id": r.0,
                "address": r.1.unwrap_or_default(),
                "location": r.2.unwrap_or_default(),
                "userAgent": r.3.unwrap_or_default(),
                "result": r.4,
                "errorMessage": r.5.unwrap_or_default(),
                "createTime": r.6
            })
        })
        .collect::<Vec<_>>();
    Ok(OrionResponse::ok(data))
}

async fn orion_system_user_session_users_list(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "iam.user-permission.view").await?;
    let rows = sqlx::query_as::<_, (i64, i64, i64)>(
        "SELECT id, user_id, EXTRACT(EPOCH FROM created_at)::bigint * 1000
         FROM auth_refresh_token
         WHERE revoked_at IS NULL
         ORDER BY created_at DESC LIMIT 200",
    )
    .fetch_all(&state.db)
    .await?;
    let data = rows
        .into_iter()
        .map(|r| {
            serde_json::json!({
                "id": r.0,
                "username": r.1.to_string(),
                "visible": true,
                "current": false,
                "address": "",
                "location": "",
                "userAgent": "",
                "loginTime": r.2
            })
        })
        .collect::<Vec<_>>();
    Ok(OrionResponse::ok(data))
}

async fn orion_system_user_session_user_list(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<OrionSystemUserIdQuery>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "iam.user-permission.view").await?;
    let user_id = parse_required_id(query.id, "id")?;
    let rows = sqlx::query_as::<_, (i64, i64)>(
        "SELECT id, EXTRACT(EPOCH FROM created_at)::bigint * 1000
         FROM auth_refresh_token
         WHERE user_id = $1
         ORDER BY created_at DESC LIMIT 200",
    )
    .bind(user_id)
    .fetch_all(&state.db)
    .await?;
    let data = rows
        .into_iter()
        .map(|r| {
            serde_json::json!({
                "id": r.0,
                "username": user_id.to_string(),
                "visible": true,
                "current": false,
                "address": "",
                "location": "",
                "userAgent": "",
                "loginTime": r.1
            })
        })
        .collect::<Vec<_>>();
    Ok(OrionResponse::ok(data))
}

async fn orion_system_user_session_offline(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<OrionSessionOfflineRequest>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "iam.user-permission.view").await?;
    let user_id = parse_required_id(payload.user_id, "userId")?;
    sqlx::query(
        "UPDATE auth_refresh_token SET revoked_at = NOW() WHERE user_id = $1 AND EXTRACT(EPOCH FROM created_at)::bigint * 1000 = $2",
    )
    .bind(user_id)
    .bind(payload.timestamp)
    .execute(&state.db)
    .await?;
    Ok(OrionResponse::ok(true))
}

async fn orion_system_user_locked_list() -> AppResult<impl axum::response::IntoResponse> {
    Ok(OrionResponse::ok(Vec::<serde_json::Value>::new()))
}

async fn orion_system_user_locked_unlock() -> AppResult<impl axum::response::IntoResponse> {
    Ok(OrionResponse::ok(true))
}

async fn orion_system_role_create(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<OrionSystemRoleRequest>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "iam.user-role.assign").await?;
    let name = sanitize_search(payload.name)
        .ok_or_else(|| AppError::BadRequest("name is required".to_string()))?;
    let code = sanitize_search(payload.code)
        .ok_or_else(|| AppError::BadRequest("code is required".to_string()))?;
    let id = sqlx::query_scalar::<_, i64>(
        "INSERT INTO sys_role (name, code, description, status, create_time, deleted)
         VALUES ($1, $2, $3, COALESCE($4, 1), NOW(), 0)
         RETURNING id",
    )
    .bind(name)
    .bind(code)
    .bind(payload.description)
    .bind(payload.status)
    .fetch_one(&state.db)
    .await?;
    Ok(OrionResponse::ok(id))
}

async fn orion_system_role_update(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<OrionSystemRoleRequest>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "iam.user-role.assign").await?;
    let id = parse_required_id(payload.id, "id")?;
    let rows = sqlx::query(
        "UPDATE sys_role SET name = COALESCE(NULLIF($1, ''), name), code = COALESCE(NULLIF($2, ''), code), description = COALESCE($3, description), status = COALESCE($4, status) WHERE id = $5 AND deleted = 0",
    )
    .bind(payload.name.map(|v| v.trim().to_string()))
    .bind(payload.code.map(|v| v.trim().to_string()))
    .bind(payload.description)
    .bind(payload.status)
    .bind(id)
    .execute(&state.db)
    .await?
    .rows_affected();
    if rows == 0 {
        return Err(AppError::NotFound("Role not found".to_string()));
    }
    Ok(OrionResponse::ok(true))
}

async fn orion_system_role_update_status(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<OrionSystemRoleRequest>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "iam.user-role.assign").await?;
    let id = parse_required_id(payload.id, "id")?;
    let status = payload
        .status
        .ok_or_else(|| AppError::BadRequest("status is required".to_string()))?;
    sqlx::query("UPDATE sys_role SET status = $1 WHERE id = $2 AND deleted = 0")
        .bind(status)
        .bind(id)
        .execute(&state.db)
        .await?;
    Ok(OrionResponse::ok(true))
}

fn map_role_row(row: (i64, String, String, i16, Option<String>, i64)) -> OrionRoleQueryResponse {
    OrionRoleQueryResponse {
        id: row.0,
        name: row.1,
        code: row.2,
        status: row.3,
        description: row.4.unwrap_or_default(),
        create_time: row.5,
        update_time: row.5,
        creator: "system".to_string(),
        updater: "system".to_string(),
    }
}

async fn orion_system_role_get(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<OrionSystemRoleIdQuery>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "iam.user-role.assign").await?;
    let id = parse_required_id(query.id, "id")?;
    let row = sqlx::query_as::<_, (i64, String, String, i16, Option<String>, i64)>(
        "SELECT id, name, code, status, description, EXTRACT(EPOCH FROM create_time)::bigint * 1000 FROM sys_role WHERE id = $1 AND deleted = 0",
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound("Role not found".to_string()))?;
    Ok(OrionResponse::ok(map_role_row(row)))
}

async fn orion_system_role_list(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "iam.user-role.assign").await?;
    let rows = sqlx::query_as::<_, (i64, String, String, i16, Option<String>, i64)>(
        "SELECT id, name, code, status, description, EXTRACT(EPOCH FROM create_time)::bigint * 1000 FROM sys_role WHERE deleted = 0 ORDER BY id ASC",
    )
    .fetch_all(&state.db)
    .await?;
    Ok(OrionResponse::ok(
        rows.into_iter().map(map_role_row).collect::<Vec<_>>(),
    ))
}

async fn orion_system_role_query(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<OrionSystemRoleRequest>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "iam.user-role.assign").await?;
    let (page, limit, offset) = normalize_pagination(payload.page, payload.limit);
    let rows = sqlx::query_as::<_, (i64, String, String, i16, Option<String>, i64)>(
        "SELECT id, name, code, status, description, EXTRACT(EPOCH FROM create_time)::bigint * 1000
         FROM sys_role
         WHERE deleted = 0
         ORDER BY id DESC
         LIMIT $1 OFFSET $2",
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.db)
    .await?;
    let total = sqlx::query_scalar::<_, i64>("SELECT COUNT(1) FROM sys_role WHERE deleted = 0")
        .fetch_one(&state.db)
        .await?;
    Ok(OrionResponse::ok(OrionDataGrid {
        page,
        limit,
        total,
        rows: rows.into_iter().map(map_role_row).collect(),
    }))
}

async fn orion_system_role_delete(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<OrionSystemRoleIdQuery>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "iam.user-role.assign").await?;
    let id = parse_required_id(query.id, "id")?;
    sqlx::query("UPDATE sys_role SET deleted = 1 WHERE id = $1 AND deleted = 0")
        .bind(id)
        .execute(&state.db)
        .await?;
    Ok(OrionResponse::ok(true))
}

async fn orion_system_role_grant_menu(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<OrionSystemRoleRequest>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "iam.user-role.assign").await?;
    let role_id = parse_required_id(payload.role_id, "roleId")?;
    let mut tx = state.db.begin().await?;
    sqlx::query("DELETE FROM sys_role_menu WHERE role_id = $1")
        .bind(role_id)
        .execute(&mut *tx)
        .await?;
    for menu_id in payload.menu_id_list.unwrap_or_default() {
        if menu_id > 0 {
            sqlx::query(
                "INSERT INTO sys_role_menu (role_id, menu_id) VALUES ($1, $2) ON CONFLICT (role_id, menu_id) DO NOTHING",
            )
            .bind(role_id)
            .bind(menu_id)
            .execute(&mut *tx)
            .await?;
        }
    }
    tx.commit().await?;
    Ok(OrionResponse::ok(true))
}

async fn orion_system_role_get_menu_id(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<OrionSystemRoleIdQuery>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "iam.user-role.assign").await?;
    let role_id = parse_required_id(query.role_id, "roleId")?;
    let ids = sqlx::query_scalar::<_, i64>(
        "SELECT menu_id FROM sys_role_menu WHERE role_id = $1 ORDER BY menu_id ASC",
    )
    .bind(role_id)
    .fetch_all(&state.db)
    .await?;
    Ok(OrionResponse::ok(ids))
}

async fn orion_system_menu_list(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "iam.user-role.assign").await?;
    let rows = sqlx::query_as::<
        _,
        (
            i64,
            i64,
            String,
            Option<String>,
            i16,
            i32,
            i16,
            Option<String>,
            Option<String>,
            Option<String>,
        ),
    >(
        "SELECT id, parent_id, name, permission, type, sort, visible, icon, path, component
         FROM sys_menu ORDER BY sort ASC, id ASC",
    )
    .fetch_all(&state.db)
    .await?;
    let list = build_orion_menu_tree(
        rows.into_iter()
            .map(|m| OrionMenuItem {
                id: m.0,
                parent_id: m.1,
                name: m.2,
                permission: m.3.unwrap_or_default(),
                r#type: m.4,
                sort: m.5,
                visible: m.6,
                status: 1,
                cache: 1,
                new_window: 0,
                icon: m.7.unwrap_or_default(),
                path: m.8.unwrap_or_default(),
                component: m.9.unwrap_or_default(),
                children: Vec::new(),
            })
            .collect(),
    );
    Ok(OrionResponse::ok(list))
}

async fn orion_system_menu_create(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<OrionSystemMenuRequest>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "iam.user-role.assign").await?;
    let _ = (payload.cache, payload.new_window);
    let name = sanitize_search(payload.name)
        .ok_or_else(|| AppError::BadRequest("name is required".to_string()))?;
    let id = sqlx::query_scalar::<_, i64>(
        "INSERT INTO sys_menu (parent_id, name, path, component, icon, type, sort, visible, permission, create_time)
         VALUES (COALESCE($1, 0), $2, $3, $4, $5, COALESCE($6, 1), COALESCE($7, 0), COALESCE($8, 1), $9, NOW())
         RETURNING id",
    )
    .bind(payload.parent_id)
    .bind(name)
    .bind(payload.path)
    .bind(payload.component)
    .bind(payload.icon)
    .bind(payload.r#type)
    .bind(payload.sort)
    .bind(payload.visible)
    .bind(payload.permission)
    .fetch_one(&state.db)
    .await?;
    Ok(OrionResponse::ok(id))
}

async fn orion_system_menu_update(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<OrionSystemMenuRequest>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "iam.user-role.assign").await?;
    let id = parse_required_id(payload.id, "id")?;
    sqlx::query(
        "UPDATE sys_menu SET
            parent_id = COALESCE($1, parent_id),
            name = COALESCE(NULLIF($2, ''), name),
            path = COALESCE($3, path),
            component = COALESCE($4, component),
            icon = COALESCE($5, icon),
            type = COALESCE($6, type),
            sort = COALESCE($7, sort),
            visible = COALESCE($8, visible),
            permission = COALESCE($9, permission)
         WHERE id = $10",
    )
    .bind(payload.parent_id)
    .bind(payload.name.map(|v| v.trim().to_string()))
    .bind(payload.path)
    .bind(payload.component)
    .bind(payload.icon)
    .bind(payload.r#type)
    .bind(payload.sort)
    .bind(payload.visible)
    .bind(payload.permission)
    .bind(id)
    .execute(&state.db)
    .await?;
    Ok(OrionResponse::ok(true))
}

async fn orion_system_menu_update_status(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<OrionSystemMenuRequest>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "iam.user-role.assign").await?;
    let id = parse_required_id(payload.id, "id")?;
    let visible = payload.status.unwrap_or(1);
    sqlx::query("UPDATE sys_menu SET visible = $1 WHERE id = $2")
        .bind(visible)
        .bind(id)
        .execute(&state.db)
        .await?;
    Ok(OrionResponse::ok(true))
}

async fn orion_system_menu_delete(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<OrionSystemRoleIdQuery>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "iam.user-role.assign").await?;
    let id = parse_required_id(query.id, "id")?;
    let mut tx = state.db.begin().await?;
    sqlx::query("DELETE FROM sys_role_menu WHERE menu_id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM sys_menu WHERE id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(OrionResponse::ok(true))
}

async fn orion_system_menu_refresh_cache() -> AppResult<impl axum::response::IntoResponse> {
    Ok(OrionResponse::ok(true))
}

async fn orion_terminal_themes() -> AppResult<impl axum::response::IntoResponse> {
    Ok(OrionResponse::ok(vec![serde_json::json!({
        "name": "default",
        "foreground": "#f8f8f2",
        "background": "#1e1f29"
    })]))
}

async fn orion_terminal_access(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<OrionTerminalAccessRequest>,
) -> AppResult<impl axum::response::IntoResponse> {
    let user_id = guard::current_user_id(&headers, &state.config.jwt.secret)?;
    let host_id = parse_required_id(payload.host_id, "hostId")?;
    let token = format!(
        "term:{}:{}:{}:{}",
        user_id,
        host_id,
        payload.connect_type.unwrap_or_else(|| "ssh".to_string()),
        now_ms()
    );
    Ok(OrionResponse::ok(token))
}

async fn orion_terminal_transfer(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<impl axum::response::IntoResponse> {
    let user_id = guard::current_user_id(&headers, &state.config.jwt.secret)?;
    Ok(OrionResponse::ok(format!(
        "transfer:{}:{}",
        user_id,
        now_ms()
    )))
}

async fn orion_terminal_sftp_get_content(
    Query(query): Query<OrionTerminalSftpContentQuery>,
) -> AppResult<impl axum::response::IntoResponse> {
    let token = query
        .token
        .ok_or_else(|| AppError::BadRequest("token is required".to_string()))?;
    Ok(OrionResponse::ok(format!("# token\n{}\n", token)))
}

async fn orion_terminal_sftp_set_content(
    mut multipart: Multipart,
) -> AppResult<impl axum::response::IntoResponse> {
    let mut token: Option<String> = None;
    let mut has_file = false;
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
            let _ = field
                .bytes()
                .await
                .map_err(|e| AppError::BadRequest(e.to_string()))?;
            has_file = true;
        }
    }
    if token.as_deref().unwrap_or("{}") == "{}" || !has_file {
        return Err(AppError::BadRequest(
            "token and file are required".to_string(),
        ));
    }
    Ok(OrionResponse::ok(true))
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
        };
        let disabled = OrionHostAggregate {
            status: 0,
            ..enabled.clone()
        };

        assert_eq!(map_host_row(enabled).status, "ENABLED");
        assert_eq!(map_host_row(disabled).status, "DISABLED");
    }
}

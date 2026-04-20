use axum::{
    extract::{ConnectInfo, Path, Query, State},
    http::{HeaderMap, Method},
    routing::{any, delete, get, post, put},
    Json, Router,
};
use base64::{engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD}, Engine as _};
use chrono::{DateTime, NaiveDateTime, Utc};
use rsa::{
    pkcs1::DecodeRsaPrivateKey,
    pkcs8::{DecodePrivateKey, EncodePrivateKey, EncodePublicKey, LineEnding},
    Oaep, RsaPrivateKey, RsaPublicKey,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;
use std::net::SocketAddr;

use crate::{
    api::{auth, guard, AppState},
    application::orion::{asset_service, audit_service, compat_service, host_service, system_user_service},
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
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OrionHostUpdateRequest {
    id: Option<i64>,
    name: Option<String>,
    address: Option<String>,
    description: Option<String>,
    group_id_list: Option<Vec<i64>>,
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

fn verify_orion_password(raw_password: &str, password_hash: &str) -> bool {
    bcrypt::verify(raw_password, password_hash).unwrap_or(false)
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

    let password_ok = verify_orion_password(&payload.password, &user.2);

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
        &state.config.jwt,
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
    let user_id = guard::current_user_id(&headers, &state.config.jwt)?;

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
    let user_id = guard::current_user_id(&headers, &state.config.jwt)?;
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
    let user_id = guard::current_user_id(&headers, &state.config.jwt)?;
    Ok(OrionResponse::ok(get_tipped_keys(&state, user_id).await))
}

async fn orion_user_menu(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<impl axum::response::IntoResponse> {
    let user_id = guard::current_user_id(&headers, &state.config.jwt)?;

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
    let actor_user_id = guard::current_user_id(&headers, &state.config.jwt)?;
    let group_ids = normalize_group_ids(payload.group_id_list)?;
    ensure_groups_exist(&state, &group_ids).await?;

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

    let mut tx = state.db.begin().await?;
    let audit_name = name.clone();
    let audit_address = address.clone();
    let audit_group_ids = group_ids.clone();
    let new_id = sqlx::query_scalar::<_, i64>(
        "INSERT INTO host (name, hostname, port, credential_type, description, status, create_time, update_time)
         VALUES ($1, $2, 22, NULL, $3, 1, NOW(), NOW())
         RETURNING id",
    )
    .bind(name)
    .bind(address)
    .bind(payload.description)
    .fetch_one(&mut *tx)
    .await?;

    for group_id in group_ids {
        sqlx::query(
            "INSERT INTO host_group_rel (host_id, group_id)
             VALUES ($1, $2)
             ON CONFLICT (host_id, group_id) DO NOTHING",
        )
        .bind(new_id)
        .bind(group_id)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;

    audit_service::log_operator_action(
        &state.db,
        &headers,
        actor_user_id,
        "asset",
        "create_host",
        serde_json::json!({
            "id": new_id,
            "name": audit_name,
            "address": audit_address,
            "groupIdList": audit_group_ids,
        }),
        1,
        None,
    )
    .await?;

    Ok(OrionResponse::ok(new_id))
}

async fn orion_update_host(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<OrionHostUpdateRequest>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "host.update").await?;
    let actor_user_id = guard::current_user_id(&headers, &state.config.jwt)?;

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
    let group_ids = normalize_group_ids(payload.group_id_list)?;
    ensure_groups_exist(&state, &group_ids).await?;
    let audit_name = name.clone();
    let audit_address = address.clone();
    let audit_group_ids = group_ids.clone();

    let mut tx = state.db.begin().await?;
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
    .execute(&mut *tx)
    .await?
    .rows_affected();

    if rows == 0 {
        return Err(AppError::NotFound("Host not found".to_string()));
    }

    sqlx::query("DELETE FROM host_group_rel WHERE host_id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    for group_id in group_ids {
        sqlx::query(
            "INSERT INTO host_group_rel (host_id, group_id)
             VALUES ($1, $2)
             ON CONFLICT (host_id, group_id) DO NOTHING",
        )
        .bind(id)
        .bind(group_id)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;

    audit_service::log_operator_action(
        &state.db,
        &headers,
        actor_user_id,
        "asset",
        "update_host",
        serde_json::json!({
            "id": id,
            "name": audit_name,
            "address": audit_address,
            "groupIdList": audit_group_ids,
        }),
        1,
        None,
    )
    .await?;

    Ok(OrionResponse::ok(true))
}

async fn orion_update_host_status(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<OrionHostUpdateStatusRequest>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "host.update").await?;
    let actor_user_id = guard::current_user_id(&headers, &state.config.jwt)?;

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

    audit_service::log_operator_action(
        &state.db,
        &headers,
        actor_user_id,
        "asset",
        "update_host_status",
        serde_json::json!({
            "id": id,
            "status": status_text,
        }),
        1,
        None,
    )
    .await?;

    Ok(OrionResponse::ok(true))
}

fn host_extra_cache_key(user_id: i64, host_id: i64, item: &str) -> String {
    format!(
        "orion:host-extra:user:{user_id}:host:{host_id}:item:{}",
        item.trim().to_ascii_uppercase()
    )
}

fn host_config_cache_key(host_id: i64, config_type: &str) -> String {
    format!(
        "orion:host-config:host:{host_id}:type:{}",
        config_type.trim().to_ascii_uppercase()
    )
}

fn normalize_host_config_type(raw: Option<String>) -> AppResult<String> {
    let value = raw
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .ok_or_else(|| AppError::BadRequest("type is required".to_string()))?
        .to_ascii_uppercase();
    match value.as_str() {
        "SSH" | "RDP" | "VNC" => Ok(value),
        _ => Err(AppError::BadRequest(
            "type must be one of SSH, RDP, VNC".to_string(),
        )),
    }
}

async fn ensure_host_exists(state: &AppState, host_id: i64) -> AppResult<()> {
    let exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM host WHERE id = $1 AND deleted = 0)",
    )
    .bind(host_id)
    .fetch_one(&state.db)
    .await?;
    if !exists {
        return Err(AppError::NotFound("Host not found".to_string()));
    }
    Ok(())
}

async fn apply_ssh_host_config(
    state: &AppState,
    host_id: i64,
    config: &serde_json::Value,
) -> AppResult<()> {
    let mut tx = state.db.begin().await?;

    let username = config
        .get("username")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(str::to_string);
    let port = config
        .get("port")
        .and_then(serde_json::Value::as_i64)
        .map(|v| v as i32);

    sqlx::query(
        "UPDATE host
         SET username = COALESCE($1, username),
             port = CASE WHEN $2 IS NULL OR $2 <= 0 THEN port ELSE $2 END,
             update_time = NOW()
         WHERE id = $3 AND deleted = 0",
    )
    .bind(username)
    .bind(port)
    .bind(host_id)
    .execute(&mut *tx)
    .await?;

    let auth_type = config
        .get("authType")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .unwrap_or("PASSWORD")
        .to_ascii_uppercase();

    match auth_type.as_str() {
        "KEY" => {
            let key_id = config
                .get("keyId")
                .and_then(serde_json::Value::as_i64)
                .ok_or_else(|| {
                    AppError::BadRequest("keyId is required when authType=KEY".to_string())
                })?;
            if key_id <= 0 {
                return Err(AppError::BadRequest(
                    "keyId must be greater than 0".to_string(),
                ));
            }
            let key_exists = sqlx::query_scalar::<_, bool>(
                "SELECT EXISTS(SELECT 1 FROM ssh_key WHERE id = $1 AND deleted = 0 AND status = 1)",
            )
            .bind(key_id)
            .fetch_one(&mut *tx)
            .await?;
            if !key_exists {
                return Err(AppError::BadRequest(
                    "keyId does not exist or is disabled".to_string(),
                ));
            }

            sqlx::query(
                "UPDATE host SET credential_type = 'ssh_key', credential_data = NULL, update_time = NOW()
                 WHERE id = $1 AND deleted = 0",
            )
            .bind(host_id)
            .execute(&mut *tx)
            .await?;

            sqlx::query("UPDATE host_ssh_key_binding SET is_default = 0 WHERE host_id = $1")
                .bind(host_id)
                .execute(&mut *tx)
                .await?;

            sqlx::query(
                "INSERT INTO host_ssh_key_binding (host_id, ssh_key_id, is_default, create_time)
                 VALUES ($1, $2, 1, NOW())
                 ON CONFLICT (host_id, ssh_key_id)
                 DO UPDATE SET is_default = 1",
            )
            .bind(host_id)
            .bind(key_id)
            .execute(&mut *tx)
            .await?;
        }
        "PASSWORD" => {
            let use_new_password = config
                .get("useNewPassword")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false);
            if use_new_password {
                let encrypted_password = config
                    .get("password")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_string)
                    .ok_or_else(|| {
                        AppError::BadRequest(
                            "password is required when useNewPassword=true".to_string(),
                        )
                    })?;
                let password =
                    decrypt_client_sensitive_input(state, Some(encrypted_password), "password")
                        .await?
                        .and_then(|v| sanitize_search(Some(v)))
                        .ok_or_else(|| {
                            AppError::BadRequest(
                                "password is required when useNewPassword=true".to_string(),
                            )
                        })?;
                let payload = serde_json::json!({"kind": "password", "password": password});
                let encrypted = security::encrypt_secret(
                    &payload.to_string(),
                    &state.config.secrets.data_encryption_key,
                )?;
                sqlx::query(
                    "UPDATE host
                     SET credential_type = 'password', credential_data = $2, update_time = NOW()
                     WHERE id = $1 AND deleted = 0",
                )
                .bind(host_id)
                .bind(encrypted)
                .execute(&mut *tx)
                .await?;
            } else {
                sqlx::query(
                    "UPDATE host
                     SET credential_type = 'password', update_time = NOW()
                     WHERE id = $1 AND deleted = 0",
                )
                .bind(host_id)
                .execute(&mut *tx)
                .await?;
            }
        }
        "IDENTITY" => {
            let identity_id = config
                .get("identityId")
                .and_then(serde_json::Value::as_i64)
                .ok_or_else(|| {
                    AppError::BadRequest(
                        "identityId is required when authType=IDENTITY".to_string(),
                    )
                })?;
            if identity_id <= 0 {
                return Err(AppError::BadRequest(
                    "identityId must be greater than 0".to_string(),
                ));
            }

            let identity =
                sqlx::query_as::<_, (String, Option<String>, Option<String>, Option<i64>)>(
                    "SELECT type, username, password_ciphertext, key_id
                 FROM host_identity
                 WHERE id = $1 AND deleted = 0 AND status = 1",
                )
                .bind(identity_id)
                .fetch_optional(&mut *tx)
                .await?
                .ok_or_else(|| {
                    AppError::BadRequest("identityId does not exist or is disabled".to_string())
                })?;

            if let Some(identity_username) = identity
                .1
                .as_deref()
                .map(str::trim)
                .filter(|v| !v.is_empty())
            {
                sqlx::query("UPDATE host SET username = $2, update_time = NOW() WHERE id = $1 AND deleted = 0")
                    .bind(host_id)
                    .bind(identity_username)
                    .execute(&mut *tx)
                    .await?;
            }

            match identity.0.as_str() {
                "PASSWORD" => {
                    let ciphertext = identity.2.as_deref().ok_or_else(|| {
                        AppError::BadRequest("identity password is missing".to_string())
                    })?;
                    let password = security::decrypt_secret(
                        ciphertext,
                        &state.config.secrets.data_encryption_key,
                    )?;
                    let payload = serde_json::json!({"kind": "password", "password": password});
                    let encrypted = security::encrypt_secret(
                        &payload.to_string(),
                        &state.config.secrets.data_encryption_key,
                    )?;
                    sqlx::query(
                        "UPDATE host
                         SET credential_type = 'password', credential_data = $2, update_time = NOW()
                         WHERE id = $1 AND deleted = 0",
                    )
                    .bind(host_id)
                    .bind(encrypted)
                    .execute(&mut *tx)
                    .await?;
                }
                "KEY" => {
                    let key_id = identity.3.ok_or_else(|| {
                        AppError::BadRequest("identity key is missing".to_string())
                    })?;

                    sqlx::query(
                        "UPDATE host SET credential_type = 'ssh_key', credential_data = NULL, update_time = NOW()
                         WHERE id = $1 AND deleted = 0",
                    )
                    .bind(host_id)
                    .execute(&mut *tx)
                    .await?;

                    sqlx::query(
                        "UPDATE host_ssh_key_binding SET is_default = 0 WHERE host_id = $1",
                    )
                    .bind(host_id)
                    .execute(&mut *tx)
                    .await?;

                    sqlx::query(
                        "INSERT INTO host_ssh_key_binding (host_id, ssh_key_id, is_default, create_time)
                         VALUES ($1, $2, 1, NOW())
                         ON CONFLICT (host_id, ssh_key_id)
                         DO UPDATE SET is_default = 1",
                    )
                    .bind(host_id)
                    .bind(key_id)
                    .execute(&mut *tx)
                    .await?;
                }
                _ => {
                    return Err(AppError::BadRequest(
                        "identity type must be PASSWORD or KEY".to_string(),
                    ))
                }
            }
        }
        _ => {
            return Err(AppError::BadRequest(
                "authType must be one of PASSWORD, KEY, IDENTITY".to_string(),
            ))
        }
    }

    tx.commit().await?;
    Ok(())
}

async fn orion_host_extra_get(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<OrionHostExtraQuery>,
) -> AppResult<impl axum::response::IntoResponse> {
    let user_id = guard::current_user_id(&headers, &state.config.jwt)?;
    let host_id = parse_required_id(query.host_id, "hostId")?;
    let item = query
        .item
        .map(|v| v.trim().to_ascii_uppercase())
        .filter(|v| !v.is_empty())
        .ok_or_else(|| AppError::BadRequest("item is required".to_string()))?;

    let key = host_extra_cache_key(user_id, host_id, &item);
    let value = sqlx::query_scalar::<_, String>(
        "SELECT cache_value FROM cache
         WHERE cache_key = $1
           AND (expire_time IS NULL OR expire_time > NOW())",
    )
    .bind(key)
    .fetch_optional(&state.db)
    .await?;

    let extra = value
        .as_deref()
        .and_then(|raw| serde_json::from_str::<serde_json::Value>(raw).ok())
        .unwrap_or_else(|| serde_json::json!({}));

    Ok(OrionResponse::ok(extra))
}

async fn orion_host_extra_update(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<OrionHostExtraUpdateRequest>,
) -> AppResult<impl axum::response::IntoResponse> {
    let user_id = guard::current_user_id(&headers, &state.config.jwt)?;
    let host_id = parse_required_id(payload.host_id, "hostId")?;
    let item = payload
        .item
        .map(|v| v.trim().to_ascii_uppercase())
        .filter(|v| !v.is_empty())
        .ok_or_else(|| AppError::BadRequest("item is required".to_string()))?;
    let extra_raw = payload
        .extra
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .ok_or_else(|| AppError::BadRequest("extra is required".to_string()))?;

    let extra: serde_json::Value = serde_json::from_str(extra_raw)
        .map_err(|_| AppError::BadRequest("extra must be valid JSON".to_string()))?;

    let cache_key = host_extra_cache_key(user_id, host_id, &item);
    let payload = serde_json::to_string(&extra).unwrap_or_else(|_| "{}".to_string());
    sqlx::query(
        "INSERT INTO cache (cache_key, cache_value, expire_time, create_time)
         VALUES ($1, $2, NULL, NOW())
         ON CONFLICT (cache_key)
         DO UPDATE SET cache_value = EXCLUDED.cache_value, create_time = NOW()",
    )
    .bind(cache_key)
    .bind(payload)
    .execute(&state.db)
    .await?;

    Ok(OrionResponse::ok(true))
}

async fn orion_host_config_get(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<OrionHostConfigGetRequest>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "host.read").await?;

    let host_id = parse_required_id(payload.host_id, "hostId")?;
    let config_type = normalize_host_config_type(payload.r#type)?;
    ensure_host_exists(&state, host_id).await?;

    let cache_key = host_config_cache_key(host_id, &config_type);
    if let Some(raw) = sqlx::query_scalar::<_, String>(
        "SELECT cache_value FROM cache
         WHERE cache_key = $1
           AND (expire_time IS NULL OR expire_time > NOW())",
    )
    .bind(cache_key)
    .fetch_optional(&state.db)
    .await?
    {
        let value = serde_json::from_str::<serde_json::Value>(&raw)
            .unwrap_or_else(|_| serde_json::json!({}));
        return Ok(OrionResponse::ok(value));
    }

    if config_type != "SSH" {
        return Ok(OrionResponse::ok(serde_json::json!({})));
    }

    let row = sqlx::query_as::<_, (Option<String>, i32, Option<String>, Option<i64>, bool)>(
        "SELECT h.username,
                h.port,
                h.credential_type,
                (
                    SELECT hb.ssh_key_id
                    FROM host_ssh_key_binding hb
                    WHERE hb.host_id = h.id AND hb.is_default = 1
                    LIMIT 1
                ) AS key_id,
                CASE WHEN h.credential_type IN ('password', 'private_key') AND h.credential_data IS NOT NULL THEN true ELSE false END AS has_password
         FROM host h
         WHERE h.id = $1 AND h.deleted = 0",
    )
    .bind(host_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound("Host not found".to_string()))?;

    let auth_type = match row.2.as_deref() {
        Some("ssh_key") => "KEY",
        Some("password") | Some("private_key") => "PASSWORD",
        _ => "PASSWORD",
    };

    Ok(OrionResponse::ok(serde_json::json!({
        "port": row.1,
        "username": row.0.unwrap_or_default(),
        "authType": auth_type,
        "keyId": row.3,
        "hasPassword": row.4,
        "connectTimeout": 30000,
        "charset": "utf-8",
        "fileNameCharset": "utf-8",
        "fileContentCharset": "utf-8"
    })))
}

async fn orion_host_config_update(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<OrionHostConfigUpdateRequest>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "host.update").await?;

    let host_id = parse_required_id(payload.host_id, "hostId")?;
    let config_type = normalize_host_config_type(payload.r#type)?;
    let config_text = payload
        .config
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .ok_or_else(|| AppError::BadRequest("config is required".to_string()))?;
    let config: serde_json::Value = serde_json::from_str(config_text)
        .map_err(|_| AppError::BadRequest("config must be valid JSON".to_string()))?;

    ensure_host_exists(&state, host_id).await?;

    let cache_key = host_config_cache_key(host_id, &config_type);
    let cache_value = serde_json::to_string(&config).unwrap_or_else(|_| "{}".to_string());
    sqlx::query(
        "INSERT INTO cache (cache_key, cache_value, expire_time, create_time)
         VALUES ($1, $2, NULL, NOW())
         ON CONFLICT (cache_key)
         DO UPDATE SET cache_value = EXCLUDED.cache_value, create_time = NOW()",
    )
    .bind(cache_key)
    .bind(cache_value)
    .execute(&state.db)
    .await?;

    if config_type == "SSH" {
        apply_ssh_host_config(&state, host_id, &config).await?;
    }

    Ok(OrionResponse::ok(true))
}

async fn orion_delete_host(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<OrionHostIdQuery>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "host.delete").await?;
    let actor_user_id = guard::current_user_id(&headers, &state.config.jwt)?;
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

    audit_service::log_operator_action(
        &state.db,
        &headers,
        actor_user_id,
        "asset",
        "delete_host",
        serde_json::json!({ "id": query.id }),
        1,
        None,
    )
    .await?;

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
    let actor_user_id = guard::current_user_id(&headers, &state.config.jwt)?;

    let name = sanitize_search(payload.name)
        .ok_or_else(|| AppError::BadRequest("name is required".to_string()))?;
    let private_key_input = payload
        .private_key
        .ok_or_else(|| AppError::BadRequest("privateKey is required".to_string()))?;
    let private_key_plain =
        decrypt_client_sensitive_input(&state, Some(private_key_input), "privateKey")
            .await?
            .ok_or_else(|| AppError::BadRequest("privateKey is required".to_string()))?;
    let private_key = normalize_and_validate_private_key(&private_key_plain)?;

    let private_key_ciphertext =
        security::encrypt_secret(&private_key, &state.config.secrets.data_encryption_key)?;
    let audit_name = name.clone();
    let audit_description = payload.description.clone();
    let password_plain =
        decrypt_client_sensitive_input(&state, payload.password, "password").await?;
    let passphrase_ciphertext = match sanitize_search(password_plain) {
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

    audit_service::log_operator_action(
        &state.db,
        &headers,
        actor_user_id,
        "asset",
        "create_host_key",
        serde_json::json!({
            "id": id,
            "name": audit_name,
            "description": audit_description,
        }),
        1,
        None,
    )
    .await?;

    Ok(OrionResponse::ok(id))
}

async fn orion_host_key_update(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<OrionHostKeyUpsertRequest>,
) -> AppResult<impl axum::response::IntoResponse> {
    let actor_user_id = guard::current_user_id(&headers, &state.config.jwt)?;
    let id = parse_required_id(payload.id, "id")?;

    let name = sanitize_search(payload.name);
    let description = sanitize_search(payload.description);
    let audit_name = name.clone();
    let audit_description = description.clone();

    let private_key_plain =
        decrypt_client_sensitive_input(&state, payload.private_key, "privateKey").await?;
    let private_key_ciphertext = match private_key_plain.as_deref() {
        Some(v) => {
            let normalized = normalize_and_validate_private_key(v)?;
            Some(security::encrypt_secret(
                &normalized,
                &state.config.secrets.data_encryption_key,
            )?)
        }
        None => None,
    };

    let use_new_password = payload.use_new_password.unwrap_or(false);
    let password_plain =
        decrypt_client_sensitive_input(&state, payload.password, "password").await?;
    let passphrase_ciphertext = if use_new_password {
        match sanitize_search(password_plain) {
            Some(v) => Some(Some(security::encrypt_secret(
                &v,
                &state.config.secrets.data_encryption_key,
            )?)),
            None => Some(None),
        }
    } else {
        None
    };

    if name.is_none()
        && description.is_none()
        && private_key_ciphertext.is_none()
        && !use_new_password
    {
        return Err(AppError::BadRequest("No fields to update".to_string()));
    }

    asset_service::update_host_key(
        &state.db,
        asset_service::OrionHostKeyUpdateInput {
            id,
            name,
            private_key_ciphertext,
            use_new_password,
            passphrase_ciphertext,
            description,
        },
    )
    .await?;

    audit_service::log_operator_action(
        &state.db,
        &headers,
        actor_user_id,
        "asset",
        "update_host_key",
        serde_json::json!({
            "id": id,
            "name": audit_name,
            "description": audit_description,
            "useNewPassword": use_new_password,
            "updatedPrivateKey": private_key_plain.is_some(),
        }),
        1,
        None,
    )
    .await?;

    Ok(OrionResponse::ok(true))
}

async fn orion_host_key_get(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<OrionIdQuery>,
) -> AppResult<impl axum::response::IntoResponse> {
    let _user_id = guard::current_user_id(&headers, &state.config.jwt)?;
    let id = parse_required_id(query.id, "id")?;

    let row = asset_service::get_host_key(&state.db, id).await?;
    Ok(OrionResponse::ok(map_host_key_item(row)))
}

async fn orion_host_key_list(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<impl axum::response::IntoResponse> {
    let _user_id = guard::current_user_id(&headers, &state.config.jwt)?;

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
    let _user_id = guard::current_user_id(&headers, &state.config.jwt)?;
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
    let actor_user_id = guard::current_user_id(&headers, &state.config.jwt)?;
    let id = parse_required_id(query.id, "id")?;

    asset_service::delete_host_key(&state.db, id).await?;

    audit_service::log_operator_action(
        &state.db,
        &headers,
        actor_user_id,
        "asset",
        "delete_host_key",
        serde_json::json!({ "id": id }),
        1,
        None,
    )
    .await?;

    Ok(OrionResponse::ok(true))
}

async fn orion_host_key_batch_delete(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<OrionDeleteIdsQuery>,
) -> AppResult<impl axum::response::IntoResponse> {
    let actor_user_id = guard::current_user_id(&headers, &state.config.jwt)?;
    let id_list = sanitize_search(query.id_list)
        .ok_or_else(|| AppError::BadRequest("idList is required".to_string()))?;

    let ids = id_list
        .split(',')
        .filter_map(|v| v.trim().parse::<i64>().ok())
        .collect::<Vec<_>>();
    let audit_ids = ids.clone();

    asset_service::batch_delete_host_keys(&state.db, ids).await?;

    audit_service::log_operator_action(
        &state.db,
        &headers,
        actor_user_id,
        "asset",
        "batch_delete_host_key",
        serde_json::json!({ "ids": audit_ids }),
        1,
        None,
    )
    .await?;

    Ok(OrionResponse::ok(true))
}

async fn orion_host_identity_create(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<OrionHostIdentityUpsertRequest>,
) -> AppResult<impl axum::response::IntoResponse> {
    let _user_id = guard::current_user_id(&headers, &state.config.jwt)?;
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

    let password_plain =
        decrypt_client_sensitive_input(&state, payload.password, "password").await?;
    let password_ciphertext = match sanitize_search(password_plain) {
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
    let _user_id = guard::current_user_id(&headers, &state.config.jwt)?;
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
    let password_plain =
        decrypt_client_sensitive_input(&state, payload.password, "password").await?;
    let password_ciphertext = if use_new_password {
        match sanitize_search(password_plain) {
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
    let _user_id = guard::current_user_id(&headers, &state.config.jwt)?;
    let id = parse_required_id(query.id, "id")?;

    let item = asset_service::get_host_identity(&state.db, id).await?;
    Ok(OrionResponse::ok(map_host_identity_item(item)))
}

async fn orion_host_identity_list(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<impl axum::response::IntoResponse> {
    let _user_id = guard::current_user_id(&headers, &state.config.jwt)?;
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
    let _user_id = guard::current_user_id(&headers, &state.config.jwt)?;
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
    let _user_id = guard::current_user_id(&headers, &state.config.jwt)?;
    let id = parse_required_id(query.id, "id")?;
    asset_service::delete_host_identity(&state.db, id).await?;
    Ok(OrionResponse::ok(true))
}

async fn orion_host_identity_batch_delete(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<OrionDeleteIdsQuery>,
) -> AppResult<impl axum::response::IntoResponse> {
    let _user_id = guard::current_user_id(&headers, &state.config.jwt)?;
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
    let _user_id = guard::current_user_id(&headers, &state.config.jwt)?;
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
    let _user_id = guard::current_user_id(&headers, &state.config.jwt)?;
    let scope = asset_service::resolve_grant_scope(query.user_id, query.role_id)?;
    let list = asset_service::list_asset_grants(&state.db, scope, "host-group").await?;
    Ok(OrionResponse::ok(list))
}

async fn orion_data_grant_host_key(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<OrionAssetDataGrantRequest>,
) -> AppResult<impl axum::response::IntoResponse> {
    let _user_id = guard::current_user_id(&headers, &state.config.jwt)?;
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
    let _user_id = guard::current_user_id(&headers, &state.config.jwt)?;
    let scope = asset_service::resolve_grant_scope(query.user_id, query.role_id)?;
    let list = asset_service::list_asset_grants(&state.db, scope, "host-key").await?;
    Ok(OrionResponse::ok(list))
}

async fn orion_data_grant_host_identity(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<OrionAssetDataGrantRequest>,
) -> AppResult<impl axum::response::IntoResponse> {
    let _user_id = guard::current_user_id(&headers, &state.config.jwt)?;
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
    let _user_id = guard::current_user_id(&headers, &state.config.jwt)?;
    let scope = asset_service::resolve_grant_scope(query.user_id, query.role_id)?;
    let list = asset_service::list_asset_grants(&state.db, scope, "host-identity").await?;
    Ok(OrionResponse::ok(list))
}

/// GET /asset/authorized-data/current-host
/// Returns authorized hosts for the current user with group tree structure
async fn orion_authorized_data_current_host(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(_query): Query<OrionHostListQuery>,
) -> AppResult<impl axum::response::IntoResponse> {
    let user_id = guard::current_user_id(&headers, &state.config.jwt)?;

    // Get user's authorized host groups (both direct user grants and role-based grants)
    let authorized_group_ids = sqlx::query_scalar::<_, i64>(
        "SELECT DISTINCT group_id FROM (
            SELECT group_id FROM user_host_group_grant WHERE user_id = $1
            UNION
            SELECT group_id FROM role_host_group_grant 
            WHERE role_id IN (
                SELECT role_id FROM sys_user_role WHERE user_id = $1
            )
        ) AS authorized_groups",
    )
    .bind(user_id)
    .fetch_all(&state.db)
    .await?;

    // If no authorized groups, return empty response
    if authorized_group_ids.is_empty() {
        return Ok(OrionResponse::ok(serde_json::json!({
            "groupTree": [],
            "hostList": [],
            "treeNodes": {},
            "latestHosts": []
        })));
    }

    // Build group tree
    let group_rows = sqlx::query_as::<_, (i64, i64, String)>(
        "SELECT id, parent_id, name FROM host_group WHERE deleted = 0 ORDER BY sort, id",
    )
    .fetch_all(&state.db)
    .await?;

    let group_tree = build_host_group_tree(group_rows);

    // Get hosts in authorized groups
    let host_rows = sqlx::query_as::<_, (i64, String, String, i32, i64, i64)>(
        "SELECT DISTINCT h.id, h.name, h.hostname, h.port,
         COALESCE((EXTRACT(EPOCH FROM h.create_time) * 1000)::BIGINT, 0),
         COALESCE((EXTRACT(EPOCH FROM h.update_time) * 1000)::BIGINT, 0)
         FROM host h
         JOIN host_group_rel hgr ON h.id = hgr.host_id
         WHERE hgr.group_id = ANY($1) AND h.deleted = 0
         ORDER BY h.id",
    )
    .bind(&authorized_group_ids)
    .fetch_all(&state.db)
    .await?;

    // Build host list with group associations
    let mut host_list = Vec::new();
    let mut tree_nodes: HashMap<String, Vec<i64>> = HashMap::new();

    for (host_id, name, hostname, port, create_time_ms, update_time_ms) in host_rows {
        // Get group IDs for this host
        let group_ids =
            sqlx::query_scalar::<_, i64>("SELECT group_id FROM host_group_rel WHERE host_id = $1")
                .bind(host_id)
                .fetch_all(&state.db)
                .await?;

        // Filter to only authorized groups
        let authorized_groups: Vec<i64> = group_ids
            .into_iter()
            .filter(|gid| authorized_group_ids.contains(gid))
            .collect();

        // Add to tree_nodes mapping
        for group_id in &authorized_groups {
            tree_nodes
                .entry(group_id.to_string())
                .or_insert_with(Vec::new)
                .push(host_id);
        }

        host_list.push(serde_json::json!({
            "id": host_id,
            "types": ["SSH"],
            "osType": "linux",
            "archType": "x86_64",
            "name": name.clone(),
            "code": format!("host-{}", host_id),
            "address": hostname,
            "port": port,
            "status": "ENABLED",
            "agentKey": "",
            "agentVersion": "",
            "agentInstallStatus": 0,
            "agentOnlineStatus": 0,
            "agentOnlineChangeTime": 0,
            "description": "",
            "groupIdList": authorized_groups,
            "alias": name,
            "color": "",
            "tags": [],
            "spec": {},
            "favorite": false,
            "editable": true,
            "loading": false,
            "modCount": 0,
            "createTime": create_time_ms,
            "updateTime": update_time_ms,
        }));
    }

    Ok(OrionResponse::ok(serde_json::json!({
        "groupTree": group_tree,
        "hostList": host_list,
        "treeNodes": tree_nodes,
        "latestHosts": []
    })))
}

/// GET /asset/authorized-data/current-host-key
/// Returns authorized SSH keys for the current user
async fn orion_authorized_data_current_host_key(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<impl axum::response::IntoResponse> {
    let user_id = guard::current_user_id(&headers, &state.config.jwt)?;

    // Get authorized key IDs (both direct user grants and role-based grants)
    let authorized_key_ids = sqlx::query_scalar::<_, i64>(
        "SELECT DISTINCT key_id FROM (
            SELECT key_id FROM user_host_key_grant WHERE user_id = $1
            UNION
            SELECT key_id FROM role_host_key_grant 
            WHERE role_id IN (
                SELECT role_id FROM sys_user_role WHERE user_id = $1
            )
        ) AS authorized_keys",
    )
    .bind(user_id)
    .fetch_all(&state.db)
    .await?;

    if authorized_key_ids.is_empty() {
        return Ok(OrionResponse::ok(Vec::<serde_json::Value>::new()));
    }

    // Get key details
    let keys = sqlx::query_as::<_, (i64, String, Option<String>, i64, i64)>(
        "SELECT id, name, description, 
         COALESCE((EXTRACT(EPOCH FROM create_time) * 1000)::BIGINT, 0),
         COALESCE((EXTRACT(EPOCH FROM update_time) * 1000)::BIGINT, 0)
         FROM ssh_key
         WHERE id = ANY($1) AND deleted = 0
         ORDER BY id",
    )
    .bind(&authorized_key_ids)
    .fetch_all(&state.db)
    .await?;

    let key_list: Vec<serde_json::Value> = keys
        .into_iter()
        .map(|(id, name, description, create_time, update_time)| {
            serde_json::json!({
                "id": id,
                "name": name,
                "description": description.unwrap_or_default(),
                "createTime": create_time,
                "updateTime": update_time,
            })
        })
        .collect();

    Ok(OrionResponse::ok(key_list))
}

/// GET /asset/authorized-data/current-host-identity
/// Returns authorized host identities for the current user
async fn orion_authorized_data_current_host_identity(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<impl axum::response::IntoResponse> {
    let user_id = guard::current_user_id(&headers, &state.config.jwt)?;

    // Get authorized identity IDs (both direct user grants and role-based grants)
    let authorized_identity_ids = sqlx::query_scalar::<_, i64>(
        "SELECT DISTINCT identity_id FROM (
            SELECT identity_id FROM user_host_identity_grant WHERE user_id = $1
            UNION
            SELECT identity_id FROM role_host_identity_grant 
            WHERE role_id IN (
                SELECT role_id FROM sys_user_role WHERE user_id = $1
            )
        ) AS authorized_identities",
    )
    .bind(user_id)
    .fetch_all(&state.db)
    .await?;

    if authorized_identity_ids.is_empty() {
        return Ok(OrionResponse::ok(Vec::<serde_json::Value>::new()));
    }

    // Get identity details
    let identities = sqlx::query_as::<
        _,
        (
            i64,
            String,
            String,
            Option<String>,
            Option<i64>,
            Option<String>,
            i64,
            i64,
        ),
    >(
        "SELECT id, name, type, username, key_id, description,
         COALESCE((EXTRACT(EPOCH FROM create_time) * 1000)::BIGINT, 0),
         COALESCE((EXTRACT(EPOCH FROM update_time) * 1000)::BIGINT, 0)
         FROM host_identity
         WHERE id = ANY($1) AND deleted = 0
         ORDER BY id",
    )
    .bind(&authorized_identity_ids)
    .fetch_all(&state.db)
    .await?;

    let identity_list: Vec<serde_json::Value> = identities
        .into_iter()
        .map(
            |(id, name, identity_type, username, key_id, description, create_time, update_time)| {
                serde_json::json!({
                    "id": id,
                    "name": name,
                    "type": identity_type,
                    "username": username.unwrap_or_default(),
                    "keyId": key_id,
                    "description": description.unwrap_or_default(),
                    "createTime": create_time,
                    "updateTime": update_time,
                })
            },
        )
        .collect();

    Ok(OrionResponse::ok(identity_list))
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

fn normalize_group_ids(ids: Option<Vec<i64>>) -> AppResult<Vec<i64>> {
    let mut set = HashSet::new();
    for id in ids.unwrap_or_default() {
        if id <= 0 {
            return Err(AppError::BadRequest(
                "groupIdList must contain positive ids".to_string(),
            ));
        }
        set.insert(id);
    }
    if set.is_empty() {
        return Err(AppError::BadRequest("groupIdList is required".to_string()));
    }
    Ok(set.into_iter().collect())
}

async fn ensure_groups_exist(state: &AppState, group_ids: &[i64]) -> AppResult<()> {
    let count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(1) FROM host_group WHERE deleted = 0 AND id = ANY($1::bigint[])",
    )
    .bind(group_ids)
    .fetch_one(&state.db)
    .await?;

    if count != group_ids.len() as i64 {
        return Err(AppError::BadRequest(
            "groupIdList contains non-existent host group id".to_string(),
        ));
    }
    Ok(())
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
    let user_id = guard::current_user_id(&headers, &state.config.jwt)?;
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

async fn query_terminal_module(
    state: &AppState,
    module: OrionCompatModule,
    payload: &serde_json::Value,
    query: &OrionCompatQuery,
) -> AppResult<serde_json::Value> {
    let (page, limit) = payload_page_limit(payload, query);
    let mut rows = compat_service::list_records(&state.db, module).await?;
    rows = filter_terminal_module_rows(module, rows, payload, query)?;

    let total = rows.len() as i64;
    let offset = ((page - 1) * limit) as usize;
    let rows = rows
        .into_iter()
        .skip(offset)
        .take(limit as usize)
        .collect::<Vec<_>>();
    Ok(serde_json::json!({"page": page, "limit": limit, "total": total, "rows": rows}))
}

async fn count_terminal_module(
    state: &AppState,
    module: OrionCompatModule,
    payload: &serde_json::Value,
    query: &OrionCompatQuery,
) -> AppResult<i64> {
    let rows = compat_service::list_records(&state.db, module).await?;
    Ok(filter_terminal_module_rows(module, rows, payload, query)?.len() as i64)
}

fn payload_i64(payload: &serde_json::Value, key: &str) -> Option<i64> {
    payload.get(key).and_then(|v| match v {
        serde_json::Value::Number(n) => n.as_i64(),
        serde_json::Value::String(s) => s.trim().parse::<i64>().ok(),
        _ => None,
    })
}

fn payload_i32(payload: &serde_json::Value, key: &str) -> Option<i32> {
    payload_i64(payload, key).and_then(|v| i32::try_from(v).ok())
}

fn payload_string(payload: &serde_json::Value, key: &str) -> Option<String> {
    payload
        .get(key)
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToString::to_string)
}

fn parse_terminal_time_value_ms(value: &serde_json::Value) -> AppResult<i64> {
    match value {
        serde_json::Value::Number(n) => n
            .as_i64()
            .ok_or_else(|| AppError::BadRequest("invalid startTimeRange value".to_string())),
        serde_json::Value::String(raw) => {
            let raw = raw.trim();
            if raw.is_empty() {
                return Err(AppError::BadRequest(
                    "startTimeRange value cannot be empty".to_string(),
                ));
            }
            if let Ok(ms) = raw.parse::<i64>() {
                return Ok(ms);
            }
            Ok(parse_datetime_text(raw)?.timestamp_millis())
        }
        _ => Err(AppError::BadRequest(
            "invalid startTimeRange value".to_string(),
        )),
    }
}

fn parse_terminal_start_time_range_ms(
    payload: &serde_json::Value,
) -> AppResult<Option<(i64, i64)>> {
    let Some(values) = payload
        .get("startTimeRange")
        .and_then(serde_json::Value::as_array)
    else {
        return Ok(None);
    };

    if values.is_empty() {
        return Ok(None);
    }
    if values.len() != 2 {
        return Err(AppError::BadRequest(
            "startTimeRange must include exactly 2 values".to_string(),
        ));
    }

    let start_ms = parse_terminal_time_value_ms(&values[0])?;
    let end_ms = parse_terminal_time_value_ms(&values[1])?;
    if end_ms < start_ms {
        return Err(AppError::BadRequest(
            "startTimeRange end cannot be earlier than start".to_string(),
        ));
    }
    Ok(Some((start_ms, end_ms)))
}

fn filter_terminal_module_rows(
    module: OrionCompatModule,
    mut rows: Vec<serde_json::Value>,
    payload: &serde_json::Value,
    query: &OrionCompatQuery,
) -> AppResult<Vec<serde_json::Value>> {
    if let Some(id) = payload_i64(payload, "id") {
        rows.retain(|r| r.get("id").and_then(serde_json::Value::as_i64) == Some(id));
    }

    if let Some(user_id) = payload_i64(payload, "userId") {
        rows.retain(|r| r.get("userId").and_then(serde_json::Value::as_i64) == Some(user_id));
    }

    if let Some(host_id) = payload_i64(payload, "hostId") {
        rows.retain(|r| r.get("hostId").and_then(serde_json::Value::as_i64) == Some(host_id));
    }

    if let Some(host_address) = payload_string(payload, "hostAddress") {
        rows.retain(|r| {
            r.get("hostAddress")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .contains(&host_address)
        });
    }

    if let Some(typ) = payload_string(payload, "type") {
        rows.retain(|r| r.get("type").and_then(serde_json::Value::as_str) == Some(typ.as_str()));
    }

    if let Some((start_ms, end_ms)) = parse_terminal_start_time_range_ms(payload)? {
        rows.retain(|r| {
            let start_time = r
                .get("startTime")
                .and_then(serde_json::Value::as_i64)
                .unwrap_or_default();
            start_time >= start_ms && start_time <= end_ms
        });
    }

    match module {
        OrionCompatModule::TerminalConnectLog => {
            if let Some(session_id) = payload_string(payload, "sessionId") {
                rows.retain(|r| {
                    r.get("sessionId").and_then(serde_json::Value::as_str)
                        == Some(session_id.as_str())
                });
            }
            if let Some(status) = payload_string(payload, "status") {
                rows.retain(|r| {
                    r.get("status").and_then(serde_json::Value::as_str) == Some(status.as_str())
                });
            } else {
                rows.retain(|r| {
                    r.get("status").and_then(serde_json::Value::as_str) != Some("CONNECTING")
                });
            }
        }
        OrionCompatModule::TerminalFileLog => {
            if let Some(result) = payload_i32(payload, "result") {
                rows.retain(|r| {
                    r.get("result").and_then(serde_json::Value::as_i64) == Some(result as i64)
                });
            }
        }
        _ => {}
    }

    if let Some(search) = payload_search_value(payload, query).filter(|v| !v.trim().is_empty()) {
        let needle = search.to_ascii_lowercase();
        rows.retain(|row| row.to_string().to_ascii_lowercase().contains(&needle));
    }

    let order = payload_i64(payload, "order").unwrap_or(0);
    if order == 1 {
        rows.sort_by_key(|r| {
            r.get("id")
                .and_then(serde_json::Value::as_i64)
                .unwrap_or_default()
        });
    } else {
        rows.sort_by_key(|r| {
            std::cmp::Reverse(
                r.get("id")
                    .and_then(serde_json::Value::as_i64)
                    .unwrap_or_default(),
            )
        });
    }

    Ok(rows)
}

async fn delete_terminal_module(
    state: &AppState,
    module: OrionCompatModule,
    query: &OrionCompatQuery,
) -> AppResult<bool> {
    let ids = if let Some(id) = query.id {
        vec![id]
    } else {
        parse_csv_i64(query.id_list.as_deref())
    };
    if ids.is_empty() {
        return Err(AppError::BadRequest("id or idList is required".to_string()));
    }
    let affected = compat_service::batch_delete_records(&state.db, module, &ids).await?;
    Ok(affected > 0)
}

async fn clear_terminal_module(state: &AppState, module: OrionCompatModule) -> AppResult<u64> {
    compat_service::clear_records(&state.db, module).await
}

fn filter_terminal_connect_sessions(
    mut rows: Vec<serde_json::Value>,
    payload: &serde_json::Value,
) -> Vec<serde_json::Value> {
    rows.sort_by_key(|r| {
        std::cmp::Reverse(
            r.get("id")
                .and_then(serde_json::Value::as_i64)
                .unwrap_or_default(),
        )
    });
    rows.retain(|r| r.get("status").and_then(serde_json::Value::as_str) == Some("CONNECTING"));

    if let Some(user_id) = payload.get("userId").and_then(serde_json::Value::as_i64) {
        rows.retain(|r| r.get("userId").and_then(serde_json::Value::as_i64) == Some(user_id));
    }

    if let Some(host_id) = payload.get("hostId").and_then(serde_json::Value::as_i64) {
        rows.retain(|r| r.get("hostId").and_then(serde_json::Value::as_i64) == Some(host_id));
    }

    if let Some(host_address) = payload
        .get("hostAddress")
        .and_then(serde_json::Value::as_str)
    {
        let needle = host_address.trim();
        if !needle.is_empty() {
            rows.retain(|r| {
                r.get("hostAddress")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default()
                    .contains(needle)
            });
        }
    }

    if let Some(typ) = payload.get("type").and_then(serde_json::Value::as_str) {
        let needle = typ.trim();
        if !needle.is_empty() {
            rows.retain(|r| r.get("type").and_then(serde_json::Value::as_str) == Some(needle));
        }
    }

    if let Some(session_id) = payload.get("sessionId").and_then(serde_json::Value::as_str) {
        let needle = session_id.trim();
        if !needle.is_empty() {
            rows.retain(|r| r.get("sessionId").and_then(serde_json::Value::as_str) == Some(needle));
        }
    }

    rows
}

async fn orion_terminal_connect_log_query(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<OrionCompatQuery>,
    body: Option<Json<serde_json::Value>>,
) -> AppResult<impl axum::response::IntoResponse> {
    let _ = guard::current_user_id(&headers, &state.config.jwt)?;
    let payload = body_json(body);
    Ok(orion_ok(
        query_terminal_module(
            &state,
            OrionCompatModule::TerminalConnectLog,
            &payload,
            &query,
        )
        .await?,
    ))
}

async fn orion_terminal_connect_log_count(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<OrionCompatQuery>,
    body: Option<Json<serde_json::Value>>,
) -> AppResult<impl axum::response::IntoResponse> {
    let _ = guard::current_user_id(&headers, &state.config.jwt)?;
    let payload = body_json(body);
    Ok(orion_ok(
        count_terminal_module(
            &state,
            OrionCompatModule::TerminalConnectLog,
            &payload,
            &query,
        )
        .await?,
    ))
}

async fn orion_terminal_connect_log_delete(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<OrionCompatQuery>,
) -> AppResult<impl axum::response::IntoResponse> {
    let _ = guard::current_user_id(&headers, &state.config.jwt)?;
    Ok(orion_ok(
        delete_terminal_module(&state, OrionCompatModule::TerminalConnectLog, &query).await?,
    ))
}

async fn orion_terminal_connect_log_clear(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<impl axum::response::IntoResponse> {
    let _ = guard::current_user_id(&headers, &state.config.jwt)?;
    Ok(orion_ok(
        clear_terminal_module(&state, OrionCompatModule::TerminalConnectLog).await?,
    ))
}

async fn orion_terminal_connect_log_sessions(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Option<Json<serde_json::Value>>,
) -> AppResult<impl axum::response::IntoResponse> {
    let _ = guard::current_user_id(&headers, &state.config.jwt)?;
    let payload = body_json(body);
    let rows =
        compat_service::list_records(&state.db, OrionCompatModule::TerminalConnectLog).await?;
    Ok(orion_ok(filter_terminal_connect_sessions(rows, &payload)))
}

async fn orion_terminal_connect_log_latest(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Option<Json<serde_json::Value>>,
) -> AppResult<impl axum::response::IntoResponse> {
    let _ = guard::current_user_id(&headers, &state.config.jwt)?;
    let payload = body_json(body);
    let limit = payload
        .get("limit")
        .and_then(serde_json::Value::as_i64)
        .unwrap_or(10)
        .clamp(1, 100) as usize;
    let rows =
        compat_service::list_records(&state.db, OrionCompatModule::TerminalConnectLog).await?;
    let ids = rows
        .into_iter()
        .filter_map(|r| r.get("hostId").and_then(serde_json::Value::as_i64))
        .take(limit)
        .collect::<Vec<_>>();
    Ok(orion_ok(ids))
}

async fn orion_terminal_connect_log_force_offline(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<OrionCompatQuery>,
    body: Option<Json<serde_json::Value>>,
) -> AppResult<impl axum::response::IntoResponse> {
    let user_id = guard::current_user_id(&headers, &state.config.jwt)?;
    let payload = body_json(body);
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

async fn orion_terminal_file_log_query(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<OrionCompatQuery>,
    body: Option<Json<serde_json::Value>>,
) -> AppResult<impl axum::response::IntoResponse> {
    let _ = guard::current_user_id(&headers, &state.config.jwt)?;
    let payload = body_json(body);
    Ok(orion_ok(
        query_terminal_module(&state, OrionCompatModule::TerminalFileLog, &payload, &query).await?,
    ))
}

async fn orion_terminal_file_log_count(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<OrionCompatQuery>,
    body: Option<Json<serde_json::Value>>,
) -> AppResult<impl axum::response::IntoResponse> {
    let _ = guard::current_user_id(&headers, &state.config.jwt)?;
    let payload = body_json(body);
    Ok(orion_ok(
        count_terminal_module(&state, OrionCompatModule::TerminalFileLog, &payload, &query).await?,
    ))
}

async fn orion_terminal_file_log_delete(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<OrionCompatQuery>,
) -> AppResult<impl axum::response::IntoResponse> {
    let _ = guard::current_user_id(&headers, &state.config.jwt)?;
    Ok(orion_ok(
        delete_terminal_module(&state, OrionCompatModule::TerminalFileLog, &query).await?,
    ))
}

async fn orion_terminal_dispatch(
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
    Path((module_name, action)): Path<(String, String)>,
    Query(query): Query<OrionCompatQuery>,
    body: Option<Json<serde_json::Value>>,
) -> AppResult<impl axum::response::IntoResponse> {
    let user_id = guard::current_user_id(&headers, &state.config.jwt)?;
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
            let rows =
                compat_service::list_records(&state.db, OrionCompatModule::TerminalConnectLog)
                    .await?;
            Ok(orion_ok(filter_terminal_connect_sessions(rows, &payload)))
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
    let user_id = guard::current_user_id(&headers, &state.config.jwt)?;
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
            let mut rng = rand::rngs::OsRng;
            let private_key = RsaPrivateKey::new(&mut rng, 2048)
                .map_err(|e| AppError::Internal(format!("failed to generate rsa keypair: {e}")))?;
            let public_key = RsaPublicKey::from(&private_key);
            let public_key_pem = public_key
                .to_public_key_pem(LineEnding::LF)
                .map_err(|e| AppError::Internal(format!("failed to encode rsa public key: {e}")))?;
            let private_key_pem = private_key
                .to_pkcs8_pem(LineEnding::LF)
                .map_err(|e| AppError::Internal(format!("failed to encode rsa private key: {e}")))?
                .to_string();
            Ok(orion_ok(serde_json::json!({
                "publicKey": public_key_pem,
                "privateKey": private_key_pem
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
                payload.clone(),
                &format!("user-{user_id}"),
            )
            .await?;
            if module == OrionCompatModule::Tag {
                let _ = audit_service::log_operator_action(
                    &state.db,
                    &headers,
                    user_id,
                    "tag",
                    "create",
                    payload,
                    1,
                    None,
                )
                .await;
            }
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
                payload.clone(),
                &format!("user-{user_id}"),
            )
            .await?;
            if module == OrionCompatModule::Tag {
                let _ = audit_service::log_operator_action(
                    &state.db,
                    &headers,
                    user_id,
                    "tag",
                    "update",
                    payload,
                    1,
                    None,
                )
                .await;
            }
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
            if module == OrionCompatModule::Tag && affected > 0 {
                let _ = audit_service::log_operator_action(
                    &state.db,
                    &headers,
                    user_id,
                    "tag",
                    "delete",
                    serde_json::json!({ "id": id }),
                    1,
                    None,
                )
                .await;
            }
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
    let _user_id = guard::current_user_id(&headers, &state.config.jwt)?;
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
    r#type: Option<String>,
    risk_level: Option<String>,
    result: Option<i16>,
    start_time_range: Option<Vec<String>>,
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

fn parse_datetime_text(raw: &str) -> AppResult<DateTime<Utc>> {
    if let Ok(value) = DateTime::parse_from_rfc3339(raw) {
        return Ok(value.with_timezone(&Utc));
    }

    if let Ok(value) = NaiveDateTime::parse_from_str(raw, "%Y-%m-%d %H:%M:%S") {
        return Ok(DateTime::<Utc>::from_naive_utc_and_offset(value, Utc));
    }

    Err(AppError::BadRequest(format!(
        "invalid datetime value `{raw}`; expected RFC3339 or YYYY-MM-DD HH:MM:SS"
    )))
}

fn parse_time_range(
    values: Option<&Vec<String>>,
) -> AppResult<(Option<DateTime<Utc>>, Option<DateTime<Utc>>)> {
    let Some(values) = values else {
        return Ok((None, None));
    };

    if values.is_empty() {
        return Ok((None, None));
    }

    if values.len() != 2 {
        return Err(AppError::BadRequest(
            "startTimeRange must include exactly 2 values".to_string(),
        ));
    }

    let start = parse_datetime_text(values[0].trim())?;
    let end = parse_datetime_text(values[1].trim())?;
    if end < start {
        return Err(AppError::BadRequest(
            "startTimeRange end cannot be earlier than start".to_string(),
        ));
    }

    Ok((Some(start), Some(end)))
}

async fn require_any_permission(
    state: &AppState,
    headers: &HeaderMap,
    permission_codes: &[&str],
) -> AppResult<i64> {
    for code in permission_codes {
        match guard::require_permission(state, headers, code).await {
            Ok(user_id) => return Ok(user_id),
            Err(AppError::Auth(_)) => continue,
            Err(err) => return Err(err),
        }
    }

    Err(AppError::Auth("Permission denied".to_string()))
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
    let user_id = guard::current_user_id(headers, &state.config.jwt)?;
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
    let user_id = guard::current_user_id(&headers, &state.config.jwt)?;
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
    let user_id = guard::current_user_id(&headers, &state.config.jwt)?;
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
    let user_id = guard::current_user_id(&headers, &state.config.jwt)?;
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
    let user_id = guard::current_user_id(&headers, &state.config.jwt)?;
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
    let user_id = guard::current_user_id(&headers, &state.config.jwt)?;
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
    let user_id = guard::current_user_id(&headers, &state.config.jwt)?;
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
    let user_id = guard::current_user_id(&headers, &state.config.jwt)?;
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
    let user_id = guard::current_user_id(&headers, &state.config.jwt)?;
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
    let user_id = guard::current_user_id(&headers, &state.config.jwt)?;

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
            WHERE user_id = $1 AND deleted = 0
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
    let _user_id = guard::current_user_id(&headers, &state.config.jwt)?;

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
    let _user_id = guard::current_user_id(&headers, &state.config.jwt)?;

    let rows =
        compat_service::list_records(&state.db, OrionCompatModule::TerminalConnectLog).await?;
    let data = build_terminal_workplace_stats(rows, Utc::now());

    Ok(OrionResponse::ok(data))
}

fn build_terminal_workplace_stats(
    rows: Vec<serde_json::Value>,
    now: DateTime<Utc>,
) -> OrionTerminalWorkplaceStatisticsResponse {
    let today_start = DateTime::<Utc>::from_naive_utc_and_offset(
        now.date_naive().and_hms_opt(0, 0, 0).unwrap_or_default(),
        Utc,
    );
    let week_start = today_start - chrono::Duration::days(6);

    let mut chart_x = Vec::with_capacity(7);
    let mut chart_data = vec![0_i64; 7];
    for i in 0..7 {
        let day = week_start + chrono::Duration::days(i as i64);
        chart_x.push(day.format("%m-%d").to_string());
    }

    let mut history = rows
        .into_iter()
        .filter(|row| row.get("status").and_then(serde_json::Value::as_str) != Some("CONNECTING"))
        .collect::<Vec<_>>();

    for row in &history {
        let Some(start_ms) = row.get("startTime").and_then(serde_json::Value::as_i64) else {
            continue;
        };
        let Some(start_time) = DateTime::<Utc>::from_timestamp_millis(start_ms) else {
            continue;
        };

        if start_time < week_start || start_time >= today_start + chrono::Duration::days(1) {
            continue;
        }

        let idx = (start_time
            .date_naive()
            .signed_duration_since(week_start.date_naive())
            .num_days()) as usize;
        if idx < chart_data.len() {
            chart_data[idx] += 1;
        }
    }

    let today_terminal_connect_count = chart_data.last().copied().unwrap_or_default();
    let week_terminal_connect_count = chart_data.iter().sum::<i64>();

    history.sort_by(|a, b| {
        let a_time = a
            .get("startTime")
            .and_then(serde_json::Value::as_i64)
            .unwrap_or_default();
        let b_time = b
            .get("startTime")
            .and_then(serde_json::Value::as_i64)
            .unwrap_or_default();
        b_time.cmp(&a_time).then_with(|| {
            let a_id = a
                .get("id")
                .and_then(serde_json::Value::as_i64)
                .unwrap_or_default();
            let b_id = b
                .get("id")
                .and_then(serde_json::Value::as_i64)
                .unwrap_or_default();
            b_id.cmp(&a_id)
        })
    });

    OrionTerminalWorkplaceStatisticsResponse {
        today_terminal_connect_count,
        week_terminal_connect_count,
        terminal_connect_chart: OrionLineSingleChartData {
            x: chart_x,
            data: chart_data,
        },
        terminal_connect_list: history.into_iter().take(10).collect(),
    }
}

async fn orion_mine_login_history(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<OrionCountQuery>,
) -> AppResult<impl axum::response::IntoResponse> {
    let user_id = guard::current_user_id(&headers, &state.config.jwt)?;
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
    let user_id = guard::current_user_id(&headers, &state.config.jwt)?;
    let rows = sqlx::query_as::<_, (i64, String, Option<String>, i64, Option<String>, Option<String>, Option<String>)>(
        "SELECT id, session_id::text, NULLIF(revoked_at::text, ''), EXTRACT(EPOCH FROM created_at)::bigint * 1000, ip, location, user_agent
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
                "address": r.4.unwrap_or_default(),
                "location": r.5.unwrap_or_default(),
                "userAgent": r.6.unwrap_or_default(),
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
    let user_id = guard::current_user_id(&headers, &state.config.jwt)?;
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
    let operation = sanitize_search(payload.r#type.clone());
    let risk_level = sanitize_search(payload.risk_level.clone()).map(|v| v.to_ascii_uppercase());
    let (start_time, end_time) = parse_time_range(payload.start_time_range.as_ref())?;
    let (page, limit, offset) = normalize_pagination(payload.page, payload.limit);

    let rows = sqlx::query_as::<_, (i64, Option<i64>, Option<String>, Option<String>, Option<String>, Option<String>, Option<String>, Option<serde_json::Value>, Option<String>, Option<String>, Option<String>, Option<i16>, Option<String>, Option<i32>, i64)>(
        "SELECT id, user_id, username, module, operation, method, path, params, trace_id, ip, user_agent, result, error_message, duration, EXTRACT(EPOCH FROM create_time)::bigint * 1000
         FROM operator_log
         WHERE deleted = 0
           AND ($1::bigint IS NULL OR user_id = $1)
           AND ($2::bigint IS NULL OR user_id = $2)
           AND ($3::text IS NULL OR username ILIKE '%' || $3 || '%')
           AND ($4::text IS NULL OR module ILIKE '%' || $4 || '%')
           AND ($5::text IS NULL OR operation ILIKE '%' || $5 || '%')
           AND ($6::text IS NULL OR (CASE
                WHEN result = 0 THEN 'HIGH'
                WHEN result = 1 THEN 'LOW'
                ELSE 'MEDIUM'
              END) = $6)
           AND ($7::smallint IS NULL OR result = $7)
           AND ($8::timestamptz IS NULL OR create_time >= $8)
           AND ($9::timestamptz IS NULL OR create_time <= $9)
         ORDER BY create_time DESC, id DESC
         LIMIT $10 OFFSET $11",
    )
    .bind(scope_user_id)
    .bind(payload.user_id)
    .bind(username)
    .bind(module)
    .bind(operation)
    .bind(risk_level)
    .bind(payload.result)
    .bind(start_time)
    .bind(end_time)
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.db)
    .await?;

    let total = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(1)
         FROM operator_log
         WHERE deleted = 0
           AND ($1::bigint IS NULL OR user_id = $1)
           AND ($2::bigint IS NULL OR user_id = $2)
           AND ($3::text IS NULL OR username ILIKE '%' || $3 || '%')
           AND ($4::text IS NULL OR module ILIKE '%' || $4 || '%')
           AND ($5::text IS NULL OR operation ILIKE '%' || $5 || '%')
           AND ($6::text IS NULL OR (CASE
                WHEN result = 0 THEN 'HIGH'
                WHEN result = 1 THEN 'LOW'
                ELSE 'MEDIUM'
              END) = $6)
           AND ($7::smallint IS NULL OR result = $7)
           AND ($8::timestamptz IS NULL OR create_time >= $8)
           AND ($9::timestamptz IS NULL OR create_time <= $9)",
    )
    .bind(scope_user_id)
    .bind(payload.user_id)
    .bind(sanitize_search(payload.username.clone()))
    .bind(sanitize_search(payload.module.clone()))
    .bind(sanitize_search(payload.r#type.clone()))
    .bind(sanitize_search(payload.risk_level.clone()).map(|v| v.to_ascii_uppercase()))
    .bind(payload.result)
    .bind(start_time)
    .bind(end_time)
    .fetch_one(&state.db)
    .await?;

    let items = rows
        .into_iter()
        .map(|r| {
            let operation = r.4.clone().unwrap_or_default();
            let log_info = {
                let text = format!(
                    "{} {}",
                    r.5.as_deref().unwrap_or(""),
                    r.6.as_deref().unwrap_or("")
                )
                .trim()
                .to_string();
                if text.is_empty() {
                    operation.clone()
                } else {
                    text
                }
            };
            OrionOperatorLogItem {
                id: r.0,
                user_id: r.1.unwrap_or_default(),
                username: r.2.unwrap_or_default(),
                trace_id: r.8.unwrap_or_default(),
                address: r.9.unwrap_or_default(),
                location: String::new(),
                user_agent: r.10.unwrap_or_default(),
                risk_level: if r.11.unwrap_or_default() == 0 {
                    "HIGH".to_string()
                } else {
                    "LOW".to_string()
                },
                module: r.3.unwrap_or_default(),
                r#type: operation.clone(),
                log_info,
                origin_log_info: r.6.clone().unwrap_or(operation),
                extra: r
                    .7
                    .and_then(|v| serde_json::to_string(&v).ok())
                    .unwrap_or_else(|| "{}".to_string()),
                result: r.11.unwrap_or_default(),
                error_message: r.12.unwrap_or_default(),
                return_value: String::new(),
                duration: r.13.unwrap_or_default(),
                start_time: r.14,
                end_time: r.14,
                create_time: r.14,
            }
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
    let user_id = guard::current_user_id(&headers, &state.config.jwt)?;
    Ok(OrionResponse::ok(
        query_operator_logs(&state, Some(user_id), &payload).await?,
    ))
}

async fn orion_mine_update_password(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<OrionMineUpdatePasswordRequest>,
) -> AppResult<impl axum::response::IntoResponse> {
    let user_id = guard::current_user_id(&headers, &state.config.jwt)?;
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

    audit_service::log_operator_action(
        &state.db,
        &headers,
        user_id,
        "iam",
        "update_password",
        serde_json::json!({}),
        1,
        None,
    )
    .await?;

    Ok(OrionResponse::ok(true))
}

async fn orion_operator_log_query(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<OrionOperatorLogQueryRequest>,
) -> AppResult<impl axum::response::IntoResponse> {
    require_any_permission(
        &state,
        &headers,
        &["infra:operator-log:query", "iam.user-permission.view"],
    )
    .await?;
    Ok(OrionResponse::ok(
        query_operator_logs(&state, None, &payload).await?,
    ))
}

async fn orion_operator_log_count(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<OrionOperatorLogQueryRequest>,
) -> AppResult<impl axum::response::IntoResponse> {
    require_any_permission(
        &state,
        &headers,
        &["infra:operator-log:query", "iam.user-permission.view"],
    )
    .await?;
    let grid = query_operator_logs(&state, None, &payload).await?;
    Ok(OrionResponse::ok(grid.total))
}

async fn orion_operator_log_delete(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<OrionDeleteIdsQuery>,
) -> AppResult<impl axum::response::IntoResponse> {
    require_any_permission(
        &state,
        &headers,
        &["infra:operator-log:delete", "iam.user-permission.view"],
    )
    .await?;
    let ids = parse_csv_ids(query.id_list);
    let mut affected = 0_i64;
    if !ids.is_empty() {
        let result = sqlx::query(
            "UPDATE operator_log
             SET deleted = 1
             WHERE deleted = 0
               AND id = ANY($1::bigint[])",
        )
        .bind(ids)
        .execute(&state.db)
        .await?;
        affected = result.rows_affected() as i64;
    }
    Ok(OrionResponse::ok(affected))
}

async fn orion_operator_log_clear(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<OrionOperatorLogQueryRequest>,
) -> AppResult<impl axum::response::IntoResponse> {
    require_any_permission(
        &state,
        &headers,
        &[
            "infra:operator-log:management:clear",
            "iam.user-permission.view",
        ],
    )
    .await?;

    let username = sanitize_search(payload.username.clone());
    let module = sanitize_search(payload.module.clone());
    let operation = sanitize_search(payload.r#type.clone());
    let risk_level = sanitize_search(payload.risk_level.clone()).map(|v| v.to_ascii_uppercase());
    let (start_time, end_time) = parse_time_range(payload.start_time_range.as_ref())?;
    let limit = payload.limit.unwrap_or(2000).clamp(1, 20_000);

    let result = sqlx::query(
        "WITH target AS (
             SELECT id, create_time
             FROM operator_log
             WHERE deleted = 0
               AND ($1::bigint IS NULL OR user_id = $1)
               AND ($2::text IS NULL OR username ILIKE '%' || $2 || '%')
               AND ($3::text IS NULL OR module ILIKE '%' || $3 || '%')
               AND ($4::text IS NULL OR operation ILIKE '%' || $4 || '%')
               AND ($5::text IS NULL OR (CASE
                    WHEN result = 0 THEN 'HIGH'
                    WHEN result = 1 THEN 'LOW'
                    ELSE 'MEDIUM'
                  END) = $5)
               AND ($6::smallint IS NULL OR result = $6)
               AND ($7::timestamptz IS NULL OR create_time >= $7)
               AND ($8::timestamptz IS NULL OR create_time <= $8)
             ORDER BY create_time DESC, id DESC
             LIMIT $9
         )
         UPDATE operator_log src
         SET deleted = 1
         FROM target
         WHERE src.id = target.id
           AND src.create_time = target.create_time
           AND src.deleted = 0",
    )
    .bind(payload.user_id)
    .bind(username)
    .bind(module)
    .bind(operation)
    .bind(risk_level)
    .bind(payload.result)
    .bind(start_time)
    .bind(end_time)
    .bind(limit)
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
    let actor_user_id = guard::current_user_id(&headers, &state.config.jwt)?;
    let username = sanitize_search(payload.username.clone())
        .ok_or_else(|| AppError::BadRequest("username is required".to_string()))?;
    let password = payload
        .password
        .clone()
        .ok_or_else(|| AppError::BadRequest("password is required".to_string()))?;
    let pass_hash = bcrypt::hash(password, bcrypt::DEFAULT_COST)
        .map_err(|e| AppError::Internal(e.to_string()))?;
    let id = sqlx::query_scalar::<_, i64>(
        "INSERT INTO sys_user (username, password, nickname, avatar, phone, email, status, create_time, update_time, deleted)
         VALUES ($1, $2, $3, $4, $5, $6, 1, NOW(), NOW(), 0)
         RETURNING id",
    )
    .bind(&username)
    .bind(pass_hash)
    .bind(&payload.nickname)
    .bind(&payload.avatar)
    .bind(&payload.mobile)
    .bind(&payload.email)
    .fetch_one(&state.db)
    .await?;

    audit_service::log_operator_action(
        &state.db,
        &headers,
        actor_user_id,
        "iam",
        "create_user",
        serde_json::json!({
            "id": id,
            "username": username,
        }),
        1,
        None,
    )
    .await?;

    Ok(OrionResponse::ok(id))
}

async fn orion_system_user_update(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<OrionSystemUserUpsertRequest>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "iam.user-permission.view").await?;
    let actor_user_id = guard::current_user_id(&headers, &state.config.jwt)?;
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
    .bind(payload.username.as_ref().map(|v| v.trim().to_string()))
    .bind(&payload.nickname)
    .bind(&payload.avatar)
    .bind(&payload.mobile)
    .bind(&payload.email)
    .bind(id)
    .execute(&state.db)
    .await?
    .rows_affected();
    if rows == 0 {
        return Err(AppError::NotFound("User not found".to_string()));
    }

    audit_service::log_operator_action(
        &state.db,
        &headers,
        actor_user_id,
        "iam",
        "update_user",
        serde_json::json!({
            "id": id,
            "username": payload.username,
        }),
        1,
        None,
    )
    .await?;

    Ok(OrionResponse::ok(true))
}

async fn orion_system_user_update_status(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<OrionSystemUserUpsertRequest>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "iam.user-permission.view").await?;
    let actor_user_id = guard::current_user_id(&headers, &state.config.jwt)?;
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

    audit_service::log_operator_action(
        &state.db,
        &headers,
        actor_user_id,
        "iam",
        "update_user_status",
        serde_json::json!({
            "id": id,
            "status": status,
        }),
        1,
        None,
    )
    .await?;

    Ok(OrionResponse::ok(true))
}

async fn orion_system_user_grant_role(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<OrionSystemUserUpsertRequest>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "iam.user-role.assign").await?;
    let actor_user_id = guard::current_user_id(&headers, &state.config.jwt)?;
    let user_id = parse_required_id(payload.id, "id")?;
    let role_ids = payload.role_id_list.clone().unwrap_or_default();
    let mut tx = state.db.begin().await?;
    sqlx::query("DELETE FROM sys_user_role WHERE user_id = $1")
        .bind(user_id)
        .execute(&mut *tx)
        .await?;
    for role_id in &role_ids {
        if *role_id > 0 {
            sqlx::query("INSERT INTO sys_user_role (user_id, role_id, create_time) VALUES ($1, $2, NOW())")
                .bind(user_id)
                .bind(role_id)
                .execute(&mut *tx)
                .await?;
        }
    }
    tx.commit().await?;

    audit_service::log_operator_action(
        &state.db,
        &headers,
        actor_user_id,
        "iam",
        "grant_user_role",
        serde_json::json!({
            "id": user_id,
            "role_ids": role_ids,
        }),
        1,
        None,
    )
    .await?;

    Ok(OrionResponse::ok(true))
}
async fn orion_system_user_reset_password(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<OrionSystemUserUpsertRequest>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "iam.user-permission.view").await?;
    let actor_user_id = guard::current_user_id(&headers, &state.config.jwt)?;
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

    audit_service::log_operator_action(
        &state.db,
        &headers,
        actor_user_id,
        "iam",
        "reset_user_password",
        serde_json::json!({ "id": id }),
        1,
        None,
    )
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
    let actor_user_id = guard::current_user_id(&headers, &state.config.jwt)?;
    let id = parse_required_id(query.id, "id")?;
    sqlx::query(
        "UPDATE sys_user SET deleted = 1, update_time = NOW() WHERE id = $1 AND deleted = 0",
    )
    .bind(id)
    .execute(&state.db)
    .await?;

    audit_service::log_operator_action(
        &state.db,
        &headers,
        actor_user_id,
        "iam",
        "delete_user",
        serde_json::json!({ "id": id }),
        1,
        None,
    )
    .await?;

    Ok(OrionResponse::ok(true))
}

async fn orion_system_user_batch_delete(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<OrionDeleteIdsQuery>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "iam.user-permission.view").await?;
    let actor_user_id = guard::current_user_id(&headers, &state.config.jwt)?;
    let ids = parse_csv_ids(query.id_list);
    if !ids.is_empty() {
        sqlx::query("UPDATE sys_user SET deleted = 1, update_time = NOW() WHERE id = ANY($1::bigint[]) AND deleted = 0")
            .bind(&ids)
            .execute(&state.db)
            .await?;

        audit_service::log_operator_action(
            &state.db,
            &headers,
            actor_user_id,
            "iam",
            "batch_delete_user",
            serde_json::json!({ "ids": ids }),
            1,
            None,
        )
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
    let rows = sqlx::query_as::<_, (i64, i64, i64, Option<String>, Option<String>, Option<String>, String)>(
        "SELECT r.id, r.user_id, EXTRACT(EPOCH FROM r.created_at)::bigint * 1000, r.ip, r.location, r.user_agent, u.username
         FROM auth_refresh_token r
         JOIN sys_user u ON u.id = r.user_id
         WHERE r.revoked_at IS NULL
         ORDER BY r.created_at DESC LIMIT 200",
    )
    .fetch_all(&state.db)
    .await?;
    let data = rows
        .into_iter()
        .map(|r| {
            serde_json::json!({
                "id": r.0,
                "username": r.6,
                "visible": true,
                "current": false,
                "address": r.3.unwrap_or_default(),
                "location": r.4.unwrap_or_default(),
                "userAgent": r.5.unwrap_or_default(),
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
    let rows = sqlx::query_as::<_, (i64, i64, Option<String>, Option<String>, Option<String>, String)>(
        "SELECT r.id,
                EXTRACT(EPOCH FROM r.created_at)::bigint * 1000,
                r.ip,
                r.location,
                r.user_agent,
                u.username
         FROM auth_refresh_token r
         JOIN sys_user u ON u.id = r.user_id
         WHERE r.user_id = $1
         ORDER BY r.created_at DESC
         LIMIT 200",
    )
    .bind(user_id)
    .fetch_all(&state.db)
    .await?;
    let data = rows
        .into_iter()
        .map(|r| {
            serde_json::json!({
                "id": r.0,
                "username": r.5,
                "visible": true,
                "current": false,
                "address": r.2.unwrap_or_default(),
                "location": r.3.unwrap_or_default(),
                "userAgent": r.4.unwrap_or_default(),
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
    let actor_user_id = guard::current_user_id(&headers, &state.config.jwt)?;
    let name = sanitize_search(payload.name)
        .ok_or_else(|| AppError::BadRequest("name is required".to_string()))?;
    let code = sanitize_search(payload.code)
        .ok_or_else(|| AppError::BadRequest("code is required".to_string()))?;
    let id = sqlx::query_scalar::<_, i64>(
        "INSERT INTO sys_role (name, code, description, status, create_time, deleted)
         VALUES ($1, $2, $3, COALESCE($4, 1), NOW(), 0)
         RETURNING id",
    )
    .bind(&name)
    .bind(&code)
    .bind(payload.description)
    .bind(payload.status)
    .fetch_one(&state.db)
    .await?;

    audit_service::log_operator_action(
        &state.db,
        &headers,
        actor_user_id,
        "iam",
        "create_role",
        serde_json::json!({
            "id": id,
            "name": name,
            "code": code,
        }),
        1,
        None,
    )
    .await?;

    Ok(OrionResponse::ok(id))
}

async fn orion_system_role_update(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<OrionSystemRoleRequest>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "iam.user-role.assign").await?;
    let actor_user_id = guard::current_user_id(&headers, &state.config.jwt)?;
    let id = parse_required_id(payload.id, "id")?;
    let rows = sqlx::query(
        "UPDATE sys_role SET name = COALESCE(NULLIF($1, ''), name), code = COALESCE(NULLIF($2, ''), code), description = COALESCE($3, description), status = COALESCE($4, status) WHERE id = $5 AND deleted = 0",
    )
    .bind(payload.name.as_ref().map(|v| v.trim().to_string()))
    .bind(payload.code.as_ref().map(|v| v.trim().to_string()))
    .bind(payload.description)
    .bind(payload.status)
    .bind(id)
    .execute(&state.db)
    .await?
    .rows_affected();
    if rows == 0 {
        return Err(AppError::NotFound("Role not found".to_string()));
    }

    audit_service::log_operator_action(
        &state.db,
        &headers,
        actor_user_id,
        "iam",
        "update_role",
        serde_json::json!({
            "id": id,
            "name": payload.name,
        }),
        1,
        None,
    )
    .await?;

    Ok(OrionResponse::ok(true))
}

async fn orion_system_role_update_status(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<OrionSystemRoleRequest>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "iam.user-role.assign").await?;
    let actor_user_id = guard::current_user_id(&headers, &state.config.jwt)?;
    let id = parse_required_id(payload.id, "id")?;
    let status = payload
        .status
        .ok_or_else(|| AppError::BadRequest("status is required".to_string()))?;
    sqlx::query("UPDATE sys_role SET status = $1 WHERE id = $2 AND deleted = 0")
        .bind(status)
        .bind(id)
        .execute(&state.db)
        .await?;

    audit_service::log_operator_action(
        &state.db,
        &headers,
        actor_user_id,
        "iam",
        "update_role_status",
        serde_json::json!({
            "id": id,
            "status": status,
        }),
        1,
        None,
    )
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

async fn orion_terminal_themes(
    State(state): State<AppState>,
) -> AppResult<impl axum::response::IntoResponse> {
    // Query terminal themes from sys_dict_value table
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
    .fetch_all(&state.db)
    .await?;

    let mut themes = Vec::new();
    for (name, schema_json, extra_json) in rows {
        let schema: serde_json::Value =
            serde_json::from_str(&schema_json).unwrap_or_else(|_| serde_json::json!({}));

        let mut dark = false;
        if let Some(extra_str) = &extra_json {
            if let Ok(extra) = serde_json::from_str::<serde_json::Value>(extra_str) {
                dark = extra.get("dark").and_then(|v| v.as_bool()).unwrap_or(false);
            }
        }

        themes.push(serde_json::json!({
            "name": name,
            "dark": dark,
            "schema": schema
        }));
    }

    Ok(OrionResponse::ok(themes))
}

async fn orion_terminal_access(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<OrionTerminalAccessRequest>,
) -> AppResult<impl axum::response::IntoResponse> {
    let user_id = guard::current_user_id(&headers, &state.config.jwt)?;
    let host_id = parse_required_id(payload.host_id, "hostId")?;
    guard::require_host_permission(&state, user_id, host_id).await?;
    let issued_at_ms = now_ms();
    let payload = format!(
        "term:{}:{}:{}:{}",
        user_id,
        host_id,
        payload.connect_type.unwrap_or_else(|| "ssh".to_string()),
        issued_at_ms
    );
    let signature = terminal_token_signature(&payload, &state.config.secrets.data_encryption_key);
    let token = format!("{payload}:{signature}");
    Ok(OrionResponse::ok(token))
}

async fn orion_terminal_transfer(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<impl axum::response::IntoResponse> {
    let user_id = guard::current_user_id(&headers, &state.config.jwt)?;
    let payload = format!("transfer:{}:{}", user_id, now_ms());
    let signature = terminal_token_signature(&payload, &state.config.secrets.data_encryption_key);
    Ok(OrionResponse::ok(format!("{payload}:{signature}")))
}

fn terminal_token_signature(payload: &str, signing_key: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(signing_key.as_bytes());
    hasher.update(b":");
    hasher.update(payload.as_bytes());
    URL_SAFE_NO_PAD.encode(hasher.finalize())
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
}

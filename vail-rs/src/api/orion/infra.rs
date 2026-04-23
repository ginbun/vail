use super::*;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct OrionMineUpdateUserRequest {
    nickname: Option<String>,
    avatar: Option<String>,
    mobile: Option<String>,
    email: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct OrionMineUpdatePasswordRequest {
    before_password: Option<String>,
    password: Option<String>,
    check_password: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct OrionCountQuery {
    count: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct OrionSessionOfflineRequest {
    user_id: Option<i64>,
    timestamp: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct OrionOperatorLogQueryRequest {
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
pub(super) struct OrionDataGrid<T> {
    pub(super) page: i64,
    pub(super) limit: i64,
    pub(super) total: i64,
    pub(super) rows: Vec<T>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct OrionOperatorLogItem {
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
pub(super) struct OrionDeleteIdsQuery {
    pub(super) id_list: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct OrionSystemUserUpsertRequest {
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
pub(super) struct OrionSystemUserQueryRequest {
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
pub(super) struct OrionSystemUserIdQuery {
    id: Option<i64>,
    user_id: Option<i64>,
    username: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct OrionRoleQueryResponse {
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
pub(super) struct OrionSystemUserQueryResponse {
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
pub(super) struct OrionSystemRoleRequest {
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
pub(super) struct OrionSystemRoleIdQuery {
    id: Option<i64>,
    role_id: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct OrionSystemMenuRequest {
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
pub(super) struct OrionTerminalAccessRequest {
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

pub(super) fn parse_datetime_text(raw: &str) -> AppResult<DateTime<Utc>> {
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

type UtcTimeRange = (Option<DateTime<Utc>>, Option<DateTime<Utc>>);

fn parse_time_range(values: Option<&Vec<String>>) -> AppResult<UtcTimeRange> {
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
    guard::require_any_permission(state, headers, permission_codes).await
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
) -> AppResult<(i64, String, String, String, String, String)> {
    let user_id = guard::current_user_id(headers, &state.config.jwt)?;
    let user = iam_service::get_current_user_profile_by_id(&state.db, user_id)
        .await?
        .ok_or_else(|| AppError::Auth("User not found".to_string()))?;

    Ok((
        user.id,
        user.username,
        user.nickname,
        user.avatar,
        user.mobile,
        user.email,
    ))
}

pub(super) async fn orion_mine_get_user(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<impl axum::response::IntoResponse> {
    let (user_id, username, nickname, avatar, mobile, email) =
        current_user_tuple(&state, &headers).await?;

    let roles = iam_service::list_user_roles_by_user_id(&state.db, user_id)
        .await?
        .into_iter()
        .map(|r| OrionRoleQueryResponse {
            id: r.id,
            name: r.name,
            code: r.code,
            status: r.status,
            description: r.description,
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
        mobile,
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

pub(super) async fn orion_mine_update_user(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<OrionMineUpdateUserRequest>,
) -> AppResult<impl axum::response::IntoResponse> {
    let user_id = guard::current_user_id(&headers, &state.config.jwt)?;
    iam_service::update_current_user_profile(
        &state.db,
        iam_service::OrionCurrentUserUpdateInput {
            user_id,
            nickname: payload.nickname,
            avatar: payload.avatar,
            mobile: payload.mobile,
            email: payload.email,
        },
    )
    .await?;
    Ok(OrionResponse::ok(true))
}

pub(super) async fn orion_dict_key_create(
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
    let (_, username, _, _, _, _) = current_user_tuple(&state, &headers).await?;

    let id = dict_key_service::create_dict_key(
        &state.db,
        dict_key_service::OrionDictKeyCreateInput {
            key_name,
            value_type,
            extra_schema,
            description,
            username,
        },
    )
    .await?;

    Ok(OrionResponse::ok(id))
}

pub(super) async fn orion_dict_key_update(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<OrionDictKeyUpsertRequest>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "infra:dict-key:update").await?;
    let id = parse_required_id(payload.id, "id")?;
    let (_, username, _, _, _, _) = current_user_tuple(&state, &headers).await?;
    let rows = dict_key_service::update_dict_key(
        &state.db,
        dict_key_service::OrionDictKeyUpdateInput {
            id,
            key_name: payload.key_name.map(|v| v.trim().to_string()),
            value_type: payload.value_type.map(|v| v.trim().to_string()),
            extra_schema: payload.extra_schema,
            description: payload.description,
            username,
        },
    )
    .await?;

    if rows == 0 {
        return Err(AppError::NotFound("Dict key not found".to_string()));
    }

    Ok(OrionResponse::ok(true))
}

pub(super) async fn orion_dict_key_list(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "infra:dict-key:query").await?;
    let rows = dict_key_service::list_dict_keys(&state.db).await?;
    let data = rows.into_iter().map(map_dict_key_row).collect::<Vec<_>>();

    Ok(OrionResponse::ok(data))
}

pub(super) async fn orion_dict_key_query(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<OrionDictKeyQueryRequest>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "infra:dict-key:query").await?;
    let (page, limit, offset) = normalize_pagination(payload.page, payload.limit);
    let search_value = sanitize_search(payload.search_value);
    let key_name = sanitize_search(payload.key_name);
    let description = sanitize_search(payload.description);

    let filters = OrionDictKeyQueryFilters {
        id: payload.id,
        key_name,
        description,
        search_value,
        limit,
        offset,
    };

    let rows = dict_key_service::query_dict_keys(&state.db, filters.clone()).await?;
    let total = dict_key_service::count_dict_keys(&state.db, filters).await?;
    let items = rows.into_iter().map(map_dict_key_row).collect::<Vec<_>>();

    Ok(OrionResponse::ok(OrionDataGrid {
        page,
        limit,
        total,
        rows: items,
    }))
}

pub(super) async fn orion_dict_key_refresh_cache(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "infra:dict-key:management:refresh-cache").await?;
    Ok(OrionResponse::ok(true))
}

pub(super) async fn orion_dict_key_delete(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<OrionIdQuery>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "infra:dict-key:delete").await?;
    let id = parse_required_id(query.id, "id")?;
    dict_key_service::delete_dict_key(&state.db, id).await?;
    Ok(OrionResponse::ok(true))
}

pub(super) async fn orion_dict_key_batch_delete(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<OrionDeleteIdsQuery>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "infra:dict-key:delete").await?;
    let ids = parse_csv_ids(query.id_list);
    if !ids.is_empty() {
        dict_key_service::batch_delete_dict_keys(&state.db, ids).await?;
    }
    Ok(OrionResponse::ok(true))
}

pub(super) async fn orion_dict_value_create(
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
    let (_, username, _, _, _, _) = current_user_tuple(&state, &headers).await?;

    let id = dict_value_service::create_dict_value(
        &state.db,
        dict_value_service::OrionDictValueCreateInput {
            key_id,
            name,
            value,
            label,
            extra,
            sort,
            username,
        },
    )
    .await?;

    Ok(OrionResponse::ok(id))
}

pub(super) async fn orion_dict_value_update(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<OrionDictValueUpsertRequest>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "infra:dict-value:update").await?;
    let id = parse_required_id(payload.id, "id")?;
    let (_, username, _, _, _, _) = current_user_tuple(&state, &headers).await?;

    let outcome = dict_value_service::update_dict_value(
        &state.db,
        dict_value_service::OrionDictValueUpdateInput {
            id,
            key_id: payload.key_id,
            name: payload.name,
            value: payload.value,
            label: payload.label,
            extra: payload.extra,
            sort: payload.sort,
            username,
        },
    )
    .await?;

    if outcome == dict_value_service::OrionDictValueUpdateOutcome::NotFound {
        return Err(AppError::NotFound("Dict value not found".to_string()));
    }

    Ok(OrionResponse::ok(true))
}

pub(super) async fn orion_dict_value_rollback(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<OrionDictValueRollbackRequest>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "infra:dict-value:update").await?;
    let id = parse_required_id(payload.id, "id")?;
    let history_id = parse_required_id(payload.value_id, "valueId")?;

    let outcome = dict_value_service::rollback_dict_value(
        &state.db,
        dict_value_service::OrionDictValueRollbackInput { id, history_id },
    )
    .await?;

    match outcome {
        dict_value_service::OrionDictValueRollbackOutcome::RolledBack => {}
        dict_value_service::OrionDictValueRollbackOutcome::HistoryNotFound => {
            return Err(AppError::NotFound("History value not found".to_string()));
        }
        dict_value_service::OrionDictValueRollbackOutcome::DictValueNotFound => {
            return Err(AppError::NotFound("Dict value not found".to_string()));
        }
    }

    Ok(OrionResponse::ok(true))
}

pub(super) async fn orion_dict_value_list(
    State(state): State<AppState>,
    Query(query): Query<OrionDictValueListQuery>,
) -> AppResult<impl axum::response::IntoResponse> {
    let keys = parse_csv_values(query.keys);
    if keys.is_empty() {
        return Ok(OrionResponse::ok(
            HashMap::<String, Vec<OrionDictOption>>::new(),
        ));
    }

    let rows = dict_value_service::list_dict_values_by_keys(&state.db, &keys)
        .await
        .unwrap_or_default();

    let mut data = HashMap::<String, Vec<OrionDictOption>>::new();
    for key in &keys {
        data.insert(key.clone(), Vec::new());
    }

    for row in rows {
        let mut extra = parse_extra_fields(row.extra.as_deref().unwrap_or(""));
        if !extra.contains_key("color") && row.key_name.ends_with("Status") {
            if row.value == "1" {
                extra.insert("color".to_string(), serde_json::json!("green"));
            } else if row.value == "0" {
                extra.insert("color".to_string(), serde_json::json!("orangered"));
            }
        }
        let option = OrionDictOption {
            label: row.label,
            value: parse_dict_option_value(&row.value_type, &row.value),
            extra,
        };
        data.entry(row.key_name).or_default().push(option);
    }

    for key in &keys {
        if data.get(key).is_none_or(|v| v.is_empty()) {
            data.insert(key.clone(), builtin_dict_options(key));
        }
    }

    Ok(OrionResponse::ok(data))
}

pub(super) async fn orion_dict_value_query(
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

    let filters = OrionDictValueQueryFilters {
        key_id: payload.key_id,
        key_name,
        value,
        label,
        extra,
        limit,
        offset,
    };

    let rows = dict_value_service::query_dict_values(&state.db, filters.clone()).await?;
    let total = dict_value_service::count_dict_values(&state.db, filters).await?;

    let items = rows.into_iter().map(map_dict_value_row).collect::<Vec<_>>();

    Ok(OrionResponse::ok(OrionDataGrid {
        page,
        limit,
        total,
        rows: items,
    }))
}

pub(super) async fn orion_dict_value_delete(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<OrionIdQuery>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "infra:dict-value:delete").await?;
    let id = parse_required_id(query.id, "id")?;
    dict_value_service::soft_delete_dict_value(&state.db, id).await?;
    Ok(OrionResponse::ok(true))
}

pub(super) async fn orion_dict_value_batch_delete(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<OrionDeleteIdsQuery>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "infra:dict-value:delete").await?;
    let ids = parse_csv_ids(query.id_list);
    dict_value_service::soft_delete_dict_values(&state.db, ids).await?;
    Ok(OrionResponse::ok(true))
}

pub(super) async fn orion_system_message_list(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<OrionSystemMessageQueryRequest>,
) -> AppResult<impl axum::response::IntoResponse> {
    let user_id = guard::current_user_id(&headers, &state.config.jwt)?;
    let (_page, limit, offset) = normalize_pagination(payload.page, payload.limit);
    let effective_offset = system_message_service::effective_offset(offset, payload.max_id);
    let classify = sanitize_search(payload.classify);
    let query_unread = payload.query_unread.unwrap_or(false);

    let rows = system_message_service::list_system_messages(
        &state.db,
        OrionSystemMessageListFilters {
            user_id,
            classify,
            query_unread,
            max_id: payload.max_id,
            limit,
            offset: effective_offset,
        },
    )
    .await?;

    let data = rows
        .into_iter()
        .map(map_system_message_row)
        .collect::<Vec<_>>();

    Ok(OrionResponse::ok(data))
}

pub(super) async fn orion_system_message_count(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<OrionSystemMessageCountQuery>,
) -> AppResult<impl axum::response::IntoResponse> {
    let user_id = guard::current_user_id(&headers, &state.config.jwt)?;
    let query_unread = query.query_unread.unwrap_or(false);

    let data = system_message_service::count_system_messages_by_classify(
        &state.db,
        OrionSystemMessageCountFilters {
            user_id,
            query_unread,
        },
    )
    .await?;
    Ok(OrionResponse::ok(data))
}

pub(super) async fn orion_system_message_has_unread(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<impl axum::response::IntoResponse> {
    let user_id = guard::current_user_id(&headers, &state.config.jwt)?;
    let has_unread = system_message_service::has_unread_system_messages(&state.db, user_id).await?;
    Ok(OrionResponse::ok(has_unread))
}

pub(super) async fn orion_system_message_read(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<OrionIdQuery>,
) -> AppResult<impl axum::response::IntoResponse> {
    let user_id = guard::current_user_id(&headers, &state.config.jwt)?;
    let id = parse_required_id(query.id, "id")?;
    system_message_service::mark_system_message_read(&state.db, id, user_id).await?;
    Ok(OrionResponse::ok(true))
}

pub(super) async fn orion_system_message_read_all(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<OrionSystemMessageReadAllQuery>,
) -> AppResult<impl axum::response::IntoResponse> {
    let user_id = guard::current_user_id(&headers, &state.config.jwt)?;
    let classify = sanitize_search(query.classify);
    system_message_service::mark_system_messages_read_all(&state.db, user_id, classify).await?;
    Ok(OrionResponse::ok(true))
}

pub(super) async fn orion_system_message_delete(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<OrionIdQuery>,
) -> AppResult<impl axum::response::IntoResponse> {
    let user_id = guard::current_user_id(&headers, &state.config.jwt)?;
    let id = parse_required_id(query.id, "id")?;
    system_message_service::delete_system_message(&state.db, id, user_id).await?;
    Ok(OrionResponse::ok(true))
}

pub(super) async fn orion_system_message_clear(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<OrionSystemMessageReadAllQuery>,
) -> AppResult<impl axum::response::IntoResponse> {
    let user_id = guard::current_user_id(&headers, &state.config.jwt)?;
    let classify = sanitize_search(query.classify);
    system_message_service::clear_system_messages(&state.db, user_id, classify).await?;
    Ok(OrionResponse::ok(true))
}

pub(super) async fn orion_infra_statistics_get_workplace(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<impl axum::response::IntoResponse> {
    let user_id = guard::current_user_id(&headers, &state.config.jwt)?;

    let unread_message_count =
        system_message_service::count_unread_system_messages(&state.db, user_id)
            .await
            .unwrap_or(0);
    let data = infra_statistics_service::get_infra_workplace_statistics(
        &state.db,
        user_id,
        unread_message_count,
    )
    .await?
    .map(map_infra_workplace_stats)
    .ok_or_else(|| AppError::Auth("User not found".to_string()))?;

    Ok(OrionResponse::ok(data))
}

pub(super) async fn orion_exec_statistics_get_workplace(
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

pub(super) async fn orion_terminal_statistics_get_workplace(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> AppResult<impl axum::response::IntoResponse> {
    let _user_id = guard::current_user_id(&headers, &state.config.jwt)?;

    let rows =
        compat_service::list_records(&state.db, OrionCompatModule::TerminalConnectLog).await?;
    let data = build_terminal_workplace_stats(rows, Utc::now());

    Ok(OrionResponse::ok(data))
}

pub(super) fn build_terminal_workplace_stats(
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

pub(super) async fn orion_mine_login_history(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<OrionCountQuery>,
) -> AppResult<impl axum::response::IntoResponse> {
    let user_id = guard::current_user_id(&headers, &state.config.jwt)?;
    let limit = query.count.unwrap_or(10).clamp(1, 100);
    let rows =
        system_user_service::list_login_history_by_user_id(&state.db, user_id, limit).await?;
    let data = rows
        .into_iter()
        .map(|r| {
            serde_json::json!({
                "id": r.id,
                "address": r.address,
                "location": r.location,
                "userAgent": r.user_agent,
                "result": r.result,
                "errorMessage": r.error_message,
                "createTime": r.create_time
            })
        })
        .collect::<Vec<_>>();
    Ok(OrionResponse::ok(data))
}

pub(super) async fn orion_mine_user_session(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<impl axum::response::IntoResponse> {
    let user_id = guard::current_user_id(&headers, &state.config.jwt)?;
    let rows = system_user_service::list_mine_sessions(&state.db, user_id, 100).await?;
    let data = rows
        .into_iter()
        .map(|r| {
            serde_json::json!({
                "id": r.id,
                "username": r.username,
                "visible": r.visible,
                "current": r.current,
                "address": r.address,
                "location": r.location,
                "userAgent": r.user_agent,
                "loginTime": r.login_time,
                "offline": r.offline
            })
        })
        .collect::<Vec<_>>();
    Ok(OrionResponse::ok(data))
}

pub(super) async fn orion_mine_offline_session(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<OrionSessionOfflineRequest>,
) -> AppResult<impl axum::response::IntoResponse> {
    let user_id = guard::current_user_id(&headers, &state.config.jwt)?;
    let _ = payload.user_id;
    system_user_service::revoke_active_refresh_tokens_by_user_and_timestamp(
        &state.db,
        user_id,
        payload.timestamp,
    )
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

    let query = operator_log_service::OrionOperatorLogQueryInput {
        scope_user_id,
        user_id: payload.user_id,
        username,
        module,
        operation,
        risk_level,
        result: payload.result,
        start_time,
        end_time,
        limit,
        offset,
    };

    let rows = operator_log_service::query_operator_logs(&state.db, query.clone()).await?;
    let total = operator_log_service::count_operator_logs(&state.db, query).await?;

    let items = rows
        .into_iter()
        .map(map_operator_log_row)
        .collect::<Vec<_>>();

    Ok(OrionDataGrid {
        page,
        limit,
        total,
        rows: items,
    })
}

pub(super) async fn orion_mine_query_operator_log(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<OrionOperatorLogQueryRequest>,
) -> AppResult<impl axum::response::IntoResponse> {
    let user_id = guard::current_user_id(&headers, &state.config.jwt)?;
    Ok(OrionResponse::ok(
        query_operator_logs(&state, Some(user_id), &payload).await?,
    ))
}

pub(super) async fn orion_mine_update_password(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<OrionMineUpdatePasswordRequest>,
) -> AppResult<impl axum::response::IntoResponse> {
    let user_id = guard::current_user_id(&headers, &state.config.jwt)?;
    let old_hash = system_user_service::get_system_user_password_hash(&state.db, user_id)
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
    system_user_service::update_system_user_password(&state.db, user_id, &new_hash).await?;

    audit_service::log_operator_action(
        &state.db,
        &headers,
        audit_service::OperatorLogParams {
            user_id,
            module: "iam",
            operation: "update_password",
            params: serde_json::json!({}),
            result: 1,
            error_message: None,
        },
    )
    .await?;

    Ok(OrionResponse::ok(true))
}

pub(super) async fn orion_operator_log_query(
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

pub(super) async fn orion_operator_log_count(
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

pub(super) async fn orion_operator_log_delete(
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
    let affected = operator_log_service::soft_delete_operator_logs(&state.db, ids).await? as i64;
    Ok(OrionResponse::ok(affected))
}

pub(super) async fn orion_operator_log_clear(
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

    let affected = operator_log_service::clear_operator_logs(
        &state.db,
        operator_log_service::OrionOperatorLogClearInput {
            user_id: payload.user_id,
            username,
            module,
            operation,
            risk_level,
            result: payload.result,
            start_time,
            end_time,
            limit,
        },
    )
    .await?;
    Ok(OrionResponse::ok(affected as i64))
}

pub(super) fn parse_required_id(v: Option<i64>, field: &str) -> AppResult<i64> {
    let id = v.ok_or_else(|| AppError::BadRequest(format!("{field} is required")))?;
    if id <= 0 {
        return Err(AppError::BadRequest(format!(
            "{field} must be greater than 0"
        )));
    }
    Ok(id)
}

async fn load_user_roles(state: &AppState, user_id: i64) -> AppResult<Vec<OrionRoleQueryResponse>> {
    let rows = role_service::list_roles_by_user_id(&state.db, user_id).await?;

    Ok(rows
        .into_iter()
        .map(|r| OrionRoleQueryResponse {
            id: r.id,
            name: r.name,
            code: r.code,
            status: r.status,
            description: r.description.unwrap_or_default(),
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

pub(super) async fn orion_system_user_create(
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
    let id = system_user_service::create_system_user(
        &state.db,
        system_user_service::OrionSystemUserCreateInput {
            username: username.clone(),
            password_hash: pass_hash,
            nickname: payload.nickname.clone(),
            avatar: payload.avatar.clone(),
            mobile: payload.mobile.clone(),
            email: payload.email.clone(),
        },
    )
    .await?;

    audit_service::log_operator_action(
        &state.db,
        &headers,
        audit_service::OperatorLogParams {
            user_id: actor_user_id,
            module: "iam",
            operation: "create_user",
            params: serde_json::json!({
                "id": id,
                "username": username,
            }),
            result: 1,
            error_message: None,
        },
    )
    .await?;

    Ok(OrionResponse::ok(id))
}

pub(super) async fn orion_system_user_update(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<OrionSystemUserUpsertRequest>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "iam.user-permission.view").await?;
    let actor_user_id = guard::current_user_id(&headers, &state.config.jwt)?;
    let id = parse_required_id(payload.id, "id")?;
    let rows = system_user_service::update_system_user(
        &state.db,
        system_user_service::OrionSystemUserUpdateInput {
            id,
            username: payload.username.as_ref().map(|v| v.trim().to_string()),
            nickname: payload.nickname.clone(),
            avatar: payload.avatar.clone(),
            mobile: payload.mobile.clone(),
            email: payload.email.clone(),
        },
    )
    .await?;
    if rows == 0 {
        return Err(AppError::NotFound("User not found".to_string()));
    }

    audit_service::log_operator_action(
        &state.db,
        &headers,
        audit_service::OperatorLogParams {
            user_id: actor_user_id,
            module: "iam",
            operation: "update_user",
            params: serde_json::json!({
                "id": id,
                "username": payload.username,
            }),
            result: 1,
            error_message: None,
        },
    )
    .await?;

    Ok(OrionResponse::ok(true))
}

pub(super) async fn orion_system_user_update_status(
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
    let rows = system_user_service::update_system_user_status(&state.db, id, status).await?;
    if rows == 0 {
        return Err(AppError::NotFound("User not found".to_string()));
    }

    audit_service::log_operator_action(
        &state.db,
        &headers,
        audit_service::OperatorLogParams {
            user_id: actor_user_id,
            module: "iam",
            operation: "update_user_status",
            params: serde_json::json!({
                "id": id,
                "status": status,
            }),
            result: 1,
            error_message: None,
        },
    )
    .await?;

    Ok(OrionResponse::ok(true))
}

pub(super) async fn orion_system_user_grant_role(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<OrionSystemUserUpsertRequest>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "iam.user-role.assign").await?;
    let actor_user_id = guard::current_user_id(&headers, &state.config.jwt)?;
    let user_id = parse_required_id(payload.id, "id")?;
    let role_ids = payload.role_id_list.clone().unwrap_or_default();
    system_user_service::replace_system_user_roles(&state.db, user_id, role_ids.clone()).await?;

    audit_service::log_operator_action(
        &state.db,
        &headers,
        audit_service::OperatorLogParams {
            user_id: actor_user_id,
            module: "iam",
            operation: "grant_user_role",
            params: serde_json::json!({
                "id": user_id,
                "role_ids": role_ids,
            }),
            result: 1,
            error_message: None,
        },
    )
    .await?;

    Ok(OrionResponse::ok(true))
}
pub(super) async fn orion_system_user_reset_password(
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
    system_user_service::update_system_user_password(&state.db, id, &pass_hash).await?;

    audit_service::log_operator_action(
        &state.db,
        &headers,
        audit_service::OperatorLogParams {
            user_id: actor_user_id,
            module: "iam",
            operation: "reset_user_password",
            params: serde_json::json!({ "id": id }),
            result: 1,
            error_message: None,
        },
    )
    .await?;

    Ok(OrionResponse::ok(true))
}

pub(super) async fn orion_system_user_get(
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

pub(super) async fn orion_system_user_list(
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

pub(super) async fn orion_system_user_query(
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
    let filters = system_user_service::OrionSystemUserQueryFilters {
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

pub(super) async fn orion_system_user_count(
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
        system_user_service::OrionSystemUserQueryFilters {
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

pub(super) async fn orion_system_user_delete(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<OrionSystemUserIdQuery>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "iam.user-permission.view").await?;
    let actor_user_id = guard::current_user_id(&headers, &state.config.jwt)?;
    let id = parse_required_id(query.id, "id")?;
    system_user_service::soft_delete_system_user(&state.db, id).await?;

    audit_service::log_operator_action(
        &state.db,
        &headers,
        audit_service::OperatorLogParams {
            user_id: actor_user_id,
            module: "iam",
            operation: "delete_user",
            params: serde_json::json!({ "id": id }),
            result: 1,
            error_message: None,
        },
    )
    .await?;

    Ok(OrionResponse::ok(true))
}

pub(super) async fn orion_system_user_batch_delete(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<OrionDeleteIdsQuery>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "iam.user-permission.view").await?;
    let actor_user_id = guard::current_user_id(&headers, &state.config.jwt)?;
    let ids = parse_csv_ids(query.id_list);
    if !ids.is_empty() {
        system_user_service::soft_delete_system_users(&state.db, ids.clone()).await?;

        audit_service::log_operator_action(
            &state.db,
            &headers,
            audit_service::OperatorLogParams {
                user_id: actor_user_id,
                module: "iam",
                operation: "batch_delete_user",
                params: serde_json::json!({ "ids": ids }),
                result: 1,
                error_message: None,
            },
        )
        .await?;
    }
    Ok(OrionResponse::ok(true))
}

pub(super) async fn orion_system_user_get_roles(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<OrionSystemUserIdQuery>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "iam.user-permission.view").await?;
    let user_id = parse_required_id(query.user_id, "userId")?;
    let ids = system_user_service::list_system_user_role_ids(&state.db, user_id).await?;
    Ok(OrionResponse::ok(ids))
}

pub(super) async fn orion_system_user_login_history(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<OrionSystemUserIdQuery>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "iam.user-permission.view").await?;
    let username = query
        .username
        .ok_or_else(|| AppError::BadRequest("username is required".to_string()))?;
    let rows =
        system_user_service::list_login_history_by_username(&state.db, &username, 20).await?;
    let data = rows
        .into_iter()
        .map(|r| {
            serde_json::json!({
                "id": r.id,
                "address": r.address,
                "location": r.location,
                "userAgent": r.user_agent,
                "result": r.result,
                "errorMessage": r.error_message,
                "createTime": r.create_time
            })
        })
        .collect::<Vec<_>>();
    Ok(OrionResponse::ok(data))
}

pub(super) async fn orion_system_user_session_users_list(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "iam.user-permission.view").await?;
    let rows = system_user_service::list_active_sessions(&state.db, 200).await?;
    let data = rows
        .into_iter()
        .map(|r| {
            serde_json::json!({
                "id": r.id,
                "username": r.username,
                "visible": true,
                "current": false,
                "address": r.address,
                "location": r.location,
                "userAgent": r.user_agent,
                "loginTime": r.login_time
            })
        })
        .collect::<Vec<_>>();
    Ok(OrionResponse::ok(data))
}

pub(super) async fn orion_system_user_session_user_list(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<OrionSystemUserIdQuery>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "iam.user-permission.view").await?;
    let user_id = parse_required_id(query.id, "id")?;
    let rows = system_user_service::list_active_sessions_by_user(&state.db, user_id, 200).await?;
    let data = rows
        .into_iter()
        .map(|r| {
            serde_json::json!({
                "id": r.id,
                "username": r.username,
                "visible": true,
                "current": false,
                "address": r.address,
                "location": r.location,
                "userAgent": r.user_agent,
                "loginTime": r.login_time
            })
        })
        .collect::<Vec<_>>();
    Ok(OrionResponse::ok(data))
}

pub(super) async fn orion_system_user_session_offline(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<OrionSessionOfflineRequest>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "iam.user-permission.view").await?;
    let user_id = parse_required_id(payload.user_id, "userId")?;
    system_user_service::revoke_refresh_tokens_by_user_and_timestamp(
        &state.db,
        user_id,
        payload.timestamp,
    )
    .await?;
    Ok(OrionResponse::ok(true))
}

pub(super) async fn orion_system_user_locked_list() -> AppResult<impl axum::response::IntoResponse>
{
    Ok(OrionResponse::ok(Vec::<serde_json::Value>::new()))
}

pub(super) async fn orion_system_user_locked_unlock() -> AppResult<impl axum::response::IntoResponse>
{
    Ok(OrionResponse::ok(true))
}

pub(super) async fn orion_system_role_create(
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
    let id = role_service::create_role(
        &state.db,
        role_service::OrionRoleCreateInput {
            name: name.clone(),
            code: code.clone(),
            description: payload.description.clone(),
            status: payload.status,
        },
    )
    .await?;

    audit_service::log_operator_action(
        &state.db,
        &headers,
        audit_service::OperatorLogParams {
            user_id: actor_user_id,
            module: "iam",
            operation: "create_role",
            params: serde_json::json!({
                "id": id,
                "name": name,
                "code": code,
            }),
            result: 1,
            error_message: None,
        },
    )
    .await?;

    Ok(OrionResponse::ok(id))
}

pub(super) async fn orion_system_role_update(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<OrionSystemRoleRequest>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "iam.user-role.assign").await?;
    let actor_user_id = guard::current_user_id(&headers, &state.config.jwt)?;
    let id = parse_required_id(payload.id, "id")?;
    let rows = role_service::update_role(
        &state.db,
        role_service::OrionRoleUpdateInput {
            id,
            name: payload.name.as_ref().map(|v| v.trim().to_string()),
            code: payload.code.as_ref().map(|v| v.trim().to_string()),
            description: payload.description.clone(),
            status: payload.status,
        },
    )
    .await?;
    if rows == 0 {
        return Err(AppError::NotFound("Role not found".to_string()));
    }

    audit_service::log_operator_action(
        &state.db,
        &headers,
        audit_service::OperatorLogParams {
            user_id: actor_user_id,
            module: "iam",
            operation: "update_role",
            params: serde_json::json!({
                "id": id,
                "name": payload.name,
            }),
            result: 1,
            error_message: None,
        },
    )
    .await?;

    Ok(OrionResponse::ok(true))
}

pub(super) async fn orion_system_role_update_status(
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
    let rows = role_service::update_role_status(&state.db, id, status).await?;
    if rows == 0 {
        return Err(AppError::NotFound("Role not found".to_string()));
    }

    audit_service::log_operator_action(
        &state.db,
        &headers,
        audit_service::OperatorLogParams {
            user_id: actor_user_id,
            module: "iam",
            operation: "update_role_status",
            params: serde_json::json!({
                "id": id,
                "status": status,
            }),
            result: 1,
            error_message: None,
        },
    )
    .await?;

    Ok(OrionResponse::ok(true))
}

fn map_role_row(row: OrionRoleAggregate) -> OrionRoleQueryResponse {
    OrionRoleQueryResponse {
        id: row.id,
        name: row.name,
        code: row.code,
        status: row.status,
        description: row.description.unwrap_or_default(),
        create_time: row.create_time_ms,
        update_time: row.create_time_ms,
        creator: "system".to_string(),
        updater: "system".to_string(),
    }
}

fn map_menu_row(menu: OrionMenuAggregate) -> OrionMenuItem {
    OrionMenuItem {
        id: menu.id,
        parent_id: menu.parent_id,
        name: menu.name,
        permission: menu.permission.unwrap_or_default(),
        r#type: menu.menu_type,
        sort: menu.sort,
        visible: menu.visible,
        status: 1,
        cache: 1,
        new_window: 0,
        icon: menu.icon.unwrap_or_default(),
        path: menu.path.unwrap_or_default(),
        component: menu.component.unwrap_or_default(),
        children: Vec::new(),
    }
}

fn map_dict_key_row(row: OrionDictKeyAggregate) -> OrionDictKeyItem {
    OrionDictKeyItem {
        id: row.id,
        key_name: row.key_name,
        value_type: row.value_type,
        extra_schema: row.extra_schema.unwrap_or_default(),
        description: row.description.unwrap_or_default(),
        create_time: row.create_time,
        update_time: row.update_time,
        creator: row.creator.unwrap_or_else(|| "system".to_string()),
        updater: row.updater.unwrap_or_else(|| "system".to_string()),
    }
}

fn map_dict_value_row(row: OrionDictValueAggregate) -> OrionDictValueItem {
    OrionDictValueItem {
        id: row.id,
        key_id: row.key_id,
        key_name: row.key_name,
        key_description: row.key_description.unwrap_or_default(),
        value: row.value,
        label: row.label,
        extra: row.extra.unwrap_or_default(),
        sort: row.sort,
        create_time: row.create_time,
        update_time: row.update_time,
        creator: row.creator.unwrap_or_else(|| "system".to_string()),
        updater: row.updater.unwrap_or_else(|| "system".to_string()),
    }
}

fn map_system_message_row(row: OrionSystemMessageAggregate) -> OrionSystemMessageItem {
    OrionSystemMessageItem {
        id: row.id,
        classify: row.classify,
        r#type: row.message_type,
        status: row.status,
        rel_key: row.rel_key.unwrap_or_default(),
        title: row.title,
        content: row.content,
        content_html: row.content_html.unwrap_or_default(),
        create_time: row.create_time,
    }
}

pub(super) fn build_operator_log_info(
    operation: &str,
    method: Option<&str>,
    path: Option<&str>,
) -> String {
    let text = format!("{} {}", method.unwrap_or(""), path.unwrap_or(""))
        .trim()
        .to_string();
    if text.is_empty() {
        operation.to_string()
    } else {
        text
    }
}

fn map_operator_log_row(row: OrionOperatorLogAggregate) -> OrionOperatorLogItem {
    let operation = row.operation.unwrap_or_default();
    let log_info = build_operator_log_info(&operation, row.method.as_deref(), row.path.as_deref());
    let result = row.result.unwrap_or_default();

    OrionOperatorLogItem {
        id: row.id,
        user_id: row.user_id.unwrap_or_default(),
        username: row.username.unwrap_or_default(),
        trace_id: row.trace_id.unwrap_or_default(),
        address: row.ip.unwrap_or_default(),
        location: String::new(),
        user_agent: row.user_agent.unwrap_or_default(),
        risk_level: if result == 0 {
            "HIGH".to_string()
        } else {
            "LOW".to_string()
        },
        module: row.module.unwrap_or_default(),
        r#type: operation.clone(),
        log_info,
        origin_log_info: row.path.unwrap_or(operation),
        extra: row
            .params
            .and_then(|v| serde_json::to_string(&v).ok())
            .unwrap_or_else(|| "{}".to_string()),
        result,
        error_message: row.error_message.unwrap_or_default(),
        return_value: String::new(),
        duration: row.duration.unwrap_or_default(),
        start_time: row.create_time,
        end_time: row.create_time,
        create_time: row.create_time,
    }
}

fn map_login_history_row(row: OrionLoginHistoryAggregate) -> OrionLoginHistoryItem {
    let row = infra_statistics_service::fill_login_history_defaults(row);
    OrionLoginHistoryItem {
        id: row.id,
        address: row.address.unwrap_or_default(),
        location: row.location.unwrap_or_default(),
        user_agent: row.user_agent.unwrap_or_default(),
        result: row.result.unwrap_or(0),
        error_message: row.error_message.unwrap_or_default(),
        create_time: row.create_time,
    }
}

fn map_infra_workplace_stats(
    row: OrionInfraWorkplaceAggregate,
) -> OrionInfraWorkplaceStatisticsResponse {
    OrionInfraWorkplaceStatisticsResponse {
        user_id: row.user_id,
        username: row.username,
        nickname: row.nickname.unwrap_or_default(),
        unread_message_count: row.unread_message_count,
        last_login_time: row.last_login_time.unwrap_or(0),
        user_session_count: row.user_session_count,
        operator_chart: OrionLineSingleChartData {
            x: row.operator_chart_labels,
            data: row.operator_chart_values,
        },
        login_history_list: row
            .login_history
            .into_iter()
            .map(map_login_history_row)
            .collect(),
    }
}

pub(super) async fn orion_system_role_get(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<OrionSystemRoleIdQuery>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "iam.user-role.assign").await?;
    let id = parse_required_id(query.id, "id")?;
    let row = role_service::get_role_by_id(&state.db, id)
        .await?
        .ok_or_else(|| AppError::NotFound("Role not found".to_string()))?;
    Ok(OrionResponse::ok(map_role_row(row)))
}

pub(super) async fn orion_system_role_list(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "iam.user-role.assign").await?;
    let rows = role_service::list_roles(&state.db).await?;
    Ok(OrionResponse::ok(
        rows.into_iter().map(map_role_row).collect::<Vec<_>>(),
    ))
}

pub(super) async fn orion_system_role_query(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<OrionSystemRoleRequest>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "iam.user-role.assign").await?;
    let (page, limit, offset) = normalize_pagination(payload.page, payload.limit);
    let rows = role_service::query_roles(&state.db, limit, offset).await?;
    let total = role_service::count_roles(&state.db).await?;
    Ok(OrionResponse::ok(OrionDataGrid {
        page,
        limit,
        total,
        rows: rows.into_iter().map(map_role_row).collect(),
    }))
}

pub(super) async fn orion_system_role_delete(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<OrionSystemRoleIdQuery>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "iam.user-role.assign").await?;
    let id = parse_required_id(query.id, "id")?;
    role_service::soft_delete_role(&state.db, id).await?;
    Ok(OrionResponse::ok(true))
}

pub(super) async fn orion_system_role_grant_menu(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<OrionSystemRoleRequest>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "iam.user-role.assign").await?;
    let role_id = parse_required_id(payload.role_id, "roleId")?;
    role_service::replace_role_menus(&state.db, role_id, payload.menu_id_list.unwrap_or_default())
        .await?;
    Ok(OrionResponse::ok(true))
}

pub(super) async fn orion_system_role_get_menu_id(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<OrionSystemRoleIdQuery>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "iam.user-role.assign").await?;
    let role_id = parse_required_id(query.role_id, "roleId")?;
    let ids = role_service::list_role_menu_ids(&state.db, role_id).await?;
    Ok(OrionResponse::ok(ids))
}

pub(super) async fn orion_system_menu_list(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "iam.user-role.assign").await?;
    let rows = menu_service::list_menus(&state.db).await?;
    let list = build_orion_menu_tree(rows.into_iter().map(map_menu_row).collect());
    Ok(OrionResponse::ok(list))
}

pub(super) async fn orion_system_menu_create(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<OrionSystemMenuRequest>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "iam.user-role.assign").await?;
    let _ = (payload.cache, payload.new_window);
    let name = sanitize_search(payload.name)
        .ok_or_else(|| AppError::BadRequest("name is required".to_string()))?;
    let id = menu_service::create_menu(
        &state.db,
        menu_service::OrionMenuCreateInput {
            parent_id: payload.parent_id,
            name,
            path: payload.path,
            component: payload.component,
            icon: payload.icon,
            menu_type: payload.r#type,
            sort: payload.sort,
            visible: payload.visible,
            permission: payload.permission,
        },
    )
    .await?;
    Ok(OrionResponse::ok(id))
}

pub(super) async fn orion_system_menu_update(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<OrionSystemMenuRequest>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "iam.user-role.assign").await?;
    let id = parse_required_id(payload.id, "id")?;
    menu_service::update_menu(
        &state.db,
        menu_service::OrionMenuUpdateInput {
            id,
            parent_id: payload.parent_id,
            name: payload.name.map(|v| v.trim().to_string()),
            path: payload.path,
            component: payload.component,
            icon: payload.icon,
            menu_type: payload.r#type,
            sort: payload.sort,
            visible: payload.visible,
            permission: payload.permission,
        },
    )
    .await?;
    Ok(OrionResponse::ok(true))
}

pub(super) async fn orion_system_menu_update_status(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<OrionSystemMenuRequest>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "iam.user-role.assign").await?;
    let id = parse_required_id(payload.id, "id")?;
    let visible = payload.status.unwrap_or(1);
    menu_service::update_menu_visible(&state.db, id, visible).await?;
    Ok(OrionResponse::ok(true))
}

pub(super) async fn orion_system_menu_delete(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<OrionSystemRoleIdQuery>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "iam.user-role.assign").await?;
    let id = parse_required_id(query.id, "id")?;
    menu_service::delete_menu(&state.db, id).await?;
    Ok(OrionResponse::ok(true))
}

pub(super) async fn orion_system_menu_refresh_cache() -> AppResult<impl axum::response::IntoResponse>
{
    Ok(OrionResponse::ok(true))
}

pub(super) async fn orion_terminal_themes(
    State(state): State<AppState>,
) -> AppResult<impl axum::response::IntoResponse> {
    let keys = vec!["terminalTheme".to_string()];
    let rows = dict_value_service::list_dict_values_by_keys(&state.db, &keys).await?;
    let themes = build_terminal_themes(rows);

    Ok(OrionResponse::ok(themes))
}

pub(super) fn build_terminal_themes(
    rows: Vec<OrionDictValueOptionAggregate>,
) -> Vec<serde_json::Value> {
    let mut themes = Vec::with_capacity(rows.len());
    for row in rows {
        let schema: serde_json::Value =
            serde_json::from_str(&row.value).unwrap_or_else(|_| serde_json::json!({}));
        let dark = row
            .extra
            .as_deref()
            .and_then(|extra| serde_json::from_str::<serde_json::Value>(extra).ok())
            .and_then(|extra| extra.get("dark").and_then(serde_json::Value::as_bool))
            .unwrap_or(false);

        themes.push(serde_json::json!({
            "name": row.label,
            "dark": dark,
            "schema": schema
        }));
    }
    themes
}

pub(super) async fn orion_terminal_access(
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

pub(super) async fn orion_terminal_transfer(
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

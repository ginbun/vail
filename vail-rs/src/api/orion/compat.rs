use super::*;

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub(super) struct OrionCompatQuery {
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

pub(super) fn normalize_group_ids(ids: Option<Vec<i64>>) -> AppResult<Vec<i64>> {
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

pub(super) async fn orion_exec_dispatch(
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

pub(super) async fn orion_terminal_connect_log_query(
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

pub(super) async fn orion_terminal_connect_log_count(
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

pub(super) async fn orion_terminal_connect_log_delete(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<OrionCompatQuery>,
) -> AppResult<impl axum::response::IntoResponse> {
    let _ = guard::current_user_id(&headers, &state.config.jwt)?;
    Ok(orion_ok(
        delete_terminal_module(&state, OrionCompatModule::TerminalConnectLog, &query).await?,
    ))
}

pub(super) async fn orion_terminal_connect_log_clear(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<impl axum::response::IntoResponse> {
    let _ = guard::current_user_id(&headers, &state.config.jwt)?;
    Ok(orion_ok(
        clear_terminal_module(&state, OrionCompatModule::TerminalConnectLog).await?,
    ))
}

pub(super) async fn orion_terminal_connect_log_sessions(
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

pub(super) async fn orion_terminal_connect_log_latest(
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

pub(super) async fn orion_terminal_connect_log_force_offline(
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

pub(super) async fn orion_terminal_file_log_query(
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

pub(super) async fn orion_terminal_file_log_count(
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

pub(super) async fn orion_terminal_file_log_delete(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<OrionCompatQuery>,
) -> AppResult<impl axum::response::IntoResponse> {
    let _ = guard::current_user_id(&headers, &state.config.jwt)?;
    Ok(orion_ok(
        delete_terminal_module(&state, OrionCompatModule::TerminalFileLog, &query).await?,
    ))
}

pub(super) async fn orion_terminal_dispatch(
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

pub(super) async fn orion_infra_dispatch(
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
            let mut rng = OsRng;
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
                    audit_service::OperatorLogParams {
                        user_id,
                        module: "tag",
                        operation: "create",
                        params: payload,
                        result: 1,
                        error_message: None,
                    },
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
                    audit_service::OperatorLogParams {
                        user_id,
                        module: "tag",
                        operation: "update",
                        params: payload,
                        result: 1,
                        error_message: None,
                    },
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
                    audit_service::OperatorLogParams {
                        user_id,
                        module: "tag",
                        operation: "delete",
                        params: serde_json::json!({ "id": id }),
                        result: 1,
                        error_message: None,
                    },
                )
                .await;
            }
            Ok(orion_ok(affected > 0))
        }
        _ => Err(AppError::NotFound("unsupported infra action".to_string())),
    }
}

pub(super) async fn orion_compat_fallback(
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
    } else if path.ends_with("/get") || path.ends_with("/status") {
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

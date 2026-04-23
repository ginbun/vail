use super::*;

pub(super) async fn orion_list_hosts(
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

pub(super) async fn orion_get_host(
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

pub(super) async fn orion_query_hosts(
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

    let filters = host_service::OrionHostQueryFilters {
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

pub(super) async fn orion_count_hosts(
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
        host_service::OrionHostQueryFilters {
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

pub(super) async fn orion_create_host(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<OrionHostCreateRequest>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "host.create").await?;
    let actor_user_id = guard::current_user_id(&headers, &state.config.jwt)?;
    let group_ids = normalize_group_ids(payload.group_id_list)?;

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

    let audit_name = name.clone();
    let audit_address = address.clone();
    let audit_group_ids = group_ids.clone();
    let new_id = asset_service::create_host(
        &state.db,
        asset_service::OrionHostCreateInput {
            name,
            hostname: address,
            description: payload.description,
            group_ids,
        },
    )
    .await?;

    audit_service::log_operator_action(
        &state.db,
        &headers,
        audit_service::OperatorLogParams {
            user_id: actor_user_id,
            module: "asset",
            operation: "create_host",
            params: serde_json::json!({
                "id": new_id,
                "name": audit_name,
                "address": audit_address,
                "groupIdList": audit_group_ids,
            }),
            result: 1,
            error_message: None,
        },
    )
    .await?;

    Ok(OrionResponse::ok(new_id))
}

pub(super) async fn orion_update_host(
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
    let audit_name = name.clone();
    let audit_address = address.clone();
    let audit_group_ids = group_ids.clone();

    asset_service::update_host(
        &state.db,
        asset_service::OrionHostUpdateInput {
            id,
            name,
            hostname: address,
            description,
            group_ids,
        },
    )
    .await?;

    audit_service::log_operator_action(
        &state.db,
        &headers,
        audit_service::OperatorLogParams {
            user_id: actor_user_id,
            module: "asset",
            operation: "update_host",
            params: serde_json::json!({
                "id": id,
                "name": audit_name,
                "address": audit_address,
                "groupIdList": audit_group_ids,
            }),
            result: 1,
            error_message: None,
        },
    )
    .await?;

    Ok(OrionResponse::ok(true))
}

pub(super) async fn orion_update_host_status(
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

    asset_service::update_host_status(&state.db, id, status).await?;

    audit_service::log_operator_action(
        &state.db,
        &headers,
        audit_service::OperatorLogParams {
            user_id: actor_user_id,
            module: "asset",
            operation: "update_host_status",
            params: serde_json::json!({
                "id": id,
                "status": status_text,
            }),
            result: 1,
            error_message: None,
        },
    )
    .await?;

    Ok(OrionResponse::ok(true))
}

pub(super) async fn orion_host_extra_get(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<OrionHostExtraQuery>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "host.read").await?;

    let user_id = guard::current_user_id(&headers, &state.config.jwt)?;
    let host_id = parse_required_id(query.host_id, "hostId")?;
    let item = query
        .item
        .map(|v| v.trim().to_ascii_uppercase())
        .filter(|v| !v.is_empty())
        .ok_or_else(|| AppError::BadRequest("item is required".to_string()))?;

    let extra = asset_service::get_host_extra(&state.db, user_id, host_id, &item).await?;

    Ok(OrionResponse::ok(extra))
}

pub(super) async fn orion_host_extra_update(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<OrionHostExtraUpdateRequest>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "host.update").await?;

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

    asset_service::update_host_extra(&state.db, user_id, host_id, &item, &extra).await?;

    Ok(OrionResponse::ok(true))
}

pub(super) async fn orion_host_config_get(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<OrionHostConfigGetRequest>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "host.read").await?;

    let host_id = parse_required_id(payload.host_id, "hostId")?;
    let config_type = asset_service::normalize_host_config_type(payload.r#type)?;
    asset_service::ensure_host_exists(&state.db, host_id).await?;
    let config = asset_service::get_host_config(&state.db, host_id, &config_type).await?;

    Ok(OrionResponse::ok(config))
}

pub(super) async fn orion_host_config_update(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<OrionHostConfigUpdateRequest>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "host.update").await?;

    let host_id = parse_required_id(payload.host_id, "hostId")?;
    let config_type = asset_service::normalize_host_config_type(payload.r#type)?;
    let config_text = payload
        .config
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .ok_or_else(|| AppError::BadRequest("config is required".to_string()))?;
    let config: serde_json::Value = serde_json::from_str(config_text)
        .map_err(|_| AppError::BadRequest("config must be valid JSON".to_string()))?;

    let mut ssh_password_plain: Option<String> = None;
    if config_type == "SSH" {
        let auth_type = config
            .get("authType")
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .unwrap_or("PASSWORD")
            .to_ascii_uppercase();
        let use_new_password = config
            .get("useNewPassword")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);

        if auth_type == "PASSWORD" && use_new_password {
            let encrypted_password = config
                .get("password")
                .and_then(serde_json::Value::as_str)
                .map(str::to_string)
                .ok_or_else(|| {
                    AppError::BadRequest(
                        "password is required when useNewPassword=true".to_string(),
                    )
                })?;
            ssh_password_plain =
                decrypt_client_sensitive_input(&state, Some(encrypted_password), "password")
                    .await?
                    .and_then(|v| sanitize_search(Some(v)));
        }
    }

    asset_service::update_host_config(
        &state.db,
        host_id,
        &config_type,
        &config,
        ssh_password_plain.as_deref(),
        &state.config.secrets.data_encryption_key,
    )
    .await?;

    Ok(OrionResponse::ok(true))
}

pub(super) async fn orion_delete_host(
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

    asset_service::delete_host(&state.db, query.id).await?;

    audit_service::log_operator_action(
        &state.db,
        &headers,
        audit_service::OperatorLogParams {
            user_id: actor_user_id,
            module: "asset",
            operation: "delete_host",
            params: serde_json::json!({ "id": query.id }),
            result: 1,
            error_message: None,
        },
    )
    .await?;

    Ok(OrionResponse::ok(true))
}

pub(super) async fn orion_host_group_tree(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "host.read").await?;

    let rows = asset_service::list_host_groups_for_tree(&state.db).await?;
    Ok(OrionResponse::ok(build_host_group_tree(rows)))
}

pub(super) async fn orion_create_host_group(
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

    let id = asset_service::create_host_group(&state.db, parent_id, &name).await?;

    Ok(OrionResponse::ok(id))
}

pub(super) async fn orion_rename_host_group(
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

    asset_service::rename_host_group(&state.db, id, &name).await?;

    Ok(OrionResponse::ok(true))
}

pub(super) async fn orion_move_host_group(
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

    asset_service::move_host_group(&state.db, id, target_id, position).await?;

    Ok(OrionResponse::ok(true))
}

pub(super) async fn orion_delete_host_group(
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

    asset_service::delete_host_group(&state.db, query.id).await?;

    Ok(OrionResponse::ok(true))
}

pub(super) async fn orion_host_group_rel_list(
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

    let list = asset_service::list_host_group_rel_host_ids(&state.db, query.group_id).await?;

    Ok(OrionResponse::ok(list))
}

pub(super) async fn orion_update_host_group_rel(
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

    asset_service::replace_host_group_rel(
        &state.db,
        group_id,
        host_ids.into_iter().collect::<Vec<_>>(),
    )
    .await?;

    Ok(OrionResponse::ok(true))
}

pub(super) async fn orion_host_key_create(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<OrionHostKeyUpsertRequest>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "host.create").await?;
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
        audit_service::OperatorLogParams {
            user_id: actor_user_id,
            module: "asset",
            operation: "create_host_key",
            params: serde_json::json!({
                "id": id,
                "name": audit_name,
                "description": audit_description,
            }),
            result: 1,
            error_message: None,
        },
    )
    .await?;

    Ok(OrionResponse::ok(id))
}

pub(super) async fn orion_host_key_update(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<OrionHostKeyUpsertRequest>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "host.update").await?;
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
        audit_service::OperatorLogParams {
            user_id: actor_user_id,
            module: "asset",
            operation: "update_host_key",
            params: serde_json::json!({
                "id": id,
                "name": audit_name,
                "description": audit_description,
                "useNewPassword": use_new_password,
                "updatedPrivateKey": private_key_plain.is_some(),
            }),
            result: 1,
            error_message: None,
        },
    )
    .await?;

    Ok(OrionResponse::ok(true))
}

pub(super) async fn orion_host_key_get(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<OrionIdQuery>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "host.read").await?;
    let _user_id = guard::current_user_id(&headers, &state.config.jwt)?;
    let id = parse_required_id(query.id, "id")?;

    let row = asset_service::get_host_key(&state.db, id).await?;
    Ok(OrionResponse::ok(map_host_key_item(row)))
}

pub(super) async fn orion_host_key_list(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "host.read").await?;
    let _user_id = guard::current_user_id(&headers, &state.config.jwt)?;

    let list = asset_service::list_host_keys(&state.db)
        .await?
        .into_iter()
        .map(map_host_key_item)
        .collect::<Vec<_>>();

    Ok(OrionResponse::ok(list))
}

pub(super) async fn orion_host_key_query(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<OrionHostKeyQueryRequest>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "host.read").await?;
    let _user_id = guard::current_user_id(&headers, &state.config.jwt)?;
    let (page, limit, offset) = normalize_pagination(payload.page, payload.limit);
    let (total, rows) = asset_service::query_host_keys(
        &state.db,
        asset_service::OrionHostKeyQueryFilters {
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

pub(super) async fn orion_host_key_delete(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<OrionIdQuery>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "host.delete").await?;
    let actor_user_id = guard::current_user_id(&headers, &state.config.jwt)?;
    let id = parse_required_id(query.id, "id")?;

    asset_service::delete_host_key(&state.db, id).await?;

    audit_service::log_operator_action(
        &state.db,
        &headers,
        audit_service::OperatorLogParams {
            user_id: actor_user_id,
            module: "asset",
            operation: "delete_host_key",
            params: serde_json::json!({ "id": id }),
            result: 1,
            error_message: None,
        },
    )
    .await?;

    Ok(OrionResponse::ok(true))
}

pub(super) async fn orion_host_key_batch_delete(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<OrionDeleteIdsQuery>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "host.delete").await?;
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
        audit_service::OperatorLogParams {
            user_id: actor_user_id,
            module: "asset",
            operation: "batch_delete_host_key",
            params: serde_json::json!({ "ids": audit_ids }),
            result: 1,
            error_message: None,
        },
    )
    .await?;

    Ok(OrionResponse::ok(true))
}

pub(super) async fn orion_host_identity_create(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<OrionHostIdentityUpsertRequest>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "host.create").await?;
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

pub(super) async fn orion_host_identity_update(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<OrionHostIdentityUpsertRequest>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "host.update").await?;
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

pub(super) async fn orion_host_identity_get(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<OrionIdQuery>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "host.read").await?;
    let _user_id = guard::current_user_id(&headers, &state.config.jwt)?;
    let id = parse_required_id(query.id, "id")?;

    let item = asset_service::get_host_identity(&state.db, id).await?;
    Ok(OrionResponse::ok(map_host_identity_item(item)))
}

pub(super) async fn orion_host_identity_list(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "host.read").await?;
    let _user_id = guard::current_user_id(&headers, &state.config.jwt)?;
    let items = asset_service::list_host_identities(&state.db)
        .await?
        .into_iter()
        .map(map_host_identity_item)
        .collect::<Vec<_>>();
    Ok(OrionResponse::ok(items))
}

pub(super) async fn orion_host_identity_query(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<OrionHostIdentityQueryRequest>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "host.read").await?;
    let _user_id = guard::current_user_id(&headers, &state.config.jwt)?;
    let (page, limit, offset) = normalize_pagination(payload.page, payload.limit);

    let (total, rows) = asset_service::query_host_identities(
        &state.db,
        asset_service::OrionHostIdentityQueryFilters {
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

pub(super) async fn orion_host_identity_delete(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<OrionIdQuery>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "host.delete").await?;
    let _user_id = guard::current_user_id(&headers, &state.config.jwt)?;
    let id = parse_required_id(query.id, "id")?;
    asset_service::delete_host_identity(&state.db, id).await?;
    Ok(OrionResponse::ok(true))
}

pub(super) async fn orion_host_identity_batch_delete(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<OrionDeleteIdsQuery>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "host.delete").await?;
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

pub(super) async fn orion_data_grant_host_group(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<OrionAssetDataGrantRequest>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_any_permission(
        &state,
        &headers,
        &["asset.data-grant.host-group.assign", "host.update"],
    )
    .await?;
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

pub(super) async fn orion_data_grant_get_host_group(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<OrionAssetAuthorizedDataQuery>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_any_permission(
        &state,
        &headers,
        &["asset.data-grant.host-group.view", "host.read"],
    )
    .await?;
    let _user_id = guard::current_user_id(&headers, &state.config.jwt)?;
    let scope = asset_service::resolve_grant_scope(query.user_id, query.role_id)?;
    let list = asset_service::list_asset_grants(&state.db, scope, "host-group").await?;
    Ok(OrionResponse::ok(list))
}

pub(super) async fn orion_data_grant_host_key(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<OrionAssetDataGrantRequest>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_any_permission(
        &state,
        &headers,
        &["asset.data-grant.host-key.assign", "host.update"],
    )
    .await?;
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

pub(super) async fn orion_data_grant_get_host_key(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<OrionAssetAuthorizedDataQuery>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_any_permission(
        &state,
        &headers,
        &["asset.data-grant.host-key.view", "host.read"],
    )
    .await?;
    let _user_id = guard::current_user_id(&headers, &state.config.jwt)?;
    let scope = asset_service::resolve_grant_scope(query.user_id, query.role_id)?;
    let list = asset_service::list_asset_grants(&state.db, scope, "host-key").await?;
    Ok(OrionResponse::ok(list))
}

pub(super) async fn orion_data_grant_host_identity(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<OrionAssetDataGrantRequest>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_any_permission(
        &state,
        &headers,
        &["asset.data-grant.host-identity.assign", "host.update"],
    )
    .await?;
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

pub(super) async fn orion_data_grant_get_host_identity(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<OrionAssetAuthorizedDataQuery>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_any_permission(
        &state,
        &headers,
        &["asset.data-grant.host-identity.view", "host.read"],
    )
    .await?;
    let _user_id = guard::current_user_id(&headers, &state.config.jwt)?;
    let scope = asset_service::resolve_grant_scope(query.user_id, query.role_id)?;
    let list = asset_service::list_asset_grants(&state.db, scope, "host-identity").await?;
    Ok(OrionResponse::ok(list))
}

/// GET /asset/authorized-data/current-host
/// Returns authorized hosts for the current user with group tree structure
pub(super) async fn orion_authorized_data_current_host(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(_query): Query<OrionHostListQuery>,
) -> AppResult<impl axum::response::IntoResponse> {
    let user_id = guard::current_user_id(&headers, &state.config.jwt)?;
    let data = asset_service::list_authorized_current_hosts(&state.db, user_id).await?;
    let group_tree = build_host_group_tree(data.group_tree);
    let host_list = data
        .host_list
        .into_iter()
        .map(|host| {
            serde_json::json!({
                "id": host.id,
                "types": ["SSH"],
                "osType": "linux",
                "archType": "x86_64",
                "name": host.name,
                "code": format!("host-{}", host.id),
                "address": host.hostname,
                "port": host.port,
                "status": "ENABLED",
                "agentKey": "",
                "agentVersion": "",
                "agentInstallStatus": 0,
                "agentOnlineStatus": 0,
                "agentOnlineChangeTime": 0,
                "description": "",
                "groupIdList": host.group_ids,
                "alias": host.name,
                "color": "",
                "tags": [],
                "spec": {},
                "favorite": false,
                "editable": true,
                "loading": false,
                "modCount": 0,
                "createTime": host.create_time_ms,
                "updateTime": host.update_time_ms,
            })
        })
        .collect::<Vec<_>>();

    Ok(OrionResponse::ok(serde_json::json!({
        "groupTree": group_tree,
        "hostList": host_list,
        "treeNodes": data.tree_nodes,
        "latestHosts": []
    })))
}

/// GET /asset/authorized-data/current-host-key
/// Returns authorized SSH keys for the current user
pub(super) async fn orion_authorized_data_current_host_key(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<impl axum::response::IntoResponse> {
    let user_id = guard::current_user_id(&headers, &state.config.jwt)?;
    let key_list = asset_service::list_authorized_current_host_keys(&state.db, user_id)
        .await?
        .into_iter()
        .map(|key| {
            serde_json::json!({
                "id": key.id,
                "name": key.name,
                "description": key.description.unwrap_or_default(),
                "createTime": key.create_time_ms,
                "updateTime": key.update_time_ms,
            })
        })
        .collect::<Vec<_>>();

    Ok(OrionResponse::ok(key_list))
}

/// GET /asset/authorized-data/current-host-identity
/// Returns authorized host identities for the current user
pub(super) async fn orion_authorized_data_current_host_identity(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<impl axum::response::IntoResponse> {
    let user_id = guard::current_user_id(&headers, &state.config.jwt)?;
    let identity_list = asset_service::list_authorized_current_host_identities(&state.db, user_id)
        .await?
        .into_iter()
        .map(|identity| {
            serde_json::json!({
                "id": identity.id,
                "name": identity.name,
                "type": identity.identity_type,
                "username": identity.username.unwrap_or_default(),
                "keyId": identity.key_id,
                "description": identity.description.unwrap_or_default(),
                "createTime": identity.create_time_ms,
                "updateTime": identity.update_time_ms,
            })
        })
        .collect::<Vec<_>>();

    Ok(OrionResponse::ok(identity_list))
}

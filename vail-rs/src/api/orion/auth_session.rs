use super::*;

pub(super) async fn orion_login(
    State(state): State<AppState>,
    headers: HeaderMap,
    connect_info: Option<ConnectInfo<SocketAddr>>,
    Json(payload): Json<OrionLoginRequest>,
) -> AppResult<impl axum::response::IntoResponse> {
    let source_ip = get_source_ip(&headers, connect_info.as_ref());
    let user_agent = headers
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .map(|v| v.to_string());
    let user = iam_service::get_login_user_by_username(&state.db, &payload.username)
        .await?
        .ok_or_else(|| AppError::Auth("Invalid username or password".to_string()))?;

    let password_ok = verify_orion_password(&payload.password, &user.password_hash);

    if !password_ok {
        iam_service::insert_login_log(
            &state.db,
            None,
            &payload.username,
            &source_ip,
            0,
            Some("invalid password"),
        )
        .await
        .ok();

        return Err(AppError::Auth("Invalid username or password".to_string()));
    }

    let session_id = uuid::Uuid::new_v4().to_string();
    let token = auth::create_token(
        user.id,
        &user.username,
        &session_id,
        &state.config.jwt,
        state.config.jwt.expiration,
    )?;
    let mut token_hasher = Sha256::new();
    token_hasher.update(format!("orion:{}", token).as_bytes());
    let token_hash = format!("{:x}", token_hasher.finalize());

    iam_service::insert_refresh_token(
        &state.db,
        user.id,
        &token_hash,
        &session_id,
        state.config.jwt.expiration as i64,
        &source_ip,
        user_agent.as_deref(),
    )
    .await?;

    iam_service::update_user_last_login(&state.db, user.id, &source_ip).await?;

    iam_service::insert_login_log(
        &state.db,
        Some(user.id),
        &user.username,
        &source_ip,
        1,
        None,
    )
    .await?;

    Ok(OrionResponse::ok(OrionLoginResponse { token }))
}

pub(super) async fn orion_logout(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<impl axum::response::IntoResponse> {
    if let Ok(user_id) = guard::current_user_id(&headers, &state.config.jwt) {
        iam_service::revoke_active_refresh_tokens(&state.db, user_id).await?;
    }

    Ok(OrionResponse::ok(true))
}

pub(super) async fn orion_user_aggregate(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<impl axum::response::IntoResponse> {
    let user_id = guard::current_user_id(&headers, &state.config.jwt)?;

    let user = iam_service::get_user_aggregate_principal_by_id(&state.db, user_id)
        .await?
        .ok_or_else(|| AppError::Auth("User not found".to_string()))?;
    let roles = iam_service::list_role_codes_by_user_id(&state.db, user_id).await?;
    let permissions = iam_service::list_permissions_by_user_id(&state.db, user_id).await?;

    let tipped_keys = get_tipped_keys(&state, user_id).await?;

    let data = OrionUserAggregateResponse {
        user: OrionUserBaseResponse {
            id: user.id,
            username: user.username,
            nickname: user.nickname,
            avatar: user.avatar,
        },
        roles,
        permissions,
        system_preference: serde_json::json!({}),
        tipped_keys,
    };

    Ok(OrionResponse::ok(data))
}

pub(super) async fn orion_tips_tipped(
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

    let mut tipped_keys = get_tipped_keys(&state, user_id).await?;
    if !tipped_keys.iter().any(|k| k == &key) {
        tipped_keys.push(key);
        save_tipped_keys(&state, user_id, &tipped_keys).await?;
    }

    Ok(OrionResponse::ok(true))
}

pub(super) async fn orion_tips_get(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<impl axum::response::IntoResponse> {
    let user_id = guard::current_user_id(&headers, &state.config.jwt)?;
    Ok(OrionResponse::ok(get_tipped_keys(&state, user_id).await?))
}

pub(super) async fn orion_user_menu(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<impl axum::response::IntoResponse> {
    let user_id = guard::current_user_id(&headers, &state.config.jwt)?;

    let rows = iam_service::list_user_menu_entries_by_user_id(&state.db, user_id).await?;

    let tree = build_orion_menu_tree(
        rows.into_iter()
            .map(|row| OrionMenuItem {
                id: row.id,
                parent_id: row.parent_id,
                name: row.name,
                permission: row.permission,
                r#type: row.menu_type,
                sort: row.sort,
                visible: row.visible,
                status: 1,
                cache: 1,
                new_window: 0,
                icon: row.icon,
                path: row.path,
                component: row.component,
                children: Vec::new(),
            })
            .collect(),
    );

    Ok(OrionResponse::ok(tree))
}

use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    routing::post,
    Json, Router,
};
use serde::Deserialize;

use crate::{
    api::{guard, AppState},
    error::{AppError, AppResult},
    model::{ApiResponse, CreateSshSessionRequest, SshSessionItem},
    ssh_client,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/ssh/sessions", post(create_session).get(list_sessions))
        .route("/ssh/sessions/:id/disconnect", post(disconnect_session))
}

#[derive(Debug, Deserialize)]
struct ListSessionsQuery {
    host_id: Option<i64>,
}

fn header_string(headers: &HeaderMap, key: &str) -> Option<String> {
    headers
        .get(key)
        .and_then(|v| v.to_str().ok())
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

fn source_ip(headers: &HeaderMap) -> Option<String> {
    header_string(headers, "x-forwarded-for")
        .and_then(|v| v.split(',').next().map(|s| s.trim().to_string()))
        .or_else(|| header_string(headers, "x-real-ip"))
}

async fn append_operator_log(
    state: &AppState,
    headers: &HeaderMap,
    actor_user_id: i64,
    operation: &str,
    params: serde_json::Value,
    result: i16,
    error_message: Option<String>,
) -> AppResult<()> {
    let username = sqlx::query_scalar::<_, Option<String>>(
        "SELECT username FROM sys_user WHERE id = $1 AND deleted = 0",
    )
    .bind(actor_user_id)
    .fetch_one(&state.db)
    .await?
    .unwrap_or_else(|| actor_user_id.to_string());

    sqlx::query(
        "INSERT INTO operator_log (
            user_id,
            username,
            module,
            operation,
            method,
            path,
            params,
            result,
            error_message,
            duration,
            trace_id,
            ip,
            user_agent,
            create_time
        ) VALUES ($1, $2, 'ssh', $3, NULL, NULL, $4::jsonb, $5, $6, NULL, NULL, $7, $8, NOW())",
    )
    .bind(actor_user_id)
    .bind(username)
    .bind(operation)
    .bind(params.to_string())
    .bind(result)
    .bind(error_message)
    .bind(source_ip(headers))
    .bind(header_string(headers, "user-agent"))
    .execute(&state.db)
    .await?;

    Ok(())
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

async fn create_session(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<CreateSshSessionRequest>,
) -> AppResult<impl axum::response::IntoResponse> {
    if payload.host_id <= 0 {
        return Err(AppError::BadRequest(
            "host_id must be greater than 0".to_string(),
        ));
    }

    let user_id = guard::current_user_id(&headers, &state.config.jwt.secret)?;
    let is_admin = has_host_read_permission(&state, user_id).await?;
    if !can_access_host(&state, user_id, payload.host_id, is_admin).await? {
        return Err(AppError::Auth("Host access denied".to_string()));
    }

    let host_ssh_config = ssh_client::resolve_host_ssh_config(
        &state.db,
        &state.config.secrets.data_encryption_key,
        payload.host_id,
    )
    .await?;
    if let Err(err) =
        ssh_client::verify_login(host_ssh_config, state.config.ssh.connection_timeout).await
    {
        append_operator_log(
            &state,
            &headers,
            user_id,
            "create_ssh_session",
            serde_json::json!({
                "host_id": payload.host_id,
            }),
            0,
            Some(err.to_string()),
        )
        .await?;
        return Err(err);
    }

    let session_id = format!("ssh-{}", uuid::Uuid::new_v4());
    let row = sqlx::query_as::<_, (i64, String)>(
        "INSERT INTO ssh_session (user_id, host_id, session_id, status, start_time, create_time)
         VALUES ($1, $2, $3, 1, NOW(), NOW())
         RETURNING id, start_time::text",
    )
    .bind(user_id)
    .bind(payload.host_id)
    .bind(&session_id)
    .fetch_one(&state.db)
    .await?;

    append_operator_log(
        &state,
        &headers,
        user_id,
        "create_ssh_session",
        serde_json::json!({
            "host_id": payload.host_id,
            "session_id": session_id,
        }),
        1,
        None,
    )
    .await?;

    Ok(Json(ApiResponse::success(SshSessionItem {
        id: row.0,
        host_id: payload.host_id,
        session_id,
        status: 1,
        start_time: row.1,
        end_time: None,
    })))
}

async fn list_sessions(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<ListSessionsQuery>,
) -> AppResult<impl axum::response::IntoResponse> {
    let user_id = guard::current_user_id(&headers, &state.config.jwt.secret)?;
    let is_admin = has_host_read_permission(&state, user_id).await?;

    let rows = if is_admin {
        sqlx::query_as::<_, (i64, i64, String, i16, String, Option<String>)>(
            "SELECT id, host_id, session_id, status, start_time::text, end_time::text
             FROM ssh_session
             WHERE ($1::bigint IS NULL OR host_id = $1)
             ORDER BY start_time DESC
             LIMIT 100",
        )
        .bind(query.host_id)
        .fetch_all(&state.db)
        .await?
    } else {
        sqlx::query_as::<_, (i64, i64, String, i16, String, Option<String>)>(
            "SELECT id, host_id, session_id, status, start_time::text, end_time::text
             FROM ssh_session
             WHERE user_id = $1
               AND ($2::bigint IS NULL OR host_id = $2)
             ORDER BY start_time DESC
             LIMIT 100",
        )
        .bind(user_id)
        .bind(query.host_id)
        .fetch_all(&state.db)
        .await?
    };

    let sessions = rows
        .into_iter()
        .map(|row| SshSessionItem {
            id: row.0,
            host_id: row.1,
            session_id: row.2,
            status: row.3,
            start_time: row.4,
            end_time: row.5,
        })
        .collect::<Vec<_>>();

    Ok(Json(ApiResponse::success(sessions)))
}

async fn disconnect_session(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> AppResult<impl axum::response::IntoResponse> {
    if id <= 0 {
        return Err(AppError::BadRequest(
            "session id must be greater than 0".to_string(),
        ));
    }

    let actor_id = guard::current_user_id(&headers, &state.config.jwt.secret)?;
    let is_admin = has_host_read_permission(&state, actor_id).await?;

    let row = sqlx::query_as::<_, (i64, i64, i16)>(
        "SELECT user_id, host_id, status
         FROM ssh_session
         WHERE id = $1
         ORDER BY start_time DESC
         LIMIT 1",
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound("Session not found".to_string()))?;

    if !is_admin && row.0 != actor_id {
        return Err(AppError::Auth("Session access denied".to_string()));
    }

    let status = 2;
    sqlx::query(
        "UPDATE ssh_session
         SET status = $1, end_time = COALESCE(end_time, NOW())
         WHERE id = $2 AND end_time IS NULL",
    )
    .bind(status)
    .bind(id)
    .execute(&state.db)
    .await?;

    append_operator_log(
        &state,
        &headers,
        actor_id,
        "disconnect_ssh_session",
        serde_json::json!({
            "session_id": id,
            "target_user_id": row.0,
            "host_id": row.1,
        }),
        1,
        None,
    )
    .await?;

    Ok(Json(ApiResponse::success(serde_json::json!({
        "id": id,
        "status": status
    }))))
}

use axum::{
    extract::State,
    http::HeaderMap,
    routing::{get, post},
    Json, Router,
};

use crate::{
    api::{guard, AppState},
    error::AppResult,
    model::{ApiResponse, JitApproveRequest, JitRequestCreateRequest},
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/jit/request", post(create_request))
        .route("/jit/approve", post(approve_request))
        .route("/jit/my-active", get(list_my_active))
}

async fn create_request(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Json(payload): axum::extract::Json<JitRequestCreateRequest>,
) -> AppResult<impl axum::response::IntoResponse> {
    let requester_id = guard::require_permission(&state, &headers, "jit.request").await?;

    let minutes = payload.duration_minutes.unwrap_or(60).clamp(5, 480);

    let row = sqlx::query_as::<_, (i64, String, Option<String>)>(
        "INSERT INTO jit_request (requester_id, reason, status, requested_at, expires_at)
         VALUES ($1, $2, 'requested', NOW(), NOW() + ($3 || ' minutes')::interval)
         RETURNING id, status, expires_at::text",
    )
    .bind(requester_id)
    .bind(payload.reason)
    .bind(minutes)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(ApiResponse::success(serde_json::json!({
        "id": row.0,
        "status": row.1,
        "expires_at": row.2
    }))))
}

async fn approve_request(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Json(payload): axum::extract::Json<JitApproveRequest>,
) -> AppResult<impl axum::response::IntoResponse> {
    let approver_id = guard::require_permission(&state, &headers, "jit.approve").await?;

    if payload.approve {
        let minutes = payload.duration_minutes.unwrap_or(60).clamp(5, 480);
        sqlx::query(
            "UPDATE jit_request
             SET status = 'approved',
                 approver_id = $1,
                 approved_at = NOW(),
                 revoked_at = NULL,
                 expires_at = COALESCE(expires_at, NOW() + ($2 || ' minutes')::interval)
             WHERE id = $3 AND status = 'requested'",
        )
        .bind(approver_id)
        .bind(minutes)
        .bind(payload.request_id)
        .execute(&state.db)
        .await?;
    } else {
        sqlx::query(
            "UPDATE jit_request
             SET status = 'revoked',
                 approver_id = $1,
                 revoked_at = NOW()
             WHERE id = $2 AND status IN ('requested', 'approved')",
        )
        .bind(approver_id)
        .bind(payload.request_id)
        .execute(&state.db)
        .await?;
    }

    let row = sqlx::query_as::<_, (i64, String, Option<String>, Option<i64>)>(
        "SELECT id, status, expires_at::text, approver_id
         FROM jit_request
         WHERE id = $1",
    )
    .bind(payload.request_id)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(ApiResponse::success(serde_json::json!({
        "id": row.0,
        "status": row.1,
        "expires_at": row.2,
        "approver_id": row.3
    }))))
}

async fn list_my_active(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<impl axum::response::IntoResponse> {
    let user_id = guard::current_user_id(&headers, &state.config.jwt.secret)?;

    let rows = sqlx::query_as::<_, (i64, String, String, Option<String>)>(
        "SELECT id, reason, status, expires_at::text
         FROM jit_request
         WHERE requester_id = $1
           AND status = 'approved'
           AND expires_at IS NOT NULL
           AND expires_at > NOW()
         ORDER BY id DESC",
    )
    .bind(user_id)
    .fetch_all(&state.db)
    .await?;

    let data: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|r| {
            serde_json::json!({
                "id": r.0,
                "reason": r.1,
                "status": r.2,
                "expires_at": r.3
            })
        })
        .collect();

    Ok(Json(ApiResponse::success(data)))
}

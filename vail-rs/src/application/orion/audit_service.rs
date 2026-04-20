use axum::http::HeaderMap;
use sqlx::PgPool;
use crate::error::AppResult;

pub async fn log_operator_action(
    db: &PgPool,
    headers: &HeaderMap,
    user_id: i64,
    module: &str,
    operation: &str,
    params: serde_json::Value,
    result: i16,
    error_message: Option<String>,
) -> AppResult<()> {
    let username = sqlx::query_scalar::<_, Option<String>>(
        "SELECT username FROM sys_user WHERE id = $1 AND deleted = 0",
    )
    .bind(user_id)
    .fetch_one(db)
    .await?
    .unwrap_or_else(|| user_id.to_string());

    let ip = headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(',').next())
        .map(|v| v.trim().to_string())
        .or_else(|| {
            headers
                .get("x-real-ip")
                .and_then(|v| v.to_str().ok())
                .map(|v| v.to_string())
        });

    let user_agent = headers
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .map(|v| v.to_string());

    sqlx::query(
        "INSERT INTO operator_log (
            user_id,
            username,
            module,
            operation,
            params,
            result,
            error_message,
            ip,
            user_agent,
            create_time
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, NOW())",
    )
    .bind(user_id)
    .bind(username)
    .bind(module)
    .bind(operation)
    .bind(params)
    .bind(result)
    .bind(error_message)
    .bind(ip)
    .bind(user_agent)
    .execute(db)
    .await?;

    Ok(())
}

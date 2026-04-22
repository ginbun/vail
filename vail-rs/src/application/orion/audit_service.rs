use axum::http::HeaderMap;
use sqlx::PgPool;
use crate::error::AppResult;

pub struct OperatorLogParams<'a> {
    pub user_id: i64,
    pub module: &'a str,
    pub operation: &'a str,
    pub params: serde_json::Value,
    pub result: i16,
    pub error_message: Option<String>,
}

pub async fn log_operator_action(
    db: &PgPool,
    headers: &HeaderMap,
    params: OperatorLogParams<'_>,
) -> AppResult<()> {
    let username = sqlx::query_scalar::<_, Option<String>>(
        "SELECT username FROM sys_user WHERE id = $1 AND deleted = 0",
    )
    .bind(params.user_id)
    .fetch_one(db)
    .await?
    .unwrap_or_else(|| params.user_id.to_string());

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
    .bind(params.user_id)
    .bind(username)
    .bind(params.module)
    .bind(params.operation)
    .bind(params.params)
    .bind(params.result)
    .bind(params.error_message)
    .bind(ip)
    .bind(user_agent)
    .execute(db)
    .await?;

    Ok(())
}

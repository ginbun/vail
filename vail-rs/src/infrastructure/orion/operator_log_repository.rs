use sqlx::PgPool;

use crate::domain::orion::operator_log::{
    OrionOperatorLogAggregate, OrionOperatorLogClearFilters, OrionOperatorLogFilters,
};

#[derive(Debug, sqlx::FromRow)]
struct OrionOperatorLogRow {
    id: i64,
    user_id: Option<i64>,
    username: Option<String>,
    module: Option<String>,
    operation: Option<String>,
    method: Option<String>,
    path: Option<String>,
    params: Option<serde_json::Value>,
    trace_id: Option<String>,
    ip: Option<String>,
    user_agent: Option<String>,
    result: Option<i16>,
    error_message: Option<String>,
    duration: Option<i32>,
    create_time: i64,
}

pub async fn query_operator_logs(
    pool: &PgPool,
    filters: &OrionOperatorLogFilters,
) -> Result<Vec<OrionOperatorLogAggregate>, sqlx::Error> {
    let rows = sqlx::query_as::<_, OrionOperatorLogRow>(
        "SELECT id,
                user_id,
                username,
                module,
                operation,
                method,
                path,
                params,
                trace_id,
                ip,
                user_agent,
                result,
                error_message,
                duration,
                EXTRACT(EPOCH FROM create_time)::bigint * 1000 AS create_time
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
    .bind(filters.scope_user_id)
    .bind(filters.user_id)
    .bind(filters.username.as_deref())
    .bind(filters.module.as_deref())
    .bind(filters.operation.as_deref())
    .bind(filters.risk_level.as_deref())
    .bind(filters.result)
    .bind(filters.start_time)
    .bind(filters.end_time)
    .bind(filters.limit)
    .bind(filters.offset)
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(Into::into).collect())
}

pub async fn count_operator_logs(
    pool: &PgPool,
    filters: &OrionOperatorLogFilters,
) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar::<_, i64>(
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
    .bind(filters.scope_user_id)
    .bind(filters.user_id)
    .bind(filters.username.as_deref())
    .bind(filters.module.as_deref())
    .bind(filters.operation.as_deref())
    .bind(filters.risk_level.as_deref())
    .bind(filters.result)
    .bind(filters.start_time)
    .bind(filters.end_time)
    .fetch_one(pool)
    .await
}

pub async fn soft_delete_operator_logs(pool: &PgPool, ids: &[i64]) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE operator_log
         SET deleted = 1
         WHERE deleted = 0
           AND id = ANY($1::bigint[])",
    )
    .bind(ids)
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
}

pub async fn clear_operator_logs(
    pool: &PgPool,
    filters: &OrionOperatorLogClearFilters,
) -> Result<u64, sqlx::Error> {
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
    .bind(filters.user_id)
    .bind(filters.username.as_deref())
    .bind(filters.module.as_deref())
    .bind(filters.operation.as_deref())
    .bind(filters.risk_level.as_deref())
    .bind(filters.result)
    .bind(filters.start_time)
    .bind(filters.end_time)
    .bind(filters.limit)
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
}

impl From<OrionOperatorLogRow> for OrionOperatorLogAggregate {
    fn from(value: OrionOperatorLogRow) -> Self {
        Self {
            id: value.id,
            user_id: value.user_id,
            username: value.username,
            module: value.module,
            operation: value.operation,
            method: value.method,
            path: value.path,
            params: value.params,
            trace_id: value.trace_id,
            ip: value.ip,
            user_agent: value.user_agent,
            result: value.result,
            error_message: value.error_message,
            duration: value.duration,
            create_time: value.create_time,
        }
    }
}

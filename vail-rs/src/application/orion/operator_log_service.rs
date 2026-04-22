use chrono::{DateTime, Utc};
use sqlx::PgPool;

use crate::{
    domain::orion::operator_log::{
        OrionOperatorLogAggregate, OrionOperatorLogClearFilters, OrionOperatorLogFilters,
    },
    infrastructure::orion::operator_log_repository,
};

#[derive(Debug, Clone, Default)]
pub struct OrionOperatorLogQueryInput {
    pub scope_user_id: Option<i64>,
    pub user_id: Option<i64>,
    pub username: Option<String>,
    pub module: Option<String>,
    pub operation: Option<String>,
    pub risk_level: Option<String>,
    pub result: Option<i16>,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub limit: i64,
    pub offset: i64,
}

#[derive(Debug, Clone)]
pub struct OrionOperatorLogClearInput {
    pub user_id: Option<i64>,
    pub username: Option<String>,
    pub module: Option<String>,
    pub operation: Option<String>,
    pub risk_level: Option<String>,
    pub result: Option<i16>,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub limit: i64,
}

pub async fn query_operator_logs(
    pool: &PgPool,
    input: OrionOperatorLogQueryInput,
) -> Result<Vec<OrionOperatorLogAggregate>, sqlx::Error> {
    operator_log_repository::query_operator_logs(pool, &to_domain_filters(input)).await
}

pub async fn count_operator_logs(
    pool: &PgPool,
    input: OrionOperatorLogQueryInput,
) -> Result<i64, sqlx::Error> {
    operator_log_repository::count_operator_logs(pool, &to_domain_filters(input)).await
}

pub async fn soft_delete_operator_logs(pool: &PgPool, ids: Vec<i64>) -> Result<u64, sqlx::Error> {
    let ids = normalize_ids(ids);
    if ids.is_empty() {
        return Ok(0);
    }
    operator_log_repository::soft_delete_operator_logs(pool, &ids).await
}

pub async fn clear_operator_logs(
    pool: &PgPool,
    input: OrionOperatorLogClearInput,
) -> Result<u64, sqlx::Error> {
    operator_log_repository::clear_operator_logs(
        pool,
        &OrionOperatorLogClearFilters {
            user_id: input.user_id,
            username: input.username,
            module: input.module,
            operation: input.operation,
            risk_level: input.risk_level,
            result: input.result,
            start_time: input.start_time,
            end_time: input.end_time,
            limit: input.limit,
        },
    )
    .await
}

fn to_domain_filters(input: OrionOperatorLogQueryInput) -> OrionOperatorLogFilters {
    OrionOperatorLogFilters {
        scope_user_id: input.scope_user_id,
        user_id: input.user_id,
        username: input.username,
        module: input.module,
        operation: input.operation,
        risk_level: input.risk_level,
        result: input.result,
        start_time: input.start_time,
        end_time: input.end_time,
        limit: input.limit,
        offset: input.offset,
    }
}

fn normalize_ids(mut ids: Vec<i64>) -> Vec<i64> {
    ids.retain(|v| *v > 0);
    ids.sort_unstable();
    ids.dedup();
    ids
}

#[cfg(test)]
mod tests {
    use super::normalize_ids;

    #[test]
    fn normalize_ids_drops_invalid_and_duplicates() {
        let ids = normalize_ids(vec![0, -1, 4, 4, 3]);
        assert_eq!(ids, vec![3, 4]);
    }
}

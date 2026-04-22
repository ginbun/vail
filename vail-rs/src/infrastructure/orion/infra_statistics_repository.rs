use sqlx::PgPool;

use crate::domain::orion::infra_statistics::{
    OrionChartPointAggregate, OrionInfraUserSummaryAggregate, OrionLoginHistoryAggregate,
};

#[derive(Debug, sqlx::FromRow)]
struct OrionInfraUserSummaryRow {
    username: String,
    nickname: Option<String>,
    last_login_time: Option<i64>,
}

#[derive(Debug, sqlx::FromRow)]
struct OrionChartPointRow {
    label: String,
    count: i64,
}

#[derive(Debug, sqlx::FromRow)]
struct OrionLoginHistoryRow {
    id: i64,
    address: Option<String>,
    location: Option<String>,
    user_agent: Option<String>,
    result: Option<i16>,
    error_message: Option<String>,
    create_time: i64,
}

pub async fn get_infra_user_summary(
    pool: &PgPool,
    user_id: i64,
) -> Result<Option<OrionInfraUserSummaryAggregate>, sqlx::Error> {
    let row = sqlx::query_as::<_, OrionInfraUserSummaryRow>(
        "SELECT username,
                nickname,
                (EXTRACT(EPOCH FROM last_login_time) * 1000)::BIGINT AS last_login_time
         FROM sys_user
         WHERE id = $1 AND deleted = 0",
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(Into::into))
}

pub async fn count_active_ssh_sessions(pool: &PgPool, user_id: i64) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*)::BIGINT FROM ssh_session WHERE user_id = $1 AND end_time IS NULL",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await
}

pub async fn list_operator_chart_points(
    pool: &PgPool,
    user_id: i64,
) -> Result<Vec<OrionChartPointAggregate>, sqlx::Error> {
    let rows = sqlx::query_as::<_, OrionChartPointRow>(
        "SELECT to_char(day_list.day, 'MM-DD') AS label,
                COALESCE(day_count.cnt, 0)::bigint AS count
         FROM generate_series(
                (date_trunc('day', NOW()) - INTERVAL '6 day')::timestamp,
                date_trunc('day', NOW())::timestamp,
                INTERVAL '1 day'
              ) AS day_list(day)
         LEFT JOIN (
            SELECT date_trunc('day', create_time) AS day, COUNT(1) AS cnt
            FROM operator_log
            WHERE user_id = $1 AND deleted = 0
            GROUP BY date_trunc('day', create_time)
         ) AS day_count
           ON day_count.day = day_list.day
         ORDER BY day_list.day ASC",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(Into::into).collect())
}

pub async fn list_login_history(
    pool: &PgPool,
    user_id: i64,
    limit: i64,
) -> Result<Vec<OrionLoginHistoryAggregate>, sqlx::Error> {
    let rows = sqlx::query_as::<_, OrionLoginHistoryRow>(
        "SELECT id,
                ip AS address,
                location,
                user_agent,
                result,
                error_message,
                COALESCE((EXTRACT(EPOCH FROM create_time) * 1000)::BIGINT, 0) AS create_time
         FROM login_log
         WHERE user_id = $1
         ORDER BY create_time DESC
         LIMIT $2",
    )
    .bind(user_id)
    .bind(limit)
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(Into::into).collect())
}

impl From<OrionInfraUserSummaryRow> for OrionInfraUserSummaryAggregate {
    fn from(value: OrionInfraUserSummaryRow) -> Self {
        Self {
            username: value.username,
            nickname: value.nickname,
            last_login_time: value.last_login_time,
        }
    }
}

impl From<OrionChartPointRow> for OrionChartPointAggregate {
    fn from(value: OrionChartPointRow) -> Self {
        Self {
            label: value.label,
            count: value.count,
        }
    }
}

impl From<OrionLoginHistoryRow> for OrionLoginHistoryAggregate {
    fn from(value: OrionLoginHistoryRow) -> Self {
        Self {
            id: value.id,
            address: value.address,
            location: value.location,
            user_agent: value.user_agent,
            result: value.result,
            error_message: value.error_message,
            create_time: value.create_time,
        }
    }
}

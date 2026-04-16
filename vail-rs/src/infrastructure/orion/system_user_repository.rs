use sqlx::PgPool;

use crate::domain::orion::system_user::{OrionSystemUserAggregate, OrionSystemUserRow};

#[derive(Debug, Clone, Default)]
pub struct OrionSystemUserQueryFilters {
    pub id: Option<i64>,
    pub username: Option<String>,
    pub nickname: Option<String>,
    pub mobile: Option<String>,
    pub email: Option<String>,
    pub status: Option<i16>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

pub async fn get_system_user_by_id(
    pool: &PgPool,
    user_id: i64,
) -> Result<Option<OrionSystemUserAggregate>, sqlx::Error> {
    let row = sqlx::query_as::<_, OrionSystemUserRow>(
        "SELECT id, username, nickname, avatar, phone, email, status,
            CASE WHEN last_login_time IS NULL THEN NULL ELSE EXTRACT(EPOCH FROM last_login_time)::bigint * 1000 END,
            EXTRACT(EPOCH FROM create_time)::bigint * 1000,
            EXTRACT(EPOCH FROM update_time)::bigint * 1000
         FROM sys_user WHERE id = $1 AND deleted = 0",
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(Into::into))
}

pub async fn list_system_users(
    pool: &PgPool,
) -> Result<Vec<OrionSystemUserAggregate>, sqlx::Error> {
    let rows = sqlx::query_as::<_, OrionSystemUserRow>(
        "SELECT id, username, nickname, avatar, phone, email, status,
            CASE WHEN last_login_time IS NULL THEN NULL ELSE EXTRACT(EPOCH FROM last_login_time)::bigint * 1000 END,
            EXTRACT(EPOCH FROM create_time)::bigint * 1000,
            EXTRACT(EPOCH FROM update_time)::bigint * 1000
         FROM sys_user WHERE deleted = 0 ORDER BY id ASC",
    )
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(Into::into).collect())
}

pub async fn query_system_users(
    pool: &PgPool,
    filters: &OrionSystemUserQueryFilters,
) -> Result<Vec<OrionSystemUserAggregate>, sqlx::Error> {
    let rows = sqlx::query_as::<_, OrionSystemUserRow>(
        "SELECT id, username, nickname, avatar, phone, email, status,
            CASE WHEN last_login_time IS NULL THEN NULL ELSE EXTRACT(EPOCH FROM last_login_time)::bigint * 1000 END,
            EXTRACT(EPOCH FROM create_time)::bigint * 1000,
            EXTRACT(EPOCH FROM update_time)::bigint * 1000
         FROM sys_user
         WHERE deleted = 0
           AND ($1::bigint IS NULL OR id = $1)
           AND ($2::text IS NULL OR username ILIKE '%' || $2 || '%')
           AND ($3::text IS NULL OR nickname ILIKE '%' || $3 || '%')
           AND ($4::text IS NULL OR phone ILIKE '%' || $4 || '%')
           AND ($5::text IS NULL OR email ILIKE '%' || $5 || '%')
           AND ($6::smallint IS NULL OR status = $6)
         ORDER BY id DESC LIMIT $7 OFFSET $8",
    )
    .bind(filters.id)
    .bind(filters.username.as_ref())
    .bind(filters.nickname.as_ref())
    .bind(filters.mobile.as_ref())
    .bind(filters.email.as_ref())
    .bind(filters.status)
    .bind(filters.limit.unwrap_or(20))
    .bind(filters.offset.unwrap_or(0))
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(Into::into).collect())
}

pub async fn count_system_users(
    pool: &PgPool,
    filters: &OrionSystemUserQueryFilters,
) -> Result<i64, sqlx::Error> {
    let total = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(1) FROM sys_user
         WHERE deleted = 0
           AND ($1::bigint IS NULL OR id = $1)
           AND ($2::text IS NULL OR username ILIKE '%' || $2 || '%')
           AND ($3::text IS NULL OR nickname ILIKE '%' || $3 || '%')
           AND ($4::text IS NULL OR phone ILIKE '%' || $4 || '%')
           AND ($5::text IS NULL OR email ILIKE '%' || $5 || '%')
           AND ($6::smallint IS NULL OR status = $6)",
    )
    .bind(filters.id)
    .bind(filters.username.as_ref())
    .bind(filters.nickname.as_ref())
    .bind(filters.mobile.as_ref())
    .bind(filters.email.as_ref())
    .bind(filters.status)
    .fetch_one(pool)
    .await?;

    Ok(total)
}

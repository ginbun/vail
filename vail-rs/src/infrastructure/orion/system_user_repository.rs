use sqlx::PgPool;

use crate::domain::orion::system_user::OrionSystemUserAggregate;

#[derive(sqlx::FromRow)]
struct OrionSystemUserRow {
    id: i64,
    username: String,
    nickname: Option<String>,
    avatar: Option<String>,
    mobile: Option<String>,
    email: Option<String>,
    status: i16,
    last_login_time: Option<i64>,
    create_time_ms: i64,
    update_time_ms: i64,
}

#[derive(Debug, sqlx::FromRow)]
pub struct OrionSystemUserLoginHistoryRow {
    pub id: i64,
    pub address: Option<String>,
    pub location: Option<String>,
    pub user_agent: Option<String>,
    pub result: i16,
    pub error_message: Option<String>,
    pub create_time: i64,
}

#[derive(Debug, sqlx::FromRow)]
pub struct OrionActiveSessionRow {
    pub id: i64,
    pub user_id: i64,
    pub login_time: i64,
    pub address: Option<String>,
    pub location: Option<String>,
    pub user_agent: Option<String>,
    pub username: String,
}

#[derive(Debug, sqlx::FromRow)]
pub struct OrionMineSessionRow {
    pub id: i64,
    pub session_id: String,
    pub revoked_at: Option<String>,
    pub login_time: i64,
    pub address: Option<String>,
    pub location: Option<String>,
    pub user_agent: Option<String>,
}

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
        "SELECT id, username, nickname, avatar, phone AS mobile, email, status,
            CASE WHEN last_login_time IS NULL THEN NULL ELSE EXTRACT(EPOCH FROM last_login_time)::bigint * 1000 END AS last_login_time,
            EXTRACT(EPOCH FROM create_time)::bigint * 1000 AS create_time_ms,
            EXTRACT(EPOCH FROM update_time)::bigint * 1000 AS update_time_ms
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
        "SELECT id, username, nickname, avatar, phone AS mobile, email, status,
            CASE WHEN last_login_time IS NULL THEN NULL ELSE EXTRACT(EPOCH FROM last_login_time)::bigint * 1000 END AS last_login_time,
            EXTRACT(EPOCH FROM create_time)::bigint * 1000 AS create_time_ms,
            EXTRACT(EPOCH FROM update_time)::bigint * 1000 AS update_time_ms
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
        "SELECT id, username, nickname, avatar, phone AS mobile, email, status,
            CASE WHEN last_login_time IS NULL THEN NULL ELSE EXTRACT(EPOCH FROM last_login_time)::bigint * 1000 END AS last_login_time,
            EXTRACT(EPOCH FROM create_time)::bigint * 1000 AS create_time_ms,
            EXTRACT(EPOCH FROM update_time)::bigint * 1000 AS update_time_ms
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

pub async fn soft_delete_system_user(pool: &PgPool, id: i64) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE sys_user SET deleted = 1, update_time = NOW() WHERE id = $1 AND deleted = 0",
    )
    .bind(id)
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
}

pub async fn soft_delete_system_users(pool: &PgPool, ids: &[i64]) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE sys_user SET deleted = 1, update_time = NOW() WHERE id = ANY($1::bigint[]) AND deleted = 0",
    )
    .bind(ids)
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
}

pub async fn list_system_user_role_ids(
    pool: &PgPool,
    user_id: i64,
) -> Result<Vec<i64>, sqlx::Error> {
    sqlx::query_scalar::<_, i64>(
        "SELECT role_id FROM sys_user_role WHERE user_id = $1 ORDER BY role_id ASC",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
}

pub async fn list_login_history_by_username(
    pool: &PgPool,
    username: &str,
    limit: i64,
) -> Result<Vec<OrionSystemUserLoginHistoryRow>, sqlx::Error> {
    sqlx::query_as::<_, OrionSystemUserLoginHistoryRow>(
        "SELECT id,
                ip AS address,
                location,
                user_agent,
                result,
                error_message,
                EXTRACT(EPOCH FROM create_time)::bigint * 1000 AS create_time
         FROM login_log
         WHERE username = $1
         ORDER BY create_time DESC
         LIMIT $2",
    )
    .bind(username)
    .bind(limit)
    .fetch_all(pool)
    .await
}

pub async fn list_login_history_by_user_id(
    pool: &PgPool,
    user_id: i64,
    limit: i64,
) -> Result<Vec<OrionSystemUserLoginHistoryRow>, sqlx::Error> {
    sqlx::query_as::<_, OrionSystemUserLoginHistoryRow>(
        "SELECT id,
                ip AS address,
                location,
                user_agent,
                result,
                error_message,
                EXTRACT(EPOCH FROM create_time)::bigint * 1000 AS create_time
         FROM login_log
         WHERE user_id = $1
         ORDER BY create_time DESC
         LIMIT $2",
    )
    .bind(user_id)
    .bind(limit)
    .fetch_all(pool)
    .await
}

pub async fn revoke_refresh_tokens_by_user_and_timestamp(
    pool: &PgPool,
    user_id: i64,
    timestamp: i64,
) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE auth_refresh_token SET revoked_at = NOW() WHERE user_id = $1 AND EXTRACT(EPOCH FROM created_at)::bigint * 1000 = $2",
    )
    .bind(user_id)
    .bind(timestamp)
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
}

pub async fn list_active_sessions(
    pool: &PgPool,
    limit: i64,
) -> Result<Vec<OrionActiveSessionRow>, sqlx::Error> {
    sqlx::query_as::<_, OrionActiveSessionRow>(
        "SELECT r.id,
                r.user_id,
                EXTRACT(EPOCH FROM r.created_at)::bigint * 1000 AS login_time,
                r.ip AS address,
                r.location,
                r.user_agent,
                u.username
         FROM auth_refresh_token r
         JOIN sys_user u ON u.id = r.user_id
         WHERE r.revoked_at IS NULL
         ORDER BY r.created_at DESC
         LIMIT $1",
    )
    .bind(limit)
    .fetch_all(pool)
    .await
}

pub async fn list_active_sessions_by_user(
    pool: &PgPool,
    user_id: i64,
    limit: i64,
) -> Result<Vec<OrionActiveSessionRow>, sqlx::Error> {
    sqlx::query_as::<_, OrionActiveSessionRow>(
        "SELECT r.id,
                r.user_id,
                EXTRACT(EPOCH FROM r.created_at)::bigint * 1000 AS login_time,
                r.ip AS address,
                r.location,
                r.user_agent,
                u.username
         FROM auth_refresh_token r
         JOIN sys_user u ON u.id = r.user_id
         WHERE r.user_id = $1
         ORDER BY r.created_at DESC
         LIMIT $2",
    )
    .bind(user_id)
    .bind(limit)
    .fetch_all(pool)
    .await
}

pub async fn list_mine_sessions(
    pool: &PgPool,
    user_id: i64,
    limit: i64,
) -> Result<Vec<OrionMineSessionRow>, sqlx::Error> {
    sqlx::query_as::<_, OrionMineSessionRow>(
        "SELECT id,
                session_id::text AS session_id,
                NULLIF(revoked_at::text, '') AS revoked_at,
                EXTRACT(EPOCH FROM created_at)::bigint * 1000 AS login_time,
                ip AS address,
                location,
                user_agent
         FROM auth_refresh_token
         WHERE user_id = $1
         ORDER BY created_at DESC
         LIMIT $2",
    )
    .bind(user_id)
    .bind(limit)
    .fetch_all(pool)
    .await
}

pub async fn revoke_active_refresh_tokens_by_user_and_timestamp(
    pool: &PgPool,
    user_id: i64,
    timestamp: i64,
) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE auth_refresh_token
         SET revoked_at = NOW()
         WHERE user_id = $1
           AND EXTRACT(EPOCH FROM created_at)::bigint * 1000 = $2
           AND revoked_at IS NULL",
    )
    .bind(user_id)
    .bind(timestamp)
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
}

pub async fn create_system_user(
    pool: &PgPool,
    username: &str,
    password_hash: &str,
    nickname: Option<&str>,
    avatar: Option<&str>,
    mobile: Option<&str>,
    email: Option<&str>,
) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar::<_, i64>(
        "INSERT INTO sys_user (username, password, nickname, avatar, phone, email, status, create_time, update_time, deleted)
         VALUES ($1, $2, $3, $4, $5, $6, 1, NOW(), NOW(), 0)
         RETURNING id",
    )
    .bind(username)
    .bind(password_hash)
    .bind(nickname)
    .bind(avatar)
    .bind(mobile)
    .bind(email)
    .fetch_one(pool)
    .await
}

pub async fn update_system_user(
    pool: &PgPool,
    id: i64,
    username: Option<&str>,
    nickname: Option<&str>,
    avatar: Option<&str>,
    mobile: Option<&str>,
    email: Option<&str>,
) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE sys_user SET
            username = COALESCE(NULLIF($1, ''), username),
            nickname = COALESCE($2, nickname),
            avatar = COALESCE($3, avatar),
            phone = COALESCE($4, phone),
            email = COALESCE($5, email),
            update_time = NOW()
         WHERE id = $6 AND deleted = 0",
    )
    .bind(username)
    .bind(nickname)
    .bind(avatar)
    .bind(mobile)
    .bind(email)
    .bind(id)
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
}

pub async fn update_system_user_status(
    pool: &PgPool,
    id: i64,
    status: i16,
) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE sys_user SET status = $1, update_time = NOW() WHERE id = $2 AND deleted = 0",
    )
    .bind(status)
    .bind(id)
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
}

pub async fn replace_system_user_roles(
    pool: &PgPool,
    user_id: i64,
    role_ids: &[i64],
) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;
    sqlx::query("DELETE FROM sys_user_role WHERE user_id = $1")
        .bind(user_id)
        .execute(&mut *tx)
        .await?;
    for role_id in role_ids {
        sqlx::query(
            "INSERT INTO sys_user_role (user_id, role_id, create_time) VALUES ($1, $2, NOW())",
        )
        .bind(user_id)
        .bind(role_id)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await
}

pub async fn update_system_user_password(
    pool: &PgPool,
    id: i64,
    password_hash: &str,
) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE sys_user SET password = $1, update_time = NOW() WHERE id = $2 AND deleted = 0",
    )
    .bind(password_hash)
    .bind(id)
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
}

pub async fn get_system_user_password_hash(
    pool: &PgPool,
    user_id: i64,
) -> Result<Option<String>, sqlx::Error> {
    sqlx::query_scalar::<_, String>("SELECT password FROM sys_user WHERE id = $1 AND deleted = 0")
        .bind(user_id)
        .fetch_optional(pool)
        .await
}

impl From<OrionSystemUserRow> for OrionSystemUserAggregate {
    fn from(value: OrionSystemUserRow) -> Self {
        Self {
            id: value.id,
            username: value.username,
            nickname: value.nickname,
            avatar: value.avatar,
            mobile: value.mobile,
            email: value.email,
            status: value.status,
            last_login_time: value.last_login_time,
            create_time_ms: value.create_time_ms,
            update_time_ms: value.update_time_ms,
        }
    }
}

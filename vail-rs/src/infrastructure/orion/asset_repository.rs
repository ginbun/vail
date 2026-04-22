use sqlx::PgPool;

use crate::domain::orion::asset::{
    OrionGrantScope, OrionHostIdentityAggregate, OrionHostKeyAggregate,
};
use crate::error::AppResult;

#[derive(Debug, sqlx::FromRow)]
struct OrionHostKeyRow {
    id: i64,
    name: String,
    description: Option<String>,
    create_time_ms: i64,
    update_time_ms: i64,
}

#[derive(Debug, sqlx::FromRow)]
struct OrionHostIdentityRow {
    id: i64,
    name: String,
    identity_type: String,
    username: Option<String>,
    key_id: Option<i64>,
    description: Option<String>,
    create_time_ms: i64,
    update_time_ms: i64,
}

#[derive(Debug, Default, Clone)]
pub struct OrionHostKeyQueryFilters {
    pub id: Option<i64>,
    pub search_value: Option<String>,
    pub name: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Default, Clone)]
pub struct OrionHostIdentityQueryFilters {
    pub id: Option<i64>,
    pub search_value: Option<String>,
    pub name: Option<String>,
    pub identity_type: Option<String>,
    pub username: Option<String>,
    pub key_id: Option<i64>,
    pub description: Option<String>,
}

pub async fn get_host_key_by_id(
    pool: &PgPool,
    id: i64,
) -> AppResult<Option<OrionHostKeyAggregate>> {
    let row = sqlx::query_as::<_, OrionHostKeyRow>(
        "SELECT
            id,
            name,
            description,
            COALESCE((EXTRACT(EPOCH FROM create_time) * 1000)::BIGINT, 0) AS create_time_ms,
            COALESCE((EXTRACT(EPOCH FROM update_time) * 1000)::BIGINT, 0) AS update_time_ms
         FROM ssh_key
         WHERE id = $1 AND deleted = 0",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(Into::into))
}

pub async fn list_host_keys(pool: &PgPool) -> AppResult<Vec<OrionHostKeyAggregate>> {
    let rows = sqlx::query_as::<_, OrionHostKeyRow>(
        "SELECT
            id,
            name,
            description,
            COALESCE((EXTRACT(EPOCH FROM create_time) * 1000)::BIGINT, 0) AS create_time_ms,
            COALESCE((EXTRACT(EPOCH FROM update_time) * 1000)::BIGINT, 0) AS update_time_ms
         FROM ssh_key
         WHERE deleted = 0
         ORDER BY id DESC",
    )
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(Into::into).collect())
}

pub async fn count_host_keys(pool: &PgPool, f: &OrionHostKeyQueryFilters) -> AppResult<i64> {
    let total = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(1)::BIGINT
         FROM ssh_key
         WHERE deleted = 0
           AND ($1::BIGINT IS NULL OR id = $1)
           AND ($2::text IS NULL OR name ILIKE CONCAT('%', $2, '%'))
           AND ($3::text IS NULL OR description ILIKE CONCAT('%', $3, '%'))
           AND (
               $4::text IS NULL
               OR name ILIKE CONCAT('%', $4, '%')
               OR description ILIKE CONCAT('%', $4, '%')
           )",
    )
    .bind(f.id)
    .bind(f.name.as_deref())
    .bind(f.description.as_deref())
    .bind(f.search_value.as_deref())
    .fetch_one(pool)
    .await?;
    Ok(total)
}

pub async fn query_host_keys(
    pool: &PgPool,
    f: &OrionHostKeyQueryFilters,
    offset: i64,
    limit: i64,
) -> AppResult<Vec<OrionHostKeyAggregate>> {
    let rows = sqlx::query_as::<_, OrionHostKeyRow>(
        "SELECT
            id,
            name,
            description,
            COALESCE((EXTRACT(EPOCH FROM create_time) * 1000)::BIGINT, 0) AS create_time_ms,
            COALESCE((EXTRACT(EPOCH FROM update_time) * 1000)::BIGINT, 0) AS update_time_ms
         FROM ssh_key
         WHERE deleted = 0
           AND ($1::BIGINT IS NULL OR id = $1)
           AND ($2::text IS NULL OR name ILIKE CONCAT('%', $2, '%'))
           AND ($3::text IS NULL OR description ILIKE CONCAT('%', $3, '%'))
           AND (
               $4::text IS NULL
               OR name ILIKE CONCAT('%', $4, '%')
               OR description ILIKE CONCAT('%', $4, '%')
           )
         ORDER BY id DESC
         OFFSET $5 LIMIT $6",
    )
    .bind(f.id)
    .bind(f.name.as_deref())
    .bind(f.description.as_deref())
    .bind(f.search_value.as_deref())
    .bind(offset)
    .bind(limit)
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(Into::into).collect())
}

pub async fn soft_delete_host_key(pool: &PgPool, id: i64) -> AppResult<u64> {
    let result = sqlx::query("UPDATE ssh_key SET deleted = 1, update_time = NOW() WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected())
}

pub async fn soft_delete_host_keys(pool: &PgPool, ids: &[i64]) -> AppResult<u64> {
    let result = sqlx::query(
        "UPDATE ssh_key SET deleted = 1, update_time = NOW() WHERE id = ANY($1::BIGINT[])",
    )
    .bind(ids)
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
}

pub async fn create_host_identity(
    pool: &PgPool,
    name: &str,
    identity_type: &str,
    username: Option<&str>,
    password_ciphertext: Option<&str>,
    key_id: Option<i64>,
    description: Option<&str>,
) -> AppResult<i64> {
    let id = sqlx::query_scalar::<_, i64>(
        "INSERT INTO host_identity (
            name,
            type,
            username,
            password_ciphertext,
            key_id,
            description,
            status,
            create_time,
            update_time,
            deleted
        ) VALUES ($1, $2, $3, $4, $5, $6, 1, NOW(), NOW(), 0)
         RETURNING id",
    )
    .bind(name)
    .bind(identity_type)
    .bind(username)
    .bind(password_ciphertext)
    .bind(key_id)
    .bind(description)
    .fetch_one(pool)
    .await?;
    Ok(id)
}

pub struct HostIdentityPatch<'a> {
    pub id: i64,
    pub name: Option<&'a str>,
    pub identity_type: Option<&'a str>,
    pub username: Option<&'a str>,
    pub key_id: Option<Option<i64>>,
    pub description: Option<&'a str>,
    pub use_new_password: bool,
    pub password_ciphertext: Option<Option<&'a str>>,
}

pub async fn update_host_identity(pool: &PgPool, patch: HostIdentityPatch<'_>) -> AppResult<u64> {
    let result = sqlx::query(
        "UPDATE host_identity SET
            name = COALESCE($2, name),
            type = COALESCE($3, type),
            username = COALESCE($4, username),
            key_id = COALESCE($5, key_id),
            description = COALESCE($6, description),
            password_ciphertext = CASE WHEN $7 THEN $8 ELSE password_ciphertext END,
            update_time = NOW()
         WHERE id = $1 AND deleted = 0",
    )
    .bind(patch.id)
    .bind(patch.name)
    .bind(patch.identity_type)
    .bind(patch.username)
    .bind(patch.key_id.flatten())
    .bind(patch.description)
    .bind(patch.use_new_password)
    .bind(patch.password_ciphertext.flatten())
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
}

pub async fn get_host_identity_by_id(
    pool: &PgPool,
    id: i64,
) -> AppResult<Option<OrionHostIdentityAggregate>> {
    let row = sqlx::query_as::<_, OrionHostIdentityRow>(
        "SELECT
            id,
            name,
            type AS identity_type,
            username,
            key_id,
            description,
            COALESCE((EXTRACT(EPOCH FROM create_time) * 1000)::BIGINT, 0) AS create_time_ms,
            COALESCE((EXTRACT(EPOCH FROM update_time) * 1000)::BIGINT, 0) AS update_time_ms
         FROM host_identity
         WHERE id = $1 AND deleted = 0",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(Into::into))
}

pub async fn list_host_identities(pool: &PgPool) -> AppResult<Vec<OrionHostIdentityAggregate>> {
    let rows = sqlx::query_as::<_, OrionHostIdentityRow>(
        "SELECT
            id,
            name,
            type AS identity_type,
            username,
            key_id,
            description,
            COALESCE((EXTRACT(EPOCH FROM create_time) * 1000)::BIGINT, 0) AS create_time_ms,
            COALESCE((EXTRACT(EPOCH FROM update_time) * 1000)::BIGINT, 0) AS update_time_ms
         FROM host_identity
         WHERE deleted = 0
         ORDER BY id DESC",
    )
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(Into::into).collect())
}

pub async fn count_host_identities(
    pool: &PgPool,
    f: &OrionHostIdentityQueryFilters,
) -> AppResult<i64> {
    let total = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(1)::BIGINT
         FROM host_identity
         WHERE deleted = 0
           AND ($1::BIGINT IS NULL OR id = $1)
           AND ($2::text IS NULL OR name ILIKE CONCAT('%', $2, '%'))
           AND ($3::text IS NULL OR type = $3)
           AND ($4::text IS NULL OR username ILIKE CONCAT('%', $4, '%'))
           AND ($5::BIGINT IS NULL OR key_id = $5)
           AND ($6::text IS NULL OR description ILIKE CONCAT('%', $6, '%'))
           AND (
               $7::text IS NULL
               OR name ILIKE CONCAT('%', $7, '%')
               OR username ILIKE CONCAT('%', $7, '%')
               OR description ILIKE CONCAT('%', $7, '%')
           )",
    )
    .bind(f.id)
    .bind(f.name.as_deref())
    .bind(f.identity_type.as_deref())
    .bind(f.username.as_deref())
    .bind(f.key_id)
    .bind(f.description.as_deref())
    .bind(f.search_value.as_deref())
    .fetch_one(pool)
    .await?;
    Ok(total)
}

pub async fn query_host_identities(
    pool: &PgPool,
    f: &OrionHostIdentityQueryFilters,
    offset: i64,
    limit: i64,
) -> AppResult<Vec<OrionHostIdentityAggregate>> {
    let rows = sqlx::query_as::<_, OrionHostIdentityRow>(
        "SELECT
            id,
            name,
            type AS identity_type,
            username,
            key_id,
            description,
            COALESCE((EXTRACT(EPOCH FROM create_time) * 1000)::BIGINT, 0) AS create_time_ms,
            COALESCE((EXTRACT(EPOCH FROM update_time) * 1000)::BIGINT, 0) AS update_time_ms
         FROM host_identity
         WHERE deleted = 0
           AND ($1::BIGINT IS NULL OR id = $1)
           AND ($2::text IS NULL OR name ILIKE CONCAT('%', $2, '%'))
           AND ($3::text IS NULL OR type = $3)
           AND ($4::text IS NULL OR username ILIKE CONCAT('%', $4, '%'))
           AND ($5::BIGINT IS NULL OR key_id = $5)
           AND ($6::text IS NULL OR description ILIKE CONCAT('%', $6, '%'))
           AND (
               $7::text IS NULL
               OR name ILIKE CONCAT('%', $7, '%')
               OR username ILIKE CONCAT('%', $7, '%')
               OR description ILIKE CONCAT('%', $7, '%')
           )
         ORDER BY id DESC
         OFFSET $8 LIMIT $9",
    )
    .bind(f.id)
    .bind(f.name.as_deref())
    .bind(f.identity_type.as_deref())
    .bind(f.username.as_deref())
    .bind(f.key_id)
    .bind(f.description.as_deref())
    .bind(f.search_value.as_deref())
    .bind(offset)
    .bind(limit)
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(Into::into).collect())
}

pub async fn soft_delete_host_identity(pool: &PgPool, id: i64) -> AppResult<u64> {
    let result =
        sqlx::query("UPDATE host_identity SET deleted = 1, update_time = NOW() WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?;
    Ok(result.rows_affected())
}

pub async fn soft_delete_host_identities(pool: &PgPool, ids: &[i64]) -> AppResult<u64> {
    let result = sqlx::query(
        "UPDATE host_identity SET deleted = 1, update_time = NOW() WHERE id = ANY($1::BIGINT[])",
    )
    .bind(ids)
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
}

pub async fn replace_asset_grants(
    pool: &PgPool,
    scope: OrionGrantScope,
    resource: &str,
    ids: &[i64],
) -> AppResult<()> {
    let mut tx = pool.begin().await?;

    match (scope, resource) {
        (OrionGrantScope::Role(role_id), "host-group") => {
            sqlx::query("DELETE FROM role_host_group_grant WHERE role_id = $1")
                .bind(role_id)
                .execute(&mut *tx)
                .await?;
            for id in ids {
                sqlx::query(
                    "INSERT INTO role_host_group_grant (role_id, group_id, create_time)
                     VALUES ($1, $2, NOW())",
                )
                .bind(role_id)
                .bind(id)
                .execute(&mut *tx)
                .await?;
            }
        }
        (OrionGrantScope::Role(role_id), "host-key") => {
            sqlx::query("DELETE FROM role_host_key_grant WHERE role_id = $1")
                .bind(role_id)
                .execute(&mut *tx)
                .await?;
            for id in ids {
                sqlx::query(
                    "INSERT INTO role_host_key_grant (role_id, key_id, create_time)
                     VALUES ($1, $2, NOW())",
                )
                .bind(role_id)
                .bind(id)
                .execute(&mut *tx)
                .await?;
            }
        }
        (OrionGrantScope::Role(role_id), "host-identity") => {
            sqlx::query("DELETE FROM role_host_identity_grant WHERE role_id = $1")
                .bind(role_id)
                .execute(&mut *tx)
                .await?;
            for id in ids {
                sqlx::query(
                    "INSERT INTO role_host_identity_grant (role_id, identity_id, create_time)
                     VALUES ($1, $2, NOW())",
                )
                .bind(role_id)
                .bind(id)
                .execute(&mut *tx)
                .await?;
            }
        }
        (OrionGrantScope::User(user_id), "host-group") => {
            sqlx::query("DELETE FROM user_host_group_grant WHERE user_id = $1")
                .bind(user_id)
                .execute(&mut *tx)
                .await?;
            for id in ids {
                sqlx::query(
                    "INSERT INTO user_host_group_grant (user_id, group_id, create_time)
                     VALUES ($1, $2, NOW())",
                )
                .bind(user_id)
                .bind(id)
                .execute(&mut *tx)
                .await?;
            }
        }
        (OrionGrantScope::User(user_id), "host-key") => {
            sqlx::query("DELETE FROM user_host_key_grant WHERE user_id = $1")
                .bind(user_id)
                .execute(&mut *tx)
                .await?;
            for id in ids {
                sqlx::query(
                    "INSERT INTO user_host_key_grant (user_id, key_id, create_time)
                     VALUES ($1, $2, NOW())",
                )
                .bind(user_id)
                .bind(id)
                .execute(&mut *tx)
                .await?;
            }
        }
        (OrionGrantScope::User(user_id), "host-identity") => {
            sqlx::query("DELETE FROM user_host_identity_grant WHERE user_id = $1")
                .bind(user_id)
                .execute(&mut *tx)
                .await?;
            for id in ids {
                sqlx::query(
                    "INSERT INTO user_host_identity_grant (user_id, identity_id, create_time)
                     VALUES ($1, $2, NOW())",
                )
                .bind(user_id)
                .bind(id)
                .execute(&mut *tx)
                .await?;
            }
        }
        _ => {}
    }

    tx.commit().await?;
    Ok(())
}

pub async fn list_asset_grants(
    pool: &PgPool,
    scope: OrionGrantScope,
    resource: &str,
) -> AppResult<Vec<i64>> {
    let rows = match (scope, resource) {
        (OrionGrantScope::Role(role_id), "host-group") => {
            sqlx::query_scalar::<_, i64>(
                "SELECT group_id FROM role_host_group_grant WHERE role_id = $1 ORDER BY group_id",
            )
            .bind(role_id)
            .fetch_all(pool)
            .await?
        }
        (OrionGrantScope::Role(role_id), "host-key") => {
            sqlx::query_scalar::<_, i64>(
                "SELECT key_id FROM role_host_key_grant WHERE role_id = $1 ORDER BY key_id",
            )
            .bind(role_id)
            .fetch_all(pool)
            .await?
        }
        (OrionGrantScope::Role(role_id), "host-identity") => {
            sqlx::query_scalar::<_, i64>(
                "SELECT identity_id FROM role_host_identity_grant WHERE role_id = $1 ORDER BY identity_id",
            )
            .bind(role_id)
            .fetch_all(pool)
            .await?
        }
        (OrionGrantScope::User(user_id), "host-group") => {
            sqlx::query_scalar::<_, i64>(
                "SELECT group_id FROM user_host_group_grant WHERE user_id = $1 ORDER BY group_id",
            )
            .bind(user_id)
            .fetch_all(pool)
            .await?
        }
        (OrionGrantScope::User(user_id), "host-key") => {
            sqlx::query_scalar::<_, i64>(
                "SELECT key_id FROM user_host_key_grant WHERE user_id = $1 ORDER BY key_id",
            )
            .bind(user_id)
            .fetch_all(pool)
            .await?
        }
        (OrionGrantScope::User(user_id), "host-identity") => {
            sqlx::query_scalar::<_, i64>(
                "SELECT identity_id FROM user_host_identity_grant WHERE user_id = $1 ORDER BY identity_id",
            )
            .bind(user_id)
            .fetch_all(pool)
            .await?
        }
        _ => Vec::new(),
    };

    Ok(rows)
}

impl From<OrionHostKeyRow> for OrionHostKeyAggregate {
    fn from(value: OrionHostKeyRow) -> Self {
        Self {
            id: value.id,
            name: value.name,
            description: value.description,
            create_time_ms: value.create_time_ms,
            update_time_ms: value.update_time_ms,
        }
    }
}

impl From<OrionHostIdentityRow> for OrionHostIdentityAggregate {
    fn from(value: OrionHostIdentityRow) -> Self {
        Self {
            id: value.id,
            name: value.name,
            identity_type: value.identity_type,
            username: value.username,
            key_id: value.key_id,
            description: value.description,
            create_time_ms: value.create_time_ms,
            update_time_ms: value.update_time_ms,
        }
    }
}

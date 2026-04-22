use serde_json::Value;
use sqlx::{PgPool, Transaction};

use crate::domain::orion::asset::{
    OrionGrantScope, OrionHostGroupAggregate, OrionHostIdentityAggregate, OrionHostKeyAggregate,
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

#[derive(Debug, sqlx::FromRow)]
struct OrionHostGroupRow {
    id: i64,
    parent_id: i64,
    name: String,
}

#[derive(Debug, sqlx::FromRow)]
pub struct OrionHostSshConfigRow {
    pub username: Option<String>,
    pub port: i32,
    pub credential_type: Option<String>,
    pub key_id: Option<i64>,
    pub has_password: bool,
}

#[derive(Debug, sqlx::FromRow)]
pub struct OrionHostIdentityAuthRow {
    pub identity_type: String,
    pub username: Option<String>,
    pub password_ciphertext: Option<String>,
    pub key_id: Option<i64>,
}

#[derive(Debug, sqlx::FromRow)]
pub struct OrionAuthorizedHostRow {
    pub id: i64,
    pub name: String,
    pub hostname: String,
    pub port: i32,
    pub create_time_ms: i64,
    pub update_time_ms: i64,
    pub group_id: i64,
}

#[derive(Debug, sqlx::FromRow)]
pub struct OrionAuthorizedHostKeyRow {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub create_time_ms: i64,
    pub update_time_ms: i64,
}

#[derive(Debug, sqlx::FromRow)]
pub struct OrionAuthorizedHostIdentityRow {
    pub id: i64,
    pub name: String,
    pub identity_type: String,
    pub username: Option<String>,
    pub key_id: Option<i64>,
    pub description: Option<String>,
    pub create_time_ms: i64,
    pub update_time_ms: i64,
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

pub async fn host_exists(pool: &PgPool, host_id: i64) -> AppResult<bool> {
    let exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM host WHERE id = $1 AND deleted = 0)",
    )
    .bind(host_id)
    .fetch_one(pool)
    .await?;
    Ok(exists)
}

pub async fn load_cache_json_value(pool: &PgPool, cache_key: &str) -> AppResult<Option<Value>> {
    let raw = sqlx::query_scalar::<_, String>(
        "SELECT cache_value FROM cache
         WHERE cache_key = $1
           AND (expire_time IS NULL OR expire_time > NOW())",
    )
    .bind(cache_key)
    .fetch_optional(pool)
    .await?;
    Ok(raw.and_then(|v| serde_json::from_str::<Value>(&v).ok()))
}

pub async fn save_cache_json_value(pool: &PgPool, cache_key: &str, value: &Value) -> AppResult<()> {
    let payload = serde_json::to_string(value).unwrap_or_else(|_| "{}".to_string());
    sqlx::query(
        "INSERT INTO cache (cache_key, cache_value, expire_time, create_time)
         VALUES ($1, $2, NULL, NOW())
         ON CONFLICT (cache_key)
         DO UPDATE SET cache_value = EXCLUDED.cache_value, create_time = NOW()",
    )
    .bind(cache_key)
    .bind(payload)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_host_ssh_config(
    pool: &PgPool,
    host_id: i64,
) -> AppResult<Option<OrionHostSshConfigRow>> {
    sqlx::query_as::<_, OrionHostSshConfigRow>(
        "SELECT h.username,
                h.port,
                h.credential_type,
                (
                    SELECT hb.ssh_key_id
                    FROM host_ssh_key_binding hb
                    WHERE hb.host_id = h.id AND hb.is_default = 1
                    LIMIT 1
                ) AS key_id,
                CASE
                    WHEN h.credential_type IN ('password', 'private_key')
                         AND h.credential_data IS NOT NULL
                    THEN true
                    ELSE false
                END AS has_password
         FROM host h
         WHERE h.id = $1 AND h.deleted = 0",
    )
    .bind(host_id)
    .fetch_optional(pool)
    .await
    .map_err(Into::into)
}

pub async fn update_host_connection_settings_tx(
    tx: &mut Transaction<'_, sqlx::Postgres>,
    host_id: i64,
    username: Option<&str>,
    port: Option<i32>,
) -> AppResult<()> {
    sqlx::query(
        "UPDATE host
         SET username = COALESCE($1, username),
             port = CASE WHEN $2 IS NULL OR $2 <= 0 THEN port ELSE $2 END,
             update_time = NOW()
         WHERE id = $3 AND deleted = 0",
    )
    .bind(username)
    .bind(port)
    .bind(host_id)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

pub async fn ssh_key_is_active_tx(
    tx: &mut Transaction<'_, sqlx::Postgres>,
    key_id: i64,
) -> AppResult<bool> {
    let exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM ssh_key WHERE id = $1 AND deleted = 0 AND status = 1)",
    )
    .bind(key_id)
    .fetch_one(&mut **tx)
    .await?;
    Ok(exists)
}

pub async fn set_host_credential_to_ssh_key_tx(
    tx: &mut Transaction<'_, sqlx::Postgres>,
    host_id: i64,
) -> AppResult<()> {
    sqlx::query(
        "UPDATE host
         SET credential_type = 'ssh_key', credential_data = NULL, update_time = NOW()
         WHERE id = $1 AND deleted = 0",
    )
    .bind(host_id)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

pub async fn set_host_credential_to_password_tx(
    tx: &mut Transaction<'_, sqlx::Postgres>,
    host_id: i64,
    credential_data: Option<&str>,
) -> AppResult<()> {
    if let Some(data) = credential_data {
        sqlx::query(
            "UPDATE host
             SET credential_type = 'password', credential_data = $2, update_time = NOW()
             WHERE id = $1 AND deleted = 0",
        )
        .bind(host_id)
        .bind(data)
        .execute(&mut **tx)
        .await?;
    } else {
        sqlx::query(
            "UPDATE host
             SET credential_type = 'password', update_time = NOW()
             WHERE id = $1 AND deleted = 0",
        )
        .bind(host_id)
        .execute(&mut **tx)
        .await?;
    }
    Ok(())
}

pub async fn clear_default_host_ssh_key_binding_tx(
    tx: &mut Transaction<'_, sqlx::Postgres>,
    host_id: i64,
) -> AppResult<()> {
    sqlx::query("UPDATE host_ssh_key_binding SET is_default = 0 WHERE host_id = $1")
        .bind(host_id)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

pub async fn upsert_default_host_ssh_key_binding_tx(
    tx: &mut Transaction<'_, sqlx::Postgres>,
    host_id: i64,
    key_id: i64,
) -> AppResult<()> {
    sqlx::query(
        "INSERT INTO host_ssh_key_binding (host_id, ssh_key_id, is_default, create_time)
         VALUES ($1, $2, 1, NOW())
         ON CONFLICT (host_id, ssh_key_id)
         DO UPDATE SET is_default = 1",
    )
    .bind(host_id)
    .bind(key_id)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

pub async fn get_active_host_identity_tx(
    tx: &mut Transaction<'_, sqlx::Postgres>,
    identity_id: i64,
) -> AppResult<Option<OrionHostIdentityAuthRow>> {
    sqlx::query_as::<_, OrionHostIdentityAuthRow>(
        "SELECT type AS identity_type, username, password_ciphertext, key_id
         FROM host_identity
         WHERE id = $1 AND deleted = 0 AND status = 1",
    )
    .bind(identity_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(Into::into)
}

pub async fn update_host_username_tx(
    tx: &mut Transaction<'_, sqlx::Postgres>,
    host_id: i64,
    username: &str,
) -> AppResult<()> {
    sqlx::query("UPDATE host SET username = $2, update_time = NOW() WHERE id = $1 AND deleted = 0")
        .bind(host_id)
        .bind(username)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

pub async fn list_authorized_host_group_ids(pool: &PgPool, user_id: i64) -> AppResult<Vec<i64>> {
    sqlx::query_scalar::<_, i64>(
        "SELECT DISTINCT group_id FROM (
            SELECT group_id FROM user_host_group_grant WHERE user_id = $1
            UNION
            SELECT group_id FROM role_host_group_grant
            WHERE role_id IN (
                SELECT role_id FROM sys_user_role WHERE user_id = $1
            )
        ) AS authorized_groups",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(Into::into)
}

pub async fn list_authorized_hosts_by_group_ids(
    pool: &PgPool,
    group_ids: &[i64],
) -> AppResult<Vec<OrionAuthorizedHostRow>> {
    sqlx::query_as::<_, OrionAuthorizedHostRow>(
        "SELECT DISTINCT
            h.id,
            h.name,
            h.hostname,
            h.port,
            COALESCE((EXTRACT(EPOCH FROM h.create_time) * 1000)::BIGINT, 0) AS create_time_ms,
            COALESCE((EXTRACT(EPOCH FROM h.update_time) * 1000)::BIGINT, 0) AS update_time_ms,
            hgr.group_id
         FROM host h
         JOIN host_group_rel hgr ON h.id = hgr.host_id
         WHERE hgr.group_id = ANY($1::BIGINT[]) AND h.deleted = 0
         ORDER BY h.id, hgr.group_id",
    )
    .bind(group_ids)
    .fetch_all(pool)
    .await
    .map_err(Into::into)
}

pub async fn list_authorized_host_key_ids(pool: &PgPool, user_id: i64) -> AppResult<Vec<i64>> {
    sqlx::query_scalar::<_, i64>(
        "SELECT DISTINCT key_id FROM (
            SELECT key_id FROM user_host_key_grant WHERE user_id = $1
            UNION
            SELECT key_id FROM role_host_key_grant
            WHERE role_id IN (
                SELECT role_id FROM sys_user_role WHERE user_id = $1
            )
        ) AS authorized_keys",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(Into::into)
}

pub async fn list_authorized_host_keys_by_ids(
    pool: &PgPool,
    key_ids: &[i64],
) -> AppResult<Vec<OrionAuthorizedHostKeyRow>> {
    sqlx::query_as::<_, OrionAuthorizedHostKeyRow>(
        "SELECT
            id,
            name,
            description,
            COALESCE((EXTRACT(EPOCH FROM create_time) * 1000)::BIGINT, 0) AS create_time_ms,
            COALESCE((EXTRACT(EPOCH FROM update_time) * 1000)::BIGINT, 0) AS update_time_ms
         FROM ssh_key
         WHERE id = ANY($1::BIGINT[]) AND deleted = 0
         ORDER BY id",
    )
    .bind(key_ids)
    .fetch_all(pool)
    .await
    .map_err(Into::into)
}

pub async fn list_authorized_host_identity_ids(pool: &PgPool, user_id: i64) -> AppResult<Vec<i64>> {
    sqlx::query_scalar::<_, i64>(
        "SELECT DISTINCT identity_id FROM (
            SELECT identity_id FROM user_host_identity_grant WHERE user_id = $1
            UNION
            SELECT identity_id FROM role_host_identity_grant
            WHERE role_id IN (
                SELECT role_id FROM sys_user_role WHERE user_id = $1
            )
        ) AS authorized_identities",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(Into::into)
}

pub async fn list_authorized_host_identities_by_ids(
    pool: &PgPool,
    identity_ids: &[i64],
) -> AppResult<Vec<OrionAuthorizedHostIdentityRow>> {
    sqlx::query_as::<_, OrionAuthorizedHostIdentityRow>(
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
         WHERE id = ANY($1::BIGINT[]) AND deleted = 0
         ORDER BY id",
    )
    .bind(identity_ids)
    .fetch_all(pool)
    .await
    .map_err(Into::into)
}

pub async fn count_host_groups_by_ids(pool: &PgPool, ids: &[i64]) -> AppResult<i64> {
    let count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(1) FROM host_group WHERE deleted = 0 AND id = ANY($1::bigint[])",
    )
    .bind(ids)
    .fetch_one(pool)
    .await?;
    Ok(count)
}

pub async fn create_host_with_groups(
    pool: &PgPool,
    name: &str,
    hostname: &str,
    description: Option<&str>,
    group_ids: &[i64],
) -> AppResult<i64> {
    let mut tx = pool.begin().await?;

    let new_id = sqlx::query_scalar::<_, i64>(
        "INSERT INTO host (name, hostname, port, credential_type, description, status, create_time, update_time)
         VALUES ($1, $2, 22, NULL, $3, 1, NOW(), NOW())
         RETURNING id",
    )
    .bind(name)
    .bind(hostname)
    .bind(description)
    .fetch_one(&mut *tx)
    .await?;

    for group_id in group_ids {
        sqlx::query(
            "INSERT INTO host_group_rel (host_id, group_id)
             VALUES ($1, $2)
             ON CONFLICT (host_id, group_id) DO NOTHING",
        )
        .bind(new_id)
        .bind(group_id)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    Ok(new_id)
}

pub async fn update_host_with_groups(
    pool: &PgPool,
    id: i64,
    name: Option<&str>,
    hostname: Option<&str>,
    description: Option<&str>,
    group_ids: &[i64],
) -> AppResult<u64> {
    let mut tx = pool.begin().await?;

    let rows = sqlx::query(
        "UPDATE host
         SET name = COALESCE(NULLIF($1, ''), name),
             hostname = COALESCE(NULLIF($2, ''), hostname),
             description = COALESCE($3, description),
             update_time = NOW()
         WHERE id = $4 AND deleted = 0",
    )
    .bind(name)
    .bind(hostname)
    .bind(description)
    .bind(id)
    .execute(&mut *tx)
    .await?
    .rows_affected();

    if rows == 0 {
        tx.rollback().await?;
        return Ok(0);
    }

    sqlx::query("DELETE FROM host_group_rel WHERE host_id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await?;

    for group_id in group_ids {
        sqlx::query(
            "INSERT INTO host_group_rel (host_id, group_id)
             VALUES ($1, $2)
             ON CONFLICT (host_id, group_id) DO NOTHING",
        )
        .bind(id)
        .bind(group_id)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    Ok(rows)
}

pub async fn update_host_status(pool: &PgPool, id: i64, status: i16) -> AppResult<u64> {
    let rows = sqlx::query(
        "UPDATE host SET status = $1, update_time = NOW() WHERE id = $2 AND deleted = 0",
    )
    .bind(status)
    .bind(id)
    .execute(pool)
    .await?
    .rows_affected();
    Ok(rows)
}

pub async fn soft_delete_host(pool: &PgPool, id: i64) -> AppResult<u64> {
    let rows = sqlx::query(
        "UPDATE host SET deleted = 1, update_time = NOW() WHERE id = $1 AND deleted = 0",
    )
    .bind(id)
    .execute(pool)
    .await?
    .rows_affected();
    Ok(rows)
}

pub async fn list_host_groups_for_tree(pool: &PgPool) -> AppResult<Vec<OrionHostGroupAggregate>> {
    let rows = sqlx::query_as::<_, OrionHostGroupRow>(
        "SELECT id, parent_id, name
         FROM host_group
         WHERE deleted = 0
         ORDER BY sort ASC, id ASC",
    )
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(Into::into).collect())
}

pub async fn host_group_exists(pool: &PgPool, id: i64) -> AppResult<bool> {
    let exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM host_group WHERE id = $1 AND deleted = 0)",
    )
    .bind(id)
    .fetch_one(pool)
    .await?;
    Ok(exists)
}

pub async fn create_host_group(pool: &PgPool, name: &str, parent_id: i64) -> AppResult<i64> {
    let id = sqlx::query_scalar::<_, i64>(
        "INSERT INTO host_group (name, parent_id, sort, create_time, deleted)
         VALUES ($1, $2, 0, NOW(), 0)
         RETURNING id",
    )
    .bind(name)
    .bind(parent_id)
    .fetch_one(pool)
    .await?;
    Ok(id)
}

pub async fn rename_host_group(pool: &PgPool, id: i64, name: &str) -> AppResult<u64> {
    let rows = sqlx::query("UPDATE host_group SET name = $1 WHERE id = $2 AND deleted = 0")
        .bind(name)
        .bind(id)
        .execute(pool)
        .await?
        .rows_affected();
    Ok(rows)
}

pub async fn move_host_group(
    pool: &PgPool,
    id: i64,
    target_id: i64,
    position: i32,
) -> AppResult<u64> {
    let rows = sqlx::query(
        "UPDATE host_group
         SET parent_id = $1, sort = $2
         WHERE id = $3 AND deleted = 0",
    )
    .bind(target_id)
    .bind(position)
    .bind(id)
    .execute(pool)
    .await?
    .rows_affected();
    Ok(rows)
}

pub async fn host_group_has_children(pool: &PgPool, id: i64) -> AppResult<bool> {
    let exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM host_group WHERE parent_id = $1 AND deleted = 0)",
    )
    .bind(id)
    .fetch_one(pool)
    .await?;
    Ok(exists)
}

pub async fn soft_delete_host_group(pool: &PgPool, id: i64) -> AppResult<u64> {
    let mut tx = pool.begin().await?;
    sqlx::query("DELETE FROM host_group_rel WHERE group_id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await?;

    let rows = sqlx::query("UPDATE host_group SET deleted = 1 WHERE id = $1 AND deleted = 0")
        .bind(id)
        .execute(&mut *tx)
        .await?
        .rows_affected();

    if rows == 0 {
        tx.rollback().await?;
        return Ok(0);
    }

    tx.commit().await?;
    Ok(rows)
}

pub async fn list_host_group_rel_host_ids(pool: &PgPool, group_id: i64) -> AppResult<Vec<i64>> {
    let list = sqlx::query_scalar::<_, i64>(
        "SELECT host_id FROM host_group_rel WHERE group_id = $1 ORDER BY host_id ASC",
    )
    .bind(group_id)
    .fetch_all(pool)
    .await?;
    Ok(list)
}

pub async fn count_hosts_by_ids(pool: &PgPool, host_ids: &[i64]) -> AppResult<i64> {
    let count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(1)
         FROM host
         WHERE deleted = 0 AND id = ANY($1::bigint[])",
    )
    .bind(host_ids)
    .fetch_one(pool)
    .await?;
    Ok(count)
}

pub async fn replace_host_group_rel(
    pool: &PgPool,
    group_id: i64,
    host_ids: &[i64],
) -> AppResult<()> {
    let mut tx = pool.begin().await?;
    sqlx::query("DELETE FROM host_group_rel WHERE group_id = $1")
        .bind(group_id)
        .execute(&mut *tx)
        .await?;

    for host_id in host_ids {
        sqlx::query(
            "INSERT INTO host_group_rel (host_id, group_id)
             VALUES ($1, $2)
             ON CONFLICT (host_id, group_id) DO NOTHING",
        )
        .bind(host_id)
        .bind(group_id)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    Ok(())
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

impl From<OrionHostGroupRow> for OrionHostGroupAggregate {
    fn from(value: OrionHostGroupRow) -> Self {
        Self {
            id: value.id,
            parent_id: value.parent_id,
            name: value.name,
        }
    }
}

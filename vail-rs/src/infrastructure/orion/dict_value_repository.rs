use sqlx::PgPool;

use crate::domain::orion::dict_value::{
    OrionDictValueAggregate, OrionDictValueOptionAggregate, OrionDictValueQueryFilters,
};

#[derive(Debug, sqlx::FromRow)]
struct OrionDictValueRow {
    id: i64,
    key_id: i64,
    key_name: String,
    key_description: Option<String>,
    value: String,
    label: String,
    extra: Option<String>,
    sort: i32,
    create_time: i64,
    update_time: i64,
    creator: Option<String>,
    updater: Option<String>,
}

#[derive(Debug, sqlx::FromRow)]
struct OrionDictValueOptionRow {
    key_name: String,
    value_type: String,
    label: String,
    value: String,
    extra: Option<String>,
}

pub async fn create_dict_value(
    pool: &PgPool,
    key_id: i64,
    name: &str,
    value: &str,
    label: &str,
    extra: &str,
    sort: i32,
    username: &str,
) -> Result<i64, sqlx::Error> {
    let mut tx = pool.begin().await?;

    let id = sqlx::query_scalar::<_, i64>(
        "INSERT INTO sys_dict_value (key_id, name, value, label, extra, sort, creator, updater, create_time, update_time, deleted)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $7, NOW(), NOW(), 0)
         RETURNING id",
    )
    .bind(key_id)
    .bind(name)
    .bind(value)
    .bind(label)
    .bind(extra)
    .bind(sort)
    .bind(username)
    .fetch_one(&mut *tx)
    .await?;

    insert_dict_value_history_tx(&mut tx, id, "", value).await?;
    tx.commit().await?;
    Ok(id)
}

pub async fn get_dict_value_value(pool: &PgPool, id: i64) -> Result<Option<String>, sqlx::Error> {
    sqlx::query_scalar::<_, String>(
        "SELECT value FROM sys_dict_value WHERE id = $1 AND deleted = 0",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
}

pub async fn update_dict_value(
    pool: &PgPool,
    id: i64,
    key_id: Option<i64>,
    name: Option<&str>,
    value: Option<&str>,
    label: Option<&str>,
    extra: Option<&str>,
    sort: Option<i32>,
    username: &str,
) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE sys_dict_value SET
            key_id = COALESCE($1, key_id),
            name = COALESCE($2, name),
            value = COALESCE($3, value),
            label = COALESCE($4, label),
            extra = COALESCE($5, extra),
            sort = COALESCE($6, sort),
            updater = $7,
            update_time = NOW()
         WHERE id = $8 AND deleted = 0",
    )
    .bind(key_id)
    .bind(name)
    .bind(value)
    .bind(label)
    .bind(extra)
    .bind(sort)
    .bind(username)
    .bind(id)
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
}

pub async fn insert_dict_value_history(
    pool: &PgPool,
    rel_id: i64,
    before_value: &str,
    after_value: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO sys_dict_value_history (rel_id, before_value, after_value, create_time)
         VALUES ($1, $2, $3, NOW())",
    )
    .bind(rel_id)
    .bind(before_value)
    .bind(after_value)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_history_before_value(
    pool: &PgPool,
    history_id: i64,
    rel_id: i64,
) -> Result<Option<String>, sqlx::Error> {
    sqlx::query_scalar::<_, String>(
        "SELECT before_value FROM sys_dict_value_history WHERE id = $1 AND rel_id = $2",
    )
    .bind(history_id)
    .bind(rel_id)
    .fetch_optional(pool)
    .await
}

pub async fn update_dict_value_value(
    pool: &PgPool,
    id: i64,
    value: &str,
) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE sys_dict_value SET value = $1, update_time = NOW() WHERE id = $2 AND deleted = 0",
    )
    .bind(value)
    .bind(id)
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
}

pub async fn list_dict_values_by_keys(
    pool: &PgPool,
    keys: &[String],
) -> Result<Vec<OrionDictValueOptionAggregate>, sqlx::Error> {
    let rows = sqlx::query_as::<_, OrionDictValueOptionRow>(
        "SELECT dk.key_name,
                dk.value_type,
                dv.label,
                dv.value,
                dv.extra
         FROM sys_dict_key dk
         JOIN sys_dict_value dv ON dv.key_id = dk.id AND dv.deleted = 0
         WHERE dk.key_name = ANY($1::text[])
         ORDER BY dk.id ASC, dv.sort ASC, dv.id ASC",
    )
    .bind(keys)
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(Into::into).collect())
}

pub async fn query_dict_values(
    pool: &PgPool,
    filters: &OrionDictValueQueryFilters,
) -> Result<Vec<OrionDictValueAggregate>, sqlx::Error> {
    let rows = sqlx::query_as::<_, OrionDictValueRow>(
        "SELECT dv.id,
                dv.key_id,
                dk.key_name,
                dk.description AS key_description,
                dv.value,
                dv.label,
                dv.extra,
                dv.sort,
                EXTRACT(EPOCH FROM dv.create_time)::bigint * 1000 AS create_time,
                EXTRACT(EPOCH FROM dv.update_time)::bigint * 1000 AS update_time,
                dv.creator,
                dv.updater
         FROM sys_dict_value dv
         JOIN sys_dict_key dk ON dk.id = dv.key_id
         WHERE dv.deleted = 0
           AND ($1::bigint IS NULL OR dv.key_id = $1)
           AND ($2::text IS NULL OR dk.key_name ILIKE '%' || $2 || '%')
           AND ($3::text IS NULL OR dv.value ILIKE '%' || $3 || '%')
           AND ($4::text IS NULL OR dv.label ILIKE '%' || $4 || '%')
           AND ($5::text IS NULL OR dv.extra ILIKE '%' || $5 || '%')
         ORDER BY dv.id DESC
         LIMIT $6 OFFSET $7",
    )
    .bind(filters.key_id)
    .bind(filters.key_name.as_deref())
    .bind(filters.value.as_deref())
    .bind(filters.label.as_deref())
    .bind(filters.extra.as_deref())
    .bind(filters.limit)
    .bind(filters.offset)
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(Into::into).collect())
}

pub async fn count_dict_values(
    pool: &PgPool,
    filters: &OrionDictValueQueryFilters,
) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(1)
         FROM sys_dict_value dv
         JOIN sys_dict_key dk ON dk.id = dv.key_id
         WHERE dv.deleted = 0
           AND ($1::bigint IS NULL OR dv.key_id = $1)
           AND ($2::text IS NULL OR dk.key_name ILIKE '%' || $2 || '%')
           AND ($3::text IS NULL OR dv.value ILIKE '%' || $3 || '%')
           AND ($4::text IS NULL OR dv.label ILIKE '%' || $4 || '%')
           AND ($5::text IS NULL OR dv.extra ILIKE '%' || $5 || '%')",
    )
    .bind(filters.key_id)
    .bind(filters.key_name.as_deref())
    .bind(filters.value.as_deref())
    .bind(filters.label.as_deref())
    .bind(filters.extra.as_deref())
    .fetch_one(pool)
    .await
}

pub async fn soft_delete_dict_value(pool: &PgPool, id: i64) -> Result<u64, sqlx::Error> {
    let result =
        sqlx::query("UPDATE sys_dict_value SET deleted = 1, update_time = NOW() WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?;
    Ok(result.rows_affected())
}

pub async fn soft_delete_dict_values(pool: &PgPool, ids: &[i64]) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE sys_dict_value SET deleted = 1, update_time = NOW() WHERE id = ANY($1::bigint[])",
    )
    .bind(ids)
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
}

async fn insert_dict_value_history_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    rel_id: i64,
    before_value: &str,
    after_value: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO sys_dict_value_history (rel_id, before_value, after_value, create_time)
         VALUES ($1, $2, $3, NOW())",
    )
    .bind(rel_id)
    .bind(before_value)
    .bind(after_value)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

impl From<OrionDictValueRow> for OrionDictValueAggregate {
    fn from(value: OrionDictValueRow) -> Self {
        Self {
            id: value.id,
            key_id: value.key_id,
            key_name: value.key_name,
            key_description: value.key_description,
            value: value.value,
            label: value.label,
            extra: value.extra,
            sort: value.sort,
            create_time: value.create_time,
            update_time: value.update_time,
            creator: value.creator,
            updater: value.updater,
        }
    }
}

impl From<OrionDictValueOptionRow> for OrionDictValueOptionAggregate {
    fn from(value: OrionDictValueOptionRow) -> Self {
        Self {
            key_name: value.key_name,
            value_type: value.value_type,
            label: value.label,
            value: value.value,
            extra: value.extra,
        }
    }
}

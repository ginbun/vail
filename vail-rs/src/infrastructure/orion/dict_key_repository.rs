use sqlx::PgPool;

use crate::domain::orion::dict_key::{OrionDictKeyAggregate, OrionDictKeyQueryFilters};

#[derive(Debug, sqlx::FromRow)]
struct OrionDictKeyRow {
    id: i64,
    key_name: String,
    value_type: String,
    extra_schema: Option<String>,
    description: Option<String>,
    create_time: i64,
    update_time: i64,
    creator: Option<String>,
    updater: Option<String>,
}

pub async fn create_dict_key(
    pool: &PgPool,
    key_name: &str,
    value_type: &str,
    extra_schema: &str,
    description: &str,
    username: &str,
) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar::<_, i64>(
        "INSERT INTO sys_dict_key (key_name, value_type, extra_schema, description, creator, updater, create_time, update_time)
         VALUES ($1, $2, $3, $4, $5, $5, NOW(), NOW())
         RETURNING id",
    )
    .bind(key_name)
    .bind(value_type)
    .bind(extra_schema)
    .bind(description)
    .bind(username)
    .fetch_one(pool)
    .await
}

pub async fn update_dict_key(
    pool: &PgPool,
    id: i64,
    key_name: Option<&str>,
    value_type: Option<&str>,
    extra_schema: Option<String>,
    description: Option<String>,
    username: &str,
) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE sys_dict_key SET
            key_name = COALESCE(NULLIF($1, ''), key_name),
            value_type = COALESCE(NULLIF($2, ''), value_type),
            extra_schema = COALESCE($3, extra_schema),
            description = COALESCE($4, description),
            updater = $5,
            update_time = NOW()
         WHERE id = $6",
    )
    .bind(key_name)
    .bind(value_type)
    .bind(extra_schema)
    .bind(description)
    .bind(username)
    .bind(id)
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
}

pub async fn list_dict_keys(pool: &PgPool) -> Result<Vec<OrionDictKeyAggregate>, sqlx::Error> {
    let rows = sqlx::query_as::<_, OrionDictKeyRow>(
        "SELECT id,
                key_name,
                value_type,
                extra_schema,
                description,
                EXTRACT(EPOCH FROM create_time)::bigint * 1000 AS create_time,
                EXTRACT(EPOCH FROM update_time)::bigint * 1000 AS update_time,
                creator,
                updater
         FROM sys_dict_key
         ORDER BY id DESC",
    )
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(Into::into).collect())
}

pub async fn query_dict_keys(
    pool: &PgPool,
    filters: &OrionDictKeyQueryFilters,
) -> Result<Vec<OrionDictKeyAggregate>, sqlx::Error> {
    let rows = sqlx::query_as::<_, OrionDictKeyRow>(
        "SELECT id,
                key_name,
                value_type,
                extra_schema,
                description,
                EXTRACT(EPOCH FROM create_time)::bigint * 1000 AS create_time,
                EXTRACT(EPOCH FROM update_time)::bigint * 1000 AS update_time,
                creator,
                updater
         FROM sys_dict_key
         WHERE ($1::bigint IS NULL OR id = $1)
           AND ($2::text IS NULL OR key_name ILIKE '%' || $2 || '%')
           AND ($3::text IS NULL OR description ILIKE '%' || $3 || '%')
           AND ($4::text IS NULL OR key_name ILIKE '%' || $4 || '%' OR description ILIKE '%' || $4 || '%')
         ORDER BY id DESC
         LIMIT $5 OFFSET $6",
    )
    .bind(filters.id)
    .bind(filters.key_name.as_deref())
    .bind(filters.description.as_deref())
    .bind(filters.search_value.as_deref())
    .bind(filters.limit)
    .bind(filters.offset)
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(Into::into).collect())
}

pub async fn count_dict_keys(
    pool: &PgPool,
    filters: &OrionDictKeyQueryFilters,
) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(1)
         FROM sys_dict_key
         WHERE ($1::bigint IS NULL OR id = $1)
           AND ($2::text IS NULL OR key_name ILIKE '%' || $2 || '%')
           AND ($3::text IS NULL OR description ILIKE '%' || $3 || '%')
           AND ($4::text IS NULL OR key_name ILIKE '%' || $4 || '%' OR description ILIKE '%' || $4 || '%')",
    )
    .bind(filters.id)
    .bind(filters.key_name.as_deref())
    .bind(filters.description.as_deref())
    .bind(filters.search_value.as_deref())
    .fetch_one(pool)
    .await
}

pub async fn delete_dict_key(pool: &PgPool, id: i64) -> Result<u64, sqlx::Error> {
    let result = sqlx::query("DELETE FROM sys_dict_key WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected())
}

pub async fn batch_delete_dict_keys(pool: &PgPool, ids: &[i64]) -> Result<u64, sqlx::Error> {
    let result = sqlx::query("DELETE FROM sys_dict_key WHERE id = ANY($1::bigint[])")
        .bind(ids)
        .execute(pool)
        .await?;
    Ok(result.rows_affected())
}

impl From<OrionDictKeyRow> for OrionDictKeyAggregate {
    fn from(value: OrionDictKeyRow) -> Self {
        Self {
            id: value.id,
            key_name: value.key_name,
            value_type: value.value_type,
            extra_schema: value.extra_schema,
            description: value.description,
            create_time: value.create_time,
            update_time: value.update_time,
            creator: value.creator,
            updater: value.updater,
        }
    }
}

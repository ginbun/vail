use sqlx::PgPool;

use crate::domain::orion::host::{OrionHostAggregate, OrionHostRow};

#[derive(Debug, Clone, Default)]
pub struct OrionHostQueryFilters {
    pub id: Option<i64>,
    pub name: Option<String>,
    pub address: Option<String>,
    pub search_value: Option<String>,
    pub status: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

pub async fn list_hosts(pool: &PgPool) -> Result<Vec<OrionHostAggregate>, sqlx::Error> {
    let rows = sqlx::query_as::<_, OrionHostRow>(
        "SELECT
            h.id,
            h.name,
            h.hostname,
            h.description,
            h.status,
            EXTRACT(EPOCH FROM h.create_time)::bigint * 1000,
            EXTRACT(EPOCH FROM h.update_time)::bigint * 1000,
            ARRAY_REMOVE(ARRAY_AGG(hgr.group_id), NULL)
         FROM host h
         LEFT JOIN host_group_rel hgr ON hgr.host_id = h.id
         WHERE h.deleted = 0
         GROUP BY h.id
         ORDER BY h.id DESC",
    )
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(Into::into).collect())
}

pub async fn get_host_by_id(
    pool: &PgPool,
    host_id: i64,
) -> Result<Option<OrionHostAggregate>, sqlx::Error> {
    let row = sqlx::query_as::<_, OrionHostRow>(
        "SELECT
            h.id,
            h.name,
            h.hostname,
            h.description,
            h.status,
            EXTRACT(EPOCH FROM h.create_time)::bigint * 1000,
            EXTRACT(EPOCH FROM h.update_time)::bigint * 1000,
            ARRAY_REMOVE(ARRAY_AGG(hgr.group_id), NULL)
         FROM host h
         LEFT JOIN host_group_rel hgr ON hgr.host_id = h.id
         WHERE h.deleted = 0 AND h.id = $1
         GROUP BY h.id
         LIMIT 1",
    )
    .bind(host_id)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(Into::into))
}

pub async fn query_hosts(
    pool: &PgPool,
    filters: &OrionHostQueryFilters,
) -> Result<Vec<OrionHostAggregate>, sqlx::Error> {
    let rows = sqlx::query_as::<_, OrionHostRow>(
        "SELECT
            h.id,
            h.name,
            h.hostname,
            h.description,
            h.status,
            EXTRACT(EPOCH FROM h.create_time)::bigint * 1000,
            EXTRACT(EPOCH FROM h.update_time)::bigint * 1000,
            ARRAY_REMOVE(ARRAY_AGG(hgr.group_id), NULL)
         FROM host h
         LEFT JOIN host_group_rel hgr ON hgr.host_id = h.id
         WHERE h.deleted = 0
           AND ($1::bigint IS NULL OR h.id = $1)
           AND ($2::text IS NULL OR h.name ILIKE '%' || $2 || '%')
           AND ($3::text IS NULL OR h.hostname ILIKE '%' || $3 || '%')
           AND ($4::text IS NULL OR h.name ILIKE '%' || $4 || '%' OR h.hostname ILIKE '%' || $4 || '%')
           AND (
               $5::text IS NULL
               OR ($5 = 'ENABLED' AND h.status = 1)
               OR ($5 = 'DISABLED' AND h.status <> 1)
           )
         GROUP BY h.id
         ORDER BY h.id DESC
         LIMIT $6 OFFSET $7",
    )
    .bind(filters.id)
    .bind(filters.name.as_ref())
    .bind(filters.address.as_ref())
    .bind(filters.search_value.as_ref())
    .bind(filters.status.as_ref())
    .bind(filters.limit.unwrap_or(20))
    .bind(filters.offset.unwrap_or(0))
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(Into::into).collect())
}

pub async fn count_hosts(
    pool: &PgPool,
    filters: &OrionHostQueryFilters,
) -> Result<i64, sqlx::Error> {
    let total = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(1)
         FROM host h
         WHERE h.deleted = 0
           AND ($1::bigint IS NULL OR h.id = $1)
           AND ($2::text IS NULL OR h.name ILIKE '%' || $2 || '%')
           AND ($3::text IS NULL OR h.hostname ILIKE '%' || $3 || '%')
           AND ($4::text IS NULL OR h.name ILIKE '%' || $4 || '%' OR h.hostname ILIKE '%' || $4 || '%')
           AND (
               $5::text IS NULL
               OR ($5 = 'ENABLED' AND h.status = 1)
               OR ($5 = 'DISABLED' AND h.status <> 1)
           )",
    )
    .bind(filters.id)
    .bind(filters.name.as_ref())
    .bind(filters.address.as_ref())
    .bind(filters.search_value.as_ref())
    .bind(filters.status.as_ref())
    .fetch_one(pool)
    .await?;

    Ok(total)
}

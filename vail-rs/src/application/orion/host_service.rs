use sqlx::PgPool;

use crate::{
    domain::orion::host::OrionHostAggregate,
    infrastructure::orion::host_repository::{self, OrionHostQueryFilters},
};

pub async fn list_hosts(pool: &PgPool) -> Result<Vec<OrionHostAggregate>, sqlx::Error> {
    host_repository::list_hosts(pool).await
}

pub async fn get_host_by_id(
    pool: &PgPool,
    host_id: i64,
) -> Result<Option<OrionHostAggregate>, sqlx::Error> {
    host_repository::get_host_by_id(pool, host_id).await
}

pub async fn query_hosts(
    pool: &PgPool,
    filters: OrionHostQueryFilters,
) -> Result<Vec<OrionHostAggregate>, sqlx::Error> {
    host_repository::query_hosts(pool, &filters).await
}

pub async fn count_hosts(
    pool: &PgPool,
    filters: OrionHostQueryFilters,
) -> Result<i64, sqlx::Error> {
    host_repository::count_hosts(pool, &filters).await
}

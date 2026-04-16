use sqlx::PgPool;

use crate::{
    domain::orion::system_user::OrionSystemUserAggregate,
    infrastructure::orion::system_user_repository::{self, OrionSystemUserQueryFilters},
};

pub async fn get_system_user_by_id(
    pool: &PgPool,
    user_id: i64,
) -> Result<Option<OrionSystemUserAggregate>, sqlx::Error> {
    system_user_repository::get_system_user_by_id(pool, user_id).await
}

pub async fn list_system_users(
    pool: &PgPool,
) -> Result<Vec<OrionSystemUserAggregate>, sqlx::Error> {
    system_user_repository::list_system_users(pool).await
}

pub async fn query_system_users(
    pool: &PgPool,
    filters: OrionSystemUserQueryFilters,
) -> Result<Vec<OrionSystemUserAggregate>, sqlx::Error> {
    system_user_repository::query_system_users(pool, &filters).await
}

pub async fn count_system_users(
    pool: &PgPool,
    filters: OrionSystemUserQueryFilters,
) -> Result<i64, sqlx::Error> {
    system_user_repository::count_system_users(pool, &filters).await
}

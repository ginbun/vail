use sqlx::PgPool;

use crate::{domain::orion::role::OrionRoleAggregate, infrastructure::orion::role_repository};

#[derive(Debug, Clone)]
pub struct OrionRoleCreateInput {
    pub name: String,
    pub code: String,
    pub description: Option<String>,
    pub status: Option<i16>,
}

#[derive(Debug, Clone)]
pub struct OrionRoleUpdateInput {
    pub id: i64,
    pub name: Option<String>,
    pub code: Option<String>,
    pub description: Option<String>,
    pub status: Option<i16>,
}

pub async fn create_role(pool: &PgPool, input: OrionRoleCreateInput) -> Result<i64, sqlx::Error> {
    role_repository::create_role(
        pool,
        &input.name,
        &input.code,
        input.description.as_deref(),
        input.status,
    )
    .await
}

pub async fn update_role(pool: &PgPool, input: OrionRoleUpdateInput) -> Result<u64, sqlx::Error> {
    role_repository::update_role(
        pool,
        input.id,
        input.name.as_deref(),
        input.code.as_deref(),
        input.description.as_deref(),
        input.status,
    )
    .await
}

pub async fn update_role_status(pool: &PgPool, id: i64, status: i16) -> Result<u64, sqlx::Error> {
    role_repository::update_role_status(pool, id, status).await
}

pub async fn get_role_by_id(
    pool: &PgPool,
    id: i64,
) -> Result<Option<OrionRoleAggregate>, sqlx::Error> {
    role_repository::get_role_by_id(pool, id).await
}

pub async fn list_roles(pool: &PgPool) -> Result<Vec<OrionRoleAggregate>, sqlx::Error> {
    role_repository::list_roles(pool).await
}

pub async fn query_roles(
    pool: &PgPool,
    limit: i64,
    offset: i64,
) -> Result<Vec<OrionRoleAggregate>, sqlx::Error> {
    role_repository::query_roles(pool, limit, offset).await
}

pub async fn count_roles(pool: &PgPool) -> Result<i64, sqlx::Error> {
    role_repository::count_roles(pool).await
}

pub async fn soft_delete_role(pool: &PgPool, id: i64) -> Result<u64, sqlx::Error> {
    role_repository::soft_delete_role(pool, id).await
}

pub async fn replace_role_menus(
    pool: &PgPool,
    role_id: i64,
    menu_ids: Vec<i64>,
) -> Result<(), sqlx::Error> {
    let menu_ids = normalize_ids(menu_ids);
    role_repository::replace_role_menus(pool, role_id, &menu_ids).await
}

pub async fn list_role_menu_ids(pool: &PgPool, role_id: i64) -> Result<Vec<i64>, sqlx::Error> {
    role_repository::list_role_menu_ids(pool, role_id).await
}

pub async fn list_roles_by_user_id(
    pool: &PgPool,
    user_id: i64,
) -> Result<Vec<OrionRoleAggregate>, sqlx::Error> {
    role_repository::list_roles_by_user_id(pool, user_id).await
}

fn normalize_ids(mut ids: Vec<i64>) -> Vec<i64> {
    ids.retain(|v| *v > 0);
    ids.sort_unstable();
    ids.dedup();
    ids
}

#[cfg(test)]
mod tests {
    use super::normalize_ids;

    #[test]
    fn normalize_ids_drops_invalid_and_duplicates() {
        let ids = normalize_ids(vec![0, -1, 3, 3, 2]);
        assert_eq!(ids, vec![2, 3]);
    }
}

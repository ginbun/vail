use sqlx::PgPool;

use crate::{
    domain::orion::dict_key::{OrionDictKeyAggregate, OrionDictKeyQueryFilters},
    infrastructure::orion::dict_key_repository,
};

#[derive(Debug, Clone)]
pub struct OrionDictKeyCreateInput {
    pub key_name: String,
    pub value_type: String,
    pub extra_schema: String,
    pub description: String,
    pub username: String,
}

#[derive(Debug, Clone)]
pub struct OrionDictKeyUpdateInput {
    pub id: i64,
    pub key_name: Option<String>,
    pub value_type: Option<String>,
    pub extra_schema: Option<String>,
    pub description: Option<String>,
    pub username: String,
}

pub async fn create_dict_key(
    pool: &PgPool,
    input: OrionDictKeyCreateInput,
) -> Result<i64, sqlx::Error> {
    dict_key_repository::create_dict_key(
        pool,
        &input.key_name,
        &input.value_type,
        &input.extra_schema,
        &input.description,
        &input.username,
    )
    .await
}

pub async fn update_dict_key(
    pool: &PgPool,
    input: OrionDictKeyUpdateInput,
) -> Result<u64, sqlx::Error> {
    dict_key_repository::update_dict_key(
        pool,
        input.id,
        input.key_name.as_deref(),
        input.value_type.as_deref(),
        input.extra_schema,
        input.description,
        &input.username,
    )
    .await
}

pub async fn list_dict_keys(pool: &PgPool) -> Result<Vec<OrionDictKeyAggregate>, sqlx::Error> {
    dict_key_repository::list_dict_keys(pool).await
}

pub async fn query_dict_keys(
    pool: &PgPool,
    filters: OrionDictKeyQueryFilters,
) -> Result<Vec<OrionDictKeyAggregate>, sqlx::Error> {
    dict_key_repository::query_dict_keys(pool, &filters).await
}

pub async fn count_dict_keys(
    pool: &PgPool,
    filters: OrionDictKeyQueryFilters,
) -> Result<i64, sqlx::Error> {
    dict_key_repository::count_dict_keys(pool, &filters).await
}

pub async fn delete_dict_key(pool: &PgPool, id: i64) -> Result<u64, sqlx::Error> {
    dict_key_repository::delete_dict_key(pool, id).await
}

pub async fn batch_delete_dict_keys(pool: &PgPool, ids: Vec<i64>) -> Result<u64, sqlx::Error> {
    let ids = normalize_ids(ids);
    if ids.is_empty() {
        return Ok(0);
    }
    dict_key_repository::batch_delete_dict_keys(pool, &ids).await
}

fn normalize_ids(mut ids: Vec<i64>) -> Vec<i64> {
    ids.retain(|v| *v > 0);
    ids.sort_unstable();
    ids.dedup();
    ids
}

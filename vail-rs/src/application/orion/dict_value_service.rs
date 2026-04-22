use sqlx::PgPool;

use crate::{
    domain::orion::dict_value::{
        OrionDictValueAggregate, OrionDictValueOptionAggregate, OrionDictValueQueryFilters,
    },
    infrastructure::orion::dict_value_repository,
};

#[derive(Debug, Clone)]
pub struct OrionDictValueCreateInput {
    pub key_id: i64,
    pub name: String,
    pub value: String,
    pub label: String,
    pub extra: String,
    pub sort: i32,
    pub username: String,
}

#[derive(Debug, Clone)]
pub struct OrionDictValueUpdateInput {
    pub id: i64,
    pub key_id: Option<i64>,
    pub name: Option<String>,
    pub value: Option<String>,
    pub label: Option<String>,
    pub extra: Option<String>,
    pub sort: Option<i32>,
    pub username: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrionDictValueUpdateOutcome {
    Updated,
    NotFound,
}

#[derive(Debug, Clone)]
pub struct OrionDictValueRollbackInput {
    pub id: i64,
    pub history_id: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrionDictValueRollbackOutcome {
    RolledBack,
    HistoryNotFound,
    DictValueNotFound,
}

pub async fn create_dict_value(
    pool: &PgPool,
    input: OrionDictValueCreateInput,
) -> Result<i64, sqlx::Error> {
    dict_value_repository::create_dict_value(
        pool,
        input.key_id,
        &input.name,
        &input.value,
        &input.label,
        &input.extra,
        input.sort,
        &input.username,
    )
    .await
}

pub async fn update_dict_value(
    pool: &PgPool,
    input: OrionDictValueUpdateInput,
) -> Result<OrionDictValueUpdateOutcome, sqlx::Error> {
    let before_value = match dict_value_repository::get_dict_value_value(pool, input.id).await? {
        Some(value) => value,
        None => return Ok(OrionDictValueUpdateOutcome::NotFound),
    };

    let rows = dict_value_repository::update_dict_value(
        pool,
        input.id,
        input.key_id,
        input.name.as_deref(),
        input.value.as_deref(),
        input.label.as_deref(),
        input.extra.as_deref(),
        input.sort,
        &input.username,
    )
    .await?;

    if rows == 0 {
        return Ok(OrionDictValueUpdateOutcome::NotFound);
    }

    let Some(after_value) = dict_value_repository::get_dict_value_value(pool, input.id).await?
    else {
        return Ok(OrionDictValueUpdateOutcome::NotFound);
    };

    if before_value != after_value {
        dict_value_repository::insert_dict_value_history(
            pool,
            input.id,
            &before_value,
            &after_value,
        )
        .await?;
    }

    Ok(OrionDictValueUpdateOutcome::Updated)
}

pub async fn rollback_dict_value(
    pool: &PgPool,
    input: OrionDictValueRollbackInput,
) -> Result<OrionDictValueRollbackOutcome, sqlx::Error> {
    let Some(rollback_to) =
        dict_value_repository::get_history_before_value(pool, input.history_id, input.id).await?
    else {
        return Ok(OrionDictValueRollbackOutcome::HistoryNotFound);
    };

    let Some(current_value) = dict_value_repository::get_dict_value_value(pool, input.id).await?
    else {
        return Ok(OrionDictValueRollbackOutcome::DictValueNotFound);
    };

    let rows = dict_value_repository::update_dict_value_value(pool, input.id, &rollback_to).await?;
    if rows == 0 {
        return Ok(OrionDictValueRollbackOutcome::DictValueNotFound);
    }

    dict_value_repository::insert_dict_value_history(pool, input.id, &current_value, &rollback_to)
        .await?;
    Ok(OrionDictValueRollbackOutcome::RolledBack)
}

pub async fn list_dict_values_by_keys(
    pool: &PgPool,
    keys: &[String],
) -> Result<Vec<OrionDictValueOptionAggregate>, sqlx::Error> {
    dict_value_repository::list_dict_values_by_keys(pool, keys).await
}

pub async fn query_dict_values(
    pool: &PgPool,
    filters: OrionDictValueQueryFilters,
) -> Result<Vec<OrionDictValueAggregate>, sqlx::Error> {
    dict_value_repository::query_dict_values(pool, &filters).await
}

pub async fn count_dict_values(
    pool: &PgPool,
    filters: OrionDictValueQueryFilters,
) -> Result<i64, sqlx::Error> {
    dict_value_repository::count_dict_values(pool, &filters).await
}

pub async fn soft_delete_dict_value(pool: &PgPool, id: i64) -> Result<u64, sqlx::Error> {
    dict_value_repository::soft_delete_dict_value(pool, id).await
}

pub async fn soft_delete_dict_values(pool: &PgPool, ids: Vec<i64>) -> Result<u64, sqlx::Error> {
    let ids = normalize_ids(ids);
    if ids.is_empty() {
        return Ok(0);
    }
    dict_value_repository::soft_delete_dict_values(pool, &ids).await
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
        let ids = normalize_ids(vec![4, 0, -2, 4, 3, 3]);
        assert_eq!(ids, vec![3, 4]);
    }
}

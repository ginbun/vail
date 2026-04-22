use sqlx::PgPool;
use std::collections::HashMap;

use crate::{
    domain::orion::system_message::{
        OrionSystemMessageAggregate, OrionSystemMessageCountFilters, OrionSystemMessageListFilters,
    },
    infrastructure::orion::system_message_repository,
};

pub async fn list_system_messages(
    pool: &PgPool,
    filters: OrionSystemMessageListFilters,
) -> Result<Vec<OrionSystemMessageAggregate>, sqlx::Error> {
    system_message_repository::list_system_messages(pool, &filters).await
}

pub async fn count_system_messages_by_classify(
    pool: &PgPool,
    filters: OrionSystemMessageCountFilters,
) -> Result<HashMap<String, i64>, sqlx::Error> {
    let rows = system_message_repository::count_system_messages_by_classify(pool, &filters).await?;
    Ok(rows
        .into_iter()
        .map(|row| (row.classify, row.count))
        .collect())
}

pub async fn has_unread_system_messages(pool: &PgPool, user_id: i64) -> Result<bool, sqlx::Error> {
    system_message_repository::has_unread_system_messages(pool, user_id).await
}

pub async fn mark_system_message_read(
    pool: &PgPool,
    id: i64,
    user_id: i64,
) -> Result<u64, sqlx::Error> {
    system_message_repository::mark_system_message_read(pool, id, user_id).await
}

pub async fn mark_system_messages_read_all(
    pool: &PgPool,
    user_id: i64,
    classify: Option<String>,
) -> Result<u64, sqlx::Error> {
    system_message_repository::mark_system_messages_read_all(pool, user_id, classify.as_deref())
        .await
}

pub async fn delete_system_message(
    pool: &PgPool,
    id: i64,
    user_id: i64,
) -> Result<u64, sqlx::Error> {
    system_message_repository::delete_system_message(pool, id, user_id).await
}

pub async fn clear_system_messages(
    pool: &PgPool,
    user_id: i64,
    classify: Option<String>,
) -> Result<u64, sqlx::Error> {
    system_message_repository::clear_system_messages(pool, user_id, classify.as_deref()).await
}

pub async fn count_unread_system_messages(pool: &PgPool, user_id: i64) -> Result<i64, sqlx::Error> {
    system_message_repository::count_unread_system_messages(pool, user_id).await
}

pub fn effective_offset(offset: i64, max_id: Option<i64>) -> i64 {
    if max_id.is_some() {
        0
    } else {
        offset
    }
}

#[cfg(test)]
mod tests {
    use super::effective_offset;

    #[test]
    fn max_id_forces_zero_offset() {
        assert_eq!(effective_offset(40, Some(1000)), 0);
    }

    #[test]
    fn no_max_id_keeps_pagination_offset() {
        assert_eq!(effective_offset(40, None), 40);
    }
}

use sqlx::PgPool;

use crate::domain::orion::system_message::{
    OrionSystemMessageAggregate, OrionSystemMessageClassifyCount, OrionSystemMessageCountFilters,
    OrionSystemMessageListFilters,
};

#[derive(Debug, sqlx::FromRow)]
struct OrionSystemMessageRow {
    id: i64,
    classify: String,
    message_type: String,
    status: i16,
    rel_key: Option<String>,
    title: String,
    content: String,
    content_html: Option<String>,
    create_time: i64,
}

#[derive(Debug, sqlx::FromRow)]
struct OrionSystemMessageClassifyCountRow {
    classify: String,
    count: i64,
}

pub async fn list_system_messages(
    pool: &PgPool,
    filters: &OrionSystemMessageListFilters,
) -> Result<Vec<OrionSystemMessageAggregate>, sqlx::Error> {
    let rows = sqlx::query_as::<_, OrionSystemMessageRow>(
        "SELECT id,
                classify,
                type AS message_type,
                status,
                rel_key,
                title,
                content,
                content_html,
                EXTRACT(EPOCH FROM create_time)::bigint * 1000 AS create_time
         FROM sys_system_message
         WHERE (user_id IS NULL OR user_id = $1)
           AND ($2::text IS NULL OR classify = $2)
           AND ($3::boolean = FALSE OR status = 0)
           AND ($4::bigint IS NULL OR id < $4)
         ORDER BY id DESC
         LIMIT $5 OFFSET $6",
    )
    .bind(filters.user_id)
    .bind(filters.classify.as_deref())
    .bind(filters.query_unread)
    .bind(filters.max_id)
    .bind(filters.limit)
    .bind(filters.offset)
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(Into::into).collect())
}

pub async fn count_system_messages_by_classify(
    pool: &PgPool,
    filters: &OrionSystemMessageCountFilters,
) -> Result<Vec<OrionSystemMessageClassifyCount>, sqlx::Error> {
    let rows = sqlx::query_as::<_, OrionSystemMessageClassifyCountRow>(
        "SELECT classify, COUNT(1)::bigint AS count
         FROM sys_system_message
         WHERE (user_id IS NULL OR user_id = $1)
           AND ($2::boolean = FALSE OR status = 0)
         GROUP BY classify",
    )
    .bind(filters.user_id)
    .bind(filters.query_unread)
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(Into::into).collect())
}

pub async fn has_unread_system_messages(pool: &PgPool, user_id: i64) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(
            SELECT 1 FROM sys_system_message
            WHERE (user_id IS NULL OR user_id = $1) AND status = 0
         )",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await
}

pub async fn mark_system_message_read(
    pool: &PgPool,
    id: i64,
    user_id: i64,
) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE sys_system_message
         SET status = 1, read_time = NOW()
         WHERE id = $1
           AND status = 0
           AND (user_id IS NULL OR user_id = $2)",
    )
    .bind(id)
    .bind(user_id)
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
}

pub async fn mark_system_messages_read_all(
    pool: &PgPool,
    user_id: i64,
    classify: Option<&str>,
) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE sys_system_message
         SET status = 1, read_time = NOW()
         WHERE status = 0
           AND (user_id IS NULL OR user_id = $1)
           AND ($2::text IS NULL OR classify = $2)",
    )
    .bind(user_id)
    .bind(classify)
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
}

pub async fn delete_system_message(
    pool: &PgPool,
    id: i64,
    user_id: i64,
) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        "DELETE FROM sys_system_message WHERE id = $1 AND (user_id IS NULL OR user_id = $2)",
    )
    .bind(id)
    .bind(user_id)
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
}

pub async fn clear_system_messages(
    pool: &PgPool,
    user_id: i64,
    classify: Option<&str>,
) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        "DELETE FROM sys_system_message
         WHERE status = 1
           AND (user_id IS NULL OR user_id = $1)
           AND ($2::text IS NULL OR classify = $2)",
    )
    .bind(user_id)
    .bind(classify)
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
}

pub async fn count_unread_system_messages(pool: &PgPool, user_id: i64) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(1)::bigint
         FROM sys_system_message
         WHERE (user_id IS NULL OR user_id = $1) AND status = 0",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await
}

impl From<OrionSystemMessageRow> for OrionSystemMessageAggregate {
    fn from(value: OrionSystemMessageRow) -> Self {
        Self {
            id: value.id,
            classify: value.classify,
            message_type: value.message_type,
            status: value.status,
            rel_key: value.rel_key,
            title: value.title,
            content: value.content,
            content_html: value.content_html,
            create_time: value.create_time,
        }
    }
}

impl From<OrionSystemMessageClassifyCountRow> for OrionSystemMessageClassifyCount {
    fn from(value: OrionSystemMessageClassifyCountRow) -> Self {
        Self {
            classify: value.classify,
            count: value.count,
        }
    }
}

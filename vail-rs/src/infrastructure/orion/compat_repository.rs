use serde_json::Value;
use sqlx::PgPool;

use crate::error::AppResult;

pub async fn load_cache_json(pool: &PgPool, key: &str) -> AppResult<Option<Value>> {
    let raw = sqlx::query_scalar::<_, String>(
        "SELECT cache_value FROM cache WHERE cache_key = $1 AND (expire_time IS NULL OR expire_time > NOW())",
    )
    .bind(key)
    .fetch_optional(pool)
    .await?;

    Ok(raw.and_then(|v| serde_json::from_str::<Value>(&v).ok()))
}

pub async fn save_cache_json(pool: &PgPool, key: &str, value: &Value) -> AppResult<()> {
    let payload = serde_json::to_string(value).unwrap_or_else(|_| "null".to_string());
    sqlx::query(
        "INSERT INTO cache (cache_key, cache_value, expire_time, create_time)
         VALUES ($1, $2, NULL, NOW())
         ON CONFLICT (cache_key)
         DO UPDATE SET cache_value = EXCLUDED.cache_value, create_time = NOW()",
    )
    .bind(key)
    .bind(payload)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn delete_cache_key(pool: &PgPool, key: &str) -> AppResult<u64> {
    let result = sqlx::query("DELETE FROM cache WHERE cache_key = $1")
        .bind(key)
        .execute(pool)
        .await?;
    Ok(result.rows_affected())
}

pub async fn load_cache_value(pool: &PgPool, key: &str) -> AppResult<Option<String>> {
    sqlx::query_scalar::<_, String>(
        "SELECT cache_value
         FROM cache
         WHERE cache_key = $1
           AND (expire_time IS NULL OR expire_time > NOW())",
    )
    .bind(key)
    .fetch_optional(pool)
    .await
    .map_err(Into::into)
}

pub async fn save_cache_value_with_expire_days(
    pool: &PgPool,
    key: &str,
    value: &str,
    expire_days: i64,
) -> AppResult<()> {
    sqlx::query(
        "INSERT INTO cache (cache_key, cache_value, expire_time, create_time)
         VALUES ($1, $2, NOW() + ($3 || ' days')::interval, NOW())
         ON CONFLICT (cache_key)
         DO UPDATE SET cache_value = EXCLUDED.cache_value,
                       expire_time = EXCLUDED.expire_time",
    )
    .bind(key)
    .bind(value)
    .bind(expire_days)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn next_sequence(pool: &PgPool, seq_key: &str) -> AppResult<i64> {
    let row = sqlx::query_scalar::<_, String>("SELECT cache_value FROM cache WHERE cache_key = $1")
        .bind(seq_key)
        .fetch_optional(pool)
        .await?;

    let current = row
        .as_deref()
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(0);
    let next = current.saturating_add(1);

    sqlx::query(
        "INSERT INTO cache (cache_key, cache_value, expire_time, create_time)
         VALUES ($1, $2, NULL, NOW())
         ON CONFLICT (cache_key)
         DO UPDATE SET cache_value = EXCLUDED.cache_value, create_time = NOW()",
    )
    .bind(seq_key)
    .bind(next.to_string())
    .execute(pool)
    .await?;

    Ok(next)
}

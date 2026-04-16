use chrono::Utc;
use serde_json::{Map, Value};
use sqlx::PgPool;

use crate::domain::orion::compat::OrionCompatModule;
use crate::error::{AppError, AppResult};
use crate::infrastructure::orion::compat_repository;

#[derive(Debug, Clone, Copy)]
pub struct PageQuery {
    pub page: i64,
    pub limit: i64,
}

fn now_ms() -> i64 {
    Utc::now().timestamp_millis()
}

fn as_array(value: Option<Value>) -> Vec<Value> {
    value
        .and_then(|v| v.as_array().cloned())
        .unwrap_or_default()
}

fn seq_key(store_key: &str) -> String {
    format!("{store_key}:seq")
}

fn sanitize_page(page: i64, limit: i64) -> (i64, i64) {
    let p = if page <= 0 { 1 } else { page };
    let l = limit.clamp(1, 200);
    (p, l)
}

fn object_or_default(value: Value) -> Map<String, Value> {
    value.as_object().cloned().unwrap_or_default()
}

pub async fn list_records(pool: &PgPool, module: OrionCompatModule) -> AppResult<Vec<Value>> {
    Ok(as_array(
        compat_repository::load_cache_json(pool, module.store_key()).await?,
    ))
}

pub async fn save_records(
    pool: &PgPool,
    module: OrionCompatModule,
    rows: Vec<Value>,
) -> AppResult<()> {
    compat_repository::save_cache_json(pool, module.store_key(), &Value::Array(rows)).await
}

pub async fn create_record(
    pool: &PgPool,
    module: OrionCompatModule,
    payload: Value,
    operator: &str,
) -> AppResult<Value> {
    let mut rows = list_records(pool, module).await?;
    let id = compat_repository::next_sequence(pool, &seq_key(module.store_key())).await?;
    let mut obj = object_or_default(payload);
    let now = now_ms();
    obj.insert("id".to_string(), Value::from(id));
    obj.entry("createTime".to_string())
        .or_insert(Value::from(now));
    obj.insert("updateTime".to_string(), Value::from(now));
    obj.entry("creator".to_string())
        .or_insert(Value::from(operator.to_string()));
    obj.insert("updater".to_string(), Value::from(operator.to_string()));
    let value = Value::Object(obj);
    rows.push(value.clone());
    compat_repository::save_cache_json(pool, module.store_key(), &Value::Array(rows)).await?;
    Ok(value)
}

pub async fn update_record(
    pool: &PgPool,
    module: OrionCompatModule,
    id: i64,
    payload: Value,
    operator: &str,
) -> AppResult<Value> {
    let mut rows = list_records(pool, module).await?;
    let patch = object_or_default(payload);
    let mut updated = None;

    for row in &mut rows {
        let Some(obj) = row.as_object_mut() else {
            continue;
        };
        if obj.get("id").and_then(Value::as_i64) != Some(id) {
            continue;
        }
        for (k, v) in &patch {
            if k != "id" {
                obj.insert(k.clone(), v.clone());
            }
        }
        obj.insert("updateTime".to_string(), Value::from(now_ms()));
        obj.insert("updater".to_string(), Value::from(operator.to_string()));
        updated = Some(Value::Object(obj.clone()));
        break;
    }

    let value = updated.ok_or_else(|| AppError::NotFound("record not found".to_string()))?;
    compat_repository::save_cache_json(pool, module.store_key(), &Value::Array(rows)).await?;
    Ok(value)
}

pub async fn get_record(pool: &PgPool, module: OrionCompatModule, id: i64) -> AppResult<Value> {
    let rows = list_records(pool, module).await?;
    rows.into_iter()
        .find(|r| r.get("id").and_then(Value::as_i64) == Some(id))
        .ok_or_else(|| AppError::NotFound("record not found".to_string()))
}

pub async fn delete_record(pool: &PgPool, module: OrionCompatModule, id: i64) -> AppResult<u64> {
    let mut rows = list_records(pool, module).await?;
    let before = rows.len();
    rows.retain(|r| r.get("id").and_then(Value::as_i64) != Some(id));
    let deleted = (before.saturating_sub(rows.len())) as u64;
    compat_repository::save_cache_json(pool, module.store_key(), &Value::Array(rows)).await?;
    Ok(deleted)
}

pub async fn batch_delete_records(
    pool: &PgPool,
    module: OrionCompatModule,
    ids: &[i64],
) -> AppResult<u64> {
    let mut rows = list_records(pool, module).await?;
    let before = rows.len();
    rows.retain(|r| {
        let id = r.get("id").and_then(Value::as_i64).unwrap_or_default();
        !ids.contains(&id)
    });
    let deleted = (before.saturating_sub(rows.len())) as u64;
    compat_repository::save_cache_json(pool, module.store_key(), &Value::Array(rows)).await?;
    Ok(deleted)
}

pub async fn clear_records(pool: &PgPool, module: OrionCompatModule) -> AppResult<u64> {
    let rows = list_records(pool, module).await?;
    let count = rows.len() as u64;
    compat_repository::delete_cache_key(pool, module.store_key()).await?;
    Ok(count)
}

pub async fn query_records(
    pool: &PgPool,
    module: OrionCompatModule,
    page_query: PageQuery,
    search_value: Option<&str>,
) -> AppResult<(i64, Vec<Value>)> {
    let mut rows = list_records(pool, module).await?;
    rows.sort_by_key(|r| std::cmp::Reverse(r.get("id").and_then(Value::as_i64).unwrap_or(0)));

    if let Some(search) = search_value.filter(|v| !v.trim().is_empty()) {
        let needle = search.to_ascii_lowercase();
        rows.retain(|row| row.to_string().to_ascii_lowercase().contains(&needle));
    }

    let total = rows.len() as i64;
    let (page, limit) = sanitize_page(page_query.page, page_query.limit);
    let offset = ((page - 1) * limit) as usize;
    let paged = rows.into_iter().skip(offset).take(limit as usize).collect();
    Ok((total, paged))
}

pub async fn get_config_map(pool: &PgPool, key: &str) -> AppResult<Map<String, Value>> {
    let json = compat_repository::load_cache_json(pool, key).await?;
    Ok(json
        .and_then(|v| v.as_object().cloned())
        .unwrap_or_default())
}

pub async fn set_config_map(pool: &PgPool, key: &str, map: &Map<String, Value>) -> AppResult<()> {
    compat_repository::save_cache_json(pool, key, &Value::Object(map.clone())).await
}

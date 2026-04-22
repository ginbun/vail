use std::collections::{HashMap, HashSet};

use serde_json::Value;
use sqlx::PgPool;

use crate::domain::orion::asset::{
    OrionGrantScope, OrionHostGroupAggregate, OrionHostIdentityAggregate, OrionHostKeyAggregate,
};
use crate::error::{AppError, AppResult};
use crate::infrastructure::orion::asset_repository::{
    self, HostIdentityPatch, OrionHostIdentityQueryFilters as RepositoryHostIdentityQueryFilters,
    OrionHostKeyQueryFilters as RepositoryHostKeyQueryFilters,
};
use crate::security;

#[derive(Debug, Default, Clone)]
pub struct OrionHostKeyQueryFilters {
    pub id: Option<i64>,
    pub search_value: Option<String>,
    pub name: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Default, Clone)]
pub struct OrionHostIdentityQueryFilters {
    pub id: Option<i64>,
    pub search_value: Option<String>,
    pub name: Option<String>,
    pub identity_type: Option<String>,
    pub username: Option<String>,
    pub key_id: Option<i64>,
    pub description: Option<String>,
}

#[derive(Debug)]
pub struct OrionHostKeyCreateInput {
    pub name: String,
    pub private_key_ciphertext: String,
    pub passphrase_ciphertext: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug)]
pub struct OrionHostKeyUpdateInput {
    pub id: i64,
    pub name: Option<String>,
    pub private_key_ciphertext: Option<String>,
    pub use_new_password: bool,
    pub passphrase_ciphertext: Option<Option<String>>,
    pub description: Option<String>,
}

#[derive(Debug)]
pub struct OrionHostIdentityCreateInput {
    pub name: String,
    pub identity_type: String,
    pub username: Option<String>,
    pub password_ciphertext: Option<String>,
    pub key_id: Option<i64>,
    pub description: Option<String>,
}

#[derive(Debug)]
pub struct OrionHostIdentityUpdateInput {
    pub id: i64,
    pub name: Option<String>,
    pub identity_type: Option<String>,
    pub username: Option<String>,
    pub key_id: Option<Option<i64>>,
    pub use_new_password: bool,
    pub password_ciphertext: Option<Option<String>>,
    pub description: Option<String>,
}

#[derive(Debug)]
pub struct OrionHostCreateInput {
    pub name: String,
    pub hostname: String,
    pub description: Option<String>,
    pub group_ids: Vec<i64>,
}

#[derive(Debug)]
pub struct OrionHostUpdateInput {
    pub id: i64,
    pub name: Option<String>,
    pub hostname: Option<String>,
    pub description: Option<String>,
    pub group_ids: Vec<i64>,
}

#[derive(Debug, Clone)]
pub struct OrionAuthorizedCurrentHostItem {
    pub id: i64,
    pub name: String,
    pub hostname: String,
    pub port: i32,
    pub create_time_ms: i64,
    pub update_time_ms: i64,
    pub group_ids: Vec<i64>,
}

#[derive(Debug, Clone)]
pub struct OrionAuthorizedCurrentHostData {
    pub group_tree: Vec<OrionHostGroupAggregate>,
    pub host_list: Vec<OrionAuthorizedCurrentHostItem>,
    pub tree_nodes: HashMap<String, Vec<i64>>,
}

#[derive(Debug, Clone)]
pub struct OrionAuthorizedCurrentHostKeyItem {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub create_time_ms: i64,
    pub update_time_ms: i64,
}

#[derive(Debug, Clone)]
pub struct OrionAuthorizedCurrentHostIdentityItem {
    pub id: i64,
    pub name: String,
    pub identity_type: String,
    pub username: Option<String>,
    pub key_id: Option<i64>,
    pub description: Option<String>,
    pub create_time_ms: i64,
    pub update_time_ms: i64,
}

pub fn resolve_grant_scope(
    user_id: Option<i64>,
    role_id: Option<i64>,
) -> AppResult<OrionGrantScope> {
    if let Some(id) = role_id.filter(|v| *v > 0) {
        return Ok(OrionGrantScope::Role(id));
    }
    if let Some(id) = user_id.filter(|v| *v > 0) {
        return Ok(OrionGrantScope::User(id));
    }
    Err(AppError::BadRequest(
        "roleId or userId is required".to_string(),
    ))
}

pub fn normalize_ids(mut ids: Vec<i64>) -> Vec<i64> {
    ids.retain(|v| *v > 0);
    ids.sort_unstable();
    ids.dedup();
    ids
}

pub async fn create_host_key(pool: &PgPool, input: OrionHostKeyCreateInput) -> AppResult<i64> {
    let id = sqlx::query_scalar::<_, i64>(
        "INSERT INTO ssh_key (
            name,
            private_key_ciphertext,
            passphrase_ciphertext,
            description,
            status,
            create_time,
            update_time,
            deleted
        ) VALUES ($1, $2, $3, $4, 1, NOW(), NOW(), 0)
         RETURNING id",
    )
    .bind(input.name)
    .bind(input.private_key_ciphertext)
    .bind(input.passphrase_ciphertext)
    .bind(input.description)
    .fetch_one(pool)
    .await?;
    Ok(id)
}

pub async fn ensure_host_groups_exist(pool: &PgPool, group_ids: &[i64]) -> AppResult<()> {
    if group_ids.is_empty() {
        return Err(AppError::BadRequest("groupIdList is required".to_string()));
    }

    let count = asset_repository::count_host_groups_by_ids(pool, group_ids).await?;
    if count != group_ids.len() as i64 {
        return Err(AppError::BadRequest(
            "groupIdList contains non-existent host group id".to_string(),
        ));
    }

    Ok(())
}

pub async fn create_host(pool: &PgPool, input: OrionHostCreateInput) -> AppResult<i64> {
    let group_ids = normalize_ids(input.group_ids);
    ensure_host_groups_exist(pool, &group_ids).await?;

    asset_repository::create_host_with_groups(
        pool,
        &input.name,
        &input.hostname,
        input.description.as_deref(),
        &group_ids,
    )
    .await
}

pub async fn update_host(pool: &PgPool, input: OrionHostUpdateInput) -> AppResult<()> {
    let group_ids = normalize_ids(input.group_ids);
    ensure_host_groups_exist(pool, &group_ids).await?;

    let rows = asset_repository::update_host_with_groups(
        pool,
        input.id,
        input.name.as_deref(),
        input.hostname.as_deref(),
        input.description.as_deref(),
        &group_ids,
    )
    .await?;

    if rows == 0 {
        return Err(AppError::NotFound("Host not found".to_string()));
    }

    Ok(())
}

pub async fn update_host_status(pool: &PgPool, id: i64, status: i16) -> AppResult<()> {
    let rows = asset_repository::update_host_status(pool, id, status).await?;
    if rows == 0 {
        return Err(AppError::NotFound("Host not found".to_string()));
    }
    Ok(())
}

pub async fn delete_host(pool: &PgPool, id: i64) -> AppResult<()> {
    let rows = asset_repository::soft_delete_host(pool, id).await?;
    if rows == 0 {
        return Err(AppError::NotFound("Host not found".to_string()));
    }
    Ok(())
}

pub fn normalize_host_config_type(raw: Option<String>) -> AppResult<String> {
    let value = raw
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .ok_or_else(|| AppError::BadRequest("type is required".to_string()))?
        .to_ascii_uppercase();
    match value.as_str() {
        "SSH" | "RDP" | "VNC" => Ok(value),
        _ => Err(AppError::BadRequest(
            "type must be one of SSH, RDP, VNC".to_string(),
        )),
    }
}

pub async fn ensure_host_exists(pool: &PgPool, host_id: i64) -> AppResult<()> {
    if !asset_repository::host_exists(pool, host_id).await? {
        return Err(AppError::NotFound("Host not found".to_string()));
    }
    Ok(())
}

fn host_extra_cache_key(user_id: i64, host_id: i64, item: &str) -> String {
    format!(
        "orion:host-extra:user:{user_id}:host:{host_id}:item:{}",
        item.trim().to_ascii_uppercase()
    )
}

fn host_config_cache_key(host_id: i64, config_type: &str) -> String {
    format!(
        "orion:host-config:host:{host_id}:type:{}",
        config_type.trim().to_ascii_uppercase()
    )
}

pub async fn get_host_extra(
    pool: &PgPool,
    user_id: i64,
    host_id: i64,
    item: &str,
) -> AppResult<Value> {
    let cache_key = host_extra_cache_key(user_id, host_id, item);
    Ok(asset_repository::load_cache_json_value(pool, &cache_key)
        .await?
        .unwrap_or_else(|| serde_json::json!({})))
}

pub async fn update_host_extra(
    pool: &PgPool,
    user_id: i64,
    host_id: i64,
    item: &str,
    extra: &Value,
) -> AppResult<()> {
    let cache_key = host_extra_cache_key(user_id, host_id, item);
    asset_repository::save_cache_json_value(pool, &cache_key, extra).await
}

pub async fn get_host_config(pool: &PgPool, host_id: i64, config_type: &str) -> AppResult<Value> {
    let cache_key = host_config_cache_key(host_id, config_type);
    if let Some(value) = asset_repository::load_cache_json_value(pool, &cache_key).await? {
        return Ok(value);
    }

    if config_type != "SSH" {
        return Ok(serde_json::json!({}));
    }

    let row = asset_repository::get_host_ssh_config(pool, host_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Host not found".to_string()))?;

    let auth_type = match row.credential_type.as_deref() {
        Some("ssh_key") => "KEY",
        Some("password") | Some("private_key") => "PASSWORD",
        _ => "PASSWORD",
    };

    Ok(serde_json::json!({
        "port": row.port,
        "username": row.username.unwrap_or_default(),
        "authType": auth_type,
        "keyId": row.key_id,
        "hasPassword": row.has_password,
        "connectTimeout": 30000,
        "charset": "utf-8",
        "fileNameCharset": "utf-8",
        "fileContentCharset": "utf-8"
    }))
}

pub async fn update_host_config(
    pool: &PgPool,
    host_id: i64,
    config_type: &str,
    config: &Value,
    ssh_password_plain: Option<&str>,
    data_encryption_key: &str,
) -> AppResult<()> {
    ensure_host_exists(pool, host_id).await?;

    let db_write_result = if config_type == "SSH" {
        apply_ssh_host_config(
            pool,
            host_id,
            config,
            ssh_password_plain,
            data_encryption_key,
        )
        .await
    } else {
        Ok(())
    };

    if should_update_host_config_cache(config_type, db_write_result.is_ok()) {
        let cache_key = host_config_cache_key(host_id, config_type);
        asset_repository::save_cache_json_value(pool, &cache_key, config).await?;
    }

    db_write_result?;

    Ok(())
}

fn should_update_host_config_cache(config_type: &str, db_write_succeeded: bool) -> bool {
    config_type != "SSH" || db_write_succeeded
}

pub async fn apply_ssh_host_config(
    pool: &PgPool,
    host_id: i64,
    config: &Value,
    password_plain: Option<&str>,
    data_encryption_key: &str,
) -> AppResult<()> {
    let mut tx = pool.begin().await?;

    let username = config
        .get("username")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|v| !v.is_empty());
    let port = config.get("port").and_then(Value::as_i64).map(|v| v as i32);

    asset_repository::update_host_connection_settings_tx(&mut tx, host_id, username, port).await?;

    let auth_type = config
        .get("authType")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .unwrap_or("PASSWORD")
        .to_ascii_uppercase();

    match auth_type.as_str() {
        "KEY" => {
            let key_id = config.get("keyId").and_then(Value::as_i64).ok_or_else(|| {
                AppError::BadRequest("keyId is required when authType=KEY".to_string())
            })?;
            if key_id <= 0 {
                return Err(AppError::BadRequest(
                    "keyId must be greater than 0".to_string(),
                ));
            }
            if !asset_repository::ssh_key_is_active_tx(&mut tx, key_id).await? {
                return Err(AppError::BadRequest(
                    "keyId does not exist or is disabled".to_string(),
                ));
            }

            asset_repository::set_host_credential_to_ssh_key_tx(&mut tx, host_id).await?;
            asset_repository::clear_default_host_ssh_key_binding_tx(&mut tx, host_id).await?;
            asset_repository::upsert_default_host_ssh_key_binding_tx(&mut tx, host_id, key_id)
                .await?;
        }
        "PASSWORD" => {
            let use_new_password = config
                .get("useNewPassword")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            if use_new_password {
                let password = password_plain
                    .map(str::trim)
                    .filter(|v| !v.is_empty())
                    .ok_or_else(|| {
                        AppError::BadRequest(
                            "password is required when useNewPassword=true".to_string(),
                        )
                    })?;
                let payload = serde_json::json!({"kind": "password", "password": password});
                let encrypted =
                    security::encrypt_secret(&payload.to_string(), data_encryption_key)?;
                asset_repository::set_host_credential_to_password_tx(
                    &mut tx,
                    host_id,
                    Some(encrypted.as_str()),
                )
                .await?;
            } else {
                asset_repository::set_host_credential_to_password_tx(&mut tx, host_id, None)
                    .await?;
            }
        }
        "IDENTITY" => {
            let identity_id = config
                .get("identityId")
                .and_then(Value::as_i64)
                .ok_or_else(|| {
                    AppError::BadRequest(
                        "identityId is required when authType=IDENTITY".to_string(),
                    )
                })?;
            if identity_id <= 0 {
                return Err(AppError::BadRequest(
                    "identityId must be greater than 0".to_string(),
                ));
            }

            let identity = asset_repository::get_active_host_identity_tx(&mut tx, identity_id)
                .await?
                .ok_or_else(|| {
                    AppError::BadRequest("identityId does not exist or is disabled".to_string())
                })?;

            if let Some(identity_username) = identity
                .username
                .as_deref()
                .map(str::trim)
                .filter(|v| !v.is_empty())
            {
                asset_repository::update_host_username_tx(&mut tx, host_id, identity_username)
                    .await?;
            }

            match identity.identity_type.as_str() {
                "PASSWORD" => {
                    let ciphertext = identity.password_ciphertext.as_deref().ok_or_else(|| {
                        AppError::BadRequest("identity password is missing".to_string())
                    })?;
                    let password = security::decrypt_secret(ciphertext, data_encryption_key)?;
                    let payload = serde_json::json!({"kind": "password", "password": password});
                    let encrypted =
                        security::encrypt_secret(&payload.to_string(), data_encryption_key)?;
                    asset_repository::set_host_credential_to_password_tx(
                        &mut tx,
                        host_id,
                        Some(encrypted.as_str()),
                    )
                    .await?;
                }
                "KEY" => {
                    let key_id = identity.key_id.ok_or_else(|| {
                        AppError::BadRequest("identity key is missing".to_string())
                    })?;
                    asset_repository::set_host_credential_to_ssh_key_tx(&mut tx, host_id).await?;
                    asset_repository::clear_default_host_ssh_key_binding_tx(&mut tx, host_id)
                        .await?;
                    asset_repository::upsert_default_host_ssh_key_binding_tx(
                        &mut tx, host_id, key_id,
                    )
                    .await?;
                }
                _ => {
                    return Err(AppError::BadRequest(
                        "identity type must be PASSWORD or KEY".to_string(),
                    ))
                }
            }
        }
        _ => {
            return Err(AppError::BadRequest(
                "authType must be one of PASSWORD, KEY, IDENTITY".to_string(),
            ))
        }
    }

    tx.commit().await?;
    Ok(())
}

pub async fn list_authorized_current_hosts(
    pool: &PgPool,
    user_id: i64,
) -> AppResult<OrionAuthorizedCurrentHostData> {
    let authorized_group_ids =
        asset_repository::list_authorized_host_group_ids(pool, user_id).await?;
    if authorized_group_ids.is_empty() {
        return Ok(OrionAuthorizedCurrentHostData {
            group_tree: Vec::new(),
            host_list: Vec::new(),
            tree_nodes: HashMap::new(),
        });
    }

    let group_tree = list_host_groups_for_tree(pool).await?;
    let rows =
        asset_repository::list_authorized_hosts_by_group_ids(pool, &authorized_group_ids).await?;

    let authorized_group_set = authorized_group_ids.into_iter().collect::<HashSet<_>>();
    let mut host_list = Vec::<OrionAuthorizedCurrentHostItem>::new();
    let mut host_index = HashMap::<i64, usize>::new();
    let mut tree_nodes = HashMap::<String, Vec<i64>>::new();

    for row in rows {
        if !authorized_group_set.contains(&row.group_id) {
            continue;
        }
        let index = if let Some(index) = host_index.get(&row.id).copied() {
            index
        } else {
            let index = host_list.len();
            host_list.push(OrionAuthorizedCurrentHostItem {
                id: row.id,
                name: row.name,
                hostname: row.hostname,
                port: row.port,
                create_time_ms: row.create_time_ms,
                update_time_ms: row.update_time_ms,
                group_ids: Vec::new(),
            });
            host_index.insert(row.id, index);
            index
        };

        let host = host_list.get_mut(index).expect("host index must exist");
        if !host.group_ids.contains(&row.group_id) {
            host.group_ids.push(row.group_id);
        }
        tree_nodes
            .entry(row.group_id.to_string())
            .or_default()
            .push(row.id);
    }

    Ok(OrionAuthorizedCurrentHostData {
        group_tree,
        host_list,
        tree_nodes,
    })
}

pub async fn list_authorized_current_host_keys(
    pool: &PgPool,
    user_id: i64,
) -> AppResult<Vec<OrionAuthorizedCurrentHostKeyItem>> {
    let key_ids = asset_repository::list_authorized_host_key_ids(pool, user_id).await?;
    if key_ids.is_empty() {
        return Ok(Vec::new());
    }
    let rows = asset_repository::list_authorized_host_keys_by_ids(pool, &key_ids).await?;
    Ok(rows
        .into_iter()
        .map(|row| OrionAuthorizedCurrentHostKeyItem {
            id: row.id,
            name: row.name,
            description: row.description,
            create_time_ms: row.create_time_ms,
            update_time_ms: row.update_time_ms,
        })
        .collect())
}

pub async fn list_authorized_current_host_identities(
    pool: &PgPool,
    user_id: i64,
) -> AppResult<Vec<OrionAuthorizedCurrentHostIdentityItem>> {
    let identity_ids = asset_repository::list_authorized_host_identity_ids(pool, user_id).await?;
    if identity_ids.is_empty() {
        return Ok(Vec::new());
    }
    let rows =
        asset_repository::list_authorized_host_identities_by_ids(pool, &identity_ids).await?;
    Ok(rows
        .into_iter()
        .map(|row| OrionAuthorizedCurrentHostIdentityItem {
            id: row.id,
            name: row.name,
            identity_type: row.identity_type,
            username: row.username,
            key_id: row.key_id,
            description: row.description,
            create_time_ms: row.create_time_ms,
            update_time_ms: row.update_time_ms,
        })
        .collect())
}

pub async fn list_host_groups_for_tree(pool: &PgPool) -> AppResult<Vec<OrionHostGroupAggregate>> {
    asset_repository::list_host_groups_for_tree(pool).await
}

pub async fn create_host_group(pool: &PgPool, parent_id: i64, name: &str) -> AppResult<i64> {
    if parent_id > 0 && !asset_repository::host_group_exists(pool, parent_id).await? {
        return Err(AppError::NotFound(
            "Parent host group not found".to_string(),
        ));
    }

    asset_repository::create_host_group(pool, name, parent_id).await
}

pub async fn rename_host_group(pool: &PgPool, id: i64, name: &str) -> AppResult<()> {
    let rows = asset_repository::rename_host_group(pool, id, name).await?;
    if rows == 0 {
        return Err(AppError::NotFound("Host group not found".to_string()));
    }
    Ok(())
}

pub async fn move_host_group(
    pool: &PgPool,
    id: i64,
    target_id: i64,
    position: i32,
) -> AppResult<()> {
    if target_id > 0 && !asset_repository::host_group_exists(pool, target_id).await? {
        return Err(AppError::NotFound(
            "Target host group not found".to_string(),
        ));
    }

    let rows = asset_repository::move_host_group(pool, id, target_id, position).await?;
    if rows == 0 {
        return Err(AppError::NotFound("Host group not found".to_string()));
    }
    Ok(())
}

pub async fn delete_host_group(pool: &PgPool, id: i64) -> AppResult<()> {
    if asset_repository::host_group_has_children(pool, id).await? {
        return Err(AppError::BadRequest(
            "cannot delete host group with children".to_string(),
        ));
    }

    let rows = asset_repository::soft_delete_host_group(pool, id).await?;
    if rows == 0 {
        return Err(AppError::NotFound("Host group not found".to_string()));
    }
    Ok(())
}

pub async fn list_host_group_rel_host_ids(pool: &PgPool, group_id: i64) -> AppResult<Vec<i64>> {
    asset_repository::list_host_group_rel_host_ids(pool, group_id).await
}

pub async fn replace_host_group_rel(
    pool: &PgPool,
    group_id: i64,
    host_ids: Vec<i64>,
) -> AppResult<()> {
    if !asset_repository::host_group_exists(pool, group_id).await? {
        return Err(AppError::NotFound("Host group not found".to_string()));
    }

    let host_ids = normalize_ids(host_ids);
    if !host_ids.is_empty() {
        let existing_count = asset_repository::count_hosts_by_ids(pool, &host_ids).await?;
        if existing_count != host_ids.len() as i64 {
            return Err(AppError::BadRequest(
                "hostIdList contains non-existent host id".to_string(),
            ));
        }
    }

    asset_repository::replace_host_group_rel(pool, group_id, &host_ids).await
}

pub async fn update_host_key(pool: &PgPool, input: OrionHostKeyUpdateInput) -> AppResult<()> {
    let result = sqlx::query(
        "UPDATE ssh_key SET
            name = COALESCE($2, name),
            description = COALESCE($3, description),
            private_key_ciphertext = COALESCE($4, private_key_ciphertext),
            passphrase_ciphertext = CASE WHEN $5 THEN $6 ELSE passphrase_ciphertext END,
            update_time = NOW()
         WHERE id = $1 AND deleted = 0",
    )
    .bind(input.id)
    .bind(input.name)
    .bind(input.description)
    .bind(input.private_key_ciphertext)
    .bind(input.use_new_password)
    .bind(input.passphrase_ciphertext.flatten())
    .execute(pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("Host key not found".to_string()));
    }
    Ok(())
}

pub async fn get_host_key(pool: &PgPool, id: i64) -> AppResult<OrionHostKeyAggregate> {
    asset_repository::get_host_key_by_id(pool, id)
        .await?
        .ok_or_else(|| AppError::NotFound("Host key not found".to_string()))
}

pub async fn list_host_keys(pool: &PgPool) -> AppResult<Vec<OrionHostKeyAggregate>> {
    asset_repository::list_host_keys(pool).await
}

pub async fn query_host_keys(
    pool: &PgPool,
    filters: OrionHostKeyQueryFilters,
    offset: i64,
    limit: i64,
) -> AppResult<(i64, Vec<OrionHostKeyAggregate>)> {
    let repository_filters = to_repository_host_key_filters(&filters);
    let total = asset_repository::count_host_keys(pool, &repository_filters).await?;
    let rows = asset_repository::query_host_keys(pool, &repository_filters, offset, limit).await?;
    Ok((total, rows))
}

pub async fn delete_host_key(pool: &PgPool, id: i64) -> AppResult<()> {
    let affected = asset_repository::soft_delete_host_key(pool, id).await?;
    if affected == 0 {
        return Err(AppError::NotFound("Host key not found".to_string()));
    }
    Ok(())
}

pub async fn batch_delete_host_keys(pool: &PgPool, ids: Vec<i64>) -> AppResult<()> {
    let ids = normalize_ids(ids);
    if ids.is_empty() {
        return Err(AppError::BadRequest("idList is invalid".to_string()));
    }
    asset_repository::soft_delete_host_keys(pool, &ids).await?;
    Ok(())
}

pub async fn create_host_identity(
    pool: &PgPool,
    input: OrionHostIdentityCreateInput,
) -> AppResult<i64> {
    asset_repository::create_host_identity(
        pool,
        &input.name,
        &input.identity_type,
        input.username.as_deref(),
        input.password_ciphertext.as_deref(),
        input.key_id,
        input.description.as_deref(),
    )
    .await
}

pub async fn update_host_identity(
    pool: &PgPool,
    input: OrionHostIdentityUpdateInput,
) -> AppResult<()> {
    let affected = asset_repository::update_host_identity(
        pool,
        HostIdentityPatch {
            id: input.id,
            name: input.name.as_deref(),
            identity_type: input.identity_type.as_deref(),
            username: input.username.as_deref(),
            key_id: input.key_id,
            description: input.description.as_deref(),
            use_new_password: input.use_new_password,
            password_ciphertext: input.password_ciphertext.as_ref().map(|x| x.as_deref()),
        },
    )
    .await?;

    if affected == 0 {
        return Err(AppError::NotFound("Host identity not found".to_string()));
    }
    Ok(())
}

pub async fn get_host_identity(pool: &PgPool, id: i64) -> AppResult<OrionHostIdentityAggregate> {
    asset_repository::get_host_identity_by_id(pool, id)
        .await?
        .ok_or_else(|| AppError::NotFound("Host identity not found".to_string()))
}

pub async fn list_host_identities(pool: &PgPool) -> AppResult<Vec<OrionHostIdentityAggregate>> {
    asset_repository::list_host_identities(pool).await
}

pub async fn query_host_identities(
    pool: &PgPool,
    filters: OrionHostIdentityQueryFilters,
    offset: i64,
    limit: i64,
) -> AppResult<(i64, Vec<OrionHostIdentityAggregate>)> {
    let repository_filters = to_repository_host_identity_filters(&filters);
    let total = asset_repository::count_host_identities(pool, &repository_filters).await?;
    let rows =
        asset_repository::query_host_identities(pool, &repository_filters, offset, limit).await?;
    Ok((total, rows))
}

pub async fn delete_host_identity(pool: &PgPool, id: i64) -> AppResult<()> {
    let affected = asset_repository::soft_delete_host_identity(pool, id).await?;
    if affected == 0 {
        return Err(AppError::NotFound("Host identity not found".to_string()));
    }
    Ok(())
}

pub async fn batch_delete_host_identities(pool: &PgPool, ids: Vec<i64>) -> AppResult<()> {
    let ids = normalize_ids(ids);
    if ids.is_empty() {
        return Err(AppError::BadRequest("idList is invalid".to_string()));
    }
    asset_repository::soft_delete_host_identities(pool, &ids).await?;
    Ok(())
}

pub async fn replace_asset_grants(
    pool: &PgPool,
    scope: OrionGrantScope,
    resource: &str,
    ids: Vec<i64>,
) -> AppResult<()> {
    asset_repository::replace_asset_grants(pool, scope, resource, &normalize_ids(ids)).await
}

pub async fn list_asset_grants(
    pool: &PgPool,
    scope: OrionGrantScope,
    resource: &str,
) -> AppResult<Vec<i64>> {
    asset_repository::list_asset_grants(pool, scope, resource).await
}

fn to_repository_host_key_filters(
    filters: &OrionHostKeyQueryFilters,
) -> RepositoryHostKeyQueryFilters {
    RepositoryHostKeyQueryFilters {
        id: filters.id,
        search_value: filters.search_value.clone(),
        name: filters.name.clone(),
        description: filters.description.clone(),
    }
}

fn to_repository_host_identity_filters(
    filters: &OrionHostIdentityQueryFilters,
) -> RepositoryHostIdentityQueryFilters {
    RepositoryHostIdentityQueryFilters {
        id: filters.id,
        search_value: filters.search_value.clone(),
        name: filters.name.clone(),
        identity_type: filters.identity_type.clone(),
        username: filters.username.clone(),
        key_id: filters.key_id,
        description: filters.description.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        normalize_host_config_type, normalize_ids, resolve_grant_scope,
        should_update_host_config_cache, to_repository_host_identity_filters,
        to_repository_host_key_filters, OrionGrantScope, OrionHostIdentityQueryFilters,
        OrionHostKeyQueryFilters,
    };

    #[test]
    fn converts_host_key_query_filters_to_repository_type() {
        let filters = OrionHostKeyQueryFilters {
            id: Some(9),
            search_value: Some("rsa".to_string()),
            name: Some("deploy".to_string()),
            description: Some("ci".to_string()),
        };

        let mapped = to_repository_host_key_filters(&filters);
        assert_eq!(mapped.id, Some(9));
        assert_eq!(mapped.search_value.as_deref(), Some("rsa"));
        assert_eq!(mapped.name.as_deref(), Some("deploy"));
        assert_eq!(mapped.description.as_deref(), Some("ci"));
    }

    #[test]
    fn converts_host_identity_query_filters_to_repository_type() {
        let filters = OrionHostIdentityQueryFilters {
            id: Some(11),
            search_value: Some("db".to_string()),
            name: Some("jump".to_string()),
            identity_type: Some("PASSWORD".to_string()),
            username: Some("root".to_string()),
            key_id: Some(3),
            description: Some("primary".to_string()),
        };

        let mapped = to_repository_host_identity_filters(&filters);
        assert_eq!(mapped.id, Some(11));
        assert_eq!(mapped.search_value.as_deref(), Some("db"));
        assert_eq!(mapped.name.as_deref(), Some("jump"));
        assert_eq!(mapped.identity_type.as_deref(), Some("PASSWORD"));
        assert_eq!(mapped.username.as_deref(), Some("root"));
        assert_eq!(mapped.key_id, Some(3));
        assert_eq!(mapped.description.as_deref(), Some("primary"));
    }

    #[test]
    fn normalize_ids_keeps_positive_unique_sorted_values() {
        let normalized = normalize_ids(vec![3, -1, 2, 3, 0, 5, 2]);
        assert_eq!(normalized, vec![2, 3, 5]);
    }

    #[test]
    fn resolve_grant_scope_prefers_role_scope() {
        let scope = resolve_grant_scope(Some(7), Some(9)).expect("scope should resolve");
        match scope {
            OrionGrantScope::Role(id) => assert_eq!(id, 9),
            OrionGrantScope::User(_) => panic!("expected role scope"),
        }
    }

    #[test]
    fn resolve_grant_scope_requires_positive_ids() {
        let err = resolve_grant_scope(Some(0), Some(-1)).expect_err("scope should fail");
        assert!(err.to_string().contains("roleId or userId is required"));
    }

    #[test]
    fn normalize_host_config_type_accepts_known_values() {
        assert_eq!(
            normalize_host_config_type(Some(" ssh ".to_string())).expect("ssh should normalize"),
            "SSH"
        );
        assert_eq!(
            normalize_host_config_type(Some("Rdp".to_string())).expect("rdp should normalize"),
            "RDP"
        );
    }

    #[test]
    fn normalize_host_config_type_rejects_invalid_or_missing_values() {
        let missing = normalize_host_config_type(None).expect_err("missing type should fail");
        assert!(missing.to_string().contains("type is required"));

        let invalid = normalize_host_config_type(Some("telnet".to_string()))
            .expect_err("invalid type should fail");
        assert!(invalid
            .to_string()
            .contains("type must be one of SSH, RDP, VNC"));
    }

    #[test]
    fn should_update_host_config_cache_requires_success_for_ssh() {
        assert!(should_update_host_config_cache("SSH", true));
        assert!(!should_update_host_config_cache("SSH", false));
    }

    #[test]
    fn should_update_host_config_cache_allows_non_ssh_without_db_write() {
        assert!(should_update_host_config_cache("RDP", false));
        assert!(should_update_host_config_cache("VNC", false));
    }
}

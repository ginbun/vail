use sqlx::PgPool;

use crate::domain::orion::asset::{
    OrionGrantScope, OrionHostIdentityAggregate, OrionHostKeyAggregate,
};
use crate::error::{AppError, AppResult};
use crate::infrastructure::orion::asset_repository::{
    self, HostIdentityPatch, OrionHostIdentityQueryFilters as RepositoryHostIdentityQueryFilters,
    OrionHostKeyQueryFilters as RepositoryHostKeyQueryFilters,
};

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
        to_repository_host_identity_filters, to_repository_host_key_filters,
        OrionHostIdentityQueryFilters, OrionHostKeyQueryFilters,
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
}

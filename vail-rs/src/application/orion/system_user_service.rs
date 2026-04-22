use sqlx::PgPool;

use crate::{
    domain::orion::system_user::OrionSystemUserAggregate,
    infrastructure::orion::system_user_repository::{
        self, OrionActiveSessionRow, OrionSystemUserLoginHistoryRow,
        OrionSystemUserQueryFilters as RepositorySystemUserQueryFilters,
    },
};

#[derive(Debug, Clone, Default)]
pub struct OrionSystemUserQueryFilters {
    pub id: Option<i64>,
    pub username: Option<String>,
    pub nickname: Option<String>,
    pub mobile: Option<String>,
    pub email: Option<String>,
    pub status: Option<i16>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct OrionSystemUserLoginHistoryEntry {
    pub id: i64,
    pub address: String,
    pub location: String,
    pub user_agent: String,
    pub result: i16,
    pub error_message: String,
    pub create_time: i64,
}

#[derive(Debug, Clone)]
pub struct OrionActiveSessionEntry {
    pub id: i64,
    pub user_id: i64,
    pub username: String,
    pub address: String,
    pub location: String,
    pub user_agent: String,
    pub login_time: i64,
}

#[derive(Debug, Clone)]
pub struct OrionMineSessionEntry {
    pub id: i64,
    pub username: String,
    pub visible: bool,
    pub current: bool,
    pub address: String,
    pub location: String,
    pub user_agent: String,
    pub login_time: i64,
    pub offline: bool,
}

#[derive(Debug, Clone)]
pub struct OrionSystemUserCreateInput {
    pub username: String,
    pub password_hash: String,
    pub nickname: Option<String>,
    pub avatar: Option<String>,
    pub mobile: Option<String>,
    pub email: Option<String>,
}

#[derive(Debug, Clone)]
pub struct OrionSystemUserUpdateInput {
    pub id: i64,
    pub username: Option<String>,
    pub nickname: Option<String>,
    pub avatar: Option<String>,
    pub mobile: Option<String>,
    pub email: Option<String>,
}

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
    system_user_repository::query_system_users(pool, &to_repository_filters(&filters)).await
}

pub async fn count_system_users(
    pool: &PgPool,
    filters: OrionSystemUserQueryFilters,
) -> Result<i64, sqlx::Error> {
    system_user_repository::count_system_users(pool, &to_repository_filters(&filters)).await
}

pub async fn soft_delete_system_user(pool: &PgPool, id: i64) -> Result<u64, sqlx::Error> {
    system_user_repository::soft_delete_system_user(pool, id).await
}

pub async fn soft_delete_system_users(pool: &PgPool, ids: Vec<i64>) -> Result<u64, sqlx::Error> {
    let ids = normalize_ids(ids);
    if ids.is_empty() {
        return Ok(0);
    }
    system_user_repository::soft_delete_system_users(pool, &ids).await
}

pub async fn list_system_user_role_ids(
    pool: &PgPool,
    user_id: i64,
) -> Result<Vec<i64>, sqlx::Error> {
    system_user_repository::list_system_user_role_ids(pool, user_id).await
}

pub async fn create_system_user(
    pool: &PgPool,
    input: OrionSystemUserCreateInput,
) -> Result<i64, sqlx::Error> {
    system_user_repository::create_system_user(
        pool,
        &input.username,
        &input.password_hash,
        input.nickname.as_deref(),
        input.avatar.as_deref(),
        input.mobile.as_deref(),
        input.email.as_deref(),
    )
    .await
}

pub async fn update_system_user(
    pool: &PgPool,
    input: OrionSystemUserUpdateInput,
) -> Result<u64, sqlx::Error> {
    system_user_repository::update_system_user(
        pool,
        input.id,
        input.username.as_deref(),
        input.nickname.as_deref(),
        input.avatar.as_deref(),
        input.mobile.as_deref(),
        input.email.as_deref(),
    )
    .await
}

pub async fn update_system_user_status(
    pool: &PgPool,
    id: i64,
    status: i16,
) -> Result<u64, sqlx::Error> {
    system_user_repository::update_system_user_status(pool, id, status).await
}

pub async fn replace_system_user_roles(
    pool: &PgPool,
    user_id: i64,
    role_ids: Vec<i64>,
) -> Result<(), sqlx::Error> {
    let role_ids = normalize_ids(role_ids);
    system_user_repository::replace_system_user_roles(pool, user_id, &role_ids).await
}

pub async fn update_system_user_password(
    pool: &PgPool,
    id: i64,
    password_hash: &str,
) -> Result<u64, sqlx::Error> {
    system_user_repository::update_system_user_password(pool, id, password_hash).await
}

pub async fn get_system_user_password_hash(
    pool: &PgPool,
    user_id: i64,
) -> Result<Option<String>, sqlx::Error> {
    system_user_repository::get_system_user_password_hash(pool, user_id).await
}

pub async fn list_login_history_by_username(
    pool: &PgPool,
    username: &str,
    limit: i64,
) -> Result<Vec<OrionSystemUserLoginHistoryEntry>, sqlx::Error> {
    let rows =
        system_user_repository::list_login_history_by_username(pool, username, limit).await?;
    Ok(rows.into_iter().map(Into::into).collect())
}

pub async fn list_login_history_by_user_id(
    pool: &PgPool,
    user_id: i64,
    limit: i64,
) -> Result<Vec<OrionSystemUserLoginHistoryEntry>, sqlx::Error> {
    let rows = system_user_repository::list_login_history_by_user_id(pool, user_id, limit).await?;
    Ok(rows.into_iter().map(Into::into).collect())
}

pub async fn revoke_refresh_tokens_by_user_and_timestamp(
    pool: &PgPool,
    user_id: i64,
    timestamp: i64,
) -> Result<u64, sqlx::Error> {
    system_user_repository::revoke_refresh_tokens_by_user_and_timestamp(pool, user_id, timestamp)
        .await
}

pub async fn list_active_sessions(
    pool: &PgPool,
    limit: i64,
) -> Result<Vec<OrionActiveSessionEntry>, sqlx::Error> {
    let rows = system_user_repository::list_active_sessions(pool, limit).await?;
    Ok(rows.into_iter().map(Into::into).collect())
}

pub async fn list_active_sessions_by_user(
    pool: &PgPool,
    user_id: i64,
    limit: i64,
) -> Result<Vec<OrionActiveSessionEntry>, sqlx::Error> {
    let rows = system_user_repository::list_active_sessions_by_user(pool, user_id, limit).await?;
    Ok(rows.into_iter().map(Into::into).collect())
}

pub async fn list_mine_sessions(
    pool: &PgPool,
    user_id: i64,
    limit: i64,
) -> Result<Vec<OrionMineSessionEntry>, sqlx::Error> {
    let rows = system_user_repository::list_mine_sessions(pool, user_id, limit).await?;
    Ok(rows
        .into_iter()
        .map(|row| {
            let (current, offline) =
                map_mine_session_row(&row.session_id, row.revoked_at.as_deref());
            OrionMineSessionEntry {
                id: row.id,
                username: user_id.to_string(),
                visible: true,
                current,
                address: row.address.unwrap_or_default(),
                location: row.location.unwrap_or_default(),
                user_agent: row.user_agent.unwrap_or_default(),
                login_time: row.login_time,
                offline,
            }
        })
        .collect())
}

pub async fn revoke_active_refresh_tokens_by_user_and_timestamp(
    pool: &PgPool,
    user_id: i64,
    timestamp: i64,
) -> Result<u64, sqlx::Error> {
    system_user_repository::revoke_active_refresh_tokens_by_user_and_timestamp(
        pool, user_id, timestamp,
    )
    .await
}

fn to_repository_filters(
    filters: &OrionSystemUserQueryFilters,
) -> RepositorySystemUserQueryFilters {
    RepositorySystemUserQueryFilters {
        id: filters.id,
        username: filters.username.clone(),
        nickname: filters.nickname.clone(),
        mobile: filters.mobile.clone(),
        email: filters.email.clone(),
        status: filters.status,
        limit: filters.limit,
        offset: filters.offset,
    }
}

fn normalize_ids(mut ids: Vec<i64>) -> Vec<i64> {
    ids.retain(|v| *v > 0);
    ids.sort_unstable();
    ids.dedup();
    ids
}

impl From<OrionSystemUserLoginHistoryRow> for OrionSystemUserLoginHistoryEntry {
    fn from(value: OrionSystemUserLoginHistoryRow) -> Self {
        Self {
            id: value.id,
            address: value.address.unwrap_or_default(),
            location: value.location.unwrap_or_default(),
            user_agent: value.user_agent.unwrap_or_default(),
            result: value.result,
            error_message: value.error_message.unwrap_or_default(),
            create_time: value.create_time,
        }
    }
}

impl From<OrionActiveSessionRow> for OrionActiveSessionEntry {
    fn from(value: OrionActiveSessionRow) -> Self {
        Self {
            id: value.id,
            user_id: value.user_id,
            username: value.username,
            address: value.address.unwrap_or_default(),
            location: value.location.unwrap_or_default(),
            user_agent: value.user_agent.unwrap_or_default(),
            login_time: value.login_time,
        }
    }
}

fn map_mine_session_row(session_id: &str, revoked_at: Option<&str>) -> (bool, bool) {
    (session_id.contains('-'), revoked_at.is_some())
}

#[cfg(test)]
mod tests {
    use super::{
        map_mine_session_row, normalize_ids, to_repository_filters, OrionActiveSessionEntry,
        OrionSystemUserQueryFilters,
    };
    use crate::infrastructure::orion::system_user_repository::OrionActiveSessionRow;

    #[test]
    fn converts_system_user_query_filters_to_repository_type() {
        let filters = OrionSystemUserQueryFilters {
            id: Some(42),
            username: Some("alice".to_string()),
            nickname: Some("ops".to_string()),
            mobile: Some("138".to_string()),
            email: Some("a@vail.dev".to_string()),
            status: Some(1),
            limit: Some(20),
            offset: Some(40),
        };

        let mapped = to_repository_filters(&filters);
        assert_eq!(mapped.id, Some(42));
        assert_eq!(mapped.username.as_deref(), Some("alice"));
        assert_eq!(mapped.nickname.as_deref(), Some("ops"));
        assert_eq!(mapped.mobile.as_deref(), Some("138"));
        assert_eq!(mapped.email.as_deref(), Some("a@vail.dev"));
        assert_eq!(mapped.status, Some(1));
        assert_eq!(mapped.limit, Some(20));
        assert_eq!(mapped.offset, Some(40));
    }

    #[test]
    fn normalize_ids_drops_invalid_and_duplicates() {
        let ids = normalize_ids(vec![3, 0, -1, 3, 2, 2]);
        assert_eq!(ids, vec![2, 3]);
    }

    #[test]
    fn active_session_row_mapping_defaults_optional_fields() {
        let row = OrionActiveSessionRow {
            id: 1,
            user_id: 2,
            login_time: 123,
            address: None,
            location: None,
            user_agent: None,
            username: "ops".to_string(),
        };

        let mapped: OrionActiveSessionEntry = row.into();
        assert_eq!(mapped.id, 1);
        assert_eq!(mapped.user_id, 2);
        assert_eq!(mapped.username, "ops");
        assert_eq!(mapped.address, "");
        assert_eq!(mapped.location, "");
        assert_eq!(mapped.user_agent, "");
        assert_eq!(mapped.login_time, 123);
    }

    #[test]
    fn mine_session_row_mapping_sets_current_and_offline_flags() {
        let (current, offline) = map_mine_session_row("abc-def", Some("2026-01-01T00:00:00Z"));
        assert!(current);
        assert!(offline);
    }
}

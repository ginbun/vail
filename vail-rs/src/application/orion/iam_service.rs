use std::collections::HashSet;

use sqlx::PgPool;

use crate::infrastructure::orion::iam_repository::{self, OrionUserMenuRow};

#[derive(Debug, Clone)]
pub struct OrionLoginUser {
    pub id: i64,
    pub username: String,
    pub password_hash: String,
}

#[derive(Debug, Clone)]
pub struct OrionUserAggregatePrincipal {
    pub id: i64,
    pub username: String,
    pub nickname: String,
    pub avatar: String,
}

#[derive(Debug, Clone)]
pub struct OrionCurrentUserProfile {
    pub id: i64,
    pub username: String,
    pub nickname: String,
    pub avatar: String,
    pub mobile: String,
    pub email: String,
}

#[derive(Debug, Clone)]
pub struct OrionCurrentUserUpdateInput {
    pub user_id: i64,
    pub nickname: Option<String>,
    pub avatar: Option<String>,
    pub mobile: Option<String>,
    pub email: Option<String>,
}

#[derive(Debug, Clone)]
pub struct OrionUserMenuEntry {
    pub id: i64,
    pub parent_id: i64,
    pub name: String,
    pub permission: String,
    pub menu_type: i16,
    pub sort: i32,
    pub visible: i16,
    pub icon: String,
    pub path: String,
    pub component: String,
}

#[derive(Debug, Clone)]
pub struct OrionUserRoleEntry {
    pub id: i64,
    pub name: String,
    pub code: String,
    pub status: i16,
    pub description: String,
}

pub async fn get_login_user_by_username(
    pool: &PgPool,
    username: &str,
) -> Result<Option<OrionLoginUser>, sqlx::Error> {
    let row = iam_repository::get_login_user_by_username(pool, username).await?;
    Ok(row.map(|v| OrionLoginUser {
        id: v.id,
        username: v.username,
        password_hash: v.password_hash,
    }))
}

pub async fn insert_login_log(
    pool: &PgPool,
    user_id: Option<i64>,
    username: &str,
    ip: &str,
    result: i16,
    error_message: Option<&str>,
) -> Result<(), sqlx::Error> {
    iam_repository::insert_login_log(pool, user_id, username, ip, result, error_message).await
}

pub async fn insert_refresh_token(
    pool: &PgPool,
    user_id: i64,
    token_hash: &str,
    session_id: &str,
    expiration_seconds: i64,
    ip: &str,
    user_agent: Option<&str>,
) -> Result<(), sqlx::Error> {
    iam_repository::insert_refresh_token(
        pool,
        user_id,
        token_hash,
        session_id,
        expiration_seconds,
        ip,
        user_agent,
    )
    .await
}

pub async fn update_user_last_login(
    pool: &PgPool,
    user_id: i64,
    ip: &str,
) -> Result<(), sqlx::Error> {
    iam_repository::update_user_last_login(pool, user_id, ip).await
}

pub async fn revoke_active_refresh_tokens(pool: &PgPool, user_id: i64) -> Result<(), sqlx::Error> {
    iam_repository::revoke_active_refresh_tokens(pool, user_id).await
}

pub async fn get_user_aggregate_principal_by_id(
    pool: &PgPool,
    user_id: i64,
) -> Result<Option<OrionUserAggregatePrincipal>, sqlx::Error> {
    let row = iam_repository::get_user_aggregate_by_id(pool, user_id).await?;
    Ok(row.map(|v| OrionUserAggregatePrincipal {
        id: v.id,
        username: v.username,
        nickname: v.nickname.unwrap_or_default(),
        avatar: v.avatar.unwrap_or_default(),
    }))
}

pub async fn get_current_user_profile_by_id(
    pool: &PgPool,
    user_id: i64,
) -> Result<Option<OrionCurrentUserProfile>, sqlx::Error> {
    let row = iam_repository::get_current_user_profile_by_id(pool, user_id).await?;
    Ok(row.map(|v| OrionCurrentUserProfile {
        id: v.id,
        username: v.username,
        nickname: v.nickname.unwrap_or_default(),
        avatar: v.avatar.unwrap_or_default(),
        mobile: v.mobile.unwrap_or_default(),
        email: v.email.unwrap_or_default(),
    }))
}

pub async fn update_current_user_profile(
    pool: &PgPool,
    input: OrionCurrentUserUpdateInput,
) -> Result<u64, sqlx::Error> {
    iam_repository::update_current_user_profile(
        pool,
        input.user_id,
        input.nickname.as_deref(),
        input.avatar.as_deref(),
        input.mobile.as_deref(),
        input.email.as_deref(),
    )
    .await
}

pub async fn list_role_codes_by_user_id(
    pool: &PgPool,
    user_id: i64,
) -> Result<Vec<String>, sqlx::Error> {
    iam_repository::list_role_codes_by_user_id(pool, user_id).await
}

pub async fn list_permissions_by_user_id(
    pool: &PgPool,
    user_id: i64,
) -> Result<Vec<String>, sqlx::Error> {
    let role_permissions = iam_repository::list_role_permissions_by_user_id(pool, user_id).await?;
    let menu_permissions = iam_repository::list_menu_permissions_by_user_id(pool, user_id).await?;
    Ok(merge_permissions(role_permissions, menu_permissions))
}

pub async fn list_user_menu_entries_by_user_id(
    pool: &PgPool,
    user_id: i64,
) -> Result<Vec<OrionUserMenuEntry>, sqlx::Error> {
    let rows = iam_repository::list_user_menus_by_user_id(pool, user_id).await?;
    Ok(rows.into_iter().map(map_user_menu_row).collect())
}

pub async fn list_user_roles_by_user_id(
    pool: &PgPool,
    user_id: i64,
) -> Result<Vec<OrionUserRoleEntry>, sqlx::Error> {
    let rows = iam_repository::list_user_roles_by_user_id(pool, user_id).await?;
    Ok(rows
        .into_iter()
        .map(|v| OrionUserRoleEntry {
            id: v.id,
            name: v.name,
            code: v.code,
            status: v.status,
            description: v.description.unwrap_or_default(),
        })
        .collect())
}

fn map_user_menu_row(row: OrionUserMenuRow) -> OrionUserMenuEntry {
    OrionUserMenuEntry {
        id: row.id,
        parent_id: row.parent_id,
        name: row.name,
        permission: row.permission.unwrap_or_default(),
        menu_type: row.menu_type,
        sort: row.sort,
        visible: row.visible,
        icon: row.icon.unwrap_or_default(),
        path: row.path.unwrap_or_default(),
        component: row.component.unwrap_or_default(),
    }
}

fn merge_permissions(role_permissions: Vec<String>, menu_permissions: Vec<String>) -> Vec<String> {
    let mut merged = role_permissions
        .into_iter()
        .chain(menu_permissions)
        .collect::<HashSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    merged.sort();
    merged
}

#[cfg(test)]
mod tests {
    use super::{map_user_menu_row, merge_permissions};
    use crate::infrastructure::orion::iam_repository::OrionUserMenuRow;

    #[test]
    fn merge_permissions_deduplicates_and_sorts() {
        let merged = merge_permissions(
            vec!["host.read".to_string(), "host.update".to_string()],
            vec!["host.update".to_string(), "iam.user.create".to_string()],
        );
        assert_eq!(
            merged,
            vec![
                "host.read".to_string(),
                "host.update".to_string(),
                "iam.user.create".to_string()
            ]
        );
    }

    #[test]
    fn map_user_menu_row_defaults_optional_fields() {
        let mapped = map_user_menu_row(OrionUserMenuRow {
            id: 9,
            parent_id: 0,
            name: "Dashboard".to_string(),
            permission: None,
            menu_type: 2,
            sort: 1,
            visible: 1,
            icon: None,
            path: None,
            component: None,
        });

        assert_eq!(mapped.id, 9);
        assert_eq!(mapped.permission, "");
        assert_eq!(mapped.icon, "");
        assert_eq!(mapped.path, "");
        assert_eq!(mapped.component, "");
    }
}

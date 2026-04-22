use sqlx::PgPool;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct OrionLoginUserRow {
    pub id: i64,
    pub username: String,
    pub password_hash: String,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct OrionUserAggregateRow {
    pub id: i64,
    pub username: String,
    pub nickname: Option<String>,
    pub avatar: Option<String>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct OrionCurrentUserProfileRow {
    pub id: i64,
    pub username: String,
    pub nickname: Option<String>,
    pub avatar: Option<String>,
    pub mobile: Option<String>,
    pub email: Option<String>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct OrionUserMenuRow {
    pub id: i64,
    pub parent_id: i64,
    pub name: String,
    pub permission: Option<String>,
    pub menu_type: i16,
    pub sort: i32,
    pub visible: i16,
    pub icon: Option<String>,
    pub path: Option<String>,
    pub component: Option<String>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct OrionUserRoleRow {
    pub id: i64,
    pub name: String,
    pub code: String,
    pub status: i16,
    pub description: Option<String>,
}

pub async fn get_login_user_by_username(
    pool: &PgPool,
    username: &str,
) -> Result<Option<OrionLoginUserRow>, sqlx::Error> {
    sqlx::query_as::<_, OrionLoginUserRow>(
        "SELECT id, username, password AS password_hash
         FROM sys_user
         WHERE username = $1 AND deleted = 0",
    )
    .bind(username)
    .fetch_optional(pool)
    .await
}

pub async fn insert_login_log(
    pool: &PgPool,
    user_id: Option<i64>,
    username: &str,
    ip: &str,
    result: i16,
    error_message: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO login_log (user_id, username, ip, result, error_message, create_time)
         VALUES ($1, $2, $3, $4, $5, NOW())",
    )
    .bind(user_id)
    .bind(username)
    .bind(ip)
    .bind(result)
    .bind(error_message)
    .execute(pool)
    .await?;
    Ok(())
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
    sqlx::query(
        "INSERT INTO auth_refresh_token (user_id, token_hash, session_id, expires_at, ip, user_agent)
         VALUES ($1, $2, $3::uuid, NOW() + ($4 || ' seconds')::interval, $5, $6)",
    )
    .bind(user_id)
    .bind(token_hash)
    .bind(session_id)
    .bind(expiration_seconds)
    .bind(ip)
    .bind(user_agent)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn update_user_last_login(
    pool: &PgPool,
    user_id: i64,
    ip: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE sys_user SET last_login_time = NOW(), last_login_ip = $1 WHERE id = $2")
        .bind(ip)
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn revoke_active_refresh_tokens(pool: &PgPool, user_id: i64) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE auth_refresh_token SET revoked_at = NOW() WHERE user_id = $1 AND revoked_at IS NULL")
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn get_user_aggregate_by_id(
    pool: &PgPool,
    user_id: i64,
) -> Result<Option<OrionUserAggregateRow>, sqlx::Error> {
    sqlx::query_as::<_, OrionUserAggregateRow>(
        "SELECT id, username, nickname, avatar
         FROM sys_user
         WHERE id = $1 AND deleted = 0",
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await
}

pub async fn get_current_user_profile_by_id(
    pool: &PgPool,
    user_id: i64,
) -> Result<Option<OrionCurrentUserProfileRow>, sqlx::Error> {
    sqlx::query_as::<_, OrionCurrentUserProfileRow>(
        "SELECT id, username, nickname, avatar, phone AS mobile, email
         FROM sys_user
         WHERE id = $1 AND deleted = 0",
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await
}

pub async fn update_current_user_profile(
    pool: &PgPool,
    user_id: i64,
    nickname: Option<&str>,
    avatar: Option<&str>,
    mobile: Option<&str>,
    email: Option<&str>,
) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE sys_user
         SET nickname = COALESCE($1, nickname),
             avatar = COALESCE($2, avatar),
             phone = COALESCE($3, phone),
             email = COALESCE($4, email),
             update_time = NOW()
         WHERE id = $5 AND deleted = 0",
    )
    .bind(nickname)
    .bind(avatar)
    .bind(mobile)
    .bind(email)
    .bind(user_id)
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
}

pub async fn list_role_codes_by_user_id(
    pool: &PgPool,
    user_id: i64,
) -> Result<Vec<String>, sqlx::Error> {
    sqlx::query_scalar::<_, String>(
        "SELECT DISTINCT r.code
         FROM sys_user_role ur
         JOIN sys_role r ON r.id = ur.role_id
         WHERE ur.user_id = $1 AND r.deleted = 0 AND r.status = 1
         ORDER BY r.code",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
}

pub async fn list_role_permissions_by_user_id(
    pool: &PgPool,
    user_id: i64,
) -> Result<Vec<String>, sqlx::Error> {
    sqlx::query_scalar::<_, String>(
        "SELECT DISTINCT p.code
         FROM sys_user_role ur
         JOIN sys_role r ON r.id = ur.role_id
         JOIN sys_role_permission rp ON rp.role_id = ur.role_id
         JOIN sys_permission p ON p.id = rp.permission_id
         WHERE ur.user_id = $1 AND r.deleted = 0 AND r.status = 1
         ORDER BY p.code",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
}

pub async fn list_menu_permissions_by_user_id(
    pool: &PgPool,
    user_id: i64,
) -> Result<Vec<String>, sqlx::Error> {
    sqlx::query_scalar::<_, String>(
        "SELECT DISTINCT m.permission
         FROM sys_menu m
         JOIN sys_role_menu rm ON rm.menu_id = m.id
         JOIN sys_user_role ur ON ur.role_id = rm.role_id
         JOIN sys_role r ON r.id = ur.role_id
         WHERE ur.user_id = $1
           AND r.deleted = 0
           AND r.status = 1
           AND m.permission IS NOT NULL
           AND m.permission <> ''
         ORDER BY m.permission",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
}

pub async fn list_user_menus_by_user_id(
    pool: &PgPool,
    user_id: i64,
) -> Result<Vec<OrionUserMenuRow>, sqlx::Error> {
    sqlx::query_as::<_, OrionUserMenuRow>(
        "SELECT DISTINCT
            m.id,
            m.parent_id,
            m.name,
            m.permission,
            m.type AS menu_type,
            m.sort,
            m.visible,
            m.icon,
            m.path,
            m.component
         FROM sys_menu m
         JOIN sys_role_menu rm ON rm.menu_id = m.id
         JOIN sys_user_role ur ON ur.role_id = rm.role_id
         JOIN sys_role r ON r.id = ur.role_id
         WHERE ur.user_id = $1 AND r.deleted = 0 AND r.status = 1
         ORDER BY m.sort ASC, m.id ASC",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
}

pub async fn list_user_roles_by_user_id(
    pool: &PgPool,
    user_id: i64,
) -> Result<Vec<OrionUserRoleRow>, sqlx::Error> {
    sqlx::query_as::<_, OrionUserRoleRow>(
        "SELECT r.id, r.name, r.code, r.status, r.description
         FROM sys_user_role ur
         JOIN sys_role r ON r.id = ur.role_id
         WHERE ur.user_id = $1 AND r.deleted = 0",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
}

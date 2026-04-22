use sqlx::PgPool;

use crate::domain::orion::role::OrionRoleAggregate;

#[derive(Debug, sqlx::FromRow)]
struct OrionRoleRow {
    id: i64,
    name: String,
    code: String,
    status: i16,
    description: Option<String>,
    create_time_ms: i64,
}

pub async fn create_role(
    pool: &PgPool,
    name: &str,
    code: &str,
    description: Option<&str>,
    status: Option<i16>,
) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar::<_, i64>(
        "INSERT INTO sys_role (name, code, description, status, create_time, deleted)
         VALUES ($1, $2, $3, COALESCE($4, 1), NOW(), 0)
         RETURNING id",
    )
    .bind(name)
    .bind(code)
    .bind(description)
    .bind(status)
    .fetch_one(pool)
    .await
}

pub async fn update_role(
    pool: &PgPool,
    id: i64,
    name: Option<&str>,
    code: Option<&str>,
    description: Option<&str>,
    status: Option<i16>,
) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE sys_role SET name = COALESCE(NULLIF($1, ''), name), code = COALESCE(NULLIF($2, ''), code), description = COALESCE($3, description), status = COALESCE($4, status) WHERE id = $5 AND deleted = 0",
    )
    .bind(name)
    .bind(code)
    .bind(description)
    .bind(status)
    .bind(id)
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
}

pub async fn update_role_status(pool: &PgPool, id: i64, status: i16) -> Result<u64, sqlx::Error> {
    let result = sqlx::query("UPDATE sys_role SET status = $1 WHERE id = $2 AND deleted = 0")
        .bind(status)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected())
}

pub async fn get_role_by_id(
    pool: &PgPool,
    id: i64,
) -> Result<Option<OrionRoleAggregate>, sqlx::Error> {
    let row = sqlx::query_as::<_, OrionRoleRow>(
        "SELECT id, name, code, status, description,
                EXTRACT(EPOCH FROM create_time)::bigint * 1000 AS create_time_ms
         FROM sys_role
         WHERE id = $1 AND deleted = 0",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    Ok(row.map(Into::into))
}

pub async fn list_roles(pool: &PgPool) -> Result<Vec<OrionRoleAggregate>, sqlx::Error> {
    let rows = sqlx::query_as::<_, OrionRoleRow>(
        "SELECT id, name, code, status, description,
                EXTRACT(EPOCH FROM create_time)::bigint * 1000 AS create_time_ms
         FROM sys_role
         WHERE deleted = 0
         ORDER BY id ASC",
    )
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(Into::into).collect())
}

pub async fn query_roles(
    pool: &PgPool,
    limit: i64,
    offset: i64,
) -> Result<Vec<OrionRoleAggregate>, sqlx::Error> {
    let rows = sqlx::query_as::<_, OrionRoleRow>(
        "SELECT id, name, code, status, description,
                EXTRACT(EPOCH FROM create_time)::bigint * 1000 AS create_time_ms
         FROM sys_role
         WHERE deleted = 0
         ORDER BY id DESC
         LIMIT $1 OFFSET $2",
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(Into::into).collect())
}

pub async fn count_roles(pool: &PgPool) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar::<_, i64>("SELECT COUNT(1) FROM sys_role WHERE deleted = 0")
        .fetch_one(pool)
        .await
}

pub async fn soft_delete_role(pool: &PgPool, id: i64) -> Result<u64, sqlx::Error> {
    let result = sqlx::query("UPDATE sys_role SET deleted = 1 WHERE id = $1 AND deleted = 0")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected())
}

pub async fn replace_role_menus(
    pool: &PgPool,
    role_id: i64,
    menu_ids: &[i64],
) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;
    sqlx::query("DELETE FROM sys_role_menu WHERE role_id = $1")
        .bind(role_id)
        .execute(&mut *tx)
        .await?;
    for menu_id in menu_ids {
        sqlx::query(
            "INSERT INTO sys_role_menu (role_id, menu_id) VALUES ($1, $2) ON CONFLICT (role_id, menu_id) DO NOTHING",
        )
        .bind(role_id)
        .bind(menu_id)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await
}

pub async fn list_role_menu_ids(pool: &PgPool, role_id: i64) -> Result<Vec<i64>, sqlx::Error> {
    sqlx::query_scalar::<_, i64>(
        "SELECT menu_id FROM sys_role_menu WHERE role_id = $1 ORDER BY menu_id ASC",
    )
    .bind(role_id)
    .fetch_all(pool)
    .await
}

pub async fn list_roles_by_user_id(
    pool: &PgPool,
    user_id: i64,
) -> Result<Vec<OrionRoleAggregate>, sqlx::Error> {
    let rows = sqlx::query_as::<_, OrionRoleRow>(
        "SELECT r.id,
                r.name,
                r.code,
                r.status,
                r.description,
                EXTRACT(EPOCH FROM r.create_time)::bigint * 1000 AS create_time_ms
         FROM sys_user_role ur
         JOIN sys_role r ON r.id = ur.role_id
         WHERE ur.user_id = $1 AND r.deleted = 0
         ORDER BY r.id ASC",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(Into::into).collect())
}

impl From<OrionRoleRow> for OrionRoleAggregate {
    fn from(value: OrionRoleRow) -> Self {
        Self {
            id: value.id,
            name: value.name,
            code: value.code,
            status: value.status,
            description: value.description,
            create_time_ms: value.create_time_ms,
        }
    }
}

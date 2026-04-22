use sqlx::PgPool;

use crate::domain::orion::menu::OrionMenuAggregate;

#[derive(Debug, sqlx::FromRow)]
struct OrionMenuRow {
    id: i64,
    parent_id: i64,
    name: String,
    permission: Option<String>,
    menu_type: i16,
    sort: i32,
    visible: i16,
    icon: Option<String>,
    path: Option<String>,
    component: Option<String>,
}

pub async fn list_menus(pool: &PgPool) -> Result<Vec<OrionMenuAggregate>, sqlx::Error> {
    let rows = sqlx::query_as::<_, OrionMenuRow>(
        "SELECT id, parent_id, name, permission, type AS menu_type, sort, visible, icon, path, component
         FROM sys_menu
         ORDER BY sort ASC, id ASC",
    )
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(Into::into).collect())
}

pub async fn create_menu(
    pool: &PgPool,
    parent_id: Option<i64>,
    name: &str,
    path: Option<&str>,
    component: Option<&str>,
    icon: Option<&str>,
    menu_type: Option<i16>,
    sort: Option<i32>,
    visible: Option<i16>,
    permission: Option<&str>,
) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar::<_, i64>(
        "INSERT INTO sys_menu (parent_id, name, path, component, icon, type, sort, visible, permission, create_time)
         VALUES (COALESCE($1, 0), $2, $3, $4, $5, COALESCE($6, 1), COALESCE($7, 0), COALESCE($8, 1), $9, NOW())
         RETURNING id",
    )
    .bind(parent_id)
    .bind(name)
    .bind(path)
    .bind(component)
    .bind(icon)
    .bind(menu_type)
    .bind(sort)
    .bind(visible)
    .bind(permission)
    .fetch_one(pool)
    .await
}

pub struct OrionMenuPatch<'a> {
    pub id: i64,
    pub parent_id: Option<i64>,
    pub name: Option<&'a str>,
    pub path: Option<&'a str>,
    pub component: Option<&'a str>,
    pub icon: Option<&'a str>,
    pub menu_type: Option<i16>,
    pub sort: Option<i32>,
    pub visible: Option<i16>,
    pub permission: Option<&'a str>,
}

pub async fn update_menu(pool: &PgPool, patch: OrionMenuPatch<'_>) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE sys_menu SET
            parent_id = COALESCE($1, parent_id),
            name = COALESCE(NULLIF($2, ''), name),
            path = COALESCE($3, path),
            component = COALESCE($4, component),
            icon = COALESCE($5, icon),
            type = COALESCE($6, type),
            sort = COALESCE($7, sort),
            visible = COALESCE($8, visible),
            permission = COALESCE($9, permission)
         WHERE id = $10",
    )
    .bind(patch.parent_id)
    .bind(patch.name)
    .bind(patch.path)
    .bind(patch.component)
    .bind(patch.icon)
    .bind(patch.menu_type)
    .bind(patch.sort)
    .bind(patch.visible)
    .bind(patch.permission)
    .bind(patch.id)
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
}

pub async fn update_menu_visible(pool: &PgPool, id: i64, visible: i16) -> Result<u64, sqlx::Error> {
    let result = sqlx::query("UPDATE sys_menu SET visible = $1 WHERE id = $2")
        .bind(visible)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected())
}

pub async fn delete_menu(pool: &PgPool, id: i64) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;
    sqlx::query("DELETE FROM sys_role_menu WHERE menu_id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM sys_menu WHERE id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await
}

impl From<OrionMenuRow> for OrionMenuAggregate {
    fn from(value: OrionMenuRow) -> Self {
        Self {
            id: value.id,
            parent_id: value.parent_id,
            name: value.name,
            permission: value.permission,
            menu_type: value.menu_type,
            sort: value.sort,
            visible: value.visible,
            icon: value.icon,
            path: value.path,
            component: value.component,
        }
    }
}

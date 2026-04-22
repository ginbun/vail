use sqlx::PgPool;

use crate::{
    domain::orion::menu::OrionMenuAggregate,
    infrastructure::orion::menu_repository::{self, OrionMenuPatch},
};

#[derive(Debug, Clone)]
pub struct OrionMenuCreateInput {
    pub parent_id: Option<i64>,
    pub name: String,
    pub path: Option<String>,
    pub component: Option<String>,
    pub icon: Option<String>,
    pub menu_type: Option<i16>,
    pub sort: Option<i32>,
    pub visible: Option<i16>,
    pub permission: Option<String>,
}

#[derive(Debug, Clone)]
pub struct OrionMenuUpdateInput {
    pub id: i64,
    pub parent_id: Option<i64>,
    pub name: Option<String>,
    pub path: Option<String>,
    pub component: Option<String>,
    pub icon: Option<String>,
    pub menu_type: Option<i16>,
    pub sort: Option<i32>,
    pub visible: Option<i16>,
    pub permission: Option<String>,
}

pub async fn list_menus(pool: &PgPool) -> Result<Vec<OrionMenuAggregate>, sqlx::Error> {
    menu_repository::list_menus(pool).await
}

pub async fn create_menu(pool: &PgPool, input: OrionMenuCreateInput) -> Result<i64, sqlx::Error> {
    menu_repository::create_menu(
        pool,
        input.parent_id,
        &input.name,
        input.path.as_deref(),
        input.component.as_deref(),
        input.icon.as_deref(),
        input.menu_type,
        input.sort,
        input.visible,
        input.permission.as_deref(),
    )
    .await
}

pub async fn update_menu(pool: &PgPool, input: OrionMenuUpdateInput) -> Result<u64, sqlx::Error> {
    menu_repository::update_menu(
        pool,
        OrionMenuPatch {
            id: input.id,
            parent_id: input.parent_id,
            name: input.name.as_deref(),
            path: input.path.as_deref(),
            component: input.component.as_deref(),
            icon: input.icon.as_deref(),
            menu_type: input.menu_type,
            sort: input.sort,
            visible: input.visible,
            permission: input.permission.as_deref(),
        },
    )
    .await
}

pub async fn update_menu_visible(pool: &PgPool, id: i64, visible: i16) -> Result<u64, sqlx::Error> {
    menu_repository::update_menu_visible(pool, id, visible).await
}

pub async fn delete_menu(pool: &PgPool, id: i64) -> Result<(), sqlx::Error> {
    menu_repository::delete_menu(pool, id).await
}

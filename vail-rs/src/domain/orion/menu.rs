#[derive(Debug, Clone)]
pub struct OrionMenuAggregate {
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

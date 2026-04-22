#[derive(Debug, Clone)]
pub struct OrionHostKeyAggregate {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub create_time_ms: i64,
    pub update_time_ms: i64,
}

#[derive(Debug, Clone)]
pub struct OrionHostIdentityAggregate {
    pub id: i64,
    pub name: String,
    pub identity_type: String,
    pub username: Option<String>,
    pub key_id: Option<i64>,
    pub description: Option<String>,
    pub create_time_ms: i64,
    pub update_time_ms: i64,
}

#[derive(Debug, Clone)]
pub struct OrionHostGroupAggregate {
    pub id: i64,
    pub parent_id: i64,
    pub name: String,
}

#[derive(Debug, Clone, Copy)]
pub enum OrionGrantScope {
    Role(i64),
    User(i64),
}

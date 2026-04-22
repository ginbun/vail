#[derive(Debug, Clone)]
pub struct OrionRoleAggregate {
    pub id: i64,
    pub name: String,
    pub code: String,
    pub status: i16,
    pub description: Option<String>,
    pub create_time_ms: i64,
}

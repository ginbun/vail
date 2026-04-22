#[derive(Debug, Clone)]
pub struct OrionHostAggregate {
    pub id: i64,
    pub name: String,
    pub hostname: String,
    pub description: Option<String>,
    pub status: i16,
    pub create_time_ms: i64,
    pub update_time_ms: i64,
    pub group_ids: Vec<i64>,
}

impl OrionHostAggregate {
    pub fn status_label(&self) -> &'static str {
        if self.status == 1 {
            "ENABLED"
        } else {
            "DISABLED"
        }
    }
}

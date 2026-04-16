pub type OrionHostRow = (
    i64,
    String,
    String,
    Option<String>,
    i16,
    i64,
    i64,
    Option<Vec<i64>>,
);

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

impl From<OrionHostRow> for OrionHostAggregate {
    fn from(value: OrionHostRow) -> Self {
        Self {
            id: value.0,
            name: value.1,
            hostname: value.2,
            description: value.3,
            status: value.4,
            create_time_ms: value.5,
            update_time_ms: value.6,
            group_ids: value.7.unwrap_or_default(),
        }
    }
}

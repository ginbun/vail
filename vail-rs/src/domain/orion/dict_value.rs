#[derive(Debug, Clone)]
pub struct OrionDictValueAggregate {
    pub id: i64,
    pub key_id: i64,
    pub key_name: String,
    pub key_description: Option<String>,
    pub value: String,
    pub label: String,
    pub extra: Option<String>,
    pub sort: i32,
    pub create_time: i64,
    pub update_time: i64,
    pub creator: Option<String>,
    pub updater: Option<String>,
}

#[derive(Debug, Clone)]
pub struct OrionDictValueOptionAggregate {
    pub key_name: String,
    pub value_type: String,
    pub label: String,
    pub value: String,
    pub extra: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct OrionDictValueQueryFilters {
    pub key_id: Option<i64>,
    pub key_name: Option<String>,
    pub value: Option<String>,
    pub label: Option<String>,
    pub extra: Option<String>,
    pub limit: i64,
    pub offset: i64,
}

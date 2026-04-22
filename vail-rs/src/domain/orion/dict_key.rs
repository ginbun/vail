#[derive(Debug, Clone)]
pub struct OrionDictKeyAggregate {
    pub id: i64,
    pub key_name: String,
    pub value_type: String,
    pub extra_schema: Option<String>,
    pub description: Option<String>,
    pub create_time: i64,
    pub update_time: i64,
    pub creator: Option<String>,
    pub updater: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct OrionDictKeyQueryFilters {
    pub id: Option<i64>,
    pub key_name: Option<String>,
    pub description: Option<String>,
    pub search_value: Option<String>,
    pub limit: i64,
    pub offset: i64,
}

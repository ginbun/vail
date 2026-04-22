#[derive(Debug, Clone)]
pub struct OrionSystemMessageAggregate {
    pub id: i64,
    pub classify: String,
    pub message_type: String,
    pub status: i16,
    pub rel_key: Option<String>,
    pub title: String,
    pub content: String,
    pub content_html: Option<String>,
    pub create_time: i64,
}

#[derive(Debug, Clone)]
pub struct OrionSystemMessageClassifyCount {
    pub classify: String,
    pub count: i64,
}

#[derive(Debug, Clone, Default)]
pub struct OrionSystemMessageListFilters {
    pub user_id: i64,
    pub classify: Option<String>,
    pub query_unread: bool,
    pub max_id: Option<i64>,
    pub limit: i64,
    pub offset: i64,
}

#[derive(Debug, Clone, Default)]
pub struct OrionSystemMessageCountFilters {
    pub user_id: i64,
    pub query_unread: bool,
}

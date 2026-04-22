use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct OrionOperatorLogAggregate {
    pub id: i64,
    pub user_id: Option<i64>,
    pub username: Option<String>,
    pub module: Option<String>,
    pub operation: Option<String>,
    pub method: Option<String>,
    pub path: Option<String>,
    pub params: Option<serde_json::Value>,
    pub trace_id: Option<String>,
    pub ip: Option<String>,
    pub user_agent: Option<String>,
    pub result: Option<i16>,
    pub error_message: Option<String>,
    pub duration: Option<i32>,
    pub create_time: i64,
}

#[derive(Debug, Clone, Default)]
pub struct OrionOperatorLogFilters {
    pub scope_user_id: Option<i64>,
    pub user_id: Option<i64>,
    pub username: Option<String>,
    pub module: Option<String>,
    pub operation: Option<String>,
    pub risk_level: Option<String>,
    pub result: Option<i16>,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub limit: i64,
    pub offset: i64,
}

#[derive(Debug, Clone)]
pub struct OrionOperatorLogClearFilters {
    pub user_id: Option<i64>,
    pub username: Option<String>,
    pub module: Option<String>,
    pub operation: Option<String>,
    pub risk_level: Option<String>,
    pub result: Option<i16>,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub limit: i64,
}

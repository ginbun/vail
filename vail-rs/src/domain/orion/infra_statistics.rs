#[derive(Debug, Clone)]
pub struct OrionInfraWorkplaceAggregate {
    pub user_id: i64,
    pub username: String,
    pub nickname: Option<String>,
    pub unread_message_count: i64,
    pub last_login_time: Option<i64>,
    pub user_session_count: i64,
    pub operator_chart_labels: Vec<String>,
    pub operator_chart_values: Vec<i64>,
    pub login_history: Vec<OrionLoginHistoryAggregate>,
}

#[derive(Debug, Clone)]
pub struct OrionLoginHistoryAggregate {
    pub id: i64,
    pub address: Option<String>,
    pub location: Option<String>,
    pub user_agent: Option<String>,
    pub result: Option<i16>,
    pub error_message: Option<String>,
    pub create_time: i64,
}

#[derive(Debug, Clone)]
pub struct OrionInfraUserSummaryAggregate {
    pub username: String,
    pub nickname: Option<String>,
    pub last_login_time: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct OrionChartPointAggregate {
    pub label: String,
    pub count: i64,
}

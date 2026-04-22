#[derive(Debug, Clone)]
pub struct OrionSystemUserAggregate {
    pub id: i64,
    pub username: String,
    pub nickname: Option<String>,
    pub avatar: Option<String>,
    pub mobile: Option<String>,
    pub email: Option<String>,
    pub status: i16,
    pub last_login_time: Option<i64>,
    pub create_time_ms: i64,
    pub update_time_ms: i64,
}

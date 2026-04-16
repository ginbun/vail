pub type OrionSystemUserRow = (
    i64,
    String,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    i16,
    Option<i64>,
    i64,
    i64,
);

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

impl From<OrionSystemUserRow> for OrionSystemUserAggregate {
    fn from(value: OrionSystemUserRow) -> Self {
        Self {
            id: value.0,
            username: value.1,
            nickname: value.2,
            avatar: value.3,
            mobile: value.4,
            email: value.5,
            status: value.6,
            last_login_time: value.7,
            create_time_ms: value.8,
            update_time_ms: value.9,
        }
    }
}

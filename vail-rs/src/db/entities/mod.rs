use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SysUser {
    pub id: i64,
    pub username: String,
    pub password: String,
    pub nickname: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub avatar: Option<String>,
    pub status: i16,
    pub last_login_time: Option<chrono::DateTime<chrono::Utc>>,
    pub last_login_ip: Option<String>,
    pub create_time: chrono::DateTime<chrono::Utc>,
    pub update_time: chrono::DateTime<chrono::Utc>,
    pub deleted: i16,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SysRole {
    pub id: i64,
    pub name: String,
    pub code: String,
    pub description: Option<String>,
    pub status: i16,
    pub create_time: chrono::DateTime<chrono::Utc>,
    pub deleted: i16,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Host {
    pub id: i64,
    pub name: String,
    pub hostname: String,
    pub port: i32,
    pub username: Option<String>,
    pub credential_type: Option<String>,
    pub credential_data: Option<String>,
    pub description: Option<String>,
    pub tags: Option<serde_json::Value>,
    pub status: i16,
    pub create_time: chrono::DateTime<chrono::Utc>,
    pub update_time: chrono::DateTime<chrono::Utc>,
    pub deleted: i16,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HostGroup {
    pub id: i64,
    pub name: String,
    pub parent_id: Option<i64>,
    pub description: Option<String>,
    pub sort: Option<i32>,
    pub create_time: chrono::DateTime<chrono::Utc>,
    pub deleted: i16,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UploadTask {
    pub id: i64,
    pub task_no: String,
    pub user_id: i64,
    pub host_id: i64,
    pub remote_path: String,
    pub file_name: Option<String>,
    pub file_size: Option<i64>,
    pub file_md5: Option<String>,
    pub chunk_size: i64,
    pub uploaded_size: i64,
    pub status: i16,
    pub error_message: Option<String>,
    pub create_time: chrono::DateTime<chrono::Utc>,
    pub update_time: chrono::DateTime<chrono::Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LoginLog {
    pub id: i64,
    pub user_id: Option<i64>,
    pub username: Option<String>,
    pub ip: Option<String>,
    pub location: Option<String>,
    pub user_agent: Option<String>,
    pub result: Option<i16>,
    pub error_message: Option<String>,
    pub create_time: chrono::DateTime<chrono::Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OperatorLog {
    pub id: i64,
    pub user_id: Option<i64>,
    pub username: Option<String>,
    pub module: Option<String>,
    pub operation: Option<String>,
    pub method: Option<String>,
    pub path: Option<String>,
    pub params: Option<serde_json::Value>,
    pub result: Option<i16>,
    pub error_message: Option<String>,
    pub duration: Option<i32>,
    pub trace_id: Option<String>,
    pub ip: Option<String>,
    pub user_agent: Option<String>,
    pub create_time: chrono::DateTime<chrono::Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Cache {
    pub cache_key: String,
    pub cache_value: String,
    pub expire_time: Option<chrono::DateTime<chrono::Utc>>,
    pub create_time: chrono::DateTime<chrono::Utc>,
}

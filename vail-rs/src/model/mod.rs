use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Serialize, Deserialize)]
pub struct LoginResponse {
    pub mfa_required: bool,
    pub login_challenge_id: Option<String>,
    pub challenge_expires_in: Option<u64>,
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub expires_in: Option<u64>,
    pub user: Option<UserInfo>,
}

#[derive(Serialize, Deserialize)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RefreshResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: u64,
}

#[derive(Serialize, Deserialize)]
pub struct TotpVerifyRequest {
    pub login_challenge_id: String,
    pub code: String,
}

#[derive(Serialize, Deserialize)]
pub struct LogoutRequest {
    pub refresh_token: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserInfo {
    pub id: i64,
    pub username: String,
    pub nickname: Option<String>,
    pub avatar: Option<String>,
    pub email: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct CreateHostRequest {
    pub name: String,
    pub hostname: String,
    pub port: Option<i32>,
    pub username: Option<String>,
    pub credential_type: Option<String>,
    pub credential_data: Option<String>,
    pub credential_passphrase: Option<String>,
    pub ssh_key_id: Option<i64>,
    pub description: Option<String>,
    pub tags: Option<serde_json::Value>,
}

#[derive(Serialize, Deserialize)]
pub struct UpdateHostRequest {
    pub name: Option<String>,
    pub hostname: Option<String>,
    pub port: Option<i32>,
    pub username: Option<String>,
    pub credential_type: Option<String>,
    pub credential_data: Option<String>,
    pub credential_passphrase: Option<String>,
    pub ssh_key_id: Option<i64>,
    pub description: Option<String>,
    pub tags: Option<serde_json::Value>,
    pub status: Option<i16>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HostResponse {
    pub id: i64,
    pub name: String,
    pub hostname: String,
    pub port: i32,
    pub username: Option<String>,
    pub credential_type: Option<String>,
    pub ssh_key_id: Option<i64>,
    pub description: Option<String>,
    pub tags: Option<serde_json::Value>,
    pub status: i16,
    pub create_time: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateSshKeyRequest {
    pub name: String,
    pub private_key: String,
    pub passphrase: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateSshKeyRequest {
    pub name: Option<String>,
    pub private_key: Option<String>,
    pub passphrase: Option<String>,
    pub description: Option<String>,
    pub status: Option<i16>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SshKeyResponse {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub status: i16,
    pub has_passphrase: bool,
    pub create_time: String,
    pub bound_host_count: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BindHostSshKeyRequest {
    pub ssh_key_id: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateUploadTaskRequest {
    pub host_id: i64,
    pub remote_path: String,
    pub file_name: String,
    pub file_size: i64,
    pub file_md5: Option<String>,
    pub chunk_size: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UploadTaskResponse {
    pub id: i64,
    pub task_no: String,
    pub status: i16,
    pub uploaded_size: i64,
    pub file_size: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UploadChunkRequest {
    pub task_id: i64,
    pub chunk_index: i32,
    pub offset: i64,
    pub content: Vec<u8>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateSshSessionRequest {
    pub host_id: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SshSessionItem {
    pub id: i64,
    pub host_id: i64,
    pub session_id: String,
    pub status: i16,
    pub start_time: String,
    pub end_time: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JitRequestCreateRequest {
    pub reason: String,
    pub duration_minutes: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JitApproveRequest {
    pub request_id: i64,
    pub approve: bool,
    pub duration_minutes: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AssignUserRolesRequest {
    pub role_ids: Vec<i64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AssignUserHostsRequest {
    pub host_ids: Vec<i64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct IamMeSummaryResponse {
    pub user_id: i64,
    pub roles: Vec<String>,
    pub permissions: Vec<String>,
    pub host_access_count: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct IamUserPermissionsResponse {
    pub user_id: i64,
    pub roles: Vec<String>,
    pub permissions: Vec<String>,
    pub host_ids: Vec<i64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct IamUserListItem {
    pub id: i64,
    pub username: String,
    pub nickname: Option<String>,
    pub status: i16,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct IamRoleListItem {
    pub id: i64,
    pub code: String,
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct IamUserBindingsResponse {
    pub user_id: i64,
    pub role_ids: Vec<i64>,
    pub host_ids: Vec<i64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub code: u16,
    pub message: String,
    pub msg: String,
    pub data: Option<T>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            code: 200,
            message: "success".to_string(),
            msg: "success".to_string(),
            data: Some(data),
        }
    }

    pub fn error(code: u16, message: String) -> Self {
        Self {
            code,
            message: message.clone(),
            msg: message,
            data: None,
        }
    }
}

use axum::{
    extract::{Multipart, State},
    http::HeaderMap,
    routing::{get, post},
    Router,
};
use std::{fs, path::PathBuf};

use crate::{
    api::{guard, AppState},
    error::{AppError, AppResult},
    model::*,
    ssh_client,
};

fn header_string(headers: &HeaderMap, key: &str) -> Option<String> {
    headers
        .get(key)
        .and_then(|v| v.to_str().ok())
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

fn source_ip(headers: &HeaderMap) -> Option<String> {
    header_string(headers, "x-forwarded-for")
        .and_then(|v| v.split(',').next().map(|s| s.trim().to_string()))
        .or_else(|| header_string(headers, "x-real-ip"))
}

async fn append_operator_log(
    state: &AppState,
    headers: &HeaderMap,
    actor_user_id: i64,
    operation: &str,
    params: serde_json::Value,
    result: i16,
    error_message: Option<String>,
) -> AppResult<()> {
    let username = sqlx::query_scalar::<_, Option<String>>(
        "SELECT username FROM sys_user WHERE id = $1 AND deleted = 0",
    )
    .bind(actor_user_id)
    .fetch_one(&state.db)
    .await?
    .unwrap_or_else(|| actor_user_id.to_string());

    sqlx::query(
        "INSERT INTO operator_log (
            user_id,
            username,
            module,
            operation,
            method,
            path,
            params,
            result,
            error_message,
            duration,
            trace_id,
            ip,
            user_agent,
            create_time
        ) VALUES ($1, $2, 'sftp', $3, NULL, NULL, $4::jsonb, $5, $6, NULL, NULL, $7, $8, NOW())",
    )
    .bind(actor_user_id)
    .bind(username)
    .bind(operation)
    .bind(params.to_string())
    .bind(result)
    .bind(error_message)
    .bind(source_ip(headers))
    .bind(header_string(headers, "user-agent"))
    .execute(&state.db)
    .await?;

    Ok(())
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/sftp/upload-batch", post(upload_batch))
        .route("/sftp/task", post(create_upload_task))
        .route("/sftp/upload", post(upload_chunk))
        .route("/sftp/complete", post(complete_upload))
        .route("/sftp/tasks", get(list_upload_tasks))
        .route("/sftp/tasks/:id", get(get_upload_task))
}

fn normalize_remote_base_path(raw: &str) -> Result<String, AppError> {
    let candidate = raw.trim().replace('\\', "/");
    if !candidate.starts_with('/') {
        return Err(AppError::BadRequest(
            "remote_base_path must be an absolute unix path".to_string(),
        ));
    }

    let mut parts = Vec::new();
    for segment in candidate.split('/') {
        if segment.is_empty() || segment == "." {
            continue;
        }
        if segment == ".." {
            return Err(AppError::BadRequest(
                "remote_base_path cannot contain parent directory traversal".to_string(),
            ));
        }
        parts.push(segment);
    }

    if parts.is_empty() {
        Ok("/".to_string())
    } else {
        Ok(format!("/{}", parts.join("/")))
    }
}

fn normalize_relative_path(raw: &str) -> Result<String, AppError> {
    let candidate = raw.trim().replace('\\', "/");
    let candidate = candidate.trim_start_matches('/');
    if candidate.is_empty() {
        return Err(AppError::BadRequest(
            "file relative path is required".to_string(),
        ));
    }

    let mut parts = Vec::new();
    for segment in candidate.split('/') {
        if segment.is_empty() || segment == "." {
            continue;
        }
        if segment == ".." {
            return Err(AppError::BadRequest(
                "file relative path cannot contain parent directory traversal".to_string(),
            ));
        }
        parts.push(segment);
    }

    if parts.is_empty() {
        return Err(AppError::BadRequest(
            "file relative path is invalid".to_string(),
        ));
    }

    Ok(parts.join("/"))
}

fn join_remote_path(base: &str, relative: &str) -> String {
    if base == "/" {
        format!("/{relative}")
    } else {
        format!("{base}/{relative}")
    }
}

fn calc_uploaded_size_from_chunks(task_dir: &PathBuf) -> Result<i64, AppError> {
    let mut total: i64 = 0;
    let entries = fs::read_dir(task_dir).map_err(|e| AppError::Internal(e.to_string()))?;
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.starts_with("chunk_") {
            continue;
        }
        let metadata = entry
            .metadata()
            .map_err(|e| AppError::Internal(e.to_string()))?;
        total = total.saturating_add(metadata.len() as i64);
    }
    Ok(total)
}

async fn has_host_read_permission(state: &AppState, user_id: i64) -> AppResult<bool> {
    let allowed = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(
            SELECT 1
            FROM sys_user_role ur
            JOIN sys_role r ON r.id = ur.role_id AND r.deleted = 0 AND r.status = 1
            JOIN sys_role_permission rp ON rp.role_id = ur.role_id
            JOIN sys_permission p ON p.id = rp.permission_id
            WHERE ur.user_id = $1 AND p.code = 'host.read'
        )",
    )
    .bind(user_id)
    .fetch_one(&state.db)
    .await?;

    Ok(allowed)
}

async fn can_access_host(
    state: &AppState,
    user_id: i64,
    host_id: i64,
    is_admin: bool,
) -> AppResult<bool> {
    if is_admin {
        let exists = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM host WHERE id = $1 AND deleted = 0 AND status = 1)",
        )
        .bind(host_id)
        .fetch_one(&state.db)
        .await?;
        return Ok(exists);
    }

    let exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(
            SELECT 1
            FROM user_host_access uha
            JOIN host h ON h.id = uha.host_id
            WHERE uha.user_id = $1
              AND uha.host_id = $2
              AND h.deleted = 0
              AND h.status = 1
        )",
    )
    .bind(user_id)
    .bind(host_id)
    .fetch_one(&state.db)
    .await?;

    Ok(exists)
}

async fn upload_batch(
    State(state): State<AppState>,
    headers: HeaderMap,
    mut multipart: Multipart,
) -> AppResult<impl axum::response::IntoResponse> {
    let user_id = guard::current_user_id(&headers, &state.config.jwt.secret)?;

    let mut host_id: Option<i64> = None;
    let mut remote_base_path: Option<String> = None;
    let mut files: Vec<(String, Vec<u8>)> = Vec::new();
    let mut total_size: u64 = 0;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::BadRequest(e.to_string()))?
    {
        let name = field.name().unwrap_or("").to_string();
        match name.as_str() {
            "host_id" => {
                let text = field
                    .text()
                    .await
                    .map_err(|e| AppError::BadRequest(e.to_string()))?;
                host_id = text.trim().parse().ok();
            }
            "remote_base_path" => {
                let text = field
                    .text()
                    .await
                    .map_err(|e| AppError::BadRequest(e.to_string()))?;
                remote_base_path = Some(text);
            }
            "files" => {
                let file_name = field
                    .file_name()
                    .ok_or_else(|| AppError::BadRequest("missing multipart file name".to_string()))?
                    .to_string();
                let data = field
                    .bytes()
                    .await
                    .map_err(|e| AppError::BadRequest(e.to_string()))?
                    .to_vec();
                total_size += data.len() as u64;
                files.push((file_name, data));
            }
            _ => {}
        }
    }

    let host_id = host_id.ok_or_else(|| AppError::BadRequest("host_id is required".to_string()))?;
    if host_id <= 0 {
        return Err(AppError::BadRequest(
            "host_id must be greater than 0".to_string(),
        ));
    }
    if files.is_empty() {
        return Err(AppError::BadRequest(
            "at least one file is required".to_string(),
        ));
    }

    let max_upload_size_bytes = state
        .config
        .storage
        .max_upload_size
        .saturating_mul(1024 * 1024);
    if max_upload_size_bytes > 0 && total_size > max_upload_size_bytes {
        return Err(AppError::BadRequest(format!(
            "total upload size exceeds limit: {} MB",
            state.config.storage.max_upload_size
        )));
    }

    let is_admin = has_host_read_permission(&state, user_id).await?;
    if !can_access_host(&state, user_id, host_id, is_admin).await? {
        return Err(AppError::Auth("Host access denied".to_string()));
    }

    let base = normalize_remote_base_path(remote_base_path.as_deref().unwrap_or("/tmp"))?;
    let mut pending_uploads = Vec::with_capacity(files.len());
    let mut uploaded_paths = Vec::with_capacity(files.len());

    for (raw_path, bytes) in files {
        let relative = normalize_relative_path(&raw_path)?;
        let remote_path = join_remote_path(&base, &relative);
        uploaded_paths.push(remote_path.clone());
        pending_uploads.push((remote_path, bytes));
    }

    let host_ssh_config = ssh_client::resolve_host_ssh_config(
        &state.db,
        &state.config.secrets.data_encryption_key,
        Some(user_id),
        host_id,
    )
    .await?;
    ssh_client::upload_files(
        host_ssh_config,
        state.config.ssh.connection_timeout,
        pending_uploads,
    )
    .await?;

    append_operator_log(
        &state,
        &headers,
        user_id,
        "sftp_upload_batch",
        serde_json::json!({
            "host_id": host_id,
            "remote_base_path": base,
            "uploaded_count": uploaded_paths.len(),
            "uploaded_paths": uploaded_paths.clone(),
        }),
        1,
        None,
    )
    .await?;

    Ok(axum::Json(ApiResponse::success(serde_json::json!({
        "host_id": host_id,
        "uploaded_count": uploaded_paths.len(),
        "uploaded_paths": uploaded_paths,
    }))))
}

async fn create_upload_task(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Json(payload): axum::extract::Json<CreateUploadTaskRequest>,
) -> AppResult<impl axum::response::IntoResponse> {
    let user_id = guard::current_user_id(&headers, &state.config.jwt.secret)?;
    let is_admin = has_host_read_permission(&state, user_id).await?;
    if !can_access_host(&state, user_id, payload.host_id, is_admin).await? {
        return Err(AppError::Auth("Host access denied".to_string()));
    }

    let task_no = uuid::Uuid::new_v4().to_string();

    let task_id = sqlx::query_scalar::<_, i64>(
        "INSERT INTO upload_task (task_no, user_id, host_id, remote_path, file_name, file_size, file_md5, chunk_size, uploaded_size, status, create_time, update_time) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 0, 0, NOW(), NOW()) RETURNING id"
    )
    .bind(&task_no)
    .bind(user_id)
    .bind(payload.host_id)
    .bind(&payload.remote_path)
    .bind(&payload.file_name)
    .bind(payload.file_size)
    .bind(&payload.file_md5)
    .bind(payload.chunk_size.unwrap_or(1048576))
    .fetch_one(&state.db)
    .await?;

    let task_dir = PathBuf::from(&state.config.storage.temp_dir).join(&task_no);
    std::fs::create_dir_all(&task_dir).ok();

    append_operator_log(
        &state,
        &headers,
        user_id,
        "sftp_create_upload_task",
        serde_json::json!({
            "task_id": task_id,
            "task_no": task_no,
            "host_id": payload.host_id,
            "remote_path": payload.remote_path,
            "file_name": payload.file_name,
            "file_size": payload.file_size,
        }),
        1,
        None,
    )
    .await?;

    Ok(axum::Json(ApiResponse::success(UploadTaskResponse {
        id: task_id,
        task_no,
        status: 0,
        uploaded_size: 0,
        file_size: payload.file_size,
    })))
}

async fn upload_chunk(
    State(state): State<AppState>,
    headers: HeaderMap,
    mut multipart: axum::extract::Multipart,
) -> AppResult<impl axum::response::IntoResponse> {
    let user_id = guard::current_user_id(&headers, &state.config.jwt.secret)?;

    let mut task_id: Option<i64> = None;
    let mut chunk_index: Option<i32> = None;
    let mut offset: Option<i64> = None;
    let mut content: Option<Vec<u8>> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| crate::error::AppError::BadRequest(e.to_string()))?
    {
        let name = field.name().unwrap_or("").to_string();
        match name.as_str() {
            "task_id" => {
                if let Ok(text) = field.text().await {
                    task_id = text.parse().ok();
                }
            }
            "chunk_index" => {
                if let Ok(text) = field.text().await {
                    chunk_index = text.parse().ok();
                }
            }
            "offset" => {
                if let Ok(text) = field.text().await {
                    offset = text.parse().ok();
                }
            }
            "content" => {
                if let Ok(data) = field.bytes().await {
                    content = Some(data.to_vec());
                }
            }
            _ => {}
        }
    }

    let task_id =
        task_id.ok_or_else(|| crate::error::AppError::BadRequest("Missing task_id".to_string()))?;
    let chunk_index = chunk_index
        .ok_or_else(|| crate::error::AppError::BadRequest("Missing chunk_index".to_string()))?;
    let _offset = offset.unwrap_or(0);
    let content =
        content.ok_or_else(|| crate::error::AppError::BadRequest("Missing content".to_string()))?;

    let task = sqlx::query_scalar::<_, String>(
        "SELECT task_no FROM upload_task WHERE id = $1 AND user_id = $2",
    )
    .bind(task_id)
    .bind(user_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| crate::error::AppError::NotFound("Task not found".to_string()))?;

    let task_dir = PathBuf::from(&state.config.storage.temp_dir).join(&task);
    if !task_dir.exists() {
        fs::create_dir_all(&task_dir).map_err(|e| AppError::Internal(e.to_string()))?;
    }
    let chunk_file = task_dir.join(format!("chunk_{}", chunk_index));

    std::fs::write(&chunk_file, &content)
        .map_err(|e| crate::error::AppError::Internal(e.to_string()))?;

    let new_size = calc_uploaded_size_from_chunks(&task_dir)?;
    sqlx::query("UPDATE upload_task SET uploaded_size = $1, update_time = NOW() WHERE id = $2")
        .bind(new_size)
        .bind(task_id)
        .execute(&state.db)
        .await?;

    Ok(axum::Json(ApiResponse::success(serde_json::json!({
        "task_id": task_id,
        "chunk_index": chunk_index,
        "offset": new_size,
        "uploaded_size": new_size
    }))))
}

async fn complete_upload(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Json(payload): axum::extract::Json<serde_json::Value>,
) -> AppResult<impl axum::response::IntoResponse> {
    let user_id = guard::current_user_id(&headers, &state.config.jwt.secret)?;

    let task_id = payload
        .get("task_id")
        .and_then(|v| v.as_i64())
        .ok_or_else(|| crate::error::AppError::BadRequest("Missing task_id".to_string()))?;

    let task = sqlx::query_as::<_, (String, i64, String, i64, i64)>(
        "SELECT task_no, file_size, remote_path, uploaded_size, host_id FROM upload_task WHERE id = $1 AND user_id = $2",
    )
    .bind(task_id)
    .bind(user_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| crate::error::AppError::NotFound("Task not found".to_string()))?;

    if task.1 != task.3 {
        return Err(crate::error::AppError::BadRequest(
            "File not fully uploaded".to_string(),
        ));
    }

    let task_dir = PathBuf::from(&state.config.storage.temp_dir).join(&task.0);

    let mut chunks: Vec<_> = std::fs::read_dir(&task_dir)
        .map_err(|e| crate::error::AppError::Internal(e.to_string()))?
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().to_string();
            if !name.starts_with("chunk_") {
                return None;
            }
            let index = name.trim_start_matches("chunk_").parse::<i32>().ok()?;
            Some((index, entry.path()))
        })
        .collect();
    chunks.sort_by_key(|(idx, _)| *idx);

    let merged_path = task_dir.join("merged_upload.bin");
    let mut final_file = fs::File::create(&merged_path)
        .map_err(|e| crate::error::AppError::Internal(e.to_string()))?;

    for (_, chunk_path) in chunks {
        let data = std::fs::read(chunk_path)
            .map_err(|e| crate::error::AppError::Internal(e.to_string()))?;
        std::io::Write::write_all(&mut final_file, &data)
            .map_err(|e| crate::error::AppError::Internal(e.to_string()))?;
    }

    let payload = fs::read(&merged_path).map_err(|e| AppError::Internal(e.to_string()))?;
    let host_ssh_config = ssh_client::resolve_host_ssh_config(
        &state.db,
        &state.config.secrets.data_encryption_key,
        Some(user_id),
        task.4,
    )
    .await?;
    ssh_client::upload_files(
        host_ssh_config,
        state.config.ssh.connection_timeout,
        vec![(task.2.clone(), payload)],
    )
    .await?;

    std::fs::remove_dir_all(&task_dir).ok();

    sqlx::query("UPDATE upload_task SET status = 2, update_time = NOW() WHERE id = $1")
        .bind(task_id)
        .execute(&state.db)
        .await?;

    append_operator_log(
        &state,
        &headers,
        user_id,
        "sftp_complete_upload_task",
        serde_json::json!({
            "task_id": task_id,
            "host_id": task.4,
            "remote_path": task.2,
            "file_size": task.1,
        }),
        1,
        None,
    )
    .await?;

    Ok(axum::Json(ApiResponse::success(serde_json::json!({
        "task_id": task_id,
        "status": "completed"
    }))))
}

#[cfg(test)]
mod tests {
    use super::{join_remote_path, normalize_relative_path, normalize_remote_base_path};

    #[test]
    fn normalize_remote_base_path_rejects_relative() {
        assert!(normalize_remote_base_path("tmp/files").is_err());
    }

    #[test]
    fn normalize_remote_base_path_compacts_segments() {
        let out = normalize_remote_base_path("/var//log/./app").expect("normalize success");
        assert_eq!(out, "/var/log/app");
    }

    #[test]
    fn normalize_relative_path_rejects_parent_traversal() {
        assert!(normalize_relative_path("../secret.txt").is_err());
    }

    #[test]
    fn normalize_relative_path_keeps_nested_path() {
        let out = normalize_relative_path("dir/sub/file.txt").expect("normalize success");
        assert_eq!(out, "dir/sub/file.txt");
    }

    #[test]
    fn join_remote_path_handles_root_base() {
        let out = join_remote_path("/", "dir/file.txt");
        assert_eq!(out, "/dir/file.txt");
    }
}

async fn list_upload_tasks(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<impl axum::response::IntoResponse> {
    let user_id = guard::current_user_id(&headers, &state.config.jwt.secret)?;
    let is_admin = has_host_read_permission(&state, user_id).await?;

    let tasks = sqlx::query_as::<_, (i64, String, i64, i64, i64, i16, String)>(if is_admin {
        "SELECT id, task_no, host_id, file_size, uploaded_size, status, create_time::text FROM upload_task ORDER BY id DESC LIMIT 50"
    } else {
        "SELECT id, task_no, host_id, file_size, uploaded_size, status, create_time::text FROM upload_task WHERE user_id = $1 ORDER BY id DESC LIMIT 50"
    });
    let tasks = if is_admin {
        tasks.fetch_all(&state.db).await?
    } else {
        tasks.bind(user_id).fetch_all(&state.db).await?
    };

    Ok(axum::Json(ApiResponse::success(tasks)))
}

async fn get_upload_task(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Path(id): axum::extract::Path<i64>,
) -> AppResult<impl axum::response::IntoResponse> {
    let user_id = guard::current_user_id(&headers, &state.config.jwt.secret)?;
    let is_admin = has_host_read_permission(&state, user_id).await?;

    let task = if is_admin {
        sqlx::query_as::<_, (i64, String, i64, i64, i64, i16, String)>(
            "SELECT id, task_no, host_id, file_size, uploaded_size, status, create_time::text FROM upload_task WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&state.db)
        .await?
    } else {
        sqlx::query_as::<_, (i64, String, i64, i64, i64, i16, String)>(
            "SELECT id, task_no, host_id, file_size, uploaded_size, status, create_time::text FROM upload_task WHERE id = $1 AND user_id = $2",
        )
        .bind(id)
        .bind(user_id)
        .fetch_optional(&state.db)
        .await?
    }
    .ok_or_else(|| crate::error::AppError::NotFound("Task not found".to_string()))?;

    Ok(axum::Json(ApiResponse::success(task)))
}

use axum::{extract::State, http::HeaderMap, routing::get, Router};

use crate::{
    api::{guard, AppState},
    error::{AppError, AppResult},
    model::*,
    security,
};

fn normalize_credential_type(raw: Option<&str>) -> Result<Option<String>, AppError> {
    let Some(value) = raw.map(str::trim).filter(|v| !v.is_empty()) else {
        return Ok(None);
    };

    match value {
        "none" => Ok(None),
        "password" => Ok(Some("password".to_string())),
        "private_key" => Ok(Some("private_key".to_string())),
        "ssh_key" => Ok(Some("ssh_key".to_string())),
        _ => Err(AppError::BadRequest(
            "credential_type must be one of: none, password, private_key, ssh_key".to_string(),
        )),
    }
}

fn build_encrypted_credential_payload(
    credential_type: Option<&str>,
    credential_data: Option<&str>,
    credential_passphrase: Option<&str>,
    username: Option<&str>,
    encryption_key: &str,
) -> Result<Option<String>, AppError> {
    let normalized = normalize_credential_type(credential_type)?;

    match normalized.as_deref() {
        None => {
            if credential_data
                .map(str::trim)
                .map(|v| !v.is_empty())
                .unwrap_or(false)
            {
                return Err(AppError::BadRequest(
                    "credential_data requires credential_type=password or private_key".to_string(),
                ));
            }
            Ok(None)
        }
        Some("ssh_key") => {
            if credential_data
                .map(str::trim)
                .map(|v| !v.is_empty())
                .unwrap_or(false)
            {
                return Err(AppError::BadRequest(
                    "credential_data is not allowed for credential_type=ssh_key".to_string(),
                ));
            }
            Ok(None)
        }
        Some(kind) => {
            let username_empty = username
                .map(str::trim)
                .map(|v| v.is_empty())
                .unwrap_or(true);
            if username_empty {
                return Err(AppError::BadRequest(
                    "username is required when credentials are configured".to_string(),
                ));
            }

            let secret = credential_data
                .map(str::trim)
                .filter(|v| !v.is_empty())
                .ok_or_else(|| {
                    AppError::BadRequest(
                        "credential_data is required when credential_type is password/private_key"
                            .to_string(),
                    )
                })?;

            let secret_json = if kind == "password" {
                serde_json::json!({
                    "type": "password",
                    "password": secret,
                })
            } else {
                serde_json::json!({
                    "type": "private_key",
                    "private_key": secret,
                    "passphrase": credential_passphrase
                        .map(str::trim)
                        .filter(|v| !v.is_empty()),
                })
            };

            let encrypted = security::encrypt_secret(&secret_json.to_string(), encryption_key)?;
            Ok(Some(encrypted))
        }
    }
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/hosts", get(list_hosts).post(create_host))
        .route(
            "/hosts/:id",
            get(get_host).put(update_host).delete(delete_host),
        )
        .route(
            "/host-groups",
            get(list_host_groups).post(create_host_group),
        )
        .route(
            "/host-groups/:id",
            get(get_host_group)
                .put(update_host_group)
                .delete(delete_host_group),
        )
}

async fn list_hosts(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "host.read").await?;

    let hosts = sqlx::query_as::<
        _,
        (
            i64,
            String,
            String,
            i32,
            Option<String>,
            Option<String>,
            Option<i64>,
            Option<String>,
            Option<String>,
            i16,
            String,
        ),
    >(
        "SELECT
            h.id,
            h.name,
            h.hostname,
            h.port,
            h.username,
            h.credential_type,
            (
                SELECT hb.ssh_key_id
                FROM host_ssh_key_binding hb
                WHERE hb.host_id = h.id AND hb.is_default = 1
                LIMIT 1
            ) AS ssh_key_id,
            h.description,
            h.tags::text,
            h.status,
            h.create_time::text
         FROM host h
         WHERE h.deleted = 0
         ORDER BY h.id DESC",
    )
    .fetch_all(&state.db)
    .await?;

    let list: Vec<HostResponse> = hosts
        .into_iter()
        .map(|h| HostResponse {
            id: h.0,
            name: h.1,
            hostname: h.2,
            port: h.3,
            username: h.4,
            credential_type: h.5,
            ssh_key_id: h.6,
            description: h.7,
            tags: h.8.and_then(|t| serde_json::from_str(&t).ok()),
            status: h.9,
            create_time: h.10,
        })
        .collect();

    Ok(axum::Json(ApiResponse::success(list)))
}

async fn create_host(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Json(payload): axum::extract::Json<CreateHostRequest>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "host.create").await?;

    if payload.name.trim().is_empty() {
        return Err(crate::error::AppError::BadRequest(
            "host name is required".to_string(),
        ));
    }
    if payload.hostname.trim().is_empty() {
        return Err(crate::error::AppError::BadRequest(
            "host hostname is required".to_string(),
        ));
    }

    if payload.credential_passphrase.is_some()
        && payload.credential_type.as_deref().map(str::trim) != Some("private_key")
    {
        return Err(AppError::BadRequest(
            "credential_passphrase can only be used with credential_type=private_key".to_string(),
        ));
    }

    if payload.ssh_key_id.is_some()
        && payload.credential_type.as_deref().map(str::trim) != Some("ssh_key")
    {
        return Err(AppError::BadRequest(
            "ssh_key_id requires credential_type=ssh_key".to_string(),
        ));
    }

    let normalized_credential_type = normalize_credential_type(payload.credential_type.as_deref())?;
    let encrypted_credential_data = build_encrypted_credential_payload(
        normalized_credential_type.as_deref(),
        payload.credential_data.as_deref(),
        payload.credential_passphrase.as_deref(),
        payload.username.as_deref(),
        &state.config.secrets.data_encryption_key,
    )?;

    let selected_ssh_key_id = if normalized_credential_type.as_deref() == Some("ssh_key") {
        let key_id = payload.ssh_key_id.ok_or_else(|| {
            AppError::BadRequest("ssh_key_id is required for credential_type=ssh_key".to_string())
        })?;
        if key_id <= 0 {
            return Err(AppError::BadRequest(
                "ssh_key_id must be greater than 0".to_string(),
            ));
        }
        let exists = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM ssh_key WHERE id = $1 AND deleted = 0 AND status = 1)",
        )
        .bind(key_id)
        .fetch_one(&state.db)
        .await?;
        if !exists {
            return Err(AppError::NotFound(
                "SSH key not found or disabled".to_string(),
            ));
        }
        Some(key_id)
    } else {
        None
    };

    let mut tx = state.db.begin().await?;
    let new_id = sqlx::query_scalar::<_, i64>(
        "INSERT INTO host (name, hostname, port, username, credential_type, credential_data, description, tags, status, create_time, update_time) VALUES ($1, $2, $3, $4, $5, $6, $7, $8::jsonb, 1, NOW(), NOW()) RETURNING id"
    )
    .bind(&payload.name)
    .bind(&payload.hostname)
    .bind(payload.port.unwrap_or(22))
    .bind(&payload.username)
    .bind(&normalized_credential_type)
    .bind(&encrypted_credential_data)
    .bind(&payload.description)
    .bind(payload.tags.map(|t| t.to_string()))
    .fetch_one(&mut *tx)
    .await?;

    if let Some(ssh_key_id) = selected_ssh_key_id {
        sqlx::query(
            "INSERT INTO host_ssh_key_binding (host_id, ssh_key_id, is_default, create_time)
             VALUES ($1, $2, 1, NOW())",
        )
        .bind(new_id)
        .bind(ssh_key_id)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;

    Ok(axum::Json(ApiResponse::success(serde_json::json!({
        "id": new_id
    }))))
}

async fn get_host(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Path(id): axum::extract::Path<i64>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "host.read").await?;

    let host = sqlx::query_as::<
        _,
        (
            i64,
            String,
            String,
            i32,
            Option<String>,
            Option<String>,
            Option<i64>,
            Option<String>,
            Option<String>,
            i16,
            String,
        ),
    >(
        "SELECT
            h.id,
            h.name,
            h.hostname,
            h.port,
            h.username,
            h.credential_type,
            (
                SELECT hb.ssh_key_id
                FROM host_ssh_key_binding hb
                WHERE hb.host_id = h.id AND hb.is_default = 1
                LIMIT 1
            ) AS ssh_key_id,
            h.description,
            h.tags::text,
            h.status,
            h.create_time::text
         FROM host h
         WHERE h.id = $1 AND h.deleted = 0",
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| crate::error::AppError::NotFound("Host not found".to_string()))?;

    Ok(axum::Json(ApiResponse::success(HostResponse {
        id: host.0,
        name: host.1,
        hostname: host.2,
        port: host.3,
        username: host.4,
        credential_type: host.5,
        ssh_key_id: host.6,
        description: host.7,
        tags: host.8.and_then(|t| serde_json::from_str(&t).ok()),
        status: host.9,
        create_time: host.10,
    })))
}

async fn update_host(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Path(id): axum::extract::Path<i64>,
    axum::extract::Json(payload): axum::extract::Json<UpdateHostRequest>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "host.update").await?;

    let has_any_update = payload.name.is_some()
        || payload.hostname.is_some()
        || payload.port.is_some()
        || payload.username.is_some()
        || payload.credential_type.is_some()
        || payload.credential_data.is_some()
        || payload.ssh_key_id.is_some()
        || payload.description.is_some()
        || payload.tags.is_some()
        || payload.status.is_some()
        || payload.credential_passphrase.is_some();

    if !has_any_update {
        return Err(crate::error::AppError::BadRequest(
            "No fields to update".to_string(),
        ));
    }

    if payload.credential_data.is_some() && payload.credential_type.is_none() {
        return Err(AppError::BadRequest(
            "credential_type is required when credential_data is provided".to_string(),
        ));
    }

    if payload.credential_passphrase.is_some()
        && payload.credential_type.as_deref().map(str::trim) != Some("private_key")
    {
        return Err(AppError::BadRequest(
            "credential_passphrase can only be used with credential_type=private_key".to_string(),
        ));
    }

    if payload.ssh_key_id.is_some()
        && payload
            .credential_type
            .as_deref()
            .map(str::trim)
            .is_some_and(|t| t != "ssh_key")
    {
        return Err(AppError::BadRequest(
            "ssh_key_id requires credential_type=ssh_key".to_string(),
        ));
    }

    let tags = payload.tags.map(|t| t.to_string());
    let credential_type_provided = payload.credential_type.is_some();
    let normalized_credential_type = normalize_credential_type(payload.credential_type.as_deref())?;

    let selected_ssh_key_id = match payload.ssh_key_id {
        Some(v) if v > 0 => Some(v),
        Some(_) => {
            return Err(AppError::BadRequest(
                "ssh_key_id must be greater than 0".to_string(),
            ))
        }
        None => None,
    };

    if let Some(key_id) = selected_ssh_key_id {
        let exists = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM ssh_key WHERE id = $1 AND deleted = 0 AND status = 1)",
        )
        .bind(key_id)
        .fetch_one(&state.db)
        .await?;
        if !exists {
            return Err(AppError::NotFound(
                "SSH key not found or disabled".to_string(),
            ));
        }
    }

    let final_credential_type = if selected_ssh_key_id.is_some() && !credential_type_provided {
        Some("ssh_key".to_string())
    } else {
        normalized_credential_type
    };

    let (credential_data_provided, encrypted_credential_data) = if credential_type_provided {
        if final_credential_type.is_none() {
            (true, None)
        } else {
            let encrypted = build_encrypted_credential_payload(
                final_credential_type.as_deref(),
                payload.credential_data.as_deref(),
                payload.credential_passphrase.as_deref(),
                payload.username.as_deref(),
                &state.config.secrets.data_encryption_key,
            )?;
            (true, encrypted)
        }
    } else if selected_ssh_key_id.is_some() {
        (true, None)
    } else {
        (false, None)
    };

    let mut tx = state.db.begin().await?;
    let result = sqlx::query(
        "UPDATE host SET
            name = COALESCE($1, name),
            hostname = COALESCE($2, hostname),
            port = COALESCE($3, port),
            username = COALESCE($4, username),
            credential_type = CASE WHEN $10 THEN $5 ELSE credential_type END,
            credential_data = CASE WHEN $11 THEN $6 ELSE credential_data END,
            description = COALESCE($7, description),
            tags = COALESCE($8::jsonb, tags),
            status = COALESCE($9, status),
            update_time = NOW()
         WHERE id = $12 AND deleted = 0",
    )
    .bind(payload.name)
    .bind(payload.hostname)
    .bind(payload.port)
    .bind(payload.username)
    .bind(final_credential_type.clone())
    .bind(encrypted_credential_data)
    .bind(payload.description)
    .bind(tags)
    .bind(payload.status)
    .bind(credential_type_provided || selected_ssh_key_id.is_some())
    .bind(credential_data_provided)
    .bind(id)
    .execute(&mut *tx)
    .await?;

    if result.rows_affected() == 0 {
        return Err(crate::error::AppError::NotFound(
            "Host not found".to_string(),
        ));
    }

    if let Some(ssh_key_id) = selected_ssh_key_id {
        sqlx::query("DELETE FROM host_ssh_key_binding WHERE host_id = $1")
            .bind(id)
            .execute(&mut *tx)
            .await?;
        sqlx::query(
            "INSERT INTO host_ssh_key_binding (host_id, ssh_key_id, is_default, create_time)
             VALUES ($1, $2, 1, NOW())",
        )
        .bind(id)
        .bind(ssh_key_id)
        .execute(&mut *tx)
        .await?;
    } else if credential_type_provided && final_credential_type.as_deref() != Some("ssh_key") {
        sqlx::query("DELETE FROM host_ssh_key_binding WHERE host_id = $1")
            .bind(id)
            .execute(&mut *tx)
            .await?;
    }

    tx.commit().await?;

    Ok(axum::Json(ApiResponse::success("Updated")))
}

async fn delete_host(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Path(id): axum::extract::Path<i64>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "host.delete").await?;

    let result = sqlx::query(
        "UPDATE host SET deleted = 1, update_time = NOW() WHERE id = $1 AND deleted = 0",
    )
    .bind(id)
    .execute(&state.db)
    .await?;

    if result.rows_affected() == 0 {
        return Err(crate::error::AppError::NotFound(
            "Host not found".to_string(),
        ));
    }

    Ok(axum::Json(ApiResponse::success("Deleted")))
}

async fn list_host_groups(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "host.read").await?;

    let groups = sqlx::query_as::<_, (i64, String, Option<i64>, Option<String>, Option<i32>, String)>(
        "SELECT id, name, parent_id, description, sort, create_time::text FROM host_group WHERE deleted = 0 ORDER BY sort, id"
    )
    .fetch_all(&state.db)
    .await?;

    Ok(axum::Json(ApiResponse::success(groups)))
}

async fn create_host_group(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Json(payload): axum::extract::Json<serde_json::Value>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "host.create").await?;

    let name = payload
        .get("name")
        .and_then(|v| v.as_str())
        .map(|v| v.trim())
        .unwrap_or("");
    if name.is_empty() {
        return Err(crate::error::AppError::BadRequest(
            "host group name is required".to_string(),
        ));
    }

    let new_id = sqlx::query_scalar::<_, i64>(
        "INSERT INTO host_group (name, parent_id, description, sort, create_time) VALUES ($1, $2, $3, $4, NOW()) RETURNING id"
    )
    .bind(name)
    .bind(payload.get("parent_id").and_then(|v| v.as_i64()))
    .bind(payload.get("description").and_then(|v| v.as_str()))
    .bind(payload.get("sort").and_then(|v| v.as_i64()).unwrap_or(0))
    .fetch_one(&state.db)
    .await?;

    Ok(axum::Json(ApiResponse::success(serde_json::json!({
        "id": new_id
    }))))
}

async fn get_host_group(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Path(id): axum::extract::Path<i64>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "host.read").await?;

    let group = sqlx::query_as::<_, (i64, String, Option<i64>, Option<String>, Option<i32>, String)>(
        "SELECT id, name, parent_id, description, sort, create_time::text FROM host_group WHERE id = $1 AND deleted = 0"
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| crate::error::AppError::NotFound("Host group not found".to_string()))?;

    Ok(axum::Json(ApiResponse::success(group)))
}

async fn update_host_group(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Path(id): axum::extract::Path<i64>,
    axum::extract::Json(payload): axum::extract::Json<serde_json::Value>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "host.update").await?;

    let result = sqlx::query("UPDATE host_group SET name = COALESCE($1, name), parent_id = COALESCE($2, parent_id), description = COALESCE($3, description), sort = COALESCE($4, sort) WHERE id = $5 AND deleted = 0")
        .bind(payload.get("name").and_then(|v| v.as_str()))
        .bind(payload.get("parent_id").and_then(|v| v.as_i64()))
        .bind(payload.get("description").and_then(|v| v.as_str()))
        .bind(payload.get("sort").and_then(|v| v.as_i64()))
        .bind(id)
        .execute(&state.db)
        .await?;

    if result.rows_affected() == 0 {
        return Err(crate::error::AppError::NotFound(
            "Host group not found".to_string(),
        ));
    }

    Ok(axum::Json(ApiResponse::success("Updated")))
}

async fn delete_host_group(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Path(id): axum::extract::Path<i64>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "host.delete").await?;

    let result = sqlx::query("UPDATE host_group SET deleted = 1 WHERE id = $1 AND deleted = 0")
        .bind(id)
        .execute(&state.db)
        .await?;

    if result.rows_affected() == 0 {
        return Err(crate::error::AppError::NotFound(
            "Host group not found".to_string(),
        ));
    }

    Ok(axum::Json(ApiResponse::success("Deleted")))
}

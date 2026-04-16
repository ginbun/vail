use axum::{
    extract::{Path, State},
    http::HeaderMap,
    routing::{delete, get},
    Json, Router,
};

use crate::{
    api::{guard, AppState},
    error::{AppError, AppResult},
    model::{
        ApiResponse, BindHostSshKeyRequest, CreateSshKeyRequest, SshKeyResponse,
        UpdateSshKeyRequest,
    },
    security,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/ssh-keys", get(list_ssh_keys).post(create_ssh_key))
        .route(
            "/ssh-keys/:id",
            get(get_ssh_key).put(update_ssh_key).delete(delete_ssh_key),
        )
        .route(
            "/hosts/:host_id/ssh-keys",
            get(list_host_ssh_keys).post(bind_host_ssh_key),
        )
        .route(
            "/hosts/:host_id/ssh-keys/:ssh_key_id",
            delete(unbind_host_ssh_key),
        )
}

fn normalize_status(v: Option<i16>) -> Result<Option<i16>, AppError> {
    match v {
        Some(0 | 1) => Ok(v),
        Some(_) => Err(AppError::BadRequest("status must be 0 or 1".to_string())),
        None => Ok(None),
    }
}

async fn list_ssh_keys(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "sshkey.read").await?;

    let rows = sqlx::query_as::<_, (i64, String, Option<String>, i16, bool, String, i64)>(
        "SELECT
            k.id,
            k.name,
            k.description,
            k.status,
            (k.passphrase_ciphertext IS NOT NULL) AS has_passphrase,
            k.create_time::text,
            COALESCE(COUNT(hb.host_id), 0)::bigint AS bound_host_count
         FROM ssh_key k
         LEFT JOIN host_ssh_key_binding hb ON hb.ssh_key_id = k.id
         WHERE k.deleted = 0
         GROUP BY k.id
         ORDER BY k.id DESC",
    )
    .fetch_all(&state.db)
    .await?;

    let list = rows
        .into_iter()
        .map(|r| SshKeyResponse {
            id: r.0,
            name: r.1,
            description: r.2,
            status: r.3,
            has_passphrase: r.4,
            create_time: r.5,
            bound_host_count: r.6,
        })
        .collect::<Vec<_>>();

    Ok(Json(ApiResponse::success(list)))
}

async fn get_ssh_key(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "sshkey.read").await?;

    let row = sqlx::query_as::<_, (i64, String, Option<String>, i16, bool, String, i64)>(
        "SELECT
            k.id,
            k.name,
            k.description,
            k.status,
            (k.passphrase_ciphertext IS NOT NULL) AS has_passphrase,
            k.create_time::text,
            COALESCE(COUNT(hb.host_id), 0)::bigint AS bound_host_count
         FROM ssh_key k
         LEFT JOIN host_ssh_key_binding hb ON hb.ssh_key_id = k.id
         WHERE k.id = $1 AND k.deleted = 0
         GROUP BY k.id",
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound("SSH key not found".to_string()))?;

    Ok(Json(ApiResponse::success(SshKeyResponse {
        id: row.0,
        name: row.1,
        description: row.2,
        status: row.3,
        has_passphrase: row.4,
        create_time: row.5,
        bound_host_count: row.6,
    })))
}

async fn create_ssh_key(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<CreateSshKeyRequest>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "sshkey.create").await?;

    let name = payload.name.trim();
    if name.is_empty() {
        return Err(AppError::BadRequest("ssh key name is required".to_string()));
    }

    let private_key = payload.private_key.trim();
    if private_key.is_empty() {
        return Err(AppError::BadRequest("private_key is required".to_string()));
    }

    let private_key_ciphertext =
        security::encrypt_secret(private_key, &state.config.secrets.data_encryption_key)?;
    let passphrase_ciphertext = match payload.passphrase.as_deref().map(str::trim) {
        Some(v) if !v.is_empty() => Some(security::encrypt_secret(
            v,
            &state.config.secrets.data_encryption_key,
        )?),
        _ => None,
    };

    let new_id = sqlx::query_scalar::<_, i64>(
        "INSERT INTO ssh_key (
            name,
            private_key_ciphertext,
            passphrase_ciphertext,
            description,
            status,
            create_time,
            update_time,
            deleted
        ) VALUES ($1, $2, $3, $4, 1, NOW(), NOW(), 0)
        RETURNING id",
    )
    .bind(name)
    .bind(private_key_ciphertext)
    .bind(passphrase_ciphertext)
    .bind(payload.description)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(ApiResponse::success(
        serde_json::json!({ "id": new_id }),
    )))
}

async fn update_ssh_key(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(payload): Json<UpdateSshKeyRequest>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "sshkey.update").await?;

    let has_any_update = payload.name.is_some()
        || payload.private_key.is_some()
        || payload.passphrase.is_some()
        || payload.description.is_some()
        || payload.status.is_some();
    if !has_any_update {
        return Err(AppError::BadRequest("No fields to update".to_string()));
    }

    let status = normalize_status(payload.status)?;
    let private_key_ciphertext = match payload.private_key.as_deref() {
        Some(v) => {
            let t = v.trim();
            if t.is_empty() {
                return Err(AppError::BadRequest(
                    "private_key cannot be empty".to_string(),
                ));
            }
            Some(security::encrypt_secret(
                t,
                &state.config.secrets.data_encryption_key,
            )?)
        }
        None => None,
    };

    let passphrase_ciphertext = match payload.passphrase.as_deref() {
        Some(v) => {
            let t = v.trim();
            if t.is_empty() {
                Some(None)
            } else {
                Some(Some(security::encrypt_secret(
                    t,
                    &state.config.secrets.data_encryption_key,
                )?))
            }
        }
        None => None,
    };
    let passphrase_provided = passphrase_ciphertext.is_some();
    let passphrase_ciphertext_value = passphrase_ciphertext.flatten();

    let result = sqlx::query(
        "UPDATE ssh_key SET
            name = COALESCE($1, name),
            private_key_ciphertext = COALESCE($2, private_key_ciphertext),
            passphrase_ciphertext = CASE WHEN $6 THEN $3 ELSE passphrase_ciphertext END,
            description = COALESCE($4, description),
            status = COALESCE($5, status),
            update_time = NOW()
         WHERE id = $7 AND deleted = 0",
    )
    .bind(payload.name)
    .bind(private_key_ciphertext)
    .bind(passphrase_ciphertext_value)
    .bind(payload.description)
    .bind(status)
    .bind(passphrase_provided)
    .bind(id)
    .execute(&state.db)
    .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("SSH key not found".to_string()));
    }

    Ok(Json(ApiResponse::success("Updated")))
}

async fn delete_ssh_key(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "sshkey.delete").await?;

    let mut tx = state.db.begin().await?;
    let result = sqlx::query(
        "UPDATE ssh_key SET deleted = 1, update_time = NOW() WHERE id = $1 AND deleted = 0",
    )
    .bind(id)
    .execute(&mut *tx)
    .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("SSH key not found".to_string()));
    }

    sqlx::query("DELETE FROM host_ssh_key_binding WHERE ssh_key_id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;

    Ok(Json(ApiResponse::success("Deleted")))
}

async fn list_host_ssh_keys(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(host_id): Path<i64>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "sshkey.read").await?;

    let rows = sqlx::query_as::<_, (i64, String, Option<String>, i16, bool, String, i64)>(
        "SELECT
            k.id,
            k.name,
            k.description,
            k.status,
            (k.passphrase_ciphertext IS NOT NULL) AS has_passphrase,
            k.create_time::text,
            1::bigint AS bound_host_count
         FROM host_ssh_key_binding hb
         JOIN ssh_key k ON k.id = hb.ssh_key_id
         JOIN host h ON h.id = hb.host_id
         WHERE hb.host_id = $1 AND h.deleted = 0 AND k.deleted = 0
         ORDER BY k.id DESC",
    )
    .bind(host_id)
    .fetch_all(&state.db)
    .await?;

    let list = rows
        .into_iter()
        .map(|r| SshKeyResponse {
            id: r.0,
            name: r.1,
            description: r.2,
            status: r.3,
            has_passphrase: r.4,
            create_time: r.5,
            bound_host_count: r.6,
        })
        .collect::<Vec<_>>();

    Ok(Json(ApiResponse::success(list)))
}

async fn bind_host_ssh_key(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(host_id): Path<i64>,
    Json(payload): Json<BindHostSshKeyRequest>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "sshkey.create").await?;

    if payload.ssh_key_id <= 0 || host_id <= 0 {
        return Err(AppError::BadRequest(
            "host_id and ssh_key_id must be greater than 0".to_string(),
        ));
    }

    let host_exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM host WHERE id = $1 AND deleted = 0)",
    )
    .bind(host_id)
    .fetch_one(&state.db)
    .await?;
    if !host_exists {
        return Err(AppError::NotFound("Host not found".to_string()));
    }

    let key_exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM ssh_key WHERE id = $1 AND deleted = 0 AND status = 1)",
    )
    .bind(payload.ssh_key_id)
    .fetch_one(&state.db)
    .await?;
    if !key_exists {
        return Err(AppError::NotFound(
            "SSH key not found or disabled".to_string(),
        ));
    }

    let mut tx = state.db.begin().await?;
    sqlx::query("DELETE FROM host_ssh_key_binding WHERE host_id = $1 AND is_default = 1")
        .bind(host_id)
        .execute(&mut *tx)
        .await?;

    sqlx::query(
        "INSERT INTO host_ssh_key_binding (host_id, ssh_key_id, is_default, create_time)
         VALUES ($1, $2, 1, NOW())
         ON CONFLICT (host_id, ssh_key_id) DO UPDATE SET is_default = 1",
    )
    .bind(host_id)
    .bind(payload.ssh_key_id)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        "UPDATE host
         SET credential_type = 'ssh_key', credential_data = NULL, update_time = NOW()
         WHERE id = $1 AND deleted = 0",
    )
    .bind(host_id)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(Json(ApiResponse::success(serde_json::json!({
        "host_id": host_id,
        "ssh_key_id": payload.ssh_key_id
    }))))
}

async fn unbind_host_ssh_key(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((host_id, ssh_key_id)): Path<(i64, i64)>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "sshkey.delete").await?;

    let mut tx = state.db.begin().await?;
    let result =
        sqlx::query("DELETE FROM host_ssh_key_binding WHERE host_id = $1 AND ssh_key_id = $2")
            .bind(host_id)
            .bind(ssh_key_id)
            .execute(&mut *tx)
            .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("Host-key binding not found".to_string()));
    }

    sqlx::query(
        "UPDATE host
         SET credential_type = CASE
             WHEN NOT EXISTS (SELECT 1 FROM host_ssh_key_binding WHERE host_id = $1) THEN NULL
             ELSE credential_type
         END,
         update_time = NOW()
         WHERE id = $1 AND deleted = 0",
    )
    .bind(host_id)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(Json(ApiResponse::success("Unbound")))
}

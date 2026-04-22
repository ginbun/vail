use std::collections::BTreeSet;

use axum::{
    extract::{Path, State},
    http::HeaderMap,
    routing::{get, post},
    Json, Router,
};

use crate::{
    api::{guard, AppState},
    application::orion::audit_service,
    error::{AppError, AppResult},
    model::*,
};


pub fn router() -> Router<AppState> {
    Router::new()
        .route("/iam/users", get(list_users))
        .route("/iam/roles", get(list_roles))
        .route("/iam/resources/hosts", get(list_assignable_hosts))
        .route("/iam/users/:id/roles", post(assign_user_roles))
        .route("/iam/users/:id/resources/hosts", post(assign_user_hosts))
        .route("/iam/users/:id/bindings", get(get_user_bindings))
        .route("/iam/users/:id/permissions", get(get_user_permissions))
        .route("/iam/me/summary", get(get_me_summary))
        .route("/iam/me/hosts", get(get_me_hosts))
}

async fn list_users(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "iam.user-permission.view").await?;

    let rows = sqlx::query_as::<_, (i64, String, Option<String>, i16)>(
        "SELECT id, username, nickname, status
         FROM sys_user
         WHERE deleted = 0
         ORDER BY id ASC",
    )
    .fetch_all(&state.db)
    .await?;

    let users = rows
        .into_iter()
        .map(|row| crate::model::IamUserListItem {
            id: row.0,
            username: row.1,
            nickname: row.2,
            status: row.3,
        })
        .collect::<Vec<_>>();

    Ok(Json(ApiResponse::success(users)))
}

async fn list_roles(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "iam.user-permission.view").await?;

    let rows = sqlx::query_as::<_, (i64, String, String)>(
        "SELECT id, code, name
         FROM sys_role
         WHERE deleted = 0
           AND status = 1
         ORDER BY id ASC",
    )
    .fetch_all(&state.db)
    .await?;

    let roles = rows
        .into_iter()
        .map(|row| crate::model::IamRoleListItem {
            id: row.0,
            code: row.1,
            name: row.2,
        })
        .collect::<Vec<_>>();

    Ok(Json(ApiResponse::success(roles)))
}

async fn list_assignable_hosts(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "iam.user-resource.assign").await?;

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

    Ok(Json(ApiResponse::success(list)))
}

fn normalize_ids(ids: Vec<i64>) -> Vec<i64> {
    ids.into_iter()
        .filter(|v| *v > 0)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

async fn assign_user_roles(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(user_id): Path<i64>,
    axum::extract::Json(payload): axum::extract::Json<AssignUserRolesRequest>,
) -> AppResult<impl axum::response::IntoResponse> {
    let actor_user_id = guard::require_permission(&state, &headers, "iam.user-role.assign").await?;

    if user_id <= 0 {
        return Err(AppError::BadRequest(
            "user_id must be greater than 0".to_string(),
        ));
    }

    let user_exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM sys_user WHERE id = $1 AND deleted = 0)",
    )
    .bind(user_id)
    .fetch_one(&state.db)
    .await?;
    if !user_exists {
        return Err(AppError::NotFound("User not found".to_string()));
    }

    let role_ids = normalize_ids(payload.role_ids);

    if !role_ids.is_empty() {
        let active_role_ids = sqlx::query_scalar::<_, i64>(
            "SELECT id
             FROM sys_role
             WHERE id = ANY($1::bigint[])
               AND deleted = 0
               AND status = 1",
        )
        .bind(&role_ids)
        .fetch_all(&state.db)
        .await?;

        let active_set: BTreeSet<i64> = active_role_ids.into_iter().collect();
        let missing_or_inactive: Vec<i64> = role_ids
            .iter()
            .copied()
            .filter(|id| !active_set.contains(id))
            .collect();

        if !missing_or_inactive.is_empty() {
            return Err(AppError::BadRequest(format!(
                "Invalid or inactive role_ids: {:?}",
                missing_or_inactive
            )));
        }
    }

    let mut tx = state.db.begin().await?;
    sqlx::query("DELETE FROM sys_user_role WHERE user_id = $1")
        .bind(user_id)
        .execute(&mut *tx)
        .await?;

    if !role_ids.is_empty() {
        sqlx::query(
            "INSERT INTO sys_user_role (user_id, role_id, create_time)
             SELECT $1, role_id, NOW()
             FROM unnest($2::bigint[]) AS role_id",
        )
        .bind(user_id)
        .bind(&role_ids)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;

    audit_service::log_operator_action(
        &state.db,
        &headers,
        audit_service::OperatorLogParams {
            user_id: actor_user_id,
            module: "iam",
            operation: "assign_user_roles",
            params: serde_json::json!({
                "target_user_id": user_id,
                "role_ids": role_ids,
            }),
            result: 1,
            error_message: None,
        },
    )
    .await?;

    Ok(Json(ApiResponse::success(serde_json::json!({
        "user_id": user_id,
        "role_ids": role_ids
    }))))
}

async fn assign_user_hosts(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(user_id): Path<i64>,
    axum::extract::Json(payload): axum::extract::Json<AssignUserHostsRequest>,
) -> AppResult<impl axum::response::IntoResponse> {
    let actor_user_id =
        guard::require_permission(&state, &headers, "iam.user-resource.assign").await?;

    if user_id <= 0 {
        return Err(AppError::BadRequest(
            "user_id must be greater than 0".to_string(),
        ));
    }

    let user_exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM sys_user WHERE id = $1 AND deleted = 0)",
    )
    .bind(user_id)
    .fetch_one(&state.db)
    .await?;
    if !user_exists {
        return Err(AppError::NotFound("User not found".to_string()));
    }

    let host_ids = normalize_ids(payload.host_ids);

    if !host_ids.is_empty() {
        let active_host_ids = sqlx::query_scalar::<_, i64>(
            "SELECT id
             FROM host
             WHERE id = ANY($1::bigint[])
               AND deleted = 0
               AND status = 1",
        )
        .bind(&host_ids)
        .fetch_all(&state.db)
        .await?;

        let active_set: BTreeSet<i64> = active_host_ids.into_iter().collect();
        let missing_or_inactive: Vec<i64> = host_ids
            .iter()
            .copied()
            .filter(|id| !active_set.contains(id))
            .collect();

        if !missing_or_inactive.is_empty() {
            return Err(AppError::BadRequest(format!(
                "Invalid or inactive host_ids: {:?}",
                missing_or_inactive
            )));
        }
    }

    let mut tx = state.db.begin().await?;
    sqlx::query("DELETE FROM user_host_access WHERE user_id = $1")
        .bind(user_id)
        .execute(&mut *tx)
        .await?;

    if !host_ids.is_empty() {
        sqlx::query(
            "INSERT INTO user_host_access (user_id, host_id, create_time)
             SELECT $1, host_id, NOW()
             FROM unnest($2::bigint[]) AS host_id",
        )
        .bind(user_id)
        .bind(&host_ids)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;

    audit_service::log_operator_action(
        &state.db,
        &headers,
        audit_service::OperatorLogParams {
            user_id: actor_user_id,
            module: "iam",
            operation: "assign_user_hosts",
            params: serde_json::json!({
                "target_user_id": user_id,
                "host_ids": host_ids,
            }),
            result: 1,
            error_message: None,
        },
    )
    .await?;

    Ok(Json(ApiResponse::success(serde_json::json!({
        "user_id": user_id,
        "host_ids": host_ids
    }))))
}

async fn get_user_permissions(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(user_id): Path<i64>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "iam.user-permission.view").await?;

    if user_id <= 0 {
        return Err(AppError::BadRequest(
            "user_id must be greater than 0".to_string(),
        ));
    }

    let user_exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM sys_user WHERE id = $1 AND deleted = 0)",
    )
    .bind(user_id)
    .fetch_one(&state.db)
    .await?;
    if !user_exists {
        return Err(AppError::NotFound("User not found".to_string()));
    }

    let role_codes = sqlx::query_scalar::<_, String>(
        "SELECT r.code
         FROM sys_user_role ur
         JOIN sys_role r ON r.id = ur.role_id
         WHERE ur.user_id = $1 AND r.deleted = 0 AND r.status = 1
         ORDER BY r.code",
    )
    .bind(user_id)
    .fetch_all(&state.db)
    .await?;

    let permission_codes = sqlx::query_scalar::<_, String>(
        "SELECT DISTINCT p.code
         FROM sys_user_role ur
         JOIN sys_role r ON r.id = ur.role_id AND r.deleted = 0 AND r.status = 1
         JOIN sys_role_permission rp ON rp.role_id = ur.role_id
         JOIN sys_permission p ON p.id = rp.permission_id
         WHERE ur.user_id = $1
         ORDER BY p.code",
    )
    .bind(user_id)
    .fetch_all(&state.db)
    .await?;

    let host_ids = sqlx::query_scalar::<_, i64>(
        "SELECT host_id
         FROM user_host_access
         WHERE user_id = $1
         ORDER BY host_id",
    )
    .bind(user_id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(ApiResponse::success(IamUserPermissionsResponse {
        user_id,
        roles: role_codes,
        permissions: permission_codes,
        host_ids,
    })))
}

async fn get_user_bindings(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(user_id): Path<i64>,
) -> AppResult<impl axum::response::IntoResponse> {
    guard::require_permission(&state, &headers, "iam.user-permission.view").await?;

    if user_id <= 0 {
        return Err(AppError::BadRequest(
            "user_id must be greater than 0".to_string(),
        ));
    }

    let user_exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM sys_user WHERE id = $1 AND deleted = 0)",
    )
    .bind(user_id)
    .fetch_one(&state.db)
    .await?;
    if !user_exists {
        return Err(AppError::NotFound("User not found".to_string()));
    }

    let role_ids = sqlx::query_scalar::<_, i64>(
        "SELECT ur.role_id
         FROM sys_user_role ur
         JOIN sys_role r ON r.id = ur.role_id
         WHERE ur.user_id = $1
           AND r.deleted = 0
           AND r.status = 1
         ORDER BY ur.role_id",
    )
    .bind(user_id)
    .fetch_all(&state.db)
    .await?;

    let host_ids = sqlx::query_scalar::<_, i64>(
        "SELECT uha.host_id
         FROM user_host_access uha
         JOIN host h ON h.id = uha.host_id
         WHERE uha.user_id = $1
           AND h.deleted = 0
         ORDER BY uha.host_id",
    )
    .bind(user_id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(ApiResponse::success(
        crate::model::IamUserBindingsResponse {
            user_id,
            role_ids,
            host_ids,
        },
    )))
}

async fn get_me_summary(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<impl axum::response::IntoResponse> {
    let user_id = guard::current_user_id(&headers, &state.config.jwt)?;

    let role_codes = sqlx::query_scalar::<_, String>(
        "SELECT r.code
         FROM sys_user_role ur
         JOIN sys_role r ON r.id = ur.role_id
         WHERE ur.user_id = $1 AND r.deleted = 0 AND r.status = 1
         ORDER BY r.code",
    )
    .bind(user_id)
    .fetch_all(&state.db)
    .await?;

    let permission_codes = sqlx::query_scalar::<_, String>(
        "SELECT DISTINCT p.code
         FROM sys_user_role ur
         JOIN sys_role r ON r.id = ur.role_id AND r.deleted = 0 AND r.status = 1
         JOIN sys_role_permission rp ON rp.role_id = ur.role_id
         JOIN sys_permission p ON p.id = rp.permission_id
         WHERE ur.user_id = $1
         ORDER BY p.code",
    )
    .bind(user_id)
    .fetch_all(&state.db)
    .await?;

    let is_admin = guard::has_host_read_permission(&state, user_id).await?;

    let host_access_count = if is_admin {
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM host WHERE deleted = 0",
        )
        .fetch_one(&state.db)
        .await?
    } else {
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(DISTINCT h.id)
             FROM host h
             LEFT JOIN user_host_access uha ON h.id = uha.host_id AND uha.user_id = $1
             LEFT JOIN host_group_rel hgr ON h.id = hgr.host_id
             LEFT JOIN host_group hg ON hgr.group_id = hg.id AND hg.deleted = 0
             LEFT JOIN user_host_group_grant uhgg ON hg.id = uhgg.group_id AND uhgg.user_id = $1
             LEFT JOIN role_host_group_grant rhgg ON hg.id = rhgg.group_id
             LEFT JOIN sys_user_role ur ON rhgg.role_id = ur.role_id AND ur.user_id = $1
             LEFT JOIN sys_role r ON ur.role_id = r.id AND r.deleted = 0 AND r.status = 1
             WHERE h.deleted = 0
               AND (uha.user_id IS NOT NULL OR uhgg.user_id IS NOT NULL OR r.id IS NOT NULL)",
        )
        .bind(user_id)
        .fetch_one(&state.db)
        .await?
    };

    Ok(Json(ApiResponse::success(IamMeSummaryResponse {
        user_id,
        roles: role_codes,
        permissions: permission_codes,
        host_access_count,
    })))
}

async fn get_me_hosts(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<impl axum::response::IntoResponse> {
    let user_id = guard::current_user_id(&headers, &state.config.jwt)?;
    let is_admin = guard::has_host_read_permission(&state, user_id).await?;

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
        "SELECT DISTINCT
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
         LEFT JOIN user_host_access uha ON h.id = uha.host_id AND uha.user_id = $1
         LEFT JOIN host_group_rel hgr ON h.id = hgr.host_id
         LEFT JOIN host_group hg ON hgr.group_id = hg.id AND hg.deleted = 0
         LEFT JOIN user_host_group_grant uhgg ON hg.id = uhgg.group_id AND uhgg.user_id = $1
         LEFT JOIN role_host_group_grant rhgg ON hg.id = rhgg.group_id
         LEFT JOIN sys_user_role ur ON rhgg.role_id = ur.role_id AND ur.user_id = $1
         LEFT JOIN sys_role r ON ur.role_id = r.id AND r.deleted = 0 AND r.status = 1
         WHERE h.deleted = 0
           AND ($2 = TRUE OR uha.user_id IS NOT NULL OR uhgg.user_id IS NOT NULL OR r.id IS NOT NULL)
         ORDER BY h.id DESC",
    )
    .bind(user_id)
    .bind(is_admin)
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

    Ok(Json(ApiResponse::success(list)))
}

#[cfg(test)]
mod tests {
    use super::normalize_ids;

    #[test]
    fn normalize_ids_removes_duplicates_and_invalid_values() {
        let ids = vec![5, 2, 5, 0, -1, 2, 9];
        assert_eq!(normalize_ids(ids), vec![2, 5, 9]);
    }
}

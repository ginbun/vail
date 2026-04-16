use axum::{
    extract::State,
    http::HeaderMap,
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use totp_rs::{Algorithm, Secret, TOTP};
use uuid::Uuid;

use crate::{
    api::AppState,
    error::{AppError, AppResult},
    model::*,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/auth/login", post(login))
        .route("/auth/mfa/totp/verify", post(verify_totp_login))
        .route("/auth/logout", post(logout))
        .route("/auth/refresh", post(refresh))
        .route("/auth/me", get(me))
}

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String,
    user_id: i64,
    session_id: String,
    exp: i64,
    iat: i64,
}

struct TokenPair {
    access_token: String,
    refresh_token: String,
    expires_in: u64,
}

pub fn create_token(
    user_id: i64,
    username: &str,
    session_id: &str,
    secret: &str,
    expiration: u64,
) -> String {
    let now = Utc::now();
    let exp = now + Duration::seconds(expiration as i64);

    let claims = Claims {
        sub: username.to_string(),
        user_id,
        session_id: session_id.to_string(),
        exp: exp.timestamp(),
        iat: now.timestamp(),
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .unwrap()
}

fn verify_token(token: &str, secret: &str) -> AppResult<Claims> {
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )
    .map_err(|e| crate::error::AppError::Auth(e.to_string()))?;

    Ok(token_data.claims)
}

fn get_source_ip(headers: &HeaderMap) -> String {
    headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(',').next())
        .map(|v| v.trim().to_string())
        .or_else(|| {
            headers
                .get("x-real-ip")
                .and_then(|v| v.to_str().ok())
                .map(|v| v.to_string())
        })
        .unwrap_or_else(|| "0.0.0.0".to_string())
}

fn hash_refresh_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn generate_refresh_token() -> String {
    format!("{}.{}", Uuid::new_v4(), Uuid::new_v4())
}

fn verify_totp_code(secret_encoded: &str, code: &str) -> bool {
    let Ok(secret_bytes) = Secret::Encoded(secret_encoded.to_string()).to_bytes() else {
        return false;
    };

    let Ok(totp) = TOTP::new(Algorithm::SHA1, 6, 1, 30, secret_bytes) else {
        return false;
    };

    totp.check_current(code).unwrap_or(false)
}

async fn issue_token_pair(
    state: &AppState,
    user_id: i64,
    username: &str,
    session_id: Option<String>,
) -> AppResult<TokenPair> {
    let session_id = session_id.unwrap_or_else(|| Uuid::new_v4().to_string());
    let access_token = create_token(
        user_id,
        username,
        &session_id,
        &state.config.jwt.secret,
        state.config.jwt.expiration,
    );
    let refresh_token = generate_refresh_token();
    let refresh_hash = hash_refresh_token(&refresh_token);

    sqlx::query(
        "INSERT INTO auth_refresh_token (user_id, token_hash, session_id, expires_at) VALUES ($1, $2, $3::uuid, NOW() + ($4 || ' seconds')::interval)",
    )
    .bind(user_id)
    .bind(refresh_hash)
    .bind(&session_id)
    .bind(state.config.jwt.refresh_expiration as i64)
    .execute(&state.db)
    .await?;

    Ok(TokenPair {
        access_token,
        refresh_token,
        expires_in: state.config.jwt.expiration,
    })
}

async fn login(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Json(payload): axum::extract::Json<LoginRequest>,
) -> AppResult<impl IntoResponse> {
    let source_ip = get_source_ip(&headers);

    let user = sqlx::query_as::<_, (i64, String, String, Option<String>, bool)>(
        "SELECT u.id, u.username, u.password, u.nickname, COALESCE(m.enabled, false) AS mfa_enabled FROM sys_user u LEFT JOIN user_mfa_totp m ON m.user_id = u.id WHERE u.username = $1 AND u.deleted = 0",
    )
    .bind(&payload.username)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::Auth("User not found".to_string()))?;

    if !bcrypt::verify(&payload.password, &user.2).unwrap_or(false) {
        sqlx::query(
            "INSERT INTO login_log (user_id, username, ip, result, error_message, create_time) VALUES ($1, $2, $3, 0, $4, NOW())",
        )
        .bind(Option::<i64>::None)
        .bind(&payload.username)
        .bind(&source_ip)
        .bind("invalid password")
        .execute(&state.db)
        .await
        .ok();

        return Err(AppError::Auth("Invalid password".to_string()));
    }

    if user.4 {
        let challenge_id = Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO auth_login_challenge (id, user_id, source_ip, expires_at) VALUES ($1::uuid, $2, $3, NOW() + interval '5 minutes')",
        )
        .bind(&challenge_id)
        .bind(user.0)
        .bind(&source_ip)
        .execute(&state.db)
        .await?;

        return Ok(axum::Json(ApiResponse::success(LoginResponse {
            mfa_required: true,
            login_challenge_id: Some(challenge_id),
            challenge_expires_in: Some(300),
            access_token: None,
            refresh_token: None,
            expires_in: None,
            user: None,
        })));
    }

    let pair = issue_token_pair(&state, user.0, &user.1, None).await?;

    sqlx::query("UPDATE sys_user SET last_login_time = NOW(), last_login_ip = $1 WHERE id = $2")
        .bind(&source_ip)
        .bind(user.0)
        .execute(&state.db)
        .await?;

    sqlx::query(
        "INSERT INTO login_log (user_id, username, ip, result, create_time) VALUES ($1, $2, $3, 1, NOW())"
    )
    .bind(user.0)
    .bind(&user.1)
    .bind(&source_ip)
    .execute(&state.db)
    .await?;

    Ok(axum::Json(ApiResponse::success(LoginResponse {
        mfa_required: false,
        login_challenge_id: None,
        challenge_expires_in: None,
        access_token: Some(pair.access_token),
        refresh_token: Some(pair.refresh_token),
        expires_in: Some(pair.expires_in),
        user: Some(UserInfo {
            id: user.0,
            username: user.1,
            nickname: user.3,
            avatar: None,
            email: None,
        }),
    })))
}

async fn verify_totp_login(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Json(payload): axum::extract::Json<TotpVerifyRequest>,
) -> AppResult<impl IntoResponse> {
    let source_ip = get_source_ip(&headers);

    let challenge = sqlx::query_as::<_, (i64, String, Option<String>, String, i16)>(
        "SELECT c.user_id, u.username, u.nickname, m.secret_ciphertext, c.attempts FROM auth_login_challenge c JOIN sys_user u ON u.id = c.user_id JOIN user_mfa_totp m ON m.user_id = u.id WHERE c.id = $1::uuid AND c.used_at IS NULL AND c.expires_at > NOW() AND m.enabled = TRUE",
    )
    .bind(&payload.login_challenge_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::Auth("Invalid or expired login challenge".to_string()))?;

    if challenge.4 >= 5 {
        return Err(AppError::Auth("Too many MFA attempts".to_string()));
    }

    if !verify_totp_code(&challenge.3, &payload.code) {
        sqlx::query("UPDATE auth_login_challenge SET attempts = attempts + 1 WHERE id = $1::uuid")
            .bind(&payload.login_challenge_id)
            .execute(&state.db)
            .await?;
        return Err(AppError::Auth("Invalid MFA code".to_string()));
    }

    sqlx::query("UPDATE auth_login_challenge SET used_at = NOW() WHERE id = $1::uuid")
        .bind(&payload.login_challenge_id)
        .execute(&state.db)
        .await?;

    let pair = issue_token_pair(&state, challenge.0, &challenge.1, None).await?;

    sqlx::query("UPDATE sys_user SET last_login_time = NOW(), last_login_ip = $1 WHERE id = $2")
        .bind(&source_ip)
        .bind(challenge.0)
        .execute(&state.db)
        .await?;

    sqlx::query("INSERT INTO login_log (user_id, username, ip, result, create_time) VALUES ($1, $2, $3, 1, NOW())")
        .bind(challenge.0)
        .bind(&challenge.1)
        .bind(&source_ip)
        .execute(&state.db)
        .await?;

    Ok(axum::Json(ApiResponse::success(LoginResponse {
        mfa_required: false,
        login_challenge_id: None,
        challenge_expires_in: None,
        access_token: Some(pair.access_token),
        refresh_token: Some(pair.refresh_token),
        expires_in: Some(pair.expires_in),
        user: Some(UserInfo {
            id: challenge.0,
            username: challenge.1,
            nickname: challenge.2,
            avatar: None,
            email: None,
        }),
    })))
}

async fn logout(
    State(state): State<AppState>,
    axum::extract::Json(payload): axum::extract::Json<LogoutRequest>,
) -> AppResult<impl IntoResponse> {
    if let Some(refresh_token) = payload.refresh_token {
        let refresh_hash = hash_refresh_token(&refresh_token);
        sqlx::query(
            "UPDATE auth_refresh_token SET revoked_at = NOW() WHERE token_hash = $1 AND revoked_at IS NULL",
        )
        .bind(refresh_hash)
        .execute(&state.db)
        .await?;
    }

    Ok(axum::Json(ApiResponse::success("Logged out")))
}

async fn refresh(
    State(state): State<AppState>,
    axum::extract::Json(payload): axum::extract::Json<RefreshRequest>,
) -> AppResult<impl IntoResponse> {
    let refresh_hash = hash_refresh_token(&payload.refresh_token);

    let refresh_row = sqlx::query_as::<_, (i64, i64, String, String)>(
        "SELECT r.id, r.user_id, u.username, r.session_id::text FROM auth_refresh_token r JOIN sys_user u ON u.id = r.user_id WHERE r.token_hash = $1 AND r.revoked_at IS NULL AND r.expires_at > NOW()",
    )
    .bind(refresh_hash)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::Auth("Invalid refresh token".to_string()))?;

    sqlx::query(
        "UPDATE auth_refresh_token SET revoked_at = NOW(), rotated_at = NOW() WHERE id = $1",
    )
    .bind(refresh_row.0)
    .execute(&state.db)
    .await?;

    let pair = issue_token_pair(&state, refresh_row.1, &refresh_row.2, Some(refresh_row.3)).await?;

    Ok(axum::Json(ApiResponse::success(RefreshResponse {
        access_token: pair.access_token,
        refresh_token: pair.refresh_token,
        expires_in: pair.expires_in,
    })))
}

async fn me(State(state): State<AppState>, headers: HeaderMap) -> AppResult<impl IntoResponse> {
    let token = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or_else(|| crate::error::AppError::Auth("Missing token".to_string()))?;

    let claims = verify_token(token, &state.config.jwt.secret)?;

    let user = sqlx::query_as::<_, (i64, String, Option<String>, Option<String>, Option<String>)>(
        "SELECT id, username, nickname, avatar, email FROM sys_user WHERE id = $1 AND deleted = 0",
    )
    .bind(claims.user_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| crate::error::AppError::Auth("User not found".to_string()))?;

    Ok(axum::Json(ApiResponse::success(UserInfo {
        id: user.0,
        username: user.1,
        nickname: user.2,
        avatar: user.3,
        email: user.4,
    })))
}

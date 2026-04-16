use axum::http::HeaderMap;
use jsonwebtoken::{decode, DecodingKey, Validation};
use serde::Deserialize;

use crate::{
    api::AppState,
    error::{AppError, AppResult},
};

#[derive(Debug, Deserialize)]
struct GuardClaims {
    user_id: i64,
}

fn bearer_token(headers: &HeaderMap) -> AppResult<&str> {
    headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or_else(|| AppError::Auth("Missing token".to_string()))
}

fn parse_user_id(token: &str, secret: &str) -> AppResult<i64> {
    let claims = decode::<GuardClaims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )
    .map_err(|e| AppError::Auth(e.to_string()))?;

    Ok(claims.claims.user_id)
}

pub fn current_user_id(headers: &HeaderMap, secret: &str) -> AppResult<i64> {
    let token = bearer_token(headers)?;
    parse_user_id(token, secret)
}

pub async fn require_permission(
    state: &AppState,
    headers: &HeaderMap,
    permission_code: &str,
) -> AppResult<i64> {
    let user_id = current_user_id(headers, &state.config.jwt.secret)?;

    let allowed = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(
            SELECT 1
            FROM sys_user_role ur
            JOIN sys_role r ON r.id = ur.role_id AND r.deleted = 0 AND r.status = 1
            JOIN sys_role_permission rp ON rp.role_id = ur.role_id
            JOIN sys_permission p ON p.id = rp.permission_id
            WHERE ur.user_id = $1 AND p.code = $2
        )",
    )
    .bind(user_id)
    .bind(permission_code)
    .fetch_one(&state.db)
    .await?;

    if !allowed {
        return Err(AppError::Auth("Permission denied".to_string()));
    }

    Ok(user_id)
}

#[cfg(test)]
mod tests {
    use axum::http::HeaderValue;
    use jsonwebtoken::{encode, EncodingKey, Header};
    use serde::Serialize;

    use super::*;

    #[derive(Serialize)]
    struct TestClaims {
        user_id: i64,
        exp: i64,
        iat: i64,
        sub: String,
        session_id: String,
    }

    #[test]
    fn bearer_token_extracts_value() {
        let mut headers = HeaderMap::new();
        headers.insert("authorization", HeaderValue::from_static("Bearer abc.def"));
        let token = bearer_token(&headers).expect("token should exist");
        assert_eq!(token, "abc.def");
    }

    #[test]
    fn parse_user_id_decodes_jwt_claim() {
        let secret = "test-secret";
        let claims = TestClaims {
            user_id: 42,
            exp: 4_102_444_800,
            iat: 1_700_000_000,
            sub: "demo".to_string(),
            session_id: "session".to_string(),
        };
        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(secret.as_bytes()),
        )
        .expect("jwt encode");

        let user_id = parse_user_id(&token, secret).expect("jwt parse");
        assert_eq!(user_id, 42);
    }
}

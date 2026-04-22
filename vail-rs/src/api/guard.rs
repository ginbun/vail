use axum::http::HeaderMap;
use jsonwebtoken::{decode, decode_header, Validation};
use serde::Deserialize;

use crate::{
    api::AppState,
    config::JwtConfig,
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

fn parse_user_id(token: &str, jwt: &JwtConfig) -> AppResult<i64> {
    let header = decode_header(token).map_err(|e| AppError::Auth(e.to_string()))?;
    let validation = Validation::new(jwt.algorithm);

    if header.alg != jwt.algorithm {
        return Err(AppError::Auth(format!(
            "Unexpected jwt algorithm in token: {:?}",
            header.alg
        )));
    }

    let key = jwt.verification_key().map_err(AppError::Auth)?;

    let claims = decode::<GuardClaims>(token, &key, &validation)
        .map_err(|e| AppError::Auth(e.to_string()))?;

    Ok(claims.claims.user_id)
}

pub fn current_user_id(headers: &HeaderMap, jwt: &JwtConfig) -> AppResult<i64> {
    let token = bearer_token(headers)?;
    parse_user_id(token, jwt)
}

pub async fn require_permission(
    state: &AppState,
    headers: &HeaderMap,
    permission_code: &str,
) -> AppResult<i64> {
    let user_id = current_user_id(headers, &state.config.jwt)?;

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
        return Err(AppError::Forbidden("Permission denied".to_string()));
    }

    Ok(user_id)
}

pub async fn has_host_read_permission(state: &AppState, user_id: i64) -> AppResult<bool> {
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

pub async fn check_host_permission(
    state: &AppState,
    user_id: i64,
    host_id: i64,
) -> AppResult<bool> {
    // 1. Check host.read permission
    let allowed = has_host_read_permission(state, user_id).await?;

    // 2. Check specific host access
    let access = if allowed {
        sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM host WHERE id = $1 AND deleted = 0 AND status = 1)",
        )
        .bind(host_id)
        .fetch_one(&state.db)
        .await?
    } else {
        sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(
                SELECT 1
                FROM host h
                WHERE h.id = $2 AND h.deleted = 0 AND h.status = 1
                AND (
                    -- Direct user-to-host access
                    EXISTS(SELECT 1 FROM user_host_access WHERE user_id = $1 AND host_id = $2)
                    OR
                    -- Access via host group (direct user grant)
                    EXISTS(
                        SELECT 1 
                        FROM host_group_rel hgr
                        JOIN host_group hg ON hgr.group_id = hg.id AND hg.deleted = 0
                        JOIN user_host_group_grant uhgg ON hg.id = uhgg.group_id
                        WHERE hgr.host_id = $2 AND uhgg.user_id = $1
                    )
                    OR
                    -- Access via host group (role-based grant)
                    EXISTS(
                        SELECT 1
                        FROM host_group_rel hgr
                        JOIN host_group hg ON hgr.group_id = hg.id AND hg.deleted = 0
                        JOIN role_host_group_grant rhgg ON hg.id = rhgg.group_id
                        JOIN sys_user_role ur ON rhgg.role_id = ur.role_id
                        JOIN sys_role r ON r.id = ur.role_id AND r.deleted = 0 AND r.status = 1
                        WHERE hgr.host_id = $2 AND ur.user_id = $1
                    )
                )
            )",
        )
        .bind(user_id)
        .bind(host_id)
        .fetch_one(&state.db)
        .await?
    };

    Ok(access)
}

pub async fn require_host_permission(
    state: &AppState,
    user_id: i64,
    host_id: i64,
) -> AppResult<()> {
    if !check_host_permission(state, user_id, host_id).await? {
        return Err(AppError::Forbidden("Host access denied".to_string()));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use axum::http::HeaderValue;
    use base64::{engine::general_purpose::STANDARD, Engine as _};
    use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
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

    const TEST_ED25519_PRIVATE_KEY: &str =
        "MC4CAQAwBQYDK2VwBCIEIHCDX8ke/yslwa9SElPghVHhz700q1H6SO9hmUJ6i8Ld";
    const TEST_ED25519_PUBLIC_KEY: &str = "sA29J+hOVKaDdV0/Ksm2B3zFrbDqFphgTpO79LTQ4zk=";

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

        let jwt = JwtConfig {
            algorithm: Algorithm::HS256,
            secret: secret.to_string(),
            private_key: String::new(),
            public_key: String::new(),
            expiration: 3600,
            refresh_expiration: 604800,
        };

        let user_id = parse_user_id(&token, &jwt).expect("jwt parse");
        assert_eq!(user_id, 42);
    }

    #[test]
    fn parse_user_id_decodes_eddsa_jwt_claim() {
        let claims = TestClaims {
            user_id: 7,
            exp: 4_102_444_800,
            iat: 1_700_000_000,
            sub: "demo".to_string(),
            session_id: "session".to_string(),
        };

        let mut header = Header::new(Algorithm::EdDSA);
        header.typ = Some("JWT".to_string());
        let private_der = STANDARD
            .decode(TEST_ED25519_PRIVATE_KEY)
            .expect("base64 der");
        let token =
            encode(&header, &claims, &EncodingKey::from_ed_der(&private_der)).expect("jwt encode");

        let jwt = JwtConfig {
            algorithm: Algorithm::EdDSA,
            secret: String::new(),
            private_key: TEST_ED25519_PRIVATE_KEY.to_string(),
            public_key: TEST_ED25519_PUBLIC_KEY.to_string(),
            expiration: 3600,
            refresh_expiration: 604800,
        };

        let user_id = parse_user_id(&token, &jwt).expect("jwt parse");
        assert_eq!(user_id, 7);
    }
}

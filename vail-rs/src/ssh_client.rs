use std::{io::Write, net::TcpStream, path::Path, time::Duration};

use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use ssh_key::HashAlg;

use crate::{error::AppError, security};

#[derive(Clone, Debug)]
pub enum HostAuthMethod {
    Password(String),
    PrivateKey {
        private_key: String,
        passphrase: Option<String>,
    },
}

#[derive(Clone, Debug)]
pub struct HostSshConfig {
    pub host_id: i64,
    pub hostname: String,
    pub port: u16,
    pub username: String,
    pub auth: HostAuthMethod,
}

#[derive(Deserialize)]
struct StoredCredentialPayload {
    #[serde(rename = "type")]
    kind: String,
    password: Option<String>,
    private_key: Option<String>,
    passphrase: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct HostSshExtraSetting {
    auth_type: Option<String>,
    username: Option<String>,
    key_id: Option<i64>,
    identity_id: Option<i64>,
}

const EXTRA_AUTH_DEFAULT: &str = "DEFAULT";
const EXTRA_AUTH_CUSTOM_KEY: &str = "CUSTOM_KEY";
const EXTRA_AUTH_CUSTOM_IDENTITY: &str = "CUSTOM_IDENTITY";

fn host_extra_key(user_id: i64, host_id: i64, item: &str) -> String {
    format!(
        "orion:host-extra:user:{user_id}:host:{host_id}:item:{}",
        item.trim().to_ascii_uppercase()
    )
}

async fn has_authorized_key(
    db: &sqlx::PgPool,
    user_id: i64,
    key_id: i64,
) -> Result<bool, AppError> {
    let allowed = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(
            SELECT 1
            FROM user_host_key_grant ug
            WHERE ug.user_id = $1 AND ug.key_id = $2
            UNION
            SELECT 1
            FROM role_host_key_grant rg
            WHERE rg.key_id = $2
              AND rg.role_id IN (SELECT role_id FROM sys_user_role WHERE user_id = $1)
        )",
    )
    .bind(user_id)
    .bind(key_id)
    .fetch_one(db)
    .await?;
    Ok(allowed)
}

async fn has_authorized_identity(
    db: &sqlx::PgPool,
    user_id: i64,
    identity_id: i64,
) -> Result<bool, AppError> {
    let allowed = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(
            SELECT 1
            FROM user_host_identity_grant ug
            WHERE ug.user_id = $1 AND ug.identity_id = $2
            UNION
            SELECT 1
            FROM role_host_identity_grant rg
            WHERE rg.identity_id = $2
              AND rg.role_id IN (SELECT role_id FROM sys_user_role WHERE user_id = $1)
        )",
    )
    .bind(user_id)
    .bind(identity_id)
    .fetch_one(db)
    .await?;
    Ok(allowed)
}

async fn has_host_read_permission(db: &sqlx::PgPool, user_id: i64) -> Result<bool, AppError> {
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
    .fetch_one(db)
    .await?;
    Ok(allowed)
}

async fn load_ssh_key_auth(
    db: &sqlx::PgPool,
    encryption_key: &str,
    key_id: i64,
) -> Result<HostAuthMethod, AppError> {
    let row = sqlx::query_as::<_, (String, Option<String>)>(
        "SELECT private_key_ciphertext, passphrase_ciphertext
         FROM ssh_key
         WHERE id = $1 AND deleted = 0 AND status = 1",
    )
    .bind(key_id)
    .fetch_optional(db)
    .await?
    .ok_or_else(|| AppError::NotFound("SSH key not found or disabled".to_string()))?;

    let private_key = security::decrypt_secret(&row.0, encryption_key)?;
    let passphrase = match row.1.as_deref() {
        Some(v) if !v.trim().is_empty() => Some(security::decrypt_secret(v, encryption_key)?),
        _ => None,
    };

    Ok(HostAuthMethod::PrivateKey {
        private_key,
        passphrase,
    })
}

fn normalize_username(raw: Option<String>) -> Option<String> {
    raw.as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(str::to_string)
}

pub async fn resolve_host_ssh_config(
    db: &sqlx::PgPool,
    encryption_key: &str,
    user_id: Option<i64>,
    host_id: i64,
) -> Result<HostSshConfig, AppError> {
    let row = sqlx::query_as::<
        _,
        (
            i64,
            String,
            i32,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<i64>,
        ),
    >(
        "SELECT
            h.id,
            h.hostname,
            h.port,
            h.username,
            h.credential_type,
            h.credential_data,
            k.private_key_ciphertext,
            k.passphrase_ciphertext,
            hb.ssh_key_id
         FROM host h
         LEFT JOIN host_ssh_key_binding hb ON hb.host_id = h.id AND hb.is_default = 1
         LEFT JOIN ssh_key k ON k.id = hb.ssh_key_id AND k.deleted = 0 AND k.status = 1
         WHERE h.id = $1 AND h.deleted = 0 AND h.status = 1",
    )
    .bind(host_id)
    .fetch_optional(db)
    .await?
    .ok_or_else(|| AppError::NotFound("Host not found or disabled".to_string()))?;

    if row.2 <= 0 || row.2 > u16::MAX as i32 {
        return Err(AppError::BadRequest("Invalid host port".to_string()));
    }

    let mut username = row
        .3
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .ok_or_else(|| AppError::BadRequest("Host username is required".to_string()))?
        .to_string();

    let mut auth_source = "host".to_string();
    let mut selected_key_id: Option<i64> = None;
    let mut selected_identity_id: Option<i64> = None;

    let auth = match row.4.as_deref() {
        Some("password") | Some("private_key") => {
            let credential_data = row.5.as_deref().ok_or_else(|| {
                AppError::BadRequest("Host credential_data is missing".to_string())
            })?;
            let plain = security::decrypt_secret(credential_data, encryption_key)?;
            let payload: StoredCredentialPayload = serde_json::from_str(&plain).map_err(|_| {
                AppError::BadRequest("Host credential_data format is invalid".to_string())
            })?;
            match payload.kind.as_str() {
                "password" => {
                    let password = payload
                        .password
                        .as_deref()
                        .map(str::trim)
                        .filter(|v| !v.is_empty())
                        .ok_or_else(|| {
                            AppError::BadRequest("Host password credential is empty".to_string())
                        })?;
                    HostAuthMethod::Password(password.to_string())
                }
                "private_key" => {
                    let private_key = payload
                        .private_key
                        .as_deref()
                        .map(str::trim)
                        .filter(|v| !v.is_empty())
                        .ok_or_else(|| {
                            AppError::BadRequest("Host private key credential is empty".to_string())
                        })?;
                    HostAuthMethod::PrivateKey {
                        private_key: private_key.to_string(),
                        passphrase: payload.passphrase,
                    }
                }
                _ => {
                    return Err(AppError::BadRequest(
                        "Unsupported host credential payload type".to_string(),
                    ))
                }
            }
        }
        Some("ssh_key") => {
            selected_key_id = row.8;
            let private_key_ciphertext = row.6.as_deref().ok_or_else(|| {
                AppError::BadRequest("Host has no active default SSH key binding".to_string())
            })?;
            let private_key = security::decrypt_secret(private_key_ciphertext, encryption_key)?;
            let passphrase = match row.7.as_deref() {
                Some(v) if !v.trim().is_empty() => {
                    Some(security::decrypt_secret(v, encryption_key)?)
                }
                _ => None,
            };
            HostAuthMethod::PrivateKey {
                private_key,
                passphrase,
            }
        }
        _ => {
            return Err(AppError::BadRequest(
                "Host does not have usable SSH credential".to_string(),
            ))
        }
    };

    let mut auth = auth;

    if let Some(uid) = user_id {
        let cache_key = host_extra_key(uid, host_id, "SSH");
        if let Some(extra_json) = sqlx::query_scalar::<_, String>(
            "SELECT cache_value FROM cache
             WHERE cache_key = $1
               AND (expire_time IS NULL OR expire_time > NOW())",
        )
        .bind(&cache_key)
        .fetch_optional(db)
        .await?
        {
            let parsed: Option<HostSshExtraSetting> = serde_json::from_str::<Value>(&extra_json)
                .ok()
                .and_then(|v| serde_json::from_value(v).ok());

            if let Some(extra) = parsed {
                let auth_type = extra
                    .auth_type
                    .as_deref()
                    .map(str::trim)
                    .filter(|v| !v.is_empty())
                    .unwrap_or(EXTRA_AUTH_DEFAULT);

                match auth_type {
                    EXTRA_AUTH_CUSTOM_KEY => {
                        let key_id = extra.key_id.ok_or_else(|| {
                            AppError::BadRequest("SSH extra config keyId is required".to_string())
                        })?;
                        if key_id <= 0 {
                            return Err(AppError::BadRequest(
                                "SSH extra config keyId must be greater than 0".to_string(),
                            ));
                        }

                        let is_admin = has_host_read_permission(db, uid).await?;
                        if !is_admin && !has_authorized_key(db, uid, key_id).await? {
                            return Err(AppError::Auth(
                                "SSH key is not authorized for current user".to_string(),
                            ));
                        }

                        if let Some(v) = normalize_username(extra.username) {
                            username = v;
                        }
                        auth_source = "extra_custom_key".to_string();
                        selected_key_id = Some(key_id);
                        selected_identity_id = None;
                        auth = load_ssh_key_auth(db, encryption_key, key_id).await?;
                    }
                    EXTRA_AUTH_CUSTOM_IDENTITY => {
                        let identity_id = extra.identity_id.ok_or_else(|| {
                            AppError::BadRequest(
                                "SSH extra config identityId is required".to_string(),
                            )
                        })?;
                        if identity_id <= 0 {
                            return Err(AppError::BadRequest(
                                "SSH extra config identityId must be greater than 0".to_string(),
                            ));
                        }

                        let is_admin = has_host_read_permission(db, uid).await?;
                        if !is_admin && !has_authorized_identity(db, uid, identity_id).await? {
                            return Err(AppError::Auth(
                                "Host identity is not authorized for current user".to_string(),
                            ));
                        }

                        let identity = sqlx::query_as::<
                            _,
                            (String, Option<String>, Option<String>, Option<i64>),
                        >(
                            "SELECT type, username, password_ciphertext, key_id
                             FROM host_identity
                             WHERE id = $1 AND deleted = 0 AND status = 1",
                        )
                        .bind(identity_id)
                        .fetch_optional(db)
                        .await?
                        .ok_or_else(|| {
                            AppError::NotFound("Host identity not found or disabled".to_string())
                        })?;

                        auth_source = "extra_custom_identity".to_string();
                        selected_identity_id = Some(identity_id);
                        selected_key_id = None;

                        if let Some(v) = normalize_username(identity.1.clone()) {
                            username = v;
                        }

                        match identity.0.as_str() {
                            "PASSWORD" => {
                                let ciphertext = identity.2.as_deref().ok_or_else(|| {
                                    AppError::BadRequest(
                                        "Host identity password is missing".to_string(),
                                    )
                                })?;
                                let password =
                                    security::decrypt_secret(ciphertext, encryption_key)?;
                                auth = HostAuthMethod::Password(password);
                            }
                            "KEY" => {
                                let key_id = identity.3.ok_or_else(|| {
                                    AppError::BadRequest("Host identity key is missing".to_string())
                                })?;
                                selected_key_id = Some(key_id);
                                auth = load_ssh_key_auth(db, encryption_key, key_id).await?;
                            }
                            _ => {
                                return Err(AppError::BadRequest(
                                    "Host identity type is invalid".to_string(),
                                ))
                            }
                        }
                    }
                    EXTRA_AUTH_DEFAULT => {}
                    _ => {
                        return Err(AppError::BadRequest(
                            "SSH extra config authType is invalid".to_string(),
                        ))
                    }
                }
            }
        }
    }

    let resolved_auth_type = match &auth {
        HostAuthMethod::Password(_) => "password",
        HostAuthMethod::PrivateKey { .. } => {
            if selected_key_id.is_some() {
                "ssh_key"
            } else {
                "private_key"
            }
        }
    };

    tracing::info!(
        target: "security::ssh_auth",
        host_id = row.0,
        hostname = %row.1,
        port = row.2,
        username = %username,
        auth_source = %auth_source,
        auth_type = resolved_auth_type,
        selected_key_id = ?selected_key_id,
        selected_identity_id = ?selected_identity_id,
        "resolved ssh authentication material"
    );

    Ok(HostSshConfig {
        host_id: row.0,
        hostname: row.1,
        port: row.2 as u16,
        username,
        auth,
    })
}

pub async fn verify_login(config: HostSshConfig, timeout_secs: u64) -> Result<(), AppError> {
    tokio::task::spawn_blocking(move || {
        let session = connect_session(&config, timeout_secs)?;
        let _ = session.disconnect(None, "verified", None);
        Ok(())
    })
    .await
    .map_err(|e| AppError::Internal(format!("ssh login task join error: {e}")))?
}

pub async fn upload_files(
    config: HostSshConfig,
    timeout_secs: u64,
    files: Vec<(String, Vec<u8>)>,
) -> Result<(), AppError> {
    tokio::task::spawn_blocking(move || {
        let session = connect_session(&config, timeout_secs)?;
        let sftp = session
            .sftp()
            .map_err(|e| AppError::Sftp(format!("failed to initialize sftp channel: {e}")))?;

        for (remote_path, content) in files {
            let path = Path::new(&remote_path);
            if let Some(parent) = path.parent() {
                ensure_remote_dir(&sftp, parent).map_err(|e| {
                    AppError::Sftp(format!("failed to ensure remote directory: {e}"))
                })?;
            }
            let mut remote_file = sftp.create(path).map_err(|e| {
                AppError::Sftp(format!("failed to create remote file {remote_path}: {e}"))
            })?;
            remote_file.write_all(&content).map_err(|e| {
                AppError::Sftp(format!("failed to write remote file {remote_path}: {e}"))
            })?;
        }

        let _ = session.disconnect(None, "upload-complete", None);
        Ok(())
    })
    .await
    .map_err(|e| AppError::Internal(format!("sftp upload task join error: {e}")))?
}

pub fn connect_session(
    config: &HostSshConfig,
    timeout_secs: u64,
) -> Result<ssh2::Session, AppError> {
    let stream = TcpStream::connect((config.hostname.as_str(), config.port)).map_err(|e| {
        AppError::Ssh(format!(
            "failed to connect to {}:{}: {e}",
            config.hostname, config.port
        ))
    })?;
    let timeout = Duration::from_secs(timeout_secs.max(1));
    let _ = stream.set_read_timeout(Some(timeout));
    let _ = stream.set_write_timeout(Some(timeout));

    let mut session = ssh2::Session::new()
        .map_err(|e| AppError::Ssh(format!("failed to create ssh session: {e}")))?;
    session.set_tcp_stream(stream);
    session
        .handshake()
        .map_err(|e| AppError::Ssh(format!("ssh handshake failed: {e}")))?;

    match &config.auth {
        HostAuthMethod::Password(password) => session
            .userauth_password(&config.username, password)
            .map_err(|e| AppError::Ssh(format!("ssh password authentication failed: {e}")))?,
        HostAuthMethod::PrivateKey {
            private_key,
            passphrase,
        } => {
            if let Err(e) = session.userauth_pubkey_memory(
                &config.username,
                None,
                private_key,
                passphrase.as_deref(),
            ) {
                let key_fingerprint = private_key_fingerprint(private_key);
                tracing::warn!(
                    target: "security::ssh_auth",
                    username = %config.username,
                    hostname = %config.hostname,
                    port = config.port,
                    key_fingerprint = %key_fingerprint,
                    "ssh private key authentication rejected by server"
                );
                return Err(AppError::Ssh(format!(
                    "ssh private key authentication failed: {e}"
                )));
            }
        }
    }

    if !session.authenticated() {
        return Err(AppError::Ssh(
            "ssh authentication failed with invalid credentials".to_string(),
        ));
    }

    Ok(session)
}

fn private_key_fingerprint(private_key: &str) -> String {
    if let Ok(parsed) = ssh_key::PrivateKey::from_openssh(private_key) {
        let fp = parsed.public_key().fingerprint(HashAlg::Sha256);
        return fp.to_string();
    }

    let mut hasher = Sha256::new();
    hasher.update(private_key.as_bytes());
    let digest = hasher.finalize();
    let mut out = String::with_capacity(16);
    for byte in digest.iter().take(8) {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

fn ensure_remote_dir(sftp: &ssh2::Sftp, path: &Path) -> Result<(), ssh2::Error> {
    let mut current = String::new();
    for part in path.components() {
        let segment = part.as_os_str().to_string_lossy();
        if segment == "/" {
            current.push('/');
            continue;
        }
        if segment.is_empty() || segment == "." {
            continue;
        }

        if current.is_empty() || !current.ends_with('/') {
            current.push('/');
        }
        current.push_str(&segment);

        let current_path = Path::new(&current);
        if sftp.stat(current_path).is_err() {
            if let Err(err) = sftp.mkdir(current_path, 0o755) {
                if sftp.stat(current_path).is_err() {
                    return Err(err);
                }
            }
        }
    }
    Ok(())
}

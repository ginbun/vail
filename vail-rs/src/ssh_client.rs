use std::{io::Write, net::TcpStream, path::Path, time::Duration};

use serde::Deserialize;

use crate::{error::AppError, security};

#[derive(Clone)]
pub enum HostAuthMethod {
    Password(String),
    PrivateKey {
        private_key: String,
        passphrase: Option<String>,
    },
}

#[derive(Clone)]
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

pub async fn resolve_host_ssh_config(
    db: &sqlx::PgPool,
    encryption_key: &str,
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
            k.passphrase_ciphertext
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

    let username = row
        .3
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .ok_or_else(|| AppError::BadRequest("Host username is required".to_string()))?
        .to_string();

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

fn connect_session(config: &HostSshConfig, timeout_secs: u64) -> Result<ssh2::Session, AppError> {
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
        } => session
            .userauth_pubkey_memory(&config.username, None, private_key, passphrase.as_deref())
            .map_err(|e| AppError::Ssh(format!("ssh private key authentication failed: {e}")))?,
    }

    if !session.authenticated() {
        return Err(AppError::Ssh(
            "ssh authentication failed with invalid credentials".to_string(),
        ));
    }

    Ok(session)
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

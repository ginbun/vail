use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use once_cell::sync::Lazy;
use sha2::{Digest, Sha256};
use sqlx::PgPool;

use crate::{
    error::{AppError, AppResult},
    infrastructure::terminal::ticket_repository,
};

const TOKEN_EXPIRE_MS: i64 = 5 * 60 * 1000;
const TERMINAL_TICKET_CLEANUP_INTERVAL_MS: i64 = 60 * 1000;
const TERMINAL_TICKET_CLEANUP_BATCH_SIZE: i64 = 500;

static TERMINAL_TICKET_CLEANUP_LAST_RUN_MS: Lazy<std::sync::Mutex<i64>> =
    Lazy::new(|| std::sync::Mutex::new(0));

#[derive(Debug, Clone)]
pub struct TerminalAccessTicketIssue {
    pub access_id: String,
    pub ws_ticket: String,
    pub ws_url: String,
    pub expires_at_ms: i64,
    pub session_hint: String,
}

#[derive(Debug, Clone)]
pub struct ConsumedTerminalAccessTicket {
    pub user_id: i64,
    pub host_id: i64,
    pub connect_type: String,
}

pub async fn issue_terminal_access_v2_ticket(
    db: &PgPool,
    user_id: i64,
    host_id: i64,
    connect_type: &str,
    signing_key: &str,
) -> AppResult<TerminalAccessTicketIssue> {
    maybe_cleanup_expired_terminal_access_tickets(db).await;

    let connect_type = connect_type.to_ascii_lowercase();
    if !matches!(connect_type.as_str(), "ssh" | "sftp") {
        return Err(AppError::BadRequest(
            "invalid connect type for terminal access".to_string(),
        ));
    }

    let access_id = uuid::Uuid::new_v4().to_string();
    let session_hint = uuid::Uuid::new_v4().to_string();
    let issued_at_ms = now_ms();
    let expires_at_ms = issued_at_ms + TOKEN_EXPIRE_MS;
    let payload = format!(
        "termv2:{}:{}:{}:{}:{}:{}",
        access_id, user_id, host_id, connect_type, issued_at_ms, session_hint
    );
    let signature = token_signature(&payload, signing_key);
    let ws_ticket = format!("{payload}:{signature}");
    let ticket_hash = ticket_digest(&ws_ticket);

    ticket_repository::insert_access_ticket(
        db,
        ticket_repository::InsertTerminalAccessTicket {
            access_id: &access_id,
            user_id,
            host_id,
            connect_type: &connect_type,
            session_hint: &session_hint,
            ticket_hash: &ticket_hash,
            expires_at_ms,
        },
    )
    .await?;

    Ok(TerminalAccessTicketIssue {
        access_id,
        ws_ticket,
        ws_url: format!("/terminal/access/{connect_type}"),
        expires_at_ms,
        session_hint,
    })
}

pub async fn consume_terminal_access_v2_ticket(
    db: &PgPool,
    token: &str,
    session_hint: &str,
    signing_key: &str,
) -> AppResult<ConsumedTerminalAccessTicket> {
    let (access_id, user_id, host_id, connect_type, token_session_hint) =
        parse_v2_access_ticket(token, session_hint, signing_key)?;
    let ticket_hash = ticket_digest(token);

    let stored = ticket_repository::consume_access_ticket(
        db,
        &access_id,
        &ticket_hash,
        session_hint,
        now_ms(),
    )
    .await?;

    let Some(stored) = stored else {
        return Err(AppError::BadRequest(
            "terminal access ticket already used or expired".to_string(),
        ));
    };

    let _ = stored.access_id;
    if stored.user_id != user_id
        || stored.host_id != host_id
        || stored.connect_type != connect_type
        || stored.session_hint != token_session_hint
    {
        return Err(AppError::BadRequest(
            "terminal access ticket context mismatch".to_string(),
        ));
    }
    if stored.expires_at_ms < now_ms() {
        return Err(AppError::BadRequest(
            "terminal access ticket expired".to_string(),
        ));
    }

    Ok(ConsumedTerminalAccessTicket {
        user_id: stored.user_id,
        host_id: stored.host_id,
        connect_type: stored.connect_type,
    })
}

async fn maybe_cleanup_expired_terminal_access_tickets(db: &PgPool) {
    let now = now_ms();
    let should_run = {
        let mut last_run = TERMINAL_TICKET_CLEANUP_LAST_RUN_MS
            .lock()
            .expect("terminal ticket cleanup mutex poisoned");
        if now - *last_run >= TERMINAL_TICKET_CLEANUP_INTERVAL_MS {
            *last_run = now;
            true
        } else {
            false
        }
    };

    if !should_run {
        return;
    }

    let _ = ticket_repository::cleanup_expired_access_tickets(db, TERMINAL_TICKET_CLEANUP_BATCH_SIZE)
        .await;
}

fn parse_v2_access_ticket(
    token: &str,
    session_hint: &str,
    signing_key: &str,
) -> AppResult<(String, i64, i64, String, String)> {
    let parts = token.split(':').collect::<Vec<_>>();
    if parts.len() != 8 || parts[0] != "termv2" {
        return Err(AppError::BadRequest(
            "invalid terminal access ticket".to_string(),
        ));
    }

    let unsigned = parts[0..7].join(":");
    ensure_token_signature(&unsigned, parts[7], signing_key)?;

    let access_id = parts[1].to_string();
    let user_id = parts[2]
        .parse::<i64>()
        .map_err(|_| AppError::BadRequest("invalid user in terminal access ticket".to_string()))?;
    let host_id = parts[3]
        .parse::<i64>()
        .map_err(|_| AppError::BadRequest("invalid host in terminal access ticket".to_string()))?;
    let connect_type = parts[4].to_ascii_lowercase();
    if !matches!(connect_type.as_str(), "ssh" | "sftp") {
        return Err(AppError::BadRequest(
            "invalid connect type in terminal access ticket".to_string(),
        ));
    }
    let issued_at_ms = parts[5].parse::<i64>().map_err(|_| {
        AppError::BadRequest("invalid timestamp in terminal access ticket".to_string())
    })?;
    let now = now_ms();
    if issued_at_ms > now || now - issued_at_ms > TOKEN_EXPIRE_MS {
        return Err(AppError::BadRequest(
            "terminal access ticket expired".to_string(),
        ));
    }
    let token_session_hint = parts[6].to_string();
    if token_session_hint != session_hint {
        return Err(AppError::BadRequest(
            "terminal access session hint mismatch".to_string(),
        ));
    }

    Ok((access_id, user_id, host_id, connect_type, token_session_hint))
}

fn token_signature(payload: &str, signing_key: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(signing_key.as_bytes());
    hasher.update(b":");
    hasher.update(payload.as_bytes());
    URL_SAFE_NO_PAD.encode(hasher.finalize())
}

fn ensure_token_signature(payload: &str, signature: &str, signing_key: &str) -> AppResult<()> {
    let expected = token_signature(payload, signing_key);
    if signature != expected {
        return Err(AppError::BadRequest(
            "invalid terminal token signature".to_string(),
        ));
    }
    Ok(())
}

fn ticket_digest(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    URL_SAFE_NO_PAD.encode(hasher.finalize())
}

fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

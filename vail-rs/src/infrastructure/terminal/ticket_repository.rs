use sqlx::PgPool;

use crate::error::{AppError, AppResult};

pub struct InsertTerminalAccessTicket<'a> {
    pub access_id: &'a str,
    pub user_id: i64,
    pub host_id: i64,
    pub connect_type: &'a str,
    pub session_hint: &'a str,
    pub ticket_hash: &'a str,
    pub expires_at_ms: i64,
}

pub struct ConsumedTerminalAccessTicket {
    pub access_id: String,
    pub user_id: i64,
    pub host_id: i64,
    pub connect_type: String,
    pub session_hint: String,
    pub expires_at_ms: i64,
}

pub async fn insert_access_ticket(
    db: &PgPool,
    input: InsertTerminalAccessTicket<'_>,
) -> AppResult<()> {
    sqlx::query(
        r#"
        INSERT INTO terminal_access_ticket (
            access_id,
            user_id,
            host_id,
            connect_type,
            session_hint,
            ticket_hash,
            expires_at
        ) VALUES (
            $1::uuid,
            $2,
            $3,
            $4,
            $5::uuid,
            $6,
            to_timestamp($7::double precision / 1000.0)
        )
        "#,
    )
    .bind(input.access_id)
    .bind(input.user_id)
    .bind(input.host_id)
    .bind(input.connect_type)
    .bind(input.session_hint)
    .bind(input.ticket_hash)
    .bind(input.expires_at_ms as f64)
    .execute(db)
    .await
    .map_err(AppError::Database)?;
    Ok(())
}

pub async fn consume_access_ticket(
    db: &PgPool,
    access_id: &str,
    ticket_hash: &str,
    session_hint: &str,
    now_ms: i64,
) -> AppResult<Option<ConsumedTerminalAccessTicket>> {
    let stored = sqlx::query_as::<_, (String, i64, i64, String, String, i64)>(
        r#"
        UPDATE terminal_access_ticket
        SET used_at = now()
        WHERE access_id = $1::uuid
          AND ticket_hash = $2
          AND session_hint = $3::uuid
          AND used_at IS NULL
          AND expires_at > to_timestamp($4::double precision / 1000.0)
        RETURNING access_id::text,
                  user_id,
                  host_id,
                  connect_type,
                  session_hint::text,
                  FLOOR(EXTRACT(EPOCH FROM expires_at) * 1000)::bigint
        "#,
    )
    .bind(access_id)
    .bind(ticket_hash)
    .bind(session_hint)
    .bind(now_ms as f64)
    .fetch_optional(db)
    .await
    .map_err(AppError::Database)?;

    Ok(stored.map(
        |(stored_access_id, stored_user_id, stored_host_id, stored_connect_type, stored_session_hint, stored_expires_at_ms)| {
            ConsumedTerminalAccessTicket {
                access_id: stored_access_id,
                user_id: stored_user_id,
                host_id: stored_host_id,
                connect_type: stored_connect_type,
                session_hint: stored_session_hint,
                expires_at_ms: stored_expires_at_ms,
            }
        },
    ))
}

pub async fn cleanup_expired_access_tickets(db: &PgPool, batch_size: i64) -> AppResult<u64> {
    let result = sqlx::query(
        r#"
        DELETE FROM terminal_access_ticket
        WHERE ctid IN (
            SELECT ctid
            FROM terminal_access_ticket
            WHERE expires_at < now()
            LIMIT $1
        )
        "#,
    )
    .bind(batch_size)
    .execute(db)
    .await
    .map_err(AppError::Database)?;
    Ok(result.rows_affected())
}

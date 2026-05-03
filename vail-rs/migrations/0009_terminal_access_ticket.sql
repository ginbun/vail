CREATE UNLOGGED TABLE IF NOT EXISTS terminal_access_ticket (
    access_id uuid PRIMARY KEY,
    user_id bigint NOT NULL,
    host_id bigint NOT NULL,
    connect_type varchar(16) NOT NULL,
    session_hint uuid NOT NULL,
    ticket_hash varchar(96) NOT NULL,
    expires_at timestamptz NOT NULL,
    used_at timestamptz,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX IF NOT EXISTS ux_terminal_access_ticket_hash
    ON terminal_access_ticket (ticket_hash);

CREATE INDEX IF NOT EXISTS ix_terminal_access_ticket_expires_at
    ON terminal_access_ticket (expires_at);

CREATE INDEX IF NOT EXISTS ix_terminal_access_ticket_user_host
    ON terminal_access_ticket (user_id, host_id, created_at DESC);

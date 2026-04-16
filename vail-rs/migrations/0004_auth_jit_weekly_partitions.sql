-- V4__auth_jit_weekly_partitions.sql
-- Auth/MFA/JIT schema and PostgreSQL 18 weekly partitions

CREATE EXTENSION IF NOT EXISTS pgcrypto;

CREATE TABLE IF NOT EXISTS user_mfa_totp (
    user_id BIGINT PRIMARY KEY,
    secret_ciphertext TEXT NOT NULL,
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS auth_login_challenge (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id BIGINT NOT NULL,
    source_ip VARCHAR(64),
    expires_at TIMESTAMPTZ NOT NULL,
    used_at TIMESTAMPTZ,
    attempts SMALLINT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_auth_login_challenge_user_id
    ON auth_login_challenge (user_id);
CREATE INDEX IF NOT EXISTS idx_auth_login_challenge_expires_at
    ON auth_login_challenge (expires_at);

CREATE TABLE IF NOT EXISTS auth_refresh_token (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL,
    token_hash VARCHAR(128) NOT NULL,
    session_id UUID NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    revoked_at TIMESTAMPTZ,
    rotated_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX IF NOT EXISTS uq_auth_refresh_token_hash
    ON auth_refresh_token (token_hash);
CREATE INDEX IF NOT EXISTS idx_auth_refresh_token_user
    ON auth_refresh_token (user_id, revoked_at, expires_at);

CREATE TABLE IF NOT EXISTS sys_permission (
    id BIGSERIAL PRIMARY KEY,
    code VARCHAR(128) NOT NULL UNIQUE,
    name VARCHAR(64) NOT NULL,
    description VARCHAR(256),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS sys_role_permission (
    role_id BIGINT NOT NULL,
    permission_id BIGINT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (role_id, permission_id)
);

CREATE TABLE IF NOT EXISTS jit_request (
    id BIGSERIAL PRIMARY KEY,
    requester_id BIGINT NOT NULL,
    reason VARCHAR(512) NOT NULL,
    status VARCHAR(24) NOT NULL DEFAULT 'requested',
    requested_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ,
    approved_at TIMESTAMPTZ,
    approver_id BIGINT,
    revoked_at TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_jit_request_status_expires
    ON jit_request (status, expires_at);

DROP TABLE IF EXISTS login_log CASCADE;

CREATE TABLE login_log (
    id BIGSERIAL,
    user_id BIGINT,
    username VARCHAR(32),
    ip VARCHAR(64),
    location VARCHAR(128),
    user_agent VARCHAR(256),
    result SMALLINT,
    error_message TEXT,
    create_time TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (id, create_time)
) PARTITION BY RANGE (create_time);

CREATE TABLE login_log_default PARTITION OF login_log DEFAULT;

CREATE INDEX idx_login_log_time_user ON login_log (create_time, user_id);
CREATE INDEX idx_login_log_time_result ON login_log (create_time, result);

DROP TABLE IF EXISTS operator_log CASCADE;

CREATE TABLE operator_log (
    id BIGSERIAL,
    user_id BIGINT,
    username VARCHAR(32),
    module VARCHAR(32),
    operation VARCHAR(64),
    method VARCHAR(16),
    path VARCHAR(256),
    params JSONB,
    result SMALLINT,
    error_message TEXT,
    duration INT,
    trace_id VARCHAR(64),
    ip VARCHAR(64),
    user_agent VARCHAR(256),
    create_time TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (id, create_time)
) PARTITION BY RANGE (create_time);

CREATE TABLE operator_log_default PARTITION OF operator_log DEFAULT;

CREATE INDEX idx_operator_log_time_user ON operator_log (create_time, user_id);
CREATE INDEX idx_operator_log_time_module ON operator_log (create_time, module);

DROP TABLE IF EXISTS ssh_session CASCADE;

CREATE TABLE ssh_session (
    id BIGSERIAL,
    user_id BIGINT NOT NULL,
    host_id BIGINT NOT NULL,
    session_id VARCHAR(64) NOT NULL,
    status SMALLINT DEFAULT 0,
    source_ip VARCHAR(64),
    decision_id VARCHAR(64),
    start_time TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    end_time TIMESTAMPTZ,
    create_time TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (id, start_time),
    UNIQUE (session_id, start_time)
) PARTITION BY RANGE (start_time);

CREATE TABLE ssh_session_default PARTITION OF ssh_session DEFAULT;

CREATE INDEX idx_ssh_session_time_user ON ssh_session (start_time, user_id);
CREATE INDEX idx_ssh_session_time_host ON ssh_session (start_time, host_id);

CREATE OR REPLACE FUNCTION create_weekly_partition_for(base_table TEXT, base_ts TIMESTAMPTZ)
RETURNS VOID AS $$
DECLARE
    week_start TIMESTAMPTZ;
    week_end TIMESTAMPTZ;
    partition_name TEXT;
BEGIN
    week_start := date_trunc('week', base_ts);
    week_end := week_start + INTERVAL '1 week';
    partition_name := format('%s_y%sw%s',
        base_table,
        to_char(week_start, 'IYYY'),
        to_char(week_start, 'IW')
    );

    EXECUTE format(
        'CREATE TABLE IF NOT EXISTS %I PARTITION OF %I FOR VALUES FROM (%L) TO (%L)',
        partition_name,
        base_table,
        week_start,
        week_end
    );
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION ensure_weekly_partitions(base_table TEXT, weeks_ahead INT DEFAULT 12)
RETURNS VOID AS $$
DECLARE
    i INT;
BEGIN
    FOR i IN 0..weeks_ahead LOOP
        PERFORM create_weekly_partition_for(
            base_table,
            date_trunc('week', NOW()) + make_interval(weeks => i)
        );
    END LOOP;
END;
$$ LANGUAGE plpgsql;

SELECT ensure_weekly_partitions('login_log', 12);
SELECT ensure_weekly_partitions('operator_log', 12);
SELECT ensure_weekly_partitions('ssh_session', 12);

INSERT INTO sys_permission (code, name, description)
VALUES
    ('jit.request', 'JIT Request', 'Create JIT privilege elevation request'),
    ('jit.approve', 'JIT Approve', 'Approve or revoke JIT elevation request')
ON CONFLICT (code) DO NOTHING;

-- 0001_init_schema.sql
-- Baseline migration for empty databases only.

CREATE EXTENSION IF NOT EXISTS pgcrypto;

CREATE TABLE sys_user (
    id BIGSERIAL PRIMARY KEY,
    username VARCHAR(32) NOT NULL UNIQUE,
    password VARCHAR(128) NOT NULL,
    nickname VARCHAR(64),
    email VARCHAR(128),
    phone VARCHAR(32),
    avatar VARCHAR(256),
    status SMALLINT NOT NULL DEFAULT 1,
    last_login_time TIMESTAMPTZ,
    last_login_ip VARCHAR(64),
    create_time TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    update_time TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted SMALLINT NOT NULL DEFAULT 0
);

CREATE TABLE sys_role (
    id BIGSERIAL PRIMARY KEY,
    name VARCHAR(32) NOT NULL,
    code VARCHAR(64) NOT NULL UNIQUE,
    description VARCHAR(256),
    status SMALLINT NOT NULL DEFAULT 1,
    create_time TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted SMALLINT NOT NULL DEFAULT 0
);

CREATE TABLE sys_user_role (
    user_id BIGINT NOT NULL,
    role_id BIGINT NOT NULL,
    create_time TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_id, role_id),
    CONSTRAINT fk_sys_user_role_user_id FOREIGN KEY (user_id) REFERENCES sys_user(id) ON DELETE CASCADE,
    CONSTRAINT fk_sys_user_role_role_id FOREIGN KEY (role_id) REFERENCES sys_role(id) ON DELETE CASCADE
);

CREATE INDEX idx_sys_user_role_role_id ON sys_user_role (role_id);

CREATE TABLE sys_menu (
    id BIGSERIAL PRIMARY KEY,
    parent_id BIGINT NOT NULL DEFAULT 0,
    name VARCHAR(64) NOT NULL,
    path VARCHAR(128),
    component VARCHAR(128),
    icon VARCHAR(64),
    type SMALLINT NOT NULL DEFAULT 1,
    sort INT NOT NULL DEFAULT 0,
    visible SMALLINT NOT NULL DEFAULT 1,
    permission VARCHAR(128),
    create_time TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE sys_role_menu (
    role_id BIGINT NOT NULL,
    menu_id BIGINT NOT NULL,
    PRIMARY KEY (role_id, menu_id),
    CONSTRAINT fk_sys_role_menu_role FOREIGN KEY (role_id) REFERENCES sys_role(id) ON DELETE CASCADE,
    CONSTRAINT fk_sys_role_menu_menu FOREIGN KEY (menu_id) REFERENCES sys_menu(id) ON DELETE CASCADE
);

CREATE TABLE host (
    id BIGSERIAL PRIMARY KEY,
    name VARCHAR(64) NOT NULL,
    hostname VARCHAR(128) NOT NULL,
    port INT NOT NULL DEFAULT 22,
    username VARCHAR(64),
    credential_type VARCHAR(16),
    credential_data TEXT,
    description VARCHAR(512),
    tags JSONB,
    status SMALLINT NOT NULL DEFAULT 1,
    create_time TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    update_time TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted SMALLINT NOT NULL DEFAULT 0,
    CONSTRAINT chk_host_port_range CHECK (port >= 1 AND port <= 65535)
);

CREATE INDEX idx_host_name ON host(name);

CREATE TABLE host_group (
    id BIGSERIAL PRIMARY KEY,
    name VARCHAR(64) NOT NULL,
    parent_id BIGINT NOT NULL DEFAULT 0,
    description VARCHAR(256),
    sort INT NOT NULL DEFAULT 0,
    create_time TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted SMALLINT NOT NULL DEFAULT 0
);

CREATE TABLE host_group_rel (
    host_id BIGINT NOT NULL,
    group_id BIGINT NOT NULL,
    PRIMARY KEY (host_id, group_id),
    CONSTRAINT fk_host_group_rel_host FOREIGN KEY (host_id) REFERENCES host(id) ON DELETE CASCADE,
    CONSTRAINT fk_host_group_rel_group FOREIGN KEY (group_id) REFERENCES host_group(id) ON DELETE CASCADE
);

CREATE TABLE user_host_access (
    user_id BIGINT NOT NULL,
    host_id BIGINT NOT NULL,
    create_time TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_id, host_id),
    CONSTRAINT fk_user_host_access_user_id FOREIGN KEY (user_id) REFERENCES sys_user(id) ON DELETE CASCADE,
    CONSTRAINT fk_user_host_access_host_id FOREIGN KEY (host_id) REFERENCES host(id) ON DELETE CASCADE
);

CREATE INDEX idx_user_host_access_user_id ON user_host_access (user_id);
CREATE INDEX idx_user_host_access_host_id ON user_host_access (host_id);

CREATE TABLE ssh_key (
    id BIGSERIAL PRIMARY KEY,
    name VARCHAR(128) NOT NULL,
    private_key_ciphertext TEXT NOT NULL,
    passphrase_ciphertext TEXT,
    description VARCHAR(512),
    status SMALLINT NOT NULL DEFAULT 1,
    create_time TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    update_time TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted SMALLINT NOT NULL DEFAULT 0
);

CREATE UNIQUE INDEX uq_ssh_key_name_not_deleted ON ssh_key (name) WHERE deleted = 0;

CREATE TABLE host_ssh_key_binding (
    host_id BIGINT NOT NULL,
    ssh_key_id BIGINT NOT NULL,
    is_default SMALLINT NOT NULL DEFAULT 1,
    create_time TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (host_id, ssh_key_id),
    CONSTRAINT fk_host_ssh_key_binding_host FOREIGN KEY (host_id) REFERENCES host(id) ON DELETE CASCADE,
    CONSTRAINT fk_host_ssh_key_binding_ssh_key FOREIGN KEY (ssh_key_id) REFERENCES ssh_key(id) ON DELETE CASCADE
);

CREATE UNIQUE INDEX uq_host_default_ssh_key ON host_ssh_key_binding (host_id) WHERE is_default = 1;
CREATE INDEX idx_host_ssh_key_binding_ssh_key ON host_ssh_key_binding (ssh_key_id);

CREATE TABLE upload_task (
    id BIGSERIAL PRIMARY KEY,
    task_no VARCHAR(64) NOT NULL UNIQUE,
    user_id BIGINT NOT NULL,
    host_id BIGINT NOT NULL,
    remote_path VARCHAR(512) NOT NULL,
    file_name VARCHAR(256),
    file_size BIGINT,
    file_md5 VARCHAR(32),
    chunk_size BIGINT NOT NULL DEFAULT 1048576,
    uploaded_size BIGINT NOT NULL DEFAULT 0,
    status SMALLINT NOT NULL DEFAULT 0,
    error_message TEXT,
    create_time TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    update_time TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_upload_task_task_no ON upload_task(task_no);
CREATE INDEX idx_upload_task_status ON upload_task(status);

CREATE TABLE download_task (
    id BIGSERIAL PRIMARY KEY,
    task_no VARCHAR(64) NOT NULL UNIQUE,
    user_id BIGINT NOT NULL,
    host_id BIGINT NOT NULL,
    remote_path VARCHAR(512) NOT NULL,
    local_path VARCHAR(512),
    file_name VARCHAR(256),
    file_size BIGINT,
    downloaded_size BIGINT NOT NULL DEFAULT 0,
    status SMALLINT NOT NULL DEFAULT 0,
    error_message TEXT,
    create_time TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    update_time TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNLOGGED TABLE cache (
    cache_key VARCHAR(128) PRIMARY KEY,
    cache_value TEXT NOT NULL,
    expire_time TIMESTAMPTZ,
    create_time TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_cache_expire ON cache(expire_time) WHERE expire_time IS NOT NULL;

CREATE TABLE user_mfa_totp (
    user_id BIGINT PRIMARY KEY,
    secret_ciphertext TEXT NOT NULL,
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT fk_user_mfa_totp_user_id FOREIGN KEY (user_id) REFERENCES sys_user(id) ON DELETE CASCADE
);

CREATE TABLE auth_login_challenge (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id BIGINT NOT NULL,
    source_ip VARCHAR(64),
    expires_at TIMESTAMPTZ NOT NULL,
    used_at TIMESTAMPTZ,
    attempts SMALLINT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT fk_auth_login_challenge_user_id FOREIGN KEY (user_id) REFERENCES sys_user(id) ON DELETE CASCADE
);

CREATE INDEX idx_auth_login_challenge_user_id ON auth_login_challenge (user_id);
CREATE INDEX idx_auth_login_challenge_expires_at ON auth_login_challenge (expires_at);

CREATE TABLE auth_refresh_token (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL,
    token_hash VARCHAR(128) NOT NULL,
    session_id UUID NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    revoked_at TIMESTAMPTZ,
    rotated_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT fk_auth_refresh_token_user_id FOREIGN KEY (user_id) REFERENCES sys_user(id) ON DELETE CASCADE
);

CREATE UNIQUE INDEX uq_auth_refresh_token_hash ON auth_refresh_token (token_hash);
CREATE INDEX idx_auth_refresh_token_user ON auth_refresh_token (user_id, revoked_at, expires_at);
CREATE INDEX idx_auth_refresh_token_session_id ON auth_refresh_token (session_id);

CREATE TABLE sys_permission (
    id BIGSERIAL PRIMARY KEY,
    code VARCHAR(128) NOT NULL UNIQUE,
    name VARCHAR(64) NOT NULL,
    description VARCHAR(256),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE sys_role_permission (
    role_id BIGINT NOT NULL,
    permission_id BIGINT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (role_id, permission_id),
    CONSTRAINT fk_sys_role_permission_role_id FOREIGN KEY (role_id) REFERENCES sys_role(id) ON DELETE CASCADE,
    CONSTRAINT fk_sys_role_permission_permission_id FOREIGN KEY (permission_id) REFERENCES sys_permission(id) ON DELETE CASCADE
);

CREATE INDEX idx_sys_role_permission_permission_id ON sys_role_permission (permission_id);

CREATE TABLE jit_request (
    id BIGSERIAL PRIMARY KEY,
    requester_id BIGINT NOT NULL,
    reason VARCHAR(512) NOT NULL,
    status VARCHAR(24) NOT NULL DEFAULT 'requested',
    requested_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ,
    approved_at TIMESTAMPTZ,
    approver_id BIGINT,
    revoked_at TIMESTAMPTZ,
    CONSTRAINT fk_jit_request_requester FOREIGN KEY (requester_id) REFERENCES sys_user(id) ON DELETE CASCADE
);

CREATE INDEX idx_jit_request_status_expires ON jit_request (status, expires_at);

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

CREATE TABLE ssh_session (
    id BIGSERIAL,
    user_id BIGINT NOT NULL,
    host_id BIGINT NOT NULL,
    session_id VARCHAR(64) NOT NULL,
    status SMALLINT NOT NULL DEFAULT 0,
    source_ip VARCHAR(64),
    decision_id VARCHAR(64),
    start_time TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    end_time TIMESTAMPTZ,
    create_time TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (id, start_time),
    UNIQUE (session_id, start_time),
    CONSTRAINT fk_ssh_session_user FOREIGN KEY (user_id) REFERENCES sys_user(id),
    CONSTRAINT fk_ssh_session_host FOREIGN KEY (host_id) REFERENCES host(id)
) PARTITION BY RANGE (start_time);

CREATE TABLE ssh_session_default PARTITION OF ssh_session DEFAULT;
CREATE INDEX idx_ssh_session_session_id ON ssh_session(session_id);
CREATE INDEX idx_ssh_session_time_user ON ssh_session (start_time, user_id);
CREATE INDEX idx_ssh_session_time_host ON ssh_session (start_time, host_id);

CREATE TABLE sys_dict_key (
    id BIGSERIAL PRIMARY KEY,
    key_name VARCHAR(128) NOT NULL UNIQUE,
    value_type VARCHAR(32) NOT NULL DEFAULT 'STRING',
    extra_schema TEXT,
    description VARCHAR(512),
    creator VARCHAR(64) NOT NULL DEFAULT 'system',
    updater VARCHAR(64) NOT NULL DEFAULT 'system',
    create_time TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    update_time TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE sys_dict_value (
    id BIGSERIAL PRIMARY KEY,
    key_id BIGINT NOT NULL,
    name VARCHAR(128) NOT NULL,
    value TEXT NOT NULL,
    label VARCHAR(256) NOT NULL,
    extra TEXT,
    sort INT NOT NULL DEFAULT 0,
    creator VARCHAR(64) NOT NULL DEFAULT 'system',
    updater VARCHAR(64) NOT NULL DEFAULT 'system',
    create_time TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    update_time TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted SMALLINT NOT NULL DEFAULT 0,
    CONSTRAINT fk_sys_dict_value_key_id FOREIGN KEY (key_id) REFERENCES sys_dict_key(id) ON DELETE CASCADE,
    CONSTRAINT uq_sys_dict_value_key_value UNIQUE (key_id, value)
);

CREATE INDEX idx_sys_dict_value_key_sort ON sys_dict_value (key_id, sort, id);

CREATE TABLE sys_dict_value_history (
    id BIGSERIAL PRIMARY KEY,
    rel_id BIGINT NOT NULL,
    before_value TEXT,
    after_value TEXT,
    create_time TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT fk_sys_dict_value_history_rel_id FOREIGN KEY (rel_id) REFERENCES sys_dict_value(id) ON DELETE CASCADE
);

CREATE INDEX idx_sys_dict_value_history_rel_time ON sys_dict_value_history (rel_id, create_time DESC);

CREATE TABLE sys_system_message (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT,
    classify VARCHAR(64) NOT NULL,
    type VARCHAR(64) NOT NULL,
    status SMALLINT NOT NULL DEFAULT 0,
    rel_key VARCHAR(128),
    title VARCHAR(256) NOT NULL,
    content TEXT NOT NULL,
    content_html TEXT,
    read_time TIMESTAMPTZ,
    create_time TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT fk_sys_system_message_user_id FOREIGN KEY (user_id) REFERENCES sys_user(id) ON DELETE SET NULL
);

CREATE INDEX idx_sys_system_message_user_status ON sys_system_message (user_id, status, create_time DESC);
CREATE INDEX idx_sys_system_message_classify ON sys_system_message (classify, create_time DESC);

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

INSERT INTO sys_role (name, code, description, status, create_time, deleted)
VALUES ('Administrator', 'admin', 'System administrator role', 1, NOW(), 0);

INSERT INTO sys_user (
    username,
    password,
    nickname,
    email,
    status,
    create_time,
    update_time,
    deleted
)
VALUES (
    'admin',
    crypt('Admin@123456', gen_salt('bf')),
    'Administrator',
    'admin@local',
    1,
    NOW(),
    NOW(),
    0
);

INSERT INTO sys_permission (code, name, description)
VALUES
    ('jit.request', 'JIT Request', 'Create JIT privilege elevation request'),
    ('jit.approve', 'JIT Approve', 'Approve or revoke JIT elevation request'),
    ('iam.user-role.assign', 'Assign User Roles', 'Assign role set to target user'),
    ('iam.user-resource.assign', 'Assign User Resources', 'Assign host resources to target user'),
    ('iam.user-permission.view', 'View User Permissions', 'View effective role and permission set'),
    ('host.read', 'Read Hosts', 'View host and host-group resources'),
    ('host.create', 'Create Host', 'Create host and host-group resources'),
    ('host.update', 'Update Host', 'Update host and host-group resources'),
    ('host.delete', 'Delete Host', 'Delete host and host-group resources'),
    ('sshkey.read', 'Read SSH Keys', 'View SSH key resources and host-key bindings'),
    ('sshkey.create', 'Create SSH Keys', 'Create SSH key resources and host-key bindings'),
    ('sshkey.update', 'Update SSH Keys', 'Update SSH key resources and host-key bindings'),
    ('sshkey.delete', 'Delete SSH Keys', 'Delete SSH key resources and host-key bindings'),
    ('infra:dict-key:query', 'Query Dict Key', 'Query dictionary keys'),
    ('infra:dict-key:create', 'Create Dict Key', 'Create dictionary keys'),
    ('infra:dict-key:update', 'Update Dict Key', 'Update dictionary keys'),
    ('infra:dict-key:delete', 'Delete Dict Key', 'Delete dictionary keys'),
    ('infra:dict-key:management:refresh-cache', 'Refresh Dict Cache', 'Refresh dictionary cache'),
    ('infra:dict-value:query', 'Query Dict Value', 'Query dictionary values'),
    ('infra:dict-value:create', 'Create Dict Value', 'Create dictionary values'),
    ('infra:dict-value:update', 'Update Dict Value', 'Update dictionary values'),
    ('infra:dict-value:delete', 'Delete Dict Value', 'Delete dictionary values');

INSERT INTO sys_role_permission (role_id, permission_id, created_at)
SELECT r.id, p.id, NOW()
FROM sys_role r
JOIN sys_permission p ON TRUE
WHERE r.code = 'admin';

INSERT INTO sys_user_role (user_id, role_id, create_time)
SELECT u.id, r.id, NOW()
FROM sys_user u
JOIN sys_role r ON r.code = 'admin'
WHERE u.username = 'admin' AND u.deleted = 0;

INSERT INTO sys_dict_key (key_name, value_type, extra_schema, description, creator, updater, create_time, update_time)
VALUES
    ('systemMenuType', 'NUMBER', '', 'System menu type', 'system', 'system', NOW(), NOW()),
    ('systemMenuStatus', 'NUMBER', '', 'System menu status', 'system', 'system', NOW(), NOW()),
    ('systemMenuVisible', 'NUMBER', '', 'System menu visible', 'system', 'system', NOW(), NOW()),
    ('systemMenuCache', 'NUMBER', '', 'System menu cache switch', 'system', 'system', NOW(), NOW()),
    ('systemMenuNewWindow', 'NUMBER', '', 'System menu new window switch', 'system', 'system', NOW(), NOW()),
    ('messageClassify', 'STRING', '', 'Message classify', 'system', 'system', NOW(), NOW()),
    ('messageType', 'STRING', '', 'Message type', 'system', 'system', NOW(), NOW()),
    ('dictValueType', 'STRING', '', 'Dictionary value data type', 'system', 'system', NOW(), NOW());

INSERT INTO sys_dict_value (key_id, name, value, label, extra, sort, creator, updater, create_time, update_time, deleted)
SELECT dk.id, src.name, src.value, src.label, src.extra, src.sort, 'system', 'system', NOW(), NOW(), 0
FROM (
    VALUES
        ('systemMenuType', 'Directory', '1', 'Directory', '{}', 1),
        ('systemMenuType', 'Menu', '2', 'Menu', '{}', 2),
        ('systemMenuType', 'Button', '3', 'Button', '{}', 3),
        ('systemMenuStatus', 'Disabled', '0', 'Disabled', '{"color":"orangered"}', 1),
        ('systemMenuStatus', 'Enabled', '1', 'Enabled', '{"color":"green"}', 2),
        ('systemMenuVisible', 'Hidden', '0', 'Hidden', '{"color":"orangered"}', 1),
        ('systemMenuVisible', 'Visible', '1', 'Visible', '{"color":"green"}', 2),
        ('systemMenuCache', 'Off', '0', 'Off', '{}', 1),
        ('systemMenuCache', 'On', '1', 'On', '{}', 2),
        ('systemMenuNewWindow', 'Off', '0', 'Off', '{}', 1),
        ('systemMenuNewWindow', 'On', '1', 'On', '{}', 2),
        ('messageClassify', 'Notice', 'NOTICE', 'Notice', '{}', 1),
        ('messageType', 'General', 'GENERAL', 'General', '{"redirectComponent":"0"}', 1),
        ('dictValueType', 'String', 'STRING', 'String', '{"color":"arcoblue"}', 1),
        ('dictValueType', 'Number', 'NUMBER', 'Number', '{"color":"green"}', 2),
        ('dictValueType', 'Boolean', 'BOOLEAN', 'Boolean', '{"color":"orange"}', 3),
        ('dictValueType', 'JSON', 'JSON', 'JSON', '{"color":"purple"}', 4)
) AS src(key_name, name, value, label, extra, sort)
JOIN sys_dict_key dk ON dk.key_name = src.key_name;

INSERT INTO sys_dict_value_history (rel_id, before_value, after_value, create_time)
SELECT id, '', value, NOW()
FROM sys_dict_value;

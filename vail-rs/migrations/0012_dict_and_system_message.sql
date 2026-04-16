-- V12__dict_and_system_message.sql
-- Dictionary and system message storage for Orion-compatible APIs

CREATE TABLE IF NOT EXISTS sys_dict_key (
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

CREATE TABLE IF NOT EXISTS sys_dict_value (
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
    CONSTRAINT fk_sys_dict_value_key_id
        FOREIGN KEY (key_id) REFERENCES sys_dict_key(id) ON DELETE CASCADE,
    CONSTRAINT uq_sys_dict_value_key_value UNIQUE (key_id, value)
);

CREATE INDEX IF NOT EXISTS idx_sys_dict_value_key_sort
    ON sys_dict_value (key_id, sort, id);

CREATE TABLE IF NOT EXISTS sys_dict_value_history (
    id BIGSERIAL PRIMARY KEY,
    rel_id BIGINT NOT NULL,
    before_value TEXT,
    after_value TEXT,
    create_time TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT fk_sys_dict_value_history_rel_id
        FOREIGN KEY (rel_id) REFERENCES sys_dict_value(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_sys_dict_value_history_rel_time
    ON sys_dict_value_history (rel_id, create_time DESC);

CREATE TABLE IF NOT EXISTS sys_system_message (
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
    create_time TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_sys_system_message_user_status
    ON sys_system_message (user_id, status, create_time DESC);
CREATE INDEX IF NOT EXISTS idx_sys_system_message_classify
    ON sys_system_message (classify, create_time DESC);

INSERT INTO sys_permission (code, name, description)
VALUES
    ('infra:dict-key:query', 'Query Dict Key', 'Query dictionary keys'),
    ('infra:dict-key:create', 'Create Dict Key', 'Create dictionary keys'),
    ('infra:dict-key:update', 'Update Dict Key', 'Update dictionary keys'),
    ('infra:dict-key:delete', 'Delete Dict Key', 'Delete dictionary keys'),
    ('infra:dict-key:management:refresh-cache', 'Refresh Dict Cache', 'Refresh dictionary cache'),
    ('infra:dict-value:query', 'Query Dict Value', 'Query dictionary values'),
    ('infra:dict-value:create', 'Create Dict Value', 'Create dictionary values'),
    ('infra:dict-value:update', 'Update Dict Value', 'Update dictionary values'),
    ('infra:dict-value:delete', 'Delete Dict Value', 'Delete dictionary values')
ON CONFLICT (code) DO NOTHING;

INSERT INTO sys_role_permission (role_id, permission_id, created_at)
SELECT
    r.id,
    p.id,
    NOW()
FROM sys_role r
JOIN sys_permission p ON p.code IN (
    'infra:dict-key:query',
    'infra:dict-key:create',
    'infra:dict-key:update',
    'infra:dict-key:delete',
    'infra:dict-key:management:refresh-cache',
    'infra:dict-value:query',
    'infra:dict-value:create',
    'infra:dict-value:update',
    'infra:dict-value:delete'
)
WHERE r.code = 'admin'
ON CONFLICT (role_id, permission_id) DO NOTHING;

INSERT INTO sys_dict_key (key_name, value_type, extra_schema, description, creator, updater, create_time, update_time)
VALUES
    ('systemMenuType', 'NUMBER', '', 'System menu type', 'system', 'system', NOW(), NOW()),
    ('systemMenuStatus', 'NUMBER', '', 'System menu status', 'system', 'system', NOW(), NOW()),
    ('systemMenuVisible', 'NUMBER', '', 'System menu visible', 'system', 'system', NOW(), NOW()),
    ('systemMenuCache', 'NUMBER', '', 'System menu cache switch', 'system', 'system', NOW(), NOW()),
    ('systemMenuNewWindow', 'NUMBER', '', 'System menu new window switch', 'system', 'system', NOW(), NOW()),
    ('messageClassify', 'STRING', '', 'Message classify', 'system', 'system', NOW(), NOW()),
    ('messageType', 'STRING', '', 'Message type', 'system', 'system', NOW(), NOW()),
    ('dictValueType', 'STRING', '', 'Dictionary value data type', 'system', 'system', NOW(), NOW())
ON CONFLICT (key_name) DO NOTHING;

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
JOIN sys_dict_key dk ON dk.key_name = src.key_name
ON CONFLICT (key_id, value) DO NOTHING;

INSERT INTO sys_dict_value_history (rel_id, before_value, after_value, create_time)
SELECT id, '', value, NOW()
FROM sys_dict_value
ON CONFLICT DO NOTHING;

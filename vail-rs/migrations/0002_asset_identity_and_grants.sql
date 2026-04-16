-- 0002_asset_identity_and_grants.sql

CREATE TABLE IF NOT EXISTS host_identity (
    id BIGSERIAL PRIMARY KEY,
    name VARCHAR(128) NOT NULL,
    type VARCHAR(16) NOT NULL,
    username VARCHAR(128),
    password_ciphertext TEXT,
    key_id BIGINT,
    description VARCHAR(512),
    status SMALLINT NOT NULL DEFAULT 1,
    create_time TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    update_time TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted SMALLINT NOT NULL DEFAULT 0,
    CONSTRAINT fk_host_identity_key_id FOREIGN KEY (key_id) REFERENCES ssh_key(id) ON DELETE SET NULL,
    CONSTRAINT chk_host_identity_type CHECK (type IN ('PASSWORD', 'KEY'))
);

CREATE UNIQUE INDEX IF NOT EXISTS uq_host_identity_name_not_deleted
ON host_identity (name)
WHERE deleted = 0;

CREATE INDEX IF NOT EXISTS idx_host_identity_type ON host_identity (type);

CREATE TABLE IF NOT EXISTS role_host_group_grant (
    role_id BIGINT NOT NULL,
    group_id BIGINT NOT NULL,
    create_time TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (role_id, group_id),
    CONSTRAINT fk_role_host_group_grant_role FOREIGN KEY (role_id) REFERENCES sys_role(id) ON DELETE CASCADE,
    CONSTRAINT fk_role_host_group_grant_group FOREIGN KEY (group_id) REFERENCES host_group(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS role_host_key_grant (
    role_id BIGINT NOT NULL,
    key_id BIGINT NOT NULL,
    create_time TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (role_id, key_id),
    CONSTRAINT fk_role_host_key_grant_role FOREIGN KEY (role_id) REFERENCES sys_role(id) ON DELETE CASCADE,
    CONSTRAINT fk_role_host_key_grant_key FOREIGN KEY (key_id) REFERENCES ssh_key(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS role_host_identity_grant (
    role_id BIGINT NOT NULL,
    identity_id BIGINT NOT NULL,
    create_time TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (role_id, identity_id),
    CONSTRAINT fk_role_host_identity_grant_role FOREIGN KEY (role_id) REFERENCES sys_role(id) ON DELETE CASCADE,
    CONSTRAINT fk_role_host_identity_grant_identity FOREIGN KEY (identity_id) REFERENCES host_identity(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS user_host_group_grant (
    user_id BIGINT NOT NULL,
    group_id BIGINT NOT NULL,
    create_time TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_id, group_id),
    CONSTRAINT fk_user_host_group_grant_user FOREIGN KEY (user_id) REFERENCES sys_user(id) ON DELETE CASCADE,
    CONSTRAINT fk_user_host_group_grant_group FOREIGN KEY (group_id) REFERENCES host_group(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS user_host_key_grant (
    user_id BIGINT NOT NULL,
    key_id BIGINT NOT NULL,
    create_time TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_id, key_id),
    CONSTRAINT fk_user_host_key_grant_user FOREIGN KEY (user_id) REFERENCES sys_user(id) ON DELETE CASCADE,
    CONSTRAINT fk_user_host_key_grant_key FOREIGN KEY (key_id) REFERENCES ssh_key(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS user_host_identity_grant (
    user_id BIGINT NOT NULL,
    identity_id BIGINT NOT NULL,
    create_time TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_id, identity_id),
    CONSTRAINT fk_user_host_identity_grant_user FOREIGN KEY (user_id) REFERENCES sys_user(id) ON DELETE CASCADE,
    CONSTRAINT fk_user_host_identity_grant_identity FOREIGN KEY (identity_id) REFERENCES host_identity(id) ON DELETE CASCADE
);

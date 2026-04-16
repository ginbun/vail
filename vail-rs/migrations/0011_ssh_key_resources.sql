-- 0011_ssh_key_resources.sql
-- Independent SSH key resources and host-key bindings.

CREATE TABLE IF NOT EXISTS ssh_key (
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

CREATE UNIQUE INDEX IF NOT EXISTS uq_ssh_key_name_not_deleted
    ON ssh_key (name)
    WHERE deleted = 0;

CREATE TABLE IF NOT EXISTS host_ssh_key_binding (
    host_id BIGINT NOT NULL,
    ssh_key_id BIGINT NOT NULL,
    is_default SMALLINT NOT NULL DEFAULT 1,
    create_time TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (host_id, ssh_key_id)
);

CREATE UNIQUE INDEX IF NOT EXISTS uq_host_default_ssh_key
    ON host_ssh_key_binding (host_id)
    WHERE is_default = 1;

CREATE INDEX IF NOT EXISTS idx_host_ssh_key_binding_ssh_key
    ON host_ssh_key_binding (ssh_key_id);

DO $$
BEGIN
    ALTER TABLE host_ssh_key_binding
        ADD CONSTRAINT fk_host_ssh_key_binding_host
            FOREIGN KEY (host_id) REFERENCES host(id) ON DELETE CASCADE,
        ADD CONSTRAINT fk_host_ssh_key_binding_ssh_key
            FOREIGN KEY (ssh_key_id) REFERENCES ssh_key(id) ON DELETE CASCADE;
EXCEPTION
    WHEN duplicate_object THEN NULL;
END $$;

INSERT INTO sys_permission (code, name, description)
VALUES
    ('sshkey.read', 'Read SSH Keys', 'View SSH key resources and host-key bindings'),
    ('sshkey.create', 'Create SSH Keys', 'Create SSH key resources and host-key bindings'),
    ('sshkey.update', 'Update SSH Keys', 'Update SSH key resources and host-key bindings'),
    ('sshkey.delete', 'Delete SSH Keys', 'Delete SSH key resources and host-key bindings')
ON CONFLICT (code) DO NOTHING;

INSERT INTO sys_role_permission (role_id, permission_id, created_at)
SELECT
    r.id,
    p.id,
    NOW()
FROM sys_role r
JOIN sys_permission p ON p.code IN (
    'sshkey.read',
    'sshkey.create',
    'sshkey.update',
    'sshkey.delete'
)
WHERE r.code = 'admin'
ON CONFLICT (role_id, permission_id) DO NOTHING;

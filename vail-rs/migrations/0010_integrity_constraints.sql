-- 0010_integrity_constraints.sql
-- Add integrity constraints and indexes for production safety.

CREATE INDEX IF NOT EXISTS idx_sys_user_role_role_id ON sys_user_role (role_id);
CREATE INDEX IF NOT EXISTS idx_sys_role_permission_permission_id ON sys_role_permission (permission_id);
CREATE INDEX IF NOT EXISTS idx_user_host_access_user_id ON user_host_access (user_id);
CREATE INDEX IF NOT EXISTS idx_auth_refresh_token_session_id ON auth_refresh_token (session_id);

DO $$
BEGIN
    ALTER TABLE host
        ADD CONSTRAINT chk_host_port_range CHECK (port >= 1 AND port <= 65535);
EXCEPTION
    WHEN duplicate_object THEN NULL;
END $$;

DO $$
BEGIN
    ALTER TABLE sys_user_role
        ADD CONSTRAINT fk_sys_user_role_user_id
            FOREIGN KEY (user_id) REFERENCES sys_user(id) ON DELETE CASCADE,
        ADD CONSTRAINT fk_sys_user_role_role_id
            FOREIGN KEY (role_id) REFERENCES sys_role(id) ON DELETE CASCADE;
EXCEPTION
    WHEN duplicate_object THEN NULL;
END $$;

DO $$
BEGIN
    ALTER TABLE sys_role_permission
        ADD CONSTRAINT fk_sys_role_permission_role_id
            FOREIGN KEY (role_id) REFERENCES sys_role(id) ON DELETE CASCADE,
        ADD CONSTRAINT fk_sys_role_permission_permission_id
            FOREIGN KEY (permission_id) REFERENCES sys_permission(id) ON DELETE CASCADE;
EXCEPTION
    WHEN duplicate_object THEN NULL;
END $$;

DO $$
BEGIN
    ALTER TABLE user_host_access
        ADD CONSTRAINT fk_user_host_access_user_id
            FOREIGN KEY (user_id) REFERENCES sys_user(id) ON DELETE CASCADE,
        ADD CONSTRAINT fk_user_host_access_host_id
            FOREIGN KEY (host_id) REFERENCES host(id) ON DELETE CASCADE;
EXCEPTION
    WHEN duplicate_object THEN NULL;
END $$;

DO $$
BEGIN
    ALTER TABLE auth_refresh_token
        ADD CONSTRAINT fk_auth_refresh_token_user_id
            FOREIGN KEY (user_id) REFERENCES sys_user(id) ON DELETE CASCADE;
EXCEPTION
    WHEN duplicate_object THEN NULL;
END $$;

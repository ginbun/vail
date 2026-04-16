-- V5__iam_resource_permission.sql
-- User resource assignment and IAM permission seeds

CREATE TABLE IF NOT EXISTS user_host_access (
    user_id BIGINT NOT NULL,
    host_id BIGINT NOT NULL,
    create_time TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_id, host_id)
);

CREATE INDEX IF NOT EXISTS idx_user_host_access_host
    ON user_host_access (host_id);

INSERT INTO sys_permission (code, name, description)
VALUES
    ('iam.user-role.assign', 'Assign User Roles', 'Assign role set to target user'),
    ('iam.user-resource.assign', 'Assign User Resources', 'Assign host resources to target user'),
    ('iam.user-permission.view', 'View User Permissions', 'View effective role and permission set')
ON CONFLICT (code) DO NOTHING;

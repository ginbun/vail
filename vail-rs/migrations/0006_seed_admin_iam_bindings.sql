-- V6__seed_admin_iam_bindings.sql
-- Ensure admin role has IAM permissions and bind to admin user when present

INSERT INTO sys_role (name, code, description, status, create_time, deleted)
VALUES ('Administrator', 'admin', 'System administrator role', 1, NOW(), 0)
ON CONFLICT (code) DO UPDATE
SET
    name = EXCLUDED.name,
    description = EXCLUDED.description,
    status = 1,
    deleted = 0;

INSERT INTO sys_role_permission (role_id, permission_id, created_at)
SELECT
    r.id,
    p.id,
    NOW()
FROM sys_role r
JOIN sys_permission p ON p.code IN (
    'iam.user-role.assign',
    'iam.user-resource.assign',
    'iam.user-permission.view'
)
WHERE r.code = 'admin'
ON CONFLICT (role_id, permission_id) DO NOTHING;

INSERT INTO sys_user_role (user_id, role_id, create_time)
SELECT
    u.id,
    r.id,
    NOW()
FROM sys_user u
JOIN sys_role r ON r.code = 'admin'
WHERE u.username = 'admin' AND u.deleted = 0
ON CONFLICT (user_id, role_id) DO NOTHING;

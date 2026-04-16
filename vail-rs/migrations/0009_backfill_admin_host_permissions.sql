-- 0009_backfill_admin_host_permissions.sql
-- Ensure admin role and admin user always have host management permissions.

INSERT INTO sys_role (name, code, description, status, create_time, deleted)
VALUES ('Administrator', 'admin', 'System administrator role', 1, NOW(), 0)
ON CONFLICT (code) DO UPDATE
SET
    name = EXCLUDED.name,
    description = EXCLUDED.description,
    status = 1,
    deleted = 0;

INSERT INTO sys_permission (code, name, description)
VALUES
    ('iam.user-role.assign', 'Assign User Roles', 'Assign role set to target user'),
    ('iam.user-resource.assign', 'Assign User Resources', 'Assign host resources to target user'),
    ('iam.user-permission.view', 'View User Permissions', 'View effective role and permission set'),
    ('host.read', 'Read Hosts', 'View host and host-group resources'),
    ('host.create', 'Create Host', 'Create host and host-group resources'),
    ('host.update', 'Update Host', 'Update host and host-group resources'),
    ('host.delete', 'Delete Host', 'Delete host and host-group resources')
ON CONFLICT (code) DO NOTHING;

INSERT INTO sys_role_permission (role_id, permission_id, created_at)
SELECT
    r.id,
    p.id,
    NOW()
FROM sys_role r
JOIN sys_permission p ON p.code IN (
    'iam.user-role.assign',
    'iam.user-resource.assign',
    'iam.user-permission.view',
    'host.read',
    'host.create',
    'host.update',
    'host.delete'
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

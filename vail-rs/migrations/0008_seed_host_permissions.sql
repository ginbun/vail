-- 0008_seed_host_permissions.sql
-- Seed host management permissions and bind to admin role

INSERT INTO sys_permission (code, name, description)
VALUES
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
    'host.read',
    'host.create',
    'host.update',
    'host.delete'
)
WHERE r.code = 'admin'
ON CONFLICT (role_id, permission_id) DO NOTHING;

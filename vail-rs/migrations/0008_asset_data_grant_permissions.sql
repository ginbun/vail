-- 0008_asset_data_grant_permissions.sql

INSERT INTO sys_permission (code, name, description)
VALUES
    ('asset.data-grant.host-group.assign', 'Assign Host Group Grant', 'Assign host-group data grants to users or roles'),
    ('asset.data-grant.host-group.view', 'View Host Group Grant', 'View host-group data grants for users or roles'),
    ('asset.data-grant.host-key.assign', 'Assign Host Key Grant', 'Assign host-key data grants to users or roles'),
    ('asset.data-grant.host-key.view', 'View Host Key Grant', 'View host-key data grants for users or roles'),
    ('asset.data-grant.host-identity.assign', 'Assign Host Identity Grant', 'Assign host-identity data grants to users or roles'),
    ('asset.data-grant.host-identity.view', 'View Host Identity Grant', 'View host-identity data grants for users or roles')
ON CONFLICT (code) DO NOTHING;

INSERT INTO sys_role_permission (role_id, permission_id, created_at)
SELECT r.id, p.id, NOW()
FROM sys_role r
JOIN sys_permission p
    ON p.code IN (
        'asset.data-grant.host-group.assign',
        'asset.data-grant.host-group.view',
        'asset.data-grant.host-key.assign',
        'asset.data-grant.host-key.view',
        'asset.data-grant.host-identity.assign',
        'asset.data-grant.host-identity.view'
    )
WHERE r.code = 'admin'
ON CONFLICT (role_id, permission_id) DO NOTHING;

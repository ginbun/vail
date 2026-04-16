-- 0007_seed_admin_user.sql
-- Seed default admin user and ensure admin role binding

CREATE EXTENSION IF NOT EXISTS pgcrypto;

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
)
ON CONFLICT (username) DO NOTHING;

INSERT INTO sys_user_role (user_id, role_id, create_time)
SELECT
    u.id,
    r.id,
    NOW()
FROM sys_user u
JOIN sys_role r ON r.code = 'admin'
WHERE u.username = 'admin' AND u.deleted = 0
ON CONFLICT (user_id, role_id) DO NOTHING;

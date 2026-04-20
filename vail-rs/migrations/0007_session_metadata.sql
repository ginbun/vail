-- 0007_session_metadata.sql
-- Add IP, Location and User-Agent to auth_refresh_token for better auditing.

ALTER TABLE auth_refresh_token
    ADD COLUMN IF NOT EXISTS ip VARCHAR(64),
    ADD COLUMN IF NOT EXISTS location VARCHAR(128),
    ADD COLUMN IF NOT EXISTS user_agent VARCHAR(512);

-- Update existing records if any
UPDATE auth_refresh_token r
SET ip = l.ip,
    location = l.location,
    user_agent = l.user_agent
FROM (
    SELECT DISTINCT ON (user_id, create_time) user_id, ip, location, user_agent, create_time
    FROM login_log
    ORDER BY user_id, create_time DESC
) l
WHERE r.user_id = l.user_id
  AND r.created_at >= l.create_time - interval '2 seconds'
  AND r.created_at <= l.create_time + interval '2 seconds'
  AND r.ip IS NULL;

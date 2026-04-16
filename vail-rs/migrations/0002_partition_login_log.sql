-- V2__partition_login_log.sql
-- 登录日志分区表

-- 登录日志 - 声明式分区 (月度)
CREATE TABLE login_log (
    id BIGSERIAL,
    user_id BIGINT,
    username VARCHAR(32),
    ip VARCHAR(64),
    location VARCHAR(128),
    user_agent VARCHAR(256),
    result SMALLINT,
    error_message TEXT,
    create_time TIMESTAMP NOT NULL DEFAULT NOW()
) PARTITION BY RANGE (create_time);

-- 默认分区 (当没有匹配分区时使用)
CREATE TABLE login_log_default PARTITION OF login_log
    DEFAULT;

-- 自动分区函数
CREATE OR REPLACE FUNCTION create_login_log_partition()
RETURNS void AS $$
DECLARE
    current_month TEXT;
    partition_name TEXT;
BEGIN
    current_month := to_char(NOW(), 'YYYY_MM');
    partition_name := 'login_log_' || current_month;
    
    IF NOT EXISTS (
        SELECT 1 FROM pg_tables 
        WHERE tablename = partition_name
    ) THEN
        EXECUTE format(
            'CREATE TABLE %I PARTITION OF login_log FOR VALUES FROM (%L) TO (%L)',
            partition_name,
            date_trunc('month', NOW()),
            date_trunc('month', NOW() + interval '1 month')
        );
    END IF;
END;
$$ LANGUAGE plpgsql;

-- 创建当月分区
SELECT create_login_log_partition();

-- 创建定时任务 (每月1号创建下月分区)
-- CREATE EVENT EVENT_create_login_log_partition 
-- ON SCHEDULE EVERY 1 MONTH 
-- DO SELECT create_login_log_partition();

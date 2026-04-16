-- V3__partition_operator_log.sql
-- 操作日志分区表

-- 操作日志 - 声明式分区 (月度)
CREATE TABLE operator_log (
    id BIGSERIAL,
    user_id BIGINT,
    username VARCHAR(32),
    module VARCHAR(32),
    operation VARCHAR(64),
    method VARCHAR(16),
    path VARCHAR(256),
    params JSONB,
    result SMALLINT,
    error_message TEXT,
    duration INT,
    trace_id VARCHAR(64),
    ip VARCHAR(64),
    user_agent VARCHAR(256),
    create_time TIMESTAMP NOT NULL DEFAULT NOW()
) PARTITION BY RANGE (create_time);

-- 默认分区
CREATE TABLE operator_log_default PARTITION OF operator_log
    DEFAULT;

-- 自动分区函数
CREATE OR REPLACE FUNCTION create_operator_log_partition()
RETURNS void AS $$
DECLARE
    current_month TEXT;
    partition_name TEXT;
BEGIN
    current_month := to_char(NOW(), 'YYYY_MM');
    partition_name := 'operator_log_' || current_month;
    
    IF NOT EXISTS (
        SELECT 1 FROM pg_tables 
        WHERE tablename = partition_name
    ) THEN
        EXECUTE format(
            'CREATE TABLE %I PARTITION OF operator_log FOR VALUES FROM (%L) TO (%L)',
            partition_name,
            date_trunc('month', NOW()),
            date_trunc('month', NOW() + interval '1 month')
        );
    END IF;
END;
$$ LANGUAGE plpgsql;

-- 创建当月分区
SELECT create_operator_log_partition();

-- 0005_operator_log_hardening.sql
-- Harden operator log access control and retention behavior.

ALTER TABLE operator_log
    ADD COLUMN IF NOT EXISTS deleted SMALLINT NOT NULL DEFAULT 0;

CREATE INDEX IF NOT EXISTS idx_operator_log_deleted_time
    ON operator_log (deleted, create_time DESC);

INSERT INTO sys_permission (code, name, description)
VALUES
    ('infra:operator-log:query', 'Query Operator Log', 'Query operator logs'),
    ('infra:operator-log:delete', 'Delete Operator Log', 'Delete operator log entries'),
    ('infra:operator-log:management:clear', 'Clear Operator Log', 'Clear operator log entries with filters')
ON CONFLICT (code) DO NOTHING;

INSERT INTO sys_role_permission (role_id, permission_id, created_at)
SELECT r.id, p.id, NOW()
FROM sys_role r
JOIN sys_permission p
  ON p.code IN (
      'infra:operator-log:query',
      'infra:operator-log:delete',
      'infra:operator-log:management:clear'
  )
WHERE r.code = 'admin'
ON CONFLICT (role_id, permission_id) DO NOTHING;

INSERT INTO sys_dict_key (key_name, value_type, extra_schema, description, creator, updater, create_time, update_time)
VALUES
    ('operatorLogModule', 'STRING', '', 'Operator log module', 'system', 'system', NOW(), NOW()),
    ('operatorLogType', 'STRING', '', 'Operator log type', 'system', 'system', NOW(), NOW()),
    ('operatorRiskLevel', 'STRING', '', 'Operator risk level', 'system', 'system', NOW(), NOW()),
    ('operatorLogResult', 'NUMBER', '', 'Operator log result', 'system', 'system', NOW(), NOW())
ON CONFLICT (key_name) DO NOTHING;

INSERT INTO sys_dict_value (key_id, name, value, label, extra, sort, creator, updater, create_time, update_time, deleted)
SELECT dk.id, src.name, src.value, src.label, src.extra, src.sort, 'system', 'system', NOW(), NOW(), 0
FROM (
    VALUES
        ('operatorLogModule', 'IAM', 'iam', 'IAM', '{}', 10),
        ('operatorLogModule', 'SSH', 'ssh', 'SSH', '{}', 20),
        ('operatorLogModule', 'SFTP', 'sftp', 'SFTP', '{}', 30),

        ('operatorLogType', 'Assign User Roles', 'assign_user_roles', '分配用户角色', '{}', 10),
        ('operatorLogType', 'Assign User Hosts', 'assign_user_hosts', '分配用户主机', '{}', 20),
        ('operatorLogType', 'Create SSH Session', 'create_ssh_session', '创建 SSH 会话', '{}', 30),
        ('operatorLogType', 'Disconnect SSH Session', 'disconnect_ssh_session', '断开 SSH 会话', '{}', 40),
        ('operatorLogType', 'SFTP Upload Batch', 'sftp_upload_batch', 'SFTP 批量上传', '{}', 50),
        ('operatorLogType', 'SFTP Create Task', 'sftp_create_upload_task', 'SFTP 创建上传任务', '{}', 60),
        ('operatorLogType', 'SFTP Complete Task', 'sftp_complete_upload_task', 'SFTP 完成上传任务', '{}', 70),

        ('operatorRiskLevel', 'Low', 'LOW', '低', '{"color":"green"}', 10),
        ('operatorRiskLevel', 'Medium', 'MEDIUM', '中', '{"color":"orange"}', 20),
        ('operatorRiskLevel', 'High', 'HIGH', '高', '{"color":"orangered"}', 30),

        ('operatorLogResult', 'Failed', '0', '失败', '{"color":"orangered"}', 10),
        ('operatorLogResult', 'Success', '1', '成功', '{"color":"green"}', 20)
) AS src(key_name, name, value, label, extra, sort)
JOIN sys_dict_key dk ON dk.key_name = src.key_name
ON CONFLICT (key_id, value) DO UPDATE
SET name = EXCLUDED.name,
    label = EXCLUDED.label,
    extra = EXCLUDED.extra,
    sort = EXCLUDED.sort,
    updater = EXCLUDED.updater,
    update_time = EXCLUDED.update_time,
    deleted = 0;

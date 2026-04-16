use sqlx::PgPool;

pub async fn ensure_partitions(pool: &PgPool) {
    sqlx::query("SELECT ensure_weekly_partitions('login_log', 12)")
        .execute(pool)
        .await
        .ok();

    sqlx::query("SELECT ensure_weekly_partitions('operator_log', 12)")
        .execute(pool)
        .await
        .ok();

    sqlx::query("SELECT ensure_weekly_partitions('ssh_session', 12)")
        .execute(pool)
        .await
        .ok();
}

pub async fn ensure_default_admin_menu(pool: &PgPool) {
    sqlx::query(
        "WITH desired(parent_component, name, path, component, icon, type, sort, visible, permission) AS (
            VALUES
                (NULL::text, '工作台', '/workplace', 'workplace', 'icon-dashboard', 2, 0, 1, NULL::text),
                (NULL::text, '用户管理', '/user-module', 'userModule', 'icon-user', 1, 10, 1, NULL::text),
                ('userModule', '角色管理', '/user/role', 'userRole', 'icon-safe', 2, 1, 1, NULL::text),
                ('userModule', '用户管理', '/user/list', 'userList', 'icon-user-group', 2, 2, 1, NULL::text),
                ('userModule', '个人信息', '/user/info', 'userInfo', 'icon-user', 2, 3, 1, NULL::text),
                ('userModule', '操作日志', '/user/operator-log', 'operatorLog', 'icon-file', 2, 4, 1, NULL::text),
                ('userModule', '会话管理', '/user/session', 'userSession', 'icon-schedule', 2, 5, 1, NULL::text),
                ('userModule', '锁定用户', '/user/locked', 'lockedUser', 'icon-lock', 2, 6, 1, NULL::text),
                (NULL::text, '系统管理', '/system-module', 'systemModule', 'icon-settings', 1, 20, 1, NULL::text),
                ('systemModule', '菜单管理', '/system/menu', 'systemMenu', 'icon-menu', 2, 1, 1, NULL::text),
                ('systemModule', '字典键', '/system/dict-key', 'dictKey', 'icon-book', 2, 2, 1, NULL::text),
                ('systemModule', '字典值', '/system/dict-value', 'dictValue', 'icon-list', 2, 3, 1, NULL::text),
                ('systemModule', '通知模板', '/system/notify-template', 'notifyTemplate', 'icon-notification', 2, 4, 1, NULL::text),
                ('systemModule', '系统标签', '/system/tags', 'systemTags', 'icon-tag', 2, 5, 1, NULL::text),
                ('systemModule', '系统设置', '/system/setting', 'systemSetting', 'icon-settings', 2, 6, 1, NULL::text),
                (NULL::text, '资产管理', '/asset-module', 'assetModule', 'icon-storage', 1, 30, 1, NULL::text),
                ('assetModule', '主机列表', '/asset/host', 'hostList', 'icon-desktop', 2, 1, 1, NULL::text),
                ('assetModule', '主机密钥', '/asset/host-key', 'hostKey', 'icon-key', 2, 2, 1, NULL::text),
                ('assetModule', '凭证身份', '/asset/host-identity', 'hostIdentity', 'icon-safe', 2, 3, 1, NULL::text),
                ('assetModule', '授权管理', '/asset/grant', 'assetGrant', 'icon-share-alt', 2, 4, 1, NULL::text),
                (NULL::text, '审计中心', '/asset-audit-module', 'assetAuditModule', 'icon-search', 1, 40, 1, NULL::text),
                ('assetAuditModule', '终端连接日志', '/audit/terminal-connect-log', 'terminalConnectLog', 'icon-file', 2, 1, 1, NULL::text),
                ('assetAuditModule', '终端连接会话', '/audit/terminal-connect-session', 'terminalConnectSession', 'icon-history', 2, 2, 1, NULL::text),
                ('assetAuditModule', '终端文件日志', '/audit/terminal-file-log', 'terminalFileLog', 'icon-folder', 2, 3, 1, NULL::text),
                (NULL::text, '终端管理', '/terminal-module', 'terminalModule', 'icon-terminal', 1, 50, 1, NULL::text),
                ('terminalModule', '终端', '/terminal', 'terminal', 'icon-terminal', 2, 1, 1, NULL::text),
                (NULL::text, '批量执行', '/exec-module', 'execModule', 'icon-command', 1, 60, 1, NULL::text),
                ('execModule', '批量命令', '/exec/command', 'execCommand', 'icon-code', 2, 1, 1, NULL::text),
                ('execModule', '命令日志', '/exec/command-log', 'execCommandLog', 'icon-file', 2, 2, 1, NULL::text),
                ('execModule', '执行计划', '/exec/job', 'execJob', 'icon-calendar', 2, 3, 1, NULL::text),
                ('execModule', '计划日志', '/exec/job-log', 'execJobLog', 'icon-history', 2, 4, 1, NULL::text),
                ('execModule', '批量上传', '/exec/upload', 'batchUpload', 'icon-upload', 2, 5, 1, NULL::text),
                ('execModule', '上传任务', '/exec/upload-task', 'uploadTask', 'icon-list', 2, 6, 1, NULL::text),
                ('execModule', '命令模板', '/exec/template', 'execTemplate', 'icon-apps', 2, 7, 1, NULL::text),
                (NULL::text, '监控中心', '/monitor-module', 'monitorModule', 'icon-dashboard', 1, 70, 1, NULL::text),
                ('monitorModule', '指标监控', '/monitor/metrics', 'metrics', 'icon-bar-chart', 2, 1, 1, NULL::text),
                ('monitorModule', '主机监控', '/monitor/monitor-host', 'monitorHost', 'icon-desktop', 2, 2, 1, NULL::text),
                ('monitorModule', '告警策略', '/monitor/alarm-policy', 'alarmPolicy', 'icon-alarm', 2, 3, 1, NULL::text),
                ('monitorModule', '告警事件', '/monitor/alarm-event', 'hostAlarmEvent', 'icon-bell', 2, 4, 1, NULL::text),
                ('monitorModule', '监控详情', '/monitor/detail', 'monitorDetail', 'icon-search', 2, 5, 0, NULL::text),
                ('monitorModule', '告警规则', '/monitor/alarm-rule', 'alarmRule', 'icon-settings', 2, 6, 0, NULL::text),
                (NULL::text, '执行详情', '/exec-full-module', 'execFullModule', 'icon-file', 1, 61, 0, NULL::text),
                ('execFullModule', '计划日志详情', '/exec/job-log/view', 'execJobLogView', 'icon-file', 2, 1, 0, NULL::text)
        ),
        resolved AS (
            SELECT
                d.parent_component,
                COALESCE((SELECT MIN(pm.id) FROM sys_menu pm WHERE pm.component = d.parent_component), 0) AS parent_id,
                d.name,
                d.path,
                d.component,
                d.icon,
                d.type,
                d.sort,
                d.visible,
                d.permission
            FROM desired d
        )
        INSERT INTO sys_menu (
            parent_id,
            name,
            path,
            component,
            icon,
            type,
            sort,
            visible,
            permission,
            create_time
        )
        SELECT
            r.parent_id,
            r.name,
            r.path,
            r.component,
            r.icon,
            r.type,
            r.sort,
            r.visible,
            r.permission,
            NOW()
        FROM resolved r
        WHERE (r.parent_component IS NULL OR r.parent_id > 0)
          AND NOT EXISTS (
              SELECT 1
              FROM sys_menu m
              WHERE m.component = r.component
          )",
    )
    .execute(pool)
    .await
    .ok();

    sqlx::query(
        "WITH desired(parent_component, name, path, component, icon, type, sort, visible, permission) AS (
            VALUES
                ('userModule', '角色管理', '/user/role', 'userRole', 'icon-safe', 2, 1, 1, NULL::text),
                ('userModule', '用户管理', '/user/list', 'userList', 'icon-user-group', 2, 2, 1, NULL::text),
                ('userModule', '个人信息', '/user/info', 'userInfo', 'icon-user', 2, 3, 1, NULL::text),
                ('userModule', '操作日志', '/user/operator-log', 'operatorLog', 'icon-file', 2, 4, 1, NULL::text),
                ('userModule', '会话管理', '/user/session', 'userSession', 'icon-schedule', 2, 5, 1, NULL::text),
                ('userModule', '锁定用户', '/user/locked', 'lockedUser', 'icon-lock', 2, 6, 1, NULL::text),
                ('systemModule', '菜单管理', '/system/menu', 'systemMenu', 'icon-menu', 2, 1, 1, NULL::text),
                ('systemModule', '字典键', '/system/dict-key', 'dictKey', 'icon-book', 2, 2, 1, NULL::text),
                ('systemModule', '字典值', '/system/dict-value', 'dictValue', 'icon-list', 2, 3, 1, NULL::text),
                ('systemModule', '通知模板', '/system/notify-template', 'notifyTemplate', 'icon-notification', 2, 4, 1, NULL::text),
                ('systemModule', '系统标签', '/system/tags', 'systemTags', 'icon-tag', 2, 5, 1, NULL::text),
                ('systemModule', '系统设置', '/system/setting', 'systemSetting', 'icon-settings', 2, 6, 1, NULL::text),
                ('assetModule', '主机列表', '/asset/host', 'hostList', 'icon-desktop', 2, 1, 1, NULL::text),
                ('assetModule', '主机密钥', '/asset/host-key', 'hostKey', 'icon-key', 2, 2, 1, NULL::text),
                ('assetModule', '凭证身份', '/asset/host-identity', 'hostIdentity', 'icon-safe', 2, 3, 1, NULL::text),
                ('assetModule', '授权管理', '/asset/grant', 'assetGrant', 'icon-share-alt', 2, 4, 1, NULL::text),
                ('assetAuditModule', '终端连接日志', '/audit/terminal-connect-log', 'terminalConnectLog', 'icon-file', 2, 1, 1, NULL::text),
                ('assetAuditModule', '终端连接会话', '/audit/terminal-connect-session', 'terminalConnectSession', 'icon-history', 2, 2, 1, NULL::text),
                ('assetAuditModule', '终端文件日志', '/audit/terminal-file-log', 'terminalFileLog', 'icon-folder', 2, 3, 1, NULL::text),
                ('terminalModule', '终端', '/terminal', 'terminal', 'icon-terminal', 2, 1, 1, NULL::text),
                ('execModule', '批量命令', '/exec/command', 'execCommand', 'icon-code', 2, 1, 1, NULL::text),
                ('execModule', '命令日志', '/exec/command-log', 'execCommandLog', 'icon-file', 2, 2, 1, NULL::text),
                ('execModule', '执行计划', '/exec/job', 'execJob', 'icon-calendar', 2, 3, 1, NULL::text),
                ('execModule', '计划日志', '/exec/job-log', 'execJobLog', 'icon-history', 2, 4, 1, NULL::text),
                ('execModule', '批量上传', '/exec/upload', 'batchUpload', 'icon-upload', 2, 5, 1, NULL::text),
                ('execModule', '上传任务', '/exec/upload-task', 'uploadTask', 'icon-list', 2, 6, 1, NULL::text),
                ('execModule', '命令模板', '/exec/template', 'execTemplate', 'icon-apps', 2, 7, 1, NULL::text),
                ('monitorModule', '指标监控', '/monitor/metrics', 'metrics', 'icon-bar-chart', 2, 1, 1, NULL::text),
                ('monitorModule', '主机监控', '/monitor/monitor-host', 'monitorHost', 'icon-desktop', 2, 2, 1, NULL::text),
                ('monitorModule', '告警策略', '/monitor/alarm-policy', 'alarmPolicy', 'icon-alarm', 2, 3, 1, NULL::text),
                ('monitorModule', '告警事件', '/monitor/alarm-event', 'hostAlarmEvent', 'icon-bell', 2, 4, 1, NULL::text),
                ('monitorModule', '监控详情', '/monitor/detail', 'monitorDetail', 'icon-search', 2, 5, 0, NULL::text),
                ('monitorModule', '告警规则', '/monitor/alarm-rule', 'alarmRule', 'icon-settings', 2, 6, 0, NULL::text),
                ('execFullModule', '计划日志详情', '/exec/job-log/view', 'execJobLogView', 'icon-file', 2, 1, 0, NULL::text)
        )
        INSERT INTO sys_menu (
            parent_id,
            name,
            path,
            component,
            icon,
            type,
            sort,
            visible,
            permission,
            create_time
        )
        SELECT
            p.id,
            d.name,
            d.path,
            d.component,
            d.icon,
            d.type,
            d.sort,
            d.visible,
            d.permission,
            NOW()
        FROM desired d
        JOIN sys_menu p ON p.component = d.parent_component
        WHERE NOT EXISTS (
            SELECT 1
            FROM sys_menu m
            WHERE m.component = d.component
        )",
    )
    .execute(pool)
    .await
    .ok();

    sqlx::query(
        "INSERT INTO sys_role_menu (role_id, menu_id)
         SELECT r.id, m.id
         FROM sys_role r
         JOIN sys_menu m ON TRUE
         WHERE r.code = 'admin'
         ON CONFLICT (role_id, menu_id) DO NOTHING",
    )
    .execute(pool)
    .await
    .ok();
}

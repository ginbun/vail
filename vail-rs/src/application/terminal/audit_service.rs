use crate::{
    application::orion::compat_service,
    domain::orion::compat::OrionCompatModule,
    domain::terminal::command_audit::SshCommandAuditSnapshot,
};

pub struct TerminalCommandSnapshotRecordInput<'a> {
    pub user_id: i64,
    pub host_id: i64,
    pub username: &'a str,
    pub host_name: &'a str,
    pub host_address: &'a str,
    pub session_id: &'a str,
    pub start_time: i64,
    pub end_time: i64,
    pub snapshot: SshCommandAuditSnapshot,
}

pub async fn append_terminal_command_snapshot(
    db: &sqlx::PgPool,
    input: TerminalCommandSnapshotRecordInput<'_>,
) {
    if input.snapshot.command_count() == 0 {
        return;
    }

    let command_count = input.snapshot.command_count();
    let truncated = input.snapshot.is_truncated();
    let commands = input.snapshot.into_commands();

    let payload = serde_json::json!({
        "userId": input.user_id,
        "username": input.username,
        "hostId": input.host_id,
        "hostName": input.host_name,
        "hostAddress": input.host_address,
        "address": "",
        "location": "",
        "userAgent": "",
        "paths": [],
        "type": "terminal:ssh-command-snapshot",
        "result": 1,
        "startTime": input.start_time,
        "extra": {
            "sessionId": input.session_id,
            "endTime": input.end_time,
            "commandCount": command_count,
            "truncated": truncated,
            "commandsMasked": commands
        }
    });

    let _ = compat_service::create_record(
        db,
        OrionCompatModule::TerminalFileLog,
        payload,
        &format!("user-{}", input.user_id),
    )
    .await;
}

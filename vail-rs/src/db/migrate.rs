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

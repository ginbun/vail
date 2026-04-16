pub mod entities;
pub mod migrate;

use sqlx::migrate::Migrator;
use sqlx::postgres::{PgPool, PgPoolOptions};

use crate::config::DatabaseConfig;

static MIGRATOR: Migrator = sqlx::migrate!("./migrations");

pub async fn init_pool(config: &DatabaseConfig) -> PgPool {
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&config.connection_string())
        .await
        .expect("Failed to create database pool");

    pool
}

pub async fn run_migrations(pool: &PgPool) {
    MIGRATOR.run(pool).await.expect("Failed to run migrations");
}

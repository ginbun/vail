pub mod migrate;

use sqlx::migrate::Migrator;
use sqlx::postgres::{PgPool, PgPoolOptions};

use crate::config::DatabaseConfig;

static MIGRATOR: Migrator = sqlx::migrate!("./migrations");

pub async fn init_pool(config: &DatabaseConfig) -> PgPool {
    match PgPoolOptions::new()
        .max_connections(5)
        .connect(&config.connection_string())
        .await
    {
        Ok(pool) => pool,
        Err(e) => {
            eprintln!("Failed to create database pool: {}", e);
            std::process::exit(1);
        }
    }
}

pub async fn run_migrations(pool: &PgPool) {
    if let Err(e) = MIGRATOR.run(pool).await {
        eprintln!("Failed to run migrations: {}", e);
        std::process::exit(1);
    }
}

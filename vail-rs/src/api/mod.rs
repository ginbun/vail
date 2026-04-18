pub mod auth;
pub mod guard;
pub mod host;
pub mod iam;
pub mod jit;
pub mod orion;
pub mod sftp;
pub mod ssh;
pub mod ssh_key;
pub mod terminal;
pub mod web;

use sqlx::PgPool;

use crate::config::Config;

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub config: Config,
}

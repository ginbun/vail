use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    #[serde(default = "default_server")]
    pub server: ServerConfig,
    #[serde(default = "default_database")]
    pub database: DatabaseConfig,
    #[serde(default)]
    pub jwt: JwtConfig,
    #[serde(default)]
    pub ssh: SshConfig,
    #[serde(default)]
    pub storage: StorageConfig,
    #[serde(default)]
    pub secrets: SecretsConfig,
}

fn default_server() -> ServerConfig {
    ServerConfig {
        host: "0.0.0.0".to_string(),
        port: 8080,
    }
}

fn default_database() -> DatabaseConfig {
    DatabaseConfig {
        host: "localhost".to_string(),
        port: 5432,
        username: "postgres".to_string(),
        password: "postgres".to_string(),
        database: "vail".to_string(),
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DatabaseConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub database: String,
}

impl DatabaseConfig {
    pub fn connection_string(&self) -> String {
        format!(
            "postgres://{}:{}@{}:{}/{}",
            self.username, self.password, self.host, self.port, self.database
        )
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct JwtConfig {
    #[serde(default = "default_jwt_secret")]
    pub secret: String,
    #[serde(default = "default_expiration")]
    pub expiration: u64,
    #[serde(default = "default_refresh_expiration")]
    pub refresh_expiration: u64,
}

impl Default for JwtConfig {
    fn default() -> Self {
        Self {
            secret: default_jwt_secret(),
            expiration: default_expiration(),
            refresh_expiration: default_refresh_expiration(),
        }
    }
}

fn default_jwt_secret() -> String {
    "vail-secret-key-change-in-production".to_string()
}

fn default_expiration() -> u64 {
    3600
}

fn default_refresh_expiration() -> u64 {
    604800
}

#[derive(Debug, Deserialize, Clone)]
pub struct SshConfig {
    #[serde(default = "default_ssh_port")]
    pub default_port: u16,
    #[serde(default = "default_connection_timeout")]
    pub connection_timeout: u64,
    #[serde(default = "default_keepalive_interval")]
    pub keepalive_interval: u64,
}

impl Default for SshConfig {
    fn default() -> Self {
        Self {
            default_port: default_ssh_port(),
            connection_timeout: default_connection_timeout(),
            keepalive_interval: default_keepalive_interval(),
        }
    }
}

fn default_ssh_port() -> u16 {
    22
}

fn default_connection_timeout() -> u64 {
    30
}

fn default_keepalive_interval() -> u64 {
    60
}

#[derive(Debug, Deserialize, Clone)]
pub struct StorageConfig {
    #[serde(default = "default_temp_dir")]
    pub temp_dir: String,
    #[serde(default = "default_max_upload_size")]
    pub max_upload_size: u64,
    #[serde(default = "default_chunk_size")]
    pub default_chunk_size: u64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct SecretsConfig {
    #[serde(default = "default_data_encryption_key")]
    pub data_encryption_key: String,
}

impl Default for SecretsConfig {
    fn default() -> Self {
        Self {
            data_encryption_key: default_data_encryption_key(),
        }
    }
}

fn default_data_encryption_key() -> String {
    std::env::var("VAIL_DATA_ENCRYPTION_KEY").unwrap_or_default()
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            temp_dir: default_temp_dir(),
            max_upload_size: default_max_upload_size(),
            default_chunk_size: default_chunk_size(),
        }
    }
}

fn default_temp_dir() -> String {
    "/tmp/vail".to_string()
}

fn default_max_upload_size() -> u64 {
    1024
}

fn default_chunk_size() -> u64 {
    1048576
}

pub fn load_config() -> Config {
    let config_path = std::env::var("VAIL_CONFIG").unwrap_or_else(|_| "config.toml".to_string());
    let path = PathBuf::from(&config_path);

    if path.exists() {
        let content = std::fs::read_to_string(&path).unwrap();
        toml::from_str(&content).unwrap_or_else(|e| {
            tracing::warn!("Failed to parse config, using defaults: {}", e);
            Config::default()
        })
    } else {
        tracing::warn!("Config file not found, using defaults");
        Config::default()
    }
}

impl Default for Config {
    fn default() -> Self {
        Config {
            server: ServerConfig {
                host: "0.0.0.0".to_string(),
                port: 8080,
            },
            database: DatabaseConfig {
                host: "localhost".to_string(),
                port: 5432,
                username: "postgres".to_string(),
                password: "postgres".to_string(),
                database: "vail".to_string(),
            },
            jwt: JwtConfig {
                secret: "vail-secret-key".to_string(),
                expiration: 3600,
                refresh_expiration: 604800,
            },
            ssh: SshConfig {
                default_port: 22,
                connection_timeout: 30,
                keepalive_interval: 60,
            },
            storage: StorageConfig {
                temp_dir: "/tmp/vail".to_string(),
                max_upload_size: 1024,
                default_chunk_size: 1048576,
            },
            secrets: SecretsConfig {
                data_encryption_key: default_data_encryption_key(),
            },
        }
    }
}

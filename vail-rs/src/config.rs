use base64::{engine::general_purpose::STANDARD, Engine as _};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey};
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
    #[serde(default = "default_jwt_algorithm")]
    pub algorithm: Algorithm,
    #[serde(default = "default_jwt_secret")]
    pub secret: String,
    #[serde(default)]
    pub private_key: String,
    #[serde(default)]
    pub public_key: String,
    #[serde(default = "default_expiration")]
    pub expiration: u64,
    #[serde(default = "default_refresh_expiration")]
    pub refresh_expiration: u64,
}

impl Default for JwtConfig {
    fn default() -> Self {
        Self {
            algorithm: default_jwt_algorithm(),
            secret: default_jwt_secret(),
            private_key: String::new(),
            public_key: String::new(),
            expiration: default_expiration(),
            refresh_expiration: default_refresh_expiration(),
        }
    }
}

impl JwtConfig {
    const ED25519_SPKI_PREFIX: [u8; 12] = [
        0x30, 0x2a, 0x30, 0x05, 0x06, 0x03, 0x2b, 0x65, 0x70, 0x03, 0x21, 0x00,
    ];

    fn decode_ed25519_der(&self, value: &str, field_name: &str) -> Result<Vec<u8>, String> {
        STANDARD
            .decode(value.trim())
            .map_err(|e| format!("invalid jwt.{field_name}: expected base64-encoded DER: {e}"))
    }

    fn decode_ed25519_public_key(&self) -> Result<Vec<u8>, String> {
        let decoded = self.decode_ed25519_der(&self.public_key, "public_key")?;
        if decoded.len() == 32 {
            return Ok(decoded);
        }

        if decoded.starts_with(&Self::ED25519_SPKI_PREFIX)
            && decoded.len() == 32 + Self::ED25519_SPKI_PREFIX.len()
        {
            return Ok(decoded[Self::ED25519_SPKI_PREFIX.len()..].to_vec());
        }

        Err(
            "invalid jwt.public_key: expected base64-encoded Ed25519 public key or SPKI DER"
                .to_string(),
        )
    }

    pub fn signing_key(&self) -> Result<EncodingKey, String> {
        match self.algorithm {
            Algorithm::HS256 => Ok(EncodingKey::from_secret(self.secret.as_bytes())),
            Algorithm::EdDSA => {
                let der = self.decode_ed25519_der(&self.private_key, "private_key")?;
                Ok(EncodingKey::from_ed_der(&der))
            }
            other => Err(format!("unsupported jwt.algorithm: {other:?}")),
        }
    }

    pub fn verification_key(&self) -> Result<DecodingKey, String> {
        match self.algorithm {
            Algorithm::HS256 => Ok(DecodingKey::from_secret(self.secret.as_bytes())),
            Algorithm::EdDSA => {
                let public_key = self.decode_ed25519_public_key()?;
                Ok(DecodingKey::from_ed_der(&public_key))
            }
            other => Err(format!("unsupported jwt.algorithm: {other:?}")),
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        match self.algorithm {
            Algorithm::HS256 => {
                if self.secret.trim().is_empty() {
                    return Err("jwt.secret is required when jwt.algorithm=HS256".to_string());
                }

                if self.secret.trim().len() < 32 {
                    return Err("jwt.secret must be at least 32 characters for HS256".to_string());
                }

                let weak_defaults = [
                    "vail-secret-key-change-in-production",
                    "vail-secret-key",
                    "please-change-this-in-production",
                    "your-super-secret-jwt-key-change-in-production",
                ];
                if weak_defaults.contains(&self.secret.trim()) {
                    return Err("jwt.secret is using an insecure default value".to_string());
                }

                let _ = self.signing_key()?;
                let _ = self.verification_key()?;
                Ok(())
            }
            Algorithm::EdDSA => {
                if self.private_key.trim().is_empty() {
                    return Err("jwt.private_key is required when jwt.algorithm=EdDSA".to_string());
                }
                if self.public_key.trim().is_empty() {
                    return Err("jwt.public_key is required when jwt.algorithm=EdDSA".to_string());
                }

                let _ = self.signing_key()?;
                let _ = self.verification_key()?;
                Ok(())
            }
            other => Err(format!(
                "unsupported jwt.algorithm: {other:?}. only HS256 and EdDSA are supported"
            )),
        }
    }
}

fn default_jwt_algorithm() -> Algorithm {
    Algorithm::EdDSA
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

    let mut config = if path.exists() {
        let content = std::fs::read_to_string(&path).unwrap();
        toml::from_str(&content).unwrap_or_else(|e| {
            tracing::warn!("Failed to parse config, using defaults: {}", e);
            Config::default()
        })
    } else {
        tracing::warn!("Config file not found, using defaults");
        Config::default()
    };

    apply_env_overrides(&mut config);

    config
        .validate()
        .unwrap_or_else(|e| panic!("invalid configuration: {e}"));

    config
}

impl Config {
    pub fn validate(&self) -> Result<(), String> {
        self.jwt.validate()?;

        if self.secrets.data_encryption_key.trim().len() < 32 {
            return Err("secrets.data_encryption_key must be at least 32 characters".to_string());
        }

        Ok(())
    }
}

fn env_string(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

fn env_u16(name: &str) -> Option<u16> {
    env_string(name).map(|v| {
        v.parse::<u16>()
            .unwrap_or_else(|_| panic!("invalid {name}: expected u16, got '{v}'"))
    })
}

fn env_u64(name: &str) -> Option<u64> {
    env_string(name).map(|v| {
        v.parse::<u64>()
            .unwrap_or_else(|_| panic!("invalid {name}: expected u64, got '{v}'"))
    })
}

fn env_algorithm(name: &str) -> Option<Algorithm> {
    env_string(name).map(|v| match v.to_uppercase().as_str() {
        "HS256" => Algorithm::HS256,
        "EDDSA" => Algorithm::EdDSA,
        _ => panic!("invalid {name}: only HS256 or EdDSA are supported"),
    })
}

fn apply_env_overrides(config: &mut Config) {
    if let Some(v) = env_string("VAIL_SERVER_HOST") {
        config.server.host = v;
    }
    if let Some(v) = env_u16("VAIL_SERVER_PORT") {
        config.server.port = v;
    }

    if let Some(v) = env_string("VAIL_DB_HOST") {
        config.database.host = v;
    }
    if let Some(v) = env_u16("VAIL_DB_PORT") {
        config.database.port = v;
    }
    if let Some(v) = env_string("VAIL_DB_USERNAME") {
        config.database.username = v;
    }
    if let Some(v) = env_string("VAIL_DB_PASSWORD") {
        config.database.password = v;
    }
    if let Some(v) = env_string("VAIL_DB_DATABASE") {
        config.database.database = v;
    }

    if let Some(v) = env_algorithm("VAIL_JWT_ALGORITHM") {
        config.jwt.algorithm = v;
    }
    if let Some(v) = env_string("VAIL_JWT_SECRET") {
        config.jwt.secret = v;
    }
    if let Some(v) = env_string("VAIL_JWT_PRIVATE_KEY") {
        config.jwt.private_key = v;
    }
    if let Some(v) = env_string("VAIL_JWT_PUBLIC_KEY") {
        config.jwt.public_key = v;
    }
    if let Some(v) = env_u64("VAIL_JWT_EXPIRATION") {
        config.jwt.expiration = v;
    }
    if let Some(v) = env_u64("VAIL_JWT_REFRESH_EXPIRATION") {
        config.jwt.refresh_expiration = v;
    }

    if let Some(v) = env_string("VAIL_DATA_ENCRYPTION_KEY") {
        config.secrets.data_encryption_key = v;
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
                algorithm: default_jwt_algorithm(),
                secret: "vail-secret-key".to_string(),
                private_key: String::new(),
                public_key: String::new(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, OnceLock};

    const TEST_ED25519_PRIVATE_KEY: &str =
        "MC4CAQAwBQYDK2VwBCIEIHCDX8ke/yslwa9SElPghVHhz700q1H6SO9hmUJ6i8Ld";
    const TEST_ED25519_PUBLIC_KEY: &str = "sA29J+hOVKaDdV0/Ksm2B3zFrbDqFphgTpO79LTQ4zk=";
    const TEST_ED25519_PUBLIC_KEY_DER: &str =
        "MCowBQYDK2VwAyEAsA29J+hOVKaDdV0/Ksm2B3zFrbDqFphgTpO79LTQ4zk=";

    fn env_test_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    #[test]
    fn config_parses_jwt_algorithm_and_keys() {
        let cfg = toml::from_str::<Config>(
            r#"
            [jwt]
            algorithm = "EdDSA"
            private_key = "abc123base64"
            public_key = "def456base64"
            expiration = 3600
            refresh_expiration = 604800

            [secrets]
            data_encryption_key = "12345678901234567890123456789012"
            "#,
        )
        .expect("config should parse");

        assert_eq!(cfg.jwt.algorithm, jsonwebtoken::Algorithm::EdDSA);
        assert_eq!(cfg.jwt.private_key, "abc123base64");
        assert_eq!(cfg.jwt.public_key, "def456base64");
    }

    #[test]
    fn config_validation_rejects_weak_hs256_secret() {
        let mut cfg = Config::default();
        cfg.jwt.algorithm = jsonwebtoken::Algorithm::HS256;
        cfg.jwt.secret = "please-change-this-in-production".to_string();
        cfg.secrets.data_encryption_key = "12345678901234567890123456789012".to_string();

        let err = cfg.validate().expect_err("weak secret must fail");
        assert!(err.contains("jwt.secret"));
    }

    #[test]
    fn config_validation_accepts_eddsa_keypair() {
        let mut cfg = Config::default();
        cfg.jwt.algorithm = jsonwebtoken::Algorithm::EdDSA;
        cfg.jwt.private_key = TEST_ED25519_PRIVATE_KEY.to_string();
        cfg.jwt.public_key = TEST_ED25519_PUBLIC_KEY.to_string();
        cfg.secrets.data_encryption_key = "12345678901234567890123456789012".to_string();

        cfg.validate().expect("valid eddsa config");
    }

    #[test]
    fn config_validation_accepts_eddsa_der_public_key() {
        let mut cfg = Config::default();
        cfg.jwt.algorithm = jsonwebtoken::Algorithm::EdDSA;
        cfg.jwt.private_key = TEST_ED25519_PRIVATE_KEY.to_string();
        cfg.jwt.public_key = TEST_ED25519_PUBLIC_KEY_DER.to_string();
        cfg.secrets.data_encryption_key = "12345678901234567890123456789012".to_string();

        cfg.validate()
            .expect("valid eddsa config with der public key");
    }

    #[test]
    fn env_overrides_apply_to_server_database_and_jwt() {
        let _guard = env_test_lock().lock().expect("env lock");
        let vars = [
            "VAIL_SERVER_HOST",
            "VAIL_SERVER_PORT",
            "VAIL_DB_HOST",
            "VAIL_DB_PORT",
            "VAIL_DB_USERNAME",
            "VAIL_DB_PASSWORD",
            "VAIL_DB_DATABASE",
            "VAIL_JWT_ALGORITHM",
            "VAIL_JWT_PRIVATE_KEY",
            "VAIL_JWT_PUBLIC_KEY",
            "VAIL_JWT_EXPIRATION",
            "VAIL_JWT_REFRESH_EXPIRATION",
            "VAIL_DATA_ENCRYPTION_KEY",
        ];

        let previous: Vec<(String, Option<String>)> = vars
            .iter()
            .map(|k| (k.to_string(), std::env::var(k).ok()))
            .collect();

        std::env::set_var("VAIL_SERVER_HOST", "127.0.0.1");
        std::env::set_var("VAIL_SERVER_PORT", "18080");
        std::env::set_var("VAIL_DB_HOST", "db.internal");
        std::env::set_var("VAIL_DB_PORT", "15432");
        std::env::set_var("VAIL_DB_USERNAME", "dbuser");
        std::env::set_var("VAIL_DB_PASSWORD", "dbpass");
        std::env::set_var("VAIL_DB_DATABASE", "vail_prod");
        std::env::set_var("VAIL_JWT_ALGORITHM", "EdDSA");
        std::env::set_var(
            "VAIL_JWT_PRIVATE_KEY",
            TEST_ED25519_PRIVATE_KEY.replace('\n', "\\n"),
        );
        std::env::set_var(
            "VAIL_JWT_PUBLIC_KEY",
            TEST_ED25519_PUBLIC_KEY.replace('\n', "\\n"),
        );
        std::env::set_var("VAIL_JWT_EXPIRATION", "7200");
        std::env::set_var("VAIL_JWT_REFRESH_EXPIRATION", "1209600");
        std::env::set_var(
            "VAIL_DATA_ENCRYPTION_KEY",
            "12345678901234567890123456789012",
        );

        let mut cfg = Config::default();
        apply_env_overrides(&mut cfg);

        assert_eq!(cfg.server.host, "127.0.0.1");
        assert_eq!(cfg.server.port, 18080);
        assert_eq!(cfg.database.host, "db.internal");
        assert_eq!(cfg.database.port, 15432);
        assert_eq!(cfg.database.username, "dbuser");
        assert_eq!(cfg.database.password, "dbpass");
        assert_eq!(cfg.database.database, "vail_prod");
        assert_eq!(cfg.jwt.algorithm, jsonwebtoken::Algorithm::EdDSA);
        assert_eq!(cfg.jwt.expiration, 7200);
        assert_eq!(cfg.jwt.refresh_expiration, 1_209_600);
        assert_eq!(cfg.jwt.private_key, TEST_ED25519_PRIVATE_KEY);
        assert_eq!(cfg.jwt.public_key, TEST_ED25519_PUBLIC_KEY);
        assert_eq!(
            cfg.secrets.data_encryption_key,
            "12345678901234567890123456789012"
        );

        for (k, v) in previous {
            if let Some(value) = v {
                std::env::set_var(&k, value);
            } else {
                std::env::remove_var(&k);
            }
        }
    }

    #[test]
    fn load_config_does_not_bridge_eddsa_public_key_into_secret() {
        let _guard = env_test_lock().lock().expect("env lock");
        let prev_config = std::env::var("VAIL_CONFIG").ok();

        let temp_file = std::env::temp_dir().join(format!(
            "vail-config-test-{}-{}.toml",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));

        let content = format!(
            "[jwt]\nalgorithm = \"EdDSA\"\nsecret = \"\"\nprivate_key = \"{TEST_ED25519_PRIVATE_KEY}\"\npublic_key = \"{TEST_ED25519_PUBLIC_KEY}\"\nexpiration = 3600\nrefresh_expiration = 604800\n\n[secrets]\ndata_encryption_key = \"12345678901234567890123456789012\"\n"
        );
        std::fs::write(&temp_file, content).expect("write temp config");

        std::env::set_var(
            "VAIL_CONFIG",
            temp_file.to_str().expect("temp file path utf8"),
        );

        let cfg = load_config();
        assert!(
            cfg.jwt.secret.is_empty(),
            "EdDSA mode must not copy public_key into jwt.secret"
        );

        if let Some(v) = prev_config {
            std::env::set_var("VAIL_CONFIG", v);
        } else {
            std::env::remove_var("VAIL_CONFIG");
        }
        let _ = std::fs::remove_file(&temp_file);
    }

    #[test]
    fn config_validation_rejects_pem_for_eddsa_keys() {
        let mut cfg = Config::default();
        cfg.jwt.algorithm = jsonwebtoken::Algorithm::EdDSA;
        cfg.jwt.private_key =
            "-----BEGIN PRIVATE KEY-----\nabc\n-----END PRIVATE KEY-----".to_string();
        cfg.jwt.public_key =
            "-----BEGIN PUBLIC KEY-----\ndef\n-----END PUBLIC KEY-----".to_string();
        cfg.secrets.data_encryption_key = "12345678901234567890123456789012".to_string();

        let err = cfg.validate().expect_err("PEM must be rejected for EdDSA");
        assert!(err.contains("base64"));
    }
}

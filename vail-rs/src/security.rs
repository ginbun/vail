use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm,
};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use rand::RngCore;
use sha2::{Digest, Sha256};

use crate::error::AppError;

pub fn encrypt_secret(plaintext: &str, key_material: &str) -> Result<String, AppError> {
    let key_material = key_material.trim();
    if key_material.len() < 32 {
        return Err(AppError::Internal(
            "secrets.data_encryption_key must be set and at least 32 characters".to_string(),
        ));
    }

    let key_hash = Sha256::digest(key_material.as_bytes());
    let cipher = Aes256Gcm::new_from_slice(&key_hash)
        .map_err(|_| AppError::Internal("failed to initialize encryption cipher".to_string()))?;

    let mut nonce_bytes = [0u8; 12];
    rand::rngs::OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = aes_gcm::Nonce::from_slice(&nonce_bytes);

    let encrypted = cipher
        .encrypt(nonce, plaintext.as_bytes())
        .map_err(|_| AppError::Internal("failed to encrypt secret payload".to_string()))?;

    Ok(format!(
        "v1:{}:{}",
        STANDARD.encode(nonce_bytes),
        STANDARD.encode(encrypted)
    ))
}

pub fn decrypt_secret(ciphertext: &str, key_material: &str) -> Result<String, AppError> {
    let key_material = key_material.trim();
    if key_material.len() < 32 {
        return Err(AppError::Internal(
            "secrets.data_encryption_key must be set and at least 32 characters".to_string(),
        ));
    }

    let parts: Vec<&str> = ciphertext.split(':').collect();
    if parts.len() != 3 || parts[0] != "v1" {
        return Err(AppError::Internal(
            "unsupported encrypted secret format".to_string(),
        ));
    }

    let nonce = STANDARD
        .decode(parts[1])
        .map_err(|_| AppError::Internal("invalid encrypted secret nonce".to_string()))?;
    let encrypted = STANDARD
        .decode(parts[2])
        .map_err(|_| AppError::Internal("invalid encrypted secret payload".to_string()))?;

    if nonce.len() != 12 {
        return Err(AppError::Internal(
            "invalid encrypted secret nonce length".to_string(),
        ));
    }

    let key_hash = Sha256::digest(key_material.as_bytes());
    let cipher = Aes256Gcm::new_from_slice(&key_hash)
        .map_err(|_| AppError::Internal("failed to initialize encryption cipher".to_string()))?;
    let decrypted = cipher
        .decrypt(aes_gcm::Nonce::from_slice(&nonce), encrypted.as_ref())
        .map_err(|_| AppError::Internal("failed to decrypt secret payload".to_string()))?;

    String::from_utf8(decrypted)
        .map_err(|_| AppError::Internal("decrypted secret is not valid utf-8".to_string()))
}

#[cfg(test)]
mod tests {
    use super::{decrypt_secret, encrypt_secret};

    #[test]
    fn encrypt_secret_returns_versioned_ciphertext() {
        let out = encrypt_secret("top-secret", "12345678901234567890123456789012")
            .expect("encrypt success");

        assert!(out.starts_with("v1:"));
        assert_ne!(out, "top-secret");
    }

    #[test]
    fn encrypt_secret_rejects_short_key() {
        let err = encrypt_secret("top-secret", "short-key").expect_err("must fail");
        assert!(err
            .to_string()
            .contains("secrets.data_encryption_key must be set"));
    }

    #[test]
    fn decrypt_secret_roundtrip() {
        let key = "12345678901234567890123456789012";
        let cipher = encrypt_secret("hello", key).expect("encrypt success");
        let plain = decrypt_secret(&cipher, key).expect("decrypt success");
        assert_eq!(plain, "hello");
    }
}

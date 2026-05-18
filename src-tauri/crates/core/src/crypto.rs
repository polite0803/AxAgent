use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead, KeyInit, OsRng, rand_core::RngCore},
};
use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use sha2::{Digest, Sha256};

use crate::error::{AxAgentError, Result};

const NONCE_SIZE: usize = 12;

pub fn generate_master_key() -> [u8; 32] {
    let mut key = [0u8; 32];
    OsRng.fill_bytes(&mut key);
    key
}

pub fn encrypt_key(plaintext: &str, master_key: &[u8; 32]) -> Result<String> {
    let cipher = Aes256Gcm::new_from_slice(master_key)
        .map_err(|e| AxAgentError::Crypto(format!("Failed to create cipher: {}", e)))?;

    let mut nonce_bytes = [0u8; NONCE_SIZE];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, plaintext.as_bytes())
        .map_err(|e| AxAgentError::Crypto(format!("Encryption failed: {}", e)))?;

    let mut combined = Vec::with_capacity(NONCE_SIZE + ciphertext.len());
    combined.extend_from_slice(&nonce_bytes);
    combined.extend_from_slice(&ciphertext);

    Ok(BASE64.encode(&combined))
}

pub fn decrypt_key(encrypted: &str, master_key: &[u8; 32]) -> Result<String> {
    let combined = BASE64
        .decode(encrypted)
        .map_err(|e| AxAgentError::Crypto(format!("Base64 decode failed: {}", e)))?;

    if combined.len() < NONCE_SIZE {
        return Err(AxAgentError::Crypto("Invalid encrypted data".to_string()));
    }

    let (nonce_bytes, ciphertext) = combined.split_at(NONCE_SIZE);
    let nonce = Nonce::from_slice(nonce_bytes);

    let cipher = Aes256Gcm::new_from_slice(master_key)
        .map_err(|e| AxAgentError::Crypto(format!("Failed to create cipher: {}", e)))?;

    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| AxAgentError::Crypto(format!("Decryption failed: {}", e)))?;

    String::from_utf8(plaintext)
        .map_err(|e| AxAgentError::Crypto(format!("UTF-8 decode failed: {}", e)))
}

pub fn sha256_hash(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    format!("{:x}", hasher.finalize())
}

pub fn key_prefix(_key: &str) -> String {
    "****".to_string()
}

const BACKUP_KEY_DERIVATION_SALT: &[u8] = b"axagent-backup-key-derivation-v1";

pub fn encrypt_backup_key(key_data: &[u8]) -> Result<Vec<u8>> {
    let mut derived_key = [0u8; 32];
    let mut hasher = Sha256::new();
    hasher.update(BACKUP_KEY_DERIVATION_SALT);
    hasher.update(b"axagent-backup-encryption");
    derived_key.copy_from_slice(&hasher.finalize());

    let cipher = Aes256Gcm::new_from_slice(&derived_key)
        .map_err(|e| AxAgentError::Crypto(format!("Failed to create backup cipher: {}", e)))?;

    let mut nonce_bytes = [0u8; NONCE_SIZE];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, key_data)
        .map_err(|e| AxAgentError::Crypto(format!("Backup key encryption failed: {}", e)))?;

    let mut combined = Vec::with_capacity(NONCE_SIZE + ciphertext.len());
    combined.extend_from_slice(&nonce_bytes);
    combined.extend_from_slice(&ciphertext);
    Ok(combined)
}

pub fn decrypt_backup_key(enc_data: &[u8]) -> Result<Vec<u8>> {
    if enc_data.len() < NONCE_SIZE + 16 {
        return Err(AxAgentError::Crypto("Invalid encrypted backup key data".to_string()));
    }

    let (nonce_bytes, ciphertext) = enc_data.split_at(NONCE_SIZE);
    let nonce = Nonce::from_slice(nonce_bytes);

    // Try decryption with derived key approach
    // Since we derive the key from the original key data, we need a different approach
    // Use a fixed derivation from a known constant for backup key protection
    let mut derived_key = [0u8; 32];
    let mut hasher = Sha256::new();
    hasher.update(BACKUP_KEY_DERIVATION_SALT);
    hasher.update(b"axagent-backup-encryption");
    derived_key.copy_from_slice(&hasher.finalize());

    let cipher = Aes256Gcm::new_from_slice(&derived_key)
        .map_err(|e| AxAgentError::Crypto(format!("Failed to create backup cipher: {}", e)))?;

    cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| AxAgentError::Crypto(format!("Backup key decryption failed: {}", e)))
}

pub fn generate_gateway_key() -> String {
    let mut bytes = [0u8; 32];
    OsRng.fill_bytes(&mut bytes);
    format!("aq-{}", hex::encode(bytes))
}

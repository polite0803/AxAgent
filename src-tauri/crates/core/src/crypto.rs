use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead, KeyInit, OsRng, rand_core::RngCore},
};
use argon2::{Algorithm, Argon2, Params, Version};
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

const BACKUP_VERSION_BYTE: u8 = 0x02;
const BACKUP_SALT_SIZE: usize = 16;
const ARGON2_MEMORY_COST: u32 = 65536; // 64 MB
const ARGON2_TIME_COST: u32 = 3;
const ARGON2_PARALLELISM: u32 = 4;

/// 使用 Argon2id 从机器特征 + 内置常量派生备份加密密钥。
/// 结合机器唯一标识使密钥与当前设备绑定，即使源码泄露也无法在其他机器上解密。
fn derive_backup_key_v2(salt: &[u8]) -> Result<[u8; 32]> {
    let mut key = [0u8; 32];
    let params = Params::new(ARGON2_MEMORY_COST, ARGON2_TIME_COST, ARGON2_PARALLELISM, Some(32))
        .map_err(|e| AxAgentError::Crypto(format!("Argon2 参数无效: {e}")))?;
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let machine_id = get_machine_fingerprint();
    let mut password = Vec::with_capacity(
        b"axagent-backup-key-v2:".len() + machine_id.len() + b":axagent-backup-encryption-v2".len(),
    );
    password.extend_from_slice(b"axagent-backup-key-v2:");
    password.extend_from_slice(machine_id.as_bytes());
    password.extend_from_slice(b":axagent-backup-encryption-v2");
    argon2
        .hash_password_into(&password, salt, &mut key)
        .map_err(|e| AxAgentError::Crypto(format!("Argon2 密钥派生失败: {e}")))?;
    Ok(key)
}

fn get_machine_fingerprint() -> String {
    let hostname = std::env::var("HOSTNAME")
        .or_else(|_| std::env::var("COMPUTERNAME"))
        .or_else(|_| std::env::var("NAME"))
        .unwrap_or_else(|_| "unknown".to_string());
    let username = std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_else(|_| "unknown".to_string());
    let os_info = format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH);
    let raw = format!("{}:{}:{}", hostname, username, os_info);
    sha256_hash(&raw)
}

pub fn encrypt_backup_key(key_data: &[u8]) -> Result<Vec<u8>> {
    let mut salt = [0u8; BACKUP_SALT_SIZE];
    OsRng.fill_bytes(&mut salt);
    let derived_key = derive_backup_key_v2(&salt)?;

    let cipher = Aes256Gcm::new_from_slice(&derived_key)
        .map_err(|e| AxAgentError::Crypto(format!("Failed to create backup cipher: {}", e)))?;

    let mut nonce_bytes = [0u8; NONCE_SIZE];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, key_data)
        .map_err(|e| AxAgentError::Crypto(format!("Backup key encryption failed: {}", e)))?;

    // Format: version_byte(1) + salt(16) + nonce(12) + ciphertext
    let mut combined = Vec::with_capacity(1 + BACKUP_SALT_SIZE + NONCE_SIZE + ciphertext.len());
    combined.push(BACKUP_VERSION_BYTE);
    combined.extend_from_slice(&salt);
    combined.extend_from_slice(&nonce_bytes);
    combined.extend_from_slice(&ciphertext);
    Ok(combined)
}

pub fn decrypt_backup_key(enc_data: &[u8]) -> Result<Vec<u8>> {
    if enc_data.len() < 1 + NONCE_SIZE + 16 {
        return Err(AxAgentError::Crypto("Invalid encrypted backup key data".to_string()));
    }

    // v2 format: version_byte(0x02) + salt(16) + nonce(12) + ciphertext
    if enc_data[0] == BACKUP_VERSION_BYTE {
        let min_len = 1 + BACKUP_SALT_SIZE + NONCE_SIZE + 16;
        if enc_data.len() < min_len {
            return Err(AxAgentError::Crypto("Truncated v2 backup key data".to_string()));
        }
        let salt = &enc_data[1..1 + BACKUP_SALT_SIZE];
        let nonce_bytes = &enc_data[1 + BACKUP_SALT_SIZE..1 + BACKUP_SALT_SIZE + NONCE_SIZE];
        let ciphertext = &enc_data[1 + BACKUP_SALT_SIZE + NONCE_SIZE..];

        let derived_key = derive_backup_key_v2(salt)?;
        let cipher = Aes256Gcm::new_from_slice(&derived_key)
            .map_err(|e| AxAgentError::Crypto(format!("Failed to create backup cipher: {}", e)))?;
        let nonce = Nonce::from_slice(nonce_bytes);

        return cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| AxAgentError::Crypto(format!("Backup key decryption failed: {}", e)));
    }

    // Legacy v1 format: nonce(12) + ciphertext (SHA256-based KDF)
    decrypt_backup_key_v1(enc_data)
}

/// Legacy decrypt for v1 backups (SHA256 KDF, fixed salt).
/// ⚠️ 已弃用：v1 使用弱 KDF（无盐 SHA256），将在未来版本中移除。
/// 请尽快迁移到 v2 格式（Argon2id + 随机盐 + 机器指纹）。
fn decrypt_backup_key_v1(enc_data: &[u8]) -> Result<Vec<u8>> {
    tracing::warn!("正在使用已弃用的 v1 备份密钥解密（弱 KDF），请尽快重新加密为 v2 格式");
    let (nonce_bytes, ciphertext) = enc_data.split_at(NONCE_SIZE);
    let nonce = Nonce::from_slice(nonce_bytes);

    let mut derived_key = [0u8; 32];
    let mut hasher = Sha256::new();
    hasher.update(b"axagent-backup-key-derivation-v1");
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

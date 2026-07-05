#![allow(clippy::result_large_err)]
// SPDX-License-Identifier: AGPL-3.0-only

use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead, KeyInit, OsRng, rand_core::RngCore},
};
use argon2::{Algorithm, Argon2, Params, Version};
use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use sha2::{Digest, Sha256};
use zeroize::{Zeroize, Zeroizing};

use axagent_harness::core_error::{AxAgentError, Result};

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

    let result = Ok(BASE64.encode(&combined));
    combined.zeroize();
    result
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

    let mut plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| AxAgentError::Crypto(format!("Decryption failed: {}", e)))?;

    // Convert to String and zeroize the intermediate buffer
    let result = String::from_utf8(plaintext.clone())
        .map_err(|e| AxAgentError::Crypto(format!("UTF-8 decode failed: {}", e)));
    plaintext.zeroize();
    result
}

pub fn sha256_hash(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// SECURITY (H7): 真正从 key 中提取可识别前缀。
/// 取前 2 + 末 2 字符；长度不足时返回全 `*`。
/// 仅用于 UI 展示，不参与任何权限判定。
pub fn key_prefix(key: &str) -> String {
    let chars: Vec<char> = key.chars().collect();
    if chars.len() < 4 {
        return "*".repeat(chars.len());
    }
    let head: String = chars.iter().take(2).collect();
    let tail: String = chars
        .iter()
        .rev()
        .take(2)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    format!("{head}…{tail}")
}

/// SECURITY (C10): HMAC-SHA256 使用标准 `hmac` + `sha2` crate，替代自实现。
/// 标准实现经过广泛审计，无时序攻击风险。
pub fn hmac_sha256(key: &[u8], msg: &str) -> String {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;

    let mut mac =
        <Hmac<Sha256> as Mac>::new_from_slice(key).expect("HMAC-SHA256 accepts any key length");
    mac.update(msg.as_bytes());
    let result = mac.finalize();
    format!("{:x}", result.into_bytes())
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
    password.zeroize();
    Ok(key)
}

fn get_machine_fingerprint() -> String {
    read_or_create_machine_id().unwrap_or_else(|| {
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
    })
}

fn read_or_create_machine_id() -> Option<String> {
    let dir = if cfg!(target_os = "windows") {
        std::env::var("LOCALAPPDATA").ok()?
    } else {
        let home = std::env::var("HOME").ok()?;
        format!("{}/.config", home)
    };
    let dir = std::path::PathBuf::from(dir).join("axagent");
    let file_path = dir.join("machine-id");

    if let Ok(content) = std::fs::read_to_string(&file_path) {
        let id = content.trim().to_string();
        if id.len() == 64 {
            return Some(id);
        }
    }

    std::fs::create_dir_all(&dir).ok()?;
    let mut id = [0u8; 32];
    OsRng.fill_bytes(&mut id);
    let hex_id = hex::encode(id);
    std::fs::write(&file_path, &hex_id).ok()?;
    Some(hex_id)
}

pub fn encrypt_backup_key(key_data: &[u8]) -> Result<Vec<u8>> {
    let mut salt = [0u8; BACKUP_SALT_SIZE];
    OsRng.fill_bytes(&mut salt);
    let mut derived_key = derive_backup_key_v2(&salt)?;

    let cipher = Aes256Gcm::new_from_slice(&derived_key)
        .map_err(|e| AxAgentError::Crypto(format!("Failed to create backup cipher: {}", e)))?;

    let mut nonce_bytes = [0u8; NONCE_SIZE];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, key_data)
        .map_err(|e| AxAgentError::Crypto(format!("Backup key encryption failed: {}", e)))?;

    // Zeroize derived key after use
    derived_key.zeroize();

    // Format: version_byte(1) + salt(16) + nonce(12) + ciphertext
    let mut combined = Vec::with_capacity(1 + BACKUP_SALT_SIZE + NONCE_SIZE + ciphertext.len());
    combined.push(BACKUP_VERSION_BYTE);
    combined.extend_from_slice(&salt);
    combined.extend_from_slice(&nonce_bytes);
    combined.extend_from_slice(&ciphertext);
    let result = Ok(combined);
    // combined moved to result on success — no explicit zeroize needed
    result
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

        let mut derived_key = derive_backup_key_v2(salt)?;
        let cipher = Aes256Gcm::new_from_slice(&derived_key)
            .map_err(|e| AxAgentError::Crypto(format!("Failed to create backup cipher: {}", e)))?;
        let nonce = Nonce::from_slice(nonce_bytes);

        let result = cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| AxAgentError::Crypto(format!("Backup key decryption failed: {}", e)));
        derived_key.zeroize();
        return result;
    }

    // Legacy v1 format: nonce(12) + ciphertext (SHA256-based KDF)
    // Only available with "backup_v1_compat" feature gate.
    #[cfg(feature = "backup_v1_compat")]
    return decrypt_backup_key_v1(enc_data);

    #[cfg(not(feature = "backup_v1_compat"))]
    Err(AxAgentError::Crypto(
        "v1 backup format detected but feature 'backup_v1_compat' is not enabled. \
         Use auto_upgrade_backup_to_v2() to migrate this backup to v2 (Argon2id) format."
            .to_string(),
    ))
}

/// Legacy decrypt for v1 backups (SHA256 KDF, fixed salt).
/// ⚠️ **已弃用**: v1 使用弱 KDF（无盐 SHA256），存在已知安全缺陷：
///   - 无盐哈希：同一密码总是生成相同密钥，易受彩虹表攻击
///   - 固定字符串 KDF：密钥空间小，暴力破解成本低
///   - 无密钥拉伸：单次 SHA256 极快，无法抵抗 GPU 暴力搜索
///
/// **迁移计划**: 2026-Q3 移除 v1 支持，启动时自动检测并升级 v1 备份到 v2 (Argon2id)。
/// 请尽快重新加密为 v2 格式。
#[cfg(feature = "backup_v1_compat")]
fn decrypt_backup_key_v1(enc_data: &[u8]) -> Result<Vec<u8>> {
    tracing::warn!(
        "SECURITY: 正在使用已弃用的 v1 备份密钥解密（弱 KDF: 无盐 SHA256）。\
         请尽快通过「重新加密备份」按钮升级到 v2 格式（Argon2id + 随机盐 + 机器指纹）。\
         v1 格式将在 2026-Q3 移除。"
    );
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

/// 自动将 v1 格式的备份密钥升级到 v2 (Argon2id)。
///
/// 如果输入已经是 v2 格式，直接返回输入（无操作）。
/// 如果是 v1 格式，解密后用 Argon2id 重新加密并返回 v2 格式数据。
/// 调用方应将返回的 v2 数据写回原始文件路径以完成迁移。
///
/// 此函数**不需要** `backup_v1_compat` feature gate，
/// 专为一次性迁移场景设计。
pub fn auto_upgrade_backup_to_v2(enc_data: &[u8]) -> Result<Vec<u8>> {
    if enc_data.len() < 1 + NONCE_SIZE + 16 {
        return Err(AxAgentError::Crypto(
            "Invalid encrypted backup key data for upgrade".to_string(),
        ));
    }

    // Already v2: no upgrade needed
    if enc_data[0] == BACKUP_VERSION_BYTE {
        return Ok(enc_data.to_vec());
    }

    // v1 format detected — decrypt with legacy KDF, then re-encrypt with v2
    tracing::info!("检测到 v1 格式备份密钥，正在自动升级到 v2 (Argon2id)...");

    let (nonce_bytes, ciphertext) = enc_data.split_at(NONCE_SIZE);
    let nonce = Nonce::from_slice(nonce_bytes);

    let mut derived_key = [0u8; 32];
    let mut hasher = Sha256::new();
    hasher.update(b"axagent-backup-key-derivation-v1");
    hasher.update(b"axagent-backup-encryption");
    derived_key.copy_from_slice(&hasher.finalize());

    let cipher = Aes256Gcm::new_from_slice(&derived_key)
        .map_err(|e| AxAgentError::Crypto(format!("v1 decrypt cipher init failed: {e}")))?;

    let key_data = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| AxAgentError::Crypto(format!("v1 backup key decryption failed: {e}")))?;

    // Re-encrypt with v2 (Argon2id)
    let upgraded = encrypt_backup_key(&key_data)?;

    derived_key.zeroize();
    // key_data is returned as Vec<u8> — caller should zeroize after use

    tracing::info!("备份密钥已成功升级到 v2 格式（{} → {} bytes）", enc_data.len(), upgraded.len());
    Ok(upgraded)
}

pub fn generate_gateway_key() -> String {
    let mut bytes = [0u8; 32];
    OsRng.fill_bytes(&mut bytes);
    format!("aq-{}", hex::encode(bytes))
}

/// 从机器指纹派生前端 secure storage 的 AES-256 主密钥。
/// 密钥与机器绑定，用于 encrypt_key/decrypt_key 保护 localStorage 中的敏感数据。
pub fn derive_storage_master_key() -> [u8; 32] {
    let fingerprint = get_machine_fingerprint();
    let seed = format!("axagent-storage-key-v2:{}:storage-encryption", fingerprint);
    let hash = sha256_hash(&seed);
    let mut key = [0u8; 32];
    let decoded = hex::decode(&hash).unwrap_or_else(|_| vec![0u8; 32]);
    let len = decoded.len().min(32);
    key[..len].copy_from_slice(&decoded[..len]);
    key
}

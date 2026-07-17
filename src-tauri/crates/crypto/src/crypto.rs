#![allow(clippy::result_large_err)]
// SPDX-License-Identifier: AGPL-3.0-only

use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead, KeyInit},
};
use argon2::{Algorithm, Argon2, Params, Version};
use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use sha2::{Digest, Sha256};
use zeroize::Zeroize;

use axagent_harness::core_error::{AxAgentError, Result};

const NONCE_SIZE: usize = 12;

pub fn generate_master_key() -> [u8; 32] {
    let mut key = [0u8; 32];
    getrandom::getrandom(&mut key[..]).expect("生成随机数失败");
    key
}

pub fn encrypt_key(plaintext: &str, master_key: &[u8; 32]) -> Result<String> {
    let cipher = Aes256Gcm::new_from_slice(master_key)
        .map_err(|e| AxAgentError::Crypto(format!("Failed to create cipher: {}", e)))?;

    let mut nonce_bytes = [0u8; NONCE_SIZE];
    getrandom::getrandom(&mut nonce_bytes[..]).expect("生成 nonce 失败");
    let nonce = Nonce::try_from(&nonce_bytes[..]).expect("nonce 必须为 12 字节");

    let ciphertext = cipher
        .encrypt(&nonce, plaintext.as_bytes())
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
    let nonce = Nonce::try_from(nonce_bytes).expect("nonce 必须为 12 字节");

    let cipher = Aes256Gcm::new_from_slice(master_key)
        .map_err(|e| AxAgentError::Crypto(format!("Failed to create cipher: {}", e)))?;

    let mut plaintext = cipher
        .decrypt(&nonce, ciphertext)
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
    // m1: 使用固定 64 字符 hex 输出，不省略前导零。
    // digest 0.11 的 finalize() 返回 generic-array 的 Array，未实现 LowerHex，
    // 故转字节切片后用 hex 编码。
    hex::encode(hasher.finalize().as_slice())
}

/// 解密 secure storage 中的值，自动兼容 v1（SHA256）和 v2（Argon2id）密钥派生。
/// 先用 v2 密钥尝试，失败则回退到 v1 密钥（用于解密升级前存储的旧数据）。
pub fn decrypt_storage_key(encrypted: &str) -> Result<String> {
    let v2_key = derive_storage_master_key_v2()?;
    match decrypt_key(encrypted, &v2_key) {
        Ok(plaintext) => Ok(plaintext),
        Err(_) => {
            // v2 解密失败，尝试 v1 密钥（旧数据）
            let v1_key = derive_storage_master_key_v1();
            decrypt_key(encrypted, &v1_key)
        },
    }
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
    let tail: String = chars.iter().rev().take(2).collect::<Vec<_>>().into_iter().rev().collect();
    format!("{head}…{tail}")
}

/// SECURITY (C10): HMAC-SHA256 使用标准 `hmac` + `sha2` crate，替代自实现。
/// 标准实现经过广泛审计，无时序攻击风险。
pub fn hmac_sha256(key: &[u8], msg: &str) -> String {
    use hmac::{Hmac, KeyInit, Mac};
    use sha2::Sha256;

    let mut mac =
        <Hmac<Sha256> as KeyInit>::new_from_slice(key).expect("HMAC-SHA256 accepts any key length");
    mac.update(msg.as_bytes());
    let result = mac.finalize();
    hex::encode(result.into_bytes().as_slice())
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
    getrandom::getrandom(&mut id[..]).expect("生成随机数失败");
    let hex_id = hex::encode(id);
    std::fs::write(&file_path, &hex_id).ok()?;
    Some(hex_id)
}

pub fn encrypt_backup_key(key_data: &[u8]) -> Result<Vec<u8>> {
    let mut salt = [0u8; BACKUP_SALT_SIZE];
    getrandom::getrandom(&mut salt[..]).expect("生成随机数失败");
    let mut derived_key = derive_backup_key_v2(&salt)?;

    let cipher = Aes256Gcm::new_from_slice(&derived_key)
        .map_err(|e| AxAgentError::Crypto(format!("Failed to create backup cipher: {}", e)))?;

    let mut nonce_bytes = [0u8; NONCE_SIZE];
    getrandom::getrandom(&mut nonce_bytes[..]).expect("生成 nonce 失败");
    let nonce = Nonce::try_from(&nonce_bytes[..]).expect("nonce 必须为 12 字节");

    let ciphertext = cipher
        .encrypt(&nonce, key_data)
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
        let nonce = Nonce::try_from(nonce_bytes).expect("nonce 必须为 12 字节");

        let result = cipher
            .decrypt(&nonce, ciphertext)
            .map_err(|e| AxAgentError::Crypto(format!("Backup key decryption failed: {}", e)));
        derived_key.zeroize();
        return result;
    }

    // Legacy v1 format: nonce(12) + ciphertext (SHA256-based KDF)
    // Only available with "backup_v1_compat" feature gate.
    // 此处调用 deprecated 的 v1 解密函数是「向后兼容读取」的预期行为，
    // 辅助函数显式 allow(deprecated) 以避免 -D warnings 在 CI 中阻断构建。
    #[cfg(feature = "backup_v1_compat")]
    return legacy_decrypt_backup_v1(enc_data);

    #[cfg(not(feature = "backup_v1_compat"))]
    Err(AxAgentError::Crypto(
        "v1 backup format detected but feature 'backup_v1_compat' is not enabled. \
         Use auto_upgrade_backup_to_v2() to migrate this backup to v2 (Argon2id) format."
            .to_string(),
    ))
}

/// v1 备份密钥解密的 thin wrapper。
///
/// `decrypt_backup_key_v1` 已被 `#[deprecated]` 标记（v1 KDF 不安全），
/// 但 `backup_v1_compat` feature 的存在目的就是允许读取遗留 v1 数据。
/// 直接调用会触发 `-D deprecated`，因此用此 wrapper 显式 `allow(deprecated)`，
/// 把弃用警告隔离在单一调用点上，便于将来移除 v1 支持时一并清理。
#[cfg(feature = "backup_v1_compat")]
#[allow(deprecated)]
fn legacy_decrypt_backup_v1(enc_data: &[u8]) -> Result<Vec<u8>> {
    decrypt_backup_key_v1(enc_data)
}

/// Legacy decrypt for v1 backups (SHA256 KDF, fixed salt).
///
/// ⚠️ **已弃用**: v1 使用弱 KDF（无盐 SHA256），存在已知安全缺陷：
///   - 无盐哈希：同一密码总是生成相同密钥，易受彩虹表攻击
///   - 固定字符串 KDF：密钥空间小，暴力破解成本低
///   - 无密钥拉伸：单次 SHA256 极快，无法抵抗 GPU 暴力搜索
///
/// **迁移计划**: 2026-Q3 移除 v1 支持，启动时自动检测并升级 v1 备份到 v2 (Argon2id)。
/// 请尽快重新加密为 v2 格式。
///
/// **此函数仅为读取历史遗留 v1 备份数据而保留，不应用于新备份。**
/// 读取后应通过 [`auto_upgrade_backup_to_v2`] 立即迁移到 v2。
#[deprecated(
    since = "2.8.2",
    note = "v1 backup KDF (unsalted SHA256) is insecure. This function exists only for \
            reading legacy v1 backups; new backups MUST use v2 (Argon2id). \
            Call auto_upgrade_backup_to_v2() to migrate legacy data immediately."
)]
#[cfg(feature = "backup_v1_compat")]
fn decrypt_backup_key_v1(enc_data: &[u8]) -> Result<Vec<u8>> {
    tracing::warn!(
        "SECURITY: 正在使用已弃用的 v1 备份密钥解密（弱 KDF: 无盐 SHA256）。\
         请尽快通过「重新加密备份」按钮升级到 v2 格式（Argon2id + 随机盐 + 机器指纹）。\
         v1 格式将在 2026-Q3 移除。"
    );
    let (nonce_bytes, ciphertext) = enc_data.split_at(NONCE_SIZE);
    let nonce = Nonce::try_from(nonce_bytes).expect("nonce 必须为 12 字节");

    let mut derived_key = [0u8; 32];
    let mut hasher = Sha256::new();
    hasher.update(b"axagent-backup-key-derivation-v1");
    hasher.update(b"axagent-backup-encryption");
    derived_key.copy_from_slice(hasher.finalize().as_slice());

    let cipher = Aes256Gcm::new_from_slice(&derived_key)
        .map_err(|e| AxAgentError::Crypto(format!("Failed to create backup cipher: {}", e)))?;

    cipher
        .decrypt(&nonce, ciphertext)
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
    let nonce = Nonce::try_from(nonce_bytes).expect("nonce 必须为 12 字节");

    let mut derived_key = [0u8; 32];
    let mut hasher = Sha256::new();
    hasher.update(b"axagent-backup-key-derivation-v1");
    hasher.update(b"axagent-backup-encryption");
    derived_key.copy_from_slice(hasher.finalize().as_slice());

    let cipher = Aes256Gcm::new_from_slice(&derived_key)
        .map_err(|e| AxAgentError::Crypto(format!("v1 decrypt cipher init failed: {e}")))?;

    let key_data = cipher
        .decrypt(&nonce, ciphertext)
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
    getrandom::getrandom(&mut bytes[..]).expect("生成随机数失败");
    format!("aq-{}", hex::encode(bytes))
}

/// 从机器指纹派生前端 secure storage 的 AES-256 主密钥（v2，Argon2id）。
/// 密钥与机器绑定，用于 encrypt_key/decrypt_key 保护 localStorage 中的敏感数据。
///
/// SECURITY (C3): v2 使用 Argon2id（64MB/3/4）替代单次 SHA256，
/// 提供内存硬度以抵抗离线暴力破解。salt 从机器指纹确定性派生，
/// 保持"同机同密钥"语义。v1 保留用于解密旧数据。
pub fn derive_storage_master_key() -> [u8; 32] {
    derive_storage_master_key_v2().unwrap_or_else(|_| derive_storage_master_key_v1())
}

/// v2：Argon2id 派生，提供内存硬度。
fn derive_storage_master_key_v2() -> Result<[u8; 32]> {
    let fingerprint = get_machine_fingerprint();
    // 从指纹确定性派生 16 字节 salt（同机不变）
    let mut salt = [0u8; 16];
    let salt_seed = format!("axagent-storage-salt:{fingerprint}");
    let salt_hash = sha2::Sha256::digest(salt_seed.as_bytes());
    let salt_slice: &[u8] = salt_hash.as_ref();
    salt.copy_from_slice(&salt_slice[..16]);

    let mut key = [0u8; 32];
    let params = Params::new(ARGON2_MEMORY_COST, ARGON2_TIME_COST, ARGON2_PARALLELISM, Some(32))
        .map_err(|e| AxAgentError::Crypto(format!("Argon2 参数无效: {e}")))?;
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut password = Vec::with_capacity(
        b"axagent-storage-key-v2:".len() + fingerprint.len() + b":storage-encryption".len(),
    );
    password.extend_from_slice(b"axagent-storage-key-v2:");
    password.extend_from_slice(fingerprint.as_bytes());
    password.extend_from_slice(b":storage-encryption");
    let res = argon2
        .hash_password_into(&password, &salt, &mut key)
        .map_err(|e| AxAgentError::Crypto(format!("Argon2 存储密钥派生失败: {e}")));
    password.zeroize();
    res?;
    Ok(key)
}

/// v1：单次 SHA256 派生（已弃用，仅用于解密旧数据）。
fn derive_storage_master_key_v1() -> [u8; 32] {
    let fingerprint = get_machine_fingerprint();
    let seed = format!("axagent-storage-key-v2:{fingerprint}:storage-encryption");
    let hash = sha256_hash(&seed);
    let mut key = [0u8; 32];
    let decoded = hex::decode(&hash).unwrap_or_else(|_| vec![0u8; 32]);
    let len = decoded.len().min(32);
    key[..len].copy_from_slice(&decoded[..len]);
    key
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── encrypt_key / decrypt_key 往返 ──

    #[test]
    fn encrypt_decrypt_roundtrip_ascii() {
        let key = generate_master_key();
        let enc = encrypt_key("hello world", &key).expect("encrypt 应成功");
        let dec = decrypt_key(&enc, &key).expect("decrypt 应成功");
        assert_eq!(dec, "hello world");
    }

    #[test]
    fn encrypt_decrypt_roundtrip_unicode() {
        let key = generate_master_key();
        let plaintext = "密钥🔐包含中文与emoji";
        let enc = encrypt_key(plaintext, &key).expect("encrypt 应成功");
        let dec = decrypt_key(&enc, &key).expect("decrypt 应成功");
        assert_eq!(dec, plaintext);
    }

    #[test]
    fn encrypt_decrypt_roundtrip_empty() {
        let key = generate_master_key();
        let enc = encrypt_key("", &key).expect("encrypt 空串应成功");
        let dec = decrypt_key(&enc, &key).expect("decrypt 空串应成功");
        assert_eq!(dec, "");
    }

    #[test]
    fn encrypt_is_nondeterministic_per_call() {
        let key = generate_master_key();
        let a = encrypt_key("same", &key).unwrap();
        let b = encrypt_key("same", &key).unwrap();
        // 由于随机 nonce，两次密文不应相同（抗重放/模式分析）。
        assert_ne!(a, b);
    }

    #[test]
    fn decrypt_with_wrong_key_fails() {
        let k1 = generate_master_key();
        let k2 = generate_master_key();
        let enc = encrypt_key("secret", &k1).expect("encrypt 应成功");
        assert!(decrypt_key(&enc, &k2).is_err(), "错误主密钥应解密失败");
    }

    #[test]
    fn decrypt_invalid_base64_fails() {
        let key = generate_master_key();
        assert!(decrypt_key("!!!not-base64!!!", &key).is_err());
    }

    #[test]
    fn decrypt_too_short_data_fails() {
        let key = generate_master_key();
        // 短于 nonce(12) 的 base64 串，解码后不足 12 字节。
        assert!(decrypt_key("AAAA", &key).is_err());
    }

    #[test]
    fn decrypt_tampered_ciphertext_fails() {
        let key = generate_master_key();
        let enc = encrypt_key("tamper-me", &key).unwrap();
        let mut bytes = BASE64.decode(&enc).unwrap();
        // 翻转密文最后一个字节（保留前 12 字节 nonce 不动，避免长度/nonce 错误掩盖篡改）。
        let last = bytes.len() - 1;
        bytes[last] ^= 0xFF;
        let tampered = BASE64.encode(&bytes);
        assert!(decrypt_key(&tampered, &key).is_err(), "篡改密文应解密失败");
    }

    // ── sha256_hash 确定性 ──

    #[test]
    fn sha256_known_vectors() {
        assert_eq!(
            sha256_hash(""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            sha256_hash("abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn sha256_distinct_inputs_distinct_outputs() {
        assert_ne!(sha256_hash("foo"), sha256_hash("bar"));
    }

    // ── hmac_sha256 (RFC 4231 Test Case 6) ──

    #[test]
    fn hmac_sha256_rfc4231_test_case_6() {
        let key = b"Jefe";
        let msg = "what do ya want for nothing?";
        assert_eq!(
            hmac_sha256(key, msg),
            "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843"
        );
    }

    #[test]
    fn hmac_sha256_deterministic_and_msg_dependent() {
        let key = b"shared-key";
        let a = hmac_sha256(key, "message-one");
        let b = hmac_sha256(key, "message-one");
        let c = hmac_sha256(key, "message-two");
        assert_eq!(a, b, "相同输入必须产生相同 HMAC");
        assert_ne!(a, c, "不同消息必须产生不同 HMAC");
    }

    // ── key_prefix（仅 UI 展示，不用于权限判定）──

    #[test]
    fn key_prefix_short_inputs() {
        assert_eq!(key_prefix(""), "");
        assert_eq!(key_prefix("ab"), "**");
    }

    #[test]
    fn key_prefix_normal_inputs() {
        assert_eq!(key_prefix("abcd"), "ab…cd");
        assert_eq!(key_prefix("abcde"), "ab…de");
    }

    // ── generate_gateway_key 格式 ──

    #[test]
    fn generate_gateway_key_format() {
        let k = generate_gateway_key();
        assert!(k.starts_with("aq-"), "应以 aq- 前缀开头");
        assert_eq!(k.len(), 3 + 64, "格式为 aq- + 64 hex 字符");
        assert!(k[3..].chars().all(|c| c.is_ascii_hexdigit()), "后缀须为 hex");
    }

    #[test]
    fn generate_master_key_length() {
        let k = generate_master_key();
        assert_eq!(k.len(), 32);
    }

    // ── 备份密钥加密（v2 / Argon2id）──

    #[test]
    fn backup_key_encrypt_decrypt_roundtrip() {
        let data = b"super-secret-backup-material-1234567890";
        let enc = encrypt_backup_key(data).expect("备份加密应成功");
        assert_eq!(enc[0], BACKUP_VERSION_BYTE, "应为 v2 版本字节 0x02");
        let dec = decrypt_backup_key(&enc).expect("备份解密应成功");
        assert_eq!(dec, data);
    }

    #[test]
    fn decrypt_backup_key_truncated_data_fails() {
        assert!(decrypt_backup_key(&[0u8; 4]).is_err());
    }

    #[test]
    fn auto_upgrade_passthrough_for_v2() {
        let data = b"already-v2-material";
        let enc = encrypt_backup_key(data).unwrap();
        let upgraded = auto_upgrade_backup_to_v2(&enc).expect("升级应成功");
        assert_eq!(upgraded, enc, "v2 数据应原样返回（无操作）");
    }

    #[test]
    fn auto_upgrade_invalid_data_fails() {
        assert!(auto_upgrade_backup_to_v2(&[0u8; 4]).is_err());
    }

    // ── secure storage 主密钥往返 ──
    // 注意：不经 `decrypt_storage_key`，因为它内部会重新派生密钥（依赖机器指纹
    // I/O，且带 v1/v2 回退），与外部 `derive_storage_master_key()` 可能不一致，
    // 使测试非确定。这里验证真正的不变量：派生出的存储主密钥是可用的 AES-256
    // 密钥，用同一密钥对象加解密可无损往返。

    #[test]
    fn storage_master_key_is_usable_aes_key() {
        let master = derive_storage_master_key();
        // 派生的密钥必须是 32 字节且非全零
        assert_eq!(master.len(), 32);
        assert!(master.iter().any(|&b| b != 0), "派生密钥不应全为零");

        // 同一密钥对象加解密往返无损
        let enc = encrypt_key("storage-secret-value", &master).expect("encrypt 应成功");
        let dec = decrypt_key(&enc, &master).expect("decrypt 应成功");
        assert_eq!(dec, "storage-secret-value");
    }
}

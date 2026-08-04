// SPDX-License-Identifier: AGPL-3.0-only

//! 端到端加密同步模块
//!
//! 实现设备间同步数据的加密/解密功能，确保数据在传输和存储时的安全性。
//! 使用 AES-256-GCM 进行加密，PBKDF2 进行密钥派生。

use aes_gcm::{Aes256Gcm, KeyInit, aead::Aead};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use generic_array::typenum::U12;
use pbkdf2::pbkdf2_hmac;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// 同步数据加密版本
const SYNC_ENCRYPTION_VERSION: u8 = 2;
/// AES-GCM Nonce 大小
const NONCE_SIZE: usize = 12;
/// PBKDF2 迭代次数（为安全起见，使用较高的迭代次数）
const PBKDF2_ITERATIONS: u32 = 100_000;
/// 盐值大小
const SALT_SIZE: usize = 32;

/// 同步加密配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncEncryptionConfig {
    /// 是否启用端到端加密
    pub enabled: bool,
    /// 加密算法
    pub algorithm: EncryptionAlgorithm,
    /// 密钥派生方式
    pub key_derivation: KeyDerivation,
}

impl Default for SyncEncryptionConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            algorithm: EncryptionAlgorithm::Aes256Gcm,
            key_derivation: KeyDerivation::X25519,
        }
    }
}

/// 加密算法
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EncryptionAlgorithm {
    Aes256Gcm,
}

/// 密钥派生方式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KeyDerivation {
    /// 使用预共享密钥
    PreSharedKey,
    /// 使用 X25519 密钥交换
    X25519,
}

/// 同步加密数据
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EncryptedSyncData {
    /// 加密版本
    pub version: u8,
    /// 算法标识
    pub algorithm: String,
    /// 加密后的数据
    pub ciphertext: String,
    /// Nonce (Base64 编码)
    pub nonce: String,
    /// 设备 ID（加密者）
    pub source_device_id: String,
    /// 目标设备 ID
    pub target_device_id: Option<String>,
    /// 时间戳
    pub encrypted_at: u64,
}

impl EncryptedSyncData {
    /// 创建新的加密数据
    pub fn new(
        ciphertext: String,
        nonce: String,
        source_device_id: String,
        target_device_id: Option<String>,
    ) -> Self {
        Self {
            version: SYNC_ENCRYPTION_VERSION,
            algorithm: "AES-256-GCM".to_string(),
            ciphertext,
            nonce,
            source_device_id,
            target_device_id,
            encrypted_at: chrono::Utc::now().timestamp_millis() as u64,
        }
    }
}

/// 同步加密器
pub struct SyncEncryptor {
    /// 加密密钥
    key: [u8; 32],
}

impl SyncEncryptor {
    /// 从密码派生密钥（使用 PBKDF2）
    pub fn from_password(password: &str, salt: &[u8]) -> Self {
        let mut key = [0u8; 32];
        pbkdf2_hmac::<Sha256>(password.as_bytes(), salt, PBKDF2_ITERATIONS, &mut key);

        Self { key }
    }

    /// 使用预共享密钥
    pub fn from_pre_shared_key(key_bytes: [u8; 32]) -> Self {
        Self { key: key_bytes }
    }

    /// 生成随机盐值
    pub fn generate_salt() -> [u8; SALT_SIZE] {
        use rand::RngCore;
        let mut salt = [0u8; SALT_SIZE];
        rand::thread_rng().fill_bytes(&mut salt);
        salt
    }

    /// 加密数据
    pub fn encrypt(&self, plaintext: &str) -> Result<EncryptedSyncData, String> {
        let cipher = Aes256Gcm::new_from_slice(&self.key)
            .map_err(|e| format!("Failed to create cipher: {}", e))?;

        let mut nonce_bytes = [0u8; NONCE_SIZE];
        use rand::RngCore;
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce: aes_gcm::Nonce<U12> = nonce_bytes.into();

        let ciphertext = cipher
            .encrypt(&nonce, plaintext.as_bytes())
            .map_err(|e| format!("Encryption failed: {}", e))?;

        let ciphertext_b64 = BASE64.encode(&ciphertext);
        let nonce_b64 = BASE64.encode(nonce_bytes);

        Ok(EncryptedSyncData::new(ciphertext_b64, nonce_b64, String::new(), None))
    }

    /// 解密数据
    pub fn decrypt(&self, data: &EncryptedSyncData) -> Result<String, String> {
        let ciphertext =
            BASE64.decode(&data.ciphertext).map_err(|e| format!("Base64 decode failed: {}", e))?;

        let nonce_bytes =
            BASE64.decode(&data.nonce).map_err(|e| format!("Base64 decode failed: {}", e))?;

        let nonce_array: [u8; NONCE_SIZE] = nonce_bytes
            .try_into()
            .map_err(|e: Vec<u8>| format!("Invalid nonce length: {}", e.len()))?;
        let nonce: aes_gcm::Nonce<U12> = nonce_array.into();

        let cipher = Aes256Gcm::new_from_slice(&self.key)
            .map_err(|e| format!("Failed to create cipher: {}", e))?;

        let plaintext = cipher
            .decrypt(&nonce, ciphertext.as_slice())
            .map_err(|e| format!("Decryption failed: {}", e))?;

        String::from_utf8(plaintext).map_err(|e| format!("UTF-8 decode failed: {}", e))
    }

    /// 获取密钥哈希（用于密钥验证）
    pub fn key_hash(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(self.key);
        let hash = hasher.finalize();
        hex::encode(hash)
    }
}

/// 密钥交换助手
pub struct KeyExchangeHelper;

impl KeyExchangeHelper {
    /// 生成配对码（用于密钥交换验证）
    pub fn generate_pairing_code() -> String {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        (0..6).map(|_| rng.gen_range('0'..='9')).collect()
    }

    /// 验证配对码强度
    pub fn verify_pairing_code_strength(code: &str) -> bool {
        if code.len() != 6 {
            return false;
        }
        code.chars().all(|c| c.is_ascii_digit())
    }

    /// 从设备信息派生同步密钥
    pub fn derive_sync_key(device_id: &str, pairing_code: &str, master_key: &[u8; 32]) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(b"axagent-device-sync-key-derivation-v1");
        hasher.update(device_id.as_bytes());
        hasher.update(pairing_code.as_bytes());
        hasher.update(master_key);
        let hash = hasher.finalize();

        let mut key = [0u8; 32];
        key.copy_from_slice(&hash);
        key
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_salt() -> Vec<u8> {
        vec![1u8; SALT_SIZE]
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let salt = test_salt();
        let encryptor = SyncEncryptor::from_password("test-password", &salt);
        let plaintext = r#"{"key": "value", "number": 42}"#;

        let encrypted = encryptor.encrypt(plaintext).expect("加密应成功");
        let decrypted = encryptor.decrypt(&encrypted).expect("解密应成功");

        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_different_passwords_produce_different_results() {
        let salt = test_salt();
        let enc1 = SyncEncryptor::from_password("password1", &salt);
        let enc2 = SyncEncryptor::from_password("password2", &salt);

        let encrypted = enc1.encrypt("test").unwrap();

        // 用错误密码解密应失败
        assert!(enc2.decrypt(&encrypted).is_err());
    }

    #[test]
    fn test_key_hash() {
        let salt = test_salt();
        let enc1 = SyncEncryptor::from_password("same-password", &salt);
        let enc2 = SyncEncryptor::from_password("same-password", &salt);
        let enc3 = SyncEncryptor::from_password("different-password", &salt);

        // 相同密码应产生相同哈希
        assert_eq!(enc1.key_hash(), enc2.key_hash());
        // 不同密码应产生不同哈希
        assert_ne!(enc1.key_hash(), enc3.key_hash());
    }

    #[test]
    fn test_salt_generation() {
        let salt1 = SyncEncryptor::generate_salt();
        let salt2 = SyncEncryptor::generate_salt();

        assert_eq!(salt1.len(), SALT_SIZE);
        assert_eq!(salt2.len(), SALT_SIZE);
        assert_ne!(salt1, salt2); // 每次生成的盐值应不同
    }

    #[test]
    fn test_pairing_code() {
        let code = KeyExchangeHelper::generate_pairing_code();
        assert_eq!(code.len(), 6);
        assert!(KeyExchangeHelper::verify_pairing_code_strength(&code));
        assert!(!KeyExchangeHelper::verify_pairing_code_strength("12345"));
        assert!(!KeyExchangeHelper::verify_pairing_code_strength("abcdef"));
    }

    #[test]
    fn test_derive_sync_key() {
        let key1 = KeyExchangeHelper::derive_sync_key("device1", "123456", &[0u8; 32]);
        let key2 = KeyExchangeHelper::derive_sync_key("device1", "123456", &[0u8; 32]);
        let key3 = KeyExchangeHelper::derive_sync_key("device2", "123456", &[0u8; 32]);

        assert_eq!(key1, key2);
        assert_ne!(key1, key3);
    }
}

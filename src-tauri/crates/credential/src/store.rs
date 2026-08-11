// SPDX-License-Identifier: AGPL-3.0-only

//! Encrypted credential storage using AES-256-GCM.

use std::path::PathBuf;

use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead, KeyInit},
};
use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use rand::RngExt;
use serde::{Deserialize, Serialize};

use crate::error::{CredentialError, Result};
use crate::types::Credential;

const NONCE_SIZE: usize = 12;

/// keyring 中主密钥的 service name（跨平台 OS 密钥库命名空间）
const KEYRING_SERVICE: &str = "AxAgent";
/// keyring 中主密钥的 username（同一 service 下唯一标识）
const KEYRING_MASTER_KEY_USER: &str = "credential_master_key";

/// OS 级密钥库主密钥存储封装。
///
/// 跨平台后端：
/// - macOS: Keychain
/// - Windows: Credential Manager
/// - Linux: secret-service（需 D-Bus）
///
/// 主密钥以 hex 字符串形式存入 keyring（keyring API 仅接受字符串/字节），
/// 读取时解码回 32 字节。
pub(crate) struct KeyringMasterKeyStore {
    entry: keyring::Entry,
}

impl KeyringMasterKeyStore {
    pub(crate) fn new() -> Result<Self> {
        let entry = keyring::Entry::new(KEYRING_SERVICE, KEYRING_MASTER_KEY_USER)
            .map_err(|e| CredentialError::Internal(format!("keyring entry 创建失败: {e}")))?;
        Ok(Self { entry })
    }

    /// 从 keyring 读取主密钥；无条目返回 Ok(None)，其他错误返回 Err。
    pub(crate) fn load(&self) -> Result<Option<[u8; 32]>> {
        match self.entry.get_password() {
            Ok(hex_str) => {
                let bytes = hex::decode(&hex_str).map_err(|e| {
                    CredentialError::Crypto(format!("keyring 主密钥 hex 解码失败: {e}"))
                })?;
                if bytes.len() != 32 {
                    return Err(CredentialError::Crypto(format!(
                        "keyring 主密钥长度异常: 期望 32 字节, 实际 {} 字节",
                        bytes.len()
                    )));
                }
                let mut key = [0u8; 32];
                key.copy_from_slice(&bytes);
                Ok(Some(key))
            },
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(CredentialError::Internal(format!("keyring 读取失败: {e}"))),
        }
    }

    /// 将主密钥以 hex 形式写入 keyring。写入失败返回错误（绝不静默丢弃）。
    pub(crate) fn store(&self, key: &[u8; 32]) -> Result<()> {
        let hex_str = hex::encode(key);
        self.entry
            .set_password(&hex_str)
            .map_err(|e| CredentialError::Internal(format!("keyring 写入失败: {e}")))
    }
}

/// Metadata-only view of a credential (secrets stripped).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CredentialMeta {
    pub id: String,
    pub name: String,
    pub credential_type: String,
    pub created_at: i64,
    pub updated_at: i64,
}

impl From<&Credential> for CredentialMeta {
    fn from(c: &Credential) -> Self {
        Self {
            id: c.id.clone(),
            name: c.name.clone(),
            credential_type: c.credential_type.to_string(),
            created_at: c.created_at,
            updated_at: c.updated_at,
        }
    }
}

/// Encrypted credential storage backed by the filesystem.
#[derive(Debug)]
pub struct CredentialStore {
    store_dir: PathBuf,
    master_key: [u8; 32],
}

impl CredentialStore {
    pub fn new(store_dir: PathBuf, master_key: [u8; 32]) -> Self {
        Self { store_dir, master_key }
    }

    /// 派生或加载主密钥。
    ///
    /// 优先级链（高 → 低）：
    /// 1. 环境变量 `AXAGENT_CREDENTIAL_MASTER_KEY` / `AXAGENT_MASTER_KEY`
    ///    （hex 编码 32 字节，向后兼容旧部署）
    /// 2. OS 级密钥库（keyring）读取已持久化的主密钥
    /// 3. 上述都没有：生成新的 32 字节随机主密钥并立即写入 OS 密钥库；
    ///    写入失败返回错误（绝不静默丢弃，避免重启后旧凭证无法解密）。
    pub fn derive_master_key() -> Result<[u8; 32]> {
        // 优先级 1：环境变量（向后兼容）
        if let Some(key) = load_master_key_from_env() {
            return Ok(key);
        }
        // 优先级 2 & 3：OS 密钥库
        let store = KeyringMasterKeyStore::new()?;
        if let Some(key) = store.load()? {
            return Ok(key);
        }
        // 都没有：生成新 key 并写入 keyring（写入失败返回错误而非丢弃）
        let mut key = [0u8; 32];
        rand::rng().fill(&mut key);
        store.store(&key)?;
        tracing::info!(
            target: "axagent.credential",
            "已生成新的主密钥并持久化到 OS 密钥库 (service={}, user={})",
            KEYRING_SERVICE,
            KEYRING_MASTER_KEY_USER
        );
        Ok(key)
    }

    fn encrypt(&self, plaintext: &[u8]) -> Result<Vec<u8>> {
        let cipher = Aes256Gcm::new_from_slice(&self.master_key)
            .map_err(|e| CredentialError::Crypto(format!("credential cipher init: {e}")))?;

        let mut nonce_bytes = [0u8; NONCE_SIZE];
        rand::rng().fill(&mut nonce_bytes);
        let nonce = Nonce::try_from(&nonce_bytes[..]).expect("nonce 必须为 12 字节");

        let ciphertext = cipher
            .encrypt(&nonce, plaintext)
            .map_err(|e| CredentialError::Crypto(format!("credential encrypt: {e}")))?;

        let mut combined = Vec::with_capacity(NONCE_SIZE + ciphertext.len());
        combined.extend_from_slice(&nonce_bytes);
        combined.extend_from_slice(&ciphertext);
        Ok(combined)
    }

    fn decrypt(&self, data: &[u8]) -> Result<Vec<u8>> {
        if data.len() < NONCE_SIZE + 16 {
            return Err(CredentialError::Crypto("credential data too short".to_string()));
        }
        let (nonce_bytes, ciphertext) = data.split_at(NONCE_SIZE);
        let nonce = Nonce::try_from(nonce_bytes).expect("nonce 必须为 12 字节");

        let cipher = Aes256Gcm::new_from_slice(&self.master_key)
            .map_err(|e| CredentialError::Crypto(format!("credential cipher init: {e}")))?;

        cipher
            .decrypt(&nonce, ciphertext)
            .map_err(|e| CredentialError::Crypto(format!("credential decrypt: {e}")))
    }

    fn file_path(&self, id: &str) -> PathBuf {
        self.store_dir.join(format!("{id}.enc"))
    }

    fn ensure_dir(&self) -> Result<()> {
        std::fs::create_dir_all(&self.store_dir).map_err(|e| {
            CredentialError::Io(std::io::Error::other(format!("credential store dir: {e}")))
        })
    }

    /// Save a credential to disk (serialize → encrypt → write).
    pub fn save_credential(&self, credential: &Credential) -> Result<()> {
        self.ensure_dir()?;
        let json = serde_json::to_vec(credential)
            .map_err(|e| CredentialError::Internal(format!("credential serialize: {e}")))?;
        let encrypted = self.encrypt(&json)?;
        let path = self.file_path(&credential.id);
        std::fs::write(&path, BASE64.encode(&encrypted)).map_err(|e| {
            CredentialError::Io(std::io::Error::other(format!("credential write {path:?}: {e}")))
        })
    }

    /// Load a credential by ID (read → decrypt → deserialize).
    pub fn load_credential(&self, id: &str) -> Result<Credential> {
        let path = self.file_path(id);
        let b64_data = std::fs::read_to_string(&path).map_err(|e| {
            CredentialError::Io(std::io::Error::other(format!("credential read {path:?}: {e}")))
        })?;
        let encrypted = BASE64
            .decode(b64_data.trim())
            .map_err(|e| CredentialError::Crypto(format!("credential base64 decode: {e}")))?;
        let json = self.decrypt(&encrypted)?;
        serde_json::from_slice(&json)
            .map_err(|e| CredentialError::Internal(format!("credential deserialize: {e}")))
    }

    /// Delete a credential from disk.
    pub fn delete_credential(&self, id: &str) -> Result<()> {
        let path = self.file_path(id);
        if path.exists() {
            std::fs::remove_file(&path).map_err(|e| {
                CredentialError::Io(std::io::Error::other(format!(
                    "credential delete {path:?}: {e}"
                )))
            })?;
        }
        Ok(())
    }

    /// List all stored credentials (metadata only, secrets excluded).
    pub fn list_credentials(&self) -> Result<Vec<CredentialMeta>> {
        self.ensure_dir()?;
        let mut metas = Vec::new();
        let entries = match std::fs::read_dir(&self.store_dir) {
            Ok(e) => e,
            Err(_) => return Ok(metas),
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "enc")
                && let Some(stem) = path.file_stem().and_then(|s| s.to_str())
            {
                match self.load_credential(stem) {
                    Ok(cred) => metas.push(CredentialMeta::from(&cred)),
                    Err(e) => {
                        tracing::warn!(?e, id = stem, "Failed to load credential for listing");
                    },
                }
            }
        }
        Ok(metas)
    }
}

/// 从环境变量读取主密钥（hex 编码 32 字节）。
///
/// 按顺序尝试 `AXAGENT_CREDENTIAL_MASTER_KEY` → `AXAGENT_MASTER_KEY`。
/// 任一变量存在但格式/长度非法时跳过该变量（向后兼容旧部署的容错策略）。
/// 全部未设置或全部非法时返回 None。
fn load_master_key_from_env() -> Option<[u8; 32]> {
    for var in ["AXAGENT_CREDENTIAL_MASTER_KEY", "AXAGENT_MASTER_KEY"] {
        if let Ok(hex_key) = std::env::var(var)
            && let Ok(bytes) = hex::decode(&hex_key)
            && bytes.len() == 32
        {
            let mut key = [0u8; 32];
            key.copy_from_slice(&bytes);
            return Some(key);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Credential, CredentialType};

    /// 测试用临时存储：退出时清理磁盘目录，避免遗留加密文件。
    struct TempStore {
        dir: std::path::PathBuf,
        store: CredentialStore,
    }

    impl Drop for TempStore {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.dir);
        }
    }

    fn temp_store() -> TempStore {
        let mut suffix = [0u8; 8];
        rand::rng().fill(&mut suffix);
        let dir = std::env::temp_dir().join(format!("axagent_cred_test_{}", hex::encode(suffix)));
        std::fs::create_dir_all(&dir).ok();
        // 固定主密钥，便于"错误密钥解密失败"断言。
        TempStore { dir: dir.clone(), store: CredentialStore::new(dir, [0x42u8; 32]) }
    }

    fn api_key_credential() -> Credential {
        Credential::new(
            "cred-1".to_string(),
            "OpenAI Key".to_string(),
            CredentialType::ApiKey {
                key: "sk-xxxx".to_string(),
                header_name: "X-Api-Key".to_string(),
            },
        )
    }

    #[test]
    fn save_and_load_roundtrip() {
        let ts = temp_store();
        let cred = api_key_credential();
        ts.store.save_credential(&cred).expect("测试应成功");
        let loaded = ts.store.load_credential("cred-1").expect("测试应成功");
        // Credential 未派生 PartialEq，改用 JSON 序列化比较（含时间戳）确保完全一致。
        assert_eq!(
            serde_json::to_string(&loaded).expect("测试：JSON序列化应成功"),
            serde_json::to_string(&cred).unwrap()
        );
    }

    #[test]
    fn roundtrip_all_credential_types() {
        let ts = temp_store();
        let cases = vec![
            Credential::new(
                "a".into(),
                "bearer".into(),
                CredentialType::BearerToken { token: "t".into() },
            ),
            Credential::new(
                "b".into(),
                "basic".into(),
                CredentialType::BasicAuth { username: "u".into(), password: "p".into() },
            ),
            Credential::new(
                "c".into(),
                "smtp".into(),
                CredentialType::Smtp {
                    host: "h".into(),
                    port: 25,
                    user: "u".into(),
                    pass: "p".into(),
                    tls: true,
                },
            ),
            Credential::new(
                "d".into(),
                "db".into(),
                CredentialType::DatabaseConnection { connection_string: "postgres://x".into() },
            ),
            Credential::new(
                "e".into(),
                "oauth".into(),
                CredentialType::OAuth2 {
                    client_id: "ci".into(),
                    client_secret: "cs".into(),
                    token_url: "tu".into(),
                    scopes: vec!["s1".into()],
                    access_token: None,
                    expires_at: None,
                },
            ),
        ];
        for c in &cases {
            ts.store.save_credential(c).expect("测试应成功");
            let loaded = ts.store.load_credential(&c.id).expect("测试应成功");
            assert_eq!(
                serde_json::to_string(&loaded.credential_type).expect("测试：JSON序列化应成功"),
                serde_json::to_string(&c.credential_type).expect("测试：JSON序列化应成功"),
                "类型 {} 往返不一致",
                c.id
            );
        }
    }

    #[test]
    fn wrong_master_key_fails() {
        let mut suffix = [0u8; 8];
        rand::rng().fill(&mut suffix);
        let dir = std::env::temp_dir().join(format!("axagent_cred_wk_{}", hex::encode(suffix)));
        std::fs::create_dir_all(&dir).ok();
        let store_a = CredentialStore::new(dir.clone(), [0x11u8; 32]);
        store_a.save_credential(&api_key_credential()).expect("测试应成功");
        // 用不同主密钥读取应解密失败（密钥绑定是安全边界）。
        let store_b = CredentialStore::new(dir.clone(), [0x22u8; 32]);
        assert!(store_b.load_credential("cred-1").is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn delete_then_load_fails() {
        let ts = temp_store();
        let cred = api_key_credential();
        ts.store.save_credential(&cred).expect("测试应成功");
        ts.store.delete_credential("cred-1").expect("测试应成功");
        assert!(ts.store.load_credential("cred-1").is_err());
    }

    #[test]
    fn list_credentials_metadata_only() {
        let ts = temp_store();
        let c1 = Credential::new(
            "x".into(),
            "Alpha".into(),
            CredentialType::ApiKey { key: "k".into(), header_name: "h".into() },
        );
        let c2 = Credential::new(
            "y".into(),
            "Beta".into(),
            CredentialType::BearerToken { token: "t".into() },
        );
        ts.store.save_credential(&c1).expect("测试应成功");
        ts.store.save_credential(&c2).expect("测试应成功");
        let metas = ts.store.list_credentials().expect("测试：list_credentials 应成功");
        assert_eq!(metas.len(), 2);
        let names: Vec<&str> = metas.iter().map(|m| m.name.as_str()).collect();
        assert!(names.contains(&"Alpha"));
        assert!(names.contains(&"Beta"));
        // 元数据不应包含 secret 字段（CredentialMeta 本身不含密钥）。
        for m in &metas {
            assert!(m.credential_type == "ApiKey" || m.credential_type == "BearerToken");
        }
    }

    #[test]
    fn corrupt_file_fails_to_load() {
        let ts = temp_store();
        ts.store.save_credential(&api_key_credential()).expect("测试应成功");
        // 直接覆写密文为垃圾，模拟磁盘损坏。
        let path = ts.dir.join("cred-1.enc");
        std::fs::write(&path, "not-valid-base64-@@@@").expect("测试：写入文件应成功");
        assert!(ts.store.load_credential("cred-1").is_err());
    }

    #[test]
    fn derive_master_key_length() {
        // 通过环境变量提供主密钥，避免测试时污染 OS 密钥库，
        // 同时避免依赖 keyring 后端（CI 可能无 secret-service）
        let hex_key = hex::encode([0x42u8; 32]);
        // SAFETY: 其他测试均直接用 `CredentialStore::new(dir, key)` 构造，不读 env var，
        //         此处 set/remove 不会与其他测试竞争。
        unsafe {
            std::env::set_var("AXAGENT_CREDENTIAL_MASTER_KEY", &hex_key);
        }
        let key = CredentialStore::derive_master_key().expect("env var 模式应成功");
        assert_eq!(key.len(), 32);
        assert_eq!(key, [0x42u8; 32]);
        // SAFETY: 同上
        unsafe {
            std::env::remove_var("AXAGENT_CREDENTIAL_MASTER_KEY");
        }
    }
}

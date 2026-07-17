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

    /// Derive or load the master key from environment / configuration.
    pub fn derive_master_key() -> [u8; 32] {
        if let Ok(hex_key) = std::env::var("AXAGENT_CREDENTIAL_MASTER_KEY")
            && let Ok(bytes) = hex::decode(&hex_key)
            && bytes.len() == 32
        {
            let mut key = [0u8; 32];
            key.copy_from_slice(&bytes);
            return key;
        }
        if let Ok(hex_key) = std::env::var("AXAGENT_MASTER_KEY")
            && let Ok(bytes) = hex::decode(&hex_key)
            && bytes.len() == 32
        {
            let mut key = [0u8; 32];
            key.copy_from_slice(&bytes);
            return key;
        }
        let mut key = [0u8; 32];
        rand::rng().fill(&mut key);
        key
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
        ts.store.save_credential(&cred).unwrap();
        let loaded = ts.store.load_credential("cred-1").unwrap();
        // Credential 未派生 PartialEq，改用 JSON 序列化比较（含时间戳）确保完全一致。
        assert_eq!(serde_json::to_string(&loaded).unwrap(), serde_json::to_string(&cred).unwrap());
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
                },
            ),
        ];
        for c in &cases {
            ts.store.save_credential(c).unwrap();
            let loaded = ts.store.load_credential(&c.id).unwrap();
            assert_eq!(
                serde_json::to_string(&loaded.credential_type).unwrap(),
                serde_json::to_string(&c.credential_type).unwrap(),
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
        store_a.save_credential(&api_key_credential()).unwrap();
        // 用不同主密钥读取应解密失败（密钥绑定是安全边界）。
        let store_b = CredentialStore::new(dir.clone(), [0x22u8; 32]);
        assert!(store_b.load_credential("cred-1").is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn delete_then_load_fails() {
        let ts = temp_store();
        let cred = api_key_credential();
        ts.store.save_credential(&cred).unwrap();
        ts.store.delete_credential("cred-1").unwrap();
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
        ts.store.save_credential(&c1).unwrap();
        ts.store.save_credential(&c2).unwrap();
        let metas = ts.store.list_credentials().unwrap();
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
        ts.store.save_credential(&api_key_credential()).unwrap();
        // 直接覆写密文为垃圾，模拟磁盘损坏。
        let path = ts.dir.join("cred-1.enc");
        std::fs::write(&path, "not-valid-base64-@@@@").unwrap();
        assert!(ts.store.load_credential("cred-1").is_err());
    }

    #[test]
    fn derive_master_key_length() {
        let key = CredentialStore::derive_master_key();
        assert_eq!(key.len(), 32);
    }
}

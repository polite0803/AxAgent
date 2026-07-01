// SPDX-License-Identifier: AGPL-3.0-only

//! Encrypted credential storage using AES-256-GCM.
//!
//! Credentials are serialized to JSON, encrypted with a master key, and persisted
//! to the filesystem. The master key is derived from environment variables or
//! configuration at application startup.

use std::collections::HashMap;
use std::path::PathBuf;

use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead, KeyInit},
};
use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use rand::Rng;
use serde::{Deserialize, Serialize};

use super::{Credential, CredentialType};
use crate::core_error::{AxAgentError, Result};

const NONCE_SIZE: usize = 12;

/// Metadata-only view of a credential (secrets stripped)
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
///
/// Each credential is stored as a separate encrypted file in the store directory.
/// The master key must be provided at construction time and is used for all
/// encryption/decryption operations.
pub struct CredentialStore {
    store_dir: PathBuf,
    master_key: [u8; 32],
}

impl CredentialStore {
    /// Create a new credential store.
    ///
    /// `store_dir` is the directory where encrypted credential files are kept.
    /// `master_key` is a 256-bit key used for AES-256-GCM encryption.
    pub fn new(store_dir: PathBuf, master_key: [u8; 32]) -> Self {
        Self {
            store_dir,
            master_key,
        }
    }

    /// Derive or load the master key from environment / configuration.
    ///
    /// Priority:
    /// 1. `AXAGENT_CREDENTIAL_MASTER_KEY` environment variable (hex-encoded 32 bytes)
    /// 2. `AXAGENT_MASTER_KEY` environment variable (hex-encoded, shared with other crypto)
    /// 3. Generate a new random key (persisted via the caller)
    pub fn derive_master_key() -> [u8; 32] {
        // Try credential-specific key first
        if let Ok(hex_key) = std::env::var("AXAGENT_CREDENTIAL_MASTER_KEY") {
            if let Ok(bytes) = hex::decode(&hex_key) {
                if bytes.len() == 32 {
                    let mut key = [0u8; 32];
                    key.copy_from_slice(&bytes);
                    return key;
                }
            }
        }
        // Fall back to shared master key
        if let Ok(hex_key) = std::env::var("AXAGENT_MASTER_KEY") {
            if let Ok(bytes) = hex::decode(&hex_key) {
                if bytes.len() == 32 {
                    let mut key = [0u8; 32];
                    key.copy_from_slice(&bytes);
                    return key;
                }
            }
        }
        // Generate new random key
        let mut key = [0u8; 32];
        rand::rng().fill(&mut key);
        key
    }

    /// Encrypt plaintext bytes with AES-256-GCM.
    fn encrypt(&self, plaintext: &[u8]) -> Result<Vec<u8>> {
        let cipher = Aes256Gcm::new_from_slice(&self.master_key)
            .map_err(|e| AxAgentError::Crypto(format!("credential cipher init: {e}")))?;

        let mut nonce_bytes = [0u8; NONCE_SIZE];
        rand::rng().fill(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = cipher
            .encrypt(nonce, plaintext)
            .map_err(|e| AxAgentError::Crypto(format!("credential encrypt: {e}")))?;

        let mut combined = Vec::with_capacity(NONCE_SIZE + ciphertext.len());
        combined.extend_from_slice(&nonce_bytes);
        combined.extend_from_slice(&ciphertext);
        Ok(combined)
    }

    /// Decrypt ciphertext bytes with AES-256-GCM.
    fn decrypt(&self, data: &[u8]) -> Result<Vec<u8>> {
        if data.len() < NONCE_SIZE + 16 {
            return Err(AxAgentError::Crypto(
                "credential data too short".to_string(),
            ));
        }
        let (nonce_bytes, ciphertext) = data.split_at(NONCE_SIZE);
        let nonce = Nonce::from_slice(nonce_bytes);

        let cipher = Aes256Gcm::new_from_slice(&self.master_key)
            .map_err(|e| AxAgentError::Crypto(format!("credential cipher init: {e}")))?;

        cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| AxAgentError::Crypto(format!("credential decrypt: {e}")))
    }

    /// Compute the file path for a credential by its ID.
    fn file_path(&self, id: &str) -> PathBuf {
        self.store_dir.join(format!("{id}.enc"))
    }

    /// Ensure the store directory exists.
    fn ensure_dir(&self) -> Result<()> {
        std::fs::create_dir_all(&self.store_dir).map_err(|e| {
            AxAgentError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("credential store dir: {e}"),
            ))
        })
    }

    /// Save a credential to disk (serialize → encrypt → write).
    pub fn save_credential(&self, credential: &Credential) -> Result<()> {
        self.ensure_dir()?;
        let json = serde_json::to_vec(credential)
            .map_err(|e| AxAgentError::Internal(format!("credential serialize: {e}")))?;
        let encrypted = self.encrypt(&json)?;
        let path = self.file_path(&credential.id);
        std::fs::write(&path, &BASE64.encode(&encrypted)).map_err(|e| {
            AxAgentError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("credential write {path:?}: {e}"),
            ))
        })
    }

    /// Load a credential by ID (read → decrypt → deserialize).
    pub fn load_credential(&self, id: &str) -> Result<Credential> {
        let path = self.file_path(id);
        let b64_data = std::fs::read_to_string(&path).map_err(|e| {
            AxAgentError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("credential read {path:?}: {e}"),
            ))
        })?;
        let encrypted = BASE64.decode(b64_data.trim()).map_err(|e| {
            AxAgentError::Crypto(format!("credential base64 decode: {e}"))
        })?;
        let json = self.decrypt(&encrypted)?;
        serde_json::from_slice(&json)
            .map_err(|e| AxAgentError::Internal(format!("credential deserialize: {e}")))
    }

    /// Delete a credential from disk.
    pub fn delete_credential(&self, id: &str) -> Result<()> {
        let path = self.file_path(id);
        if path.exists() {
            std::fs::remove_file(&path).map_err(|e| {
                AxAgentError::Io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("credential delete {path:?}: {e}"),
                ))
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
            if path.extension().map_or(false, |ext| ext == "enc") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    match self.load_credential(stem) {
                        Ok(cred) => metas.push(CredentialMeta::from(&cred)),
                        Err(e) => {
                            tracing::warn!(
                                ?e,
                                id = stem,
                                "Failed to load credential for listing"
                            );
                        }
                    }
                }
            }
        }
        Ok(metas)
    }
}

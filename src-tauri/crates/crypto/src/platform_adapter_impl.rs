// SPDX-License-Identifier: AGPL-3.0-only

//! `axagent_harness::platform_adapter::CryptoService` trait 的默认实现。
//!
//! 持有 master_key，各方法委托给 crate 内的 free function。

use axagent_harness::core_error::Result;
use axagent_harness::platform_adapter::CryptoService;

pub struct DefaultCryptoService {
    pub master_key: [u8; 32],
}

impl DefaultCryptoService {
    pub fn new(master_key: [u8; 32]) -> Self {
        Self { master_key }
    }
}

impl CryptoService for DefaultCryptoService {
    fn decrypt_key(&self, encrypted: &str) -> Result<String> {
        crate::crypto::decrypt_key(encrypted, &self.master_key)
    }

    fn encrypt_key(&self, plaintext: &str) -> Result<String> {
        crate::crypto::encrypt_key(plaintext, &self.master_key)
    }

    fn decrypt_key_with(&self, encrypted: &str, master_key: &[u8; 32]) -> Result<String> {
        crate::crypto::decrypt_key(encrypted, master_key)
    }

    fn encrypt_key_with(&self, plaintext: &str, master_key: &[u8; 32]) -> Result<String> {
        crate::crypto::encrypt_key(plaintext, master_key)
    }

    fn hmac_sha256(&self, key: &[u8], msg: &str) -> String {
        crate::crypto::hmac_sha256(key, msg)
    }

    fn sha256_hash(&self, input: &str) -> String {
        crate::crypto::sha256_hash(input)
    }

    fn key_prefix(&self, key: &str) -> String {
        crate::crypto::key_prefix(key)
    }

    fn generate_gateway_key(&self) -> String {
        crate::crypto::generate_gateway_key()
    }

    fn generate_master_key(&self) -> [u8; 32] {
        crate::crypto::generate_master_key()
    }

    fn encrypt_backup_key(&self, key_data: &[u8]) -> Result<Vec<u8>> {
        crate::crypto::encrypt_backup_key(key_data)
    }

    fn decrypt_backup_key(&self, enc_data: &[u8]) -> Result<Vec<u8>> {
        crate::crypto::decrypt_backup_key(enc_data)
    }
}

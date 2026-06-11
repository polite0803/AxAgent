// SPDX-License-Identifier: AGPL-3.0-only

//! `axagent_harness::platform_adapter::CryptoService` trait 的默认实现。
//!
//! 持有 master_key，decrypt_key 转发给 crate 内的 free function。

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
}

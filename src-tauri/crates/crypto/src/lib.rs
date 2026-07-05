// SPDX-License-Identifier: AGPL-3.0-only

//! axagent-crypto — 加密工具
//!
//! AES-256-GCM 加密、Argon2 密钥派生、SHA-256 哈希等。

pub mod crypto;
pub mod platform_adapter_impl;

pub use crypto::decrypt_key;
pub use crypto::derive_storage_master_key;
pub use crypto::encrypt_key;

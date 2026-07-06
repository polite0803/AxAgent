// SPDX-License-Identifier: AGPL-3.0-only

//! Gateway 层访问 dao + crypto 的 trait 抽象。
//!
//! `PlatformAdapter` 是 facade trait，把 5 个子 trait 组合起来。
//! gateway crate 不再直接依赖 axagent-dao / axagent-crypto，改为依赖本文件。

use async_trait::async_trait;

use crate::core_error::Result;
use crate::types::{AppSettings, GatewayKey, ProviderConfig, ProviderKey};

/// Chat completion parameters for platform bridge dispatches.
pub struct ChatCompletionParams {
    pub system_prompt: String,
    pub message: String,
    pub platform: String,
    pub workflow_id: Option<String>,
}

// ── 1. ProviderRepository ──

#[async_trait]
pub trait ProviderRepository: Send + Sync {
    async fn list_providers(&self) -> Result<Vec<ProviderConfig>>;
    async fn get_active_key(&self, provider_id: &str) -> Result<ProviderKey>;
    /// Report a provider API key failure (e.g. 401/403/429)
    /// so subsequent calls can failover to the next available key.
    async fn report_key_failure(&self, _key_id: &str, _error_msg: &str) -> Result<()> {
        // Default implementation: no-op (backwards compatible).
        Ok(())
    }
}

// ── 2. SettingsRepository ──

#[async_trait]
pub trait SettingsRepository: Send + Sync {
    async fn get_settings(&self) -> Result<AppSettings>;
}

// ── 3. GatewayKeyRepository ──

#[async_trait]
pub trait GatewayKeyRepository: Send + Sync {
    async fn list_gateway_keys(&self) -> Result<Vec<GatewayKey>>;
    async fn verify_key(&self, token: &str) -> Result<Option<GatewayKey>>;
    /// Look up a key by its stable id. Returns `None` if not found.
    /// SECURITY: callers must not assume the key is enabled — check `key.enabled`
    /// before granting access.
    async fn get_by_id(&self, key_id: &str) -> Result<Option<GatewayKey>>;
    async fn update_last_used(&self, key_id: &str) -> Result<()>;
    /// 将平台用户 ID 绑定到指定的 API Key（QR 扫码绑定用）。
    async fn bind_platform_user(
        &self,
        _key_id: &str,
        _platform: &str,
        _platform_user_id: &str,
    ) -> Result<()> {
        Err(crate::core_error::AxAgentError::Internal(
            "bind_platform_user not implemented by this repository".into(),
        ))
    }
    #[allow(clippy::too_many_arguments)]
    async fn record_usage(
        &self,
        key_id: &str,
        provider_id: &str,
        model_id: Option<&str>,
        request_tokens: u64,
        response_tokens: u64,
        cached_input_tokens: u64,
    ) -> Result<()>;
}

// ── 4. GatewayRequestLogRepository ──

#[async_trait]
pub trait GatewayRequestLogRepository: Send + Sync {
    #[allow(clippy::too_many_arguments)]
    async fn record_request_log(
        &self,
        key_id: &str,
        key_name: &str,
        method: &str,
        path: &str,
        model_id: Option<&str>,
        provider_id: Option<&str>,
        status_code: i32,
        duration_ms: i32,
        request_tokens: i64,
        response_tokens: i64,
        error_message: Option<&str>,
    ) -> Result<()>;
}

// ── 5. CryptoService ──

pub trait CryptoService: Send + Sync {
    /// 使用内置 master_key 解密 base64 字符串，返回明文。
    fn decrypt_key(&self, encrypted: &str) -> Result<String>;
    /// 使用内置 master_key 加密明文，返回 base64 密文。
    fn encrypt_key(&self, plaintext: &str) -> Result<String>;
    /// 使用显式密钥解密。
    fn decrypt_key_with(&self, encrypted: &str, master_key: &[u8; 32]) -> Result<String>;
    /// 使用显式密钥加密。
    fn encrypt_key_with(&self, plaintext: &str, master_key: &[u8; 32]) -> Result<String>;
    /// HMAC-SHA256，返回十六进制字符串。key 为原始字节。
    fn hmac_sha256(&self, key: &[u8], msg: &str) -> String;
    /// SHA-256 哈希，返回十六进制字符串。
    fn sha256_hash(&self, input: &str) -> String;
    /// 从 key 中提取可识别前缀（仅用于 UI 展示）。
    fn key_prefix(&self, key: &str) -> String;
    /// 生成新的 gateway API key。
    fn generate_gateway_key(&self) -> String;
    /// 生成随机 32 字节 master key。
    fn generate_master_key(&self) -> [u8; 32];
    /// 使用机器指纹派生密钥加密备份数据。
    fn encrypt_backup_key(&self, key_data: &[u8]) -> Result<Vec<u8>>;
    /// 解密备份数据。
    fn decrypt_backup_key(&self, enc_data: &[u8]) -> Result<Vec<u8>>;
}

// ── PlatformAdapter（facade trait） ──

/// 把上面 5 个子 trait 聚合成一个入口，wiring 层注入一次，gateway 内部通过
/// `state.adapter.providers().xxx()` 链式调用。
#[async_trait]
pub trait PlatformAdapter: Send + Sync {
    fn providers(&self) -> &dyn ProviderRepository;
    fn settings(&self) -> &dyn SettingsRepository;
    fn gateway_keys(&self) -> &dyn GatewayKeyRepository;
    fn request_log(&self) -> &dyn GatewayRequestLogRepository;
    fn crypto(&self) -> &dyn CryptoService;

    /// Dispatch a chat completion request via the configured provider.
    /// Used by the platform bridge webhook handler to respond to incoming messages.
    async fn chat_completion(&self, params: ChatCompletionParams) -> Result<String> {
        let _ = params;
        Err(crate::core_error::AxAgentError::Internal(
            "chat_completion not implemented by this adapter".into(),
        ))
    }
}

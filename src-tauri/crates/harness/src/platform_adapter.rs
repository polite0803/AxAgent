// SPDX-License-Identifier: AGPL-3.0-only

//! Gateway 层访问 dao + crypto 的 trait 抽象。
//!
//! `PlatformAdapter` 是 facade trait，把 5 个子 trait 组合起来。
//! gateway crate 不再直接依赖 axagent-dao / axagent-crypto，改为依赖本文件。

use async_trait::async_trait;

use crate::core_error::Result;
use crate::types::{AppSettings, GatewayKey, ProviderConfig, ProviderKey};

// ── 1. ProviderRepository ──

#[async_trait]
pub trait ProviderRepository: Send + Sync {
    async fn list_providers(&self) -> Result<Vec<ProviderConfig>>;
    async fn get_active_key(&self, provider_id: &str) -> Result<ProviderKey>;
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
    /// 解密用 master_key 加密的 base64 字符串，返回明文。
    fn decrypt_key(&self, encrypted: &str) -> Result<String>;
}

// ── PlatformAdapter（facade trait） ──

/// 把上面 5 个子 trait 聚合成一个入口，wiring 层注入一次，gateway 内部通过
/// `state.adapter.providers().xxx()` 链式调用。
pub trait PlatformAdapter: Send + Sync {
    fn providers(&self) -> &dyn ProviderRepository;
    fn settings(&self) -> &dyn SettingsRepository;
    fn gateway_keys(&self) -> &dyn GatewayKeyRepository;
    fn request_log(&self) -> &dyn GatewayRequestLogRepository;
    fn crypto(&self) -> &dyn CryptoService;
}

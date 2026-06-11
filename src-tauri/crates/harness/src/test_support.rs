//! 测试替身支持
//!
//! 提供 `axagent-harness` 自身定义的 mock / empty 实现，使测试代码
//! 无需在 dev-dependencies 里引入具体的 provider / tool 实现 crate。
//!
//! 目的：测试也走 trait 抽象，与生产代码风格一致。
//!
//! 注意：本模块始终编译，但内部仅导出"返回 None / 空实现"的轻量辅助函数，
//! 不会引入运行时依赖，也不会进入生产热路径（仅测试代码使用）。

use std::sync::Arc;

use crate::core_error::{AxAgentError, Result};
use crate::platform_adapter::{
    CryptoService, GatewayKeyRepository, GatewayRequestLogRepository, PlatformAdapter,
    ProviderRepository, SettingsRepository,
};
use crate::provider::ProviderAdapter;
use crate::types::{AppSettings, GatewayKey, ProviderConfig, ProviderKey};

/// 一个返回 `None` 的空 ProviderRegistry，测试 gateway / runtime 时使用。
pub struct EmptyProviderRegistry;

impl crate::registry::ProviderRegistry for EmptyProviderRegistry {
    fn get(&self, _provider_type: &str) -> Option<Arc<dyn ProviderAdapter>> {
        None
    }
}

/// 工厂：构造一个 `Arc<dyn ProviderRegistry>` 测试替身
pub fn empty_provider_registry() -> Arc<dyn crate::registry::ProviderRegistry> {
    Arc::new(EmptyProviderRegistry)
}

// ── PlatformAdapter 测试替身 ──

struct EmptyProviderRepository;
#[async_trait::async_trait]
impl ProviderRepository for EmptyProviderRepository {
    async fn list_providers(&self) -> Result<Vec<ProviderConfig>> {
        Ok(vec![])
    }
    async fn get_active_key(&self, _provider_id: &str) -> Result<ProviderKey> {
        Err(AxAgentError::NotFound("test stub".into()))
    }
}

struct EmptySettingsRepository;
#[async_trait::async_trait]
impl SettingsRepository for EmptySettingsRepository {
    async fn get_settings(&self) -> Result<AppSettings> {
        Err(AxAgentError::NotFound("test stub".into()))
    }
}

struct EmptyGatewayKeyRepository;
#[async_trait::async_trait]
impl GatewayKeyRepository for EmptyGatewayKeyRepository {
    async fn list_gateway_keys(&self) -> Result<Vec<GatewayKey>> {
        Ok(vec![])
    }
    async fn verify_key(&self, _token: &str) -> Result<Option<GatewayKey>> {
        Ok(None)
    }
    async fn update_last_used(&self, _key_id: &str) -> Result<()> {
        Ok(())
    }
    async fn record_usage(
        &self,
        _key_id: &str,
        _provider_id: &str,
        _model_id: Option<&str>,
        _request_tokens: u64,
        _response_tokens: u64,
        _cached_input_tokens: u64,
    ) -> Result<()> {
        Ok(())
    }
}

struct EmptyGatewayRequestLogRepository;
#[async_trait::async_trait]
impl GatewayRequestLogRepository for EmptyGatewayRequestLogRepository {
    async fn record_request_log(
        &self,
        _key_id: &str,
        _key_name: &str,
        _method: &str,
        _path: &str,
        _model_id: Option<&str>,
        _provider_id: Option<&str>,
        _status_code: i32,
        _duration_ms: i32,
        _request_tokens: i64,
        _response_tokens: i64,
        _error_message: Option<&str>,
    ) -> Result<()> {
        Ok(())
    }
}

struct EmptyCryptoService;
impl CryptoService for EmptyCryptoService {
    fn decrypt_key(&self, _encrypted: &str) -> Result<String> {
        Err(AxAgentError::Crypto("test stub".into()))
    }
}

struct EmptyPlatformAdapter;
impl PlatformAdapter for EmptyPlatformAdapter {
    fn providers(&self) -> &dyn ProviderRepository {
        &EmptyProviderRepository
    }
    fn settings(&self) -> &dyn SettingsRepository {
        &EmptySettingsRepository
    }
    fn gateway_keys(&self) -> &dyn GatewayKeyRepository {
        &EmptyGatewayKeyRepository
    }
    fn request_log(&self) -> &dyn GatewayRequestLogRepository {
        &EmptyGatewayRequestLogRepository
    }
    fn crypto(&self) -> &dyn CryptoService {
        &EmptyCryptoService
    }
}

/// 工厂：构造一个 `Arc<dyn PlatformAdapter>` 测试替身（所有方法返回空 / 错误）
pub fn empty_platform_adapter() -> Arc<dyn PlatformAdapter> {
    Arc::new(EmptyPlatformAdapter)
}

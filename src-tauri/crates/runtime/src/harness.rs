//! RuntimeHarness — 中心化 Harness 容器
//!
//! 负责统一管理核心基础设施的生命周期和依赖注入。
//! 当前范围：ProviderRegistry + ProviderAdapter 缓存。
//! 未来可扩展：ToolRegistry、SessionManager、WorkEngine 等。

use sea_orm::DatabaseConnection;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

use axagent_harness::ProviderAdapter;
use axagent_harness::registry::ProviderRegistry as ProviderRegistryTrait;

/// 统一容器：管理核心服务的创建与注入
#[derive(Clone)]
pub struct RuntimeHarness {
    db: DatabaseConnection,
    master_key: [u8; 32],
    /// Provider 注册表 — 可查找所有 LLM 提供商适配器
    provider_registry: Arc<dyn ProviderRegistryTrait>,
    /// ProviderAdapter 缓存（按 provider 类型名）
    adapter_cache: Arc<Mutex<HashMap<String, Arc<dyn ProviderAdapter>>>>,
}

impl RuntimeHarness {
    /// 创建 Harness 容器
    pub fn new(db: DatabaseConnection, master_key: [u8; 32]) -> Self {
        let concrete_registry =
            Arc::new(axagent_providers::registry::ProviderRegistry::create_default());
        Self {
            db,
            master_key,
            provider_registry: concrete_registry as Arc<dyn ProviderRegistryTrait>,
            adapter_cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    // ── Accessors ─────────────────────────────────────────────

    pub fn db(&self) -> &DatabaseConnection {
        &self.db
    }

    pub fn master_key(&self) -> &[u8; 32] {
        &self.master_key
    }

    /// 获取 ProviderRegistry（用于查找 LLM 适配器）
    pub fn provider_registry(&self) -> &Arc<dyn ProviderRegistryTrait> {
        &self.provider_registry
    }

    /// 获取或缓存指定类型的 ProviderAdapter
    pub async fn get_adapter(&self, provider_type: &str) -> Option<Arc<dyn ProviderAdapter>> {
        let mut cache = self.adapter_cache.lock().await;
        if let Some(adapter) = cache.get(provider_type) {
            return Some(adapter.clone());
        }
        if let Some(adapter) = self.provider_registry.get(provider_type) {
            cache.insert(provider_type.to_string(), adapter.clone());
            Some(adapter)
        } else {
            None
        }
    }

    // ── Builder 方法 ──────────────────────────────────────────

    /// 构建已注入 ProviderRegistry 的 PlatformBridge
    pub fn build_platform_bridge(
        &self,
        platform_manager: Arc<
            axagent_rt_messaging::message_gateway::platform_manager::PlatformManager,
        >,
    ) -> Arc<axagent_rt_messaging::message_gateway::platform_bridge::PlatformBridge> {
        let bridge = axagent_rt_messaging::message_gateway::platform_bridge::PlatformBridge::new(
            self.db.clone(),
            self.master_key,
            platform_manager,
        )
        .with_provider_registry(self.provider_registry.clone());
        Arc::new(bridge)
    }
}

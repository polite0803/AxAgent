//! RuntimeHarness — 中心化 Harness 容器
//!
//! 负责统一管理核心基础设施的生命周期和依赖注入。
//! 当前范围：
//! - Persistence（数据库连接）
//! - ProviderRegistry + ProviderAdapter 缓存
//! - master_key
//!
//! 未来可扩展：ToolRegistry、CronJobStore、WorkEngine 等。

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

use axagent_harness::Persistence;
use axagent_harness::ProviderAdapter;
use axagent_harness::registry::ProviderRegistry as ProviderRegistryTrait;

/// 统一容器：管理核心服务的创建与注入
#[derive(Clone)]
pub struct RuntimeHarness {
    persistence: Arc<dyn Persistence>,
    master_key: [u8; 32],
    /// Provider 注册表 — 可查找所有 LLM 提供商适配器
    provider_registry: Arc<dyn ProviderRegistryTrait>,
    /// ProviderAdapter 缓存（按 provider 类型名）
    adapter_cache: Arc<Mutex<HashMap<String, Arc<dyn ProviderAdapter>>>>,
}

/// 构造 RuntimeHarness 时的依赖
pub struct HarnessDeps {
    pub persistence: Arc<dyn Persistence>,
    pub master_key: [u8; 32],
    /// Provider 注册表（由调用方创建并传入，不设默认值）
    pub provider_registry: Arc<dyn ProviderRegistryTrait>,
}

impl RuntimeHarness {
    /// 创建 Harness 容器
    ///
    /// 所有依赖必须由调用方注入，不再硬编码具体实现。
    pub fn new(deps: HarnessDeps) -> Self {
        Self {
            persistence: deps.persistence,
            master_key: deps.master_key,
            provider_registry: deps.provider_registry,
            adapter_cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    // ── Accessors ─────────────────────────────────────────────

    /// 拿到底层持久化句柄（数据库连接）
    pub fn persistence(&self) -> &Arc<dyn Persistence> {
        &self.persistence
    }

    /// 兼容旧调用：`db()` 仍然返回 `&DatabaseConnection`
    /// （避免一次大爆炸式重构，下游逐步迁移到 `persistence().connection()`）
    pub fn db(&self) -> &axagent_harness::DatabaseConnection {
        self.persistence.connection()
    }

    /// 兼容旧调用：`db_path()` 直接返回持久化层的路径
    pub fn db_path(&self) -> &str {
        self.persistence.db_path()
    }

    pub fn master_key(&self) -> &[u8; 32] {
        &self.master_key
    }

    /// 拿到 master_key 的 owned 副本（用于按值传递的调用方）
    pub fn master_key_owned(&self) -> [u8; 32] {
        self.master_key
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
        let mut bridge =
            axagent_rt_messaging::message_gateway::platform_bridge::PlatformBridge::new(
                self.persistence.connection().clone(),
                self.master_key,
                platform_manager,
            );
        use axagent_harness::HasProviderRegistry;
        bridge.set_provider_registry(self.provider_registry.clone());
        Arc::new(bridge)
    }
}

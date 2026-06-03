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
use axagent_harness::ToolRegistry as ToolRegistryTrait;
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
    /// 工具注册表（由调用方注入；通过 `set_tool_registry` / `tool_registry` 访问）
    tool_registry: Arc<Mutex<Option<Arc<dyn ToolRegistryTrait>>>>,
}

/// 构造 RuntimeHarness 时的依赖（持续扩展中）
pub struct HarnessDeps {
    pub persistence: Arc<dyn Persistence>,
    pub master_key: [u8; 32],
}

impl RuntimeHarness {
    /// 创建 Harness 容器
    ///
    /// 注意：内部使用默认 ProviderRegistry（来自 `axagent-providers`）。
    /// 想要完全自定义 ProviderRegistry 的调用方请用 `RuntimeHarness::with_registry`。
    pub fn new(deps: HarnessDeps) -> Self {
        let concrete_registry =
            Arc::new(axagent_providers::registry::ProviderRegistry::create_default());
        Self {
            persistence: deps.persistence,
            master_key: deps.master_key,
            provider_registry: concrete_registry as Arc<dyn ProviderRegistryTrait>,
            adapter_cache: Arc::new(Mutex::new(HashMap::new())),
            tool_registry: Arc::new(Mutex::new(None)),
        }
    }

    /// 创建一个可注入任意 ProviderRegistry 的 Harness（用于测试 / 嵌入）
    pub fn with_registry(deps: HarnessDeps, registry: Arc<dyn ProviderRegistryTrait>) -> Self {
        Self {
            persistence: deps.persistence,
            master_key: deps.master_key,
            provider_registry: registry,
            adapter_cache: Arc::new(Mutex::new(HashMap::new())),
            tool_registry: Arc::new(Mutex::new(None)),
        }
    }

    // ── ToolRegistry 注入点 ──────────────────────────────────

    /// 注入工具注册表（运行时由 init/state.rs 调用）
    pub async fn set_tool_registry(&self, registry: Arc<dyn ToolRegistryTrait>) {
        *self.tool_registry.lock().await = Some(registry);
    }

    /// 拿到工具注册表（如果未注入返回 None）
    pub async fn tool_registry(&self) -> Option<Arc<dyn ToolRegistryTrait>> {
        self.tool_registry.lock().await.clone()
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
        let bridge = axagent_rt_messaging::message_gateway::platform_bridge::PlatformBridge::new(
            self.persistence.connection().clone(),
            self.master_key,
            platform_manager,
        )
        .with_provider_registry(self.provider_registry.clone());
        Arc::new(bridge)
    }
}

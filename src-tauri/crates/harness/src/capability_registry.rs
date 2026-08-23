// SPDX-License-Identifier: AGPL-3.0-only

//! 运行时能力注册表 — 「一切皆插件」重构的核心接缝。
//!
//! 目标：让**内置实现**与**外部插件**以同一路径注册/检索可替换能力，
//! 对应 DeepSeek Harness 的「内置核心也是插件」（无特权核心）思想。
//!
//! 与 `capability.rs`（能力发现元数据 / 护照）互补但不同：
//! - `capability.rs`：面向检索/路由的**元数据描述**（[`crate::CapabilityPassport`]）
//! - 本模块：面向运行时的**实现注册与类型化检索**（[`CapabilityRegistry`]）
//!
//! 三件套对应 DeepSeek Harness 的 Capability Seam：
//! - **ServiceDefinition**：声明某个能力接缝的接口契约（id / 版本 / 契约路径 / 描述）
//! - **Provider**：实现该接缝的具体对象（类型擦除后以 `Arc<dyn Any + Send + Sync>` 持有）
//! - **Consumer**：通过 [`CapabilityRegistry::get_typed`] 按约束向下转型取回
//!
//! 所有注册都是可逆的（返回 [`crate::EffectHandle`]），支持运行期热插拔与隔离回滚。

#![allow(clippy::disallowed_types)]

use crate::agent_turn_runner::AgentTurnRunner;
use crate::provider::ProviderAdapter;
use crate::reversible_effect::{EffectHandle, EffectScope};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

/// 能力接缝的接口契约声明（ServiceDefinition）。
///
/// 只声明「*有* 这样一个可替换能力、其契约是什么」，不包含实现。
#[derive(Debug, Clone)]
pub struct ServiceDefinition {
    /// 全局唯一能力 ID（如 `"model.provider"`、`"agent.loop"`）。
    pub id: String,
    /// 契约版本（语义化）。
    pub version: String,
    /// 权威 trait 的完整路径（文档与调试用，如 `"axagent_harness::ProviderAdapter"`）。
    pub contract: String,
    /// 人类可读描述。
    pub description: String,
}

impl ServiceDefinition {
    /// 构造一个能力接缝声明。
    pub fn new(
        id: impl Into<String>,
        version: impl Into<String>,
        contract: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            version: version.into(),
            contract: contract.into(),
            description: description.into(),
        }
    }

    /// 预置 model-provider 接缝（LLM 适配器）。
    pub fn model_provider() -> Self {
        Self::new(
            "model.provider",
            "1.0",
            "axagent_harness::ProviderAdapter",
            "LLM 模型适配器：实现 ProviderAdapter，提供 chat/stream/embed 等模型能力",
        )
    }

    /// 预置 agent-loop 接缝（Agent 主循环）。
    pub fn agent_loop() -> Self {
        Self::new(
            "agent.loop",
            "1.0",
            "axagent_harness::AgentTurnRunner",
            "Agent 主循环：实现 AgentTurnRunner，驱动单轮或多轮 ReAct 执行",
        )
    }

    /// 预置 tool-set 接缝（工具集）。
    pub fn tool_set() -> Self {
        Self::new(
            "tool.set",
            "1.0",
            "axagent_harness::ToolRegistry",
            "工具集：实现 ToolRegistry，提供工具查找与统一执行",
        )
    }

    /// 预置 storage 接缝（对象存储后端）。
    pub fn storage() -> Self {
        Self::new(
            "storage.backend",
            "1.0",
            "axagent_harness::StorageBackend",
            "对象存储后端：实现 StorageBackend，提供 get/put/delete/list 等存储能力",
        )
    }

    /// 预置 sandbox 接缝（工作流沙箱）。
    pub fn sandbox() -> Self {
        Self::new(
            "workflow.sandbox",
            "1.0",
            "axagent_harness::WorkflowSandbox",
            "工作流沙箱：实现 WorkflowSandbox，隔离执行工作流基因并校验结果",
        )
    }

    /// 预置 workflow-reflector 接缝（工作流反思）。
    pub fn workflow_reflector() -> Self {
        Self::new(
            "workflow.reflector",
            "1.0",
            "axagent_harness::WorkflowReflector",
            "工作流反思：实现 WorkflowReflector，在工作流完成后复盘/沉淀模式",
        )
    }

    /// 预置 workflow-evolver 接缝（工作流进化）。
    pub fn workflow_evolver() -> Self {
        Self::new(
            "workflow.evolver",
            "1.0",
            "axagent_harness::WorkflowEvolver",
            "工作流进化：实现 WorkflowEvolver，驱动新一代工作流变异与验证",
        )
    }

    /// 预置 workflow-optimizer 接缝（工作流优化）。
    pub fn workflow_optimizer() -> Self {
        Self::new(
            "workflow.optimizer",
            "1.0",
            "axagent_harness::WorkflowOptimizer",
            "工作流优化：实现 WorkflowOptimizer，对执行记录给出优化建议",
        )
    }

    /// 预置 business-rule 接缝（业务规则评估）。
    pub fn business_rule() -> Self {
        Self::new(
            "workflow.business_rule",
            "1.0",
            "axagent_harness::BusinessRuleEvaluator",
            "业务规则评估：实现 BusinessRuleEvaluator，dispatch 前评估节点输入",
        )
    }

    /// 预置 message-callback 接缝（消息平台入站回调）。
    pub fn message_callback() -> Self {
        Self::new(
            "message.callback",
            "1.0",
            "axagent_harness::PlatformMessageCallback",
            "消息平台入站回调：实现 PlatformMessageCallback，统一处理平台消息并返回回复",
        )
    }

    /// 预置 webhook-dispatch 接缝（Webhook 事件派发）。
    pub fn webhook_dispatch() -> Self {
        Self::new(
            "webhook.dispatch",
            "1.0",
            "axagent_harness::WebhookDispatch",
            "Webhook 事件派发：实现 WebhookDispatch，把事件投递给订阅端点",
        )
    }

    /// 预置 event-dispatch 接缝（类型化事件派发总线，P2 事件化）。
    pub fn event_dispatch() -> Self {
        Self::new(
            "event.dispatch",
            "1.0",
            "axagent_harness::EventDispatchBus",
            "类型化事件派发总线：注册事件订阅者，支持 emit/waterfall/parallel/serial 四派发模式",
        )
    }

    /// 预置 session.log.invariant 接缝（会话日志不变量，缺陷 #3 05 项）。
    pub fn session_log_invariant() -> Self {
        Self::new(
            "session.log.invariant",
            "1.0",
            "axagent_harness::SessionLogInvariant",
            "会话日志不变量：记录模型可见内容并可重建成模型所见（Model-visible means logged）",
        )
    }

    /// 预置 platform-adapter 接缝（消息平台适配器，按平台名注册多实例）。
    pub fn platform_adapter(platform_name: &str) -> Self {
        Self::new(
            format!("platform.adapter.{platform_name}"),
            "1.0",
            "axagent_harness::MessagePlatformAdapter",
            format!("消息平台适配器：实现 MessagePlatformAdapter，接入 {platform_name} 平台"),
        )
    }
}

/// 能力来源 — 内置实现与外部插件平权的关键标记。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapabilityOrigin {
    /// 内置实现（第一方，随二进制分发）。
    BuiltIn,
    /// 外部插件（经 axagent-plugins 加载）。
    ExternalPlugin,
}

impl CapabilityOrigin {
    /// 来源的字符串表示。
    pub fn as_str(&self) -> &'static str {
        match self {
            CapabilityOrigin::BuiltIn => "builtin",
            CapabilityOrigin::ExternalPlugin => "external_plugin",
        }
    }
}

/// 外部插件声明的能力描述（声明式，P3 外部插件注册）。
///
/// 外部插件以 shell 脚本分发，无法跨进程提供 Rust trait 对象，因此在插件
/// 启用时以「声明描述」形式注册进能力注册表（[`CapabilityOrigin::ExternalPlugin`]），
/// 供检视 / 路由 / 编排使用；禁用或卸载时经 [`EffectHandle::undo`] 可逆回滚。
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PluginCapabilityDescriptor {
    /// 能力接缝 ID（如 `"platform.adapter.telegram"`、`"tool.set.myplugin"`）。
    pub seam_id: String,
    /// 来源插件 ID。
    pub plugin_id: String,
    /// 能力类型标识（如 `"platform_adapter"`、`"tool_set"`）。
    pub capability_type: String,
    /// 契约版本（语义化）。
    pub version: String,
    /// 人类可读描述。
    pub description: String,
    /// 能力配置快照（插件声明的可选配置，JSON）。
    pub config: serde_json::Value,
}

impl PluginCapabilityDescriptor {
    /// 构造一个插件能力描述。
    pub fn new(
        seam_id: impl Into<String>,
        plugin_id: impl Into<String>,
        capability_type: impl Into<String>,
        version: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        Self {
            seam_id: seam_id.into(),
            plugin_id: plugin_id.into(),
            capability_type: capability_type.into(),
            version: version.into(),
            description: description.into(),
            config: serde_json::Value::Null,
        }
    }
}

/// 能力注册错误。
#[derive(Debug, thiserror::Error)]
pub enum CapabilityError {
    /// 能力 ID 已被占用。
    #[error("capability `{id}` already registered")]
    Duplicate { id: String },
    /// 能力不存在。
    #[error("capability `{id}` not found")]
    NotFound { id: String },
    /// 类型不匹配（向下转型失败）。
    #[error("capability `{id}` type mismatch: expected {expected}")]
    TypeMismatch { id: String, expected: &'static str },
}

/// 一条已注册的能力（定义 + 来源 + 类型擦除后的 Provider）。
struct CapabilityRegistration {
    definition: ServiceDefinition,
    origin: CapabilityOrigin,
    provider: Arc<dyn std::any::Any + Send + Sync>,
}

/// 运行时能力注册表 — 内置与外部插件统一的注册 / 类型化检索入口。
///
/// 线程安全：内部由 `RwLock<HashMap>` 保护；每条注册都是可逆的，
/// 可通过 [`CapabilityRegistry::rollback_all`] 或单条句柄撤销。
#[derive(Clone)]
pub struct CapabilityRegistry {
    inner: Arc<RwLock<HashMap<String, CapabilityRegistration>>>,
    /// ProviderAdapter 特化存储 — 用于支持 trait object（`Arc<dyn ProviderAdapter>`）检索。
    ///
    /// `Arc<dyn Any>` 无法向下转型回 `dyn ProviderAdapter`（`Arc::downcast` 要求 `Sized`），
    /// 因此对 LLM 适配器这类需要以 trait object 取回的能力做类型化旁路存储。
    model_providers: Arc<RwLock<HashMap<String, Arc<dyn ProviderAdapter>>>>,
    /// AgentTurnRunner 特化存储 — 用于支持 agent-loop 接缝以 trait object 检索。
    ///
    /// 与 `model_providers` 同理，`Arc<dyn AgentTurnRunner>` 无法经 `Arc::downcast`
    /// 从类型擦除存储还原，故对 Agent 主循环做类型化旁路存储。
    agent_turn_runners: Arc<RwLock<HashMap<String, Arc<dyn AgentTurnRunner>>>>,
    /// WorkflowReflector 特化存储 — 用于支持 workflow-reflector 接缝以 trait object 检索。
    workflow_reflectors: Arc<RwLock<HashMap<String, Arc<dyn crate::WorkflowReflector>>>>,
    /// WorkflowEvolver 特化存储 — 用于支持 workflow-evolver 接缝以 trait object 检索。
    workflow_evolvers: Arc<RwLock<HashMap<String, Arc<dyn crate::WorkflowEvolver>>>>,
    /// WorkflowOptimizer 特化存储 — 用于支持 workflow-optimizer 接缝以 trait object 检索。
    workflow_optimizers: Arc<RwLock<HashMap<String, Arc<dyn crate::WorkflowOptimizer>>>>,
    /// BusinessRuleEvaluator 特化存储 — 用于支持 business-rule 接缝以 trait object 检索。
    business_rules: Arc<RwLock<HashMap<String, Arc<dyn crate::BusinessRuleEvaluator>>>>,
    /// PlatformMessageCallback 特化存储 — 用于支持 message.callback 接缝以 trait object 检索。
    message_callbacks: Arc<RwLock<HashMap<String, Arc<dyn crate::PlatformMessageCallback>>>>,
    /// WebhookDispatch 特化存储 — 用于支持 webhook.dispatch 接缝以 trait object 检索。
    webhook_dispatchers: Arc<RwLock<HashMap<String, Arc<dyn crate::WebhookDispatch>>>>,
    /// event.dispatch 特化存储 — 类型化事件派发总线（单例）。
    event_dispatchers: Arc<RwLock<HashMap<String, Arc<crate::EventDispatchBus>>>>,
    /// MessagePlatformAdapter 特化存储 — 用于支持 platform.adapter 接缝以 trait object 检索。
    /// 键 = 完整注册 ID（`platform.adapter.{name}`），与通用存储一致，便于回滚清理。
    platform_adapters: Arc<RwLock<HashMap<String, Arc<dyn crate::MessagePlatformAdapter>>>>,
    /// StorageBackend 特化存储 — 用于支持 storage 接缝以 trait object 检索。
    storage_backends: Arc<RwLock<HashMap<String, Arc<dyn crate::StorageBackend>>>>,
    /// WorkflowSandbox 特化存储 — 用于支持 sandbox 接缝以 trait object 检索。
    sandboxes: Arc<RwLock<HashMap<String, Arc<dyn crate::WorkflowSandbox>>>>,
    /// SessionLogInvariant 特化存储 — 用于支持 session.log.invariant 接缝（单例）。
    session_logs: Arc<RwLock<HashMap<String, Arc<dyn crate::SessionLogInvariant>>>>,
    /// 内置接缝注册句柄 — 记录 BuiltIn 注册返回的 EffectHandle，供外部插件经
    /// `register_external_*` 撤销内置、实现运行时替换/回滚（缺陷 #7）。
    builtin_handles: Arc<RwLock<HashMap<String, EffectHandle>>>,
    effects: EffectScope,
}

impl CapabilityRegistry {
    /// 创建空能力注册表。
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(HashMap::new())),
            model_providers: Arc::new(RwLock::new(HashMap::new())),
            agent_turn_runners: Arc::new(RwLock::new(HashMap::new())),
            workflow_reflectors: Arc::new(RwLock::new(HashMap::new())),
            workflow_evolvers: Arc::new(RwLock::new(HashMap::new())),
            workflow_optimizers: Arc::new(RwLock::new(HashMap::new())),
            business_rules: Arc::new(RwLock::new(HashMap::new())),
            message_callbacks: Arc::new(RwLock::new(HashMap::new())),
            webhook_dispatchers: Arc::new(RwLock::new(HashMap::new())),
            event_dispatchers: Arc::new(RwLock::new(HashMap::new())),
            platform_adapters: Arc::new(RwLock::new(HashMap::new())),
            storage_backends: Arc::new(RwLock::new(HashMap::new())),
            sandboxes: Arc::new(RwLock::new(HashMap::new())),
            session_logs: Arc::new(RwLock::new(HashMap::new())),
            builtin_handles: Arc::new(RwLock::new(HashMap::new())),
            effects: EffectScope::new(),
        }
    }

    /// 记录一条内置注册句柄，供外部插件替换该接缝时撤销。
    fn note_builtin_handle(&self, id: &str, handle: EffectHandle) {
        self.builtin_handles.write().insert(id.to_string(), handle);
    }

    /// 撤销并移除同 id 的内置句柄（外部插件替换内置的前置动作）。
    fn evict_builtin_for_external(&self, id: &str) {
        if let Some(h) = self.builtin_handles.write().remove(id) {
            h.undo();
        }
    }

    /// 注册前预处理：外部插件先撤销同键内置句柄（实现「外部替换内置」的平权语义），
    /// 再检查通用存储是否仍有占位（避免外部插件互相覆盖）。
    fn prepare_registration(
        &self,
        id: &str,
        origin: CapabilityOrigin,
    ) -> Result<(), CapabilityError> {
        if origin == CapabilityOrigin::ExternalPlugin {
            self.evict_builtin_for_external(id);
        }
        if self.inner.read().contains_key(id) {
            return Err(CapabilityError::Duplicate { id: id.to_string() });
        }
        Ok(())
    }

    /// 注册一个类型擦除的 Provider（通用入口）。
    ///
    /// 返回可单独撤销的句柄；重复注册同一 ID 返回 [`CapabilityError::Duplicate`]。
    pub fn register(
        &self,
        definition: ServiceDefinition,
        origin: CapabilityOrigin,
        provider: Arc<dyn std::any::Any + Send + Sync>,
    ) -> Result<EffectHandle, CapabilityError> {
        let id = definition.id.clone();
        let undo_id = id.clone();
        self.prepare_registration(&id, origin)?;
        let mut slot = self.inner.write();
        slot.insert(id.clone(), CapabilityRegistration { definition, origin, provider });
        drop(slot);

        // 注册一个可逆效果：撤销时从注册表移除该能力。
        let this = self.clone();
        let undo = this.inner.clone();
        let handle = self.effects.register(format!("capability:{id}"), move || {
            let mut guard = undo.write();
            guard.remove(&undo_id);
        });
        if origin == CapabilityOrigin::BuiltIn {
            self.note_builtin_handle(&id, handle.clone());
        }
        Ok(handle)
    }

    /// 注册一个内置 LLM 模型适配器（P1 试点接缝）。
    ///
    /// 以内置来源注册到 `"model.provider"` 接缝，consumers 可通过
    /// [`CapabilityRegistry::get_provider_adapter`] 取回。
    pub fn register_model_provider(
        &self,
        provider: Arc<dyn ProviderAdapter>,
    ) -> Result<EffectHandle, CapabilityError> {
        self.register_model_provider_impl(
            ServiceDefinition::model_provider(),
            CapabilityOrigin::BuiltIn,
            provider,
            false,
        )
    }

    /// 注册一个外部插件提供的 LLM 模型适配器。
    pub fn register_external_model_provider(
        &self,
        provider: Arc<dyn ProviderAdapter>,
    ) -> Result<EffectHandle, CapabilityError> {
        self.register_model_provider_impl(
            ServiceDefinition::model_provider(),
            CapabilityOrigin::ExternalPlugin,
            provider,
            false,
        )
    }

    /// 模型提供商注册的共享实现：同时写入通用 Any 存储与 ProviderAdapter 特化存储。
    ///
    /// `idempotent` 为 true 时（内置按类型注册路径），同键且同为 BuiltIn 的重复
    /// 注册静默跳过（返回空句柄），避免 `create_default` 被多处调用时重复注入告警
    /// （缺陷 #10：副作用式全局注入收敛为幂等注册）。
    fn register_model_provider_impl(
        &self,
        definition: ServiceDefinition,
        origin: CapabilityOrigin,
        provider: Arc<dyn ProviderAdapter>,
        idempotent: bool,
    ) -> Result<EffectHandle, CapabilityError> {
        let id = definition.id.clone();
        if idempotent && origin == CapabilityOrigin::BuiltIn {
            let is_dup_builtin =
                self.inner.read().get(&id).is_some_and(|r| r.origin == CapabilityOrigin::BuiltIn);
            if is_dup_builtin {
                // 同键内置适配器已注册：幂等跳过，撤销亦为空操作。
                return Ok(self
                    .effects
                    .register(format!("capability:{id}:model-provider:noop"), || {}));
            }
        }
        // 通用侧（检视 / contains / len 用）
        {
            self.prepare_registration(&id, origin)?;
            let mut slot = self.inner.write();
            slot.insert(
                id.clone(),
                CapabilityRegistration { definition, origin, provider: provider.clone() },
            );
        }
        // 特化侧（trait object 检索用）
        self.model_providers.write().insert(id.clone(), provider);

        // 合并可逆效果：撤销时同时清理两处存储。
        let undo_inner = self.inner.clone();
        let undo_mp = self.model_providers.clone();
        let undo_id = id.clone();
        let handle = self.effects.register(format!("capability:{id}:model-provider"), move || {
            undo_inner.write().remove(&undo_id);
            undo_mp.write().remove(&undo_id);
        });
        if origin == CapabilityOrigin::BuiltIn {
            self.note_builtin_handle(&id, handle.clone());
        }
        Ok(handle)
    }

    /// 注册一个按类型索引的内置 LLM 模型适配器（`model.provider.{provider_type}`）。
    ///
    /// 与 [`CapabilityRegistry::register_model_provider`]（默认单键接缝）不同，
    /// 此方法以内置来源把适配器注册到 `model.provider.{provider_type}` 特化键，
    /// 供 [`CapabilityRegistry::get_provider_adapter_for`] 检索。内置 provider 注册
    /// 走此路径，使「注册」与「消费」落在同一张特化 map，避免「注册即检索失败」。
    /// 幂等：同键内置重复注册静默跳过（缺陷 #10），外部调用或多次 create_default
    /// 均不会产生 Duplicate 告警。
    pub fn register_model_provider_for_type(
        &self,
        provider_type: &str,
        provider: Arc<dyn ProviderAdapter>,
    ) -> Result<EffectHandle, CapabilityError> {
        let id = format!("model.provider.{provider_type}");
        let def = ServiceDefinition::new(
            &id,
            "1.0",
            "axagent_harness::ProviderAdapter",
            format!("内置 LLM 提供商适配器：{provider_type}"),
        );
        self.register_model_provider_impl(def, CapabilityOrigin::BuiltIn, provider, true)
    }

    /// 取回按类型索引的 LLM 模型适配器（`model.provider.{provider_type}`）。
    pub fn get_provider_adapter_for(
        &self,
        provider_type: &str,
    ) -> Option<Arc<dyn ProviderAdapter>> {
        let id = format!("model.provider.{provider_type}");
        self.model_providers.read().get(&id).cloned()
    }

    pub fn register_plugin_capability(
        &self,
        descriptor: PluginCapabilityDescriptor,
    ) -> Result<EffectHandle, CapabilityError> {
        let def = ServiceDefinition::new(
            &descriptor.seam_id,
            &descriptor.version,
            "axagent_harness::PluginCapabilityDescriptor",
            &descriptor.description,
        );
        self.register(def, CapabilityOrigin::ExternalPlugin, Arc::new(descriptor))
    }

    /// 注销一个能力（直接移除，不走可逆效果）。
    pub fn unregister(&self, id: &str) -> Result<(), CapabilityError> {
        let mut slot = self.inner.write();
        if slot.remove(id).is_none() {
            return Err(CapabilityError::NotFound { id: id.to_string() });
        }
        self.model_providers.write().remove(id);
        self.agent_turn_runners.write().remove(id);
        self.workflow_reflectors.write().remove(id);
        self.workflow_evolvers.write().remove(id);
        self.workflow_optimizers.write().remove(id);
        self.business_rules.write().remove(id);
        self.message_callbacks.write().remove(id);
        self.webhook_dispatchers.write().remove(id);
        self.event_dispatchers.write().remove(id);
        self.platform_adapters.write().remove(id);
        self.storage_backends.write().remove(id);
        self.sandboxes.write().remove(id);
        Ok(())
    }

    /// 按 ID 取回类型擦除的 Provider。
    pub fn get(&self, id: &str) -> Option<Arc<dyn std::any::Any + Send + Sync>> {
        self.inner.read().get(id).map(|r| r.provider.clone())
    }

    /// 按 ID + 类型约束向下转型取回 Provider（Consumer 入口）。
    ///
    /// `T` 必须是具体类型（`Sized`），如 `Arc<String>`；需要取回 trait 对象
    /// （如 `dyn ProviderAdapter`）时，请使用对应的特化入口
    /// [`CapabilityRegistry::get_provider_adapter`]。
    pub fn get_typed<T: std::any::Any + Send + Sync>(&self, id: &str) -> Option<Arc<T>> {
        self.get(id)?.downcast::<T>().ok()
    }

    /// 取回 model-provider 接缝上的 LLM 适配器（若已注册）。
    pub fn get_provider_adapter(&self) -> Option<Arc<dyn ProviderAdapter>> {
        self.model_providers.read().get("model.provider").cloned()
    }

    /// 注册一个内置 Agent 主循环（P1 试点接缝 `agent.loop`）。
    ///
    /// 以内置来源注册，consumers 可通过 [`CapabilityRegistry::get_agent_turn_runner`] 取回。
    pub fn register_agent_loop(
        &self,
        runner: Arc<dyn AgentTurnRunner>,
    ) -> Result<EffectHandle, CapabilityError> {
        self.register_agent_loop_impl(
            ServiceDefinition::agent_loop(),
            CapabilityOrigin::BuiltIn,
            runner,
        )
    }

    /// 注册一个外部插件提供的 Agent 主循环。
    pub fn register_external_agent_loop(
        &self,
        runner: Arc<dyn AgentTurnRunner>,
    ) -> Result<EffectHandle, CapabilityError> {
        self.register_agent_loop_impl(
            ServiceDefinition::agent_loop(),
            CapabilityOrigin::ExternalPlugin,
            runner,
        )
    }

    /// Agent 主循环注册的共享实现：同时写入通用 Any 存储与 AgentTurnRunner 特化存储。
    fn register_agent_loop_impl(
        &self,
        definition: ServiceDefinition,
        origin: CapabilityOrigin,
        runner: Arc<dyn AgentTurnRunner>,
    ) -> Result<EffectHandle, CapabilityError> {
        let id = definition.id.clone();
        // 通用侧（检视 / contains / len 用）
        {
            self.prepare_registration(&id, origin)?;
            let mut slot = self.inner.write();
            slot.insert(
                id.clone(),
                CapabilityRegistration { definition, origin, provider: runner.clone() },
            );
        }
        // 特化侧（trait object 检索用）
        self.agent_turn_runners.write().insert(id.clone(), runner);

        // 合并可逆效果：撤销时同时清理两处存储。
        let undo_inner = self.inner.clone();
        let undo_atr = self.agent_turn_runners.clone();
        let undo_id = id.clone();
        let handle = self.effects.register(format!("capability:{id}:agent-loop"), move || {
            undo_inner.write().remove(&undo_id);
            undo_atr.write().remove(&undo_id);
        });
        if origin == CapabilityOrigin::BuiltIn {
            self.note_builtin_handle(&id, handle.clone());
        }
        Ok(handle)
    }

    /// 取回 agent-loop 接缝上的 Agent 主循环（若已注册）。
    pub fn get_agent_turn_runner(&self) -> Option<Arc<dyn AgentTurnRunner>> {
        self.agent_turn_runners.read().get("agent.loop").cloned()
    }

    // ── storage 接缝 ────────────────────────────────────────────────────────

    /// 注册一个内置 StorageBackend（P1 storage 接缝）。
    pub fn register_storage(
        &self,
        backend: Arc<dyn crate::StorageBackend>,
    ) -> Result<EffectHandle, CapabilityError> {
        self.register_storage_impl(ServiceDefinition::storage(), CapabilityOrigin::BuiltIn, backend)
    }

    /// 注册一个外部插件提供的 StorageBackend。
    pub fn register_external_storage(
        &self,
        backend: Arc<dyn crate::StorageBackend>,
    ) -> Result<EffectHandle, CapabilityError> {
        self.register_storage_impl(
            ServiceDefinition::storage(),
            CapabilityOrigin::ExternalPlugin,
            backend,
        )
    }

    fn register_storage_impl(
        &self,
        definition: ServiceDefinition,
        origin: CapabilityOrigin,
        backend: Arc<dyn crate::StorageBackend>,
    ) -> Result<EffectHandle, CapabilityError> {
        let id = definition.id.clone();
        {
            self.prepare_registration(&id, origin)?;
            let mut slot = self.inner.write();
            slot.insert(
                id.clone(),
                CapabilityRegistration { definition, origin, provider: backend.clone() },
            );
        }
        self.storage_backends.write().insert(id.clone(), backend);

        let undo_inner = self.inner.clone();
        let undo_storage = self.storage_backends.clone();
        let undo_id = id.clone();
        let handle = self.effects.register(format!("capability:{id}:storage"), move || {
            undo_inner.write().remove(&undo_id);
            undo_storage.write().remove(&undo_id);
        });
        if origin == CapabilityOrigin::BuiltIn {
            self.note_builtin_handle(&id, handle.clone());
        }
        Ok(handle)
    }

    /// 取回 storage 接缝上的对象存储后端（若已注册）。
    pub fn get_storage(&self) -> Option<Arc<dyn crate::StorageBackend>> {
        self.storage_backends.read().get("storage.backend").cloned()
    }

    // ── sandbox 接缝 ────────────────────────────────────────────────────────

    /// 注册一个内置 WorkflowSandbox（P1 sandbox 接缝）。
    pub fn register_sandbox(
        &self,
        sandbox: Arc<dyn crate::WorkflowSandbox>,
    ) -> Result<EffectHandle, CapabilityError> {
        self.register_sandbox_impl(ServiceDefinition::sandbox(), CapabilityOrigin::BuiltIn, sandbox)
    }

    /// 注册一个外部插件提供的 WorkflowSandbox。
    pub fn register_external_sandbox(
        &self,
        sandbox: Arc<dyn crate::WorkflowSandbox>,
    ) -> Result<EffectHandle, CapabilityError> {
        self.register_sandbox_impl(
            ServiceDefinition::sandbox(),
            CapabilityOrigin::ExternalPlugin,
            sandbox,
        )
    }

    fn register_sandbox_impl(
        &self,
        definition: ServiceDefinition,
        origin: CapabilityOrigin,
        sandbox: Arc<dyn crate::WorkflowSandbox>,
    ) -> Result<EffectHandle, CapabilityError> {
        let id = definition.id.clone();
        {
            self.prepare_registration(&id, origin)?;
            let mut slot = self.inner.write();
            slot.insert(
                id.clone(),
                CapabilityRegistration { definition, origin, provider: sandbox.clone() },
            );
        }
        self.sandboxes.write().insert(id.clone(), sandbox);

        let undo_inner = self.inner.clone();
        let undo_sandbox = self.sandboxes.clone();
        let undo_id = id.clone();
        let handle = self.effects.register(format!("capability:{id}:sandbox"), move || {
            undo_inner.write().remove(&undo_id);
            undo_sandbox.write().remove(&undo_id);
        });
        if origin == CapabilityOrigin::BuiltIn {
            self.note_builtin_handle(&id, handle.clone());
        }
        Ok(handle)
    }

    /// 取回 sandbox 接缝上的工作流沙箱（若已注册）。
    pub fn get_sandbox(&self) -> Option<Arc<dyn crate::WorkflowSandbox>> {
        self.sandboxes.read().get("workflow.sandbox").cloned()
    }

    // ── workflow-reflector 接缝 ─────────────────────────────────────────────

    /// 注册一个内置 WorkflowReflector（P2 workflow-reflector 接缝）。
    pub fn register_workflow_reflector(
        &self,
        reflector: Arc<dyn crate::WorkflowReflector>,
    ) -> Result<EffectHandle, CapabilityError> {
        self.register_workflow_reflector_impl(
            ServiceDefinition::workflow_reflector(),
            CapabilityOrigin::BuiltIn,
            reflector,
        )
    }

    /// 注册一个外部插件提供的 WorkflowReflector。
    pub fn register_external_workflow_reflector(
        &self,
        reflector: Arc<dyn crate::WorkflowReflector>,
    ) -> Result<EffectHandle, CapabilityError> {
        self.register_workflow_reflector_impl(
            ServiceDefinition::workflow_reflector(),
            CapabilityOrigin::ExternalPlugin,
            reflector,
        )
    }

    fn register_workflow_reflector_impl(
        &self,
        definition: ServiceDefinition,
        origin: CapabilityOrigin,
        reflector: Arc<dyn crate::WorkflowReflector>,
    ) -> Result<EffectHandle, CapabilityError> {
        let id = definition.id.clone();
        {
            self.prepare_registration(&id, origin)?;
            let mut slot = self.inner.write();
            slot.insert(
                id.clone(),
                CapabilityRegistration { definition, origin, provider: reflector.clone() },
            );
        }
        self.workflow_reflectors.write().insert(id.clone(), reflector);
        let handle =
            self.register_rollback(&id, origin, "workflow-reflector", &self.workflow_reflectors);
        Ok(handle)
    }

    /// 取回 workflow-reflector 接缝上的反思实现（若已注册）。
    pub fn get_workflow_reflector(&self) -> Option<Arc<dyn crate::WorkflowReflector>> {
        self.workflow_reflectors.read().get("workflow.reflector").cloned()
    }

    // ── workflow-evolver 接缝 ───────────────────────────────────────────────

    /// 注册一个内置 WorkflowEvolver（P2 workflow-evolver 接缝）。
    pub fn register_workflow_evolver(
        &self,
        evolver: Arc<dyn crate::WorkflowEvolver>,
    ) -> Result<EffectHandle, CapabilityError> {
        self.register_workflow_evolver_impl(
            ServiceDefinition::workflow_evolver(),
            CapabilityOrigin::BuiltIn,
            evolver,
        )
    }

    /// 注册一个外部插件提供的 WorkflowEvolver。
    pub fn register_external_workflow_evolver(
        &self,
        evolver: Arc<dyn crate::WorkflowEvolver>,
    ) -> Result<EffectHandle, CapabilityError> {
        self.register_workflow_evolver_impl(
            ServiceDefinition::workflow_evolver(),
            CapabilityOrigin::ExternalPlugin,
            evolver,
        )
    }

    fn register_workflow_evolver_impl(
        &self,
        definition: ServiceDefinition,
        origin: CapabilityOrigin,
        evolver: Arc<dyn crate::WorkflowEvolver>,
    ) -> Result<EffectHandle, CapabilityError> {
        let id = definition.id.clone();
        {
            self.prepare_registration(&id, origin)?;
            let mut slot = self.inner.write();
            slot.insert(
                id.clone(),
                CapabilityRegistration { definition, origin, provider: evolver.clone() },
            );
        }
        self.workflow_evolvers.write().insert(id.clone(), evolver);
        let handle =
            self.register_rollback(&id, origin, "workflow-evolver", &self.workflow_evolvers);
        Ok(handle)
    }

    /// 取回 workflow-evolver 接缝上的进化实现（若已注册）。
    pub fn get_workflow_evolver(&self) -> Option<Arc<dyn crate::WorkflowEvolver>> {
        self.workflow_evolvers.read().get("workflow.evolver").cloned()
    }

    // ── workflow-optimizer 接缝 ─────────────────────────────────────────────

    /// 注册一个内置 WorkflowOptimizer（P2 workflow-optimizer 接缝）。
    pub fn register_workflow_optimizer(
        &self,
        optimizer: Arc<dyn crate::WorkflowOptimizer>,
    ) -> Result<EffectHandle, CapabilityError> {
        self.register_workflow_optimizer_impl(
            ServiceDefinition::workflow_optimizer(),
            CapabilityOrigin::BuiltIn,
            optimizer,
        )
    }

    /// 注册一个外部插件提供的 WorkflowOptimizer。
    pub fn register_external_workflow_optimizer(
        &self,
        optimizer: Arc<dyn crate::WorkflowOptimizer>,
    ) -> Result<EffectHandle, CapabilityError> {
        self.register_workflow_optimizer_impl(
            ServiceDefinition::workflow_optimizer(),
            CapabilityOrigin::ExternalPlugin,
            optimizer,
        )
    }

    fn register_workflow_optimizer_impl(
        &self,
        definition: ServiceDefinition,
        origin: CapabilityOrigin,
        optimizer: Arc<dyn crate::WorkflowOptimizer>,
    ) -> Result<EffectHandle, CapabilityError> {
        let id = definition.id.clone();
        {
            self.prepare_registration(&id, origin)?;
            let mut slot = self.inner.write();
            slot.insert(
                id.clone(),
                CapabilityRegistration { definition, origin, provider: optimizer.clone() },
            );
        }
        self.workflow_optimizers.write().insert(id.clone(), optimizer);
        let handle =
            self.register_rollback(&id, origin, "workflow-optimizer", &self.workflow_optimizers);
        Ok(handle)
    }

    /// 取回 workflow-optimizer 接缝上的优化实现（若已注册）。
    pub fn get_workflow_optimizer(&self) -> Option<Arc<dyn crate::WorkflowOptimizer>> {
        self.workflow_optimizers.read().get("workflow.optimizer").cloned()
    }

    // ── business-rule 接缝 ──────────────────────────────────────────────────

    /// 注册一个内置 BusinessRuleEvaluator（P2 workflow.business_rule 接缝）。
    pub fn register_business_rule(
        &self,
        evaluator: Arc<dyn crate::BusinessRuleEvaluator>,
    ) -> Result<EffectHandle, CapabilityError> {
        self.register_business_rule_impl(
            ServiceDefinition::business_rule(),
            CapabilityOrigin::BuiltIn,
            evaluator,
        )
    }

    /// 注册一个外部插件提供的 BusinessRuleEvaluator。
    pub fn register_external_business_rule(
        &self,
        evaluator: Arc<dyn crate::BusinessRuleEvaluator>,
    ) -> Result<EffectHandle, CapabilityError> {
        self.register_business_rule_impl(
            ServiceDefinition::business_rule(),
            CapabilityOrigin::ExternalPlugin,
            evaluator,
        )
    }

    fn register_business_rule_impl(
        &self,
        definition: ServiceDefinition,
        origin: CapabilityOrigin,
        evaluator: Arc<dyn crate::BusinessRuleEvaluator>,
    ) -> Result<EffectHandle, CapabilityError> {
        let id = definition.id.clone();
        {
            self.prepare_registration(&id, origin)?;
            let mut slot = self.inner.write();
            slot.insert(
                id.clone(),
                CapabilityRegistration { definition, origin, provider: evaluator.clone() },
            );
        }
        self.business_rules.write().insert(id.clone(), evaluator);
        let handle = self.register_rollback(&id, origin, "business-rule", &self.business_rules);
        Ok(handle)
    }

    /// 取回 workflow.business_rule 接缝上的规则评估器（若已注册）。
    pub fn get_business_rule(&self) -> Option<Arc<dyn crate::BusinessRuleEvaluator>> {
        self.business_rules.read().get("workflow.business_rule").cloned()
    }

    // ── message.callback 接缝 ───────────────────────────────────────────────

    /// 注册一个内置 PlatformMessageCallback（message.callback 接缝）。
    ///
    /// consumers 可通过 [`CapabilityRegistry::get_message_callback`] 取回。
    pub fn register_message_callback(
        &self,
        callback: Arc<dyn crate::PlatformMessageCallback>,
    ) -> Result<EffectHandle, CapabilityError> {
        self.register_message_callback_impl(
            ServiceDefinition::message_callback(),
            CapabilityOrigin::BuiltIn,
            callback,
        )
    }

    /// 注册一个外部插件提供的 PlatformMessageCallback。
    pub fn register_external_message_callback(
        &self,
        callback: Arc<dyn crate::PlatformMessageCallback>,
    ) -> Result<EffectHandle, CapabilityError> {
        self.register_message_callback_impl(
            ServiceDefinition::message_callback(),
            CapabilityOrigin::ExternalPlugin,
            callback,
        )
    }

    /// message.callback 注册的共享实现：同时写入通用 Any 存储与特化存储。
    fn register_message_callback_impl(
        &self,
        definition: ServiceDefinition,
        origin: CapabilityOrigin,
        callback: Arc<dyn crate::PlatformMessageCallback>,
    ) -> Result<EffectHandle, CapabilityError> {
        let id = definition.id.clone();
        {
            self.prepare_registration(&id, origin)?;
            let mut slot = self.inner.write();
            slot.insert(
                id.clone(),
                CapabilityRegistration { definition, origin, provider: callback.clone() },
            );
        }
        self.message_callbacks.write().insert(id.clone(), callback);
        Ok(self.register_rollback(&id, origin, "message-callback", &self.message_callbacks))
    }

    /// 取回 message.callback 接缝上的消息回调（若已注册）。
    pub fn get_message_callback(&self) -> Option<Arc<dyn crate::PlatformMessageCallback>> {
        self.message_callbacks.read().get("message.callback").cloned()
    }

    // ── webhook.dispatch 接缝 ───────────────────────────────────────────────

    /// 注册一个内置 WebhookDispatch（webhook.dispatch 接缝）。
    ///
    /// consumers 可通过 [`CapabilityRegistry::get_webhook_dispatch`] 取回。
    pub fn register_webhook_dispatch(
        &self,
        dispatcher: Arc<dyn crate::WebhookDispatch>,
    ) -> Result<EffectHandle, CapabilityError> {
        self.register_webhook_dispatch_impl(
            ServiceDefinition::webhook_dispatch(),
            CapabilityOrigin::BuiltIn,
            dispatcher,
        )
    }

    /// 注册一个外部插件提供的 WebhookDispatch。
    pub fn register_external_webhook_dispatch(
        &self,
        dispatcher: Arc<dyn crate::WebhookDispatch>,
    ) -> Result<EffectHandle, CapabilityError> {
        self.register_webhook_dispatch_impl(
            ServiceDefinition::webhook_dispatch(),
            CapabilityOrigin::ExternalPlugin,
            dispatcher,
        )
    }

    /// webhook.dispatch 注册的共享实现：同时写入通用 Any 存储与特化存储。
    fn register_webhook_dispatch_impl(
        &self,
        definition: ServiceDefinition,
        origin: CapabilityOrigin,
        dispatcher: Arc<dyn crate::WebhookDispatch>,
    ) -> Result<EffectHandle, CapabilityError> {
        let id = definition.id.clone();
        {
            self.prepare_registration(&id, origin)?;
            let mut slot = self.inner.write();
            slot.insert(
                id.clone(),
                CapabilityRegistration { definition, origin, provider: dispatcher.clone() },
            );
        }
        self.webhook_dispatchers.write().insert(id.clone(), dispatcher);
        Ok(self.register_rollback(&id, origin, "webhook-dispatch", &self.webhook_dispatchers))
    }

    /// 取回 webhook.dispatch 接缝上的派发器（若已注册）。
    pub fn get_webhook_dispatch(&self) -> Option<Arc<dyn crate::WebhookDispatch>> {
        self.webhook_dispatchers.read().get("webhook.dispatch").cloned()
    }

    // ── event.dispatch 接缝（单例类型化事件派发总线） ──────────────────────

    /// 注册内置类型化事件派发总线（event.dispatch）。
    pub fn register_event_dispatcher(
        &self,
        dispatcher: Arc<crate::EventDispatchBus>,
    ) -> Result<EffectHandle, CapabilityError> {
        self.register_event_dispatcher_impl(
            ServiceDefinition::event_dispatch(),
            CapabilityOrigin::BuiltIn,
            dispatcher,
        )
    }

    /// 注册外部插件提供的类型化事件派发总线。
    pub fn register_external_event_dispatcher(
        &self,
        dispatcher: Arc<crate::EventDispatchBus>,
    ) -> Result<EffectHandle, CapabilityError> {
        self.register_event_dispatcher_impl(
            ServiceDefinition::event_dispatch(),
            CapabilityOrigin::ExternalPlugin,
            dispatcher,
        )
    }

    /// event.dispatch 注册的共享实现：同时写入通用 Any 存储与特化存储。
    fn register_event_dispatcher_impl(
        &self,
        definition: ServiceDefinition,
        origin: CapabilityOrigin,
        dispatcher: Arc<crate::EventDispatchBus>,
    ) -> Result<EffectHandle, CapabilityError> {
        let id = definition.id.clone();
        {
            self.prepare_registration(&id, origin)?;
            let mut slot = self.inner.write();
            slot.insert(
                id.clone(),
                CapabilityRegistration { definition, origin, provider: dispatcher.clone() },
            );
        }
        self.event_dispatchers.write().insert(id.clone(), dispatcher);
        Ok(self.register_rollback(&id, origin, "event-dispatch", &self.event_dispatchers))
    }

    /// 取回 event.dispatch 接缝上的类型化事件派发总线（若已注册）。
    pub fn get_event_dispatcher(&self) -> Option<Arc<crate::EventDispatchBus>> {
        self.event_dispatchers.read().get("event.dispatch").cloned()
    }

    // ── session.log.invariant 接缝（单例会话日志不变量） ─────────────────────

    /// 注册内置会话日志不变量（session.log.invariant）。
    pub fn register_session_log_invariant(
        &self,
        log: Arc<dyn crate::SessionLogInvariant>,
    ) -> Result<EffectHandle, CapabilityError> {
        self.register_session_log_invariant_impl(
            ServiceDefinition::session_log_invariant(),
            CapabilityOrigin::BuiltIn,
            log,
        )
    }

    /// 注册外部插件提供的会话日志不变量。
    pub fn register_external_session_log_invariant(
        &self,
        log: Arc<dyn crate::SessionLogInvariant>,
    ) -> Result<EffectHandle, CapabilityError> {
        self.register_session_log_invariant_impl(
            ServiceDefinition::session_log_invariant(),
            CapabilityOrigin::ExternalPlugin,
            log,
        )
    }

    /// session.log.invariant 注册的共享实现：同时写入通用 Any 存储与特化存储。
    fn register_session_log_invariant_impl(
        &self,
        definition: ServiceDefinition,
        origin: CapabilityOrigin,
        log: Arc<dyn crate::SessionLogInvariant>,
    ) -> Result<EffectHandle, CapabilityError> {
        let id = definition.id.clone();
        {
            self.prepare_registration(&id, origin)?;
            let mut slot = self.inner.write();
            slot.insert(
                id.clone(),
                CapabilityRegistration { definition, origin, provider: log.clone() },
            );
        }
        self.session_logs.write().insert(id.clone(), log);
        Ok(self.register_rollback(&id, origin, "session-log", &self.session_logs))
    }

    /// 取回 session.log.invariant 接缝上的会话日志不变量（若已注册）。
    pub fn get_session_log_invariant(&self) -> Option<Arc<dyn crate::SessionLogInvariant>> {
        self.session_logs.read().get("session.log.invariant").cloned()
    }

    // ── platform.adapter 接缝（多实例，按平台名注册） ──────────────────────

    /// 注册一个内置消息平台适配器（platform.adapter 接缝）。
    ///
    /// `platform_name` 即平台唯一名称（如 `"telegram"`），注册 ID 为
    /// `platform.adapter.{platform_name}`。consumers 可通过
    /// [`CapabilityRegistry::get_platform_adapter`] 按名称取回。
    pub fn register_platform_adapter(
        &self,
        platform_name: &str,
        adapter: Arc<dyn crate::MessagePlatformAdapter>,
    ) -> Result<EffectHandle, CapabilityError> {
        self.register_platform_adapter_impl(
            ServiceDefinition::platform_adapter(platform_name),
            CapabilityOrigin::BuiltIn,
            adapter,
        )
    }

    /// 注册一个外部插件提供的消息平台适配器。
    pub fn register_external_platform_adapter(
        &self,
        platform_name: &str,
        adapter: Arc<dyn crate::MessagePlatformAdapter>,
    ) -> Result<EffectHandle, CapabilityError> {
        self.register_platform_adapter_impl(
            ServiceDefinition::platform_adapter(platform_name),
            CapabilityOrigin::ExternalPlugin,
            adapter,
        )
    }

    /// platform.adapter 注册的共享实现：同时写入通用 Any 存储与特化存储。
    fn register_platform_adapter_impl(
        &self,
        definition: ServiceDefinition,
        origin: CapabilityOrigin,
        adapter: Arc<dyn crate::MessagePlatformAdapter>,
    ) -> Result<EffectHandle, CapabilityError> {
        let id = definition.id.clone();
        {
            self.prepare_registration(&id, origin)?;
            let mut slot = self.inner.write();
            slot.insert(
                id.clone(),
                CapabilityRegistration { definition, origin, provider: adapter.clone() },
            );
        }
        self.platform_adapters.write().insert(id.clone(), adapter);
        Ok(self.register_rollback(&id, origin, "platform-adapter", &self.platform_adapters))
    }

    /// 按平台名取回一个平台适配器（若已注册）。
    pub fn get_platform_adapter(
        &self,
        platform_name: &str,
    ) -> Option<Arc<dyn crate::MessagePlatformAdapter>> {
        let id = format!("platform.adapter.{platform_name}");
        self.platform_adapters.read().get(&id).cloned()
    }

    /// 列出所有已注册的平台名（去掉 `platform.adapter.` 前缀）。
    pub fn list_platform_adapters(&self) -> Vec<String> {
        self.platform_adapters
            .read()
            .keys()
            .map(|k| k.trim_start_matches("platform.adapter.").to_string())
            .collect()
    }

    /// 注册一个可逆回滚：撤销时同时清理通用存储与给定的特化存储槽。
    ///
    /// `T` 为 `?Sized`，因为特化槽存的是 `Arc<dyn SomeTrait>`（trait object）。
    ///
    /// `Send + Sync` 约束保证 `Arc<T>` 可被闭包捕获为 `Send + Sync`（`register` 要求）。
    fn register_rollback<T: ?Sized + Send + Sync + 'static>(
        &self,
        id: &str,
        origin: CapabilityOrigin,
        suffix: &str,
        slot: &Arc<RwLock<HashMap<String, Arc<T>>>>,
    ) -> EffectHandle {
        let id_owned = id.to_string();
        let undo_inner = self.inner.clone();
        let undo_slot = slot.clone();
        let handle = self.effects.register(format!("capability:{id_owned}:{suffix}"), move || {
            undo_inner.write().remove(&id_owned);
            undo_slot.write().remove(&id_owned);
        });
        if origin == CapabilityOrigin::BuiltIn {
            self.note_builtin_handle(id, handle.clone());
        }
        handle
    }

    /// 是否已注册指定 ID。
    pub fn contains(&self, id: &str) -> bool {
        self.inner.read().contains_key(id)
    }

    /// 已注册能力数量。
    pub fn len(&self) -> usize {
        self.inner.read().len()
    }

    /// 是否为空。
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// 列出全部能力定义（用于检视 / `dump-config`）。
    pub fn list_definitions(&self) -> Vec<ServiceDefinition> {
        self.inner.read().values().map(|r| r.definition.clone()).collect()
    }

    /// 列出全部能力来源（用于检视 / `dump-config`）。
    pub fn list_origins(&self) -> Vec<(String, CapabilityOrigin)> {
        self.inner.read().values().map(|r| (r.definition.id.clone(), r.origin)).collect()
    }

    /// 列出带详细来源的能力注册（用于检视 / `dump-config`）。
    ///
    /// 对每条能力，若 provider 为 [`PluginCapabilityDescriptor`]（外部插件声明），
    /// 额外标注来源插件 ID，便于前端按插件过滤已注册的外部能力。
    pub fn list_with_details(&self) -> Vec<CapabilityRegistrationDetail> {
        self.inner
            .read()
            .values()
            .map(|r| {
                let plugin_id = r
                    .provider
                    .downcast_ref::<PluginCapabilityDescriptor>()
                    .map(|d| d.plugin_id.clone());
                CapabilityRegistrationDetail {
                    definition: r.definition.clone(),
                    origin: r.origin,
                    plugin_id,
                }
            })
            .collect()
    }

    /// 逆序回滚全部注册（清空注册表并回放所有撤销闭包）。
    pub fn rollback_all(&self) {
        self.effects.rollback_all();
    }
}

impl Default for CapabilityRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for CapabilityRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // 内部为共享锁容器，无法 derive Debug；仅暴露检视友好的摘要。
        f.debug_struct("CapabilityRegistry").field("len", &self.len()).finish()
    }
}

/// 一条带详细来源的运行时能力注册（用于检视 / `dump-config`）。
#[derive(Debug, Clone)]
pub struct CapabilityRegistrationDetail {
    /// 能力接缝声明。
    pub definition: ServiceDefinition,
    /// 来源（内置 / 外部插件）。
    pub origin: CapabilityOrigin,
    /// 若该能力由外部插件注册（provider 为 [`PluginCapabilityDescriptor`]），
    /// 则为来源插件 ID；内置能力为 `None`。
    pub plugin_id: Option<String>,
}

/// DI 契约 — consumer crate 通过此 trait 获取能力注册表，不依赖具体实现。
pub trait HasCapabilityRegistry: Send + Sync {
    /// 返回能力注册表的共享引用。
    fn capability_registry(&self) -> Arc<CapabilityRegistry>;
}

/// 全局能力注册表实例 — 与 [`crate::get_service_registry`] 同构的过渡方案。
///
/// 后续可迁移到显式 DI 注入。
static CAPABILITY_REGISTRY: std::sync::OnceLock<CapabilityRegistry> = std::sync::OnceLock::new();

/// 获取全局能力注册表的引用。
pub fn get_capability_registry() -> &'static CapabilityRegistry {
    CAPABILITY_REGISTRY.get_or_init(CapabilityRegistry::new)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent_turn_runner::{AgentTurnRequest, AgentTurnResult};
    use crate::core_error::{AxAgentError, Result};
    use crate::provider::ProviderRequestContext;
    use crate::types::{
        ChatRequest, ChatResponse, ChatStreamChunk, EmbedRequest, EmbedResponse, Model, TokenUsage,
    };
    use async_trait::async_trait;
    use futures::Stream;
    use std::pin::Pin;
    use std::sync::Arc;

    /// 最小 ProviderAdapter 测试替身 — 仅实现 4 个必需方法。
    struct TestProvider;

    #[async_trait]
    impl ProviderAdapter for TestProvider {
        async fn chat(
            &self,
            _ctx: &ProviderRequestContext,
            _request: Arc<ChatRequest>,
        ) -> Result<ChatResponse> {
            Err(AxAgentError::Provider("test-only".to_string()))
        }

        fn chat_stream(
            &self,
            _ctx: &ProviderRequestContext,
            _request: ChatRequest,
            _cancel_token: Option<Arc<std::sync::atomic::AtomicBool>>,
        ) -> Pin<Box<dyn Stream<Item = Result<ChatStreamChunk>> + Send>> {
            Box::pin(futures::stream::empty())
        }

        async fn list_models(&self, _ctx: &ProviderRequestContext) -> Result<Vec<Model>> {
            Ok(Vec::new())
        }

        async fn embed(
            &self,
            _ctx: &ProviderRequestContext,
            _request: EmbedRequest,
        ) -> Result<EmbedResponse> {
            Err(AxAgentError::Provider("test-only".to_string()))
        }
    }

    #[test]
    fn register_and_retrieve_model_provider() {
        let registry = CapabilityRegistry::new();
        let adapter: Arc<dyn ProviderAdapter> = Arc::new(TestProvider);
        let _handle = registry.register_model_provider(adapter.clone()).unwrap();

        assert!(registry.contains("model.provider"));
        let got = registry.get_provider_adapter().unwrap();
        // 类型向下转型成功即证明 Provider 可被 Consumer 取回
        assert!(Arc::ptr_eq(&adapter, &got));
    }

    #[test]
    fn duplicate_registration_is_rejected() {
        let registry = CapabilityRegistry::new();
        let adapter: Arc<dyn ProviderAdapter> = Arc::new(TestProvider);
        registry.register_model_provider(adapter.clone()).unwrap();
        let err = registry.register_model_provider(adapter).unwrap_err();
        assert!(matches!(err, CapabilityError::Duplicate { .. }));
    }

    #[test]
    fn unregister_removes_capability() {
        let registry = CapabilityRegistry::new();
        let adapter: Arc<dyn ProviderAdapter> = Arc::new(TestProvider);
        let handle = registry.register_model_provider(adapter).unwrap();
        assert_eq!(registry.len(), 1);

        handle.undo();
        assert!(registry.is_empty());
        assert!(!registry.contains("model.provider"));
    }

    #[test]
    fn rollback_all_clears_registry() {
        let registry = CapabilityRegistry::new();
        let adapter: Arc<dyn ProviderAdapter> = Arc::new(TestProvider);
        registry.register_model_provider(adapter.clone()).unwrap();
        let _ = registry.register(
            ServiceDefinition::agent_loop(),
            CapabilityOrigin::BuiltIn,
            Arc::new(TestProvider),
        );
        assert_eq!(registry.len(), 2);

        registry.rollback_all();
        assert!(registry.is_empty());
    }

    #[test]
    fn generic_register_and_typed_get_roundtrip() {
        let registry = CapabilityRegistry::new();
        let marker = Arc::new(String::from("hello"));
        let _handle = registry
            .register(
                ServiceDefinition::tool_set(),
                CapabilityOrigin::ExternalPlugin,
                marker.clone(),
            )
            .unwrap();

        let got: Arc<String> = registry.get_typed("tool.set").unwrap();
        assert_eq!(*got, "hello");
        // 用错误类型向下转型（u64 不匹配）应返回 None
        assert!(registry.get_typed::<u64>("tool.set").is_none());
    }

    #[test]
    fn global_registry_is_singleton() {
        let a = get_capability_registry();
        let b = get_capability_registry();
        assert!(std::ptr::eq(a, b));
    }

    /// 最小 AgentTurnRunner 测试替身 — 仅实现 run_turn。
    struct StubLoop;

    #[async_trait]
    impl AgentTurnRunner for StubLoop {
        async fn run_turn(&self, request: AgentTurnRequest) -> Result<AgentTurnResult> {
            Ok(AgentTurnResult {
                content: format!("echo:{}", request.user_input),
                thinking: None,
                tool_calls: vec![],
                usage: TokenUsage::default(),
                iterations: 1,
                stopped_by_limit: false,
            })
        }
    }

    #[test]
    fn register_and_retrieve_agent_loop() {
        let registry = CapabilityRegistry::new();
        let runner: Arc<dyn AgentTurnRunner> = Arc::new(StubLoop);
        let _handle = registry.register_agent_loop(runner.clone()).unwrap();

        assert!(registry.contains("agent.loop"));
        let got = registry.get_agent_turn_runner().unwrap();
        assert!(Arc::ptr_eq(&runner, &got));
    }

    #[test]
    fn agent_loop_duplicate_is_rejected() {
        let registry = CapabilityRegistry::new();
        let runner: Arc<dyn AgentTurnRunner> = Arc::new(StubLoop);
        registry.register_agent_loop(runner.clone()).unwrap();
        let err = registry.register_agent_loop(runner).unwrap_err();
        assert!(matches!(err, CapabilityError::Duplicate { .. }));
    }

    #[test]
    fn agent_loop_handle_undo_removes_capability() {
        let registry = CapabilityRegistry::new();
        let runner: Arc<dyn AgentTurnRunner> = Arc::new(StubLoop);
        let handle = registry.register_agent_loop(runner).unwrap();
        assert_eq!(registry.len(), 1);

        handle.undo();
        assert!(registry.is_empty());
        assert!(!registry.contains("agent.loop"));
        assert!(registry.get_agent_turn_runner().is_none());
    }

    #[test]
    fn registered_by_type_provider_is_retrievable_by_type() {
        let registry = CapabilityRegistry::new();
        let adapter: Arc<dyn ProviderAdapter> = Arc::new(TestProvider);
        let _handle = registry.register_model_provider_for_type("openai", adapter.clone()).unwrap();

        // 特化检索路径：按类型可取回同一适配器（修复「注册即检索失败」）
        let got = registry.get_provider_adapter_for("openai").unwrap();
        assert!(Arc::ptr_eq(&adapter, &got));
        // 未注册的类型返回 None
        assert!(registry.get_provider_adapter_for("nonexistent").is_none());
        // 通用检视路径同样可见
        assert!(registry.contains("model.provider.openai"));
    }

    #[test]
    fn builtin_by_type_reregistration_is_idempotent() {
        // 缺陷 #10：内置按类型注册幂等——同键内置重复注册静默跳过，
        // 不产生 Duplicate，且不覆盖已注册的适配器、不新增能力条目。
        let registry = CapabilityRegistry::new();
        let adapter: Arc<dyn ProviderAdapter> = Arc::new(TestProvider);
        let _h1 = registry.register_model_provider_for_type("openai", adapter.clone()).unwrap();
        assert_eq!(registry.len(), 1);

        // 同键再次注册：幂等跳过，返回 Ok，能力条目仍为 1（不重复注入、不产生 Duplicate）。
        let h2 = registry.register_model_provider_for_type("openai", adapter.clone()).unwrap();
        assert_eq!(registry.len(), 1);
        // 检索仍命中首次注册的适配器
        let got = registry.get_provider_adapter_for("openai").unwrap();
        assert!(Arc::ptr_eq(&adapter, &got));

        // 幂等后撤销空句柄：不应移除首次注册。
        h2.undo();
        assert_eq!(registry.len(), 1);
        assert!(registry.get_provider_adapter_for("openai").is_some());
    }

    #[test]
    fn builtin_by_type_does_not_override_external() {
        // 平权语义：同为 `model.provider.openai` 键，外部插件先注册后，
        // 内置按类型注册应报 Duplicate（不得幂等跳过、也不得覆盖外部注册）。
        let registry = CapabilityRegistry::new();
        let external: Arc<dyn std::any::Any + Send + Sync> = Arc::new(TestProvider);
        let def = ServiceDefinition::new(
            "model.provider.openai",
            "1.0",
            "axagent_harness::ProviderAdapter",
            "外部测试适配器",
        );
        registry.register(def, CapabilityOrigin::ExternalPlugin, external).unwrap();
        // 外部注册占据通用 map 的同键，内置按类型注册必须报 Duplicate，不得覆盖。
        assert!(registry.contains("model.provider.openai"));

        let builtin: Arc<dyn ProviderAdapter> = Arc::new(TestProvider);
        let err = registry.register_model_provider_for_type("openai", builtin).unwrap_err();
        assert!(matches!(err, CapabilityError::Duplicate { .. }));
    }

    /// 内置接缝端到端闭环（缺陷 #11）：全部内置接缝经注册表注册后，
    /// 能被真实 consumer 检索 API 取回同一实例（`Arc::ptr_eq`），证明
    /// 「内置核心平权」链路是闭环——注册与消费落在同一存储，且对象可被消费方使用。
    #[test]
    fn builtin_seams_end_to_end_register_and_consumable() {
        let registry = CapabilityRegistry::new();

        // 1) model.provider.{type}：注册 → 按 type 取回
        let adapter: Arc<dyn ProviderAdapter> = Arc::new(TestProvider);
        registry.register_model_provider_for_type("openai", adapter.clone()).unwrap();
        assert!(Arc::ptr_eq(&adapter, &registry.get_provider_adapter_for("openai").unwrap()));

        // 2) agent.loop：注册 → 消费侧 get_agent_turn_runner 取回
        let loop_runner: Arc<dyn AgentTurnRunner> = Arc::new(StubLoop);
        registry.register_agent_loop(loop_runner.clone()).unwrap();
        assert!(Arc::ptr_eq(&loop_runner, &registry.get_agent_turn_runner().unwrap()));

        // 3) storage：注册 → get_storage 取回
        let storage: Arc<dyn crate::StorageBackend> = Arc::new(StubStorage);
        registry.register_storage(storage.clone()).unwrap();
        assert!(Arc::ptr_eq(&storage, &registry.get_storage().unwrap()));

        // 4) sandbox：注册 → get_sandbox 取回
        let sandbox: Arc<dyn crate::WorkflowSandbox> = Arc::new(StubSandbox);
        registry.register_sandbox(sandbox.clone()).unwrap();
        assert!(Arc::ptr_eq(&sandbox, &registry.get_sandbox().unwrap()));

        // 5) session.log.invariant：注册 → get_session_log_invariant 取回
        let log: Arc<dyn crate::SessionLogInvariant> = Arc::new(crate::InMemorySessionLog::new());
        registry.register_session_log_invariant(log.clone()).unwrap();
        assert!(Arc::ptr_eq(&log, &registry.get_session_log_invariant().unwrap()));

        // 6) message.callback：注册 → get_message_callback 取回
        let cb: Arc<dyn crate::PlatformMessageCallback> = Arc::new(StubCallback);
        registry.register_message_callback(cb.clone()).unwrap();
        assert!(Arc::ptr_eq(&cb, &registry.get_message_callback().unwrap()));

        // 7) webhook.dispatch：注册 → get_webhook_dispatch 取回
        let dispatch: Arc<dyn crate::WebhookDispatch> = Arc::new(StubDispatch);
        registry.register_webhook_dispatch(dispatch.clone()).unwrap();
        assert!(Arc::ptr_eq(&dispatch, &registry.get_webhook_dispatch().unwrap()));

        // 8) platform.adapter：注册 → get_platform_adapter 取回
        let platform: Arc<dyn crate::MessagePlatformAdapter> =
            Arc::new(StubPlatform { name: "telegram" });
        registry.register_platform_adapter("telegram", platform.clone()).unwrap();
        assert!(Arc::ptr_eq(&platform, &registry.get_platform_adapter("telegram").unwrap()));

        // 9) event.dispatch：注册 → get_event_dispatcher 取回
        let bus = Arc::new(crate::EventDispatchBus::new());
        registry.register_event_dispatcher(bus.clone()).unwrap();
        assert!(Arc::ptr_eq(&bus, &registry.get_event_dispatcher().unwrap()));

        // 9 个内置接缝全部注册成功，且消费检索能取回同一实例（闭环成立）。
        assert_eq!(registry.len(), 9);
    }

    /// 最小 PlatformMessageCallback 测试替身。
    struct StubCallback;

    #[async_trait]
    impl crate::PlatformMessageCallback for StubCallback {
        async fn on_message(
            &self,
            _platform: &str,
            user_id: &str,
            _username: Option<&str>,
            _chat_id: &str,
            text: &str,
        ) -> Option<String> {
            Some(format!("echo:{user_id}:{text}"))
        }

        async fn save_cursor(&self, _platform: &str, _cursor: i64) {}
    }

    #[test]
    fn register_and_retrieve_message_callback() {
        let registry = CapabilityRegistry::new();
        let cb: Arc<dyn crate::PlatformMessageCallback> = Arc::new(StubCallback);
        let _handle = registry.register_message_callback(cb.clone()).unwrap();

        assert!(registry.contains("message.callback"));
        let got = registry.get_message_callback().unwrap();
        assert!(Arc::ptr_eq(&cb, &got));
    }

    #[test]
    fn message_callback_duplicate_is_rejected() {
        let registry = CapabilityRegistry::new();
        let cb: Arc<dyn crate::PlatformMessageCallback> = Arc::new(StubCallback);
        registry.register_message_callback(cb.clone()).unwrap();
        let err = registry.register_message_callback(cb).unwrap_err();
        assert!(matches!(err, CapabilityError::Duplicate { .. }));
    }

    #[test]
    fn message_callback_handle_undo_removes_capability() {
        let registry = CapabilityRegistry::new();
        let cb: Arc<dyn crate::PlatformMessageCallback> = Arc::new(StubCallback);
        let handle = registry.register_message_callback(cb).unwrap();
        assert_eq!(registry.len(), 1);

        handle.undo();
        assert!(registry.is_empty());
        assert!(!registry.contains("message.callback"));
        assert!(registry.get_message_callback().is_none());
    }

    /// 最小 WebhookDispatch 测试替身。
    struct StubDispatch;

    #[async_trait]
    impl crate::WebhookDispatch for StubDispatch {
        async fn dispatch(
            &self,
            _event: crate::WebhookEvent,
            _data: std::collections::HashMap<String, serde_json::Value>,
        ) -> crate::DispatchResult {
            crate::DispatchResult { success_count: 0, failure_count: 0, errors: Vec::new() }
        }
    }

    #[test]
    fn register_and_retrieve_webhook_dispatch() {
        let registry = CapabilityRegistry::new();
        let d: Arc<dyn crate::WebhookDispatch> = Arc::new(StubDispatch);
        let _handle = registry.register_webhook_dispatch(d.clone()).unwrap();

        assert!(registry.contains("webhook.dispatch"));
        let got = registry.get_webhook_dispatch().unwrap();
        assert!(Arc::ptr_eq(&d, &got));
    }

    #[test]
    fn webhook_dispatch_handle_undo_removes_capability() {
        let registry = CapabilityRegistry::new();
        let d: Arc<dyn crate::WebhookDispatch> = Arc::new(StubDispatch);
        let handle = registry.register_webhook_dispatch(d).unwrap();
        assert_eq!(registry.len(), 1);

        handle.undo();
        assert!(registry.is_empty());
        assert!(!registry.contains("webhook.dispatch"));
        assert!(registry.get_webhook_dispatch().is_none());
    }

    /// 最小 MessagePlatformAdapter 测试替身。
    struct StubPlatform {
        name: &'static str,
    }

    #[async_trait]
    impl crate::MessagePlatformAdapter for StubPlatform {
        fn name(&self) -> &'static str {
            self.name
        }

        fn is_enabled(&self, _config: &crate::platform_config::PlatformConfig) -> bool {
            true
        }

        async fn start(
            &self,
            _config: &crate::platform_config::PlatformConfig,
        ) -> anyhow::Result<()> {
            Ok(())
        }

        async fn stop(&self) -> anyhow::Result<()> {
            Ok(())
        }

        async fn is_connected(&self) -> bool {
            true
        }

        async fn send_message(
            &self,
            _config: &crate::platform_config::PlatformConfig,
            _chat_id: &str,
            _text: &str,
            _parse_mode: Option<&str>,
        ) -> anyhow::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn register_and_retrieve_platform_adapter() {
        let registry = CapabilityRegistry::new();
        let a: Arc<dyn crate::MessagePlatformAdapter> = Arc::new(StubPlatform { name: "foo" });
        let _handle = registry.register_platform_adapter("foo", a.clone()).unwrap();

        assert!(registry.contains("platform.adapter.foo"));
        let got = registry.get_platform_adapter("foo").unwrap();
        assert!(Arc::ptr_eq(&a, &got));
    }

    #[test]
    fn platform_adapter_duplicate_is_rejected() {
        let registry = CapabilityRegistry::new();
        let a: Arc<dyn crate::MessagePlatformAdapter> = Arc::new(StubPlatform { name: "x" });
        registry.register_platform_adapter("x", a.clone()).unwrap();
        let err = registry.register_platform_adapter("x", a).unwrap_err();
        assert!(matches!(err, CapabilityError::Duplicate { .. }));
    }

    #[test]
    fn platform_adapter_handle_undo_removes_capability() {
        let registry = CapabilityRegistry::new();
        let a: Arc<dyn crate::MessagePlatformAdapter> = Arc::new(StubPlatform { name: "y" });
        let handle = registry.register_platform_adapter("y", a).unwrap();
        assert_eq!(registry.len(), 1);

        handle.undo();
        assert!(registry.is_empty());
        assert!(!registry.contains("platform.adapter.y"));
        assert!(registry.get_platform_adapter("y").is_none());
    }

    #[test]
    fn list_platform_adapters_returns_names_without_prefix() {
        let registry = CapabilityRegistry::new();
        let a1: Arc<dyn crate::MessagePlatformAdapter> =
            Arc::new(StubPlatform { name: "telegram" });
        let a2: Arc<dyn crate::MessagePlatformAdapter> = Arc::new(StubPlatform { name: "discord" });
        registry.register_platform_adapter("telegram", a1).unwrap();
        registry.register_platform_adapter("discord", a2).unwrap();

        let mut names = registry.list_platform_adapters();
        names.sort();
        assert_eq!(names, vec!["discord", "telegram"]);
    }

    #[test]
    fn register_plugin_capability_registers_and_rolls_back() {
        let registry = CapabilityRegistry::new();
        let handle = registry
            .register_plugin_capability(PluginCapabilityDescriptor::new(
                "platform.adapter.telegram",
                "external:demo",
                "platform_adapter",
                "1.0",
                "demo plugin telegram adapter",
            ))
            .unwrap();

        assert!(registry.contains("platform.adapter.telegram"));
        assert_eq!(
            registry.list_origins(),
            vec![("platform.adapter.telegram".to_string(), CapabilityOrigin::ExternalPlugin)]
        );
        // 声明式描述可经 get_typed 取回
        let got: Arc<PluginCapabilityDescriptor> =
            registry.get_typed("platform.adapter.telegram").unwrap();
        assert_eq!(got.plugin_id, "external:demo");
        assert_eq!(got.capability_type, "platform_adapter");

        // 逆序回滚撤销注册
        handle.undo();
        assert!(!registry.contains("platform.adapter.telegram"));
        assert!(registry.is_empty());
    }

    #[test]
    fn register_plugin_capability_duplicate_is_rejected() {
        let registry = CapabilityRegistry::new();
        registry
            .register_plugin_capability(PluginCapabilityDescriptor::new(
                "tool.set.demo",
                "external:demo",
                "tool_set",
                "1.0",
                "demo tool set",
            ))
            .unwrap();
        let err = registry
            .register_plugin_capability(PluginCapabilityDescriptor::new(
                "tool.set.demo",
                "external:other",
                "tool_set",
                "1.0",
                "conflict",
            ))
            .unwrap_err();
        assert!(matches!(err, CapabilityError::Duplicate { .. }));
    }

    /// 最小 StorageBackend 测试替身。
    struct StubStorage;

    #[async_trait]
    impl crate::StorageBackend for StubStorage {
        async fn get(&self, key: &str) -> crate::core_error::Result<crate::StorageObject> {
            Ok(crate::StorageObject {
                key: key.to_string(),
                data: vec![],
                content_type: "application/octet-stream".to_string(),
                etag: None,
                last_modified: None,
                size: 0,
            })
        }
        async fn put(
            &self,
            key: &str,
            _data: &[u8],
            _content_type: &str,
        ) -> crate::core_error::Result<crate::StorageObjectMeta> {
            Ok(crate::StorageObjectMeta {
                key: key.to_string(),
                etag: None,
                last_modified: None,
                size: 0,
            })
        }
        async fn delete(&self, _key: &str) -> crate::core_error::Result<()> {
            Ok(())
        }
        async fn list(
            &self,
            _prefix: &str,
            _limit: usize,
            _continuation_token: Option<&str>,
        ) -> crate::core_error::Result<crate::ListResult> {
            Ok(crate::ListResult { objects: vec![], is_truncated: false, continuation_token: None })
        }
        async fn head(&self, key: &str) -> crate::core_error::Result<crate::StorageObjectMeta> {
            Ok(crate::StorageObjectMeta {
                key: key.to_string(),
                etag: None,
                last_modified: None,
                size: 0,
            })
        }
        async fn check_connection(&self) -> crate::core_error::Result<bool> {
            Ok(true)
        }
    }

    #[test]
    fn register_and_retrieve_storage_with_undo() {
        let registry = CapabilityRegistry::new();
        let backend: Arc<dyn crate::StorageBackend> = Arc::new(StubStorage);
        let handle = registry.register_storage(backend.clone()).unwrap();

        assert!(registry.contains("storage.backend"));
        let got = registry.get_storage().unwrap();
        assert!(Arc::ptr_eq(&backend, &got));

        handle.undo();
        assert!(!registry.contains("storage.backend"));
        assert!(registry.get_storage().is_none());
    }

    /// 最小 WorkflowSandbox 测试替身。
    struct StubSandbox;

    #[async_trait]
    impl crate::WorkflowSandbox for StubSandbox {
        async fn execute(
            &self,
            _genome: &crate::WorkflowGenome,
            _test_input: &serde_json::Value,
        ) -> std::result::Result<crate::SandboxValidationResult, String> {
            Ok(crate::SandboxValidationResult::default())
        }
    }

    #[test]
    fn register_and_retrieve_sandbox_with_undo() {
        let registry = CapabilityRegistry::new();
        let sandbox: Arc<dyn crate::WorkflowSandbox> = Arc::new(StubSandbox);
        let handle = registry.register_sandbox(sandbox.clone()).unwrap();

        assert!(registry.contains("workflow.sandbox"));
        let got = registry.get_sandbox().unwrap();
        assert!(Arc::ptr_eq(&sandbox, &got));

        handle.undo();
        assert!(!registry.contains("workflow.sandbox"));
        assert!(registry.get_sandbox().is_none());
    }

    #[test]
    fn external_register_replaces_builtin_agent_loop() {
        let registry = CapabilityRegistry::new();
        let builtin: Arc<dyn AgentTurnRunner> = Arc::new(StubLoop);
        let _bh = registry.register_agent_loop(builtin.clone()).unwrap();
        assert!(registry.contains("agent.loop"));

        // 外部插件注册同一接缝：应替换内置而非 Duplicate（缺陷 #7 平权语义）
        let external: Arc<dyn AgentTurnRunner> = Arc::new(StubLoop);
        let eh = registry.register_external_agent_loop(external.clone()).unwrap();
        let got = registry.get_agent_turn_runner().unwrap();
        assert!(Arc::ptr_eq(&external, &got), "外部插件应替换内置主循环");
        assert!(!Arc::ptr_eq(&builtin, &got), "内置主循环应已被撤销");

        // 外部句柄回滚后接缝清除（内置句柄已被 evict，不会残留）
        eh.undo();
        assert!(!registry.contains("agent.loop"));
        assert!(registry.get_agent_turn_runner().is_none());
    }

    #[test]
    fn external_duplicate_is_still_rejected() {
        let registry = CapabilityRegistry::new();
        let a: Arc<dyn AgentTurnRunner> = Arc::new(StubLoop);
        registry.register_external_agent_loop(a.clone()).unwrap();
        let b: Arc<dyn AgentTurnRunner> = Arc::new(StubLoop);
        let err = registry.register_external_agent_loop(b).unwrap_err();
        assert!(matches!(err, CapabilityError::Duplicate { .. }));
    }

    #[test]
    fn register_and_retrieve_event_dispatcher() {
        let registry = CapabilityRegistry::new();
        let dispatcher = Arc::new(crate::EventDispatchBus::new());
        let handle = registry.register_event_dispatcher(dispatcher.clone()).unwrap();

        assert!(registry.contains("event.dispatch"));
        let got = registry.get_event_dispatcher().unwrap();
        assert!(Arc::ptr_eq(&dispatcher, &got));

        handle.undo();
        assert!(!registry.contains("event.dispatch"));
        assert!(registry.get_event_dispatcher().is_none());
    }

    #[test]
    fn register_and_retrieve_session_log_invariant() {
        let registry = CapabilityRegistry::new();
        let log: Arc<dyn crate::SessionLogInvariant> = Arc::new(crate::InMemorySessionLog::new());
        let handle = registry.register_session_log_invariant(log.clone()).unwrap();

        assert!(registry.contains("session.log.invariant"));
        let got = registry.get_session_log_invariant().unwrap();
        // 记录一条 model-visible 内容并校验可重建，验证接缝取回的实现可用
        got.record_model_visible(
            "s1",
            crate::ModelVisibleContent::from_chat_message(&crate::types::ChatMessage {
                role: "user".into(),
                content: crate::types::ChatContent::Text("可见内容".into()),
                tool_calls: None,
                tool_call_id: None,
                thinking: None,
            }),
        );
        assert!(got.assert_replayable("s1").is_ok());

        handle.undo();
        assert!(!registry.contains("session.log.invariant"));
        assert!(registry.get_session_log_invariant().is_none());
    }
}

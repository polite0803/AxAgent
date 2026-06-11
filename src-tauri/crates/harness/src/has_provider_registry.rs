// SPDX-License-Identifier: AGPL-3.0-only

//! `HasProviderRegistry` — 接受外部 `ProviderRegistry` 注入的组件抽象。
//!
//! 多个执行器（`LlmExecutor` / `AgentExecutor` / `ConditionExecutor` /
//! `LlmClassifierExecutor`）和消息平台桥（`PlatformBridge`）都持有
//! `Option<Arc<dyn ProviderRegistry>>`，并对外提供相同语义的"注入入口"。
//! 抽出此 trait 后：
//! 1. 调用方（WorkEngine / init/services）可用统一 API 注入
//! 2. 消除了 5 处 `with_provider_registry` 字节级重复
//! 3. 未来新增的执行器只需 `impl HasProviderRegistry` 即可获得该能力
//!
//! 设计上只暴露最小方法（`set_provider_registry`），
//! 不混入业务字段，保持 trait 易于实现与回收。

use std::sync::Arc;

use crate::registry::ProviderRegistry;

/// 接受外部 `ProviderRegistry` 注入的组件。
///
/// 实现方应当：
/// - 存储注入的 `Arc<dyn ProviderRegistry>` 到 `Option` 字段
/// - 在内部使用时 clone 出 `Arc` 后调用 `.get(registry_key)`
pub trait HasProviderRegistry {
    /// 注入 ProviderRegistry。
    ///
    /// 实现必须是幂等的（重复注入可接受，但应避免静默覆盖语义，
    /// 除非实现方有合理理由；目前 5 个实现均用 `Option::Some` 覆盖）。
    fn set_provider_registry(&mut self, registry: Arc<dyn ProviderRegistry>);
}

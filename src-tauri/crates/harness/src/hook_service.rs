// SPDX-License-Identifier: AGPL-3.0-only

//! Hook 链管理服务 trait
//!
//! 抽象 `HookChain` 的注册和生命周期方法。
//! coordinator.rs 仅将 HookChain 作为公开字段存储，不主动调用任何方法；
//! 本 trait 提供完整 API 供外部调用方使用。

use async_trait::async_trait;

use crate::plugin_hook::{HookDecision, SharedHook, ToolCallContext, ToolCallResult};

/// Hook 链服务 — 管理 Plugin Hook 的注册与执行。
#[async_trait]
pub trait HookService: Send + Sync {
    /// 注册一个 hook。
    async fn register(&self, hook: SharedHook);

    /// 按名称移除 hook。
    async fn unregister(&self, name: &str);

    /// 列出所有 hook 名称。
    async fn list(&self) -> Vec<String>;

    /// 执行工具调用前 hook 链，返回 veto 决策（若有）。
    async fn execute_pre_tool_call(&self, ctx: &ToolCallContext) -> Option<HookDecision>;

    /// 执行工具调用后 hook 链。
    async fn execute_post_tool_call(&self, ctx: &ToolCallContext, result: &ToolCallResult);
}

/// `HookService` 的共享引用类型。
pub type SharedHookService = std::sync::Arc<dyn HookService>;

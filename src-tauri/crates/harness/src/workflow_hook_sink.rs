// SPDX-License-Identifier: AGPL-3.0-only

//! 工作流 Hook 接收端 trait
//!
//! rt-workflow crate (hybrid) 不能依赖 runtime-core (consumer) 中的 HookRunner,
//! 但需要触发工作流相关的 Hook 事件。本 trait 提供 abstraction,
//! 让 rt-workflow 通过 harness trait 接口触发 hook,
//! 实际执行由 wiring 层(runtime / commands)注入实现。
//!
//! 设计原则(符合 AGENTS.md 铁律):
//! - trait 定义在 harness(foundation)
//! - rt-workflow 持有 `Option<Arc<dyn WorkflowHookSink>>`,运行时注入
//! - 若未注入则 dispatch 跳过 hook 调用(向后兼容)
//! - 业务组件 → harness ← 实现

use std::sync::Arc;

use async_trait::async_trait;

use crate::runtime_types::hooks::HookEvent;

/// 工作流 Hook 接收端 trait。
///
/// 实现方负责把事件分发给 HookRunner / HookRegistry / 外部监听器。
/// rt-workflow 的 NodeDispatcher 和 WorkEngine 在关键节点调用此 trait。
#[async_trait]
pub trait WorkflowHookSink: Send + Sync {
    /// 触发一个工作流相关 Hook 事件。
    ///
    /// # 参数
    /// - `event`: Hook 事件类型(Workflow* 系列)
    /// - `payload`: JSON 序列化后的事件载荷
    ///
    /// # 返回
    /// - `Ok(())`: 事件已被消费(或被忽略),允许继续执行
    /// - `Err(reason)`: hook 阻断执行(用于 WorkflowNodePreExecute veto)
    async fn emit(&self, event: HookEvent, payload: &str) -> Result<(), String>;
}

/// `WorkflowHookSink` 的共享引用类型。
pub type SharedWorkflowHookSink = Arc<dyn WorkflowHookSink>;

/// 空实现 — 用于未注入 sink 时的占位(向后兼容)。
pub struct NoopWorkflowHookSink;

#[async_trait]
impl WorkflowHookSink for NoopWorkflowHookSink {
    async fn emit(&self, _event: HookEvent, _payload: &str) -> Result<(), String> {
        Ok(())
    }
}

// SPDX-License-Identifier: AGPL-3.0-only

//! 运行时变异访问接口 — 自指工具（`system_evolution_*`）通过该 trait 访问
//! 运行时工具注册表与进化能力，由 wiring 层（`commands/evolution_engine.rs`）实现并注入。
//!
//! # 架构分层
//! - **harness（本文件）**：纯接口契约（foundation，零 axagent-* 依赖）；
//! - **tools**：定义自指工具（`SystemEvolutionInspectTool` 等），仅依赖本 trait；
//! - **wiring（commands）**：实现本 trait 的真实注册/卸载/检查逻辑并注入。

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// 运行时变异操作结果（统一成功/失败结构，供工具与前端消费）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MutationResult {
    /// 是否成功
    pub success: bool,
    /// 结果数据（检查到的能力列表 / 工具定义 / 注册结果等）
    pub data: Value,
    /// 错误信息（`success=false` 时填充，供工具侧透传）
    pub error: Option<String>,
}

impl MutationResult {
    /// 成功结果
    pub fn ok(data: impl Into<Value>) -> Self {
        Self { success: true, data: data.into(), error: None }
    }

    /// 失败结果
    pub fn err(message: impl Into<String>) -> Self {
        Self { success: false, data: Value::Null, error: Some(message.into()) }
    }
}

/// 自指工具访问接口 — 系统进化工具（`system_evolution_*`）的执行句柄。
///
/// # 设计动机
/// 自指工具（tools crate）不直接持有 `UnifiedToolRegistry`（无法访问 AppState），
/// 而是通过本 trait 解耦：tools 定义工具形态，wiring 层实现真实注册/卸载逻辑并注入。
///
/// # 安全约束（实现方必须遵守）
/// - 仅允许操作 `runtime_tool_sources` 标记的运行时工具，保护内置 / MCP 工具；
/// - 所有注册动作登记 `Disposer`（可逆副作用），卸载时自动回滚；
/// - 遵守 SYSTEM_ONLY 隔离，不暴露给业务能力检索（自指熔断由路由层兜底）。
#[async_trait]
pub trait RuntimeMutationAccess: Send + Sync {
    /// 检查当前运行时进化能力状态。
    ///
    /// `capability_type` ∈ { `tool`, `workflow`, `skill`, `all` }（默认 `all`）。
    /// 返回运行时已注册的进化工具列表（名称 / 来源 / 描述）等。
    async fn inspect(&self, capability_type: &str) -> Result<MutationResult, String>;

    /// 定义一个新工具（仅生成工具定义，**不注册**到运行时）。
    ///
    /// `spec` 至少包含 `name` / `description` / `code` / `input_schema`。
    /// 返回工具定义 JSON，供 Agent 审查；确认后调用 `deploy` 完成注册。
    async fn define(&self, spec: Value) -> Result<MutationResult, String>;

    /// 部署（注册）工具到运行时注册表（来源 `runtime_evolution`），
    /// 返回注册后的工具名。重复注册返回 `REGISTRATION_DUPLICATE`。
    async fn deploy(&self, spec: Value) -> Result<MutationResult, String>;

    /// 卸载运行时注册的工具（仅允许 `runtime_tool_sources` 中登记的工具）。
    async fn undeploy(&self, name: &str) -> Result<MutationResult, String>;
}

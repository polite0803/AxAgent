// SPDX-License-Identifier: AGPL-3.0-only

//! 编排派发契约 — Orchestrator 与执行方（agent / work_engine）之间的解耦 trait。
//!
//! ## 背景
//!
//! `axagent-orchestrator` 和 `axagent-agent` 都是 consumer crate，按 AGENTS.md 规则 2
//! 不能互相依赖。orchestrator 分解出 SubTask 后，需要某种机制把 SubTask 派发给
//! 实际执行方（agent 的 SessionManager 或 rt-workflow 的 work_engine）。
//!
//! 本 trait 定义在 harness 层，由执行方 crate 实现，由 runtime/wiring 层注入到
//! `OrchestratorExecutor`，从而打通 decompose → dispatch → execute 的闭环。
//!
//! ## 关联类型
//!
//! - 输入：`DispatchRequest` — 派发请求（sub_task_id + mission + role + tools）
//! - 输出：`DispatchResult` — 执行结果（成功附带 handover，失败附带 error）
//!
//! ## 实现方
//!
//! - `agent::SessionManager` 可实现此 trait，把 SubTask 转为一次 agent run
//! - `rt-workflow::WorkEngine` 可实现此 trait，把 SubTask 转为工作流节点执行
//! - 测试中可用 `MockDispatcher` 实现

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::core_error::Result;
use crate::workflow_types::ToolDef;

// ============================================================================
// 派发请求/响应 DTO
// ============================================================================

/// 派发一个 SubTask 给执行方的请求。
///
/// 执行方应根据 `role` 选择合适的 agent / worker，
/// 使用 `system_prompt`（若有）覆盖默认角色 prompt，
/// 使用 `tools` 限制可用工具集，执行 `mission` 描述的任务。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DispatchRequest {
    /// 对应 orchestrator SubTask 的 id
    pub sub_task_id: String,
    /// 任务描述（自然语言）
    pub mission: String,
    /// 期望的 agent 角色（业务层定义的角色 ID）
    pub role: String,
    /// 可选的 system prompt 覆盖
    pub system_prompt: Option<String>,
    /// 允许使用的工具列表（空表示不限制）
    pub tools: Vec<ToolDef>,
    /// 输出变量名（用于后续 SubTask 引用此结果）
    pub output_var: String,
}

/// 派发执行结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubTaskDispatchResult {
    /// 对应的 sub_task_id
    pub sub_task_id: String,
    /// 是否成功
    pub success: bool,
    /// 成功时的结构化交接信息（JSON 字符串，格式由执行方决定）
    pub handover_json: Option<String>,
    /// 失败时的错误信息
    pub error: Option<String>,
}

// ============================================================================
// SubTaskDispatcher trait
// ============================================================================

/// SubTask 派发执行契约。
///
/// orchestrator 通过此 trait 把 SubTask 派发给实际执行方，
/// 执行方完成后回调 `OrchestratorExecutor::report_sub_task_completed/failed`。
///
/// ## 实现要点
///
/// - **异步**：所有方法都是 `async fn`，实现方应使用 `#[async_trait]`
/// - **零阻塞**：长时间运行的执行应在后台 task 中进行，dispatch 方法本身应快速返回
/// - **错误隔离**：执行方的内部错误应转换为 `DispatchResult.error`，不传播 panic
/// - **idempotent**：重复 dispatch 同一 sub_task_id 应安全（由实现方决定语义）
#[async_trait]
pub trait SubTaskDispatcher: Send + Sync {
    /// 同步派发一个 SubTask 并等待执行完成。
    ///
    /// 适用于短任务或测试场景。长任务应使用 `dispatch_async` + 回调。
    async fn dispatch(&self, request: DispatchRequest) -> Result<SubTaskDispatchResult>;

    /// 异步派发一个 SubTask，立即返回，执行方在完成后调用 `on_complete` 回调。
    ///
    /// `on_complete` 的第一个参数是 `sub_task_id`，第二个是 `SubTaskDispatchResult`。
    /// 默认实现：直接调用 `dispatch` 并同步触发回调。
    async fn dispatch_async(
        &self,
        request: DispatchRequest,
        on_complete: Box<dyn FnOnce(String, SubTaskDispatchResult) + Send>,
    ) -> Result<()> {
        let sub_task_id = request.sub_task_id.clone();
        let result = self.dispatch(request).await?;
        on_complete(sub_task_id, result);
        Ok(())
    }

    /// 批量派发多个独立 SubTask（并行执行）。
    ///
    /// 默认实现：串行调用 `dispatch`。实现方可重写为并行执行。
    async fn dispatch_batch(
        &self,
        requests: Vec<DispatchRequest>,
    ) -> Result<Vec<SubTaskDispatchResult>> {
        let mut results = Vec::with_capacity(requests.len());
        for req in requests {
            results.push(self.dispatch(req).await?);
        }
        Ok(results)
    }
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dispatch_request_serialization() {
        let req = DispatchRequest {
            sub_task_id: "st-001".to_string(),
            mission: "Analyze the codebase".to_string(),
            role: "planner".to_string(),
            system_prompt: Some("You are a planner".to_string()),
            tools: vec![],
            output_var: "analysis_output".to_string(),
        };
        let json = serde_json::to_string(&req).unwrap();
        let parsed: DispatchRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.sub_task_id, "st-001");
        assert_eq!(parsed.mission, "Analyze the codebase");
    }

    #[test]
    fn dispatch_result_success() {
        let result = SubTaskDispatchResult {
            sub_task_id: "st-001".to_string(),
            success: true,
            handover_json: Some(r#"{"completed_work":"done"}"#.to_string()),
            error: None,
        };
        assert!(result.success);
        assert!(result.handover_json.is_some());
    }

    #[test]
    fn dispatch_result_failure() {
        let result = SubTaskDispatchResult {
            sub_task_id: "st-002".to_string(),
            success: false,
            handover_json: None,
            error: Some("timeout".to_string()),
        };
        assert!(!result.success);
        assert_eq!(result.error.as_deref(), Some("timeout"));
    }
}

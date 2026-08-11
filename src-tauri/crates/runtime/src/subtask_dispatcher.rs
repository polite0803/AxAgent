// SPDX-License-Identifier: AGPL-3.0-only

//! 生产用 SubTaskDispatcher wiring 实现。
//!
//! 把 `OrchestratorExecutor` 分解出的 SubTask 派发给实际执行方
//! (agent SessionManager / work_engine)。
//!
//! 由于 `OrchestratorExecutor` (consumer) 不能依赖 agent/work_engine (实现层),
//! 本模块在 runtime (wiring) crate 中提供 `RuntimeSubTaskDispatcher`,
//! 通过 `SubTaskHandler` trait 解耦,在 init 阶段注入实际 handler。
//!
//! ## 设计动机
//!
//! `SubTaskDispatcher` trait 定义在 harness,orchestrator 通过它调用 dispatch。
//! 但 trait 的**实现**需要访问 agent/work_engine 的具体 API,这些都在实现层 crate。
//! 按 AGENTS.md 铁律 2,consumer 不能依赖实现层。
//!
//! 因此在 wiring 层(runtime crate)定义 `SubTaskHandler` trait,
//! 实现方在 wiring 层注入(基于 SessionManager / WorkEngine),
//! `RuntimeSubTaskDispatcher` 桥接 `SubTaskDispatcher` 与 `SubTaskHandler`。

use std::sync::Arc;

use async_trait::async_trait;

use axagent_harness::orchestration_dispatch::{
    DispatchRequest, SubTaskDispatchResult, SubTaskDispatcher,
};

/// SubTask 执行处理器 trait(wiring 层契约)。
///
/// 实现方在 wiring 层注入,把 `DispatchRequest` 转为实际执行:
/// - agent 模式: 调用 `SessionManager::run_turn_with_tools` 执行一次 agent run
/// - 工作流模式: 调用 `WorkEngine::run_workflow` 执行子工作流
/// - 其他: 自定义实现(测试 mock / 远程 RPC 等)
///
/// 与 harness 的 `SubTaskDispatcher` trait 关系:
/// - `SubTaskDispatcher` 是 harness 层契约(orchestrator 调用)
/// - `SubTaskHandler` 是 wiring 层契约(实现方注入)
/// - `RuntimeSubTaskDispatcher` 桥接两者
#[async_trait]
pub trait SubTaskHandler: Send + Sync {
    /// 执行一个 SubTask,返回执行结果。
    ///
    /// 实现要点:
    /// - **异步**: 内部可 spawn 后台 task,但本方法应等待完成后再返回
    /// - **错误隔离**: 内部错误应转换为 `Err(anyhow::Error)`,不传播 panic
    /// - **idempotent**: 重复 dispatch 同一 sub_task_id 应安全(由实现方决定语义)
    async fn handle(&self, request: DispatchRequest) -> anyhow::Result<SubTaskDispatchResult>;
}

/// 默认 Noop handler — 用于未注入实际 handler 时的占位。
///
/// 返回 success + echo mission,保证 orchestrator 调用链不中断。
/// 生产环境应在 init 阶段替换为真实 handler(基于 SessionManager 或 WorkEngine)。
pub struct NoopSubTaskHandler;

#[async_trait]
impl SubTaskHandler for NoopSubTaskHandler {
    async fn handle(&self, request: DispatchRequest) -> anyhow::Result<SubTaskDispatchResult> {
        tracing::warn!(
            sub_task_id = %request.sub_task_id,
            mission = %request.mission,
            "NoopSubTaskHandler: 未注入实际 handler,返回占位成功结果"
        );
        Ok(SubTaskDispatchResult {
            sub_task_id: request.sub_task_id,
            success: true,
            handover_json: Some(
                serde_json::json!({
                    "noop": true,
                    "echo_mission": request.mission,
                    "role": format!("{:?}", request.role),
                })
                .to_string(),
            ),
            error: None,
        })
    }
}

/// 生产用 SubTaskDispatcher 实现 — 桥接 harness trait 与 wiring handler。
///
/// 由 wiring 层(commands/init)在初始化 OrchestratorExecutor 时通过
/// `with_dispatcher(Arc::new(RuntimeSubTaskDispatcher::new(handler)))` 注入。
pub struct RuntimeSubTaskDispatcher {
    handler: Arc<dyn SubTaskHandler>,
}

impl RuntimeSubTaskDispatcher {
    /// 使用指定 handler 创建 dispatcher。
    pub fn new(handler: Arc<dyn SubTaskHandler>) -> Self {
        Self { handler }
    }

    /// 使用 NoopSubTaskHandler 创建 — 用于未注入实际 handler 的占位场景。
    ///
    /// 适用于:
    /// - 单元测试
    /// - 命令层早期开发阶段(orchestrator decomposition 已就绪但执行层未接入)
    pub fn noop() -> Self {
        Self::new(Arc::new(NoopSubTaskHandler))
    }
}

#[async_trait]
impl SubTaskDispatcher for RuntimeSubTaskDispatcher {
    async fn dispatch(
        &self,
        request: DispatchRequest,
    ) -> axagent_harness::Result<SubTaskDispatchResult> {
        self.handler.handle(request).await.map_err(|e| {
            // 把 anyhow::Error 转为 AxAgentError::Execution，保留完整错误链作为 source
            axagent_harness::AxAgentError::execution_with_source("SubTask dispatch failed", e)
        })
    }
}

// ── 测试 ──

#[cfg(test)]
mod tests {
    use super::*;

    fn make_request(id: &str) -> DispatchRequest {
        DispatchRequest {
            sub_task_id: id.to_string(),
            mission: format!("mission-{}", id),
            role: "planner".to_string(),
            system_prompt: None,
            tools: vec![],
            output_var: format!("out_{}", id),
        }
    }

    #[tokio::test]
    async fn noop_handler_returns_success() {
        let dispatcher = RuntimeSubTaskDispatcher::noop();
        let result = dispatcher.dispatch(make_request("t1")).await.expect("测试：异步操作应成功");
        assert!(result.success);
        assert_eq!(result.sub_task_id, "t1");
        assert!(result.handover_json.is_some());
        assert!(result.error.is_none());
    }

    /// 自定义 handler 用于测试注入路径
    struct EchoHandler;
    #[async_trait]
    impl SubTaskHandler for EchoHandler {
        async fn handle(&self, request: DispatchRequest) -> anyhow::Result<SubTaskDispatchResult> {
            Ok(SubTaskDispatchResult {
                sub_task_id: request.sub_task_id,
                success: true,
                handover_json: Some(format!("{{\"echo\":\"{}\"}}", request.mission)),
                error: None,
            })
        }
    }

    #[tokio::test]
    async fn custom_handler_injection() {
        let dispatcher = RuntimeSubTaskDispatcher::new(Arc::new(EchoHandler));
        let result = dispatcher.dispatch(make_request("t2")).await.expect("测试：异步操作应成功");
        assert!(result.success);
        assert_eq!(result.handover_json.as_deref(), Some(r#"{"echo":"mission-t2"}"#));
    }

    /// 失败 handler 用于测试错误传播
    struct FailHandler;
    #[async_trait]
    impl SubTaskHandler for FailHandler {
        async fn handle(&self, _request: DispatchRequest) -> anyhow::Result<SubTaskDispatchResult> {
            Err(anyhow::anyhow!("handler failure"))
        }
    }

    #[tokio::test]
    async fn handler_error_propagates() {
        let dispatcher = RuntimeSubTaskDispatcher::new(Arc::new(FailHandler));
        let result = dispatcher.dispatch(make_request("t3")).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("handler failure"));
    }
}

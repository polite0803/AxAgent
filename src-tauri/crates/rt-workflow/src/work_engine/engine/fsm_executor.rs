// SPDX-License-Identifier: AGPL-3.0-only

//! 业务状态机执行器
//!
//! 本模块实现基于 BusinessStateMachine 的运行时执行逻辑，
//! 提供刚性状态转移校验 + 柔性节点内执行调度。
//!
//! # 架构位置
//! - 实现层：rt-workflow（hybrid 层）
//! - 依赖：harness::business_state_machine（状态机定义）
//! - 与 WorkflowEngine 协作，实现"刚性轨道 + 柔性节点"架构

use std::sync::Arc;

use axagent_harness::business_state_machine::{
    BusinessState, BusinessStateMachine, FsmContext, FsmRuntimeState, FsmTransitionError,
};
use axagent_harness::execution_trace::NodeExecutionTrace;
use tokio::sync::RwLock;

use super::guard_evaluator::GuardEvaluator;

/// FSM 执行器
///
/// 负责驱动业务状态机的状态转移，同时支持在状态内部
/// 通过 Agent 进行柔性的工具调用和知识库检索。
pub struct FsmExecutor {
    /// 状态机定义（不可变）
    fsm: Arc<BusinessStateMachine>,
    /// 运行时状态（线程安全）
    runtime: Arc<RwLock<FsmRuntimeState>>,
    /// 守卫条件评估器
    guard_evaluator: Arc<GuardEvaluator>,
    /// 状态变更监听器
    listeners: Vec<Box<dyn Fn(&FsmRuntimeState) + Send + Sync>>,
}

impl FsmExecutor {
    /// 创建新的 FSM 执行器
    pub fn new(fsm: BusinessStateMachine, instance_id: impl Into<String>) -> Self {
        let initial_state_id = fsm.initial_state_id.clone();
        let runtime_state = FsmRuntimeState::new(instance_id, fsm.id.clone(), initial_state_id);

        Self {
            fsm: Arc::new(fsm),
            runtime: Arc::new(RwLock::new(runtime_state)),
            guard_evaluator: Arc::new(GuardEvaluator::new()),
            listeners: Vec::new(),
        }
    }

    /// 从快照恢复 FSM 执行器
    pub fn from_snapshot(fsm: BusinessStateMachine, runtime_state: FsmRuntimeState) -> Self {
        Self {
            fsm: Arc::new(fsm),
            runtime: Arc::new(RwLock::new(runtime_state)),
            guard_evaluator: Arc::new(GuardEvaluator::new()),
            listeners: Vec::new(),
        }
    }

    /// 验证状态机定义
    pub fn validate(
        &self,
    ) -> Result<(), axagent_harness::business_state_machine::FsmValidationError> {
        self.fsm.validate()
    }

    /// 获取当前状态（快照）
    pub async fn current_state(&self) -> FsmRuntimeState {
        self.runtime.read().await.clone()
    }

    /// 获取当前业务状态定义
    pub async fn current_business_state(&self) -> Option<BusinessState> {
        let runtime = self.runtime.read().await;
        self.fsm.find_state(&runtime.current_state_id).cloned()
    }

    /// 尝试转移到目标状态
    pub async fn transition_to(
        &self,
        target_state_id: &str,
        context: Option<FsmContext>,
    ) -> Result<(), FsmTransitionError> {
        let mut runtime = self.runtime.write().await;

        // 1. 检查状态机是否完成
        if runtime.is_completed {
            return Err(FsmTransitionError::MachineCompleted);
        }

        // 2. 检查转移合法性
        if !self.fsm.is_valid_transition(&runtime.current_state_id, target_state_id) {
            return Err(FsmTransitionError::InvalidTransition {
                from: runtime.current_state_id.clone(),
                to: target_state_id.to_string(),
            });
        }

        // 3. 获取转移规则
        let transition = self
            .fsm
            .transitions
            .iter()
            .find(|t| t.from == runtime.current_state_id && t.to == target_state_id)
            .ok_or(FsmTransitionError::InvalidTransition {
                from: runtime.current_state_id.clone(),
                to: target_state_id.to_string(),
            })?
            .clone();

        // 4. 评估守卫条件
        if transition.has_guard() {
            let ctx = context.unwrap_or_default();
            let allowed = self.guard_evaluator.evaluate(&transition, &ctx)?;
            if !allowed {
                return Err(FsmTransitionError::GuardFailed {
                    transition_id: transition.id.clone(),
                    reason: transition
                        .guard_description
                        .unwrap_or_else(|| "守卫条件不满足".to_string()),
                });
            }
        }

        // 5. 检查是否需要审批
        if transition.requires_approval {
            return Err(FsmTransitionError::RequiresApproval {
                transition_id: transition.id.clone(),
            });
        }

        // 6. 更新状态
        let target_state = self
            .fsm
            .find_state(target_state_id)
            .ok_or(FsmTransitionError::StateNotFound(target_state_id.to_string()))?;

        let now_ms = current_timestamp_ms();
        runtime.previous_state_id = Some(runtime.current_state_id.clone());
        runtime.current_state_id = target_state_id.to_string();
        runtime.updated_at_ms = now_ms;

        let record = axagent_harness::business_state_machine::FsmTransitionRecord {
            from: runtime.previous_state_id.clone().unwrap_or_default(),
            to: target_state_id.to_string(),
            timestamp_ms: now_ms,
        };
        runtime.transition_history.push(record);

        // 7. 检查是否到达终态
        if target_state.is_terminal {
            runtime.is_completed = true;
        }

        // 8. 通知监听器
        drop(runtime);
        self.notify_listeners().await;

        Ok(())
    }

    /// 获取当前状态绑定的工作流节点 ID
    pub async fn current_node_ref(&self) -> Option<String> {
        let runtime = self.runtime.read().await;
        self.fsm.find_state(&runtime.current_state_id).and_then(|s| s.node_ref.clone())
    }

    /// 获取状态允许的工具列表
    pub async fn current_allowed_tools(&self) -> Option<Vec<String>> {
        let runtime = self.runtime.read().await;
        self.fsm.find_state(&runtime.current_state_id).and_then(|s| s.allowed_tools.clone())
    }

    /// 检查转移守卫条件
    pub async fn check_guard(
        &self,
        from: &str,
        to: &str,
        context: &FsmContext,
    ) -> Result<bool, FsmTransitionError> {
        // 查找转移规则
        let transition = self.fsm.transitions.iter().find(|t| t.from == from && t.to == to);

        // 如果没有转移规则，返回 true（由 transition_to 处理合法性）
        let transition = match transition {
            Some(t) => t,
            None => return Ok(true),
        };

        // 没有守卫条件，直接允许
        if !transition.has_guard() {
            return Ok(true);
        }

        // 评估守卫条件
        self.guard_evaluator.evaluate(transition, context)
    }

    /// 添加状态变更监听器
    pub fn add_listener(&mut self, listener: impl Fn(&FsmRuntimeState) + Send + Sync + 'static) {
        self.listeners.push(Box::new(listener));
    }

    /// 通知所有监听器
    async fn notify_listeners(&self) {
        let runtime = self.runtime.read().await;
        for listener in &self.listeners {
            listener(&runtime);
        }
    }

    /// 重置状态机到初始状态
    pub async fn reset(&self) {
        let mut runtime = self.runtime.write().await;
        let now_ms = current_timestamp_ms();
        *runtime = FsmRuntimeState::new(
            runtime.instance_id.clone(),
            runtime.fsm_id.clone(),
            self.fsm.initial_state_id.clone(),
        );
        runtime.created_at_ms = now_ms;
    }

    /// 检查是否在终态
    pub async fn is_completed(&self) -> bool {
        let runtime = self.runtime.read().await;
        runtime.is_completed
    }

    /// 获取转移历史
    pub async fn transition_history(
        &self,
    ) -> Vec<axagent_harness::business_state_machine::FsmTransitionRecord> {
        let runtime = self.runtime.read().await;
        runtime.transition_history.clone()
    }

    /// 生成节点执行轨迹（用于时间旅行）
    pub fn create_node_trace(
        &self,
        node_id: impl Into<String>,
        node_type: impl Into<String>,
    ) -> NodeExecutionTrace {
        NodeExecutionTrace::new(node_id, node_type)
    }
}

impl Clone for FsmExecutor {
    fn clone(&self) -> Self {
        Self {
            fsm: self.fsm.clone(),
            runtime: self.runtime.clone(),
            guard_evaluator: self.guard_evaluator.clone(),
            listeners: Vec::new(), // 监听器不复制
        }
    }
}

// ── 辅助函数 ──

fn current_timestamp_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

// ── 测试辅助 ──

/// 从节点执行轨迹构造状态机转移记录
pub fn create_fsm_transition_record(
    trace: &NodeExecutionTrace,
) -> Option<axagent_harness::business_state_machine::FsmTransitionRecord> {
    trace.business_state_transition.clone()
}

// ── 单元测试 ──

#[cfg(test)]
mod tests {
    use super::*;
    use axagent_harness::business_state_machine::BusinessStateMachine;
    use axagent_harness::workflow_types::NodeStatus;

    #[tokio::test]
    async fn test_fsm_executor_creation() {
        let fsm = BusinessStateMachine::approval_flow();
        let executor = FsmExecutor::new(fsm.clone(), "test-instance");

        assert!(executor.validate().is_ok());

        let runtime = executor.current_state().await;
        assert_eq!(runtime.current_state_id, "submitted");
        assert!(!runtime.is_completed);
    }

    #[tokio::test]
    async fn test_fsm_transition_valid() {
        let fsm = BusinessStateMachine::approval_flow();
        let executor = FsmExecutor::new(fsm, "test-instance");

        let result = executor.transition_to("under_review", None).await;
        assert!(result.is_ok());

        let runtime = executor.current_state().await;
        assert_eq!(runtime.current_state_id, "under_review");
        assert_eq!(runtime.transition_history.len(), 1);
    }

    #[tokio::test]
    async fn test_fsm_transition_invalid() {
        let fsm = BusinessStateMachine::approval_flow();
        let executor = FsmExecutor::new(fsm, "test-instance");

        // 非法转移：submitted → approved（需要经过 under_review）
        let result = executor.transition_to("approved", None).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), FsmTransitionError::InvalidTransition { .. }));
    }

    #[tokio::test]
    async fn test_fsm_transition_to_terminal() {
        let fsm = BusinessStateMachine::approval_flow();
        let executor = FsmExecutor::new(fsm, "test-instance");

        executor.transition_to("under_review", None).await.unwrap();
        executor.transition_to("approved", None).await.unwrap();

        assert!(executor.is_completed().await);

        // 终态后无法转移
        let result = executor.transition_to("submitted", None).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), FsmTransitionError::MachineCompleted));
    }

    #[tokio::test]
    async fn test_fsm_terminal_state() {
        let fsm = BusinessStateMachine::order_flow();
        let executor = FsmExecutor::new(fsm, "order-1");

        // cancelled 是终态
        executor.transition_to("cancelled", None).await.unwrap();
        assert!(executor.is_completed().await);
    }

    #[tokio::test]
    async fn test_fsm_node_ref() {
        let fsm = BusinessStateMachine::approval_flow();
        let executor = FsmExecutor::new(fsm, "test-instance");

        let node_ref = executor.current_node_ref().await;
        assert_eq!(node_ref, Some("node_submit".to_string()));

        executor.transition_to("under_review", None).await.unwrap();
        let node_ref = executor.current_node_ref().await;
        assert_eq!(node_ref, Some("node_review".to_string()));
    }

    #[tokio::test]
    async fn test_fsm_allowed_tools() {
        let fsm = BusinessStateMachine::approval_flow().with_state(
            axagent_harness::business_state_machine::BusinessState::new("restricted")
                .with_allowed_tools(vec!["tool_a".to_string(), "tool_b".to_string()]),
        );
        let executor = FsmExecutor::new(fsm, "test-instance");

        // "submitted" 状态没有设置 allowed_tools
        let tools = executor.current_allowed_tools().await;
        assert!(tools.is_none());
    }

    #[tokio::test]
    async fn test_fsm_listener() {
        let fsm = BusinessStateMachine::approval_flow();
        let mut executor = FsmExecutor::new(fsm, "test-instance");

        executor.add_listener(Box::new(move |_state: &FsmRuntimeState| {
            // 注意：由于 Rust 所有权限制，这里的 listener 不会真正记录
            // 实际使用时应使用 Arc<Mutex>。这个测试仅验证 API 可调用。
        }));

        let result = executor.transition_to("under_review", None).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_fsm_reset() {
        let fsm = BusinessStateMachine::approval_flow();
        let executor = FsmExecutor::new(fsm, "test-instance");

        executor.transition_to("under_review", None).await.unwrap();
        assert_eq!(executor.current_state().await.current_state_id, "under_review");

        executor.reset().await;
        assert_eq!(executor.current_state().await.current_state_id, "submitted");
        assert!(!executor.is_completed().await);
    }

    #[tokio::test]
    async fn test_fsm_transition_history() {
        let fsm = BusinessStateMachine::approval_flow();
        let executor = FsmExecutor::new(fsm, "test-instance");

        executor.transition_to("under_review", None).await.unwrap();
        executor.transition_to("approved", None).await.unwrap();

        let history = executor.transition_history().await;
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].from, "submitted");
        assert_eq!(history[0].to, "under_review");
        assert_eq!(history[1].from, "under_review");
        assert_eq!(history[1].to, "approved");
    }

    #[tokio::test]
    async fn test_fsm_context() {
        let fsm = BusinessStateMachine::approval_flow();
        let executor = FsmExecutor::new(fsm, "test-instance");

        let ctx = axagent_harness::business_state_machine::FsmContext::new()
            .with_event("submit")
            .with_user_role("manager");

        // check_guard 当前总是返回 true
        let allowed = executor.check_guard("submitted", "under_review", &ctx).await;
        assert!(allowed.unwrap());
    }

    #[tokio::test]
    async fn test_fsm_create_node_trace() {
        let fsm = BusinessStateMachine::approval_flow();
        let executor = FsmExecutor::new(fsm, "test-instance");

        let trace = executor.create_node_trace("node-1", "agent");
        assert_eq!(trace.node_id, "node-1");
        assert_eq!(trace.node_type, "agent");
        assert_eq!(trace.status, NodeStatus::Pending);
    }
}

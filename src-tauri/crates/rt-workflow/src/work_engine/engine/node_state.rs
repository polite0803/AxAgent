// SPDX-License-Identifier: AGPL-3.0-only

//! Internal tracking types: circuit breaker, backoff computation, node result.
//!
//! 本模块同时包含 Typestate 模式的实现（阶段 5），将节点状态流转
//! 从运行时检查升级为编译时保证。

use std::marker::PhantomData;

use axagent_harness::workflow_types::{BackoffType, NodeRuntimeState, NodeStatus, WorkflowNode};

use crate::work_engine::{NodeError, NodeOutput};

// ── 辅助函数 ──

/// 获取当前时间戳（毫秒）
fn current_epoch_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

// ── Typestate 状态标记（零尺寸类型，仅用于编译时检查） ──
//
// 注意：这些类型与 NodeStatus 枚举变体同名，但用途不同。
// - NodeStatus: 运行时状态枚举，用于存储和序列化
// - Pending/Ready/Running/...: 零尺寸标记类型，用于编译时检查

/// 待执行状态标记
pub struct Pending;
/// 就绪状态标记（依赖满足）
pub struct Ready;
/// 正在执行标记
pub struct Running;
/// 执行成功标记
pub struct Completed;
/// 执行失败标记
pub struct Failed;
/// 已跳过标记（补偿策略）
pub struct Skipped;

/// 终态标记（用于编译时判断是否为终态）
pub trait TerminalState {}
impl TerminalState for Completed {}
impl TerminalState for Failed {}
impl TerminalState for Skipped {}

// ── Typestate 节点包装 ──

/// 带状态标记的工作流节点包装器
///
/// 使用幽灵类型将节点状态编码进类型系统，
/// 编译器将禁止非法的状态转移。
///
/// # 状态转换规则（编译时强制）
/// ```text
/// Pending → Ready → Running → Completed / Failed / Skipped
/// ```
#[derive(Debug, Clone)]
pub struct WorkflowNodeState<State> {
    /// 节点 ID
    node_id: String,
    /// 原始节点数据
    node: WorkflowNode,
    /// 运行时状态（用于存储时间戳、错误信息等）
    runtime_state: NodeRuntimeState,
    /// 状态标记（PhantomData，不占用运行时空间）
    _state: PhantomData<State>,
}

// ── Pending 状态实现 ──

impl WorkflowNodeState<Pending> {
    /// 从 WorkflowNode 创建新的 Pending 状态节点
    pub fn new(node: WorkflowNode) -> Self {
        let node_id = node.base_id().to_string();
        Self { node_id, node, runtime_state: NodeRuntimeState::default(), _state: PhantomData }
    }

    /// 标记为就绪（所有依赖满足）
    pub fn mark_ready(self) -> WorkflowNodeState<Ready> {
        WorkflowNodeState {
            node_id: self.node_id,
            node: self.node,
            runtime_state: NodeRuntimeState { status: NodeStatus::Ready, ..self.runtime_state },
            _state: PhantomData,
        }
    }

    /// 从 WorkflowNode 创建（显式指定 Pending 状态）
    pub fn from_node(node: WorkflowNode) -> Self {
        Self::new(node)
    }
}

// ── Ready 状态实现 ──

impl WorkflowNodeState<Ready> {
    /// 开始执行
    pub fn start(self) -> WorkflowNodeState<Running> {
        let now_ms = current_epoch_ms() as i64;
        WorkflowNodeState {
            node_id: self.node_id,
            node: self.node,
            runtime_state: NodeRuntimeState {
                status: NodeStatus::Running,
                started_at: Some(now_ms),
                ..self.runtime_state
            },
            _state: PhantomData,
        }
    }

    /// 跳过执行（补偿策略）
    pub fn skip(self) -> WorkflowNodeState<Skipped> {
        let now_ms = current_epoch_ms() as i64;
        WorkflowNodeState {
            node_id: self.node_id,
            node: self.node,
            runtime_state: NodeRuntimeState {
                status: NodeStatus::Skipped,
                completed_at: Some(now_ms),
                ..self.runtime_state
            },
            _state: PhantomData,
        }
    }
}

// ── Running 状态实现 ──

impl WorkflowNodeState<Running> {
    /// 执行成功
    pub fn complete(self) -> WorkflowNodeState<Completed> {
        let now_ms = current_epoch_ms() as i64;
        WorkflowNodeState {
            node_id: self.node_id,
            node: self.node,
            runtime_state: NodeRuntimeState {
                status: NodeStatus::Completed,
                completed_at: Some(now_ms),
                attempts: 0,
                ..self.runtime_state
            },
            _state: PhantomData,
        }
    }

    /// 执行失败
    pub fn fail(self, error: String) -> WorkflowNodeState<Failed> {
        let now_ms = current_epoch_ms() as i64;
        WorkflowNodeState {
            node_id: self.node_id,
            node: self.node,
            runtime_state: NodeRuntimeState {
                status: NodeStatus::Failed,
                error: Some(error),
                completed_at: Some(now_ms),
                attempts: self.runtime_state.attempts + 1,
                ..self.runtime_state
            },
            _state: PhantomData,
        }
    }
}

// ── Failed 状态实现 ──

impl WorkflowNodeState<Failed> {
    /// 获取错误信息
    pub fn error(&self) -> Option<&str> {
        self.runtime_state.error.as_deref()
    }

    /// 获取失败次数
    pub fn attempts(&self) -> u32 {
        self.runtime_state.attempts
    }

    /// 重试（回到 Ready 状态）
    pub fn retry(self) -> WorkflowNodeState<Ready> {
        WorkflowNodeState {
            node_id: self.node_id,
            node: self.node,
            runtime_state: NodeRuntimeState {
                status: NodeStatus::Ready,
                error: None,
                completed_at: None,
                ..self.runtime_state
            },
            _state: PhantomData,
        }
    }
}

// ── 通用方法实现 ──

impl<State> WorkflowNodeState<State> {
    /// 获取节点 ID
    pub fn node_id(&self) -> &str {
        &self.node_id
    }

    /// 获取原始节点引用
    pub fn node(&self) -> &WorkflowNode {
        &self.node
    }

    /// 获取运行时状态引用
    pub fn runtime_state(&self) -> &NodeRuntimeState {
        &self.runtime_state
    }
}

impl<State: TerminalState> WorkflowNodeState<State> {
    /// 判断是否为终态（编译时保证）
    pub fn is_terminal(&self) -> bool {
        true
    }
}

// ── 类型别名（方便使用） ──

pub type PendingNode = WorkflowNodeState<Pending>;
pub type ReadyNode = WorkflowNodeState<Ready>;
pub type RunningNode = WorkflowNodeState<Running>;
pub type CompletedNode = WorkflowNodeState<Completed>;
pub type FailedNode = WorkflowNodeState<Failed>;
pub type SkippedNode = WorkflowNodeState<Skipped>;

// ── 桥接转换（与现有 NodeStatus 互操作） ──

/// 从 NodeRuntimeState 恢复 Typestate（用于从持久化状态重建）
///
/// 注意：此函数仅在运行时使用，绕过编译时检查。
/// 适用于从数据库恢复工作流状态等场景。
pub fn restore_typestate(node: WorkflowNode, state: NodeRuntimeState) -> Box<dyn AnyNodeState> {
    let node_id = node.base_id().to_string();
    match state.status {
        NodeStatus::Pending => Box::new(WorkflowNodeState {
            node_id,
            node,
            runtime_state: state,
            _state: PhantomData::<Pending>,
        }),
        NodeStatus::Ready => Box::new(WorkflowNodeState {
            node_id,
            node,
            runtime_state: state,
            _state: PhantomData::<Ready>,
        }),
        NodeStatus::Running => Box::new(WorkflowNodeState {
            node_id,
            node,
            runtime_state: state,
            _state: PhantomData::<Running>,
        }),
        NodeStatus::Completed => Box::new(WorkflowNodeState {
            node_id,
            node,
            runtime_state: state,
            _state: PhantomData::<Completed>,
        }),
        NodeStatus::Failed => Box::new(WorkflowNodeState {
            node_id,
            node,
            runtime_state: state,
            _state: PhantomData::<Failed>,
        }),
        NodeStatus::Skipped => Box::new(WorkflowNodeState {
            node_id,
            node,
            runtime_state: state,
            _state: PhantomData::<Skipped>,
        }),
    }
}

/// 类型擦除接口：用于存储不同状态的 Typestate 节点
pub trait AnyNodeState {
    fn node_id(&self) -> &str;
    fn node(&self) -> &WorkflowNode;
    fn runtime_state(&self) -> &NodeRuntimeState;
    fn current_status(&self) -> NodeStatus;
}

impl<S> AnyNodeState for WorkflowNodeState<S> {
    fn node_id(&self) -> &str {
        &self.node_id
    }

    fn node(&self) -> &WorkflowNode {
        &self.node
    }

    fn runtime_state(&self) -> &NodeRuntimeState {
        &self.runtime_state
    }

    fn current_status(&self) -> NodeStatus {
        self.runtime_state.status
    }
}

// ── 内部追踪类型 ──

/// 断路器状态（按节点追踪）
#[derive(Debug, Clone)]
pub(crate) struct NodeCircuitBreaker {
    failure_count: u32,
    failure_threshold: u32,
    reset_timeout_ms: u64,
    opened_at: Option<u64>,
}

impl NodeCircuitBreaker {
    pub(crate) fn new() -> Self {
        Self { failure_count: 0, failure_threshold: 3, reset_timeout_ms: 60_000, opened_at: None }
    }

    pub(crate) fn is_open(&self, now_ms: u64) -> bool {
        if let Some(opened_at) = self.opened_at {
            now_ms < opened_at + self.reset_timeout_ms
        } else {
            false
        }
    }

    pub(crate) fn record_success(&mut self) {
        self.failure_count = 0;
        self.opened_at = None;
    }

    pub(crate) fn record_failure(&mut self, now_ms: u64) {
        self.failure_count += 1;
        if self.failure_count >= self.failure_threshold {
            self.opened_at = Some(now_ms);
        }
    }
}

pub(crate) fn compute_backoff(
    backoff_type: BackoffType,
    base_delay_ms: u64,
    max_delay_ms: u64,
    attempt: u32,
) -> u64 {
    let delay = match backoff_type {
        BackoffType::Fixed => base_delay_ms,
        BackoffType::Linear => base_delay_ms.saturating_mul(attempt as u64),
        BackoffType::Exponential => {
            let exp = 1u64.checked_shl(attempt).unwrap_or(u64::MAX);
            base_delay_ms.saturating_mul(exp)
        },
    };
    delay.min(max_delay_ms)
}

pub(crate) struct NodeResult {
    pub(crate) node_id: String,
    pub(crate) node: WorkflowNode,
    pub(crate) input_snapshot: serde_json::Value,
    pub(crate) started_at: i64,
    pub(crate) elapsed_ms: u64,
    pub(crate) dispatch_result: Result<Result<NodeOutput, NodeError>, tokio::time::error::Elapsed>,
}

// ── 测试 ──

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn breaker_default_is_not_open() {
        let cb = NodeCircuitBreaker::new();
        assert!(!cb.is_open(0));
    }

    #[test]
    fn breaker_opens_after_threshold() {
        let mut cb = NodeCircuitBreaker::new();
        let mut now = 1000;
        cb.record_failure(now); // 1
        cb.record_failure(now); // 2
        cb.record_failure(now); // 3 → opens
        now += 1;
        assert!(cb.is_open(now));
    }

    #[test]
    fn breaker_resets_after_success() {
        let mut cb = NodeCircuitBreaker::new();
        let now = 1000;
        cb.record_failure(now);
        cb.record_failure(now);
        cb.record_failure(now); // opens
        assert!(cb.is_open(now + 1));
        cb.record_success();
        assert!(!cb.is_open(now + 1));
    }

    #[test]
    fn breaker_half_open_after_timeout() {
        let mut cb = NodeCircuitBreaker::new();
        let open_time = 1000;
        cb.record_failure(open_time);
        cb.record_failure(open_time);
        cb.record_failure(open_time); // opens at open_time
        // Still open right after
        assert!(cb.is_open(open_time + 1000));
        // Closed after reset timeout (60_000 ms)
        assert!(!cb.is_open(open_time + 61_000));
    }

    #[test]
    fn backoff_fixed() {
        assert_eq!(compute_backoff(BackoffType::Fixed, 1000, 10_000, 1), 1000);
        assert_eq!(compute_backoff(BackoffType::Fixed, 1000, 10_000, 5), 1000);
    }

    #[test]
    fn backoff_linear() {
        assert_eq!(compute_backoff(BackoffType::Linear, 1000, 10_000, 1), 1000);
        assert_eq!(compute_backoff(BackoffType::Linear, 1000, 10_000, 3), 3000);
        assert_eq!(compute_backoff(BackoffType::Linear, 1000, 10_000, 20), 10_000); // capped
    }

    #[test]
    fn backoff_exponential() {
        assert_eq!(compute_backoff(BackoffType::Exponential, 1000, 10_000, 0), 1000); // 2^0 = 1
        assert_eq!(compute_backoff(BackoffType::Exponential, 1000, 10_000, 1), 2000); // 2^1 = 2
        assert_eq!(compute_backoff(BackoffType::Exponential, 1000, 10_000, 2), 4000); // 2^2 = 4
        assert_eq!(compute_backoff(BackoffType::Exponential, 1000, 10_000, 4), 10_000); // 2^4=16 → 16000 capped
    }
}

// ── Typestate 测试 ──

#[cfg(test)]
mod typestate_tests {
    use super::*;
    use axagent_harness::workflow_types::{RetryConfig, ToolNodeConfig, WorkflowNodeBase};

    /// 创建一个测试用的 WorkflowNode
    fn create_test_node() -> WorkflowNode {
        WorkflowNode::Tool(axagent_harness::workflow_types::ToolNode {
            base: WorkflowNodeBase {
                id: "test_node".to_string(),
                title: "Test Node".to_string(),
                description: None,
                position: Default::default(),
                retry: RetryConfig::default(),
                timeout: None,
                enabled: true,
                parent_id: None,
                compensation: None,
                continue_on_fail: false,
            },
            config: ToolNodeConfig {
                tool_name: "test_tool".to_string(),
                input_mapping: Default::default(),
                output_var: "result".to_string(),
            },
        })
    }

    #[test]
    fn test_pending_to_ready() {
        let node = create_test_node();
        let pending = PendingNode::new(node);
        assert_eq!(pending.node_id(), "test_node");
        assert_eq!(pending.runtime_state().status, NodeStatus::Pending);

        let ready = pending.mark_ready();
        assert_eq!(ready.node_id(), "test_node");
        assert_eq!(ready.runtime_state().status, NodeStatus::Ready);
    }

    #[test]
    fn test_ready_to_running() {
        let node = create_test_node();
        let pending = PendingNode::new(node);
        let ready = pending.mark_ready();
        assert_eq!(ready.runtime_state().status, NodeStatus::Ready);

        let running = ready.start();
        assert_eq!(running.node_id(), "test_node");
        assert_eq!(running.runtime_state().status, NodeStatus::Running);
        assert!(running.runtime_state().started_at.is_some());
    }

    #[test]
    fn test_running_to_completed() {
        let node = create_test_node();
        let pending = PendingNode::new(node);
        let ready = pending.mark_ready();
        let running = ready.start();
        assert_eq!(running.runtime_state().status, NodeStatus::Running);

        let completed = running.complete();
        assert_eq!(completed.node_id(), "test_node");
        assert_eq!(completed.runtime_state().status, NodeStatus::Completed);
        assert!(completed.runtime_state().completed_at.is_some());
    }

    #[test]
    fn test_running_to_failed() {
        let node = create_test_node();
        let pending = PendingNode::new(node);
        let ready = pending.mark_ready();
        let running = ready.start();

        let failed = running.fail("test error".to_string());
        assert_eq!(failed.node_id(), "test_node");
        assert_eq!(failed.runtime_state().status, NodeStatus::Failed);
        assert_eq!(failed.error(), Some("test error"));
        assert_eq!(failed.attempts(), 1);
    }

    #[test]
    fn test_failed_retry() {
        let node = create_test_node();
        let pending = PendingNode::new(node);
        let ready = pending.mark_ready();
        let running = ready.start();
        let failed = running.fail("error".to_string());

        assert_eq!(failed.attempts(), 1);

        let retried = failed.retry();
        assert_eq!(retried.runtime_state().status, NodeStatus::Ready);
        assert!(retried.runtime_state().error.is_none());
        assert!(retried.runtime_state().completed_at.is_none());
    }

    #[test]
    fn test_ready_to_skipped() {
        let node = create_test_node();
        let pending = PendingNode::new(node);
        let ready = pending.mark_ready();

        let skipped = ready.skip();
        assert_eq!(skipped.node_id(), "test_node");
        assert_eq!(skipped.runtime_state().status, NodeStatus::Skipped);
    }

    #[test]
    fn test_terminal_state_check() {
        let node = create_test_node();

        let completed = WorkflowNodeState::<Completed> {
            node_id: "test".to_string(),
            node: node.clone(),
            runtime_state: NodeRuntimeState { status: NodeStatus::Completed, ..Default::default() },
            _state: PhantomData,
        };
        assert!(completed.is_terminal());

        let failed = WorkflowNodeState::<Failed> {
            node_id: "test".to_string(),
            node: node.clone(),
            runtime_state: NodeRuntimeState { status: NodeStatus::Failed, ..Default::default() },
            _state: PhantomData,
        };
        assert!(failed.is_terminal());

        let skipped = WorkflowNodeState::<Skipped> {
            node_id: "test".to_string(),
            node,
            runtime_state: NodeRuntimeState { status: NodeStatus::Skipped, ..Default::default() },
            _state: PhantomData,
        };
        assert!(skipped.is_terminal());
    }

    #[test]
    fn test_full_lifecycle() {
        let node = create_test_node();

        // Pending → Ready → Running → Completed
        let pending = PendingNode::new(node);
        let ready = pending.mark_ready();
        let running = ready.start();
        let completed = running.complete();

        assert_eq!(completed.node_id(), "test_node");
        assert_eq!(completed.runtime_state().status, NodeStatus::Completed);
    }

    #[test]
    fn test_failed_lifecycle_with_retry() {
        let node = create_test_node();

        // 第一次尝试失败
        let pending = PendingNode::new(node);
        let ready = pending.mark_ready();
        let running = ready.start();
        let failed = running.fail("first error".to_string());
        assert_eq!(failed.attempts(), 1);

        // 重试
        let ready2 = failed.retry();
        assert_eq!(ready2.runtime_state().status, NodeStatus::Ready);

        // 第二次尝试成功
        let running2 = ready2.start();
        let completed = running2.complete();
        assert_eq!(completed.runtime_state().status, NodeStatus::Completed);
    }

    #[test]
    fn test_any_node_state_trait() {
        let node = create_test_node();
        let pending = PendingNode::new(node);

        let any: Box<dyn AnyNodeState> = Box::new(pending);
        assert_eq!(any.node_id(), "test_node");
        assert_eq!(any.current_status(), NodeStatus::Pending);
    }

    #[test]
    fn test_restore_typestate() {
        let node = create_test_node();
        let state = NodeRuntimeState {
            status: NodeStatus::Running,
            started_at: Some(1000),
            ..Default::default()
        };

        let restored = restore_typestate(node, state);
        assert_eq!(restored.node_id(), "test_node");
        assert_eq!(restored.current_status(), NodeStatus::Running);
        assert_eq!(restored.runtime_state().started_at, Some(1000));
    }

    #[test]
    fn test_restore_all_states() {
        let node = create_test_node();

        for status in [
            NodeStatus::Pending,
            NodeStatus::Ready,
            NodeStatus::Running,
            NodeStatus::Completed,
            NodeStatus::Failed,
            NodeStatus::Skipped,
        ] {
            let state = NodeRuntimeState { status, ..Default::default() };
            let restored = restore_typestate(node.clone(), state);
            assert_eq!(restored.current_status(), status);
        }
    }

    // 编译时约束测试（这些测试验证编译器会阻止非法状态转移）
    // 注意：以下代码如果被取消注释会导致编译错误，这正是 Typestate 的目的！
    //
    // #[test]
    // fn test_cannot_start_from_pending() {
    //     let node = create_test_node();
    //     let pending = PendingNode::new(node);
    //     // 编译错误：Pending 没有 start() 方法
    //     // pending.start();
    // }
    //
    // #[test]
    // fn test_cannot_complete_without_result() {
    //     let node = create_test_node();
    //     let pending = PendingNode::new(node);
    //     let ready = pending.mark_ready();
    //     let running = ready.start();
    //     // 编译错误：只有 Running 有 complete() 方法
    //     // ready.complete();  // 不允许
    // }
}

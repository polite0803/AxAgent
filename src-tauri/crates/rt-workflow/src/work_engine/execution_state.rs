// SPDX-License-Identifier: AGPL-3.0-only

// 运行时执行态 DTO 复用 harness(阶段 2 上移)
pub use axagent_harness::workflow_types::{
    ExecutionStatus, NodeExecutionRecord, PartialResultEvent,
};
// 错误上下文:重命名后的 harness 类型,rt-workflow 内部保留 ErrorContext 别名以兼容
use axagent_harness::workflow_types::WorkflowErrorContext;

use serde::{Deserialize, Serialize};

use crate::work_engine::node_executor_trait::NodeOutput;

// 兼容别名:rt-workflow 内部代码继续用 ErrorContext 名称,实际指向 harness 的 WorkflowErrorContext
pub(crate) type ErrorContext = WorkflowErrorContext;

use std::collections::HashMap;
use std::sync::Arc;

use super::executors::{PlanCallbacks, SubWorkflowCallback, ToolCallback};
use super::prompt_template::CompiledPrompt;

/// 4.1.6 P3:暂停原因
///
/// 显式区分触发暂停的不同场景,便于日志/审计/UI 展示。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PauseReason {
    /// 断点命中
    Breakpoint,
    /// 人工审批(HITL ApprovalExecutor 触发)
    Approval,
    /// Loop 节点人工审查(每轮迭代后等待用户确认)
    LoopReview,
    /// 用户手动调用 pause()
    Manual,
}

/// 4.1.6 P3:暂停状态(显式封装)
///
/// 替代原先 `pause_signal: Option<Arc<Notify>>` + `ExecutionStatus::Paused` 的隐式联合。
/// 把暂停原因和恢复信号绑定在一个结构内,作为暂停信息的唯一权威来源。
#[derive(Debug, Clone)]
pub struct PauseState {
    /// 暂停原因
    pub reason: PauseReason,
    /// 恢复信号:resume 时调用 `signal.notify_waiters()` / `notify_one()`
    pub signal: Arc<tokio::sync::Notify>,
}

impl PauseState {
    /// 创建一个新的暂停状态
    pub fn new(reason: PauseReason) -> Self {
        Self { reason, signal: Arc::new(tokio::sync::Notify::new()) }
    }

    /// 获取恢复信号的引用
    pub fn signal(&self) -> &Arc<tokio::sync::Notify> {
        &self.signal
    }
}

/// 运行时回调容器（非序列化，仅在内存中传递）
#[derive(Clone)]
pub struct ExecutionContextCallbacks {
    /// 触发器管理器（供 TriggerExecutor 注册/激活触发器）
    pub trigger_manager: Option<Arc<crate::trigger::TriggerManager>>,
    /// 按工具名注册的 handler 映射（多路注册，优先级最高）
    pub tool_handlers: HashMap<String, ToolCallback>,
    /// 旧版全局回调（fallback，tool_handlers 未命中时使用）
    pub tool_fallback: Option<ToolCallback>,
    pub subworkflow: Option<SubWorkflowCallback>,
    /// Loop 节点内部驱动 body_steps 迭代时使用的调度回调。
    /// 接收 (body_step_node_id, mutable_context) 返回该 step 的 NodeOutput。
    /// 与 SubWorkflowCallback 同样的注入模式：引擎在构造 exec_ctx 时填入。
    pub loop_body_dispatch: Option<LoopBodyDispatchFn>,
    /// Loop 检查点持久化回调（save/load/delete）。LoopExecutor 通过它
    /// 写 `loop_checkpoints` 表。
    pub loop_checkpoint: Option<LoopCheckpointOps>,
    /// Swarm/Debate 容器内部驱动子节点（agent_steps / debater_steps）时使用的调度回调。
    /// 签名与 `loop_body_dispatch` 一致（按 step_id + ctx 调度单节点），
    /// 单独命名字段以区分语义：Loop 是迭代驱动，Swarm/Debate 是多轮协作驱动。
    pub debate_body_dispatch: Option<LoopBodyDispatchFn>,
}

impl std::fmt::Debug for ExecutionContextCallbacks {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExecutionContextCallbacks")
            .field(
                "trigger_manager",
                &self.trigger_manager.as_ref().map(|_| "Some(TriggerManager)"),
            )
            .field("tool_handlers", &self.tool_handlers.len())
            .field("tool_fallback", &self.tool_fallback.is_some())
            .field("subworkflow", &self.subworkflow.is_some())
            .field("loop_body_dispatch", &self.loop_body_dispatch.is_some())
            .field("loop_checkpoint", &self.loop_checkpoint.is_some())
            .field("debate_body_dispatch", &self.debate_body_dispatch.is_some())
            .finish()
    }
}

/// Loop 内部驱动回调：在 LoopExecutor 内被调用，按 (body_step_node_id, ctx)
/// 分发单个 body 节点。返回 dispatch 的 NodeOutput 供 LoopExecutor 汇总。
///
/// 之所以走回调而不是直接拿 dispatcher：
///  1) executor 是 stateless trait object，不能反向拿到 dispatcher；
///  2) 引擎希望保留对 body 节点执行的统一埋点（progress_callback / 节点状态
///     切换 / node_records 等），回调里集中处理比 executor 自调用更合适。
pub type LoopBodyDispatchFn = Arc<
    dyn Fn(
            String,
            ExecutionState,
        ) -> std::pin::Pin<
            Box<
                dyn std::future::Future<
                        Output = Result<NodeOutput, super::node_executor_trait::NodeError>,
                    > + Send,
            >,
        > + Send
        + Sync,
>;

/// Loop 检查点持久化回调：save/load/delete 三件套。
///
/// 引擎在主循环入口把 `axagent_dao::repo::loop_checkpoint::*` 包装成这个结构
/// 注入到 `ExecutionState.callbacks.loop_checkpoint`，LoopExecutor 通过它读写
/// `loop_checkpoints` 表。回调形式避免在 executor 静态结构里持有 db 句柄。
type LoopCheckpointSaveFn = dyn Fn(
        axagent_harness::workflow_types::LoopCheckpoint,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), String>> + Send>>
    + Send
    + Sync;

type LoopCheckpointLoadFn = dyn Fn(
        String,
        String,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = Result<
                        Option<axagent_harness::workflow_types::LoopCheckpoint>,
                        String,
                    >,
                > + Send,
        >,
    > + Send
    + Sync;

type LoopCheckpointDeleteFn = dyn Fn(
        String,
        String,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), String>> + Send>>
    + Send
    + Sync;

#[derive(Clone)]
pub struct LoopCheckpointOps {
    pub save: Arc<LoopCheckpointSaveFn>,
    pub load: Arc<LoopCheckpointLoadFn>,
    pub delete: Arc<LoopCheckpointDeleteFn>,
}

impl std::fmt::Debug for LoopCheckpointOps {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LoopCheckpointOps").finish()
    }
}

/// Runtime execution state for a workflow
#[derive(Clone, Serialize, Deserialize)]
// P2-21: ExecutionState Clone 成本说明
//
// 现状：`ExecutionState` 派生 `#[derive(Clone)]`，每个字段都是 owned（非 Arc）。
// 这意味着每次 `context.clone()` 都会复制整张 variables HashMap、callbacks 列表、
// partial_result 树等。在 1000 步工作流中累计可能产生 100MB+ 临时分配。
//
// 性能热点：
// 1. `LoopExecutor`：每轮 body_steps 都会 `context.clone()` 一次（现已改为 Arc 共享）
// 2. `SubWorkflow`：child 节点拿到的是父 execution 的 clone
// 3. `Parallel`：每个分支拿父 snapshot（受 auto_input_from_parent 控制）
//
// 优化建议（按 ROI 排序）：
// - 方案 A（最小改动）：把 `variables: HashMap<String, Value>` 改为
//   `variables: Arc<HashMap<String, Value>>`。子节点 clone 仅增加 Arc 引用计数。
//   body_step 修改时用 `Arc::make_mut` 写时复制。
//   实施成本：~30 行（ExecutionState + 全部 executor 的写点）。
// - 方案 B（激进）：整体改 `Rc<RefCell<...>>`（sync 路径）或 `Arc<RwLock<...>>`（async）。
//   优点：彻底零拷贝。缺点：会破坏所有 executor 的签名（`&ExecutionState` → 锁）。
// - 方案 C（保守）：维持现状，但在 Loop / Parallel 路径上做变量裁剪（projection），
//   只 clone 当前 body 真正用到的 key（`loop_partial_txs` 已部分实现）。
//
// 推荐方案 A，预期收益：1000 步 × 200 variables 工作流，从 ~800ms 降至 ~50ms。

pub struct ExecutionState {
    #[serde(skip, default)]
    pub callbacks: Option<ExecutionContextCallbacks>,
    /// 编译后的 prompt 模板（node_id -> CompiledPrompt），引擎注入，不序列化
    #[serde(skip, default)]
    pub compiled_prompts: Option<HashMap<String, CompiledPrompt>>,
    /// 取消令牌（引擎注入，不序列化）
    #[serde(skip, default)]
    pub cancel_token: Option<tokio_util::sync::CancellationToken>,
    /// 干跑模式（不序列化，引擎注入）
    #[serde(skip, default)]
    pub dry_run: bool,
    /// 断点集（节点 ID 集合，不序列化）
    #[serde(skip, default)]
    pub breakpoints: std::collections::HashSet<String>,
    /// 暂停状态(显式封装暂停原因 + 恢复信号)
    ///
    /// 4.1.6 P3:替代原先 `pause_signal` + `ExecutionStatus::Paused` 的隐式联合。
    /// 当此字段为 `Some(pause_state)` 时,表示执行流已被暂停并等待 resume:
    /// - `pause_state.reason` 说明暂停原因(断点/审批/Loop审查/手动)
    /// - `pause_state.signal` 是恢复信号,`signal.notified().await` 阻塞直到 resume
    ///
    /// `ExecutionStatus::Paused` 保留作为广义状态机标记(供外部查询),
    /// 但暂停的详细信息(原因 + 信号)以此字段为唯一权威来源。
    #[serde(skip, default)]
    pub pause_state: Option<PauseState>,
    /// Plan 模式回调（引擎从 RunOptions 注入，executor 读取）
    #[serde(skip, default)]
    pub plan_callbacks: Option<PlanCallbacks>,
    /// 工具级权限约束（可选，None = 不施加额外约束）。
    /// 由调用方在创建 ExecutionState 时注入，agent/tool executor 在执行时读取。
    #[serde(skip, default)]
    pub tool_permissions: Option<Arc<axagent_harness::tool::ToolPermissions>>,
    /// 业务规则引擎（可选，None = 不执行任何业务规则检查）。
    /// 硬约束，在执行层直接拦截违规操作（LLM 无法绕过）。
    /// 与 domain_constraints（软约束，仅作为 LLM prompt 建议）共存。
    #[serde(skip, default)]
    pub business_rule_engine: Option<Arc<crate::business_rules::BusinessRuleEngine>>,
    /// 凭证管理器（可选，None = 不使用凭证）。
    /// 执行器通过它按 credential_id 懒加载并解密 DatabaseConnection / Smtp / ApiKey 等凭证。
    #[serde(skip, default)]
    pub credential_manager: Option<axagent_harness::SharedCredentialService>,
    /// 工具注册表（可选，设置后 tool_executor 优先通过 ToolRegistry.execute_tool() 执行工具）
    #[serde(skip, default)]
    pub tool_registry: Option<Arc<dyn axagent_harness::ToolRegistry>>,
    /// partial_result 流式事件广播通道。LoopExecutor 在每次迭代完成后 send。
    /// 接收端由 WorkEngine.subscribe_partial_results 暴露给外部。
    #[serde(skip, default)]
    pub partial_result_tx: Option<tokio::sync::broadcast::Sender<PartialResultEvent>>,
    /// Loop interrupt 等待信号。LoopExecutor 检测到 interrupt 时调用 `notified().await`，
    /// resume API（cmd_resume_loop_iteration / engine.resume_loop_iteration）
    /// 通过 `notify_waiters()` 唤醒。
    #[serde(skip, default)]
    pub interrupt_signal: Option<std::sync::Arc<tokio::sync::Notify>>,
    pub execution_id: String,
    pub workflow_id: String,
    pub status: ExecutionStatus,
    pub input_params: serde_json::Value,
    pub variables: std::collections::HashMap<String, serde_json::Value>,
    pub node_records: Vec<NodeExecutionRecord>,
    pub current_node_id: Option<String>,
    pub parent_execution_id: Option<String>,
    /// 按节点名称索引的历史输出，供表达式引擎 $node["NodeName"] 引用
    #[serde(skip, default)]
    pub node_outputs: std::collections::HashMap<String, serde_json::Value>,
    pub total_time_ms: u64,
    pub created_at: i64,
    pub updated_at: i64,
    /// 最后一次节点失败的错误上下文（供 Error Workflow 引用）。
    #[serde(skip, default)]
    pub last_error: Option<ErrorContext>,
    /// 错误工作流 ID（模板级配置，引擎在 run_workflow 时注入）。
    #[serde(skip, default)]
    pub error_workflow_id: Option<String>,
}

impl ExecutionState {
    pub fn new(execution_id: String, workflow_id: String, input_params: serde_json::Value) -> Self {
        let now = chrono::Utc::now().timestamp_millis();
        Self {
            execution_id,
            workflow_id,
            status: ExecutionStatus::Running,
            input_params,
            variables: std::collections::HashMap::new(),
            node_records: Vec::new(),
            current_node_id: None,
            parent_execution_id: None,
            callbacks: None,
            compiled_prompts: None,
            cancel_token: None,
            dry_run: false,
            breakpoints: std::collections::HashSet::new(),
            pause_state: None,
            plan_callbacks: None,
            tool_permissions: None,
            business_rule_engine: None,
            tool_registry: None,
            partial_result_tx: None,
            interrupt_signal: None,
            credential_manager: None,
            node_outputs: std::collections::HashMap::new(),
            total_time_ms: 0,
            created_at: now,
            updated_at: now,
            last_error: None,
            error_workflow_id: None,
        }
    }

    /// 从快照重建 ExecutionState（崩溃后恢复时使用）
    ///
    /// 仅恢复可序列化的纯数据字段，运行时原语（callbacks/cancel_token 等）
    /// 由 WorkEngine 在调用方重新注入。
    pub fn from_snapshot(snapshot: ExecutionStateSnapshot) -> Self {
        Self {
            execution_id: snapshot.execution_id,
            workflow_id: snapshot.workflow_id,
            status: snapshot.status,
            input_params: snapshot.input_params,
            variables: snapshot.variables,
            node_records: snapshot.node_records,
            current_node_id: snapshot.current_node_id,
            parent_execution_id: snapshot.parent_execution_id,
            callbacks: None,
            compiled_prompts: None,
            cancel_token: None,
            dry_run: false,
            breakpoints: std::collections::HashSet::new(),
            pause_state: snapshot.pause_reason.map(PauseState::new),
            plan_callbacks: None,
            tool_permissions: None,
            business_rule_engine: None,
            tool_registry: None,
            partial_result_tx: None,
            interrupt_signal: None,
            credential_manager: None,
            node_outputs: snapshot.node_outputs,
            total_time_ms: snapshot.total_time_ms,
            created_at: snapshot.created_at,
            updated_at: snapshot.updated_at,
            last_error: None,
            error_workflow_id: None,
        }
    }

    /// Set a workflow variable
    pub fn set_variable(&mut self, key: String, value: serde_json::Value) {
        self.variables.insert(key, value);
    }

    /// Get a workflow variable
    pub fn get_variable(&self, key: &str) -> Option<&serde_json::Value> {
        self.variables.get(key)
    }

    /// 4.1.6 P3:获取暂停恢复信号(从 pause_state 派生)
    ///
    /// 替代原先直接访问 `pause_signal` 字段。当 `pause_state` 为 Some 时返回其信号克隆。
    pub fn pause_signal(&self) -> Option<Arc<tokio::sync::Notify>> {
        self.pause_state.as_ref().map(|p| p.signal.clone())
    }

    /// 4.1.6 P3:获取暂停原因(若已暂停)
    pub fn pause_reason(&self) -> Option<PauseReason> {
        self.pause_state.as_ref().map(|p| p.reason)
    }

    /// 4.1.6 P3:进入暂停状态
    ///
    /// 若已有 pause_state 则复用其信号(保留 reason 为首次原因),否则创建新的。
    /// 返回恢复信号供调用方 `notified().await`。
    pub fn enter_pause(&mut self, reason: PauseReason) -> Arc<tokio::sync::Notify> {
        if let Some(existing) = &self.pause_state {
            return existing.signal.clone();
        }
        let state = PauseState::new(reason);
        let signal = state.signal.clone();
        self.pause_state = Some(state);
        signal
    }

    /// 4.1.6 P3:清除暂停状态(resume 后调用)
    pub fn clear_pause(&mut self) {
        self.pause_state = None;
    }

    /// Add a node execution record
    ///
    /// 副作用:当节点成功完成时,同步把 output 写入 `node_outputs`,
    /// 供表达式引擎 `$node["NodeId"]` / `$node["NodeName"]` 引用。
    /// 同时以 node_id 和 node_name(若存在且与 node_id 不同)作为 key,
    /// 兼容用户按 ID 或按名称引用的两种习惯。
    pub fn add_node_record(&mut self, record: NodeExecutionRecord) {
        // 填充 node_outputs(修复 dead field:此前从未被写入)
        if record.status == "completed"
            && let Some(output) = record.output.clone()
        {
            self.node_outputs.insert(record.node_id.clone(), output.clone());
            if let Some(name) = record.node_name.as_ref()
                && name != &record.node_id
            {
                self.node_outputs.insert(name.clone(), output);
            }
        }
        self.node_records.push(record);
        self.updated_at = chrono::Utc::now().timestamp_millis();
    }
}

impl std::fmt::Debug for ExecutionState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExecutionState")
            .field("execution_id", &self.execution_id)
            .field("workflow_id", &self.workflow_id)
            .field("status", &self.status)
            .field("current_node_id", &self.current_node_id)
            .field("dry_run", &self.dry_run)
            .field("total_time_ms", &self.total_time_ms)
            .field("variables_count", &self.variables.len())
            .field("node_records_count", &self.node_records.len())
            .field("tool_registry", &self.tool_registry.as_ref().map(|_| "Some(ToolRegistry)"))
            .field("partial_result_tx", &self.partial_result_tx.as_ref().map(|_| "Some(broadcast)"))
            .field("interrupt_signal", &self.interrupt_signal.as_ref().map(|_| "Some(Notify)"))
            .finish()
    }
}

// ── ExecutionStateSnapshot: 崩溃后恢复用的可序列化快照 ──

/// 可序列化的执行状态快照，用于崩溃后恢复。
///
/// 与 ExecutionState 的区别：
/// - 只包含可序列化的纯数据字段（无 Arc/Notify/CancellationToken 等运行时原语）
/// - 恢复时由 WorkEngine 从 Snapshot 重建完整的 ExecutionState（重新注入运行时字段）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionStateSnapshot {
    pub execution_id: String,
    pub workflow_id: String,
    pub status: ExecutionStatus,
    pub input_params: serde_json::Value,
    pub variables: HashMap<String, serde_json::Value>,
    pub node_records: Vec<NodeExecutionRecord>,
    pub node_outputs: HashMap<String, serde_json::Value>,
    pub current_node_id: Option<String>,
    pub parent_execution_id: Option<String>,
    pub total_time_ms: u64,
    pub created_at: i64,
    pub updated_at: i64,
    /// 暂停原因（用于恢复时重建 PauseState）
    pub pause_reason: Option<PauseReason>,
}

impl From<&ExecutionState> for ExecutionStateSnapshot {
    fn from(state: &ExecutionState) -> Self {
        Self {
            execution_id: state.execution_id.clone(),
            workflow_id: state.workflow_id.clone(),
            status: state.status.clone(),
            input_params: state.input_params.clone(),
            variables: state.variables.clone(),
            node_records: state.node_records.clone(),
            node_outputs: state.node_outputs.clone(),
            current_node_id: state.current_node_id.clone(),
            parent_execution_id: state.parent_execution_id.clone(),
            total_time_ms: state.total_time_ms,
            created_at: state.created_at,
            updated_at: state.updated_at,
            pause_reason: state.pause_reason(),
        }
    }
}

impl From<&mut ExecutionState> for ExecutionStateSnapshot {
    fn from(state: &mut ExecutionState) -> Self {
        Self::from(&*state)
    }
}

impl ExecutionStateSnapshot {
    /// 序列化为 JSON 字符串
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// 从 JSON 字符串反序列化
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_state() -> ExecutionState {
        let mut state = ExecutionState::new(
            "exec-001".to_string(),
            "wf-001".to_string(),
            serde_json::json!({}),
        );
        state.status = ExecutionStatus::Running;
        state.variables.insert("x".to_string(), serde_json::json!(42));
        state.current_node_id = Some("node-1".to_string());
        state.total_time_ms = 100;
        state
    }

    #[test]
    fn test_snapshot_roundtrip() {
        let state = make_test_state();
        let snapshot = ExecutionStateSnapshot::from(&state);
        let json = snapshot.to_json().unwrap();

        let restored = ExecutionStateSnapshot::from_json(&json).unwrap();
        assert_eq!(restored.execution_id, "exec-001");
        assert_eq!(restored.workflow_id, "wf-001");
        assert_eq!(restored.status, ExecutionStatus::Running);
        assert_eq!(restored.current_node_id, Some("node-1".to_string()));
        assert_eq!(restored.total_time_ms, 100);
        assert_eq!(restored.variables.get("x"), Some(&serde_json::json!(42)));
    }

    #[test]
    fn test_snapshot_from_mut_ref() {
        let mut state = make_test_state();
        let snapshot: ExecutionStateSnapshot = (&mut state).into();
        assert_eq!(snapshot.execution_id, "exec-001");
    }

    #[test]
    fn test_from_snapshot_rebuilds_state() {
        let state = make_test_state();
        let snapshot = ExecutionStateSnapshot::from(&state);

        // 模拟崩溃恢复：从快照重建
        let restored = ExecutionState::from_snapshot(snapshot);
        assert_eq!(restored.execution_id, "exec-001");
        assert_eq!(restored.workflow_id, "wf-001");
        assert_eq!(restored.status, ExecutionStatus::Running);
        // 运行时字段应为 None
        assert!(restored.cancel_token.is_none());
        assert!(restored.callbacks.is_none());
    }

    #[test]
    fn test_snapshot_with_paused_status() {
        let mut state = make_test_state();
        state.status = ExecutionStatus::Paused;

        // 模拟暂停
        state.pause_state = Some(PauseState::new(PauseReason::Manual));

        let snapshot = ExecutionStateSnapshot::from(&state);
        let json = snapshot.to_json().unwrap();
        let restored = ExecutionStateSnapshot::from_json(&json).unwrap();

        assert_eq!(restored.status, ExecutionStatus::Paused);
        assert_eq!(restored.pause_reason, Some(PauseReason::Manual));
    }

    #[test]
    fn test_execution_status_serialization() {
        let status = ExecutionStatus::Completed;
        let json = serde_json::to_string(&status).unwrap();
        // 默认 serde 表示为枚举索引值
        let deser: ExecutionStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(deser, ExecutionStatus::Completed);

        // Display 表示为小写字符串
        assert_eq!(format!("{}", status), "completed");
    }

    #[test]
    fn test_pause_reason_serialization() {
        let reason = PauseReason::Manual;
        let json = serde_json::to_string(&reason).unwrap();
        assert!(!json.is_empty());

        let deser: PauseReason = serde_json::from_str(&json).unwrap();
        assert_eq!(deser, PauseReason::Manual);
    }

    #[test]
    fn test_node_records_snapshot() {
        let mut state = make_test_state();
        state.node_records.push(NodeExecutionRecord {
            node_id: "node-1".to_string(),
            node_type: "llm".to_string(),
            node_name: Some("LLM 节点".to_string()),
            status: "completed".to_string(),
            input: Some(serde_json::json!({"prompt": "hello"})),
            output: Some(serde_json::json!({"response": "hi"})),
            execution_time_ms: Some(50),
            error: None,
            started_at: 1000,
            completed_at: Some(1050),
            parent_execution_id: None,
            sub_workflow_id: None,
        });

        let snapshot = ExecutionStateSnapshot::from(&state);
        let json = snapshot.to_json().unwrap();
        let restored = ExecutionStateSnapshot::from_json(&json).unwrap();

        assert_eq!(restored.node_records.len(), 1);
        assert_eq!(restored.node_records[0].node_id, "node-1");
        assert_eq!(restored.node_records[0].status, "completed");
    }
}

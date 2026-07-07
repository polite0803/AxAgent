// SPDX-License-Identifier: AGPL-3.0-only

use serde::{Deserialize, Serialize};

use crate::work_engine::error_handling::ErrorContext;
use crate::work_engine::node_executor_trait::NodeOutput;

/// Overall execution status of a workflow
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionStatus {
    Running,
    Paused,
    Completed,
    PartiallyCompleted,
    Failed,
    Cancelled,
}

impl std::fmt::Display for ExecutionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExecutionStatus::Running => write!(f, "running"),
            ExecutionStatus::Paused => write!(f, "paused"),
            ExecutionStatus::Completed => write!(f, "completed"),
            ExecutionStatus::PartiallyCompleted => write!(f, "partially_completed"),
            ExecutionStatus::Failed => write!(f, "failed"),
            ExecutionStatus::Cancelled => write!(f, "cancelled"),
        }
    }
}

/// Record of a single node execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeExecutionRecord {
    pub node_id: String,
    pub node_type: String,
    pub node_name: Option<String>,
    pub status: String,
    pub input: Option<serde_json::Value>,
    pub output: Option<serde_json::Value>,
    pub execution_time_ms: Option<u64>,
    pub error: Option<String>,
    pub started_at: i64,
    pub completed_at: Option<i64>,
    pub parent_execution_id: Option<String>,
    pub sub_workflow_id: Option<String>,
}

/// 单次 Loop 迭代产出的 partial_result 事件。
///
/// 每次 LoopExecutor 完成一轮迭代后通过 `partial_result_tx` 广播。
/// 前端可订阅 `execution_id + node_id` 维度的 channel 实时刷新进度面板。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartialResultEvent {
    pub execution_id: String,
    pub node_id: String,
    /// 0-based 迭代下标。
    pub iter_index: u32,
    /// 当前元素（item），与 `iter_input_var` 数组中第 `iter_index` 个元素一致。
    pub item: serde_json::Value,
    /// body 最后一节点的输出（聚合视角下的"本轮结果"）。
    pub step_output: serde_json::Value,
    /// 累计 partial_result 数组（长度 = iter_index + 1）。
    pub cumulative_partial: Vec<serde_json::Value>,
    /// 触发本事件的源：正常完成 / interrupt / 错误。
    pub phase: String,
    /// 时间戳（毫秒）。
    pub emitted_at_ms: i64,
}

use std::collections::HashMap;
use std::sync::Arc;

use super::executors::{PlanCallbacks, SubWorkflowCallback, ToolCallback};
use super::prompt_template::CompiledPrompt;

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
}

impl std::fmt::Debug for ExecutionContextCallbacks {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExecutionContextCallbacks")
            .field(
                "trigger_manager",
                &self
                    .trigger_manager
                    .as_ref()
                    .map(|_| "Some(TriggerManager)"),
            )
            .field("tool_handlers", &self.tool_handlers.len())
            .field("tool_fallback", &self.tool_fallback.is_some())
            .field("subworkflow", &self.subworkflow.is_some())
            .field("loop_body_dispatch", &self.loop_body_dispatch.is_some())
            .field("loop_checkpoint", &self.loop_checkpoint.is_some())
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
    /// 暂停信号（引擎注入，断点命中时等待，resume 时通知）
    #[serde(skip, default)]
    pub pause_signal: Option<std::sync::Arc<tokio::sync::Notify>>,
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
    pub business_rule_engine: Option<Arc<axagent_harness::business_rules::BusinessRuleEngine>>,
    /// 凭证管理器（可选，None = 不使用凭证）。
    /// 执行器通过它按 credential_id 懒加载并解密 DatabaseConnection / Smtp / ApiKey 等凭证。
    #[serde(skip, default)]
    pub credential_manager: Option<Arc<axagent_harness::credential::CredentialManager>>,
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
            pause_signal: None,
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

    /// Set a workflow variable
    pub fn set_variable(&mut self, key: String, value: serde_json::Value) {
        self.variables.insert(key, value);
    }

    /// Get a workflow variable
    pub fn get_variable(&self, key: &str) -> Option<&serde_json::Value> {
        self.variables.get(key)
    }

    /// Add a node execution record
    pub fn add_node_record(&mut self, record: NodeExecutionRecord) {
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

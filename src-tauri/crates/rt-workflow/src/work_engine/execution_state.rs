use serde::{Deserialize, Serialize};

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

use std::collections::HashMap;
use std::sync::Arc;

use super::executors::{PlanCallbacks, SubWorkflowCallback, ToolCallback};
use super::prompt_template::CompiledPrompt;

/// 运行时回调容器（非序列化，仅在内存中传递）
#[derive(Clone)]
pub struct ExecutionContextCallbacks {
    /// 按工具名注册的 handler 映射（多路注册，优先级最高）
    pub tool_handlers: HashMap<String, ToolCallback>,
    /// 旧版全局回调（fallback，tool_handlers 未命中时使用）
    pub tool_fallback: Option<ToolCallback>,
    pub subworkflow: Option<SubWorkflowCallback>,
}

impl std::fmt::Debug for ExecutionContextCallbacks {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExecutionContextCallbacks")
            .field("tool_handlers", &self.tool_handlers.len())
            .field("tool_fallback", &self.tool_fallback.is_some())
            .field("subworkflow", &self.subworkflow.is_some())
            .finish()
    }
}

/// Runtime execution state for a workflow
#[derive(Clone, Serialize, Deserialize)]
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
    /// 工具注册表（可选，设置后 tool_executor 优先通过 ToolRegistry.execute_tool() 执行工具）
    #[serde(skip, default)]
    pub tool_registry: Option<Arc<dyn axagent_harness::ToolRegistry>>,
    pub execution_id: String,
    pub workflow_id: String,
    pub status: ExecutionStatus,
    pub input_params: serde_json::Value,
    pub variables: std::collections::HashMap<String, serde_json::Value>,
    pub node_records: Vec<NodeExecutionRecord>,
    pub current_node_id: Option<String>,
    pub parent_execution_id: Option<String>,
    pub total_time_ms: u64,
    pub created_at: i64,
    pub updated_at: i64,
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
            total_time_ms: 0,
            created_at: now,
            updated_at: now,
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
            .finish()
    }
}

//! Workflow type definitions
//!
//! This module defines the core types used in workflow execution,
//! including nodes, variables, triggers, and execution states.

use serde::{Deserialize, Serialize};

use crate::consistency_check::ConsistencyCheckConfig;
use crate::hallucination_guard::HallucinationGuardConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub x: f64,
    pub y: f64,
}

impl Default for Position {
    fn default() -> Self {
        Self { x: 0.0, y: 0.0 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    pub enabled: bool,
    pub max_retries: u32,
    pub backoff_type: BackoffType,
    pub base_delay_ms: u64,
    pub max_delay_ms: u64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            max_retries: 3,
            backoff_type: BackoffType::Exponential,
            base_delay_ms: 1000,
            max_delay_ms: 30000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BackoffType {
    Linear,
    Exponential,
    Fixed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonSchema {
    #[serde(rename = "type")]
    pub schema_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub properties: Option<std::collections::HashMap<String, JsonSchemaProperty>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub required: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub items: Option<Box<JsonSchema>>,
}

/// 工具定义 —— 包含名称、描述和参数 JSON Schema。
///
/// 反序列化支持向后兼容：旧格式的纯字符串自动转为 ToolDef { name, ..Default::default() }。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDef {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<JsonSchema>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonSchemaProperty {
    #[serde(rename = "type")]
    pub schema_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enum_values: Option<Vec<serde_json::Value>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Variable {
    pub name: String,
    pub var_type: String,
    pub value: serde_json::Value,
    pub description: Option<String>,
    pub is_secret: bool,
}

/// 补偿策略：当节点失败时，如何处理其下游节点和输出
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CompensationStrategy {
    /// 仅删除该节点输出，不处理下游
    SkipWithWarning,
    /// 删除该节点输出，并标记所有下游 Pending/Ready 节点为 Skipped
    Rollback,
    /// 记录警告，需要人工介入处理
    Escalate,
}

/// 补偿配置：定义节点失败时的恢复策略
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompensationConfig {
    pub strategy: CompensationStrategy,
    /// 需要执行补偿的节点 ID 列表（预留扩展，当前由引擎根据 DAG 自动推导下游）
    #[serde(default)]
    pub compensation_nodes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowNodeBase {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub position: Position,
    pub retry: RetryConfig,
    pub timeout: Option<u64>,
    pub enabled: bool,
    /// 容器父节点 ID。此字段由前端在保存时注入，
    /// 用于将子节点（如 Parallel 分支步骤）定位到父容器内。
    #[serde(rename = "parentId", default)]
    pub parent_id: Option<String>,
    /// 节点失败时的补偿/回滚策略。None = 不执行任何补偿。
    #[serde(default)]
    pub compensation: Option<CompensationConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TriggerType {
    #[serde(rename = "manual")]
    Manual,
    #[serde(rename = "schedule")]
    Schedule,
    #[serde(rename = "webhook")]
    Webhook,
    #[serde(rename = "event")]
    Event,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerConfig {
    #[serde(rename = "type")]
    pub trigger_type: TriggerType,
    pub config: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManualTriggerConfig {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleTriggerConfig {
    pub cron: String,
    pub timezone: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookTriggerConfig {
    pub path: String,
    pub method: String,
    pub auth_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventTriggerConfig {
    pub event_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filter: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentRole {
    #[serde(rename = "researcher")]
    Researcher,
    #[serde(rename = "planner")]
    Planner,
    #[serde(rename = "developer")]
    Developer,
    #[serde(rename = "reviewer")]
    Reviewer,
    #[serde(rename = "synthesizer")]
    Synthesizer,
    #[serde(rename = "executor")]
    Executor,
    #[serde(rename = "coordinator")]
    Coordinator,
    #[serde(rename = "browser")]
    Browser,
}

impl AgentRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            AgentRole::Researcher => "researcher",
            AgentRole::Planner => "planner",
            AgentRole::Developer => "developer",
            AgentRole::Reviewer => "reviewer",
            AgentRole::Synthesizer => "synthesizer",
            AgentRole::Executor => "executor",
            AgentRole::Coordinator => "coordinator",
            AgentRole::Browser => "browser",
        }
    }

    pub fn try_from_str(s: &str) -> Option<Self> {
        match s {
            "researcher" => Some(AgentRole::Researcher),
            "planner" => Some(AgentRole::Planner),
            "developer" => Some(AgentRole::Developer),
            "reviewer" => Some(AgentRole::Reviewer),
            "synthesizer" => Some(AgentRole::Synthesizer),
            "executor" => Some(AgentRole::Executor),
            "coordinator" => Some(AgentRole::Coordinator),
            "browser" => Some(AgentRole::Browser),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OutputMode {
    #[serde(rename = "json")]
    Json,
    #[serde(rename = "text")]
    Text,
    #[serde(rename = "artifact")]
    Artifact,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentNodeConfig {
    pub system_prompt: String,
    pub context_sources: Vec<String>,
    /// 输入变量映射：将工作流变量（如 trigger 输出）注入到 Agent 的 system_prompt 中。
    /// key = 注入到 prompt 的变量名，value = ExecutionState.variables 中的键。
    /// 运行时自动解析并追加 `【key】:value` 格式到 system_prompt 尾部。
    /// 示例: `{"stock_code": "trigger", "stock_name": "trigger"}` → 注入 "【stock_code】:600036\n【stock_name】:招商银行"
    #[serde(default)]
    pub input_mapping: std::collections::HashMap<String, String>,
    pub output_var: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    /// 工具列表，支持向后兼容旧格式 `["name1", "name2"]`
    #[serde(deserialize_with = "deserialize_tool_defs")]
    pub tools: Vec<ToolDef>,
    /// 暴露给 LLM 的工具名列表（tools 的子集）。为空时暴露全部（向后兼容）。
    /// 固定工具（上游 ToolNode 结果已通过 context_sources 注入）不应暴露。
    #[serde(default)]
    pub exposed_tools: Vec<String>,
    pub output_mode: OutputMode,
    /// AgentProfile ID — 唯一标识角色的方式，不再使用旧 role/agent_role_override
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_profile_id: Option<String>,
    /// Agent 多轮工具调用最大轮数，默认 1（不配置则仅单轮）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_tool_rounds: Option<u32>,
    /// 执行模式: "react" = 逐步思考-行动（默认）, "plan" = 先规划为工作流再执行
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub execution_mode: Option<String>,
    /// RAG 知识源 ID 列表。格式: "knowledge:<kb_id>", "memory:<ns_id>", "wiki:<wiki_id>"。
    /// 执行时从这些源检索与 query 相关的内容注入 system prompt。
    #[serde(default)]
    pub rag_source_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_role: Option<String>,
    /// 结果一致性检查配置（可选，不配置时零影响）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub consistency_check: Option<ConsistencyCheckConfig>,
    /// 防幻觉锚定检查配置（可选，不配置时零影响）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hallucination_guard: Option<HallucinationGuardConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentNode {
    #[serde(flatten)]
    pub base: WorkflowNodeBase,
    pub config: AgentNodeConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMNodeConfig {
    pub model: String,
    pub prompt: String,
    pub messages: Option<Vec<serde_json::Value>>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub tools: Option<Vec<String>>,
    pub functions: Option<Vec<serde_json::Value>>,
    /// 结果一致性检查配置（可选，不配置时零影响）
    #[serde(default)]
    pub consistency_check: Option<ConsistencyCheckConfig>,
    /// 最大上下文 token 数（可选，默认 128000）
    #[serde(default)]
    pub max_context_tokens: Option<u32>,
    /// 为输出保留的 token 数（可选，默认 4000）
    #[serde(default)]
    pub reserved_output_tokens: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMNode {
    #[serde(flatten)]
    pub base: WorkflowNodeBase,
    pub config: LLMNodeConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CompareOperator {
    #[serde(rename = "eq")]
    Eq,
    #[serde(rename = "ne")]
    Ne,
    #[serde(rename = "gt")]
    Gt,
    #[serde(rename = "lt")]
    Lt,
    #[serde(rename = "gte")]
    Gte,
    #[serde(rename = "lte")]
    Lte,
    #[serde(rename = "contains")]
    Contains,
    #[serde(rename = "notContains")]
    NotContains,
    #[serde(rename = "startsWith")]
    StartsWith,
    #[serde(rename = "endsWith")]
    EndsWith,
    #[serde(rename = "regexMatch")]
    RegexMatch,
    #[serde(rename = "isEmpty")]
    IsEmpty,
    #[serde(rename = "isNotEmpty")]
    IsNotEmpty,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogicalOperator {
    #[serde(rename = "and")]
    And,
    #[serde(rename = "or")]
    Or,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Condition {
    pub var_path: String,
    pub operator: CompareOperator,
    pub value: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConditionNodeConfig {
    pub conditions: Vec<Condition>,
    pub logical_op: LogicalOperator,
    /// 启用 LLM 动态路由：由 AI 判断走哪条分支（忽略 conditions 静态规则）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub judge_by_llm: Option<bool>,
    /// LLM 路由时的提示词（描述路由判断逻辑）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub routing_prompt: Option<String>,
    /// LLM 路由使用模型（为空则用系统默认）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub routing_model: Option<String>,
    /// 置信度阈值（0.0 - 1.0）。LLM 路由返回的置信度低于此值时，
    /// 降级为启发式判断（已有的 fallback 逻辑）。
    /// None = 不检查置信度（向后兼容）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confidence_threshold: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConditionNode {
    #[serde(flatten)]
    pub base: WorkflowNodeBase,
    pub config: ConditionNodeConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Branch {
    pub id: String,
    pub title: String,
    pub steps: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum MergeStrategy {
    #[default]
    All,
    Any,
    Race,
    Majority,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParallelNodeConfig {
    pub branches: Vec<Branch>,
    pub wait_for_all: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub aggregation: Option<MergeStrategy>,
    #[serde(default = "default_true")]
    pub auto_input_from_parent: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParallelNode {
    #[serde(flatten)]
    pub base: WorkflowNodeBase,
    pub config: ParallelNodeConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LoopType {
    #[serde(rename = "forEach")]
    ForEach,
    #[serde(rename = "while")]
    While,
    #[serde(rename = "doWhile")]
    DoWhile,
    #[serde(rename = "until")]
    Until,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoopNodeConfig {
    pub loop_type: LoopType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub items_var: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub iteratee_var: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_iterations: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub continue_condition: Option<String>,
    pub continue_on_error: bool,
    pub body_steps: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoopNode {
    #[serde(flatten)]
    pub base: WorkflowNodeBase,
    pub config: LoopNodeConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergeNodeConfig {
    #[serde(default)]
    pub merge_type: MergeStrategy,
    pub inputs: Vec<String>,
    #[serde(default)]
    pub auto_inputs_from_branches: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergeNode {
    #[serde(flatten)]
    pub base: WorkflowNodeBase,
    pub config: MergeNodeConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelayNodeConfig {
    pub delay_type: String,
    pub seconds: u64,
    pub until: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelayNode {
    #[serde(flatten)]
    pub base: WorkflowNodeBase,
    pub config: DelayNodeConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolNodeConfig {
    pub tool_name: String,
    pub input_mapping: std::collections::HashMap<String, String>,
    pub output_var: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolNode {
    #[serde(flatten)]
    pub base: WorkflowNodeBase,
    pub config: ToolNodeConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeNodeConfig {
    pub language: String,
    pub code: String,
    pub output_var: String,
    /// Rhai 脚本注册为工具名（language="rhai" 时生效，为空则用 code_<node_id>）
    #[serde(default)]
    pub tool_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeNode {
    #[serde(flatten)]
    pub base: WorkflowNodeBase,
    pub config: CodeNodeConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubWorkflowNodeConfig {
    pub sub_workflow_id: String,
    pub input_mapping: std::collections::HashMap<String, String>,
    pub output_var: String,
    pub is_async: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubWorkflowNode {
    #[serde(flatten)]
    pub base: WorkflowNodeBase,
    pub config: SubWorkflowNodeConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentParserNodeConfig {
    pub input_var: String,
    pub parser_type: String,
    pub output_var: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentParserNode {
    #[serde(flatten)]
    pub base: WorkflowNodeBase,
    pub config: DocumentParserNodeConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorRetrieveNodeConfig {
    pub query: String,
    pub knowledge_base_id: String,
    pub top_k: u32,
    pub similarity_threshold: Option<f32>,
    pub output_var: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorRetrieveNode {
    #[serde(flatten)]
    pub base: WorkflowNodeBase,
    pub config: VectorRetrieveNodeConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationNodeConfig {
    pub assertions: Vec<ValidationAssertion>,
    pub on_fail: String,
    pub max_retries: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationAssertion {
    #[serde(rename = "type")]
    pub assertion_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actual: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expression: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationNode {
    #[serde(flatten)]
    pub base: WorkflowNodeBase,
    pub config: ValidationNodeConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndNodeConfig {
    pub output_var: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndNode {
    #[serde(flatten)]
    pub base: WorkflowNodeBase,
    pub config: EndNodeConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwitchNodeConfig {
    pub input_var: String,
    pub cases: Vec<SwitchCase>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_case: Option<String>,
    #[serde(default = "default_switch_mode")]
    pub match_mode: String,
    #[serde(default)]
    pub output_var: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwitchNode {
    #[serde(flatten)]
    pub base: WorkflowNodeBase,
    pub config: SwitchNodeConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseQueryNodeConfig {
    pub query: String,
    #[serde(default)]
    pub params: Vec<String>,
    #[serde(default)]
    pub connection_name: Option<String>,
    #[serde(default = "default_query_timeout")]
    pub timeout_secs: u64,
    #[serde(default)]
    pub output_var: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseQueryNode {
    #[serde(flatten)]
    pub base: WorkflowNodeBase,
    pub config: DatabaseQueryNodeConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpRequestNodeConfig {
    pub url: String,
    #[serde(default = "default_http_method")]
    pub method: String,
    #[serde(default)]
    pub headers: std::collections::HashMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
    #[serde(default = "default_body_type")]
    pub body_type: String,
    #[serde(default = "default_http_timeout")]
    pub timeout_secs: u64,
    #[serde(default)]
    pub output_var: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwitchCase {
    pub value: String,
    pub label: String,
}

/// 工具列表反序列化，支持向后兼容旧格式 `["name1", "name2"]`
fn deserialize_tool_defs<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<ToolDef>, D::Error> {
    use serde::de;
    use std::marker::PhantomData;

    struct ToolDefOrString(PhantomData<Vec<ToolDef>>);

    impl<'de> de::Visitor<'de> for ToolDefOrString {
        type Value = Vec<ToolDef>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a list of tool definitions or a list of tool name strings")
        }

        fn visit_seq<A: de::SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
            let mut tools = Vec::new();
            while let Some(elem) = seq.next_element::<serde_json::Value>()? {
                match elem {
                    serde_json::Value::String(name) => {
                        tools.push(ToolDef {
                            name,
                            description: None,
                            parameters: None,
                        });
                    },
                    val => {
                        let tool: ToolDef =
                            serde_json::from_value(val).map_err(de::Error::custom)?;
                        tools.push(tool);
                    },
                }
            }
            Ok(tools)
        }
    }

    deserializer.deserialize_seq(ToolDefOrString(PhantomData))
}

fn default_switch_mode() -> String {
    "exact".to_string()
}
fn default_query_timeout() -> u64 {
    30
}
fn default_approval_timeout() -> u64 {
    86400
}
fn default_timeout_action() -> String {
    "auto_reject".to_string()
}
fn default_log_level() -> String {
    "info".to_string()
}
fn default_agg_strategy() -> String {
    "all".to_string()
}

fn default_http_method() -> String {
    "GET".to_string()
}
fn default_body_type() -> String {
    "json".to_string()
}
fn default_http_timeout() -> u64 {
    30
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpRequestNode {
    #[serde(flatten)]
    pub base: WorkflowNodeBase,
    pub config: HttpRequestNodeConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationNodeConfig {
    pub channel: String,
    pub message: String,
    #[serde(default)]
    pub webhook_url: Option<String>,
    #[serde(default)]
    pub recipients: Vec<String>,
    #[serde(default)]
    pub subject: Option<String>,
    pub enabled: bool,
    #[serde(default)]
    pub output_var: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationNode {
    #[serde(flatten)]
    pub base: WorkflowNodeBase,
    pub config: NotificationNodeConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalNodeConfig {
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub approver: Option<String>,
    #[serde(default = "default_approval_timeout")]
    pub timeout_secs: u64,
    #[serde(default = "default_timeout_action")]
    pub timeout_action: String,
    #[serde(default)]
    pub output_var: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalNode {
    #[serde(flatten)]
    pub base: WorkflowNodeBase,
    pub config: ApprovalNodeConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileOperationNodeConfig {
    pub operation: String,
    pub file_path: String,
    #[serde(default)]
    pub content: Option<String>,
    #[serde(default)]
    pub output_var: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileOperationNode {
    #[serde(flatten)]
    pub base: WorkflowNodeBase,
    pub config: FileOperationNodeConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataTransformerNodeConfig {
    pub input_var: String,
    pub expression: String,
    #[serde(default)]
    pub output_var: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataTransformerNode {
    #[serde(flatten)]
    pub base: WorkflowNodeBase,
    pub config: DataTransformerNodeConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookSendNodeConfig {
    pub url: String,
    #[serde(default = "default_http_method")]
    pub method: String,
    #[serde(default)]
    pub body: Option<String>,
    #[serde(default)]
    pub headers: std::collections::HashMap<String, String>,
    #[serde(default)]
    pub output_var: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookSendNode {
    #[serde(flatten)]
    pub base: WorkflowNodeBase,
    pub config: WebhookSendNodeConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingNodeConfig {
    #[serde(default = "default_log_level")]
    pub level: String,
    pub message: String,
    #[serde(default)]
    pub output_var: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingNode {
    #[serde(flatten)]
    pub base: WorkflowNodeBase,
    pub config: LoggingNodeConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmClassifierNodeConfig {
    pub categories: Vec<String>,
    pub prompt: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    pub input_var: String,
    #[serde(default)]
    pub output_var: String,
    /// 置信度阈值（0.0 - 1.0）。LLM 返回的置信度低于此值时，
    /// 使用 fallback_label（如果配置）或标记为 low_confidence 并返回错误。
    /// None = 不检查置信度（向后兼容）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confidence_threshold: Option<f64>,
    /// 置信度不足时的降级标签（可选）。不配置时直接标记失败。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fallback_label: Option<String>,
    /// 结果一致性检查配置（可选，不配置时零影响）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub consistency_check: Option<ConsistencyCheckConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmClassifierNode {
    #[serde(flatten)]
    pub base: WorkflowNodeBase,
    pub config: LlmClassifierNodeConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregatorNodeConfig {
    #[serde(default = "default_agg_strategy")]
    pub strategy: String,
    #[serde(default)]
    pub input_sources: Vec<String>,
    #[serde(default)]
    pub output_var: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregatorNode {
    #[serde(flatten)]
    pub base: WorkflowNodeBase,
    pub config: AggregatorNodeConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailNodeConfig {
    pub to: Vec<String>,
    pub subject: String,
    pub body: String,
    #[serde(default)]
    pub smtp_host: Option<String>,
    #[serde(default)]
    pub smtp_port: Option<u16>,
    #[serde(default)]
    pub smtp_user: Option<String>,
    #[serde(default)]
    pub smtp_pass: Option<String>,
    #[serde(default)]
    pub output_var: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailNode {
    #[serde(flatten)]
    pub base: WorkflowNodeBase,
    pub config: EmailNodeConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebateNodeConfig {
    #[serde(default)]
    pub debater_steps: Vec<String>,
    #[serde(default = "default_debate_rounds")]
    pub max_rounds: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub convergence_prompt: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub convergence_model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub convergence_model_role: Option<String>,
    #[serde(default)]
    pub topic_var: String,
    #[serde(default)]
    pub output_var: String,
}

fn default_debate_rounds() -> u32 {
    2
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebateNode {
    #[serde(flatten)]
    pub base: WorkflowNodeBase,
    pub config: DebateNodeConfig,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum WorkflowNode {
    Trigger(TriggerNode),
    Agent(AgentNode),
    Llm(LLMNode),
    Condition(ConditionNode),
    Parallel(ParallelNode),
    Loop(LoopNode),
    Merge(MergeNode),
    Delay(DelayNode),
    Validation(ValidationNode),
    SubWorkflow(SubWorkflowNode),
    DocumentParser(DocumentParserNode),
    VectorRetrieve(VectorRetrieveNode),
    End(EndNode),
    #[serde(rename = "httpRequest")]
    HttpRequest(HttpRequestNode),
    #[serde(rename = "switch")]
    Switch(SwitchNode),
    #[serde(rename = "databaseQuery")]
    DatabaseQuery(DatabaseQueryNode),
    #[serde(rename = "notification")]
    Notification(NotificationNode),
    #[serde(rename = "approval")]
    Approval(ApprovalNode),
    #[serde(rename = "fileOperation")]
    FileOperation(FileOperationNode),
    #[serde(rename = "dataTransformer")]
    DataTransformer(DataTransformerNode),
    #[serde(rename = "webhookSend")]
    WebhookSend(WebhookSendNode),
    #[serde(rename = "logging")]
    Logging(LoggingNode),
    #[serde(rename = "llmClassifier")]
    LlmClassifier(LlmClassifierNode),
    #[serde(rename = "aggregator")]
    Aggregator(AggregatorNode),
    #[serde(rename = "email")]
    Email(EmailNode),
    #[serde(rename = "debate")]
    Debate(DebateNode),
    #[serde(rename = "tool")]
    Tool(ToolNode),
    #[serde(rename = "code")]
    Code(CodeNode),
}

impl<'de> serde::Deserialize<'de> for WorkflowNode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = serde_json::Value::deserialize(deserializer)?;
        let type_str = value
            .get("type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| serde::de::Error::missing_field("type"))?;

        macro_rules! try_from_value {
            ($variant:ident, $inner:ty) => {
                WorkflowNode::$variant(
                    serde_json::from_value::<$inner>(value).map_err(serde::de::Error::custom)?,
                )
            };
        }

        match type_str {
            "trigger" => Ok(try_from_value!(Trigger, TriggerNode)),
            "agent" => Ok(try_from_value!(Agent, AgentNode)),
            "llm" => Ok(try_from_value!(Llm, LLMNode)),
            "condition" => Ok(try_from_value!(Condition, ConditionNode)),
            "parallel" => Ok(try_from_value!(Parallel, ParallelNode)),
            "loop" => Ok(try_from_value!(Loop, LoopNode)),
            "merge" => Ok(try_from_value!(Merge, MergeNode)),
            "delay" => Ok(try_from_value!(Delay, DelayNode)),
            "validation" => Ok(try_from_value!(Validation, ValidationNode)),
            "subWorkflow" => Ok(try_from_value!(SubWorkflow, SubWorkflowNode)),
            "documentParser" => Ok(try_from_value!(DocumentParser, DocumentParserNode)),
            "vectorRetrieve" => Ok(try_from_value!(VectorRetrieve, VectorRetrieveNode)),
            "httpRequest" => Ok(try_from_value!(HttpRequest, HttpRequestNode)),
            "switch" => Ok(try_from_value!(Switch, SwitchNode)),
            "databaseQuery" => Ok(try_from_value!(DatabaseQuery, DatabaseQueryNode)),
            "notification" => Ok(try_from_value!(Notification, NotificationNode)),
            "approval" => Ok(try_from_value!(Approval, ApprovalNode)),
            "fileOperation" => Ok(try_from_value!(FileOperation, FileOperationNode)),
            "dataTransformer" => Ok(try_from_value!(DataTransformer, DataTransformerNode)),
            "webhookSend" => Ok(try_from_value!(WebhookSend, WebhookSendNode)),
            "logging" => Ok(try_from_value!(Logging, LoggingNode)),
            "llmClassifier" => Ok(try_from_value!(LlmClassifier, LlmClassifierNode)),
            "aggregator" => Ok(try_from_value!(Aggregator, AggregatorNode)),
            "email" => Ok(try_from_value!(Email, EmailNode)),
            "debate" => Ok(try_from_value!(Debate, DebateNode)),

            "end" => Ok(try_from_value!(End, EndNode)),
            "tool" => Ok(try_from_value!(Tool, ToolNode)),
            "code" => Ok(try_from_value!(Code, CodeNode)),
            other => Err(serde::de::Error::unknown_variant(
                other,
                &[
                    "trigger",
                    "agent",
                    "llm",
                    "condition",
                    "parallel",
                    "loop",
                    "merge",
                    "delay",
                    "validation",
                    "subWorkflow",
                    "documentParser",
                    "vectorRetrieve",
                    "httpRequest",
                    "switch",
                    "databaseQuery",
                    "notification",
                    "approval",
                    "fileOperation",
                    "dataTransformer",
                    "webhookSend",
                    "logging",
                    "llmClassifier",
                    "aggregator",
                    "email",
                    "debate",
                    "end",
                    "tool",
                    "code",
                ],
            )),
        }
    }
}

impl WorkflowNode {
    pub fn base_id(&self) -> &str {
        match self {
            WorkflowNode::Trigger(n) => &n.base.id,
            WorkflowNode::Agent(n) => &n.base.id,
            WorkflowNode::Llm(n) => &n.base.id,
            WorkflowNode::Condition(n) => &n.base.id,
            WorkflowNode::Parallel(n) => &n.base.id,
            WorkflowNode::Loop(n) => &n.base.id,
            WorkflowNode::Merge(n) => &n.base.id,
            WorkflowNode::Delay(n) => &n.base.id,
            WorkflowNode::Tool(n) => &n.base.id,
            WorkflowNode::Code(n) => &n.base.id,
            WorkflowNode::SubWorkflow(n) => &n.base.id,
            WorkflowNode::DocumentParser(n) => &n.base.id,
            WorkflowNode::VectorRetrieve(n) => &n.base.id,
            WorkflowNode::Validation(n) => &n.base.id,
            WorkflowNode::HttpRequest(n) => &n.base.id,
            WorkflowNode::Switch(n) => &n.base.id,
            WorkflowNode::DatabaseQuery(n) => &n.base.id,
            WorkflowNode::Notification(n) => &n.base.id,
            WorkflowNode::Approval(n) => &n.base.id,
            WorkflowNode::FileOperation(n) => &n.base.id,
            WorkflowNode::DataTransformer(n) => &n.base.id,
            WorkflowNode::WebhookSend(n) => &n.base.id,
            WorkflowNode::Logging(n) => &n.base.id,
            WorkflowNode::LlmClassifier(n) => &n.base.id,
            WorkflowNode::Aggregator(n) => &n.base.id,
            WorkflowNode::Email(n) => &n.base.id,
            WorkflowNode::Debate(n) => &n.base.id,
            WorkflowNode::End(n) => &n.base.id,
        }
    }

    /// 从节点变体中提取基类引用
    pub fn base(&self) -> &WorkflowNodeBase {
        match self {
            WorkflowNode::Trigger(n) => &n.base,
            WorkflowNode::Agent(n) => &n.base,
            WorkflowNode::Llm(n) => &n.base,
            WorkflowNode::Condition(n) => &n.base,
            WorkflowNode::Parallel(n) => &n.base,
            WorkflowNode::Loop(n) => &n.base,
            WorkflowNode::Merge(n) => &n.base,
            WorkflowNode::Delay(n) => &n.base,
            WorkflowNode::Tool(n) => &n.base,
            WorkflowNode::Code(n) => &n.base,
            WorkflowNode::SubWorkflow(n) => &n.base,
            WorkflowNode::DocumentParser(n) => &n.base,
            WorkflowNode::VectorRetrieve(n) => &n.base,
            WorkflowNode::Validation(n) => &n.base,
            WorkflowNode::HttpRequest(n) => &n.base,
            WorkflowNode::Switch(n) => &n.base,
            WorkflowNode::DatabaseQuery(n) => &n.base,
            WorkflowNode::Notification(n) => &n.base,
            WorkflowNode::Approval(n) => &n.base,
            WorkflowNode::FileOperation(n) => &n.base,
            WorkflowNode::DataTransformer(n) => &n.base,
            WorkflowNode::WebhookSend(n) => &n.base,
            WorkflowNode::Logging(n) => &n.base,
            WorkflowNode::LlmClassifier(n) => &n.base,
            WorkflowNode::Aggregator(n) => &n.base,
            WorkflowNode::Email(n) => &n.base,
            WorkflowNode::Debate(n) => &n.base,

            WorkflowNode::End(n) => &n.base,
        }
    }

    pub fn base_timeout(&self) -> Option<u64> {
        self.base().timeout
    }

    pub fn base_retry(&self) -> &RetryConfig {
        &self.base().retry
    }

    pub fn base_enabled(&self) -> bool {
        self.base().enabled
    }

    pub fn base_title(&self) -> &str {
        &self.base().title
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerNode {
    #[serde(flatten)]
    pub base: WorkflowNodeBase,
    pub config: TriggerConfig,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EdgeType {
    #[serde(rename = "direct")]
    Direct,
    #[serde(rename = "conditionTrue")]
    ConditionTrue,
    #[serde(rename = "conditionFalse")]
    ConditionFalse,
    #[serde(rename = "loopBack")]
    LoopBack,
    #[serde(rename = "parallelBranch")]
    ParallelBranch,
    #[serde(rename = "merge")]
    Merge,
    #[serde(rename = "error")]
    Error,
    #[serde(rename = "debateRound")]
    DebateRound,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowEdge {
    pub id: String,
    pub source: String,
    pub source_handle: Option<String>,
    pub target: String,
    pub target_handle: Option<String>,
    pub edge_type: EdgeType,
    pub label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OnFailureAction {
    #[serde(rename = "abort")]
    Abort,
    #[serde(rename = "retryThenAbort")]
    RetryThenAbort,
    #[serde(rename = "runErrorBranch")]
    RunErrorBranch,
    #[serde(rename = "continueWithDefault")]
    ContinueWithDefault,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryPolicy {
    pub max_retries: u32,
    pub base_delay_ms: u64,
    pub max_delay_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompensationStep {
    pub step_id: String,
    pub compensate_type: String,
    pub target_step: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorConfig {
    pub retry_policy: Option<RetryPolicy>,
    pub on_failure: OnFailureAction,
    pub error_branch: Option<Vec<String>>,
    pub compensation_steps: Option<Vec<CompensationStep>>,
}

/// Rhai 脚本工具定义（不属于 DAG 节点，仅作为工具注册）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RhaiToolDef {
    /// 注册为工具名（Agent exposed_tools 引用此名）
    pub tool_name: String,
    /// 工具描述（发给 LLM）
    pub description: Option<String>,
    /// Rhai 脚本代码
    pub code: String,
}

pub struct WorkflowTemplateData {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub icon: String,
    pub tags: Vec<String>,
    pub version: i32,
    pub is_preset: bool,
    pub is_editable: bool,
    pub is_public: bool,
    pub trigger_config: Option<TriggerConfig>,
    pub nodes: Vec<WorkflowNode>,
    pub edges: Vec<WorkflowEdge>,
    pub input_schema: Option<JsonSchema>,
    pub output_schema: Option<JsonSchema>,
    pub variables: Vec<Variable>,
    pub error_config: Option<ErrorConfig>,
    /// Rhai 工具定义（非 DAG 节点，仅注册为可调用工具）
    pub tool_defs: Vec<RhaiToolDef>,
    pub created_at: i64,
    pub updated_at: i64,
}

impl WorkflowTemplateData {
    pub fn to_template_input(&self) -> WorkflowTemplateInput {
        WorkflowTemplateInput {
            name: self.name.clone(),
            description: self.description.clone(),
            icon: self.icon.clone(),
            tags: self.tags.clone(),
            trigger_config: self.trigger_config.clone(),
            nodes: self.nodes.clone(),
            edges: self.edges.clone(),
            input_schema: self.input_schema.clone(),
            output_schema: self.output_schema.clone(),
            variables: self.variables.clone(),
            error_config: self.error_config.clone(),
            tool_defs: Some(self.tool_defs.clone()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowTemplateInput {
    pub name: String,
    pub description: Option<String>,
    pub icon: String,
    pub tags: Vec<String>,
    pub trigger_config: Option<TriggerConfig>,
    pub nodes: Vec<WorkflowNode>,
    pub edges: Vec<WorkflowEdge>,
    pub input_schema: Option<JsonSchema>,
    pub output_schema: Option<JsonSchema>,
    pub variables: Vec<Variable>,
    pub error_config: Option<ErrorConfig>,
    pub tool_defs: Option<Vec<RhaiToolDef>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowTemplateResponse {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub icon: String,
    pub tags: Vec<String>,
    pub version: i32,
    pub is_preset: bool,
    pub is_editable: bool,
    pub is_public: bool,
    pub trigger_config: Option<TriggerConfig>,
    pub nodes: Vec<WorkflowNode>,
    pub edges: Vec<WorkflowEdge>,
    pub input_schema: Option<JsonSchema>,
    pub output_schema: Option<JsonSchema>,
    pub variables: Vec<Variable>,
    pub error_config: Option<ErrorConfig>,
    pub tool_defs: Option<Vec<RhaiToolDef>>,
    pub created_at: i64,
    pub updated_at: i64,
}

impl From<WorkflowTemplateData> for WorkflowTemplateResponse {
    fn from(data: WorkflowTemplateData) -> Self {
        Self {
            id: data.id,
            name: data.name,
            description: data.description,
            icon: data.icon,
            tags: data.tags,
            version: data.version,
            is_preset: data.is_preset,
            is_editable: data.is_editable,
            is_public: data.is_public,
            trigger_config: data.trigger_config,
            nodes: data.nodes,
            edges: data.edges,
            input_schema: data.input_schema,
            output_schema: data.output_schema,
            variables: data.variables,
            error_config: data.error_config,
            tool_defs: Some(data.tool_defs),
            created_at: data.created_at,
            updated_at: data.updated_at,
        }
    }
}

// ── 模板筛选、校验结果 ──────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateFilter {
    pub is_preset: Option<bool>,
    pub tags: Option<Vec<String>>,
    pub search: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    pub error_type: String,
    pub node_id: Option<String>,
    pub message: String,
    pub suggestion: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationWarning {
    pub warning_type: String,
    pub node_id: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub errors: Vec<ValidationError>,
    pub warnings: Vec<ValidationWarning>,
}

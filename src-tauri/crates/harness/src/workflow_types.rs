// SPDX-License-Identifier: AGPL-3.0-only

//! Workflow type definitions
//!
//! This module defines the core types used in workflow execution,
//! including nodes, variables, triggers, and execution states.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::consistency_check::ConsistencyCheckConfig;
use crate::hallucination_guard::HallucinationGuardConfig;

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct Position {
    pub x: f64,
    pub y: f64,
}

impl Default for Position {
    fn default() -> Self {
        Self { x: 0.0, y: 0.0 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
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

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, TS)]
pub enum BackoffType {
    Linear,
    Exponential,
    Fixed,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
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
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct ToolDef {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<JsonSchema>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, TS)]
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

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, TS)]
pub struct Variable {
    pub name: String,
    pub var_type: String,
    pub value: serde_json::Value,
    pub description: Option<String>,
    pub is_secret: bool,
}

/// 补偿策略：当节点失败时，如何处理其下游节点和输出
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, TS)]
pub enum CompensationStrategy {
    /// 仅删除该节点输出，不处理下游
    SkipWithWarning,
    /// 删除该节点输出，并标记所有下游 Pending/Ready 节点为 Skipped
    Rollback,
    /// 记录警告，需要人工介入处理
    Escalate,
}

/// 补偿配置：定义节点失败时的恢复策略
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, TS)]
pub struct CompensationConfig {
    pub strategy: CompensationStrategy,
    /// 需要执行补偿的节点 ID 列表（预留扩展，当前由引擎根据 DAG 自动推导下游）
    #[serde(default)]
    pub compensation_nodes: Vec<String>,
}

/// 节点语义分类（确定节点的固定颜色）。
///
/// 由引擎层定义，编辑器前端根据此分类从主题 token 映射颜色。
/// 禁止在工作流设计层面随意指定节点颜色。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema, TS)]
pub enum NodeKind {
    /// 输入/触发类（黄）
    Input,
    /// 输出/结束类（红）
    Output,
    /// 工具/执行类（绿）
    Tool,
    /// Agent/LLM 推理类（蓝）
    Agent,
    /// 条件分支/路由（橙）
    Condition,
    /// 循环控制（紫）
    Loop,
    /// 容器/并行/辩论（青）
    Container,
    /// 存储/检索（粉）
    Storage,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
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
    /// 节点失败时不中断整个工作流，继续执行后续节点。
    #[serde(default)]
    pub continue_on_fail: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, TS)]
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

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct TriggerConfig {
    #[serde(rename = "type")]
    pub trigger_type: TriggerType,
    pub config: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct ManualTriggerConfig {}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleTriggerConfig {
    /// 单个 cron 表达式（5 或 6 字段），标准调度器使用
    #[serde(default)]
    pub cron: String,
    /// 多时段 cron 表达式（named → cron），stock-analysis 等模板使用
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schedules: Option<std::collections::HashMap<String, String>>,
    #[serde(default)]
    pub timezone: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// 触发时注入工作流的输入参数（JSON）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_params: Option<serde_json::Value>,
}

impl ScheduleTriggerConfig {
    /// 检查配置是否有效（cron 表达式非空）
    pub fn is_valid(&self) -> bool {
        !self.cron.trim().is_empty()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct WebhookTriggerConfig {
    pub path: String,
    pub method: String,
    pub auth_type: String,
    /// 响应模式: "sync" 等待工作流完成后再返回, "async" 立即返回 202
    #[serde(default = "default_webhook_response_mode", skip_serializing_if = "Option::is_none")]
    pub response_mode: Option<String>,
}

fn default_webhook_response_mode() -> Option<String> {
    Some("async".to_string())
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct EventTriggerConfig {
    pub event_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filter: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, TS)]
pub enum OutputMode {
    #[serde(rename = "json")]
    Json,
    #[serde(rename = "text")]
    Text,
    #[serde(rename = "artifact")]
    Artifact,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
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
    /// H4.1 修复：主模型返回空输出时切换到的兜底模型 ID。
    /// 配置后，agent_executor 在 strict_mode 检测到空输出时会用此模型重试。
    /// 典型配置："glm-5.2"（当主模型为 qwen3.7-max 时）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fallback_model: Option<String>,
    /// 3.7 P2:任务场景 — 控制 Agent 节点的输出风格指令。
    ///
    /// - `Code`:强调直接给代码、少废话
    /// - `Research`:强调结构化分析、引用、权衡
    /// - `General`:无特殊约束(默认)
    /// - `Auto`:由 `TaskScene::infer(input)` 自动推断
    ///
    /// 缺省 `None` 时按 `General` 处理;`Some(TaskScene::Auto)` 时
    /// executor 会在拼接 prompt 前对 input 文本调用 `infer` 推断。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task_scene: Option<crate::TaskScene>,
    /// #1 修复(2026-07-22): 单节点 LLM stream chunk 超时(秒)。
    ///
    /// 默认 `None` 时 agent_executor 用 120s。
    /// 大上下文节点(如 debate-convergence: 16 节点 context_sources +
    /// 30 个 input_mapping 结构化字段, ~30k-40k input tokens)的 TTFB
    /// 偶发 > 120s, 触发 "stream chunk timeout" 失败。
    /// 配置示例: debate-convergence 设 300s, 辩手节点保持默认 120s。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stream_chunk_timeout_secs: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, TS)]
pub struct AgentNode {
    #[serde(flatten)]
    pub base: WorkflowNodeBase,
    pub config: AgentNodeConfig,
}

fn default_multi_agent_rounds() -> u32 {
    3
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, TS)]
pub struct MultiAgentNodeConfig {
    /// 委派任务描述
    #[serde(default)]
    pub task: String,
    /// 角色/agent 名称（可选）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    /// 模型覆盖（可选）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// 输出变量名
    #[serde(default)]
    pub output_var: String,
    /// 协作模式: auto / swarm / debate
    #[serde(default)]
    pub mode: String,
    /// 最大协作轮数
    #[serde(default = "default_multi_agent_rounds")]
    pub max_rounds: u32,
    /// 输入映射：从上游节点输出/变量映射到当前任务的输入参数
    ///
    /// 格式: { "target_field": "$node.source_node_id.source_field" } 或 { "target_field": "$variables.var_name" }
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_mapping: Option<std::collections::HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, TS)]
pub struct MultiAgentNode {
    #[serde(flatten)]
    pub base: WorkflowNodeBase,
    pub config: MultiAgentNodeConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, TS)]
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

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, TS)]
pub struct LLMNode {
    #[serde(flatten)]
    pub base: WorkflowNodeBase,
    pub config: LLMNodeConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, TS)]
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

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, TS)]
pub enum LogicalOperator {
    #[serde(rename = "and")]
    And,
    #[serde(rename = "or")]
    Or,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, TS)]
pub struct Condition {
    pub var_path: String,
    pub operator: CompareOperator,
    pub value: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, TS)]
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

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, TS)]
pub struct ConditionNode {
    #[serde(flatten)]
    pub base: WorkflowNodeBase,
    pub config: ConditionNodeConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, schemars::JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub enum DegradeStrategy {
    /// 超时后跳过该路径，继续其他分支
    #[default]
    Skip,
    /// 超时后使用默认值
    #[serde(rename = "useDefault")]
    UseDefault,
    /// 超时即终止整个并行执行
    Strict,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, TS)]
pub struct Branch {
    pub id: String,
    pub title: String,
    pub steps: Vec<String>,
    /// 分支级别超时（毫秒）。留空则继承节点级别或全局超时。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub branch_timeout_ms: Option<u64>,
    /// 超时后的降级策略。默认 Skip。
    #[serde(default)]
    pub degrade_strategy: DegradeStrategy,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, schemars::JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub enum MergeStrategy {
    #[default]
    All,
    Any,
    Race,
    Majority,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, TS)]
pub struct ParallelNodeConfig {
    pub branches: Vec<Branch>,
    pub wait_for_all: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub aggregation: Option<MergeStrategy>,
    #[serde(default = "default_true")]
    pub auto_input_from_parent: bool,
    /// 子图定义（可选）。编辑器根据 `isContainer: true` 渲染为可展开/折叠容器框体。
    /// sub_graph 中的节点/边将替代 branches.steps 的扁平引用模式，
    /// 提供更丰富的嵌套子工作流编辑体验。
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "subGraph")]
    pub sub_graph: Option<SubGraph>,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, TS)]
pub struct ParallelNode {
    #[serde(flatten)]
    pub base: WorkflowNodeBase,
    pub config: ParallelNodeConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, TS)]
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

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, TS)]
pub struct LoopNodeConfig {
    pub loop_type: LoopType,
    /// 数组输入端口（旧名 `items_var`，向后兼容）。
    /// 优先读取 `iter_input_var`，未设置时回退到 `items_var`。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub items_var: Option<String>,
    /// 数组输入端口（新名，语义更清晰）。
    /// 解析顺序：`iter_input_var` → `items_var` → `iteratee_var` 推断 → 空。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub iter_input_var: Option<String>,
    /// 循环体内部把当前元素写入的变量名。
    /// body_steps 中的下游节点可通过 `variables[<iteratee_var>]` 读取当前 item。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub iteratee_var: Option<String>,
    /// 循环聚合结果的输出端口变量名。
    /// 默认 `iter_output`，留空则使用 `iter_output`。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub iter_output_var: Option<String>,
    /// 流式中间结果变量名（在每次迭代完成后写入 context.variables）。
    /// 前端/下游节点可以订阅这个变量来观察进度。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub partial_result_var: Option<String>,
    /// 硬上限：实际迭代次数 = min(items.len(), max_iterations, 10_000)。
    /// 留空则按 items.len() 决定，forEach 模式默认 10_000。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_iterations: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub continue_condition: Option<String>,
    #[serde(default)]
    pub continue_on_error: bool,
    #[serde(default)]
    pub body_steps: Vec<String>,
    /// 每次迭代结束后挂起，等待人工确认后再继续。
    /// 与 `interrupt_nodes` 联合使用：留空且 `interrupt_after_each=true` 时
    /// 每一轮迭代后都进入 Paused。
    #[serde(default)]
    pub interrupt_after_each: bool,
    /// 触发 interrupt 的 body 节点 ID 集合（通常是 ApprovalNode）。
    /// 当这些节点中任意一个输出 `status=pending` 时，Loop 立即进入 Paused
    /// 并写检查点。
    #[serde(default)]
    pub interrupt_nodes: Vec<String>,
    /// 子图定义（可选）。编辑器根据 `isContainer: true` 渲染为可展开/折叠容器框体。
    /// sub_graph 中的节点/边将替代 body_steps 的扁平引用模式。
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "subGraph")]
    pub sub_graph: Option<SubGraph>,
}

impl LoopNodeConfig {
    // Business methods extracted to LoopNodeConfigResolver below.
}

/// Resolver for LoopNodeConfig variable ports (three-level fallback chain).
pub struct LoopNodeConfigResolver;

impl LoopNodeConfigResolver {
    /// 返回数组输入端口名（`iter_input_var` → `items_var` → 推测自 `iteratee_var`）。
    pub fn effective_input_var(config: &LoopNodeConfig) -> Option<&str> {
        if let Some(ref v) = config.iter_input_var
            && !v.is_empty()
        {
            return Some(v.as_str());
        }
        if let Some(ref v) = config.items_var
            && !v.is_empty()
        {
            return Some(v.as_str());
        }
        config.iteratee_var.as_deref()
    }

    /// 返回聚合输出端口名，默认 `iter_output`。
    pub fn effective_output_var(config: &LoopNodeConfig) -> &str {
        match config.iter_output_var.as_deref() {
            Some(v) if !v.is_empty() => v,
            _ => "iter_output",
        }
    }

    /// 返回 partial_result 变量名。空时表示不写入流式变量。
    pub fn effective_partial_var(config: &LoopNodeConfig) -> Option<&str> {
        config.partial_result_var.as_deref().filter(|s| !s.is_empty())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, TS)]
pub struct LoopNode {
    #[serde(flatten)]
    pub base: WorkflowNodeBase,
    pub config: LoopNodeConfig,
}

/// Loop 节点的可恢复检查点，持久化到 `loop_checkpoints` 表。
/// 字段保持可序列化（仅 `serde_json::Value` + 基础类型），便于跨进程恢复。
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, TS)]
pub struct LoopCheckpoint {
    pub execution_id: String,
    pub node_id: String,
    /// 下一个要执行的迭代下标（0-based）。恢复时从该下标继续。
    pub cursor: u32,
    /// 原始输入数组（解析后的 `Vec<Value>`）。恢复时无需重新解析。
    pub input_items: Vec<serde_json::Value>,
    /// 已完成的迭代聚合结果（Vec，按完成顺序追加）。
    pub partial_results: Vec<serde_json::Value>,
    /// 触发 interrupt 的 body 节点 ID（如有）。`None` 表示非 interrupt 中断。
    pub pending_approval_node: Option<String>,
    /// 触发 interrupt 时所在 body 节点对应的 step 输出。
    pub pending_step_output: Option<serde_json::Value>,
    /// 检查点写入时间戳（毫秒）。
    pub saved_at_ms: u64,
    /// 触发 interrupt 的循环体节点 ID（供前端高亮）。
    pub interrupting_step_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, TS)]
pub struct MergeNodeConfig {
    #[serde(default)]
    pub merge_type: MergeStrategy,
    pub inputs: Vec<String>,
    #[serde(default)]
    pub auto_inputs_from_branches: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, TS)]
pub struct MergeNode {
    #[serde(flatten)]
    pub base: WorkflowNodeBase,
    pub config: MergeNodeConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, TS)]
pub struct DelayNodeConfig {
    pub delay_type: String,
    pub seconds: u64,
    pub until: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, TS)]
pub struct DelayNode {
    #[serde(flatten)]
    pub base: WorkflowNodeBase,
    pub config: DelayNodeConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, TS)]
pub struct ToolNodeConfig {
    pub tool_name: String,
    pub input_mapping: std::collections::HashMap<String, String>,
    pub output_var: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, TS)]
pub struct ToolNode {
    #[serde(flatten)]
    pub base: WorkflowNodeBase,
    pub config: ToolNodeConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, TS)]
pub struct CodeNodeConfig {
    pub language: String,
    pub code: String,
    pub output_var: String,
    /// Rhai 脚本注册为工具名（language="rhai" 时生效，为空则用 code_<node_id>）
    #[serde(default)]
    pub tool_name: Option<String>,
    /// 是否直接执行（不经过工具注册流程），供 Rhai 脚本在 DAG 中直接运行
    #[serde(default)]
    pub execute_directly: bool,
    /// 输入映射：上游变量到 Rhai scope 变量的映射
    #[serde(default)]
    pub input_mapping: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, TS)]
pub struct CodeNode {
    #[serde(flatten)]
    pub base: WorkflowNodeBase,
    pub config: CodeNodeConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, TS)]
pub struct SubWorkflowNodeConfig {
    pub sub_workflow_id: String,
    pub input_mapping: std::collections::HashMap<String, String>,
    pub output_var: String,
    pub is_async: bool,
    /// 子图定义（可选）。与 expandedSubWorkflows 配合，编辑器可在容器内部渲染子工作流节点。
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "subGraph")]
    pub sub_graph: Option<SubGraph>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, TS)]
pub struct SubWorkflowNode {
    #[serde(flatten)]
    pub base: WorkflowNodeBase,
    pub config: SubWorkflowNodeConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, TS)]
pub struct WorkflowRefNodeConfig {
    pub target_workflow_id: String,
    #[serde(default)]
    pub input_mapping: std::collections::HashMap<String, String>,
    #[serde(default)]
    pub output_var: String,
    pub timeout: Option<i64>,
    #[serde(default = "default_context_mode")]
    pub context_mode: String,
}

fn default_context_mode() -> String {
    "inherit".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, TS)]
pub struct WorkflowRefNode {
    #[serde(flatten)]
    pub base: WorkflowNodeBase,
    pub config: WorkflowRefNodeConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, TS)]
pub struct DocumentParserNodeConfig {
    pub input_var: String,
    pub parser_type: String,
    pub output_var: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, TS)]
pub struct DocumentParserNode {
    #[serde(flatten)]
    pub base: WorkflowNodeBase,
    pub config: DocumentParserNodeConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, TS)]
pub struct VectorRetrieveNodeConfig {
    pub query: String,
    pub knowledge_base_id: String,
    pub top_k: u32,
    pub similarity_threshold: Option<f32>,
    pub output_var: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, TS)]
pub struct VectorRetrieveNode {
    #[serde(flatten)]
    pub base: WorkflowNodeBase,
    pub config: VectorRetrieveNodeConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, TS)]
pub struct ValidationNodeConfig {
    pub assertions: Vec<ValidationAssertion>,
    pub on_fail: String,
    pub max_retries: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, TS)]
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

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, TS)]
pub struct ValidationNode {
    #[serde(flatten)]
    pub base: WorkflowNodeBase,
    pub config: ValidationNodeConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, TS)]
pub struct EndNodeConfig {
    pub output_var: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, TS)]
pub struct EndNode {
    #[serde(flatten)]
    pub base: WorkflowNodeBase,
    pub config: EndNodeConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, schemars::JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub enum SwitchMatchMode {
    /// 精确字符串匹配（默认）
    #[default]
    Exact,
    /// 正则表达式匹配
    Regex,
    /// 子串包含匹配
    Contains,
    /// Rhai 表达式匹配：`input_var` 值作为 `_value` 传入表达式，返回布尔值或字符串标签
    Expression,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, TS)]
pub struct SwitchNodeConfig {
    pub input_var: String,
    pub cases: Vec<SwitchCase>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_case: Option<String>,
    #[serde(default = "default_switch_mode")]
    pub match_mode: String,
    /// 使用 LLM 进行智能路由（替代 match_mode 的值匹配）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub use_llm: Option<bool>,
    /// LLM 路由的自定义提示词
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub llm_prompt: Option<String>,
    /// 路由使用的模型（为空则用会话默认模型）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub llm_model: Option<String>,
    #[serde(default)]
    pub output_var: String,
}

/// SwitchCase 的 expression 模式下，`value` 字段存放 Rhai 表达式，
/// 接收 `_value` 变量（输入值），返回布尔值决定是否匹配该 case。
///
/// 例如：`_value > 100`、`_value.contains("error")`

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, TS)]
pub struct SwitchNode {
    #[serde(flatten)]
    pub base: WorkflowNodeBase,
    pub config: SwitchNodeConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, TS)]
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
    /// Credential ID for database connection (DatabaseConnection type)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub credential_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, TS)]
pub struct DatabaseQueryNode {
    #[serde(flatten)]
    pub base: WorkflowNodeBase,
    pub config: DatabaseQueryNodeConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, TS)]
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
    /// Credential ID for authenticated requests (supports ApiKey, BasicAuth, BearerToken)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub credential_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, TS)]
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
                        tools.push(ToolDef { name, description: None, parameters: None });
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

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, TS)]
pub struct HttpRequestNode {
    #[serde(flatten)]
    pub base: WorkflowNodeBase,
    pub config: HttpRequestNodeConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, TS)]
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

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, TS)]
pub struct NotificationNode {
    #[serde(flatten)]
    pub base: WorkflowNodeBase,
    pub config: NotificationNodeConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, TS)]
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

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, TS)]
pub struct ApprovalNode {
    #[serde(flatten)]
    pub base: WorkflowNodeBase,
    pub config: ApprovalNodeConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, TS)]
pub struct FileOperationNodeConfig {
    pub operation: String,
    pub file_path: String,
    #[serde(default)]
    pub content: Option<String>,
    #[serde(default)]
    pub output_var: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, TS)]
pub struct FileOperationNode {
    #[serde(flatten)]
    pub base: WorkflowNodeBase,
    pub config: FileOperationNodeConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, TS)]
pub struct DataTransformerNodeConfig {
    pub input_var: String,
    pub expression: String,
    #[serde(default)]
    pub output_var: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, TS)]
pub struct DataTransformerNode {
    #[serde(flatten)]
    pub base: WorkflowNodeBase,
    pub config: DataTransformerNodeConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, TS)]
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
    /// Credential ID for authenticated webhook (supports ApiKey, BasicAuth, BearerToken)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub credential_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, TS)]
pub struct WebhookSendNode {
    #[serde(flatten)]
    pub base: WorkflowNodeBase,
    pub config: WebhookSendNodeConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, TS)]
pub struct LoggingNodeConfig {
    #[serde(default = "default_log_level")]
    pub level: String,
    pub message: String,
    #[serde(default)]
    pub output_var: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, TS)]
pub struct LoggingNode {
    #[serde(flatten)]
    pub base: WorkflowNodeBase,
    pub config: LoggingNodeConfig,
}

/// 存储持久化节点配置：接收数据输入，存储到指定后端。
///
/// # 后端支持
/// - `sqlite`：写入 SQLite 表（JSON 值存储）
/// - `vectorDb`：写入向量数据库
/// - `fileSystem`：写入文件系统
///
/// # 操作模式
/// - `insert`：追加写入
/// - `upsert`：根据 key 覆盖/更新
/// - `append`：追加到已有内容（fileSystem 为追加到文件末尾）
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, TS)]
pub struct StorageNodeConfig {
    /// 存储后端："sqlite" | "vectorDb" | "fileSystem"
    #[serde(default = "default_storage_backend")]
    pub backend: String,
    /// 操作模式："insert" | "upsert" | "append"
    #[serde(default = "default_storage_operation")]
    pub operation: String,
    /// 要存储的数据的变量路径
    pub input_var: String,
    /// 存储目标（SQLite 表名 / VectorDB collection / 文件路径）
    pub collection: String,
    /// upsert 时用于匹配已有记录的 key 变量路径
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key_var: Option<String>,
    #[serde(default)]
    pub output_var: String,
}

fn default_storage_backend() -> String {
    "sqlite".to_string()
}

fn default_storage_operation() -> String {
    "insert".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, TS)]
pub struct StorageNode {
    #[serde(flatten)]
    pub base: WorkflowNodeBase,
    pub config: StorageNodeConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, TS)]
pub struct LlmClassifierNodeConfig {
    /// 静态分类目录（兜底）。动态目录不可用时的默认类别列表。
    pub categories: Vec<String>,
    /// 动态分类目录注入口：从工作流 variables 读取类别列表的变量名
    /// （h3 认知编排器：L1/L2 分类目录由能力基座运行时动态构建注入，
    /// 优先使用动态目录，读取失败或为空时回退到静态 categories）。
    /// 变量值支持两种形态：
    /// - 字符串数组 `["a", "b", ...]`：元素直接作为类别名
    /// - 对象数组 `[{"name": "a", ...}, ...]`：取 name/id/label/title 字段作为类别名
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub categories_var: Option<String>,
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

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, TS)]
pub struct LlmClassifierNode {
    #[serde(flatten)]
    pub base: WorkflowNodeBase,
    pub config: LlmClassifierNodeConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, TS)]
pub struct AggregatorNodeConfig {
    #[serde(default = "default_agg_strategy")]
    pub strategy: String,
    #[serde(default)]
    pub input_sources: Vec<String>,
    /// 等待策略：true=等待所有输入就绪再聚合；false=有输入即聚合（竞速模式）
    #[serde(default = "default_wait_all")]
    pub wait_for_all: bool,
    /// 加权策略的权重系数（与 input_sources 一一对应）。空数组视为等权。
    #[serde(default)]
    pub weights: Vec<f64>,
    /// llm_summarize 策略的自定义提示词
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summarize_prompt: Option<String>,
    /// llm_summarize 策略的模型
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summarize_model: Option<String>,
    #[serde(default)]
    pub output_var: String,
    /// 子图定义（可选）。编辑器根据 `isContainer: true` 渲染为可展开/折叠容器框体。
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "subGraph")]
    pub sub_graph: Option<SubGraph>,
}

fn default_wait_all() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, TS)]
pub struct AggregatorNode {
    #[serde(flatten)]
    pub base: WorkflowNodeBase,
    pub config: AggregatorNodeConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, TS)]
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
    /// Credential ID for SMTP configuration (Smtp type). Falls back to inline smtp_* fields.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub credential_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, TS)]
pub struct EmailNode {
    #[serde(flatten)]
    pub base: WorkflowNodeBase,
    pub config: EmailNodeConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, TS)]
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
    /// 子图定义（可选）。编辑器根据 `isContainer: true` 渲染为可展开/折叠容器框体。
    /// sub_graph 中的节点/边将替代 debater_steps 的扁平引用模式。
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "subGraph")]
    pub sub_graph: Option<SubGraph>,
}

fn default_debate_rounds() -> u32 {
    2
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, TS)]
pub struct DebateNode {
    #[serde(flatten)]
    pub base: WorkflowNodeBase,
    pub config: DebateNodeConfig,
}

fn default_swarm_rounds() -> u32 {
    3
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, TS)]
pub struct SwarmNodeConfig {
    /// 参与者节点 ID 列表（LLM/Agent 节点）
    #[serde(default)]
    pub agent_steps: Vec<String>,
    /// 最大协作轮数
    #[serde(default = "default_swarm_rounds")]
    pub max_rounds: u32,
    /// 收敛判断提示文本
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub convergence_prompt: Option<String>,
    /// 收敛判断模型
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub convergence_model: Option<String>,
    /// 讨论主题变量
    #[serde(default)]
    pub topic_var: String,
    /// 输出变量名
    #[serde(default)]
    pub output_var: String,
    /// 子图定义（可选）。编辑器根据 `isContainer: true` 渲染为可展开/折叠容器框体。
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "subGraph")]
    pub sub_graph: Option<SubGraph>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, TS)]
pub struct SwarmNode {
    #[serde(flatten)]
    pub base: WorkflowNodeBase,
    pub config: SwarmNodeConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, TS)]
#[serde(tag = "type", rename_all = "camelCase")]
#[schemars(tag = "type", rename_all = "camelCase")]
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
    #[serde(rename = "workflowRef")]
    WorkflowRef(WorkflowRefNode),
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
    #[serde(rename = "swarm")]
    Swarm(SwarmNode),
    #[serde(rename = "multiAgent")]
    MultiAgent(MultiAgentNode),
    #[serde(rename = "storage")]
    Storage(StorageNode),
    #[serde(rename = "tool")]
    Tool(ToolNode),
    #[serde(rename = "code")]
    Code(CodeNode),
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
            WorkflowNode::Swarm(n) => &n.base.id,
            WorkflowNode::MultiAgent(n) => &n.base.id,
            WorkflowNode::Storage(n) => &n.base.id,
            WorkflowNode::WorkflowRef(n) => &n.base.id,
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
            WorkflowNode::Swarm(n) => &n.base,
            WorkflowNode::MultiAgent(n) => &n.base,
            WorkflowNode::Storage(n) => &n.base,
            WorkflowNode::WorkflowRef(n) => &n.base,
            WorkflowNode::End(n) => &n.base,
        }
    }

    /// 从节点变体中提取基类可变引用(供进化器修改 retry / timeout / continue_on_fail)。
    pub fn base_mut(&mut self) -> &mut WorkflowNodeBase {
        match self {
            WorkflowNode::Trigger(n) => &mut n.base,
            WorkflowNode::Agent(n) => &mut n.base,
            WorkflowNode::Llm(n) => &mut n.base,
            WorkflowNode::Condition(n) => &mut n.base,
            WorkflowNode::Parallel(n) => &mut n.base,
            WorkflowNode::Loop(n) => &mut n.base,
            WorkflowNode::Merge(n) => &mut n.base,
            WorkflowNode::Delay(n) => &mut n.base,
            WorkflowNode::Tool(n) => &mut n.base,
            WorkflowNode::Code(n) => &mut n.base,
            WorkflowNode::SubWorkflow(n) => &mut n.base,
            WorkflowNode::DocumentParser(n) => &mut n.base,
            WorkflowNode::VectorRetrieve(n) => &mut n.base,
            WorkflowNode::Validation(n) => &mut n.base,
            WorkflowNode::HttpRequest(n) => &mut n.base,
            WorkflowNode::Switch(n) => &mut n.base,
            WorkflowNode::DatabaseQuery(n) => &mut n.base,
            WorkflowNode::Notification(n) => &mut n.base,
            WorkflowNode::Approval(n) => &mut n.base,
            WorkflowNode::FileOperation(n) => &mut n.base,
            WorkflowNode::DataTransformer(n) => &mut n.base,
            WorkflowNode::WebhookSend(n) => &mut n.base,
            WorkflowNode::Logging(n) => &mut n.base,
            WorkflowNode::LlmClassifier(n) => &mut n.base,
            WorkflowNode::Aggregator(n) => &mut n.base,
            WorkflowNode::Email(n) => &mut n.base,
            WorkflowNode::Debate(n) => &mut n.base,
            WorkflowNode::Swarm(n) => &mut n.base,
            WorkflowNode::MultiAgent(n) => &mut n.base,
            WorkflowNode::Storage(n) => &mut n.base,
            WorkflowNode::WorkflowRef(n) => &mut n.base,
            WorkflowNode::End(n) => &mut n.base,
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

    pub fn base_continue_on_fail(&self) -> bool {
        self.base().continue_on_fail
    }

    pub fn base_title(&self) -> &str {
        &self.base().title
    }

    /// 获取节点 config 中的 output_var。
    /// 用于降级（UseDefault）时把 null 写入 output_var key，
    /// 避免下游 context_sources 引用 output_var 时报"变量未找到"。
    /// 无 output_var 字段的节点类型返回 None。
    pub fn base_output_var(&self) -> Option<&str> {
        match self {
            // TriggerConfig 无 output_var 字段
            WorkflowNode::Trigger(_) => None,
            WorkflowNode::Agent(n) => Some(&n.config.output_var),
            WorkflowNode::Tool(n) => Some(&n.config.output_var),
            WorkflowNode::Code(n) => Some(&n.config.output_var),
            WorkflowNode::SubWorkflow(n) => Some(&n.config.output_var),
            WorkflowNode::DocumentParser(n) => Some(&n.config.output_var),
            WorkflowNode::VectorRetrieve(n) => Some(&n.config.output_var),
            WorkflowNode::HttpRequest(n) => Some(&n.config.output_var),
            WorkflowNode::DatabaseQuery(n) => Some(&n.config.output_var),
            WorkflowNode::Notification(n) => Some(&n.config.output_var),
            WorkflowNode::FileOperation(n) => Some(&n.config.output_var),
            WorkflowNode::DataTransformer(n) => Some(&n.config.output_var),
            WorkflowNode::WebhookSend(n) => Some(&n.config.output_var),
            WorkflowNode::Logging(n) => Some(&n.config.output_var),
            WorkflowNode::LlmClassifier(n) => Some(&n.config.output_var),
            WorkflowNode::Aggregator(n) => Some(&n.config.output_var),
            WorkflowNode::Email(n) => Some(&n.config.output_var),
            WorkflowNode::Debate(n) => Some(&n.config.output_var),
            WorkflowNode::Swarm(n) => Some(&n.config.output_var),
            WorkflowNode::MultiAgent(n) => Some(&n.config.output_var),
            WorkflowNode::Storage(n) => Some(&n.config.output_var),
            WorkflowNode::Approval(n) => Some(&n.config.output_var),
            WorkflowNode::End(n) => n.config.output_var.as_deref(),
            // 无 output_var 字段的节点类型
            WorkflowNode::Llm(_)
            | WorkflowNode::Condition(_)
            | WorkflowNode::Parallel(_)
            | WorkflowNode::Loop(_)
            | WorkflowNode::Merge(_)
            | WorkflowNode::Delay(_)
            | WorkflowNode::Validation(_)
            | WorkflowNode::Switch(_)
            | WorkflowNode::WorkflowRef(_) => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, TS)]
pub struct TriggerNode {
    #[serde(flatten)]
    pub base: WorkflowNodeBase,
    pub config: TriggerConfig,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema, TS)]
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
    #[serde(rename = "grouping")]
    Grouping,
    #[serde(rename = "debateRound")]
    DebateRound,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, TS)]
pub struct WorkflowEdge {
    pub id: String,
    pub source: String,
    #[serde(rename = "sourceHandle")]
    pub source_handle: Option<String>,
    pub target: String,
    #[serde(rename = "targetHandle")]
    pub target_handle: Option<String>,
    pub edge_type: EdgeType,
    pub label: Option<String>,
}

/// 子图定义：嵌入在容器节点中的独立工作流（nodes + edges）。
/// 编辑器展开容器节点时，渲染 `sub_graph.nodes` 作为内部子节点网格；
/// 折叠时根据子节点数量显示计数。
///
/// 子图内的入口/出口节点自动映射为容器节点的端口（port auto-passthrough）。
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, TS)]
pub struct SubGraph {
    pub nodes: Vec<WorkflowNode>,
    pub edges: Vec<WorkflowEdge>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, TS)]
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

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, TS)]
pub struct WorkflowRetryPolicy {
    pub max_retries: u32,
    pub base_delay_ms: u64,
    pub max_delay_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, TS)]
pub struct CompensationStep {
    pub step_id: String,
    pub compensate_type: String,
    pub target_step: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, TS)]
pub struct ErrorConfig {
    pub retry_policy: Option<WorkflowRetryPolicy>,
    pub on_failure: OnFailureAction,
    pub error_branch: Option<Vec<String>>,
    pub compensation_steps: Option<Vec<CompensationStep>>,
}

/// Rhai 脚本工具定义（不属于 DAG 节点，仅作为工具注册）
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, TS)]
pub struct RhaiToolDef {
    /// 注册为工具名（Agent exposed_tools 引用此名）
    pub tool_name: String,
    /// 工具描述（发给 LLM）
    pub description: Option<String>,
    /// Rhai 脚本代码
    pub code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
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
    /// 能力可见性（元能力隔离核心）：SystemOnly 的系统模板（如认知编排器）
    /// 不注册进业务能力注册表、不可被用户发现/编辑/删除。
    /// 默认 Public，保证旧数据反序列化兼容。
    #[serde(default)]
    pub visibility: crate::capability::Visibility,
    pub trigger_config: Option<TriggerConfig>,
    pub nodes: Vec<WorkflowNode>,
    pub edges: Vec<WorkflowEdge>,
    pub input_schema: Option<JsonSchema>,
    pub output_schema: Option<JsonSchema>,
    pub variables: Vec<Variable>,
    pub error_config: Option<ErrorConfig>,
    /// 错误工作流 ID：节点失败时触发独立的错误处理工作流，注入 $error 上下文
    #[serde(default)]
    pub error_workflow_id: Option<String>,
    /// Rhai 工具定义（非 DAG 节点，仅注册为可调用工具）
    pub tool_defs: Vec<RhaiToolDef>,
    /// mission 哈希（SHA-256），用于 compile_mission_to_template 去重缓存。
    /// 由 mission 编译生成的模板填充；手动创建的模板为 None。
    #[serde(default)]
    pub mission_hash: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

impl WorkflowTemplateData {
    /// 是否为系统模板（认知编排器等）。
    ///
    /// 判定规则：`is_preset=true` 且带 `cognitive_router` 标签。
    /// 与 [CapabilityPassport::visibility] 的运行时推导规则保持一致，
    /// 是系统模板与业务工作流物理隔离的唯一权威判定，供 CRUD 命令与
    /// 前端响应（is_system 字段）复用。
    pub fn is_system_template(&self) -> bool {
        if !self.is_preset {
            return false;
        }
        self.tags.iter().any(|t| t == "cognitive_router")
    }
}

// ── 工作流运行时执行态 DTO(阶段 2 从 rt-workflow 上移)──
//
// 这些类型是工作流执行过程中的纯数据快照,被 rt-workflow 引擎、
// 前端进度推送、反思器(WorkflowReflector)共同使用。
// 含运行时句柄(闭包/Arc<dyn Trait>)的类型仍保留在 rt-workflow。

use std::collections::HashMap;

/// 节点运行时状态(等价于原 StepStatus)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum NodeStatus {
    Pending,
    Ready,
    Running,
    Completed,
    Failed,
    Skipped,
}

impl NodeStatus {
    /// 状态机合法性校验:检查从 `current` 状态迁移到 `self`(目标状态) 是否合法。
    ///
    /// 合法迁移规则:
    /// - 同态幂等:任何状态保持自身都合法(用于进度心跳等场景)
    /// - 终态(Completed/Failed/Skipped) → 终态(自身或其他终态):允许(幂等或补偿)
    /// - Pending → Ready/Skipped/Completed/Failed:就绪/被跳过/补偿直接标记
    /// - Ready → Running/Skipped:开始执行或被跳过
    /// - Running → Completed/Failed/Skipped:完成/失败/被取消
    ///
    /// 非法迁移:
    /// - 终态 → 非终态(Completed/Failed/Skipped → Pending/Ready/Running):禁止
    /// - Pending → Running:必须先经过 Ready
    /// - Ready → Completed/Failed:必须先经过 Running
    pub fn is_valid_transition_from(self, current: NodeStatus) -> bool {
        use NodeStatus::*;
        // 同态幂等:任何状态保持自身都合法
        if self == current {
            return true;
        }
        // 终态不变性:终态不能回退到非终态
        if current.is_terminal() && !self.is_terminal() {
            return false;
        }
        // 从 Pending:允许 Ready/Skipped/Completed/Failed,禁止 Running
        if matches!(current, Pending) {
            return matches!(self, Ready | Skipped | Completed | Failed);
        }
        // 从 Ready:允许 Running/Skipped,禁止 Completed/Failed/Pending
        if matches!(current, Ready) {
            return matches!(self, Running | Skipped);
        }
        // 从 Running:允许 Completed/Failed/Skipped (同态 Running 已在顶部处理)
        if matches!(current, Running) {
            return matches!(self, Completed | Failed | Skipped);
        }
        // 从终态到其他终态:允许
        if current.is_terminal() && self.is_terminal() {
            return true;
        }
        false
    }

    /// 是否为终态(不可再迁移到非终态)
    pub fn is_terminal(self) -> bool {
        matches!(self, NodeStatus::Completed | NodeStatus::Failed | NodeStatus::Skipped)
    }
}

/// 工作流整体状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowStatus {
    Created,
    Running,
    Completed,
    PartiallyCompleted,
    Failed,
    Cancelled,
}

/// 工作流执行状态(与 `WorkflowStatus` 区别:含 Paused,不含 Created)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
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
            Self::Running => write!(f, "running"),
            Self::Paused => write!(f, "paused"),
            Self::Completed => write!(f, "completed"),
            Self::PartiallyCompleted => write!(f, "partially_completed"),
            Self::Failed => write!(f, "failed"),
            Self::Cancelled => write!(f, "cancelled"),
        }
    }
}

/// 单个节点的运行时追踪状态。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct NodeRuntimeState {
    pub status: NodeStatus,
    pub attempts: u32,
    pub error: Option<String>,
    pub started_at: Option<i64>,
    pub completed_at: Option<i64>,
}

impl Default for NodeRuntimeState {
    fn default() -> Self {
        Self {
            status: NodeStatus::Pending,
            attempts: 0,
            error: None,
            started_at: None,
            completed_at: None,
        }
    }
}

/// 工作流运行时容器。
///
/// nodes/edges 来自 `WorkflowNode`/`WorkflowEdge`,
/// 运行时状态(status/results/node_states)存储在内存中。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct Workflow {
    pub id: String,
    pub name: String,
    pub nodes: Vec<WorkflowNode>,
    pub edges: Vec<WorkflowEdge>,
    pub status: WorkflowStatus,
    pub created_at: u64,
    pub completed_at: Option<u64>,
    /// 节点执行结果 keyed by node_id
    pub results: HashMap<String, serde_json::Value>,
    /// 每个节点的运行时状态
    pub node_states: HashMap<String, NodeRuntimeState>,
    /// 工作流最终输出(经 output_schema 过滤或 EndNode 聚合后的精简结果)
    pub output: Option<serde_json::Value>,
    /// 错误处理配置(模板级)
    #[serde(default)]
    pub error_config: Option<ErrorConfig>,
    /// 错误工作流 ID(模板级,节点失败时触发独立的错误处理工作流)
    #[serde(default)]
    pub error_workflow_id: Option<String>,
}

/// 单个节点的执行记录(用于持久化与前端展示)。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
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
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct PartialResultEvent {
    pub execution_id: String,
    pub node_id: String,
    /// 0-based 迭代下标。
    pub iter_index: u32,
    /// 当前元素(item),与 `iter_input_var` 数组中第 `iter_index` 个元素一致。
    pub item: serde_json::Value,
    /// body 最后一节点的输出(聚合视角下的"本轮结果")。
    pub step_output: serde_json::Value,
    /// 累计 partial_result 数组(长度 = iter_index + 1)。
    pub cumulative_partial: Vec<serde_json::Value>,
    /// 触发本事件的源:正常完成 / interrupt / 错误。
    pub phase: String,
    /// 时间戳(毫秒)。
    pub emitted_at_ms: i64,
}

/// 步骤进度事件。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct StepProgressEvent {
    pub node_id: String,
    pub status: String,
    pub total_nodes: usize,
    pub completed_nodes: usize,
    pub execution_id: Option<String>,
    /// 节点失败/超时的真实错误信息。此前该字段缺失导致进度回调只能透传占位符，
    /// 前端 Debug 面板无法显示具体失败原因（AxInvest #10 数据工具节点失败不可诊断）。
    /// 仅 failed/timeout 状态携带 Some，其余为 None（向后兼容）。
    pub error: Option<String>,
    /// 节点输出（仅 completed 状态携带）。
    /// 此前该字段缺失导致前端 `workflow-step-done` 事件无法实时获取节点输出，
    /// 分析师卡片只能在工作流全部结束后由 `workflow-completed` 批量填充，
    /// 无法实现"一边进行一边填充"（AxInvest 分析师 tab 实时性修复）。
    /// running/failed/timeout 状态为 None（向后兼容）。
    pub output: Option<serde_json::Value>,
}

/// 节点执行心跳事件 —— 用于长时间执行期间的周期性反馈。
///
/// 在节点执行超过一定时间后（默认 30s），引擎会定期（默认每 10s）
/// 发送心跳事件到前端，告知用户系统仍在正常工作，避免"不知道是否
/// 需要继续等待"的问题。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct NodeHeartbeatEvent {
    pub execution_id: String,
    pub node_id: String,
    /// 节点已执行时间（毫秒）。
    pub elapsed_ms: u64,
    /// 心跳计数（第几次心跳）。
    pub heartbeat_count: u32,
    /// 预计超时时间（毫秒），None 表示使用默认。
    pub timeout_ms: Option<u64>,
    /// 时间戳（毫秒）。
    pub emitted_at_ms: i64,
}

/// 节点超时警告事件 —— 执行即将超时或已超时的预警。
///
/// 当节点执行接近超时阈值时（如剩余 30s），引擎会发送预警事件，
/// 让前端可以显示"即将超时"警告，用户可以选择等待或取消。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct NodeTimeoutWarningEvent {
    pub execution_id: String,
    pub node_id: String,
    /// 已执行时间（毫秒）。
    pub elapsed_ms: u64,
    /// 预计超时时间（毫秒）。
    pub timeout_ms: u64,
    /// 剩余时间（毫秒），None 表示已超时。
    pub remaining_ms: Option<u64>,
    /// 警告级别：warning（接近超时）/ critical（已超时但仍在执行）。
    pub level: String,
    /// 时间戳（毫秒）。
    pub emitted_at_ms: i64,
}

/// DAG 进度简报事件 —— 用于 TTS 语音播报
///
/// 在工作流执行的关键节点（开始/节点完成/结束）触发，
/// 携带自然语言描述 + 结构化进度数据，供前端 TTS 通道播报。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressBriefEvent {
    pub execution_id: String,
    pub workflow_id: String,
    /// 简报类型：workflow_start / node_progress / workflow_complete
    pub brief_type: String,
    /// 自然语言描述（用于 TTS 播报）
    pub description: String,
    /// 可选：当前节点 ID
    pub current_node_id: Option<String>,
    /// 可选：已完成节点数
    pub completed_count: Option<u32>,
    /// 可选：总节点数
    pub total_count: Option<u32>,
    /// 可选：执行耗时（ms）
    pub elapsed_ms: Option<u64>,
    /// 时间戳（毫秒）
    pub emitted_at_ms: i64,
}

/// 审核证据标识（ReviewEvidenceIdentity）
///
/// 用于审计链中标识每个评分/决策的证据来源，
/// 支持 DAG 级别的可追溯性和可验证性。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ReviewEvidenceIdentity {
    /// 证据唯一 ID
    pub id: String,
    /// 证据类型：scorecard / replay / eval_run / manual
    pub evidence_type: String,
    /// 关联的执行 ID
    pub execution_id: String,
    /// 关联的工作流 ID
    pub workflow_id: String,
    /// 证据摘要（如评分、决策说明）
    pub summary: String,
    /// 证据哈希（用于防篡改验证）
    pub evidence_hash: String,
    /// 证据来源（系统自动/人工）
    pub source: String,
    /// 创建时间（UTC）
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl ReviewEvidenceIdentity {
    pub fn new(
        evidence_type: impl Into<String>,
        execution_id: impl Into<String>,
        workflow_id: impl Into<String>,
        summary: impl Into<String>,
    ) -> Self {
        let type_str: String = evidence_type.into();
        let exec: String = execution_id.into();
        let wf: String = workflow_id.into();
        let summ: String = summary.into();
        let hash_input = format!("{type_str}:{exec}:{wf}:{summ}");

        // 使用 SHA-256 生成真正的加密哈希，防止篡改
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(hash_input.as_bytes());
        let hash_result = hasher.finalize();
        let evidence_hash = hash_result.iter().map(|b| format!("{:02x}", b)).collect::<String>();

        Self {
            id: uuid::Uuid::new_v4().to_string(),
            evidence_type: type_str,
            execution_id: exec,
            workflow_id: wf,
            summary: summ,
            evidence_hash,
            source: "system".to_string(),
            created_at: chrono::Utc::now(),
        }
    }
}

// ── WorkflowTemplateData: CapabilityPassport 实现 ──────

impl crate::capability::CapabilityPassport for WorkflowTemplateData {
    fn capability_id(&self) -> String {
        format!("workflow:{}", self.id)
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        self.description.as_deref().unwrap_or("")
    }

    fn kind(&self) -> crate::capability::CapabilityKind {
        crate::capability::CapabilityKind::Workflow
    }

    fn domain(&self) -> crate::capability::CapabilityDomain {
        infer_domain_from_tags(&self.tags)
    }

    fn input_schema(&self) -> Option<serde_json::Value> {
        self.input_schema.as_ref().and_then(|s| serde_json::to_value(s).ok())
    }

    fn tags(&self) -> Vec<String> {
        self.tags.clone()
    }

    fn visibility(&self) -> crate::capability::Visibility {
        // 系统预置路由模板（认知编排器等）运行时推导为 SystemOnly：
        // workflow_templates 表未持久化 visibility 列，模板从 DB 读出时字段
        // 会回到默认 Public，此处用 is_preset + cognitive_router 标签兜底，
        // 保证其护照永远不被注册进业务能力注册表。
        if self.is_preset && self.tags.iter().any(|t| t == "cognitive_router") {
            crate::capability::Visibility::SystemOnly
        } else {
            self.visibility
        }
    }

    fn planning_complexity(&self) -> crate::capability::PlanningComplexity {
        // 根据节点数量判断：<=3 Simple，4-10 Moderate，>10 Complex
        match self.nodes.len() {
            0..=3 => crate::capability::PlanningComplexity::Simple,
            4..=10 => crate::capability::PlanningComplexity::Moderate,
            _ => crate::capability::PlanningComplexity::Complex,
        }
    }

    fn is_enabled(&self) -> bool {
        true
    }
}

/// 根据工作流 tags 推断所属业务域。
///
/// 匹配优先级：ContentCreation > Finance > Automation > Devops > AiMedia > DataAnalysis > Communication > General
fn infer_domain_from_tags(tags: &[String]) -> crate::capability::CapabilityDomain {
    use crate::capability::CapabilityDomain;

    let tag_set: std::collections::HashSet<&str> = tags.iter().map(|s| s.as_str()).collect();

    // 内容创作（文学/小说/诗歌/散文/写作/创作）
    if [
        "literary",
        "novel",
        "poetry",
        "prose",
        "narrative-structure",
        "writing",
        "creation",
        "translation",
        "polishing",
        "content",
        "design",
        "marketing",
        "copywriting",
    ]
    .iter()
    .any(|t| tag_set.contains(t))
    {
        return CapabilityDomain::ContentCreation;
    }

    // 金融（股票/交易/行情/风控）
    if ["stock", "trading", "finance", "investment", "market", "risk", "portfolio", "quant", "fund"]
        .iter()
        .any(|t| tag_set.contains(t))
    {
        return CapabilityDomain::Finance;
    }

    // 自动化（RPA/工作流/编排）
    if ["automation", "workflow", "opc", "rpa", "orchestration", "order", "refund", "shipping"]
        .iter()
        .any(|t| tag_set.contains(t))
    {
        return CapabilityDomain::Automation;
    }

    // 运维（CI/CD/部署/监控）
    if ["devops", "deployment", "monitoring", "cicd", "docker", "kubernetes", "security"]
        .iter()
        .any(|t| tag_set.contains(t))
    {
        return CapabilityDomain::Devops;
    }

    // AI 媒体（图像/视频/音频）
    if ["ai_media", "image", "video", "audio", "generation", "viral-content", "ip-building"]
        .iter()
        .any(|t| tag_set.contains(t))
    {
        return CapabilityDomain::AiMedia;
    }

    // 数据分析
    if ["data_analysis", "sql", "visualization", "etl", "analytics", "bi"]
        .iter()
        .any(|t| tag_set.contains(t))
    {
        return CapabilityDomain::DataAnalysis;
    }

    // 通信
    if ["communication", "im", "email", "messaging", "notification", "push"]
        .iter()
        .any(|t| tag_set.contains(t))
    {
        return CapabilityDomain::Communication;
    }

    // 兜底：通用域
    CapabilityDomain::General
}

/// 活跃执行摘要(仅内存运行态,用于可观测性 / 前端轮询)。
///
/// 字段精简自 rt-workflow `ExecutionState`,剔除 callbacks / compiled_prompts / cancel_token
/// 等非序列化运行时槽,便于直接 serde 序列化回传前端。
#[derive(Debug, Clone, Serialize, TS)]
pub struct ActiveExecutionInfo {
    pub execution_id: String,
    pub workflow_id: String,
    pub status: ExecutionStatus,
    pub current_node_id: Option<String>,
    pub parent_execution_id: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

/// 工作流错误上下文 — 在 Error Workflow 中通过 `$error` / `_error` 变量访问。
///
/// 当节点执行失败且配置了 RunErrorBranch 或 error_workflow_id 时,
/// 引擎构造此上下文并注入到 ExecutionState 变量中,供错误处理
/// 工作流引用失败节点的详细信息。
///
/// **重命名说明**:原 rt-workflow `ErrorContext` 上移到 harness 时
/// 重命名为 `WorkflowErrorContext`,避免与 `crate::core_error::ErrorContext`
/// (telemetry 语义)冲突。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct WorkflowErrorContext {
    pub failed_node_id: String,
    pub failed_node_name: String,
    pub error_code: String,
    pub error_message: String,
    pub workflow_id: String,
    pub execution_id: String,
    pub timestamp: i64,
    pub last_output: Option<serde_json::Value>,
}

impl WorkflowErrorContext {
    pub fn new(
        node_id: String,
        node_name: String,
        error_code: String,
        error_message: String,
        workflow_id: String,
        execution_id: String,
        last_output: Option<serde_json::Value>,
    ) -> Self {
        Self {
            failed_node_id: node_id,
            failed_node_name: node_name,
            error_code,
            error_message,
            workflow_id,
            execution_id,
            timestamp: chrono::Utc::now().timestamp_millis(),
            last_output,
        }
    }

    /// 获取可在模板中引用的变量名。
    pub const fn variable_name() -> &'static str {
        "_error"
    }

    /// 将错误上下文序列化为 Value,注入到 variables 中。
    pub fn to_variable(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or(serde_json::Value::Null)
    }
}

/// 工作流错误类型。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub enum WorkflowError {
    DuplicateNodeId(String),
    InvalidDependency { node_id: String, missing_dep: String },
    WorkflowNotFound,
    NodeNotFound,
    CycleDetected,
    SerializationError(String),
    InputValidationFailed { errors: Vec<String> },
    OutputValidationFailed { errors: Vec<String> },
    InvalidStateTransition(String),
}

impl std::fmt::Display for WorkflowError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DuplicateNodeId(id) => write!(f, "Duplicate node ID: {id}"),
            Self::InvalidDependency { node_id, missing_dep } => {
                write!(f, "Node '{node_id}' depends on non-existent '{missing_dep}'")
            },
            Self::WorkflowNotFound => write!(f, "Workflow not found"),
            Self::NodeNotFound => write!(f, "Node not found"),
            Self::CycleDetected => write!(f, "Cycle detected in workflow"),
            Self::SerializationError(msg) => write!(f, "Serialization error: {msg}"),
            Self::InputValidationFailed { errors } => {
                write!(f, "Input validation failed: {}", errors.join("; "))
            },
            Self::OutputValidationFailed { errors } => {
                write!(f, "Output validation failed: {}", errors.join("; "))
            },
            Self::InvalidStateTransition(msg) => write!(f, "非法状态迁移: {msg}"),
        }
    }
}

impl std::error::Error for WorkflowError {}

/// 不可变的模板版本快照（用于历史表/版本对比/导入导出）。
/// 区别于 `WorkflowTemplateData`：没有 `composite_source`/`tool_defs`，
/// 但多了 `template_id` 显式外键（version 行的 `id` 是 `{template_id}_v{n}`）。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct WorkflowTemplateVersionData {
    pub template_id: String,
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
    pub created_at: i64,
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
            mission_hash: self.mission_hash.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, TS)]
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
    /// mission 哈希（SHA-256），用于 compile_mission_to_template 去重缓存。
    /// 仅当此模板由 mission 编译生成时填充；手动创建时为 None。
    #[serde(default)]
    pub mission_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
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
    /// 是否为系统模板（认知编排器等）。由后端按
    /// `is_preset + cognitive_router 标签` 权威判定，前端据此
    /// 区分系统模板页与业务模板页（系统模板可查看/编辑但禁止删除/复制/导出）。
    #[serde(default)]
    pub is_system: bool,
    pub trigger_config: Option<TriggerConfig>,
    pub nodes: Vec<WorkflowNode>,
    pub edges: Vec<WorkflowEdge>,
    pub input_schema: Option<JsonSchema>,
    pub output_schema: Option<JsonSchema>,
    pub variables: Vec<Variable>,
    pub error_config: Option<ErrorConfig>,
    pub tool_defs: Option<Vec<RhaiToolDef>>,
    /// mission 哈希（SHA-256），若模板由 mission 编译生成则填充
    #[serde(default)]
    pub mission_hash: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

impl From<WorkflowTemplateData> for WorkflowTemplateResponse {
    fn from(data: WorkflowTemplateData) -> Self {
        let is_system = data.is_system_template();
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
            is_system,
            trigger_config: data.trigger_config,
            nodes: data.nodes,
            edges: data.edges,
            input_schema: data.input_schema,
            output_schema: data.output_schema,
            variables: data.variables,
            error_config: data.error_config,
            tool_defs: Some(data.tool_defs),
            mission_hash: data.mission_hash,
            created_at: data.created_at,
            updated_at: data.updated_at,
        }
    }
}

// ── 模板筛选、校验结果 ──────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, TS)]
pub struct TemplateFilter {
    pub is_preset: Option<bool>,
    pub tags: Option<Vec<String>>,
    pub search: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, TS)]
pub struct ValidationError {
    pub error_type: String,
    pub node_id: Option<String>,
    pub message: String,
    pub suggestion: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, TS)]
pub struct ValidationWarning {
    pub warning_type: String,
    pub node_id: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, TS)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub errors: Vec<ValidationError>,
    pub warnings: Vec<ValidationWarning>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_isomorphic_transitions_always_valid() {
        // 同态幂等: 任何状态保持自身都合法
        let statuses = [
            NodeStatus::Pending,
            NodeStatus::Ready,
            NodeStatus::Running,
            NodeStatus::Completed,
            NodeStatus::Failed,
            NodeStatus::Skipped,
        ];
        for s in &statuses {
            assert!(s.is_valid_transition_from(*s), "{:?} → {:?} 应合法(同态幂等)", s, s);
        }
    }

    #[test]
    fn test_pending_valid_transitions() {
        assert!(NodeStatus::Ready.is_valid_transition_from(NodeStatus::Pending));
        assert!(NodeStatus::Skipped.is_valid_transition_from(NodeStatus::Pending));
        assert!(NodeStatus::Completed.is_valid_transition_from(NodeStatus::Pending));
        assert!(NodeStatus::Failed.is_valid_transition_from(NodeStatus::Pending));
    }

    #[test]
    fn test_pending_to_running_rejected() {
        assert!(!NodeStatus::Running.is_valid_transition_from(NodeStatus::Pending));
    }

    #[test]
    fn test_ready_valid_transitions() {
        assert!(NodeStatus::Running.is_valid_transition_from(NodeStatus::Ready));
        assert!(NodeStatus::Skipped.is_valid_transition_from(NodeStatus::Ready));
    }

    #[test]
    fn test_ready_to_terminal_rejected() {
        assert!(!NodeStatus::Completed.is_valid_transition_from(NodeStatus::Ready));
        assert!(!NodeStatus::Failed.is_valid_transition_from(NodeStatus::Ready));
    }

    #[test]
    fn test_running_valid_transitions() {
        assert!(NodeStatus::Completed.is_valid_transition_from(NodeStatus::Running));
        assert!(NodeStatus::Failed.is_valid_transition_from(NodeStatus::Running));
        assert!(NodeStatus::Skipped.is_valid_transition_from(NodeStatus::Running));
    }

    #[test]
    fn test_running_to_non_terminal_rejected() {
        assert!(!NodeStatus::Pending.is_valid_transition_from(NodeStatus::Running));
        assert!(!NodeStatus::Ready.is_valid_transition_from(NodeStatus::Running));
    }

    #[test]
    fn test_terminal_cannot_regress_to_non_terminal() {
        let terminals = [NodeStatus::Completed, NodeStatus::Failed, NodeStatus::Skipped];
        let non_terminals = [NodeStatus::Pending, NodeStatus::Ready, NodeStatus::Running];
        for term in &terminals {
            for non_term in &non_terminals {
                assert!(
                    !non_term.is_valid_transition_from(*term),
                    "终态 {:?} → 非终态 {:?} 应被拒绝",
                    term,
                    non_term
                );
            }
        }
    }

    #[test]
    fn test_terminal_to_terminal_allowed() {
        // 终态之间的幂等/补偿转换允许
        assert!(NodeStatus::Completed.is_valid_transition_from(NodeStatus::Failed));
        assert!(NodeStatus::Failed.is_valid_transition_from(NodeStatus::Completed));
        assert!(NodeStatus::Skipped.is_valid_transition_from(NodeStatus::Failed));
        assert!(NodeStatus::Completed.is_valid_transition_from(NodeStatus::Skipped));
    }

    #[test]
    fn test_is_terminal() {
        assert!(NodeStatus::Completed.is_terminal());
        assert!(NodeStatus::Failed.is_terminal());
        assert!(NodeStatus::Skipped.is_terminal());
        assert!(!NodeStatus::Pending.is_terminal());
        assert!(!NodeStatus::Ready.is_terminal());
        assert!(!NodeStatus::Running.is_terminal());
    }
}

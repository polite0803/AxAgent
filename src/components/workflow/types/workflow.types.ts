export interface Position {
  x: number;
  y: number;
}

export interface RetryConfig {
  enabled: boolean;
  max_retries: number;
  backoff_type: "Linear" | "Exponential" | "Fixed";
  base_delay_ms: number;
  max_delay_ms: number;
}

export interface JsonSchema {
  type: string;
  description?: string;
  properties?: Record<string, JsonSchemaProperty>;
  required?: string[];
  items?: JsonSchema;
}

export interface JsonSchemaProperty {
  type: string;
  description?: string;
  default?: unknown;
  enum_values?: unknown[];
  format?: string;
}

export interface Variable {
  name: string;
  var_type: string;
  value: unknown;
  description?: string;
  is_secret: boolean;
}

export interface WorkflowNodeBase {
  id: string;
  title: string;
  description?: string;
  position: Position;
  retry: RetryConfig;
  timeout?: number;
  enabled: boolean;
  /** 容器父节点 ID。保存时由编辑器注入，用于 Parallel/Merge 等容器子节点的定位。 */
  parentId?: string;
}

export type TriggerType = "manual" | "schedule" | "webhook" | "event";

export interface TriggerConfig {
  type: TriggerType;
  config: unknown;
}

export interface ManualTriggerConfig {}

export interface ScheduleTriggerConfig {
  cron: string;
  timezone: string;
  enabled: boolean;
}

export interface WebhookTriggerConfig {
  path: string;
  method: string;
  auth_type: string;
}

export interface EventTriggerConfig {
  event_type: string;
  filter?: unknown;
}

export type OutputMode = "json" | "text" | "artifact";

/** 工具定义 —— 名称、描述和参数 JSON Schema */
export interface ToolDef {
  name: string;
  description?: string;
  parameters?: JsonSchema;
}

export interface AgentNodeConfig {
  /** AgentProfile ID — 唯一标识角色/专家/模型的入口 */
  agentProfileId?: string;
  system_prompt: string;
  promptTemplateId?: string;
  context_sources: string[];
  output_var: string;
  model?: string;
  temperature?: number;
  max_tokens?: number;
  /** 工具列表，支持旧格式 `string[]` 和新格式 `ToolDef[]` */
  tools: ToolDef[];
  /** 暴露给 LLM 的工具名列表（tools 的子集）。空数组 = 暴露全部（向后兼容） */
  exposed_tools: string[];
  output_mode: OutputMode;
  /** 工具调用最大轮数（默认 5，仅 tools 非空时生效） */
  max_tool_rounds?: number;
  /** 执行模式: "react" = 逐步思考-行动, "plan" = 先规划为工作流再执行 */
  execution_mode?: "react" | "plan";
  /** RAG 知识源 ID 列表。格式: "knowledge:<kb_id>", "memory:<ns_id>", "wiki:<wiki_id>" */
  rag_source_ids?: string[];
  model_role?: "quick_think" | "deep_think";
}

export interface AgentNode extends WorkflowNodeBase {
  type: "agent";
  config: AgentNodeConfig;
}

export interface LLMNodeConfig {
  model: string;
  prompt: string;
  promptTemplateId?: string;
  messages?: unknown[];
  temperature?: number;
  max_tokens?: number;
  tools?: string[];
  functions?: unknown[];
}

export interface LLMNode extends WorkflowNodeBase {
  type: "llm";
  config: LLMNodeConfig;
}

export type CompareOperator =
  | "eq"
  | "ne"
  | "gt"
  | "lt"
  | "gte"
  | "lte"
  | "contains"
  | "notContains"
  | "startsWith"
  | "endsWith"
  | "regexMatch"
  | "isEmpty"
  | "isNotEmpty";

export type LogicalOperator = "and" | "or";

export interface Condition {
  var_path: string;
  operator: CompareOperator;
  value: unknown;
}

export interface ConditionNodeConfig {
  conditions: Condition[];
  logical_op: LogicalOperator;
  /** 启用 LLM 动态路由：由 AI 判断走哪条分支（忽略 conditions 静态规则） */
  judge_by_llm?: boolean;
  /** LLM 路由时的提示词（描述路由判断逻辑） */
  routing_prompt?: string;
  /** LLM 路由使用模型（为空则用系统默认） */
  routing_model?: string;
}

export interface ConditionNode extends WorkflowNodeBase {
  type: "condition";
  config: ConditionNodeConfig;
}

export interface Branch {
  id: string;
  title: string;
  steps: string[];
}

export type MergeStrategy = "all" | "any" | "race" | "majority";

export interface ParallelNodeConfig {
  branches: Branch[];
  wait_for_all: boolean;
  timeout?: number;
  aggregation?: MergeStrategy;
  auto_input_from_parent?: boolean;
  /**
   * 容器角色标记。
   *
   * - `"executable"`（默认）：真并行调度器。`wait_for_all` + `aggregation` 实际生效，
   *   运行时引擎并行执行子分支。
   * - `"decorative"`：装饰性分组。仅供前端画分组框，调度引擎忽略。
   *   成员通过 `parentId` 引用，实际依赖通过显式的 `edge` 表达。
   */
  kind?: "decorative" | "executable";
}

export interface ParallelNode extends WorkflowNodeBase {
  type: "parallel";
  config: ParallelNodeConfig;
}

export type LoopType = "forEach" | "while" | "doWhile" | "until";

export interface LoopNodeConfig {
  loop_type: LoopType;
  items_var?: string;
  iteratee_var?: string;
  max_iterations?: number;
  continue_condition?: string;
  continue_on_error: boolean;
  body_steps: string[];
}

export interface LoopNode extends WorkflowNodeBase {
  type: "loop";
  config: LoopNodeConfig;
}

export interface MergeNodeConfig {
  merge_type: MergeStrategy;
  inputs: string[];
  auto_inputs_from_branches?: boolean;
}

export interface MergeNode extends WorkflowNodeBase {
  type: "merge";
  config: MergeNodeConfig;
}

export interface DelayNodeConfig {
  delay_type: string;
  seconds: number;
  until?: string;
}

export interface DelayNode extends WorkflowNodeBase {
  type: "delay";
  config: DelayNodeConfig;
}

export interface ToolNodeConfig {
  tool_name: string;
  input_mapping: Record<string, string>;
  output_var: string;
}

export interface ToolNode extends WorkflowNodeBase {
  type: "tool";
  config: ToolNodeConfig;
}

export interface CodeNodeConfig {
  language: string;
  code: string;
  output_var: string;
  /** Rhai 脚本注册为工具名（language="rhai" 时生效） */
  tool_name?: string;
}

export interface CodeNode extends WorkflowNodeBase {
  type: "code";
  config: CodeNodeConfig;
}

export interface SubWorkflowNodeConfig {
  sub_workflow_id: string;
  input_mapping: Record<string, string>;
  output_var: string;
  is_async: boolean;
}

export interface SubWorkflowNode extends WorkflowNodeBase {
  type: "subWorkflow";
  config: SubWorkflowNodeConfig;
}

/** 工作流引用配置：引用另一个工作流作为子流程执行 */
export interface WorkflowRefNodeConfig {
  /** 被引用的工作流模板 ID */
  target_workflow_id: string;
  /** 参数注入映射：当前上下文变量名 → 子工作流入参名 */
  input_mapping: Record<string, string>;
  /** 子工作流输出变量名 */
  output_var: string;
  /** 超时继承：不设置则使用当前工作流默认超时 */
  timeout?: number;
  /** 上下文传递模式 */
  context_mode?: "inherit" | "isolated";
}

export interface WorkflowRefNode extends WorkflowNodeBase {
  type: "workflowRef";
  config: WorkflowRefNodeConfig;
}

export interface DocumentParserNodeConfig {
  input_var: string;
  parser_type: string;
  output_var: string;
}

export interface DocumentParserNode extends WorkflowNodeBase {
  type: "documentParser";
  config: DocumentParserNodeConfig;
}

export interface VectorRetrieveNodeConfig {
  query: string;
  knowledge_base_id: string;
  top_k: number;
  similarity_threshold?: number;
  output_var: string;
}

export interface VectorRetrieveNode extends WorkflowNodeBase {
  type: "vectorRetrieve";
  config: VectorRetrieveNodeConfig;
}

export interface EndNodeConfig {
  output_var?: string;
}

export interface EndNode extends WorkflowNodeBase {
  type: "end";
  config: EndNodeConfig;
}

export interface ValidationNodeConfig {
  assertions: Array<{
    type: "equals" | "contains" | "matches" | "exists" | "custom";
    expected?: string;
    actual?: string;
    expression?: string;
  }>;
  on_fail: "stop" | "retry" | "continue";
  max_retries: number;
}

export interface ValidationNode extends WorkflowNodeBase {
  type: "validation";
  config: ValidationNodeConfig;
}

export interface TriggerNode extends WorkflowNodeBase {
  type: "trigger";
  config: TriggerConfig;
}

export interface HttpRequestNodeConfig {
  url: string;
  method: string;
  headers: Record<string, string>;
  body?: string;
  body_type: string;
  timeout_secs: number;
  output_var: string;
}

export interface HttpRequestNode extends WorkflowNodeBase {
  type: "httpRequest";
  config: HttpRequestNodeConfig;
}

export interface SwitchCase {
  value: string;
  label: string;
}

export interface SwitchNodeConfig {
  input_var: string;
  cases: SwitchCase[];
  default_case?: string;
  match_mode: string;
  output_var: string;
}

export interface SwitchNode extends WorkflowNodeBase {
  type: "switch";
  config: SwitchNodeConfig;
}

export interface DatabaseQueryNodeConfig {
  query: string;
  params: string[];
  connection_name?: string;
  timeout_secs: number;
  output_var: string;
}

export interface DatabaseQueryNode extends WorkflowNodeBase {
  type: "databaseQuery";
  config: DatabaseQueryNodeConfig;
}

export interface NotificationNodeConfig {
  channel: string;
  message: string;
  webhook_url?: string;
  recipients: string[];
  subject?: string;
  enabled: boolean;
  output_var: string;
}
export interface NotificationNode extends WorkflowNodeBase {
  type: "notification";
  config: NotificationNodeConfig;
}

export interface ApprovalNodeConfig {
  message: string;
  approver?: string;
  timeout_secs: number;
  timeout_action: string;
  output_var: string;
}
export interface ApprovalNode extends WorkflowNodeBase {
  type: "approval";
  config: ApprovalNodeConfig;
}

export interface FileOperationNodeConfig {
  operation: string;
  file_path: string;
  content?: string;
  output_var: string;
}
export interface FileOperationNode extends WorkflowNodeBase {
  type: "fileOperation";
  config: FileOperationNodeConfig;
}

export interface DataTransformerNodeConfig {
  input_var: string;
  expression: string;
  output_var: string;
}
export interface DataTransformerNode extends WorkflowNodeBase {
  type: "dataTransformer";
  config: DataTransformerNodeConfig;
}

export interface WebhookSendNodeConfig {
  url: string;
  method: string;
  body?: string;
  headers: Record<string, string>;
  output_var: string;
}
export interface WebhookSendNode extends WorkflowNodeBase {
  type: "webhookSend";
  config: WebhookSendNodeConfig;
}

export interface LoggingNodeConfig {
  level: string;
  message: string;
  output_var: string;
}
export interface LoggingNode extends WorkflowNodeBase {
  type: "logging";
  config: LoggingNodeConfig;
}

export interface LlmClassifierNodeConfig {
  categories: string[];
  prompt: string;
  model?: string;
  input_var: string;
  output_var: string;
}
export interface LlmClassifierNode extends WorkflowNodeBase {
  type: "llmClassifier";
  config: LlmClassifierNodeConfig;
}

export interface AggregatorNodeConfig {
  strategy: string;
  input_sources: string[];
  output_var: string;
}
export interface AggregatorNode extends WorkflowNodeBase {
  type: "aggregator";
  config: AggregatorNodeConfig;
}

export interface EmailNodeConfig {
  to: string[];
  subject: string;
  body: string;
  smtp_host?: string;
  smtp_port?: number;
  smtp_user?: string;
  smtp_pass?: string;
  output_var: string;
}
export interface EmailNode extends WorkflowNodeBase {
  type: "email";
  config: EmailNodeConfig;
}

export interface DebateNodeConfig {
  debater_steps: string[];
  max_rounds: number;
  convergence_prompt?: string;
  convergence_model?: string;
  convergence_model_role?: string;
  topic_var: string;
  output_var: string;
}

export interface DebateNode extends WorkflowNodeBase {
  type: "debate";
  config: DebateNodeConfig;
}

export type WorkflowNode =
  | TriggerNode
  | AgentNode
  | LLMNode
  | ConditionNode
  | ParallelNode
  | LoopNode
  | MergeNode
  | DelayNode
  | ToolNode
  | CodeNode
  | SubWorkflowNode
  | WorkflowRefNode
  | DocumentParserNode
  | VectorRetrieveNode
  | ValidationNode
  | EndNode
  | HttpRequestNode
  | SwitchNode
  | DatabaseQueryNode
  | NotificationNode
  | ApprovalNode
  | FileOperationNode
  | DataTransformerNode
  | WebhookSendNode
  | LoggingNode
  | LlmClassifierNode
  | AggregatorNode
  | EmailNode
  | DebateNode;

export type EdgeType =
  | "direct"
  | "conditionTrue"
  | "conditionFalse"
  | "loopBack"
  | "parallelBranch"
  | "merge"
  | "debateRound"
  | "error"
  | "grouping";

export interface WorkflowEdge {
  id: string;
  source: string;
  sourceHandle?: string;
  target: string;
  targetHandle?: string;
  edge_type: EdgeType;
  label?: string;
}

export type OnFailureAction =
  | "abort"
  | "retryThenAbort"
  | "runErrorBranch"
  | "continueWithDefault";

export interface RetryPolicy {
  max_retries: number;
  base_delay_ms: number;
  max_delay_ms: number;
}

export interface CompensationStep {
  step_id: string;
  compensate_type: string;
  target_step: string;
}

export interface ErrorConfig {
  retry_policy?: RetryPolicy;
  on_failure: OnFailureAction;
  error_branch?: string[];
  compensation_steps?: CompensationStep[];
}

export interface RhaiToolDef {
  tool_name: string;
  description?: string;
  code: string;
}

export interface WorkflowTemplateInput {
  name: string;
  description?: string;
  icon: string;
  tags: string[];
  trigger_config?: TriggerConfig;
  nodes: WorkflowNode[];
  edges: WorkflowEdge[];
  input_schema?: JsonSchema;
  output_schema?: JsonSchema;
  variables: Variable[];
  error_config?: ErrorConfig;
  tool_defs?: RhaiToolDef[];
}

export interface WorkflowTemplateResponse {
  id: string;
  name: string;
  description?: string;
  icon: string;
  tags: string[];
  version: number;
  is_preset: boolean;
  is_editable: boolean;
  is_public: boolean;
  trigger_config?: TriggerConfig;
  nodes: WorkflowNode[];
  edges: WorkflowEdge[];
  input_schema?: JsonSchema;
  output_schema?: JsonSchema;
  variables: Variable[];
  error_config?: ErrorConfig;
  tool_defs?: RhaiToolDef[];
  created_at: number;
  updated_at: number;
}

export interface TemplateFilter {
  is_preset?: boolean;
  tags?: string[];
  search?: string;
}

export interface ValidationError {
  error_type: string;
  node_id?: string;
  message: string;
  suggestion?: string;
}

export interface ValidationWarning {
  warning_type: string;
  node_id?: string;
  message: string;
}

export interface ValidationResult {
  is_valid: boolean;
  errors: ValidationError[];
  warnings: ValidationWarning[];
}

export type DiagnosticSeverity = "error" | "warning" | "info";
export type DiagnosticCategory =
  | "structure"
  | "configuration"
  | "prompt"
  | "prompt_quality"
  | "performance"
  | "cost"
  | "security"
  | "reference"
  | "best_practice"
  | "semantic_conflict"
  | string;

/**
 * 自动修复动作（discriminated union，与 Rust `#[serde(tag = "action_type")]` 展平序列化对齐）
 * - set_node_field: 覆盖节点的 config 字段
 * - delete_node:    删除指定节点
 * - delete_edge:    删除指定边
 * - enable_retry:   启用节点重试
 * - set_timeout:    设置节点超时
 */
export type DiagnosticFix =
  | {
    action_type: "set_node_field";
    node_id: string;
    field: string;
    value: unknown;
  }
  | { action_type: "delete_node"; node_id: string }
  | { action_type: "delete_edge"; edge_id: string }
  | { action_type: "enable_retry"; node_id: string; max_retries: number }
  | { action_type: "set_timeout"; node_id: string; timeout_ms: number }
  | { action_type: "remove_debater_step"; node_id: string; step_id: string };

export interface DiagnosticIssue {
  id: string;
  severity: DiagnosticSeverity;
  category: DiagnosticCategory;
  title_key: string;
  message_key: string;
  message_params?: Record<string, string | number>;
  node_ids: string[];
  edge_ids?: string[];
  auto_fixable: boolean;
  fix?: DiagnosticFix;
  title_override?: string;
  detail_override?: string;
  suggestion_override?: string;
}

export interface DiagnosticSummary {
  error: number;
  warning: number;
  info: number;
}

export interface DiagnosticReport {
  issues: DiagnosticIssue[];
  summary: DiagnosticSummary;
  /** 报告生成时间（ms epoch） */
  generated_at: number;
  /** 规则诊断耗时（毫秒） */
  duration_ms: number;
}

export const NODE_CATEGORIES = [
  { id: "trigger", labelKey: "workflow.categories.trigger", color: "#722ed1" },
  {
    id: "execution",
    labelKey: "workflow.categories.execution",
    color: "#52c41a",
  },
  { id: "agent", labelKey: "workflow.categories.agent", color: "#1890ff" },
  { id: "llm", labelKey: "workflow.categories.llm", color: "#13c2c2" },
  { id: "flow", labelKey: "workflow.categories.flow", color: "#fa8c16" },
  {
    id: "integration",
    labelKey: "workflow.categories.integration",
    color: "#eb2f96",
  },
] as const;

export const NODE_TYPE_MAP: Record<
  string,
  { labelKey: string; category: string; color: string }
> = {
  trigger: {
    labelKey: "workflow.nodeTypes.trigger",
    category: "trigger",
    color: "#722ed1",
  },
  agent: {
    labelKey: "workflow.nodeTypes.agent",
    category: "agent",
    color: "#1890ff",
  },
  llm: {
    labelKey: "workflow.nodeTypes.llm",
    category: "llm",
    color: "#13c2c2",
  },
  condition: {
    labelKey: "workflow.nodeTypes.condition",
    category: "flow",
    color: "#fa8c16",
  },
  parallel: {
    labelKey: "workflow.nodeTypes.parallel",
    category: "flow",
    color: "#fa8c16",
  },
  loop: {
    labelKey: "workflow.nodeTypes.loop",
    category: "flow",
    color: "#fa8c16",
  },
  validation: {
    labelKey: "workflow.nodeTypes.validation",
    category: "flow",
    color: "#722ed1",
  },
  merge: {
    labelKey: "workflow.nodeTypes.merge",
    category: "flow",
    color: "#fa8c16",
  },
  delay: {
    labelKey: "workflow.nodeTypes.delay",
    category: "flow",
    color: "#fa8c16",
  },
  subWorkflow: {
    labelKey: "workflow.nodeTypes.subWorkflow",
    category: "integration",
    color: "#eb2f96",
  },
  workflowRef: {
    labelKey: "workflow.nodeTypes.workflowRef",
    category: "integration",
    color: "#eb2f96",
  },
  documentParser: {
    labelKey: "workflow.nodeTypes.documentParser",
    category: "integration",
    color: "#eb2f96",
  },
  vectorRetrieve: {
    labelKey: "workflow.nodeTypes.vectorRetrieve",
    category: "integration",
    color: "#eb2f96",
  },
  httpRequest: {
    labelKey: "workflow.nodeTypes.httpRequest",
    category: "integration",
    color: "#eb2f96",
  },
  debate: {
    labelKey: "workflow.nodeTypes.debate",
    category: "flow",
    color: "#1890ff",
  },
  end: {
    labelKey: "workflow.nodeTypes.end",
    category: "flow",
    color: "#fa8c16",
  },
  tool: {
    labelKey: "workflow.nodeTypes.tool",
    category: "execution",
    color: "#52c41a",
  },
  code: {
    labelKey: "workflow.nodeTypes.code",
    category: "execution",
    color: "#52c41a",
  },
  switch: {
    labelKey: "workflow.nodeTypes.switch",
    category: "flow",
    color: "#fa8c16",
  },
  databaseQuery: {
    labelKey: "workflow.nodeTypes.databaseQuery",
    category: "integration",
    color: "#eb2f96",
  },
  notification: {
    labelKey: "workflow.nodeTypes.notification",
    category: "integration",
    color: "#eb2f96",
  },
  approval: {
    labelKey: "workflow.nodeTypes.approval",
    category: "flow",
    color: "#722ed1",
  },
  fileOperation: {
    labelKey: "workflow.nodeTypes.fileOperation",
    category: "execution",
    color: "#52c41a",
  },
  dataTransformer: {
    labelKey: "workflow.nodeTypes.dataTransformer",
    category: "execution",
    color: "#52c41a",
  },
  webhookSend: {
    labelKey: "workflow.nodeTypes.webhookSend",
    category: "integration",
    color: "#eb2f96",
  },
  logging: {
    labelKey: "workflow.nodeTypes.logging",
    category: "flow",
    color: "#fa8c16",
  },
  llmClassifier: {
    labelKey: "workflow.nodeTypes.llmClassifier",
    category: "llm",
    color: "#13c2c2",
  },
  aggregator: {
    labelKey: "workflow.nodeTypes.aggregator",
    category: "execution",
    color: "#52c41a",
  },
  email: {
    labelKey: "workflow.nodeTypes.email",
    category: "integration",
    color: "#eb2f96",
  },
};

export interface SkillMatchResult {
  existing_skill: { id: string; name: string };
  similarity_score: number;
  match_reasons: string[];
}

export interface NodeSkillMatch {
  node_id: string | null;
  skill_name: string;
  matches: SkillMatchResult[];
}

export interface SemanticCheckResult {
  matches: NodeSkillMatch[];
}

export type SkillReplacementAction = "replace" | "keep" | "upgrade_existing";

export interface SkillUpgradeSuggestion {
  name: string;
  description: string;
  input_schema: Record<string, unknown> | null;
  output_schema: Record<string, unknown> | null;
  reasoning: string;
}

export interface SkillUpgradeRequest {
  existing_skill_id: string;
  generated_name: string;
  generated_description: string;
  generated_input_schema: Record<string, unknown> | null;
  generated_output_schema: Record<string, unknown> | null;
}

export interface ToolInfo {
  tool_name: string;
  tool_type: string;
  description: string;
}

export interface ToolMatchResult {
  tool_name: string;
  tool_type: string;
  description: string;
  similarity_score: number;
  match_reasons: string[];
}

export interface NodeToolMatch {
  node_id: string | null;
  tool_name: string;
  matches: ToolMatchResult[];
}

export interface ToolSemanticCheckResult {
  matches: NodeToolMatch[];
}

export type ToolReplacementAction = "replace" | "keep" | "upgrade_existing";

export interface ToolUpgradeSuggestion {
  name: string;
  description: string;
  input_schema: Record<string, unknown> | null;
  output_schema: Record<string, unknown> | null;
  reasoning: string;
}

export interface ToolUpgradeRequest {
  existing_tool_name: string;
  existing_tool_description: string;
  existing_tool_type: string;
  existing_input_schema: Record<string, unknown> | null;
  existing_output_schema: Record<string, unknown> | null;
  generated_name: string;
  generated_description: string;
  generated_input_schema: Record<string, unknown> | null;
  generated_output_schema: Record<string, unknown> | null;
}

/**
 * AI 聊天面板产出的工作流变更动作。
 * 与后端 workflow_ai_chat_stream 系统 prompt 中的 :::action 块对应，
 * 前端以 discriminated union 解析，applyAiChatAction 按 action_type 分发。
 */
export type AiChatAction =
  | { action_type: "generate_workflow"; data: { nodes: WorkflowNode[]; edges: WorkflowEdge[] } }
  | { action_type: "add_node"; data: { node: WorkflowNode; position?: { x: number; y: number } } }
  | { action_type: "add_nodes"; data: { nodes: WorkflowNode[] } }
  | { action_type: "update_node"; data: { node_id: string; changes: Partial<WorkflowNode> } }
  | { action_type: "modify_node"; data: { node_id: string; changes: Record<string, unknown> } }
  | { action_type: "delete_node"; data: { node_id: string } }
  | { action_type: "delete_nodes"; data: { node_ids: string[] } }
  | { action_type: "add_edge"; data: { edge: WorkflowEdge } }
  | { action_type: "update_edge"; data: { edge_id: string; changes: Partial<WorkflowEdge> } }
  | { action_type: "delete_edge"; data: { edge_id: string } }
  | { action_type: "optimize_prompt"; data: { node_id: string; optimized_prompt: string } };

/** AiChatAction 的 action_type 联合类型（用于 switch 穷尽性检查） */
export type AiChatActionType = AiChatAction["action_type"];

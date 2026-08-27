// SPDX-License-Identifier: AGPL-3.0-only

export interface Position {
  x: number;
  y: number;
}

export interface RetryConfig {
  enabled: boolean;
  maxRetries: number;
  backoffType: "Linear" | "Exponential" | "Fixed";
  baseDelayMs: number;
  maxDelayMs: number;
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
  enumValues?: unknown[];
  format?: string;
}

export interface Variable {
  name: string;
  varType: string;
  value: unknown;
  description?: string;
  isSecret: boolean;
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
  /** 熔断器配置（所有节点类型可选支持） */
  circuitBreaker?: {
    failureThreshold: number;
    resetTimeoutMs: number;
  };
  /** 调试断点标记 */
  _breakpoint?: boolean;
}

export type TriggerType = "manual" | "schedule" | "webhook" | "event";

export interface TriggerConfig {
  type: TriggerType;
  config: unknown;
}

export type ManualTriggerConfig = Record<string, never>;

export interface ScheduleTriggerConfig {
  cron: string;
  schedules?: Record<string, string>;
  timezone: string;
  enabled: boolean;
  input_params?: unknown;
}

export interface WebhookTriggerConfig {
  path: string;
  method: string;
  authType: string;
}

export interface EventTriggerConfig {
  eventType: string;
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
  systemPrompt: string;
  promptTemplateId?: string;
  contextSources: string[];
  outputVar: string;
  model?: string;
  temperature?: number;
  maxTokens?: number;
  /** 工具列表，支持旧格式 `string[]` 和新格式 `ToolDef[]` */
  tools: ToolDef[];
  /** 暴露给 LLM 的工具名列表（tools 的子集）。空数组 = 暴露全部（向后兼容） */
  exposedTools: string[];
  outputMode: OutputMode;
  /** 工具调用最大轮数（默认 5，仅 tools 非空时生效） */
  maxToolRounds?: number;
  /** 执行模式: "react" = 逐步思考-行动, "plan" = 先规划为工作流再执行 */
  executionMode?: "react" | "plan";
  /** RAG 知识源 ID 列表。格式: "knowledge:<kb_id>", "memory:<ns_id>", "wiki:<wiki_id>" */
  ragSourceIds?: string[];
  modelRole?: "quick_think" | "deep_think";
  /**
   * 3.7 P2:任务场景 — 控制 Agent 节点的输出风格指令。
   * - `general`:无特殊约束(默认)
   * - `code`:强调直接给代码、少废话
   * - `research`:强调结构化分析、引用、权衡
   * - `auto`:由 `TaskScene::infer(input)` 自动推断
   * 缺省 `undefined` 时按 `general` 处理。
   */
  taskScene?: "general" | "code" | "research" | "auto";
}

export interface AgentNode extends WorkflowNodeBase {
  type: "agent";
  config: AgentNodeConfig;
}

export interface MultiAgentNodeConfig {
  task: string;
  role?: string;
  model?: string;
  outputVar: string;
  mode: "auto" | "swarm" | "debate";
  maxRounds: number;
}

export interface MultiAgentNode extends WorkflowNodeBase {
  type: "multiAgent";
  config: MultiAgentNodeConfig;
}

export interface LLMNodeConfig {
  model: string;
  prompt: string;
  promptTemplateId?: string;
  messages?: unknown[];
  temperature?: number;
  maxTokens?: number;
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
  varPath: string;
  operator: CompareOperator;
  value: unknown;
}

export interface ConditionNodeConfig {
  conditions: Condition[];
  logicalOp: LogicalOperator;
  /** 启用 LLM 动态路由：由 AI 判断走哪条分支（忽略 conditions 静态规则） */
  judgeByLlm?: boolean;
  /** LLM 路由时的提示词（描述路由判断逻辑） */
  routingPrompt?: string;
  /** LLM 路由使用模型（为空则用系统默认） */
  routingModel?: string;
}

export interface ConditionNode extends WorkflowNodeBase {
  type: "condition";
  config: ConditionNodeConfig;
}

/** 超时降级策略 */
export type DegradeStrategy = "skip" | "useDefault" | "strict";

/** 超时降级策略的中文/可读标签 */
export const DEGRADE_LABELS: Record<DegradeStrategy, string> = {
  skip: "Skip",
  useDefault: "Use Default",
  strict: "Strict",
};

export interface Branch {
  id: string;
  title: string;
  steps: string[];
  /** 分支级别超时（毫秒）。留空则继承节点级别或全局超时。 */
  branchTimeoutMs?: number;
  /** 超时后的降级策略。默认 "skip"。 */
  degradeStrategy?: DegradeStrategy;
}

export type MergeStrategy = "all" | "any" | "race" | "majority";

/** 子图定义：嵌入在容器节点中的独立工作流 */
export interface SubGraph {
  nodes: WorkflowNode[];
  edges: WorkflowEdge[];
}

export interface ParallelNodeConfig {
  branches: Branch[];
  waitForAll: boolean;
  timeout?: number;
  aggregation?: MergeStrategy;
  autoInputFromParent?: boolean;
  /**
   * 容器角色标记。
   *
   * - `"executable"`（默认）：真并行调度器。`waitForAll` + `aggregation` 实际生效，
   *   运行时引擎并行执行子分支。
   * - `"decorative"`：装饰性分组。仅供前端画分组框，调度引擎忽略。
   *   成员通过 `parentId` 引用，实际依赖通过显式的 `edge` 表达。
   */
  kind?: "decorative" | "executable";
  /** 子图定义（可选）。编辑器渲染为可展开/折叠容器框体，内部渲染子节点网格。 */
  subGraph?: SubGraph;
}

export interface ParallelNode extends WorkflowNodeBase {
  type: "parallel";
  config: ParallelNodeConfig;
}

export type LoopType = "forEach" | "while" | "doWhile" | "until";

export interface LoopNodeConfig {
  loopType: LoopType;
  itemsVar?: string;
  iterateeVar?: string;
  maxIterations?: number;
  continueCondition?: string;
  continueOnError: boolean;
  bodySteps: string[];
  /** 子图定义（可选）。编辑器渲染为可展开/折叠容器框体，内部渲染子节点网格。 */
  subGraph?: SubGraph;
}

export interface LoopNode extends WorkflowNodeBase {
  type: "loop";
  config: LoopNodeConfig;
}

export interface MergeNodeConfig {
  mergeType: MergeStrategy;
  inputs: string[];
  autoInputsFromBranches?: boolean;
}

export interface MergeNode extends WorkflowNodeBase {
  type: "merge";
  config: MergeNodeConfig;
}

export interface DelayNodeConfig {
  delayType: string;
  seconds: number;
  until?: string;
}

export interface DelayNode extends WorkflowNodeBase {
  type: "delay";
  config: DelayNodeConfig;
}

export interface ToolNodeConfig {
  toolName: string;
  inputMapping: Record<string, string>;
  outputVar: string;
}

export interface ToolNode extends WorkflowNodeBase {
  type: "tool";
  config: ToolNodeConfig;
}

export interface CodeNodeConfig {
  language: string;
  code: string;
  outputVar: string;
  /** Rhai 脚本注册为工具名（language="rhai" 时生效） */
  toolName?: string;
}

export interface CodeNode extends WorkflowNodeBase {
  type: "code";
  config: CodeNodeConfig;
}

export interface SubWorkflowNodeConfig {
  subWorkflowId: string;
  inputMapping: Record<string, string>;
  outputVar: string;
  isAsync: boolean;
  /** 子图定义（可选）。与 expandedSubWorkflows 配合，编辑器可在容器内部渲染子工作流节点。 */
  subGraph?: SubGraph;
}

export interface SubWorkflowNode extends WorkflowNodeBase {
  type: "subWorkflow";
  config: SubWorkflowNodeConfig;
}

/** 工作流引用配置：引用另一个工作流作为子流程执行 */
export interface WorkflowRefNodeConfig {
  /** 被引用的工作流模板 ID */
  targetWorkflowId: string;
  /** 参数注入映射：当前上下文变量名 → 子工作流入参名 */
  inputMapping: Record<string, string>;
  /** 子工作流输出变量名 */
  outputVar: string;
  /** 超时继承：不设置则使用当前工作流默认超时 */
  timeout?: number;
  /** 上下文传递模式 */
  contextMode?: "inherit" | "isolated";
}

export interface WorkflowRefNode extends WorkflowNodeBase {
  type: "workflowRef";
  config: WorkflowRefNodeConfig;
}

export interface DocumentParserNodeConfig {
  inputVar: string;
  parserType: string;
  outputVar: string;
}

export interface DocumentParserNode extends WorkflowNodeBase {
  type: "documentParser";
  config: DocumentParserNodeConfig;
}

export interface VectorRetrieveNodeConfig {
  query: string;
  knowledgeBaseId: string;
  topK: number;
  similarityThreshold?: number;
  outputVar: string;
}

export interface VectorRetrieveNode extends WorkflowNodeBase {
  type: "vectorRetrieve";
  config: VectorRetrieveNodeConfig;
}

export interface EndNodeConfig {
  outputVar?: string;
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
  onFail: "stop" | "retry" | "continue";
  maxRetries: number;
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
  bodyType: string;
  timeoutSecs: number;
  outputVar: string;
}

export interface HttpRequestNode extends WorkflowNodeBase {
  type: "httpRequest";
  config: HttpRequestNodeConfig;
}

/** Switch 匹配模式 */
export type SwitchMatchMode = "exact" | "regex" | "contains" | "expression";

export interface SwitchCase {
  value: string;
  label: string;
}

export interface SwitchNodeConfig {
  inputVar: string;
  cases: SwitchCase[];
  defaultCase?: string;
  matchMode: SwitchMatchMode;
  /** 使用 LLM 进行智能路由（替代 matchMode 的值匹配） */
  useLlm?: boolean;
  /** LLM 路由的自定义提示词 */
  llmPrompt?: string;
  /** 路由使用的模型 */
  llmModel?: string;
  outputVar: string;
}

export interface SwitchNode extends WorkflowNodeBase {
  type: "switch";
  config: SwitchNodeConfig;
}

export interface DatabaseQueryNodeConfig {
  query: string;
  params: string[];
  connectionName?: string;
  timeoutSecs: number;
  outputVar: string;
}

export interface DatabaseQueryNode extends WorkflowNodeBase {
  type: "databaseQuery";
  config: DatabaseQueryNodeConfig;
}

export interface NotificationNodeConfig {
  channel: string;
  message: string;
  webhookUrl?: string;
  recipients: string[];
  subject?: string;
  enabled: boolean;
  outputVar: string;
}
export interface NotificationNode extends WorkflowNodeBase {
  type: "notification";
  config: NotificationNodeConfig;
}

export interface ApprovalNodeConfig {
  message: string;
  approver?: string;
  timeoutSecs: number;
  timeoutAction: string;
  outputVar: string;
}
export interface ApprovalNode extends WorkflowNodeBase {
  type: "approval";
  config: ApprovalNodeConfig;
}

export interface FileOperationNodeConfig {
  operation: string;
  filePath: string;
  content?: string;
  outputVar: string;
}
export interface FileOperationNode extends WorkflowNodeBase {
  type: "fileOperation";
  config: FileOperationNodeConfig;
}

export interface DataTransformerNodeConfig {
  inputVar: string;
  expression: string;
  outputVar: string;
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
  outputVar: string;
}
export interface WebhookSendNode extends WorkflowNodeBase {
  type: "webhookSend";
  config: WebhookSendNodeConfig;
}

export interface LoggingNodeConfig {
  level: string;
  message: string;
  outputVar: string;
}
export interface LoggingNode extends WorkflowNodeBase {
  type: "logging";
  config: LoggingNodeConfig;
}

/** 存储持久化节点配置 */
export interface StorageNodeConfig {
  /** 存储后端："sqlite" | "vectorDb" | "fileSystem" */
  backend: string;
  /** 操作模式："insert" | "upsert" | "append" */
  operation: string;
  /** 要存储的数据的变量路径 */
  inputVar: string;
  /** 存储目标（SQLite 表名 / VectorDB collection / 文件路径） */
  collection: string;
  /** upsert 时用于匹配已有记录的 key 变量路径 */
  keyVar?: string;
  outputVar: string;
}

export interface StorageNode extends WorkflowNodeBase {
  type: "storage";
  config: StorageNodeConfig;
}

export interface LlmClassifierNodeConfig {
  categories: string[];
  /** 动态分类目录注入口：从工作流 variables 读取类别列表的变量名，优先于静态 categories */
  categoriesVar?: string;
  prompt: string;
  model?: string;
  inputVar: string;
  outputVar: string;
  /** 置信度阈值（0.0-1.0）：LLM 返回置信度低于阈值时使用 fallbackLabel 降级 */
  confidenceThreshold?: number;
  /** 置信度不足时的降级标签（可选） */
  fallbackLabel?: string;
}
export interface LlmClassifierNode extends WorkflowNodeBase {
  type: "llmClassifier";
  config: LlmClassifierNodeConfig;
}

export interface AggregatorNodeConfig {
  strategy: string;
  inputSources: string[];
  /** 等待策略：true=等待所有输入就绪再聚合；false=有输入即聚合 */
  waitForAll?: boolean;
  /** 加权策略的权重系数（与 inputSources 一一对应） */
  weights?: number[];
  /** llm_summarize 策略的自定义提示词 */
  summarizePrompt?: string;
  /** llm_summarize 策略的模型 */
  summarizeModel?: string;
  outputVar: string;
  /** 子图定义（可选）。编辑器渲染为可展开/折叠容器框体，内部渲染子节点网格。 */
  subGraph?: SubGraph;
}
export interface AggregatorNode extends WorkflowNodeBase {
  type: "aggregator";
  config: AggregatorNodeConfig;
}

export interface EmailNodeConfig {
  to: string[];
  subject: string;
  body: string;
  smtpHost?: string;
  smtpPort?: number;
  smtpUser?: string;
  smtpPass?: string;
  outputVar: string;
}
export interface EmailNode extends WorkflowNodeBase {
  type: "email";
  config: EmailNodeConfig;
}

export interface DebateNodeConfig {
  debaterSteps: string[];
  maxRounds: number;
  convergencePrompt?: string;
  convergenceModel?: string;
  convergenceModelRole?: string;
  topicVar: string;
  outputVar: string;
  /** 子图定义（可选）。编辑器渲染为可展开/折叠容器框体，内部渲染子节点网格。 */
  subGraph?: SubGraph;
}

export interface DebateNode extends WorkflowNodeBase {
  type: "debate";
  config: DebateNodeConfig;
}

/** Swarm 节点配置：多 Agent 协作模式 */
export interface SwarmNodeConfig {
  /** 参与者节点 ID 列表 */
  agentSteps: string[];
  /** 最大协作轮数 */
  maxRounds: number;
  /** 收敛判断提示文本 */
  convergencePrompt?: string;
  /** 收敛判断模型 */
  convergenceModel?: string;
  /** 讨论主题变量 */
  topicVar: string;
  /** 输出变量名 */
  outputVar: string;
  /** 子图定义（可选） */
  subGraph?: SubGraph;
}

export interface SwarmNode extends WorkflowNodeBase {
  type: "swarm";
  config: SwarmNodeConfig;
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
  | DebateNode
  | SwarmNode
  | MultiAgentNode
  | StorageNode;

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
  edgeType: EdgeType;
  label?: string;
}

export type OnFailureAction =
  | "abort"
  | "retryThenAbort"
  | "runErrorBranch"
  | "continueWithDefault";

export interface RetryPolicy {
  maxRetries: number;
  baseDelayMs: number;
  maxDelayMs: number;
}

export interface CompensationStep {
  stepId: string;
  compensateType: string;
  targetStep: string;
}

export interface ErrorConfig {
  retryPolicy?: RetryPolicy;
  onFailure: OnFailureAction;
  errorBranch?: string[];
  compensationSteps?: CompensationStep[];
}

export interface RhaiToolDef {
  toolName: string;
  description?: string;
  code: string;
}

export interface WorkflowTemplateInput {
  name: string;
  description?: string;
  icon: string;
  tags: string[];
  triggerConfig?: TriggerConfig;
  nodes: WorkflowNode[];
  edges: WorkflowEdge[];
  inputSchema?: JsonSchema;
  outputSchema?: JsonSchema;
  variables: Variable[];
  errorConfig?: ErrorConfig;
  toolDefs?: RhaiToolDef[];
  /** L2 集群 ID（三层路由第二层，命令层可从 tags 推导） */
  clusterId?: string;
  /** 三层路由路径（命令层可从 tags 推导，格式 /{domain}/{cluster}/{capability}） */
  routePath?: string;
}

export interface WorkflowTemplateResponse {
  id: string;
  name: string;
  description?: string;
  icon: string;
  tags: string[];
  version: number;
  isPreset: boolean;
  isEditable: boolean;
  isPublic: boolean;
  /** 是否为系统模板（认知编排器等），由后端按 isPreset + cognitiveRouter 标签权威判定 */
  isSystem: boolean;
  triggerConfig?: TriggerConfig;
  nodes: WorkflowNode[];
  edges: WorkflowEdge[];
  inputSchema?: JsonSchema;
  outputSchema?: JsonSchema;
  variables: Variable[];
  errorConfig?: ErrorConfig;
  toolDefs?: RhaiToolDef[];
  missionHash?: string;
  /** L2 集群 ID（三层路由第二层） */
  clusterId?: string;
  /** 三层路由路径（格式 /{domain}/{cluster}/{capability}） */
  routePath?: string;
  createdAt: number;
  updatedAt: number;
}

export interface TemplateFilter {
  isPreset?: boolean;
  tags?: string[];
  search?: string;
}

export interface ValidationError {
  errorType: string;
  nodeId?: string;
  message: string;
  suggestion?: string;
}

export interface ValidationWarning {
  warningType: string;
  nodeId?: string;
  message: string;
}

export interface ValidationResult {
  isValid: boolean;
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
 * - remove_debater_step: 移除辩手子节点
 * - update_variable:     改 workflow_template.variables
 * - update_input_mapping: 改 sub-workflow 节点 input_mapping
 * - edit_asset_file:     LSP 风格锚点编辑任意文本文件
 * - rollback_to_version: 回滚到指定版本
 */
export type EditAssetOperation = "insert_after" | "replace" | "delete";

/** 验证规格 — type 由调用方定义(已知 hook: "backtest") */
export interface ValidationSpec {
  type: string;
  params?: Record<string, unknown>;
}

/** `apply_edit_asset_file` 命令的返回结果(后端 `EditAssetFileResult`) */
export interface EditAssetFileResult {
  /** 改动后的完整文件内容 */
  newContent: string;
  /** 简单的 unified diff(无颜色,前端可展示) */
  diff: string;
  /** 改动的起始行号(1-based,用于前端高亮) */
  changedStartLine: number;
  /** 改动的结束行号(1-based,包含) */
  changedEndLine: number;
}

/** `apply_diff_with_validation` 命令的返回结果(后端 `ApplyDiffValidationResult`) */
export interface ApplyDiffValidationResult {
  /** 验证是否通过 */
  validationPassed: boolean;
  /** 实际应用的 action 数(部分失败时可能 < inputs) */
  appliedCount: number;
  /** 已应用的 action 列表(action_type 字符串) */
  applied: string[];
  /** 验证指标(由 validation hook 填) */
  validationMetrics: Record<string, unknown>;
  /** 是否发生了回滚 */
  rolledBack: boolean;
  /** 错误信息(任一 action 失败时) */
  error: string | null;
}

/** `apply_diagnostic_fixes` 命令的返回结果(后端 `ApplyDiagnosticFixesResult`) */
export interface ApplyDiagnosticFixesResult {
  /** 接收的 fix 总数(去重前) */
  received: number;
  /** 去重后的 fix 数(实际进入调度器的) */
  deduped: number;
  /** 调度器结果:validationPassed(true/false/none) */
  validationPassed: boolean;
  /** 已应用的 action 列表(action_type 字符串) */
  applied: string[];
  /** 是否发生了回滚 */
  rolledBack: boolean;
  /** 错误信息(任一 action 失败时) */
  error: string | null;
}

export type DiagnosticFix =
  | {
    actionType: "set_node_field";
    nodeId: string;
    field: string;
    value: unknown;
  }
  | { actionType: "delete_node"; nodeId: string }
  | { actionType: "delete_edge"; edgeId: string }
  | { actionType: "enable_retry"; nodeId: string; maxRetries: number }
  | { actionType: "set_timeout"; nodeId: string; timeoutMs: number }
  | { actionType: "remove_debater_step"; nodeId: string; stepId: string }
  | {
    actionType: "update_variable";
    templateId: string;
    name: string;
    value: unknown;
  }
  | {
    actionType: "update_input_mapping";
    nodeId: string;
    mappings: Array<{ target: string; source: string }>;
  }
  | {
    actionType: "edit_asset_file";
    path: string;
    operation: EditAssetOperation;
    anchorLine: number;
    /** insert_after / replace 时必填;delete 时可省 */
    code?: string;
    description: string;
  }
  | { actionType: "rollback_to_version"; templateId: string; version: number };

export interface DiagnosticIssue {
  id: string;
  severity: DiagnosticSeverity;
  category: DiagnosticCategory;
  titleKey: string;
  messageKey: string;
  messageParams?: Record<string, string | number>;
  nodeIds: string[];
  edgeIds?: string[];
  autoFixable: boolean;
  fix?: DiagnosticFix;
  titleOverride?: string;
  detailOverride?: string;
  suggestionOverride?: string;
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
  generatedAt: number;
  /** 规则诊断耗时（毫秒） */
  durationMs: number;
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

/**
 * 节点语义分类 — 由引擎层定义，确定节点的固定颜色。
 *
 * 编辑器根据此分类从主题 token 读取颜色，支持深色/浅色模式自动适配。
 * 禁止在工作流设计层面随意指定节点颜色。
 *
 * Kind 命名遵循下表（与 `NodeKindLabel` 一一对应）：
 *
 * - `input`     → 输入/触发类（黄）
 * - `output`    → 输出/结束类（红）
 * - `tool`      → 工具/执行类（绿）
 * - `agent`     → Agent/LLM 推理类（蓝）
 * - `condition` → 条件分支/路由（橙）
 * - `loop`      → 循环控制（紫）
 * - `container` → 容器/并行/辩论（青）
 * - `storage`   → 存储/检索（粉）
 */
export type NodeKind = "input" | "output" | "tool" | "agent" | "condition" | "loop" | "container" | "storage";

/**
 * Kind → i18n key 映射表。消费方用 `useTranslation().t(getNodeKindLabelKey(kind))`
 * 解析为本地化字符串。
 */
export const NODE_KIND_LABEL_KEYS: Record<NodeKind, string> = {
  input: "workflow.nodeKind.input",
  output: "workflow.nodeKind.output",
  tool: "workflow.nodeKind.tool",
  agent: "workflow.nodeKind.agent",
  condition: "workflow.nodeKind.condition",
  loop: "workflow.nodeKind.loop",
  container: "workflow.nodeKind.container",
  storage: "workflow.nodeKind.storage",
};

/** 兼容性别名：旧的 `NODE_KIND_LABELS` 已被替换为 i18n 键映射，请使用 `NODE_KIND_LABEL_KEYS`。 */
export const NODE_KIND_LABELS = NODE_KIND_LABEL_KEYS;

/**
 * 每个节点类型所属的语义分类。
 * === 颜色规范（引擎定义，不可在设计层覆盖） ===
 * Input=黄  Output=红  Tool=绿  Agent=蓝
 * Condition=橙  Loop=紫  Container=青  Storage=粉
 */
export const NODE_KIND_MAP: Record<string, NodeKind> = {
  // Input（触发/输入）
  trigger: "input",
  // Output（输出/结束/通知）
  end: "output",
  notification: "output",
  approval: "output",
  email: "output",
  webhookSend: "output",
  // Tool（工具/执行）
  tool: "tool",
  code: "tool",
  delay: "tool",
  validation: "tool",
  documentParser: "tool",
  httpRequest: "tool",
  fileOperation: "tool",
  dataTransformer: "tool",
  logging: "tool",
  // Agent（AI 推理）
  agent: "agent",
  llm: "agent",
  llmClassifier: "agent",
  multiAgent: "agent",
  // Condition（条件路由）
  condition: "condition",
  switch: "condition",
  // Loop（循环）
  loop: "loop",
  // Container（容器/并行/辩论/聚合）
  parallel: "container",
  debate: "container",
  swarm: "container",
  subWorkflow: "container",
  workflowRef: "container",
  aggregator: "container",
  merge: "container",
  // Storage（存储/检索）
  vectorRetrieve: "storage",
  storage: "storage",
  databaseQuery: "storage",
  // Decorative / Separator（装饰/分隔，归类为 tool 沿用绿色）
  _phaseSeparator: "tool",
  groupFrame: "tool",
};

export const NODE_TYPE_MAP: Record<
  string,
  { labelKey: string; category: string; color: string; isContainer?: boolean; kind?: NodeKind }
> = {
  // ── Input 黄 (#fadb14) ─────────────────────────────────
  trigger: {
    labelKey: "workflow.nodeTypes.trigger",
    category: "trigger",
    color: "#fadb14",
    kind: "input",
  },
  // ── Agent 蓝 (#1677ff) ─────────────────────────────────
  agent: {
    labelKey: "workflow.nodeTypes.agent",
    category: "agent",
    color: "#1677ff",
    kind: "agent",
  },
  llm: {
    labelKey: "workflow.nodeTypes.llm",
    category: "llm",
    color: "#1677ff",
    kind: "agent",
  },
  llmClassifier: {
    labelKey: "workflow.nodeTypes.llmClassifier",
    category: "llm",
    color: "#1677ff",
    kind: "agent",
  },
  multiAgent: {
    labelKey: "workflow.nodeTypes.multiAgent",
    category: "agent",
    color: "#1677ff",
    kind: "agent",
  },
  // ── Condition 橙 (#fa8c16) ─────────────────────────────
  condition: {
    labelKey: "workflow.nodeTypes.condition",
    category: "flow",
    color: "#fa8c16",
    kind: "condition",
  },
  switch: {
    labelKey: "workflow.nodeTypes.switch",
    category: "flow",
    color: "#fa8c16",
    kind: "condition",
  },
  // ── Loop 紫 (#722ed1) ──────────────────────────────────
  loop: {
    labelKey: "workflow.nodeTypes.loop",
    category: "flow",
    color: "#722ed1",
    isContainer: true,
    kind: "loop",
  },
  // ── Container 青 (#13c2c2) ──────────────────────────────
  parallel: {
    labelKey: "workflow.nodeTypes.parallel",
    category: "flow",
    color: "#13c2c2",
    isContainer: true,
    kind: "container",
  },
  debate: {
    labelKey: "workflow.nodeTypes.debate",
    category: "flow",
    color: "#13c2c2",
    isContainer: true,
    kind: "container",
  },
  swarm: {
    labelKey: "workflow.nodeTypes.swarm",
    category: "flow",
    color: "#13c2c2",
    isContainer: true,
    kind: "container",
  },
  subWorkflow: {
    labelKey: "workflow.nodeTypes.subWorkflow",
    category: "integration",
    color: "#13c2c2",
    isContainer: true,
    kind: "container",
  },
  workflowRef: {
    labelKey: "workflow.nodeTypes.workflowRef",
    category: "integration",
    color: "#13c2c2",
    isContainer: true,
    kind: "container",
  },
  aggregator: {
    labelKey: "workflow.nodeTypes.aggregator",
    category: "execution",
    color: "#13c2c2",
    isContainer: true,
    kind: "container",
  },
  merge: {
    labelKey: "workflow.nodeTypes.merge",
    category: "flow",
    color: "#13c2c2",
    isContainer: true,
    kind: "container",
  },
  // ── Tool 绿 (#52c41a) ──────────────────────────────────
  tool: {
    labelKey: "workflow.nodeTypes.tool",
    category: "execution",
    color: "#52c41a",
    kind: "tool",
  },
  code: {
    labelKey: "workflow.nodeTypes.code",
    category: "execution",
    color: "#52c41a",
    kind: "tool",
  },
  delay: {
    labelKey: "workflow.nodeTypes.delay",
    category: "flow",
    color: "#52c41a",
    kind: "tool",
  },
  validation: {
    labelKey: "workflow.nodeTypes.validation",
    category: "flow",
    color: "#52c41a",
    kind: "tool",
  },
  documentParser: {
    labelKey: "workflow.nodeTypes.documentParser",
    category: "integration",
    color: "#52c41a",
    kind: "tool",
  },
  httpRequest: {
    labelKey: "workflow.nodeTypes.httpRequest",
    category: "integration",
    color: "#52c41a",
    kind: "tool",
  },
  fileOperation: {
    labelKey: "workflow.nodeTypes.fileOperation",
    category: "execution",
    color: "#52c41a",
    kind: "tool",
  },
  dataTransformer: {
    labelKey: "workflow.nodeTypes.dataTransformer",
    category: "execution",
    color: "#52c41a",
    kind: "tool",
  },
  logging: {
    labelKey: "workflow.nodeTypes.logging",
    category: "flow",
    color: "#52c41a",
    kind: "tool",
  },
  // ── Output 红 (#f5222d) ─────────────────────────────────
  end: {
    labelKey: "workflow.nodeTypes.end",
    category: "flow",
    color: "#f5222d",
    kind: "output",
  },
  notification: {
    labelKey: "workflow.nodeTypes.notification",
    category: "integration",
    color: "#f5222d",
    kind: "output",
  },
  approval: {
    labelKey: "workflow.nodeTypes.approval",
    category: "flow",
    color: "#f5222d",
    kind: "output",
  },
  email: {
    labelKey: "workflow.nodeTypes.email",
    category: "integration",
    color: "#f5222d",
    kind: "output",
  },
  webhookSend: {
    labelKey: "workflow.nodeTypes.webhookSend",
    category: "integration",
    color: "#f5222d",
    kind: "output",
  },
  // ── RAG（归在 Agent 分类，与 Agent 节点同组便于发现） ──────
  vectorRetrieve: {
    labelKey: "workflow.nodeTypes.vectorRetrieve",
    category: "agent",
    color: "#eb2f96",
    kind: "storage",
  },
  storage: {
    labelKey: "workflow.nodeTypes.storage",
    category: "integration",
    color: "#eb2f96",
    kind: "storage",
  },
  databaseQuery: {
    labelKey: "workflow.nodeTypes.databaseQuery",
    category: "integration",
    color: "#eb2f96",
    kind: "storage",
  },
  /** 阶段分隔线（不参与执行逻辑） */
  _phaseSeparator: {
    labelKey: "workflow.nodeTypes.phaseSeparator",
    category: "flow",
    color: "#555555",
    kind: "tool",
  },
  groupFrame: {
    labelKey: "workflow.nodeTypes.groupFrame",
    category: "flow",
    color: "#555555",
    kind: "tool",
  },
};

export interface SkillMatchResult {
  existingSkill: { id: string; name: string };
  similarityScore: number;
  matchReasons: string[];
}

export interface NodeSkillMatch {
  nodeId: string | null;
  skillName: string;
  matches: SkillMatchResult[];
}

export interface SemanticCheckResult {
  matches: NodeSkillMatch[];
}

export type SkillReplacementAction = "replace" | "keep" | "upgrade_existing";

export interface SkillUpgradeSuggestion {
  name: string;
  description: string;
  inputSchema: Record<string, unknown> | null;
  outputSchema: Record<string, unknown> | null;
  reasoning: string;
}

export interface SkillUpgradeRequest {
  existingSkillId: string;
  generatedName: string;
  generatedDescription: string;
  generatedInputSchema: Record<string, unknown> | null;
  generatedOutputSchema: Record<string, unknown> | null;
}

export interface ToolInfo {
  toolName: string;
  toolType: string;
  description: string;
}

export interface ToolMatchResult {
  toolName: string;
  toolType: string;
  description: string;
  similarityScore: number;
  matchReasons: string[];
}

export interface NodeToolMatch {
  nodeId: string | null;
  toolName: string;
  matches: ToolMatchResult[];
}

export interface ToolSemanticCheckResult {
  matches: NodeToolMatch[];
}

export type ToolReplacementAction = "replace" | "keep" | "upgrade_existing";

export interface ToolUpgradeSuggestion {
  name: string;
  description: string;
  inputSchema: Record<string, unknown> | null;
  outputSchema: Record<string, unknown> | null;
  reasoning: string;
}

export interface ToolUpgradeRequest {
  existingToolName: string;
  existingToolDescription: string;
  existingToolType: string;
  existingInputSchema: Record<string, unknown> | null;
  existingOutputSchema: Record<string, unknown> | null;
  generatedName: string;
  generatedDescription: string;
  generatedInputSchema: Record<string, unknown> | null;
  generatedOutputSchema: Record<string, unknown> | null;
}

/**
 * AI 聊天面板产出的工作流变更动作。
 * 与后端 workflow_ai_chat_stream 系统 prompt 中的 :::action 块对应，
 * 前端以 discriminated union 解析，applyAiChatAction 按 action_type 分发。
 */
export type AiChatAction =
  | { actionType: "generate_workflow"; data: { nodes: WorkflowNode[]; edges: WorkflowEdge[] } }
  | { actionType: "add_node"; data: { node: WorkflowNode; position?: { x: number; y: number } } }
  | { actionType: "add_nodes"; data: { nodes: WorkflowNode[] } }
  | { actionType: "update_node"; data: { nodeId: string; changes: Partial<WorkflowNode> } }
  | { actionType: "modify_node"; data: { nodeId: string; changes: Record<string, unknown> } }
  | { actionType: "delete_node"; data: { nodeId: string } }
  | { actionType: "delete_nodes"; data: { nodeIds: string[] } }
  | { actionType: "add_edge"; data: { edge: WorkflowEdge } }
  | { actionType: "update_edge"; data: { edgeId: string; changes: Partial<WorkflowEdge> } }
  | { actionType: "delete_edge"; data: { edgeId: string } }
  | { actionType: "optimize_prompt"; data: { nodeId: string; optimizedPrompt: string } }
  // ── v2.0 基础设施类 action(与后端 workflow_ai_protocol::ChatAction 对齐)──
  | { actionType: "update_variable"; data: { templateId: string; name: string; value: unknown } }
  | { actionType: "rollback_to_version"; data: { templateId: string; version: number } }
  | {
    actionType: "update_input_mapping";
    data: {
      nodeId: string;
      mappings: Array<{ target: string; source: string }>;
    };
  }
  | {
    actionType: "edit_asset_file";
    data: {
      path: string;
      operation: EditAssetOperation;
      anchorLine: number;
      /** insert_after / replace 时必填;delete 时可省 */
      code?: string;
      description: string;
    };
  }
  | {
    actionType: "apply_diff_with_validation";
    data: {
      actions: AiChatAction[];
      validation: ValidationSpec;
      rollbackOnFailure?: boolean;
    };
  };

/** AiChatAction 的 actionType 联合类型（用于 switch 穷尽性检查） */
export type AiChatActionType = AiChatAction["actionType"];

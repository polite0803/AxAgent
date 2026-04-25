export interface Position {
  x: number;
  y: number;
}

export interface RetryConfig {
  enabled: boolean;
  max_retries: number;
  backoff_type: 'Linear' | 'Exponential' | 'Fixed';
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
}

export type TriggerType = 'manual' | 'schedule' | 'webhook' | 'event';

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

export type AgentRole = 'researcher' | 'planner' | 'developer' | 'reviewer' | 'synthesizer' | 'executor';

export type OutputMode = 'json' | 'text' | 'artifact';

export interface AgentNodeConfig {
  role: AgentRole;
  system_prompt: string;
  context_sources: string[];
  output_var: string;
  model?: string;
  temperature?: number;
  max_tokens?: number;
  tools: string[];
  output_mode: OutputMode;
}

export interface AgentNode extends WorkflowNodeBase {
  type: 'agent';
  config: AgentNodeConfig;
}

export interface LLMNodeConfig {
  model: string;
  prompt: string;
  messages?: unknown[];
  temperature?: number;
  max_tokens?: number;
  tools?: string[];
  functions?: unknown[];
}

export interface LLMNode extends WorkflowNodeBase {
  type: 'llm';
  config: LLMNodeConfig;
}

export type CompareOperator = 'eq' | 'ne' | 'gt' | 'lt' | 'gte' | 'lte' | 'contains' | 'notContains' | 'startsWith' | 'endsWith' | 'regexMatch' | 'isEmpty' | 'isNotEmpty';

export type LogicalOperator = 'and' | 'or';

export interface Condition {
  var_path: string;
  operator: CompareOperator;
  value: unknown;
}

export interface ConditionNodeConfig {
  conditions: Condition[];
  logical_op: LogicalOperator;
}

export interface ConditionNode extends WorkflowNodeBase {
  type: 'condition';
  config: ConditionNodeConfig;
}

export interface Branch {
  id: string;
  title: string;
  steps: string[];
}

export interface ParallelNodeConfig {
  branches: Branch[];
  wait_for_all: boolean;
  timeout?: number;
}

export interface ParallelNode extends WorkflowNodeBase {
  type: 'parallel';
  config: ParallelNodeConfig;
}

export type LoopType = 'forEach' | 'while' | 'doWhile' | 'until';

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
  type: 'loop';
  config: LoopNodeConfig;
}

export interface MergeNodeConfig {
  merge_type: string;
  inputs: string[];
}

export interface MergeNode extends WorkflowNodeBase {
  type: 'merge';
  config: MergeNodeConfig;
}

export interface DelayNodeConfig {
  delay_type: string;
  seconds: number;
  until?: string;
}

export interface DelayNode extends WorkflowNodeBase {
  type: 'delay';
  config: DelayNodeConfig;
}

export interface ToolNodeConfig {
  tool_name: string;
  input_mapping: Record<string, string>;
  output_var: string;
}

export interface ToolNode extends WorkflowNodeBase {
  type: 'tool';
  config: ToolNodeConfig;
}

export interface CodeNodeConfig {
  language: string;
  code: string;
  output_var: string;
}

export interface CodeNode extends WorkflowNodeBase {
  type: 'code';
  config: CodeNodeConfig;
}

export interface SubWorkflowNodeConfig {
  sub_workflow_id: string;
  input_mapping: Record<string, string>;
  output_var: string;
  is_async: boolean;
}

export interface SubWorkflowNode extends WorkflowNodeBase {
  type: 'subWorkflow';
  config: SubWorkflowNodeConfig;
}

export interface DocumentParserNodeConfig {
  input_var: string;
  parser_type: string;
  output_var: string;
}

export interface DocumentParserNode extends WorkflowNodeBase {
  type: 'documentParser';
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
  type: 'vectorRetrieve';
  config: VectorRetrieveNodeConfig;
}

export interface EndNodeConfig {
  output_var?: string;
}

export interface AtomicSkillNodeConfig {
  skill_id?: string;
  skill_name?: string;
  entry_type?: string;
  input_mapping?: Record<string, string>;
  output_var?: string;
}

export interface AtomicSkillNode extends WorkflowNodeBase {
  type: 'atomicSkill';
  config: AtomicSkillNodeConfig;
}

export interface EndNode extends WorkflowNodeBase {
  type: 'end';
  config: EndNodeConfig;
}

export interface TriggerNode extends WorkflowNodeBase {
  type: 'trigger';
  config: TriggerConfig;
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
  | DocumentParserNode
  | VectorRetrieveNode
  | AtomicSkillNode
  | EndNode;

export type EdgeType = 'direct' | 'conditionTrue' | 'conditionFalse' | 'loopBack' | 'parallelBranch' | 'merge' | 'error';

export interface WorkflowEdge {
  id: string;
  source: string;
  sourceHandle?: string;
  target: string;
  targetHandle?: string;
  edge_type: EdgeType;
  label?: string;
}

export type OnFailureAction = 'abort' | 'retryThenAbort' | 'runErrorBranch' | 'continueWithDefault';

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

export const NODE_CATEGORIES = [
  { id: 'trigger', label: '触发器', color: '#722ed1' },
  { id: 'execution', label: '执行节点', color: '#52c41a' },
  { id: 'agent', label: 'Agent', color: '#1890ff' },
  { id: 'llm', label: 'LLM', color: '#13c2c2' },
  { id: 'flow', label: '流程控制', color: '#fa8c16' },
  { id: 'integration', label: '集成', color: '#eb2f96' },
] as const;

export const NODE_TYPE_MAP: Record<string, { label: string; category: string; color: string }> = {
  trigger: { label: '触发器', category: 'trigger', color: '#722ed1' },
  atomicSkill: { label: '原子Skill', category: 'execution', color: '#52c41a' },
  agent: { label: 'Agent', category: 'agent', color: '#1890ff' },
  llm: { label: 'LLM', category: 'llm', color: '#13c2c2' },
  condition: { label: '条件分支', category: 'flow', color: '#fa8c16' },
  parallel: { label: '并行分支', category: 'flow', color: '#fa8c16' },
  loop: { label: '循环', category: 'flow', color: '#fa8c16' },
  merge: { label: '合并', category: 'flow', color: '#fa8c16' },
  delay: { label: '延迟', category: 'flow', color: '#fa8c16' },
  subWorkflow: { label: '子工作流', category: 'integration', color: '#eb2f96' },
  documentParser: { label: '文档解析', category: 'integration', color: '#eb2f96' },
  vectorRetrieve: { label: '向量检索', category: 'integration', color: '#eb2f96' },
  end: { label: '结束', category: 'flow', color: '#fa8c16' },
  // Legacy types (kept for backward compatibility)
  tool: { label: '工具(旧)', category: 'execution', color: '#52c41a' },
  code: { label: '代码(旧)', category: 'execution', color: '#52c41a' },
};

export interface AtomicSkillInfo {
  id: string;
  name: string;
  description: string;
  entry_type: string;
  entry_ref: string;
  category: string;
  version: string;
}

export interface SkillMatchResult {
  existing_skill: AtomicSkillInfo;
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

export type SkillReplacementAction = 'replace' | 'keep' | 'upgrade_existing';

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

export type ToolReplacementAction = 'replace' | 'keep' | 'upgrade_existing';

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

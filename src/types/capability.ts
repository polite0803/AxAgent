// SPDX-License-Identifier: AGPL-3.0-only
// ! 能力发现系统前端类型定义
//
// 以后端 DTO 为权威来源对齐（见 src-tauri/crates/harness/src/capability*.rs）。
// - 后端枚举使用 `#[serde(rename_all = "snake_case")]`，前端用 snake_case 字面量
// - 后端 struct 字段默认 snake_case（无 rename_all），前端保持 snake_case
// - 后端 Tauri 命令 `DiscoverRequest` 使用 `#[serde(rename_all = "camelCase")]`，
//   故 `DiscoverRequestPayload` 顶层字段用 camelCase，嵌套结构仍为 snake_case

// ── 能力护照（数字护照） ──────────────────────────

/** 能力承载载体（对应后端 CapabilityKind） */
export type CapabilityKind =
  | "tool"
  | "workflow"
  | "knowledge_base"
  | "agent"
  | "skill";

/** 能力所属业务域（对应后端 CapabilityDomain，snake_case 新值） */
export type CapabilityDomain =
  | "general"
  | "devops"
  | "ai_media"
  | "data_analysis"
  | "content_creation"
  | "communication"
  | "finance"
  | "automation"
  | "system";

/** 安全等级（对应后端 SecurityLevel） */
export type SecurityLevel =
  | "public"
  | "internal"
  | "sensitive"
  | "restricted";

/** 输入模态（对应后端 InputModality） */
export type InputModality = "text" | "image" | "audio" | "video" | "file";

/** 规划复杂度（对应后端 PlanningComplexity） */
export type PlanningComplexity = "simple" | "moderate" | "complex";

/** 模态支持声明（对应后端 ModalitySupport） */
export interface ModalitySupport {
  supports_text: boolean;
  supports_image: boolean;
  supports_audio: boolean;
  supports_video: boolean;
  supports_file: boolean;
}

/** 输出格式能力声明（对应后端 OutputCapabilities） */
export interface OutputCapabilities {
  supports_text: boolean;
  supports_table: boolean;
  supports_chart: boolean;
  supports_image: boolean;
  supports_interactive: boolean;
}

/** 能力运行时统计快照（对应后端 CapabilityStats） */
export interface CapabilityStats {
  /** 总调用次数 */
  total_calls: number;
  /** 成功次数 */
  success_count: number;
  /** 平均执行耗时（秒） */
  avg_duration_seconds: number;
  /** 近 5 次成功率（0.0-1.0） */
  recent_success_rate: number;
  /** 熔断状态（"closed" / "open" / "half_open"） */
  circuit_state: string;
}

/** 能力护照序列化 DTO（对应后端 CapabilityPassportDto） */
export interface CapabilityPassportDto {
  capability_id: string;
  name: string;
  description: string;
  kind: CapabilityKind;
  domain: CapabilityDomain;
  input_schema?: Record<string, unknown> | null;
  tags: string[];
  negative_scenarios: string[];
  security_level: SecurityLevel;
  modality_support: ModalitySupport;
  output_capabilities: OutputCapabilities;
  estimated_cost_usd?: number | null;
  avg_duration_seconds?: number | null;
  planning_complexity: PlanningComplexity;
  model_iq_requirement: number;
  experiment_group?: string | null;
  stats: CapabilityStats;
  enabled: boolean;
}

// ── 检索请求/结果 ──────────────────────────────────

/** 能力检索请求（对应后端 CapabilityQuery） */
export interface CapabilityQuery {
  user_input: string;
  top_k: number;
  kind_filter?: CapabilityKind[] | null;
  domain_filter?: CapabilityDomain[] | null;
  required_modalities?: InputModality[] | null;
  required_tags: string[];
  exclude_ids: string[];
}

/** 命中的候选能力（对应后端 CapabilityCandidate） */
export interface CapabilityCandidate {
  capability_id: string;
  name: string;
  kind: CapabilityKind;
  domain: CapabilityDomain;
  /** 语义相似度得分（0.0-1.0） */
  semantic_score: number;
  /** BM25/关键词匹配得分（0.0-1.0） */
  keyword_score: number;
  /** 标签硬匹配得分（完全匹配=1.0，部分匹配=0.5，无匹配=0.0） */
  tag_score: number;
  /** 综合分（semantic*0.6 + keyword*0.2 + tag*0.2） */
  retrieval_score: number;
  matched_tags: string[];
  negative_hit: boolean;
  passport: CapabilityPassportDto;
}

/** 检索结果（对应后端 CapabilityRetrievalResult） */
export interface CapabilityRetrievalResult {
  candidates: CapabilityCandidate[];
  total_recalled: number;
  elapsed_ms: number;
}

// ── 过滤上下文 ────────────────────────────────────

/** 检测到的 PII 类型（对应后端 PiiType） */
export type PiiType =
  | "id_card"
  | "phone_number"
  | "email"
  | "bank_card"
  | "address"
  | "other";

/** 输出设备类型（对应后端 OutputDeviceType） */
export type OutputDeviceType =
  | "desktop"
  | "laptop"
  | "tablet"
  | "phone"
  | "smart_speaker"
  | "car"
  | "other";

/** 任务规划级别（对应后端 TaskPlanningLevel） */
export type TaskPlanningLevel = "simple" | "moderate" | "complex";

/** 能力过滤上下文（对应后端 FilterContext） */
export interface FilterContext {
  input_modalities: InputModality[];
  detected_pii_types: PiiType[];
  session_budget: SessionBudget;
  device_type: OutputDeviceType;
  task_planning_level?: TaskPlanningLevel | null;
  user_id?: string | null;
  user_history_ids: string[];
  experiment_group?: string | null;
}

// ── 用户偏好 / 预算 ──────────────────────────────────

/** 能力发现的用户偏好权重（对应后端 DiscoveryWeights） */
export interface DiscoveryWeights {
  /** 语义相似度权重（α） */
  alpha: number;
  /** 历史成功率权重（β） */
  beta: number;
  /** 耗时惩罚系数（γ） */
  gamma: number;
  /** 成本惩罚系数（δ） */
  delta: number;
  /** 个性化提权比例 */
  personalization_boost: number;
  /** 冷启动探索提权 */
  exploration_boost: number;
}

/** 单次会话预算（对应后端 SessionBudget） */
export interface SessionBudget {
  /** 总预算上限（美元） */
  max_total_usd: number;
  /** 单次调用上限（美元） */
  max_per_call_usd: number;
  /** 已使用金额 */
  used_usd: number;
}

// ── 排序结果 ──────────────────────────────────────

/** 排序后的能力条目（对应后端 RankedCapability） */
export interface RankedCapability {
  passport: CapabilityPassportDto;
  semantic_score: number;
  history_score: number;
  speed_score: number;
  cost_score: number;
  personalization_boost: number;
  exploration_boost: number;
  final_score: number;
  reasons: string[];
}

// ── 能力发现请求/结果 ──────────────────────────────

/** 能力发现最终结果（对应后端 CapabilityDiscoveryResult） */
export interface CapabilityDiscoveryResult {
  primary_match?: RankedCapability | null;
  alternatives: RankedCapability[];
  ambiguous: boolean;
  clarification_prompt?: string | null;
  suggestions: CapabilitySuggestion[];
  circuit_info?: string | null;
  total_elapsed_ms: number;
  phase_timings: PhaseTiming[];
}

/** 阶段耗时（对应后端 PhaseTiming） */
export interface PhaseTiming {
  phase: string;
  elapsed_ms: number;
}

/** 能力补全建议（对应后端 CapabilitySuggestion） */
export interface CapabilitySuggestion {
  capability_id: string;
  name: string;
  reason: string;
}

// ── 索引结果/统计 ──────────────────────────────────

/** 索引操作结果（对应后端 IndexResult） */
export interface IndexResult {
  capability_id: string;
  success: boolean;
  vector_dimensions: number;
  indexed_at_ms: number;
  error?: string | null;
}

/** 索引统计（对应后端 CapabilityIndexStats） */
export interface CapabilityIndexStats {
  total_capabilities: number;
  total_vectors: number;
  positive_vectors: number;
  negative_vectors: number;
  last_indexed_at?: number | null;
}

// ── 能力注册表（capability_registry_dump） ──────────
//
// 对应后端 capability.rs 的 CapabilityRegistrationDetailDto（`#[serde(rename_all =
// "camelCase")]`），即 `capability_registry_dump` 命令的返回类型。它检视的是
// 「一切皆插件」能力注册表（harness capability_registry）里已注册的能力接缝，
// 与上面的「能力护照」体系（capability_passport）是两套独立机制。

/** 能力来源（对应后端 harness CapabilityOrigin，snake_case） */
export type CapabilityOrigin = "builtin" | "external_plugin";

/** 能力注册表条目（对应后端 CapabilityRegistrationDetailDto，camelCase 字段） */
export interface CapabilityRegistrationDetailDto {
  /** 能力接缝 ID，如 "model.provider.openai"、"agent.loop" */
  id: string;
  /** 接缝契约版本 */
  version: string;
  /** 接缝契约类型（trait 全名），如 "axagent_harness::ProviderAdapter" */
  contract: string;
  /** 接缝描述 */
  description: string;
  /** 来源：内置（builtin）或外部插件（external_plugin） */
  origin: CapabilityOrigin;
  /** 声明该能力的插件 ID（内置能力为 null） */
  pluginId?: string | null;
}

// ── Store 载荷 ─────────────────────────────────────

/**
 * 能力发现请求载荷
 *
 * 对应后端 `DiscoverRequest`（commands/capability.rs），该 struct 使用
 * `#[serde(rename_all = "camelCase")]`，故顶层字段为 camelCase；
 * 嵌套的 FilterContext / CapabilityQuery / DiscoveryWeights / SessionBudget
 * 仍按各自 struct 的默认 snake_case 字段名传递。
 */
export interface DiscoverRequestPayload {
  userInput: string;
  filterContext?: Partial<FilterContext>;
  query?: Partial<CapabilityQuery>;
  weights?: Partial<DiscoveryWeights>;
  budget?: Partial<SessionBudget>;
  enableCompletion?: boolean;
  enableCircuitBreaker?: boolean;
}

// ── 认知编排器（路由工作流） ──────────────────────
//
// 对应后端 commands/cognitive.rs 的 cognitive_query 统一入口命令。
// 认知编排器是全局唯一被用户消息触发的工作流，完成三层路由决策后
// 按 executionMode 分发执行：Workflow → WorkEngine；其余 → agent_query。

/** 认知编排执行模式（对应后端 ExecutionMode，snake_case） */
export type CognitiveExecutionMode =
  | "ask"
  | "plan"
  | "act"
  | "workflow"
  | "direct"
  | "delegate"
  | "parameter_extract"
  | "clarify";

/** 用户意图提示（对应后端 ModeHint，snake_case）：显式覆盖执行模式，缺省 auto 由路由自动决策 */
export type CognitiveModeHint = "auto" | "ask" | "plan" | "act";

/** 澄清候选能力摘要（对应后端 CandidateSummary，camelCase） */
export interface CognitiveCandidateSummary {
  /** 能力/工作流 ID */
  capabilityId: string;
  name: string;
  description: string;
  /** 路由置信度（0.0 - 1.0） */
  score: number;
  /** 能力种类（workflow / agent 等） */
  kind: string;
  domain: string;
  cluster?: string | null;
}

/** 认知编排澄清待选状态（Clarify 分支）：模糊命中（0.60 ≤ 置信度 ≤ 0.90），
 *  返回 Top2 候选交用户选择，选中后携带 forcedCapabilityId 二次执行。 */
export interface CognitiveClarification {
  /** 候选能力（Top2，含名称/描述/置信度） */
  candidates: CognitiveCandidateSummary[];
  /** 触发澄清的原始输入 */
  originalInput: string;
  /** 所属会话 */
  conversationId: string;
  /** 澄清时乐观创建的用户消息 ID（二次执行时复用，避免重复插入用户消息） */
  userMessageId: string;
}

/** 认知编排统一入口请求（对应后端 CognitiveQueryRequest，camelCase） */
export interface CognitiveQueryRequestPayload {
  /** 用户输入 */
  input: string;
  /** 目标会话 ID（执行必需） */
  conversationId?: string;
  /** 强制路由到指定能力（Clarify 二次执行）：跳过三层路由，直接按能力类型分发执行 */
  forcedCapabilityId?: string;
  /** 用户意图提示（auto/ask/plan/act），仅显式覆盖时传入，缺省 auto 由路由自动决策 */
  modeHint?: CognitiveModeHint;
  /** 提供商 ID */
  providerId?: string;
  /** 模型 ID */
  model_id?: string;
  /** Agent 画像 ID（Agent 执行模式透传） */
  agentProfileId?: string;
  /** 用户自定义系统提示（Agent 执行模式透传） */
  systemPrompt?: string;
  /** Web 搜索提供商 ID（Agent 执行模式透传） */
  searchProviderId?: string;
  /** 前端注入的页面上下文（Agent 执行模式透传） */
  agentContext?: Record<string, unknown>;
  /** Agent 执行选项（禁用工具 / 活跃域等） */
  options?: {
    temperature?: number;
    top_p?: number;
    max_tokens?: number;
    /** 禁用的工具名称列表 */
    disabledTools?: string[];
    /** 活跃功能域列表 */
    activeDomains?: string[];
    /** P0-2 计划确认闸门：开启时后端判定复杂任务后先弹计划草稿等待用户批准 */
    requirePlanApproval?: boolean;
  };
  /** 工作流最大并发节点数（Workflow 模式透传） */
  maxConcurrent?: number;
}

/** 认知编排执行结果视图（对应后端 CognitiveExecutionView，tag="kind"） */
export type CognitiveExecutionView =
  | {
    kind: "workflow";
    workflowId: string;
    executionId: string;
  }
  | {
    kind: "agent";
    conversationId: string;
    assistantMessageId: string;
    /** 计划确认被拒绝时返回 "rejected" */
    status?: string | null;
  }
  | {
    kind: "plan";
    conversationId: string;
    planId: string;
  }
  | {
    kind: "clarify";
    candidates: CognitiveCandidateSummary[];
  };

/** 单个路由阶段的对外视图（对应后端 RouteStageView，camelCase） */
export interface CognitiveRouteStageView {
  stage: string;
  success: boolean;
  confidence: number;
  elapsedMs: number;
  summary: string;
}

/** 认知编排统一入口响应（对应后端 CognitiveQueryResponse，camelCase） */
export interface CognitiveQueryResponse {
  /** 三层路由地址（确定性路径），如 "invest/stock_analysis/tech" */
  routePath: string;
  /** 业务域 */
  domain: string;
  /** 功能集群 */
  cluster: string;
  /** 具体能力/工作流 ID */
  capabilityId: string;
  /** 路由置信度（0.0 - 1.0） */
  confidence: number;
  /** 是否通过 LLM 兜底 */
  isLlmFallback: boolean;
  /** 是否触发熔断 */
  circuitBroken: boolean;
  /** 熔断原因 */
  circuitBreakReason?: string | null;
  /** 备选路径 */
  fallbackPath?: string | null;
  /** 候选列表（Top-K） */
  candidates: string[];
  /** 候选能力详情（Clarify 模式用于用户选择） */
  candidateDetails?: CognitiveCandidateSummary[] | null;
  /** 执行模式（ask / plan / act / workflow / delegate） */
  executionMode: CognitiveExecutionMode;
  /** 各阶段执行记录 */
  stageRecords: CognitiveRouteStageView[];
  /** 总耗时（毫秒） */
  totalElapsedMs: number;
  /** 执行分支结果（Workflow → WorkEngine；其余 → agent_query） */
  execution?: CognitiveExecutionView | null;
}

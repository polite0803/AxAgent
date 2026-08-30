// SPDX-License-Identifier: AGPL-3.0-only
// ! 能力发现系统前端类型定义
//
// 以后端 DTO 为权威来源对齐（见 src-tauri/crates/harness/src/capability*.rs）。
// - 后端枚举使用 `#[serde(rename_all = "snake_case")]`，前端用 snake_case 字面量
// - 后端 struct 字段保持 snake_case，通过 `#[serde(rename_all = "camelCase")]`
//   序列化输出 camelCase，前端消费 camelCase
// - 后端 Tauri 命令 `DiscoverRequest` 使用 `#[serde(rename_all = "camelCase")]`，
//   故 `DiscoverRequestPayload` 顶层字段与嵌套 FilterContext/CapabilityQuery/
//   DiscoveryWeights/SessionBudget 均消费 camelCase

import type { TaskShapeDecision } from "./taskShape";

// ── 能力护照（数字护照） ──────────────────────────

/** 能力承载载体（对应后端 CapabilityKind） */
export type CapabilityKind =
  | "tool"
  | "workflow"
  | "knowledge_base"
  | "agent"
  | "skill"
  | "toolchain"
  | "template";

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

/** 执行模式（对应后端 ExecutionMode，snake_case）— 编排器据此决策执行路径 */
export type ExecutionMode = "sync" | "async" | "streaming";

/** Tool 实现载体类型（对应后端 ImplementationType，snake_case） */
export type ImplementationType =
  | "local_function"
  | "rest_api"
  | "grpc"
  | "shell_script"
  | "mcp"
  | "tauri_command";

/** Tool 实现契约（对应后端 ToolImplementation，camelCase）— 描述"如何调用" */
export interface ToolImplementation {
  /** 实现类型 */
  implType: ImplementationType;
  /** 端点（REST/gRPC：URL；本地：模块路径） */
  endpoint?: string | null;
  /** HTTP 方法（GET/POST 等，仅 rest_api） */
  method?: string | null;
  /** 鉴权方式描述（bearer / api_key / oauth / none） */
  auth?: string | null;
  /** 请求体模板（含占位符） */
  requestTemplate?: string | null;
  /** 响应解析规则（JSONPath / 正则 / 提取表达式） */
  responseParser?: string | null;
}

/** Skill 结构化执行步骤（对应后端 SkillStep，camelCase） */
export interface SkillStep {
  /** 步骤 ID（列表内唯一；为空时按索引隐式编号） */
  stepId?: string;
  /** 引用的能力 ID（Tool / Skill / Workflow） */
  capabilityId: string;
  /** 步骤级参数映射（可选；不填继承上级参数） */
  params?: Record<string, unknown> | null;
  /** 可选执行条件（Rhai 表达式或自然语言） */
  condition?: string | null;
  /** 错误处理策略（stop=短路 / skip=跳过继续 / fallback=回退能力ID） */
  onError?: string | null;
}

/** 能力关系类型（对应后端 RelationshipType，snake_case） */
export type RelationshipType =
  | "depends_on"
  | "uses"
  | "alternative_to"
  | "conflicts_with"
  | "parent_of"
  | "precedes"
  | "follows"
  | "requires_knowledge";

/** 能力关系（对应后端 CapabilityRelationship，camelCase）— 统一能力模型第四层图边 */
export interface CapabilityRelationship {
  /** 源能力 ID（如 tool:read_file） */
  sourceId: string;
  /** 目标能力 ID */
  targetId: string;
  /** 关系类型 */
  relationshipType: RelationshipType;
  /** 关系权重（0.0-1.0，检索排序用；默认 1.0） */
  weight: number;
  /** 关系描述上下文 */
  context?: string | null;
  /** 扩展元信息 */
  metadata?: Record<string, unknown> | null;
}

/** 能力等级（对应后端 CapabilityLevel，snake_case）
 *  由护照多维数据派生（规划复杂度 / IQ 需求 / 成功率 / 耗时 / 成本）；
 *  L1/L2 为低等级，可启用进化提升等级。 */
export type CapabilityLevel = "l1" | "l2" | "l3" | "l4" | "l5";

/** 能力来源（对应后端 CapabilitySource，snake_case） */
export type CapabilitySource = "builtin" | "plugin";

/** 能力可进化性（对应后端 CapabilityEvolvability，snake_case）
 *  - none：不可进化（外部插件只读能力）
 *  - local：载体本地可写，就地提升等级/参数
 *  - derived：进化产出独立副本，原护照保持不变 */
export type CapabilityEvolvability = "none" | "local" | "derived";

/** 模态支持声明（对应后端 ModalitySupport） */
export interface ModalitySupport {
  supportsText: boolean;
  supportsImage: boolean;
  supportsAudio: boolean;
  supportsVideo: boolean;
  supportsFile: boolean;
}

/** 输出格式能力声明（对应后端 OutputCapabilities） */
export interface OutputCapabilities {
  supportsText: boolean;
  supportsTable: boolean;
  supportsChart: boolean;
  supportsImage: boolean;
  supportsInteractive: boolean;
}

/** 能力运行时统计快照（对应后端 CapabilityStats） */
export interface CapabilityStats {
  /** 总调用次数 */
  totalCalls: number;
  /** 成功次数 */
  successCount: number;
  /** 平均执行耗时（秒） */
  avgDurationSeconds: number;
  /** 近 5 次成功率（0.0-1.0） */
  recentSuccessRate: number;
  /** 熔断状态（"closed" / "open" / "half_open"） */
  circuitState: string;
}

/** 能力护照序列化 DTO（对应后端 CapabilityPassportDto） */
export interface CapabilityPassportDto {
  capabilityId: string;
  name: string;
  description: string;
  /** 能力定义版本（语义化，如 "1.2.3"） */
  version?: string | null;
  /** 能力所有者（团队/个人标识） */
  owner?: string | null;
  /** 创建时间（unix ms） */
  createdAt?: number | null;
  /** 最后更新时间（unix ms） */
  updatedAt?: number | null;
  kind: CapabilityKind;
  domain: CapabilityDomain;
  /** L2 子分类（集群 ID）。kind=agent 时用于区分专家(agent_profile)与角色(agent_role) */
  subCategory?: string;
  inputSchema?: Record<string, unknown> | null;
  /** 输出结构的 JSON Schema（None = 无固定输出结构） */
  outputSchema?: Record<string, unknown> | null;
  /** Tool 实现契约（仅 tool 类能力有效：REST/gRPC/本地函数如何调用） */
  implementation?: ToolImplementation | null;
  tags: string[];
  negativeScenarios: string[];
  securityLevel: SecurityLevel;
  modalitySupport: ModalitySupport;
  outputCapabilities: OutputCapabilities;
  estimatedCostUsd?: number | null;
  avgDurationSeconds?: number | null;
  /** 执行模式（sync / async / streaming），默认 sync */
  executionMode: ExecutionMode;
  /** 单次执行最大超时（毫秒）。null = 未声明（使用引擎默认） */
  timeoutMs?: number | null;
  planningComplexity: PlanningComplexity;
  modelIqRequirement: number;
  experimentGroup?: string | null;
  stats: CapabilityStats;
  enabled: boolean;
  /** 能力等级（L1-L5，由多维数据派生；低等级可进化提升） */
  level: CapabilityLevel;
  /** 能力来源（内置 / 插件），用于溯源与进化边界判断 */
  source: CapabilitySource;
  /** 能力可进化性（决定进化引擎分发边界：none / local / derived） */
  evolvable: CapabilityEvolvability;
  /** 暴露模式（auto=被动全量+主动命中注入；on_demand=仅命中注入；managed=仅路由） */
  exposure: CapabilityExposure;
  /** 真实工具定义引用（主动模式命中后凭此注入 chat_tools，解决"发现的能力执行不了"） */
  toolRef?: CapabilityToolRef | null;
  /** 别名列表（用户口语→能力 ID，检索时命中别名直接进候选；如 "发邮件"→mail_send） */
  aliases: string[];
  /** 工具链步骤（仅 toolchain 类型有效：按序 capability_id 列表，线性串接、失败短路） */
  steps: string[];
  /** Skill 结构化执行步骤（仅 skill 类型有效：步骤级参数/条件/错误处理） */
  skillSteps: SkillStep[];
  /** 模板占位符（仅 template 类型有效：命中后提示"可实例化"，不直接执行） */
  placeholders: PlaceholderDef[];
  /** 模板正文（仅 template 类型有效：含占位符的模板内容） */
  templateBody?: string | null;
  /** 实例化目标类型（仅 template 类型有效：skill / workflow） */
  instantiatesTo?: CapabilityKind | null;
  /** 示例实例（仅 template 类型有效：能力 ID 或内联定义，供 LLM 参考） */
  exampleInstance?: string | null;
  /** 上游依赖能力 ID 列表（关联扩展：检索命中后一跳向上扩展） */
  upstream: string[];
  /** 下游依赖能力 ID 列表（关联扩展：检索命中后一跳向下扩展） */
  downstream: string[];
  /** 前置条件（P1：Skill preconditions，如 "network_available"；条件检查启用时未满足即过滤） */
  preconditions: string[];
  /** 附带知识片段（P2：能力与信息分离，随能力描述注入上下文，不单独执行） */
  attachedSnippets: KnowledgeSnippet[];
}

/** 随能力附带的知识片段（P2：如"漏洞扫描"Skill 附带"当前支持的 CVE 编号范围"） */
export interface KnowledgeSnippet {
  /** 片段键（如 supported_cve_range） */
  key: string;
  /** 片段内容 */
  content: string;
}

/** 模板占位符定义（如 {{target_ip}} / {{date_range}}） */
export interface PlaceholderDef {
  /** 占位符名（不含双花括号，如 target_ip） */
  name: string;
  /** 期望类型：string / ip / date_range / number / enum */
  placeholderType: string;
  /** 占位符说明 */
  description: string;
}

/** 能力暴露模式（暴露层架构：被动自动暴露 vs 主动按需注入） */
export type CapabilityExposure = "auto" | "on_demand" | "managed";

/** 护照到真实工具定义的引用 */
export interface CapabilityToolRef {
  /** 注册表中的工具名（ChatTool.function.name / UnifiedToolRegistry 键名） */
  toolName: string;
  /** 注册表来源：builtin / mcp / skill / tauri_command */
  registry: string;
}

// ── 检索请求/结果 ──────────────────────────────────

/** 能力检索请求（对应后端 CapabilityQuery） */
export interface CapabilityQuery {
  userInput: string;
  topK: number;
  kindFilter?: CapabilityKind[] | null;
  domainFilter?: CapabilityDomain[] | null;
  requiredModalities?: InputModality[] | null;
  requiredTags: string[];
  excludeIds: string[];
}

/** 检索层级（对应后端 CapabilityLayer）：应用层/任务层/原子层（P0 分层检索） */
export type CapabilityLayer = "app" | "task" | "atomic";

/** 命中的候选能力（对应后端 CapabilityCandidate） */
export interface CapabilityCandidate {
  capabilityId: string;
  name: string;
  kind: CapabilityKind;
  domain: CapabilityDomain;
  /** 检索层级（App/Task/Atomic，由 kind 推导） */
  layer: CapabilityLayer;
  /** 语义相似度得分（0.0-1.0） */
  semanticScore: number;
  /** BM25/关键词匹配得分（0.0-1.0） */
  keywordScore: number;
  /** 标签硬匹配得分（完全匹配=1.0，部分匹配=0.5，无匹配=0.0） */
  tagScore: number;
  /** 综合分（semantic*0.6 + keyword*0.2 + tag*0.2） */
  retrievalScore: number;
  matchedTags: string[];
  negativeHit: boolean;
  passport: CapabilityPassportDto;
}

/** 检索结果（对应后端 CapabilityRetrievalResult） */
export interface CapabilityRetrievalResult {
  candidates: CapabilityCandidate[];
  totalRecalled: number;
  elapsedMs: number;
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
  inputModalities: InputModality[];
  detectedPiiTypes: PiiType[];
  sessionBudget: SessionBudget;
  deviceType: OutputDeviceType;
  taskPlanningLevel?: TaskPlanningLevel | null;
  userId?: string | null;
  userHistoryIds: string[];
  experimentGroup?: string | null;
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
  personalizationBoost: number;
  /** 冷启动探索提权 */
  explorationBoost: number;
}

/** 单次会话预算（对应后端 SessionBudget） */
export interface SessionBudget {
  /** 总预算上限（美元） */
  maxTotalUsd: number;
  /** 单次调用上限（美元） */
  maxPerCallUsd: number;
  /** 已使用金额 */
  usedUsd: number;
}

// ── 排序结果 ──────────────────────────────────────

/** 排序后的能力条目（对应后端 RankedCapability） */
export interface RankedCapability {
  passport: CapabilityPassportDto;
  semanticScore: number;
  historyScore: number;
  speedScore: number;
  costScore: number;
  personalizationBoost: number;
  explorationBoost: number;
  finalScore: number;
  reasons: string[];
}

// ── 能力发现请求/结果 ──────────────────────────────

/** 能力发现最终结果（对应后端 CapabilityDiscoveryResult） */
export interface CapabilityDiscoveryResult {
  primaryMatch?: RankedCapability | null;
  alternatives: RankedCapability[];
  ambiguous: boolean;
  clarificationPrompt?: string | null;
  suggestions: CapabilitySuggestion[];
  circuitInfo?: string | null;
  totalElapsedMs: number;
  phaseTimings: PhaseTiming[];
  /** 命中的 Template 能力从用户输入提取的实体（P1：语义解析的实体部分） */
  extractedEntities: CapabilityEntity[];
}

/** 提取出的能力实体（P1：Template placeholders 从输入提取的值） */
export interface CapabilityEntity {
  /** 占位符名（对应 PlaceholderDef.name） */
  name: string;
  /** 从输入中提取到的值 */
  value: string;
  /** 实体类型（ip / date_range / number / email / url / id） */
  entityType: string;
  /** 该占位符的说明 */
  description?: string;
}

/** 阶段耗时（对应后端 PhaseTiming） */
export interface PhaseTiming {
  phase: string;
  elapsedMs: number;
}

/** 能力补全建议（对应后端 CapabilitySuggestion） */
export interface CapabilitySuggestion {
  capabilityId: string;
  name: string;
  reason: string;
}

// ── 索引结果/统计 ──────────────────────────────────

/** 索引操作结果（对应后端 IndexResult） */
export interface IndexResult {
  capabilityId: string;
  success: boolean;
  vectorDimensions: number;
  indexedAtMs: number;
  error?: string | null;
}

/** 索引统计（对应后端 CapabilityIndexStats） */
export interface CapabilityIndexStats {
  totalCapabilities: number;
  totalVectors: number;
  positiveVectors: number;
  negativeVectors: number;
  lastIndexedAt?: number | null;
}

// ── 能力进化（capability_evolve） ──────────────────────
//
// 对应后端 commands/capability.rs 的 EvolveCapabilityRequest / EvolveCapabilityResult
// （`#[serde(rename_all = "camelCase")]`），即「低等级能力一键进化提升」命令。

/** 能力进化请求（camelCase 字段） */
export interface EvolveCapabilityRequest {
  /** 能力护照 ID（如 `workflow:{template_id}` / `skill:{name}`） */
  capabilityId: string;
  /** 工作流进化反思上下文（可选；缺省走启发式变异） */
  reflections?: unknown[];
}

/** 能力进化结果（camelCase 字段） */
export interface EvolveCapabilityResult {
  capabilityId: string;
  /** 进化是否产生有效改进 */
  improved: boolean;
  /** 进化前等级 */
  oldLevel: CapabilityLevel;
  /** 进化后等级 */
  newLevel: CapabilityLevel;
  /** 进化引擎返回的原始结果摘要 */
  detail: Record<string, unknown>;
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
 * `#[serde(rename_all = "camelCase")]`，故顶层字段与嵌套
 * FilterContext / CapabilityQuery / DiscoveryWeights / SessionBudget
 * 均消费 camelCase 字段名。
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
  modelId?: string;
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
    topP?: number;
    maxTokens?: number;
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

/** 选中的执行专家（Agent 执行路径）视图（对应后端 SelectedAgentProfileView，camelCase） */
export interface CognitiveSelectedAgentProfile {
  /** AgentProfile ID */
  id: string;
  /** 专家名称 */
  name: string;
  /** 角色名（agent_role，可空） */
  role?: string | null;
  /** 关联专家（expert_id → agency_experts.name，可空） */
  expert?: string | null;
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
  /** 熔断过滤数量（RAR 原始候选数 - 最终候选数，0 表示无过滤） */
  filteredCount?: number;
  /** 执行模式（ask / plan / act / workflow / delegate） */
  executionMode: CognitiveExecutionMode;
  /** 选中工作流的可读名称（未命中工作流时为 null） */
  selectedWorkflowName?: string | null;
  /** 选中的执行专家（Agent 执行路径；未走 Agent 路径时为 null） */
  selectedAgentProfile?: CognitiveSelectedAgentProfile | null;
  /** 各阶段执行记录 */
  stageRecords: CognitiveRouteStageView[];
  /** 总耗时（毫秒） */
  totalElapsedMs: number;
  /** 执行分支结果（Workflow → WorkEngine；其余 → agent_query） */
  execution?: CognitiveExecutionView | null;
  /**
   * P1: 任务形态决策（原则三标尺输出，Step 0 产出）
   *
   * 当 UNITY_P0_TASK_SHAPE flag 启用时由 DefaultTaskShapeClassifier 在路由前产出，
   * 随响应返回前端展示决策标签（两条标尺 + 推荐策略 + 合并/拆分倾向）。
   * null 表示 flag 未启用或分类失败已回退。
   */
  taskShape?: TaskShapeDecision | null;
}

// ── 遗留边界③：任务拆解 → 逐项能力发现（cognitive_decompose_task） ──

/** 任务拆解请求（对应后端 DecomposeTaskRequest） */
export interface DecomposeTaskRequest {
  /** 用户原始任务 */
  input: string;
  /** 每个子目标的能力发现候选数（缺省 5） */
  topK?: number;
}

/** 单个子目标 + 其能力发现结果（对应后端 SubGoalDiscoveryDto） */
export interface SubGoalDiscoveryDto {
  subTaskId: string;
  name: string;
  description: string;
  /** 前置子任务 ID（依赖拓扑） */
  dependencies: string[];
  /** 该子目标的能力发现结果（primaryMatch + alternatives） */
  discovery?: CapabilityDiscoveryResult | null;
}

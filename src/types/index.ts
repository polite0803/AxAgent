// SPDX-License-Identifier: AGPL-3.0-only

// === Provider System ===
import type { RAGPipelineConfig } from "./knowledge";
import type { TaskShapeDecision } from "./taskShape";

export type ProviderType =
  | "openai"
  | "openai_responses"
  | "anthropic"
  | "gemini"
  | "openclaw"
  | "hermes"
  | "ollama"
  | "llama_cpp";

export interface ProviderConfig {
  id: string;
  name: string;
  providerType: ProviderType;
  apiHost: string;
  apiPath: string | null;
  enabled: boolean;
  models: Model[];
  keys: ProviderKey[];
  proxyConfig: ProviderProxyConfig | null;
  toolAdaptation: string | null;
  toolAdaptationMarkerPrefix: string | null;
  customHeaders: string | null;
  icon: string | null;
  builtinId: string | null;
  sortOrder: number;
  createdAt: number;
  updatedAt: number;
}

export interface ProviderKey {
  id: string;
  providerId: string;
  keyEncrypted: string;
  keyPrefix: string;
  enabled: boolean;
  lastValidatedAt: number | null;
  lastError: string | null;
  rotationIndex: number;
  createdAt: number;
}

export interface ProviderProxyConfig {
  proxyType: string | null;
  proxyAddress: string | null;
  proxyPort: number | null;
}

export interface CreateProviderInput {
  name: string;
  providerType: ProviderType;
  apiHost: string;
  apiPath?: string | null;
  enabled: boolean;
}

export interface UpdateProviderInput {
  name?: string;
  providerType?: ProviderType;
  apiHost?: string;
  apiPath?: string | null;
  enabled?: boolean;
  proxyConfig?: ProviderProxyConfig;
  toolAdaptation?: string | null;
  toolAdaptationMarkerPrefix?: string | null;
  customHeaders?: string | null;
  icon?: string | null;
  sortOrder?: number;
}

// === Model System ===
export type ModelCapability =
  | "TextChat"
  | "Vision"
  | "FunctionCalling"
  | "Reasoning"
  | "RealtimeVoice";
export type ModelType = "Chat" | "Voice" | "Embedding";

export interface Model {
  providerId: string;
  modelId: string;
  name: string;
  groupName?: string | null;
  modelType: ModelType;
  capabilities: ModelCapability[];
  maxTokens: number | null;
  enabled: boolean;
  paramOverrides: ModelParamOverrides | null;
  /** Input price per million tokens (USD). From provider sync or user settings. */
  inputPricePerMtok?: number | null;
  /** Output price per million tokens (USD). From provider sync or user settings. */
  outputPricePerMtok?: number | null;
}

export interface ModelParamOverrides {
  temperature?: number;
  maxTokens?: number;
  topP?: number;
  frequencyPenalty?: number;
  useMaxCompletionTokens?: boolean;
  noSystemRole?: boolean;
  forceMaxTokens?: boolean;
  thinkingParamStyle?: string;
  requestDelayMs?: number;
}

// === Conversation & Message ===
export type MessageRole = "system" | "user" | "assistant" | "tool";

export type MessageStatus = "complete" | "partial" | "error" | "cancelled";

export interface ConversationCategory {
  id: string;
  name: string;
  iconType: string | null;
  iconValue: string | null;
  systemPrompt: string | null;
  defaultProviderId: string | null;
  defaultModelId: string | null;
  defaultTemperature: number | null;
  defaultMaxTokens: number | null;
  defaultTopP: number | null;
  defaultFrequencyPenalty: number | null;
  sortOrder: number;
  isCollapsed: boolean;
  createdAt: number;
  updatedAt: number;
}

export interface Conversation {
  id: string;
  title: string;
  modelId: string;
  providerId: string;
  systemPrompt: string | null;
  temperature: number | null;
  maxTokens: number | null;
  topP: number | null;
  frequencyPenalty: number | null;
  searchEnabled: boolean;
  searchProviderId: string | null;
  thinkingBudget: number | null;
  enabledMcpServerIds: string[];
  enabledKnowledgeBaseIds: string[];
  enabledMemoryNamespaceIds: string[];
  enabledWikiIds: string[];
  isPinned: boolean;
  isArchived: boolean;
  contextCompression: boolean;
  categoryId: string | null;
  parentConversationId: string | null;
  mode: "chat" | "agent" | "gateway";
  /** Agent work strategy: "direct" = execute immediately, "plan" = generate plan first, await approval, then execute */
  workStrategy?: "direct" | "plan" | null;
  messageCount: number;
  createdAt: number;
  updatedAt: number;
  scenario?: string | null;
  workspaceDir?: string | null;
  enabledSkillIds: string[];
  /** Agent profile identifier, references AgentProfile.id */
  agentProfileId?: string | null;
  /** Workflow template ID bound to this conversation */
  workflowTemplateId?: string | null;
  /** Session type: "conversation" = free dialog, "workflow" = bound to workflow template */
  sessionType: "conversation" | "workflow";
  /** Workflow execution status: running / completed / failed / cancelled */
  workflowStatus?: string | null;
}

export interface ToolCall {
  id: string;
  type: string;
  function: {
    name: string;
    arguments: string;
  };
}

export interface Message {
  id: string;
  conversationId: string;
  role: MessageRole;
  content: string;
  providerId: string | null;
  modelId: string | null;
  tokenCount: number | null;
  promptTokens?: number | null;
  completionTokens?: number | null;
  attachments: Attachment[];
  thinking: string | null;
  toolCallsJson: string | null;
  toolCallId: string | null;
  createdAt: number;
  parentMessageId: string | null;
  versionIndex: number;
  isActive: boolean;
  status: MessageStatus;
  tokensPerSecond?: number | null;
  firstTokenLatencyMs?: number | null;
  /** Structured content blocks (from agent session ContentBlock). */
  blocks?: ContentBlock[];
  /** Additional metadata for extensibility */
  meta?: Record<string, unknown>;
  /** 引用回复：被引用消息的 ID（区别于 parent_message_id） */
  quotedMessageId?: string | null;
  /** 意图澄清状态（仅用户消息有） */
  intentClarification?: IntentClarification | null;
  /** 认知编排决策标签：该消息对应一轮执行的决策信息（ExecutionMode / 路由路径 / 命中工作流 / 专家等） */
  decision?: CognitiveDecisionInfo | null;
  /** P0: 任务形态决策（原则三标尺输出，每条消息显示其分类决策） */
  taskShape?: TaskShapeDecision | null;
}

// ── 认知编排决策标签 ────────────────────────────────
/** 单条消息的认知编排决策信息，由后端 cognitive_query 写入并持久化。 */
export interface CognitiveDecisionInfo {
  /** 执行模式：Workflow / Direct / Delegate / Ask / Plan / Act / ParameterExtract */
  executionMode: string;
  /** 三层路由路径（如 /trade/refund/auto） */
  routePath: string;
  /** 路由置信度 */
  confidence: number;
  /** 命中工作流名称（Workflow 执行模式有值） */
  selectedWorkflowName?: string | null;
  /** 选中的专家/角色画像（Agent 执行模式有值） */
  selectedAgentProfile?: {
    id: string;
    name: string;
    role: string | null;
    expert: string | null;
  } | null;
  /** P1: 任务形态决策（原则三标尺输出，与后端 build_decision_value.taskShape 对齐） */
  taskShape?: TaskShapeDecision | null;
}

// ── Content Block (Part-based message model, short-term) ──────────────
export type ContentBlock =
  | { type: "text"; text: string }
  | { type: "tool_use"; id: string; name: string; input: string }
  | {
    type: "tool_result";
    toolUseId: string;
    toolName: string;
    output: string;
    isError: boolean;
  };

export interface MessagePage {
  messages: Message[];
  hasOlder: boolean;
  oldestMessageId: string | null;
  totalActiveCount: number;
}

export interface ConversationStats {
  totalMessages: number;
  totalUserMessages: number;
  totalAssistantMessages: number;
  totalPromptTokens: number;
  totalCompletionTokens: number;
  totalTokens: number;
  avgTokensPerSecond: number | null;
  avgFirstTokenLatencyMs: number | null;
  avgResponseTimeMs: number | null;
}

export interface DailyUsage {
  date: string;
  messageCount: number;
  totalPromptTokens: number;
  totalCompletionTokens: number;
  totalTokens: number;
  totalCostUsd: number;
}

export interface CostByProvider {
  providerId: string;
  requestCount: number;
  tokenCount: number;
  costUsd: number;
}

export interface Attachment {
  id: string;
  fileType: string;
  fileName: string;
  filePath: string;
  fileSize: number;
  data?: string;
}

export interface AttachmentInput {
  fileName: string;
  fileType: string;
  fileSize: number;
  data: string;
}

export interface ConversationSearchResult {
  conversation: Conversation;
  matchedMessagePreview: string | null;
}

// P2: Cross-session FTS5 search result
export interface SessionSearchResult {
  conversationId: string;
  conversationTitle: string;
  role: string;
  snippet: string;
  rank: number;
}

export interface ConversationSummary {
  id: string;
  conversationId: string;
  summaryText: string;
  compressedUntilMessageId: string | null;
  tokenCount: number | null;
  modelUsed: string | null;
  createdAt: number;
  updatedAt: number;
}

export interface UpdateConversationInput {
  title?: string;
  providerId?: string;
  modelId?: string;
  isPinned?: boolean;
  isArchived?: boolean;
  systemPrompt?: string;
  temperature?: number | null;
  maxTokens?: number | null;
  topP?: number | null;
  frequencyPenalty?: number | null;
  searchEnabled?: boolean;
  searchProviderId?: string | null;
  thinkingBudget?: number | null;
  enabledMcpServerIds?: string[];
  enabledKnowledgeBaseIds?: string[];
  enabledMemoryNamespaceIds?: string[];
  enabledWikiIds?: string[];
  contextCompression?: boolean;
  categoryId?: string | null;
  parentConversationId?: string | null;
  mode?: "chat" | "agent" | "gateway";
  workStrategy?: "direct" | "plan" | null;
  scenario?: string | null;
  enabledSkillIds?: string[];
  agentProfileId?: string | null;
  workflowTemplateId?: string | null;
  sessionType?: "conversation" | "workflow";
  workflowStatus?: string | null;
}

// === Gateway System ===
export interface GatewayStatus {
  isRunning: boolean;
  listenAddress: string;
  port: number;
  sslEnabled: boolean;
  startedAt: number | null;
  /** HTTPS listener port; `null` when SSL is disabled or not yet started. */
  httpsPort: number | null;
  /** When `true` the gateway redirects all HTTP traffic to HTTPS. */
  forceSsl: boolean;
}

export interface GatewayKey {
  id: string;
  name: string;
  keyHash: string;
  keyPrefix: string;
  enabled: boolean;
  createdAt: number;
  lastUsedAt: number | null;
  hasEncryptedKey: boolean;
}

export interface CreateGatewayKeyResult {
  gatewayKey: GatewayKey;
  plainKey: string;
}

export interface GatewayMetrics {
  totalRequests: number;
  totalTokens: number;
  totalRequestTokens: number;
  totalResponseTokens: number;
  activeConnections: number;
  todayRequests: number;
  todayTokens: number;
  todayRequestTokens: number;
  todayResponseTokens: number;
  totalCostUsd: number;
  todayCostUsd: number;
}

export interface UsageByKey {
  keyId: string;
  keyName: string;
  requestCount: number;
  tokenCount: number;
  requestTokens: number;
  responseTokens: number;
}

export interface UsageByProvider {
  providerId: string;
  providerName: string;
  requestCount: number;
  tokenCount: number;
  requestTokens: number;
  responseTokens: number;
}

export interface UsageByDay {
  date: string;
  requestCount: number;
  tokenCount: number;
  requestTokens: number;
  responseTokens: number;
}

export interface ConnectedProgram {
  keyId: string;
  keyName: string;
  keyPrefix: string;
  todayRequests: number;
  todayTokens: number;
  todayRequestTokens: number;
  todayResponseTokens: number;
  lastActiveAt: number | null;
  isActive: boolean;
}

export interface GatewayStats {
  totalRequests: number;
  activeConnections: number;
  uptimeSeconds: number;
  requestsPerMinute: number;
}

export interface GatewaySettings {
  listenAddress: string;
  port: number;
  loadBalanceStrategy: "round_robin";
}

// === Settings ===
export interface AppSettings {
  language: string;
  themeMode: string;
  themePreset: string;
  primaryColor: string;
  borderRadius: number;
  autoStart: boolean;
  showOnStart: boolean;
  minimizeToTray: boolean;
  fontSize: number;
  fontWeight: number;
  fontFamily: string;
  codeFontFamily: string;
  bubbleStyle: string;
  codeTheme: string;
  codeThemeLight: string;
  defaultProviderId: string | null;
  defaultModelId: string | null;
  defaultTemperature: number | null;
  defaultMaxTokens: number | null;
  defaultTopP: number | null;
  defaultFrequencyPenalty: number | null;
  defaultContextCount: number | null;
  titleSummaryProviderId: string | null;
  titleSummaryModelId: string | null;
  titleSummaryTemperature: number | null;
  titleSummaryMaxTokens: number | null;
  titleSummaryTopP: number | null;
  titleSummaryFrequencyPenalty: number | null;
  titleSummaryContextCount: number | null;
  titleSummaryPrompt: string | null;
  compressionProviderId: string | null;
  compressionModelId: string | null;
  compressionTemperature: number | null;
  compressionMaxTokens: number | null;
  compressionTopP: number | null;
  compressionFrequencyPenalty: number | null;
  compressionPrompt: string | null;
  proxyType: string | null;
  proxyAddress: string | null;
  proxyPort: number | null;
  globalShortcut: string;
  shortcutToggleCurrentWindow: string;
  shortcutToggleAllWindows: string;
  shortcutCloseWindow: string;
  shortcutNewConversation: string;
  shortcutOpenSettings: string;
  shortcutToggleModelSelector: string;
  shortcutFillLastMessage: string;
  shortcutClearContext: string;
  shortcutClearConversationMessages: string;
  shortcutToggleGateway: string;
  shortcutToggleMode: string;
  shortcutShowQuickBar: string;
  gatewayAutoStart: boolean;
  gatewayListenAddress: string;
  gatewayPort: number;
  gatewaySslEnabled: boolean;
  gatewaySslMode: string;
  gatewaySslCertPath: string | null;
  gatewaySslKeyPath: string | null;
  gatewaySslPort: number;
  gatewayForceSsl: boolean;
  // Desktop integration
  alwaysOnTop?: boolean;
  trayEnabled?: boolean;
  globalShortcutsEnabled?: boolean;
  shortcutRegistrationLogsEnabled?: boolean;
  shortcutTriggerToastEnabled?: boolean;
  notificationsEnabled?: boolean;
  miniWindowEnabled?: boolean;
  startMinimized?: boolean;
  closeToTray?: boolean;
  notifyBackup?: boolean;
  notifyImport?: boolean;
  notifyErrors?: boolean;
  // Auto-backup settings
  backupDir?: string | null;
  autoBackupEnabled?: boolean;
  autoBackupIntervalHours?: number;
  autoBackupMaxCount?: number;
  // WebDAV sync settings
  webdavHost?: string | null;
  webdavUsername?: string | null;
  webdavPath?: string | null;
  webdavAcceptInvalidCerts?: boolean;
  webdavSyncEnabled?: boolean;
  webdavSyncIntervalMinutes?: number;
  webdavMaxRemoteBackups?: number;
  webdavIncludeDocuments?: boolean;
  // S3 sync settings
  s3Endpoint?: string | null;
  s3Region?: string | null;
  s3Bucket?: string | null;
  s3AccessKeyId?: string | null;
  s3Root?: string | null;
  s3UsePathStyle?: boolean;
  s3SyncEnabled?: boolean;
  s3SyncIntervalMinutes?: number;
  s3MaxRemoteBackups?: number;
  s3IncludeDocuments?: boolean;
  /** Closed-loop nudge scheduler enabled */
  closedLoopEnabled?: boolean;
  /** Closed-loop nudge interval in minutes (default 5) */
  closedLoopIntervalMinutes?: number;
  lastSelectedConversationId?: string | null;
  /** Custom documents root override (overrides ~/Documents/axagent/) */
  documentsRootOverride?: string | null;
  /** Auto update check interval in minutes (default 60, min 1) */
  updateCheckInterval?: number;
  /** Global system prompt fallback — used when a conversation has no custom system prompt */
  defaultSystemPrompt?: string | null;
  /** Chat minimap / navigation overlay */
  chatMinimapEnabled?: boolean;
  chatMinimapStyle?: "faq" | "sticky";
  /** Agent execution panel — show right-side panel during agent mode */
  agentPanelEnabled?: boolean;
  /** Agent execution panel — use compact (simplified) view by default */
  agentPanelCompact?: boolean;
  /** Onboarding — welcome wizard completed */
  onboardingCompleted?: boolean;
  /** Onboarding — wizard dismissed (user skipped) */
  onboardingWizardDismissed?: boolean;
  /** Onboarding — interactive tutorial completed */
  onboardingTutorialCompleted?: boolean;
  /** Onboarding — selected quick-start preset */
  onboardingSelectedPreset?: string | null;
  /** Multi-model response display mode */
  multiModelDisplayMode?: "tabs" | "side-by-side" | "stacked";
  /** Render user messages as Markdown (like AI messages). Default: false */
  renderUserMarkdown?: boolean;
  /** Default workspace directory for new sessions when not manually set */
  defaultWorkspaceDir?: string | null;
  /** Enable screen perception and vision-based UI control */
  screenPerceptionEnabled?: boolean;
  /** Enable RL optimizer for tool selection and task strategies */
  rlOptimizerEnabled?: boolean;
  /** Enable LoRA fine-tuning for custom model adaptation */
  loraFinetuneEnabled?: boolean;
  /** Enable proactive nudge suggestions based on context */
  proactiveNudgeEnabled?: boolean;
  /** Enable thought chain visualization for reasoning */
  thoughtChainEnabled?: boolean;
  /** Enable automatic error recovery suggestions */
  errorRecoveryEnabled?: boolean;
  /** Enable Tree of Thoughts multi-path reasoning (expensive) */
  totEnabled?: boolean;
  /** Show the developer tools section (Trace/Benchmark/Fine-Tune/RL) in the sidebar */
  showDeveloperTools?: boolean;
  /** Cloud workspace URI (supports s3://, webdav://, local://) */
  workspaceUri?: string | null;
  /** Cloud backend type: "s3" | "webdav" | null */
  cloudBackend?: string | null;
  /** S3 provider preset key (e.g., "Aws", "TencentCos", "Custom") */
  s3ProviderPreset?: string | null;
  /** S3 secret access key */
  s3SecretAccessKey?: string | null;
  /** WebDAV password */
  webdavPassword?: string | null;
  /** Cloud sync enabled flag */
  cloudSyncEnabled?: boolean;
  /** RAG 高级管线配置（查询增强、重排序、自省式质检） */
  ragPipelineConfig?: RAGPipelineConfig;
  /**
   * 2.7 P1:遥测级别三级开关 — "off" | "minimal" | "full"。
   *
   * - `off`:完全关闭遥测(默认)
   * - `minimal`:仅记录用户行为级事件(Analytics / SessionTrace)
   * - `full`:记录所有遥测事件(含 HTTP 请求细节)
   *
   * 后端 `FilteringSink` 装饰器在运行时通过共享 `Arc<RwLock<TelemetryLevel>>`
   * 引用此设置,`save_settings` 命令保存后立即生效。
   */
  telemetryLevel?: "off" | "minimal" | "full";
  /** Smart Router 智能路由总开关。开启后按任务复杂度自动选择模型 tier。 */
  smartRouterEnabled?: boolean;
  /** tier(budget/balanced/premium) → provider/model 映射表。 */
  smartRouterTierMappings?: Record<string, SmartRouterTierMapping>;
  /** Auto-load downloaded GGUF models into memory when RAG pipeline is active. */
  autoLoadModels?: boolean;
  /** P2-8: ACP (Agent Client Protocol) 服务端 base URL。null 时使用默认值。 */
  acpBaseUrl?: string | null;
}

/** Smart Router tier → provider/model 映射项（对应后端 harness TierModelMapping）。 */
export interface SmartRouterTierMapping {
  /** 目标模型 ID（如 "gpt-4o-mini"） */
  modelId?: string;
  /** 目标 provider ID（如 "openai"） */
  providerId?: string;
  /** 可选的 base URL 覆盖（自建端点 / 代理） */
  baseUrlOverride?: string | null;
}

// === Streaming ===
export interface ChatStreamChunk {
  content: string | null;
  thinking: string | null;
  toolCalls: ToolCall[] | null;
  done: boolean;
  isFinal?: boolean | null;
  usage: TokenUsage | null;
}

export interface ChatStreamEvent {
  conversationId: string;
  messageId: string;
  modelId?: string;
  providerId?: string;
  chunk: ChatStreamChunk;
}

export interface ChatStreamErrorEvent {
  conversationId: string;
  messageId: string;
  error: string;
}

export interface TokenUsage {
  promptTokens: number;
  completionTokens: number;
  totalTokens: number;
}

// === Voice ===
export type VoiceSessionState =
  | "Idle"
  | "Connecting"
  | "Connected"
  | "Speaking"
  | "Listening"
  | "Disconnecting"
  | "Error";

// === Intent Clarification ===
export type IntentState =
  | "draft"
  | "clarifying"
  | "needs_confirmation"
  | "submitted"
  | "cancelled";

export interface IntentClarification {
  state: IntentState;
  /** 用户原始输入（语音/文本） */
  originalInput: string;
  /** AI 理解的意图描述 */
  intentSummary?: string;
  /** 澄清问题列表 */
  clarificationQuestions: string[];
  /** 用户对澄清问题的回答 */
  clarificationAnswers: Record<string, string>;
  /** 确认候选方案 */
  confirmationOptions?: string[];
  /** 最终确认的意图 */
  confirmedIntent?: string;
  /** 关联的 DAG 执行 ID */
  workflowExecutionId?: string;
  /** 创建时间戳 */
  createdAt: number;
  /** 更新时间戳 */
  updatedAt: number;
}

export type AudioEncoding = "Pcm16" | "Opus";

export interface AudioFormat {
  sampleRate: number;
  channels: number;
  encoding: AudioEncoding;
}

export interface RealtimeConfig {
  modelId: string;
  voice: string | null;
  audioFormat: AudioFormat;
  sttProviderId?: string | null;
  ttsProviderId?: string | null;
}

// === Gateway Link (Client-side Gateway Connection) ===
export type GatewayLinkType = "openclaw" | "hermes" | "custom";
export type GatewayLinkStatus =
  | "connected"
  | "disconnected"
  | "connecting"
  | "error";

export interface GatewayLink {
  id: string;
  name: string;
  linkType: GatewayLinkType;
  endpoint: string;
  apiKeyId: string | null;
  enabled: boolean;
  status: GatewayLinkStatus;
  errorMessage: string | null;
  autoSyncModels: boolean;
  autoSyncSkills: boolean;
  lastSyncAt: number | null;
  latencyMs: number | null;
  version: string | null;
  createdAt: number;
  updatedAt: number;
}

export interface GatewayLinkModelSync {
  modelId: string;
  providerName: string;
  syncStatus: "synced" | "pending" | "failed" | "not_selected";
  lastSyncAt: number | null;
}

export interface GatewayLinkSkillSync {
  skillName: string;
  skillVersion: string | null;
  syncStatus: "synced" | "pending" | "failed" | "not_selected";
  lastSyncAt: number | null;
}

export interface GatewayLinkPolicy {
  id: string;
  linkId: string;
  routeStrategy: "round_robin" | "least_latency" | "weighted";
  modelFallbackEnabled: boolean;
  globalRpm: number | null;
  perModelRpm: number | null;
  tokenLimitPerMinute: number | null;
  keyRotationStrategy: "sequential" | "random";
  keyFailoverEnabled: boolean;
}

export interface CreateGatewayLinkInput {
  name: string;
  linkType: GatewayLinkType;
  endpoint: string;
  apiKeyId?: string | null;
  apiKey?: string | null;
  autoSyncModels?: boolean;
  autoSyncSkills?: boolean;
}

export interface GatewayLinkActivity {
  id: string;
  linkId: string;
  activityType: string;
  description: string | null;
  createdAt: number;
}

// === UI State ===
export type BuiltinPageKey =
  // ── 能力域聚合入口（对应 domainMeta.ts 的 8 个业务域） ──
  | "general"
  | "finance"
  | "automation"
  | "devops"
  | "data_analysis"
  | "content_creation"
  | "ai_media"
  | "communication"
  // ── 通用功能 ──
  | "chat"
  | "dashboard"
  | "knowledge"
  | "memory"
  | "link"
  | "gateway"
  | "files"
  | "terminal"
  | "workflow"
  | "dynamic-ui"
  | "settings"
  | "marketplace"
  | "wiki"
  | "multi-agent"
  // ── 金融域导航项 ──
  | "finance-investment"
  | "finance-analysis"
  | "finance-accounting"
  // ── 自动化域导航项 ──
  | "automation-operations"
  | "automation-sales"
  | "automation-projects"
  | "automation-consulting"
  | "automation-ecommerce"
  // ── 运维域导航项 ──
  | "devops-software"
  | "devops-security"
  // ── 数据分析域导航项 ──
  | "data-geospatial"
  | "data-ai-research"
  // ── 内容创作域导航项 ──
  | "content-media"
  | "content-design"
  | "content-education"
  // ── AI 媒体域导航项 ──
  | "ai-media-game"
  // ── 旧 key（保留作兼容重定向） ──
  | "invest"
  | "workspace"
  | "stock-analysis"
  | "screener"
  | "watchlist"
  | "portfolio"
  | "paper-portfolio"
  | "market-mainline"
  | "screenshot-diagnosis"
  | "trade"
  | "backtest"
  | "compare"
  | "scheduled-analysis"
  | "pipeline"
  | "quant"
  | "replay-workbench"
  | "cross-market"
  | "opc"
  | "opc-industry-ai-research"
  | "opc-industry-software-dev"
  | "opc-industry-finance-invest"
  | "opc-industry-sales-growth"
  | "opc-industry-content-media"
  | "opc-industry-industry-consulting"
  | "opc-industry-accounting"
  | "opc-industry-ecommerce"
  | "opc-industry-education"
  | "opc-industry-design"
  | "opc-industry-project-management"
  | "opc-industry-security"
  | "opc-industry-geospatial"
  | "opc-industry-game-dev"
  | "opc-industries"
  | "multi-agent";
export type PageKey = BuiltinPageKey | string;
export type SettingsSection =
  | "providers"
  | "defaultModel"
  | "conversationSettings"
  | "general"
  | "display"
  | "proxy"
  | "shortcuts"
  | "data"
  | "database"
  | "storage"
  | "scheduler"
  | "backup"
  | "about"
  | "searchProviders"
  | "localTools"
  | "mcpServers"
  | "tools"
  | "userProfile"
  | "skillsHub"
  | "dashboardPlugins"
  | "webhooks"
  | "messageChannels"
  | "advanced"
  | "promptTemplates"
  | "acp"
  | "evolution"
  | "persona"
  | "proactiveBehavior"
  | "cloudWorkspace"
  | "plugins"
  | "notificationCenter"
  | "imageGen"
  | "theme"
  | "animations"
  | "cron"
  | "dynamicPages"
  | "readingList"
  | "paperOverview"
  | "knowledgeGraph"
  | "gateway"
  | string;

// === Generated Tool ===
export interface GeneratedToolInfo {
  id: string;
  toolName: string;
  originalName: string;
  originalDescription: string;
  createdAt: number;
}

// === Industry Learning Config ===

/** 行业学习配置视图（从后端 YAML 解析而来） */
export interface IndustryLearningConfig {
  version: number;
  industryId: string;
  industryName: string;
  reflectionEnabled: boolean;
  evolutionEnabled: boolean;
  codeEvolverEnabled: boolean;
  selfImprovementEnabled: boolean;
  reinforcementLearningEnabled: boolean;
  configPath: string;
}

/** 行业学习配置列表项（用于列表展示） */
export interface IndustryLearningConfigSummary {
  version: number;
  industryId: string;
  industryName: string;
  reflectionEnabled: boolean;
  evolutionEnabled: boolean;
  codeEvolverEnabled: boolean;
  selfImprovementEnabled: boolean;
  reinforcementLearningEnabled: boolean;
  configPath: string;
}

/** 反思请求参数 */
export interface ReflectOnWorkflowParams {
  industryId: string;
  workflowId: string;
  workflowResult: Record<string, unknown>;
}

/** 进化请求参数 */
export interface EvolveWorkflowParams {
  industryId: string;
  workflowId: string;
  reason: string;
}

/** 自我改进请求参数 */
export interface RunSelfImprovementParams {
  industryId: string;
  target: string;
}

// === Reinforcement Learning ===

/** RL 经验记录 — 单次工作流执行的经验数据 */
export interface RLExperience {
  id: string;
  industryId: string;
  workflowId: string;
  timestampMs: number;
  qualityScore: number;
  efficiencyScore: number;
  costScore: number;
  innovationScore: number;
  satisfactionScore: number;
  totalReward: number;
  stepCount: number;
  success: boolean;
  metadata: Record<string, unknown>;
}

/** RL 策略优化结果 */
export interface RLPolicyUpdate {
  industryId: string;
  experiencesUsed: number;
  avgReward: number;
  rewardTrend: "improving" | "declining" | "stable" | string;
  suggestedAdjustments: string[];
  qualityWeightsOptimized?: Array<[string, number]>;
  reflectionThreshold?: number;
  evolutionTriggerAdjusted?: boolean;
}

/** RL 经验池统计 */
export interface ExperiencePoolStats {
  totalExperiences: number;
  industryCount: number;
  oldestTimestampMs?: number;
  newestTimestampMs?: number;
  avgReward: number;
  successRate: number;
}

/** RL 奖励权重配置 */
export interface RewardWeightConfig {
  quality: number;
  efficiency: number;
  cost: number;
  innovation: number;
  satisfaction: number;
}

/** 强化学习配置 */
export interface ReinforcementLearningConfig {
  enabled: boolean;
  reward_model?: string;
  auto_train_threshold: number;
  learning_rate: number;
  gamma: number;
  epsilon: number;
  reward_weights: RewardWeightConfig;
  optimization_goals: string[];
}

/** RL 经验记录请求参数 */
export interface RecordRLExperienceParams {
  industryId: string;
  workflowId: string;
  qualityScore: number;
  workflowResult: Record<string, unknown>;
}

/** RL 策略优化请求参数 */
export interface TriggerRLOptimizationParams {
  industryId: string;
}

/** 自动学习闭环触发结果 */
export interface AutoLearningResult {
  reflection: {
    status: "success" | "skipped" | "failed";
    qualityScore?: number;
    message?: string;
  };
  evolution?: {
    status: "success" | "skipped" | "failed";
    reason?: string;
    message?: string;
  };
  selfImprovement?: {
    status: "success" | "skipped" | "failed";
    target?: string;
    message?: string;
  };
  reinforcementLearning?: {
    status: "success" | "skipped" | "failed";
    experienceRecorded?: boolean;
    poolSize?: number;
    policyOptimized?: boolean;
    message?: string;
  };
  triggeredAt: number;
}

// === Files Module ===
export type FileCategory = "images" | "files" | "backups";

export type FileSortKey = "createdAt" | "size" | "name";

export interface FileRow {
  id: string;
  name: string;
  path: string;
  storagePath?: string;
  size?: number;
  createdAt?: string;
  category?: FileCategory;
  hasThumbnail?: boolean;
  previewUrl?: string;
  missing?: boolean;
}

export interface FilesPageEntry {
  id: string;
  sourceKind: string;
  category: FileCategory;
  displayName: string;
  path: string;
  storagePath?: string | null;
  sizeBytes: number;
  createdAt: string;
  missing: boolean;
  previewUrl?: string | null;
}

// ── Skills ─────────────────────────────────────────────────────────────
export interface Skill {
  name: string;
  description: string;
  author?: string;
  version?: string;
  source: "builtin" | "axagent" | "claude" | "agents" | "project";
  sourcePath: string;
  enabled: boolean;
  hasUpdate: boolean;
  userInvocable: boolean;
  argumentHint?: string;
  whenToUse?: string;
  group?: string;
  manifest?: SkillManifest;
}

export interface SkillDetail {
  info: Skill;
  content: string;
  files: string[];
  manifest?: SkillInstallMeta;
}

export interface SkillInstallMeta {
  sourceKind: string;
  sourceRef?: string;
  branch?: string;
  commit?: string;
  installedAt: string;
  installedVia?: string;
}

export interface MarketplaceSkill {
  name: string;
  description: string;
  repo: string;
  stars: number;
  installs: number;
  installed: boolean;
  hasUpdate?: boolean;
  currentVersion?: string;
  latestVersion?: string;
  categories?: string[];
  tags?: string[];
}

export interface SkillUpdateInfo {
  name: string;
  currentCommit: string;
  latestCommit: string;
  sourceRef: string;
  currentVersion?: string;
  latestVersion?: string;
}

export interface SkillProposal {
  taskDescription: string;
  suggestedName: string;
  suggestedContent: string;
  confidence: number;
  triggerEvent: string;
  similarSkills: string[];
}

export interface SkillCreateCheckResult {
  hasSimilar: boolean;
  similarSkills: SkillSimilarInfo[];
  canCreate: boolean;
  message: string;
}

export interface SkillSimilarInfo {
  id: string;
  name: string;
  description: string;
  version: string;
  scenarios: string[];
  successRate: number;
  similarityScore: number;
}

// ── Learning Graph Types ──

export type NodeKind = "skill" | "memory" | "insight" | "entity";

export interface GraphNode {
  id: string;
  label: string;
  kind: NodeKind;
  category: string;
  timestampMs: number;
  useCount: number;
  state: string;
  detail: string | null;
}

export interface GraphEdge {
  source: string;
  target: string;
  weight: number;
  relation: string;
}

export interface CategoryCount {
  category: string;
  count: number;
}

export interface GraphStats {
  totalSkills: number;
  totalMemories: number;
  totalInsights: number;
  totalEntities: number;
  totalEdges: number;
  linkedNodes: number;
  categories: CategoryCount[];
}

export interface LearningGraph {
  nodes: GraphNode[];
  edges: GraphEdge[];
  stats: GraphStats;
}

// ── Skill Manifest ──

/** Skill 清单定义（skill-manifest.json 解析后的统一格式） */
export interface SkillManifest {
  name: string;
  version: string;
  description: string;
  author?: string;
  icon?: string;
  dependencies?: Record<string, string>;
  /** 能力声明列表 */
  capabilities?: SkillCapability[];
  /** 权限白名单 */
  permissions?: SkillPermissions;
  /** 生命周期钩子 */
  lifecycle?: SkillLifecycleHooks;
}

/** 权限白名单 */
export interface SkillPermissions {
  commands?: string[];
  events?: string[];
  /**
   * 允许读取的 Zustand Store 字段路径列表。
   * 格式："storeName" (整个 store) 或 "storeName:fieldPath" (特定字段)。
   * 示例：["preference:theme", "preference:language", "ui"]
   */
  storeRead?: string[];
  /**
   * 允许写入的 Zustand Store 字段路径列表。
   * 格式同 storeRead。仅声明 "storeName" 时允许写入整个 store。
   */
  storeWrite?: string[];
  navigate?: string[];
  network?: string[];
  filesystem?: { read?: string[]; write?: string[] };
  tools?: string[];
}

/** 生命周期钩子 */
export interface SkillLifecycleHooks {
  onInstall?: SkillCommandAction[];
  onEnable?: SkillCommandAction[];
  onDisable?: SkillCommandAction[];
  onUninstall?: SkillCommandAction[];
}

// ── Skill Handler ──

export interface SkillHandler {
  mode: "declarative" | "agentic";
  description?: string;
  actions?: SkillCommandAction[];
  promptTemplate?: string;
  contextGatherer?: SkillContextGatherer;
  resultHandler?: SkillResultHandler;
}

// ── Navigation（旧 UI 类型已移除，统一使用 SkillManifest.capabilities） ──

export type NavPosition = "Top" | "Bottom";

// ── SkillCommandAction (声明式 + Agentic) ──

export type SkillCommandAction = DeclarativeAction | AgenticAction;

export interface DeclarativeAction {
  mode: "declarative";
  action: DeclarativeActionType;
}

export type DeclarativeActionType =
  | { type: "invoke"; command: string; args?: Record<string, unknown> }
  | { type: "navigate"; path: string }
  | { type: "emit"; event: string; payload?: unknown }
  | {
    type: "store";
    operation: "get" | "set" | "update";
    storeName: string;
    payload?: unknown;
  }
  | { type: "function"; name: string; args?: unknown[] }
  | { type: "handler"; name: string; args?: Record<string, unknown> }
  | { type: "chain"; actions: DeclarativeActionType[] }
  | { type: "update-schema"; schemaId: string; operation: string; path?: string; newSchema?: unknown };

export interface AgenticAction {
  mode: "agentic";
  prompt: string;
  skillName?: string;
  context?: SkillContextGatherer;
  resultHandler?: SkillResultHandler;
}

export interface SkillResultHandler {
  type: "store" | "emit" | "navigate" | "stream";
  target: string;
}

export interface SkillContextGatherer {
  includeConversation?: boolean;
  includeFiles?: boolean;
  includeSelection?: boolean;
}

export type SkillComponentType = "Sandbox" | "Markdown";

// ── Panels ──

export type UIPanelPosition = "Main" | "Sidebar" | "Header" | "Footer";

export type UIPanelSize = "Small" | "Medium" | "Large" | "FullWidth";

// ── Chat Command Args ──

// ── Skill Capability 类型（Capability-based Manifest） ──────────────

export type SkillCapability =
  | SkillPageCapability
  | SkillPanelCapability
  | SkillToolbarCapability
  | SkillChatCommandCapability
  | SkillStatusBarCapability
  | SkillNavigationCapability
  | SkillSettingsCapability;

export interface SkillPageCapability {
  type: "page";
  id: string;
  title: string;
  componentType: "Sandbox" | "Markdown";
  componentConfig: {
    entry: string;
    props?: Record<string, unknown>;
    layout?: "default" | "fullscreen" | "sidebar";
  };
  icon?: string;
}

export interface SkillPanelCapability {
  type: "panel";
  id: string;
  title: string;
  componentType: "Sandbox" | "Markdown";
  componentConfig: {
    entry: string;
    props?: Record<string, unknown>;
  };
  position: "Main" | "Sidebar" | "Header" | "Footer";
  size?: "Small" | "Medium" | "Large" | "FullWidth";
  collapsible?: boolean;
  defaultCollapsed?: boolean;
}

export interface SkillToolbarCapability {
  type: "toolbar";
  id: string;
  title?: string;
  icon: string;
  tooltip?: string;
  position: "left" | "right";
  priority?: number;
  onClick: SkillCommandAction[];
  menu?: { label: string; actions: SkillCommandAction[] }[];
}

export interface SkillChatCommandCapability {
  type: "chatCommand";
  id: string;
  title: string;
  commandName: string;
  description: string;
  icon?: string;
  mode: "declarative" | "agentic";
  actions?: SkillCommandAction[];
}

export interface SkillStatusBarCapability {
  type: "statusBar";
  id: string;
  title: string;
  alignment: "left" | "right";
  priority?: number;
  text?: string;
  icon?: string;
  dynamicText?: {
    command: string;
    args?: Record<string, unknown>;
    refreshIntervalMs: number;
    template?: string;
  };
  onClick?: SkillCommandAction[];
}

export interface SkillNavigationCapability {
  type: "navigation";
  id: string;
  title: string;
  icon: string;
  pageId: string;
  position?: number;
}

export interface SkillSettingsCapability {
  type: "settings";
  id: string;
  title: string;
  icon?: string;
  settingsGroup: string;
  componentType: "Sandbox" | "Markdown";
  componentConfig: {
    entry: string;
    props?: Record<string, unknown>;
  };
}

// Phase-2 type modules
export * from "./agent";
export * from "./agentProfile";
export * from "./agentRole";
export * from "./approval";
export * from "./artifact";
export * from "./backup";
export * from "./citation";
export * from "./evaluator";
export * from "./evolution";
export * from "./expert";
export * from "./knowledge";
export * from "./llmWiki";
export * from "./localTool";
export * from "./market-mainline";
export * from "./mcp";
export * from "./memory";
export * from "./multi-agent";
export * from "./nudge";
// Fleet（多办公室 AI 团队）— 与后端 axagent_harness::fleet 一一对应
export * from "./office";
export * from "./paper";
export * from "./paper-portfolio";
export * from "./permission";
export * from "./persona";
export * from "./pipeline";

// === Security Module Types ===
export type { InjectionDetection, InjectionType, SanitizationResult } from "@/lib/security/injectionDetector";

export type {
  PermissionDecision,
  PermissionLevel,
  PermissionPolicy,
  PermissionRequest,
  ResourceType,
} from "@/lib/security/permissionGuard";

export type { OutputRiskLevel, SanitizedOutput } from "@/lib/security/outputSanitizer";
export * from "./capability";
export * from "./narrative";
export * from "./platform";
export * from "./proactive";
export * from "./scheduler";
export * from "./screenshot-diagnosis";
export * from "./search";
export * from "./stock-analysis";
export * from "./style";
export * from "./taskShape";
export * from "./tracer";
export * from "./wiki";
export * from "./workflow";

// ── Workspace / Context Types (merged from workspace.ts) ───────────────
export type ContextSourceType =
  | "attachment"
  | "search"
  | "knowledge"
  | "memory"
  | "wiki"
  | "tool";

export type ContextSource = {
  id: string;
  conversationId: string;
  messageId?: string;
  type: ContextSourceType;
  refId: string;
  title: string;
  enabled: boolean;
  summary?: string;
  /** 多文档协同：限制 RAG 检索范围到这些文档 ID；空数组表示不限制 */
  docIds?: string[];
};

export type ConversationBranch = {
  id: string;
  conversationId: string;
  parentMessageId: string;
  branchLabel: string;
  branchIndex: number;
  comparedMessageIdsJson?: string;
  createdAt: string;
};

export type SearchPolicy = {
  enabled: boolean;
  searchProviderId?: string;
  queryMode: "manual" | "auto";
  resultLimit: number;
};

export type ToolBinding = {
  serverIds: string[];
  defaultTools?: string[];
  approvalMode: "inherit" | "ask" | "allow_safe";
};

export type KnowledgeBinding = {
  knowledgeBaseIds: string[];
  autoAttach: boolean;
};

export type MemoryPolicy = {
  enabled: boolean;
  namespaceId?: string;
  writeBack: boolean;
};

export type ContextToggleState = {
  searchEnabled: boolean;
  searchProviderId?: string;
  enabledKnowledgeBaseIds: string[];
  enabledMcpServerIds: string[];
  enabledWikiIds: string[];
  enabledToolNames?: string[];
  memoryEnabled: boolean;
  memoryNamespaceId?: string;
  memoryWriteBack: boolean;
  disabledContextSourceIds?: string[];
};

export type ConversationWorkspaceSnapshot = {
  searchPolicy: SearchPolicy;
  toolBinding: ToolBinding;
  knowledgeBinding: KnowledgeBinding;
  memoryPolicy: MemoryPolicy;
  toggles: ContextToggleState;
  researchMode: boolean;
  pinnedArtifactIds: string[];
  branches?: ConversationBranch[];
  activeBranchId?: string | null;
};

export type ContextOverrideInput = {
  searchEnabled?: boolean;
  searchProviderId?: string | null;
  enabledKnowledgeBaseIds?: string[];
  enabledMcpServerIds?: string[];
  enabledWikiIds?: string[];
  enabledToolNames?: string[];
  memoryEnabled?: boolean;
  memoryNamespaceId?: string | null;
  memoryWriteBack?: boolean;
  disabledContextSourceIds?: string[];
  researchMode?: boolean;
};

export type CreateConversationInput = {
  title: string;
  providerId: string;
  modelId: string;
  systemPrompt?: string;
  temperature?: number;
  maxTokens?: number;
  topP?: number;
  frequencyPenalty?: number;
  workspaceSnapshot?: ConversationWorkspaceSnapshot;
};

export type WorkspaceUpdateInput = {
  title?: string;
  providerId?: string;
  modelId?: string;
  workspaceSnapshot?: ConversationWorkspaceSnapshot;
  activeBranchId?: string | null;
  activeArtifactId?: string | null;
  researchMode?: boolean;
};

export type SendMessageInput = {
  conversationId: string;
  content: string;
  attachments?: AttachmentInput[];
  contextOverride?: ContextOverrideInput;
};

export type CompareResponsesResult = {
  leftMessage: { id: string; content: string };
  rightMessage: { id: string; content: string };
};

// ── Tool Dependencies ─────────────────────────────────────────────────
export type ToolDependencyStatus =
  | "satisfied"
  | "auto_installable"
  | "manual_installable"
  | "needs_generation";

export interface ToolDependency {
  name: string;
  toolType: string;
  status: ToolDependencyStatus;
  sourceInfo?: string;
  installInstructions?: string;
  configRequirements?: string;
}

// ── Decomposition ─────────────────────────────────────────────────────
export interface DecompositionPreview {
  toolDependencies: ToolDependency[];
  workflowNodes: unknown;
  workflowEdges: unknown;
  originalSource: {
    market: string;
    repo?: string;
    version?: string;
  };
  cacheId: string;
}

// ── Work Engine ───────────────────────────────────────────────────────
export type ExecutionStatus =
  | "running"
  | "paused"
  | "completed"
  | "partially_completed"
  | "failed"
  | "cancelled";

export interface NodeExecutionRecord {
  nodeId: string;
  nodeType: string;
  nodeName: string | null;
  status: string;
  input: unknown;
  output: unknown;
  executionTimeMs: number | null;
  error: string | null;
  startedAt: number;
  completedAt: number | null;
  parentExecutionId: string | null;
  subWorkflowId: string | null;
}

export interface ExecutionStatusResponse {
  executionId: string;
  workflowId: string;
  status: ExecutionStatus;
  currentNodeId: string | null;
  totalTimeMs: number;
  nodeCount: number;
  nodeRecords: NodeExecutionRecord[];
  variables: Record<string, unknown>;
  parentExecutionId: string | null;
}

export interface ExecutionSummary {
  id: string;
  workflowId: string;
  status: string;
  totalTimeMs: number | null;
  createdAt: number;
}

// ── Plan Mode (Agent Work Strategy) ──────────────────────────────────
export type PlanStepStatus =
  | "pending"
  | "approved"
  | "rejected"
  | "running"
  | "completed"
  | "error";

export interface PlanStep {
  id: string;
  title: string;
  description: string;
  status: PlanStepStatus;
  /** Estimated tools that will be used for this step */
  estimatedTools?: string[];
  /** Result summary after completion */
  result?: string | null;
}

export type PlanStatus =
  | "draft"
  | "reviewing"
  | "approved"
  | "executing"
  | "completed"
  | "partial"
  | "cancelled";

export interface Plan {
  id: string;
  conversationId: string;
  /** The user message that triggered this plan generation */
  userMessageId: string;
  title: string;
  steps: PlanStep[];
  status: PlanStatus;
  isActive: boolean;
  /** The work_strategy that was active when this plan was created, for restoration context */
  createdUnderStrategy?: "direct" | "plan";
  createdAt: number;
  updatedAt: number;
}

export interface PlanGeneratedEvent {
  conversationId: string;
  plan: Plan;
}

export interface PlanStepUpdateEvent {
  conversationId: string;
  planId: string;
  stepId: string;
  status: PlanStepStatus;
  result?: string | null;
}

export interface PlanExecutionCompleteEvent {
  conversationId: string;
  planId: string;
  status: "completed" | "cancelled";
}

export interface PlanGenerateRequest {
  conversationId: string;
  content: string;
}

export interface PlanExecuteRequest {
  conversationId: string;
  planId: string;
  /** Optional: execute only specific step IDs, otherwise all approved steps */
  stepIds?: string[];
}

export interface PlanModifyStepRequest {
  planId: string;
  stepId: string;
  title?: string;
  description?: string;
  approved?: boolean;
}

export interface PromptTemplate {
  id: string;
  name: string;
  description?: string;
  content: string;
  variablesSchema?: string;
  version: number;
  isActive: boolean;
  abTestEnabled: boolean;
  abTestVariant?: string;
  category?: string;
  tags?: string[];
  author?: string;
  source?: string;
  sourceType?: string;
  format?: string;
  metadataJson?: string;
  usageCount: number;
  isFavorite: boolean;
  createdAt: number;
  updatedAt: number;
}

export interface CreatePromptTemplateInput {
  name: string;
  description?: string;
  content: string;
  variablesSchema?: string;
  category?: string;
  tags?: string[];
  author?: string;
  source?: string;
  sourceType?: string;
  format?: string;
  metadataJson?: string;
}

export interface UpdatePromptTemplateInput {
  name?: string;
  description?: string;
  content?: string;
  variablesSchema?: string;
  isActive?: boolean;
  abTestEnabled?: boolean;
  category?: string;
  tags?: string[];
  author?: string;
  source?: string;
  sourceType?: string;
  format?: string;
  metadataJson?: string;
  isFavorite?: boolean;
}

export interface PromptTemplateVersion {
  id: string;
  templateId: string;
  version: number;
  content: string;
  variablesSchema?: string;
  category?: string;
  tags?: string[];
  author?: string;
  source?: string;
  changelog?: string;
  createdAt: number;
}

export interface ImportPromptTemplateInput {
  name: string;
  description?: string;
  content: string;
  variablesSchema?: string;
  category?: string;
  tags?: string[];
  author?: string;
  source?: string;
  sourceType?: string;
  format?: string;
  metadataJson?: string;
}

export interface ImportPromptResult {
  imported: PromptTemplate[];
  skipped: string[];
  errors: string[];
}

export interface ImportFromUrlInput {
  url: string;
  categoryFilter?: string;
  overwriteExisting?: boolean;
}

export type ExportPromptFormat = "json" | "yaml" | "markdown";

export * from "./wiki";

// === Plugin System ===

export interface PluginSummaryDto {
  id: string;
  name: string;
  version: string;
  description: string;
  kind: "builtin" | "bundled" | "external" | "openclaw";
  enabled: boolean;
  tools: string[];
  mcpServers: string[];
  skills: string[];
}

export interface PluginManifestDto {
  name: string;
  version: string;
  description: string;
  permissions: string[];
  defaultEnabled: boolean;
  hooks: Record<string, string[]>;
  tools: { name: string; description: string }[];
  mcpServers: { name: string; command: string }[];
  skills: { name: string; path: string }[];
  capabilities: {
    seam: string;
    capabilityType: string;
    version: string;
    description: string;
  }[];
}

export interface InstallOutcomeDto {
  pluginId: string;
  version: string;
  installPath: string;
}

export interface UpdateOutcomeDto {
  pluginId: string;
  oldVersion: string;
  newVersion: string;
  installPath: string;
}

// === Session Share ===

export interface SharePermissions {
  allowTerminalAccess: boolean;
  allowFileAccess: boolean;
  allowModelAccess: boolean;
  requireApprovalForActions: boolean;
  maxParticipants: number;
}

export interface ShareSessionInfo {
  sessionId: string;
  inviteCode: string;
  conversationId: string;
  permissions: SharePermissions;
  participantCount: number;
  createdAt: number;
}

// === Device Sync Types ===

export type {
  AuditAction,
  AuditLogEntry,
  ChangeLogEntry,
  ChangeOperation,
  ConflictInfo,
  ConflictResolutionStrategy,
  DeviceInfo,
  DevicePermissions,
  DeviceSyncStatus,
  DeviceType,
  EncryptedSyncData,
  EncryptionAlgorithm,
  EncryptionState,
  EntityType,
  KeyDerivation,
  PairingCode,
  PairingRequest,
  PairingResponse,
  PermissionUpdate,
  RealtimePushState,
  SyncDirection,
  SyncEncryptionConfig,
  SyncHistoryEntry,
  SyncPolicy,
  SyncPolicyUpdate,
  SyncResult,
  SyncSignal,
  SyncSignalResponse,
  SyncType,
  TrustLevel,
  VersionVectorEntry,
  WebSocketStatus,
} from "./deviceSync";

export interface ShareParticipant {
  id: string;
  name: string;
  joinedAt: number;
}

// ── Dynamic UI ──
export type {
  ComponentRegistryEntry,
  ConditionalDisplay,
  ConditionalRule,
  CreateDynamicUISchemaParams,
  DataSourceConfig,
  DynamicAction,
  DynamicComponentType,
  DynamicImportance,
  DynamicStatus,
  DynamicUIFormDataRecord,
  DynamicUIPinRecord,
  DynamicUIProps,
  DynamicUISchemaRecord,
  DynamicUISchemaVersion,
  EventHandler,
  ListVersionsResponse,
  PinDynamicUISchemaParams,
  SaveDynamicUIFormDataParams,
  SchemaValidationError,
  SchemaValidationResult,
  UISchema,
  UpdateDynamicUIPinParams,
  UpdateDynamicUISchemaParams,
} from "./dynamicUI";
export { COMPONENT_REQUIRED_PROPS, VALID_DYNAMIC_COMPONENT_TYPES } from "./dynamicUI";

// ── AxInvest quant types ──
export type {
  BacktestResult,
  BacktestRunRequest,
  BacktestRunResponse,
  EquityPoint,
  MetricsCompareResponse,
  QuantBacktestConfig,
  QuantBacktestSignal,
  QuantBacktestTrade,
  QuantFill,
  QuantOrder,
  QuantOrderType,
  QuantRun,
  RegisterRhaiRequest,
  StrategyMeta,
  StrategyType,
  TradeSide,
} from "./quant";
export { BUILTIN_STRATEGY_IDS, DEFAULT_RHAI_TEMPLATE, DEFAULT_STRATEGY_PARAMS } from "./quant";

export interface DashboardStats {
  totalConversations: number;
  totalMessages: number;
  totalPromptTokens: number;
  totalCompletionTokens: number;
  totalTokens: number;
  totalAgentSessions: number;
  completedAgentSessions: number;
  failedAgentSessions: number;
  totalAgentTokens: number;
  totalCostUsd: number;
  totalToolCalls: number;
  /** 今日（本地时区）消息数 */
  todayMessages: number;
  /** 今日（本地时区）输入 token 数 */
  todayPromptTokens: number;
  /** 今日（本地时区）输出 token 数 */
  todayCompletionTokens: number;
  /** 今日（本地时区）总 token 数 */
  todayTokens: number;
}

// === Local Model (llama.cpp) Management ===

export interface LocalModelInfo {
  id: string;
  nEmbd: number | null;
  nCtx: number | null;
  nCtxTrain: number | null;
  nParams: number | null;
  sizeBytes: number | null;
  ftype: string | null;
  nVocab: number | null;
}

export interface LocalModelProps {
  modelPath: string | null;
  modelAlias: string | null;
  modelFtype: string | null;
  nCtx: number | null;
  totalSlots: number | null;
}

export interface LocalModelStatus {
  running: boolean;
  health: string;
  pid: number | null;
  processName: string | null;
  memoryMb: number | null;
  baseUrl: string;
  managed: boolean;
  model: LocalModelInfo | null;
  props: LocalModelProps | null;
}

export interface LocalModelStartConfig {
  serverExe: string;
  modelPath: string;
  host: string;
  port: number;
  alias?: string | null;
  nCtx?: number | null;
  nGpuLayers?: number | null;
  embeddingMode?: boolean;
  extraArgs: string[];
}

export interface EmbedTestResult {
  dimensions: number;
  promptTokens: number | null;
  elapsedMs: number;
  preview: number[];
}

// === Local Model Download ===

export interface LocalFileModel {
  filename: string;
  sizeBytes: number;
  modifiedAt: number | null;
  modelType: string;
  isDownloading: boolean;
  downloadBytes: number;
}

export interface DownloadTaskInfo {
  filename: string;
  downloadedBytes: number;
  totalBytes: number;
  status: "downloading" | "done" | "failed";
  error: string | null;
}

export interface DownloadRequest {
  filename: string;
  hfRepo?: string | null;
  directUrl?: string | null;
}

export interface PresetModelDto {
  filename: string;
  hfRepo: string | null;
  directUrl: string | null;
  displayName: string;
  sizeBytes: number;
  modelType: string;
  isDownloaded: boolean;
}

// === llama.cpp 安装管理 ===

export interface LlamaCppVersionInfo {
  tag: string;
  name: string;
  publishedAt: string;
  downloadUrl: string;
  fileName: string;
  fileSize: number | null;
}

export interface LlamaCppInstallStatus {
  installed: boolean;
  version: string | null;
  installPath: string | null;
  executablePath: string | null;
  isDownloading: boolean;
  downloadProgress: number | null;
  downloadError: string | null;
}

// === 技能学习闭环（Hermes 借鉴） ===

export type {
  ApprovalStatus,
  LearnSkillInput,
  LearnSkillResult,
  MemoryWriteApprovalConfig,
  PendingMemoryWrite,
  PendingOperationType,
  PendingSkillOperation,
  RiskLevel,
  SkillLearningConfig,
} from "./skillLearning";

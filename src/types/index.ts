// === Provider System ===
export type ProviderType = "openai" | "openai_responses" | "anthropic" | "gemini" | "openclaw" | "hermes" | "ollama";

export interface ProviderConfig {
  id: string;
  name: string;
  provider_type: ProviderType;
  api_host: string;
  api_path: string | null;
  enabled: boolean;
  models: Model[];
  keys: ProviderKey[];
  proxy_config: ProviderProxyConfig | null;
  custom_headers: string | null;
  icon: string | null;
  builtin_id: string | null;
  sort_order: number;
  created_at: number;
  updated_at: number;
}

export interface ProviderKey {
  id: string;
  provider_id: string;
  key_encrypted: string;
  key_prefix: string;
  enabled: boolean;
  last_validated_at: number | null;
  last_error: string | null;
  rotation_index: number;
  created_at: number;
}

export interface ProviderProxyConfig {
  proxy_type: string | null;
  proxy_address: string | null;
  proxy_port: number | null;
}

export interface CreateProviderInput {
  name: string;
  provider_type: ProviderType;
  api_host: string;
  api_path?: string | null;
  enabled: boolean;
}

export interface UpdateProviderInput {
  name?: string;
  provider_type?: ProviderType;
  api_host?: string;
  api_path?: string | null;
  enabled?: boolean;
  proxy_config?: ProviderProxyConfig;
  custom_headers?: string | null;
  icon?: string | null;
  sort_order?: number;
}

// === Model System ===
export type ModelCapability = "TextChat" | "Vision" | "FunctionCalling" | "Reasoning" | "RealtimeVoice";
export type ModelType = "Chat" | "Voice" | "Embedding";

export interface Model {
  provider_id: string;
  model_id: string;
  name: string;
  group_name?: string | null;
  model_type: ModelType;
  capabilities: ModelCapability[];
  max_tokens: number | null;
  enabled: boolean;
  param_overrides: ModelParamOverrides | null;
}

export interface ModelParamOverrides {
  temperature?: number;
  max_tokens?: number;
  top_p?: number;
  frequency_penalty?: number;
  use_max_completion_tokens?: boolean;
  no_system_role?: boolean;
  force_max_tokens?: boolean;
  thinking_param_style?: string;
}

// === Conversation & Message ===
export type MessageRole = "system" | "user" | "assistant" | "tool";

export type MessageStatus = "complete" | "partial" | "error" | "cancelled";

export interface ConversationCategory {
  id: string;
  name: string;
  icon_type: string | null;
  icon_value: string | null;
  system_prompt: string | null;
  default_provider_id: string | null;
  default_model_id: string | null;
  default_temperature: number | null;
  default_max_tokens: number | null;
  default_top_p: number | null;
  default_frequency_penalty: number | null;
  sort_order: number;
  is_collapsed: boolean;
  created_at: number;
  updated_at: number;
}

export interface Conversation {
  id: string;
  title: string;
  model_id: string;
  provider_id: string;
  system_prompt: string | null;
  temperature: number | null;
  max_tokens: number | null;
  top_p: number | null;
  frequency_penalty: number | null;
  search_enabled: boolean;
  search_provider_id: string | null;
  thinking_budget: number | null;
  enabled_mcp_server_ids: string[];
  enabled_knowledge_base_ids: string[];
  enabled_memory_namespace_ids: string[];
  is_pinned: boolean;
  is_archived: boolean;
  context_compression: boolean;
  category_id: string | null;
  parent_conversation_id: string | null;
  mode: "chat" | "agent" | "gateway";
  /** Agent work strategy: "direct" = execute immediately, "plan" = generate plan first, await approval, then execute */
  work_strategy?: "direct" | "plan" | null;
  message_count: number;
  created_at: number;
  updated_at: number;
  scenario?: string | null;
  enabled_skill_ids: string[];
  /** Expert role identifier, references ExpertRole.id */
  expert_role_id?: string | null;
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
  conversation_id: string;
  role: MessageRole;
  content: string;
  provider_id: string | null;
  model_id: string | null;
  token_count: number | null;
  prompt_tokens?: number | null;
  completion_tokens?: number | null;
  attachments: Attachment[];
  thinking: string | null;
  tool_calls_json: string | null;
  tool_call_id: string | null;
  created_at: number;
  parent_message_id: string | null;
  version_index: number;
  is_active: boolean;
  status: MessageStatus;
  tokens_per_second?: number | null;
  first_token_latency_ms?: number | null;
  /** Structured content blocks (from agent session ContentBlock). */
  blocks?: ContentBlock[];
}

// ── Content Block (Part-based message model, short-term) ──────────────
export type ContentBlock =
  | { type: "text"; text: string }
  | { type: "tool_use"; id: string; name: string; input: string }
  | { type: "tool_result"; tool_use_id: string; tool_name: string; output: string; is_error: boolean };

export interface MessagePage {
  messages: Message[];
  has_older: boolean;
  oldest_message_id: string | null;
  total_active_count: number;
}

export interface ConversationStats {
  total_messages: number;
  total_user_messages: number;
  total_assistant_messages: number;
  total_prompt_tokens: number;
  total_completion_tokens: number;
  total_tokens: number;
  avg_tokens_per_second: number | null;
  avg_first_token_latency_ms: number | null;
  avg_response_time_ms: number | null;
}

export interface Attachment {
  id: string;
  file_type: string;
  file_name: string;
  file_path: string;
  file_size: number;
  data?: string;
}

export interface AttachmentInput {
  file_name: string;
  file_type: string;
  file_size: number;
  data: string;
}

export interface ConversationSearchResult {
  conversation: Conversation;
  matched_message_preview: string | null;
}

// P2: Cross-session FTS5 search result
export interface SessionSearchResult {
  conversation_id: string;
  conversation_title: string;
  role: string;
  snippet: string;
  rank: number;
}

export interface ConversationSummary {
  id: string;
  conversation_id: string;
  summary_text: string;
  compressed_until_message_id: string | null;
  token_count: number | null;
  model_used: string | null;
  created_at: number;
  updated_at: number;
}

export interface UpdateConversationInput {
  title?: string;
  provider_id?: string;
  model_id?: string;
  is_pinned?: boolean;
  is_archived?: boolean;
  system_prompt?: string;
  temperature?: number | null;
  max_tokens?: number | null;
  top_p?: number | null;
  frequency_penalty?: number | null;
  search_enabled?: boolean;
  search_provider_id?: string | null;
  thinking_budget?: number | null;
  enabled_mcp_server_ids?: string[];
  enabled_knowledge_base_ids?: string[];
  enabled_memory_namespace_ids?: string[];
  context_compression?: boolean;
  category_id?: string | null;
  parent_conversation_id?: string | null;
  mode?: "chat" | "agent" | "gateway";
  work_strategy?: "direct" | "plan" | null;
  scenario?: string | null;
  enabled_skill_ids?: string[];
  expert_role_id?: string | null;
}

// === Gateway System ===
export interface GatewayStatus {
  is_running: boolean;
  listen_address: string;
  port: number;
  ssl_enabled: boolean;
  started_at: number | null;
  /** HTTPS listener port; `null` when SSL is disabled or not yet started. */
  https_port: number | null;
  /** When `true` the gateway redirects all HTTP traffic to HTTPS. */
  force_ssl: boolean;
}

export interface GatewayKey {
  id: string;
  name: string;
  key_hash: string;
  key_prefix: string;
  enabled: boolean;
  created_at: number;
  last_used_at: number | null;
  has_encrypted_key: boolean;
}

export interface CreateGatewayKeyResult {
  gateway_key: GatewayKey;
  plain_key: string;
}

export interface GatewayMetrics {
  total_requests: number;
  total_tokens: number;
  total_request_tokens: number;
  total_response_tokens: number;
  active_connections: number;
  today_requests: number;
  today_tokens: number;
  today_request_tokens: number;
  today_response_tokens: number;
}

export interface UsageByKey {
  key_id: string;
  key_name: string;
  request_count: number;
  token_count: number;
  request_tokens: number;
  response_tokens: number;
}

export interface UsageByProvider {
  provider_id: string;
  provider_name: string;
  request_count: number;
  token_count: number;
  request_tokens: number;
  response_tokens: number;
}

export interface UsageByDay {
  date: string;
  request_count: number;
  token_count: number;
  request_tokens: number;
  response_tokens: number;
}

export interface ConnectedProgram {
  key_id: string;
  key_name: string;
  key_prefix: string;
  today_requests: number;
  today_tokens: number;
  today_request_tokens: number;
  today_response_tokens: number;
  last_active_at: number | null;
  is_active: boolean;
}

export interface GatewayStats {
  total_requests: number;
  active_connections: number;
  uptime_seconds: number;
  requests_per_minute: number;
}

export interface GatewaySettings {
  listen_address: string;
  port: number;
  load_balance_strategy: "round_robin";
}

// === Settings ===
export interface AppSettings {
  language: string;
  theme_mode: string;
  theme_preset: string;
  primary_color: string;
  border_radius: number;
  auto_start: boolean;
  show_on_start: boolean;
  minimize_to_tray: boolean;
  font_size: number;
  font_weight: number;
  font_family: string;
  code_font_family: string;
  bubble_style: string;
  code_theme: string;
  code_theme_light: string;
  default_provider_id: string | null;
  default_model_id: string | null;
  default_temperature: number | null;
  default_max_tokens: number | null;
  default_top_p: number | null;
  default_frequency_penalty: number | null;
  default_context_count: number | null;
  title_summary_provider_id: string | null;
  title_summary_model_id: string | null;
  title_summary_temperature: number | null;
  title_summary_max_tokens: number | null;
  title_summary_top_p: number | null;
  title_summary_frequency_penalty: number | null;
  title_summary_context_count: number | null;
  title_summary_prompt: string | null;
  compression_provider_id: string | null;
  compression_model_id: string | null;
  compression_temperature: number | null;
  compression_max_tokens: number | null;
  compression_top_p: number | null;
  compression_frequency_penalty: number | null;
  compression_prompt: string | null;
  proxy_type: string | null;
  proxy_address: string | null;
  proxy_port: number | null;
  global_shortcut: string;
  shortcut_toggle_current_window: string;
  shortcut_toggle_all_windows: string;
  shortcut_close_window: string;
  shortcut_new_conversation: string;
  shortcut_open_settings: string;
  shortcut_toggle_model_selector: string;
  shortcut_fill_last_message: string;
  shortcut_clear_context: string;
  shortcut_clear_conversation_messages: string;
  shortcut_toggle_gateway: string;
  shortcut_toggle_mode: string;
  shortcut_show_quick_bar: string;
  gateway_auto_start: boolean;
  gateway_listen_address: string;
  gateway_port: number;
  gateway_ssl_enabled: boolean;
  gateway_ssl_mode: string;
  gateway_ssl_cert_path: string | null;
  gateway_ssl_key_path: string | null;
  gateway_ssl_port: number;
  gateway_force_ssl: boolean;
  // Desktop integration
  always_on_top?: boolean;
  tray_enabled?: boolean;
  global_shortcuts_enabled?: boolean;
  shortcut_registration_logs_enabled?: boolean;
  shortcut_trigger_toast_enabled?: boolean;
  notifications_enabled?: boolean;
  mini_window_enabled?: boolean;
  start_minimized?: boolean;
  close_to_tray?: boolean;
  notify_backup?: boolean;
  notify_import?: boolean;
  notify_errors?: boolean;
  // Auto-backup settings
  backup_dir?: string | null;
  auto_backup_enabled?: boolean;
  auto_backup_interval_hours?: number;
  auto_backup_max_count?: number;
  // WebDAV sync settings
  webdav_host?: string | null;
  webdav_username?: string | null;
  webdav_path?: string | null;
  webdav_accept_invalid_certs?: boolean;
  webdav_sync_enabled?: boolean;
  webdav_sync_interval_minutes?: number;
  webdav_max_remote_backups?: number;
  webdav_include_documents?: boolean;
  // S3 sync settings
  s3_endpoint?: string | null;
  s3_region?: string | null;
  s3_bucket?: string | null;
  s3_access_key_id?: string | null;
  s3_root?: string | null;
  s3_use_path_style?: boolean;
  s3_sync_enabled?: boolean;
  s3_sync_interval_minutes?: number;
  s3_max_remote_backups?: number;
  s3_include_documents?: boolean;
  /** Closed-loop nudge scheduler enabled */
  closed_loop_enabled?: boolean;
  /** Closed-loop nudge interval in minutes (default 5) */
  closed_loop_interval_minutes?: number;
  last_selected_conversation_id?: string | null;
  /** Custom documents root override (overrides ~/Documents/axagent/) */
  documents_root_override?: string | null;
  /** Auto update check interval in minutes (default 60, min 1) */
  update_check_interval?: number;
  /** Global system prompt fallback — used when a conversation has no custom system prompt */
  default_system_prompt?: string | null;
  /** Chat minimap / navigation overlay */
  chat_minimap_enabled?: boolean;
  chat_minimap_style?: "faq" | "sticky";
  /** Multi-model response display mode */
  multi_model_display_mode?: "tabs" | "side-by-side" | "stacked";
  /** Render user messages as Markdown (like AI messages). Default: false */
  render_user_markdown?: boolean;
  /** Default workspace directory for new sessions when not manually set */
  default_workspace_dir?: string | null;
  /** Enable screen perception and vision-based UI control */
  screen_perception_enabled?: boolean;
  /** Enable RL optimizer for tool selection and task strategies */
  rl_optimizer_enabled?: boolean;
  /** Enable LoRA fine-tuning for custom model adaptation */
  lora_finetune_enabled?: boolean;
  /** Enable proactive nudge suggestions based on context */
  proactive_nudge_enabled?: boolean;
  /** Enable thought chain visualization for reasoning */
  thought_chain_enabled?: boolean;
  /** Enable automatic error recovery suggestions */
  error_recovery_enabled?: boolean;
}

// === Streaming ===
export interface ChatStreamChunk {
  content: string | null;
  thinking: string | null;
  tool_calls: ToolCall[] | null;
  done: boolean;
  is_final?: boolean | null;
  usage: TokenUsage | null;
}

export interface ChatStreamEvent {
  conversation_id: string;
  message_id: string;
  model_id?: string;
  provider_id?: string;
  chunk: ChatStreamChunk;
}

export interface ChatStreamErrorEvent {
  conversation_id: string;
  message_id: string;
  error: string;
}

export interface TokenUsage {
  prompt_tokens: number;
  completion_tokens: number;
  total_tokens: number;
}

// === Voice ===
export type VoiceSessionState = "Idle" | "Connecting" | "Connected" | "Speaking" | "Listening" | "Disconnecting";

export type AudioEncoding = "Pcm16" | "Opus";

export interface AudioFormat {
  sample_rate: number;
  channels: number;
  encoding: AudioEncoding;
}

export interface RealtimeConfig {
  model_id: string;
  voice: string | null;
  audio_format: AudioFormat;
}

// === Gateway Link (Client-side Gateway Connection) ===
export type GatewayLinkType = "openclaw" | "hermes" | "custom";
export type GatewayLinkStatus = "connected" | "disconnected" | "connecting" | "error";

export interface GatewayLink {
  id: string;
  name: string;
  link_type: GatewayLinkType;
  endpoint: string;
  api_key_id: string | null;
  enabled: boolean;
  status: GatewayLinkStatus;
  error_message: string | null;
  auto_sync_models: boolean;
  auto_sync_skills: boolean;
  last_sync_at: number | null;
  latency_ms: number | null;
  version: string | null;
  created_at: number;
  updated_at: number;
}

export interface GatewayLinkModelSync {
  model_id: string;
  provider_name: string;
  sync_status: "synced" | "pending" | "failed" | "not_selected";
  last_sync_at: number | null;
}

export interface GatewayLinkSkillSync {
  skill_name: string;
  skill_version: string | null;
  sync_status: "synced" | "pending" | "failed" | "not_selected";
  last_sync_at: number | null;
}

export interface GatewayLinkPolicy {
  id: string;
  link_id: string;
  route_strategy: "round_robin" | "least_latency" | "weighted";
  model_fallback_enabled: boolean;
  global_rpm: number | null;
  per_model_rpm: number | null;
  token_limit_per_minute: number | null;
  key_rotation_strategy: "sequential" | "random";
  key_failover_enabled: boolean;
}

export interface CreateGatewayLinkInput {
  name: string;
  link_type: GatewayLinkType;
  endpoint: string;
  api_key_id?: string | null;
  api_key?: string | null;
  auto_sync_models?: boolean;
  auto_sync_skills?: boolean;
}

export interface GatewayLinkActivity {
  id: string;
  link_id: string;
  activity_type: string;
  description: string | null;
  created_at: number;
}

// === UI State ===
export type BuiltinPageKey =
  | "chat"
  | "knowledge"
  | "memory"
  | "link"
  | "gateway"
  | "files"
  | "settings"
  | "skills"
  | "marketplace"
  | "wiki";
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
  | "storage"
  | "scheduler"
  | "backup"
  | "about"
  | "searchProviders"
  | "localTools"
  | "mcpServers"
  | "workflow"
  | "tools"
  | "userProfile"
  | "skillsHub"
  | "dashboardPlugins"
  | "webhooks"
  | "messageChannels"
  | "advanced"
  | "promptTemplates"
  | "acp"
  | string;

// === Generated Tool ===
export interface GeneratedToolInfo {
  id: string;
  toolName: string;
  originalName: string;
  originalDescription: string;
  createdAt: number;
}

// === Files Module ===
export type FileCategory = "images" | "files";

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
  frontend?: SkillFrontendExtension;
}

export interface SkillDetail {
  info: Skill;
  content: string;
  files: string[];
  manifest?: SkillManifest;
}

export interface SkillManifest {
  sourceKind: string;
  sourceRef?: string;
  branch?: string;
  commit?: string;
  installedAt: string;
  installedVia?: string;
  frontend?: SkillFrontendExtension;
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
  task_description: string;
  suggested_name: string;
  suggested_content: string;
  confidence: number;
  trigger_event: string;
  similar_skills: string[];
}

// ── Skill Frontend Extension ──

export interface SkillFrontendExtension {
  navigation: SkillNavItem[];
  pages: SkillPage[];
  commands: SkillUICommand[];
  panels: SkillUIPanel[];
  settingsSections: SkillSettingsSection[];
}

export interface SkillNavItem {
  id: string;
  label: string;
  icon: string;
  path: string;
  position: NavPosition;
  order: number;
}

export type NavPosition = "Top" | "Bottom";

export interface SkillPage {
  id: string;
  path: string;
  title: string;
  componentType: SkillComponentType;
  componentConfig: Record<string, unknown>;
}

export type SkillComponentType = "Html" | "Iframe" | "React" | "WebComponent" | "Markdown";

export interface SkillUICommand {
  id: string;
  label: string;
  category: string;
  icon?: string;
  shortcut?: string;
  action: SkillCommandAction;
}

export type SkillCommandAction =
  | { type: "Navigate"; path: string }
  | { type: "InvokeBackend"; command: string; args: Record<string, unknown> }
  | { type: "EmitEvent"; event: string; payload: Record<string, unknown> }
  | { type: "Custom"; handlerId: string; data: Record<string, unknown> };

export interface SkillUIPanel {
  id: string;
  title: string;
  componentType: SkillComponentType;
  componentConfig: Record<string, unknown>;
  position: UIPanelPosition;
  size: UIPanelSize;
  collapsible: boolean;
  defaultCollapsed: boolean;
}

export type UIPanelPosition = "Main" | "Sidebar" | "Header" | "Footer";

export type UIPanelSize = "Small" | "Medium" | "Large" | "FullWidth";

export interface SkillSettingsSection {
  id: string;
  label: string;
  icon?: string;
  componentType: SkillComponentType;
  componentConfig: Record<string, unknown>;
}

// Phase-2 type modules
export * from "./agent";
export * from "./artifact";
export * from "./backup";
export * from "./knowledge";
export * from "./mcp";
export * from "./memory";
export * from "./nudge";
export * from "./search";

// ── Workspace / Context Types (merged from workspace.ts) ───────────────
export type ContextSourceType = "attachment" | "search" | "knowledge" | "memory" | "tool";

export type ContextSource = {
  id: string;
  conversationId: string;
  messageId?: string;
  type: ContextSourceType;
  refId: string;
  title: string;
  enabled: boolean;
  summary?: string;
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
};

export type ContextOverrideInput = {
  searchEnabled?: boolean;
  searchProviderId?: string | null;
  enabledKnowledgeBaseIds?: string[];
  enabledMcpServerIds?: string[];
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
  model_id: string;
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
  model_id?: string;
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

// ── Atomic Skills ─────────────────────────────────────────────────────
export interface AtomicSkill {
  id: string;
  name: string;
  description: string;
  input_schema: Record<string, unknown> | null;
  output_schema: Record<string, unknown> | null;
  entry_type: "builtin" | "mcp" | "local" | "plugin";
  entry_ref: string;
  category: string;
  tags: string[];
  version: string;
  enabled: boolean;
  source: "atomic" | "auto-generated";
  created_at: number;
  updated_at: number;
}

export interface AtomicSkillExecutionResult {
  skill_id: string;
  success: boolean;
  output: unknown;
  execution_time_ms: number;
  error?: { error_type: string; message: string };
}

export interface AtomicSkillFilter {
  category?: string;
  source?: string;
  enabled?: boolean;
}

export interface CreateAtomicSkillParams {
  name: string;
  description: string;
  input_schema?: Record<string, unknown>;
  output_schema?: Record<string, unknown>;
  entry_type: "builtin" | "mcp" | "local" | "plugin";
  entry_ref: string;
  category?: string;
  tags?: string[];
  version?: string;
  enabled?: boolean;
  source?: string;
}

export interface UpdateAtomicSkillParams {
  name?: string;
  description?: string;
  input_schema?: Record<string, unknown>;
  output_schema?: Record<string, unknown>;
  entry_type?: "builtin" | "mcp" | "local" | "plugin";
  entry_ref?: string;
  category?: string;
  tags?: string[];
  version?: string;
  enabled?: boolean;
  source?: string;
}

export interface SkillReference {
  id: string;
  skill_id: string;
  workflow_id: string;
  node_id: string;
  created_at: number;
}

// ── Tool Dependencies ─────────────────────────────────────────────────
export type ToolDependencyStatus = "satisfied" | "auto_installable" | "manual_installable" | "needs_generation";

export interface ToolDependency {
  name: string;
  tool_type: string;
  status: ToolDependencyStatus;
  source_info?: string;
  install_instructions?: string;
  config_requirements?: string;
}

// ── Decomposition ─────────────────────────────────────────────────────
export interface DecompositionPreview {
  atomic_skills: Array<{
    id: string;
    name: string;
    description: string;
    entry_type: string;
    entry_ref: string;
  }>;
  tool_dependencies: ToolDependency[];
  workflow_nodes: unknown;
  workflow_edges: unknown;
  original_source: {
    market: string;
    repo?: string;
    version?: string;
  };
  cache_id: string;
}

// ── Work Engine ───────────────────────────────────────────────────────
export type ExecutionStatus = "running" | "paused" | "completed" | "failed" | "cancelled";

export interface ExecutionStatusResponse {
  execution_id: string;
  workflow_id: string;
  status: ExecutionStatus;
  current_node_id: string | null;
  total_time_ms: number;
  node_count: number;
}

export interface ExecutionSummary {
  id: string;
  workflow_id: string;
  status: string;
  total_time_ms: number | null;
  created_at: number;
}

// ── Plan Mode (Agent Work Strategy) ──────────────────────────────────
export type PlanStepStatus = "pending" | "approved" | "rejected" | "running" | "completed" | "error";

export interface PlanStep {
  id: string;
  title: string;
  description: string;
  status: PlanStepStatus;
  /** Estimated tools that will be used for this step */
  estimated_tools?: string[];
  /** Result summary after completion */
  result?: string | null;
}

export type PlanStatus = "draft" | "reviewing" | "approved" | "executing" | "completed" | "cancelled";

export interface Plan {
  id: string;
  conversation_id: string;
  /** The user message that triggered this plan generation */
  user_message_id: string;
  title: string;
  steps: PlanStep[];
  status: PlanStatus;
  is_active: boolean;
  /** The work_strategy that was active when this plan was created, for restoration context */
  created_under_strategy?: "direct" | "plan";
  created_at: number;
  updated_at: number;
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
  createdAt: number;
  updatedAt: number;
}

export interface CreatePromptTemplateInput {
  name: string;
  description?: string;
  content: string;
  variablesSchema?: string;
}

export interface UpdatePromptTemplateInput {
  name?: string;
  description?: string;
  content?: string;
  variablesSchema?: string;
  isActive?: boolean;
  abTestEnabled?: boolean;
}

export interface PromptTemplateVersion {
  id: string;
  templateId: string;
  version: number;
  content: string;
  variablesSchema?: string;
  changelog?: string;
  createdAt: number;
}

export * from "./wiki";

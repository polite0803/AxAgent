// SPDX-License-Identifier: AGPL-3.0-only

// i18n-exempt: Mock data for browser preview mode. Not user-facing UI.
import i18n from "@/i18n";
/**
 * Browser-mode mock backend using localStorage.
 * Activated when the app runs outside Tauri (e.g. `pnpm dev` in browser).
 * Provides CRUD operations for providers, conversations, apps, settings, and gateway.
 */

import type {
  Conversation,
  ConversationBranch,
  ConversationCategory,
  CreateSearchProviderInput,
  GatewayKey,
  KnowledgeBase,
  KnowledgeDocument,
  MemoryItem,
  MemoryNamespace,
  Message,
  PlatformConfig,
  PlatformSession,
  ProgramPolicy,
  SaveProgramPolicyInput,
  SearchProvider,
} from "@/types";
import type { Artifact } from "@/types";
import type { BackupManifest } from "@/types";
import type { CreateKnowledgeBaseInput } from "@/types";
import type { CreateMemoryItemInput, CreateMemoryNamespaceInput } from "@/types";
import type {
  CreateDynamicUISchemaParams,
  DynamicUIFormDataRecord,
  DynamicUIPinRecord,
  DynamicUISchemaRecord,
  DynamicUISchemaVersion,
  ListVersionsResponse,
  SaveDynamicUIFormDataParams,
  UpdateDynamicUISchemaParams,
} from "@/types";
import { emitBrowserEvent } from "./browserEvents";

interface Fleet {
  id: string;
  name: string;
  sceneTemplateSlug?: string;
  status: string;
  createdAt: number;
  updatedAt: number;
  metadata: Record<string, unknown>;
}

interface FleetMember {
  id: string;
  fleetId: string;
  agentId: string;
  agentSlug: string;
  displayName: string;
  role: string;
  agentProfileId?: string;
  roomId: string;
  status: string;
  joinedAt: number;
  todayTokens: number;
  totalTokens: number;
}

/** 浏览器模式持久化的 AgentSession（与后端 agent_sessions 表 upsert 语义对齐） */
interface MockAgentSession {
  conversationId: string;
  name: string | null;
  metadata: Record<string, unknown> | null;
  cwd: string | null;
  permissionMode: string;
  createdAt: number;
  updatedAt: number;
}

/** 浏览器模式 Channel 形状（与 Tauri v2 Channel 的 onmessage 对齐） */
interface MockChannel {
  onmessage?: (evt: unknown) => void;
}

interface WorkflowTemplate {
  id: string;
  name: string;
  description: string;
  icon: string;
  tags: string[];
  version: number;
  is_preset: boolean;
  is_editable: boolean;
  is_public: boolean;
  trigger_config: Record<string, unknown>;
  nodes: unknown[];
  edges: unknown[];
  created_at: number;
  updated_at: number;
}

interface CreateWorkflowTemplateInput {
  name?: string;
  description?: string;
  tags?: string[];
  nodes?: unknown[];
  edges?: unknown[];
}

interface UpdateWorkflowTemplateInput {
  name?: string;
  description?: string;
  tags?: string[];
  nodes?: unknown[];
  edges?: unknown[];
}

interface ProviderKey {
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

interface Provider {
  id: string;
  name: string;
  provider_type: string;
  api_host: string;
  api_path?: string;
  sort_order?: number;
  enabled: boolean;
  models: Array<{
    model_id: string;
    name: string;
    mode?: string;
    enabled?: boolean;
  }>;
  keys: ProviderKey[];
  proxy_config: unknown;
  created_at: number;
  updated_at: number;
}

interface Settings {
  [key: string]: unknown;
}

function genId(): string {
  return crypto.randomUUID();
}

function nowTs(): number {
  return Date.now();
}

function getStore<T>(key: string, defaultValue: T): T {
  try {
    const data = localStorage.getItem(`axagent_${key}`);
    return data ? JSON.parse(data) : defaultValue;
  } catch {
    return defaultValue;
  }
}

function setStore<T>(key: string, value: T): void {
  try {
    localStorage.setItem(`axagent_${key}`, JSON.stringify(value));
  } catch (e) {
    console.warn(`Failed to write localStorage key: axagent_${key}`, e);
  }
}

function generateBrowserResponse(userContent: string): string {
  const greeting = /^(hi|hello|hey)/i.test(userContent.trim());
  if (greeting) {
    return i18n.t("browserMock.greeting");
  }
  const truncatedContent = userContent.length > 50 ? userContent.slice(0, 50) + "..." : userContent;
  return i18n.t("browserMock.receivedMessage", { userContent: truncatedContent });
}

// ── Built-in Providers ──────────────────────────────────────────────────

const BUILT_IN_PROVIDERS = [
  {
    id: "builtin-openai",
    name: "OpenAI",
    provider_type: "openai",
    api_host: "https://api.openai.com",
    api_path: null,
    enabled: true,
    models: [
      {
        provider_id: "builtin-openai",
        model_id: "gpt-5.5",
        name: "gpt-5.5",
        capabilities: ["TextGeneration", "Vision", "FunctionCalling", "Reasoning"],
        max_tokens: 1048576,
        enabled: true,
        param_overrides: null,
      },
      {
        provider_id: "builtin-openai",
        model_id: "gpt-5.4",
        name: "gpt-5.4",
        capabilities: ["TextGeneration", "Vision", "FunctionCalling"],
        max_tokens: 1048576,
        enabled: true,
        param_overrides: null,
      },
      {
        provider_id: "builtin-openai",
        model_id: "gpt-5.4-mini",
        name: "gpt-5.4-mini",
        capabilities: ["TextGeneration", "Vision", "FunctionCalling"],
        max_tokens: 1048576,
        enabled: true,
        param_overrides: null,
      },
      {
        provider_id: "builtin-openai",
        model_id: "o4-mini",
        name: "o4-mini",
        capabilities: ["TextGeneration", "Reasoning", "FunctionCalling"],
        max_tokens: 200000,
        enabled: false,
        param_overrides: null,
      },
    ],
    keys: [],
    proxy_config: null,
    sort_order: 0,
    created_at: 1700000000000,
    updated_at: 1700000000000,
  },
  {
    id: "builtin-openai-responses",
    name: "OpenAI Responses",
    provider_type: "openai_responses",
    api_host: "https://api.openai.com",
    api_path: null,
    enabled: true,
    models: [
      {
        provider_id: "builtin-openai-responses",
        model_id: "gpt-5.5",
        name: "gpt-5.5",
        capabilities: ["TextGeneration", "Vision", "FunctionCalling", "Reasoning"],
        max_tokens: 1048576,
        enabled: true,
        param_overrides: null,
      },
      {
        provider_id: "builtin-openai-responses",
        model_id: "gpt-5.4",
        name: "gpt-5.4",
        capabilities: ["TextGeneration", "Vision", "FunctionCalling"],
        max_tokens: 1048576,
        enabled: true,
        param_overrides: null,
      },
      {
        provider_id: "builtin-openai-responses",
        model_id: "gpt-5.4-mini",
        name: "gpt-5.4-mini",
        capabilities: ["TextGeneration", "Vision", "FunctionCalling"],
        max_tokens: 1048576,
        enabled: true,
        param_overrides: null,
      },
      {
        provider_id: "builtin-openai-responses",
        model_id: "o4-mini",
        name: "o4-mini",
        capabilities: ["TextGeneration", "Reasoning", "FunctionCalling"],
        max_tokens: 200000,
        enabled: false,
        param_overrides: null,
      },
    ],
    keys: [],
    proxy_config: null,
    sort_order: 1,
    created_at: 1700000000000,
    updated_at: 1700000000000,
  },
  {
    id: "builtin-gemini",
    name: "Gemini",
    provider_type: "gemini",
    api_host: "https://generativelanguage.googleapis.com",
    api_path: null,
    enabled: true,
    models: [
      {
        provider_id: "builtin-gemini",
        model_id: "gemini-3.5-flash",
        name: "gemini-3.5-flash",
        capabilities: ["TextGeneration", "Vision", "FunctionCalling", "Reasoning"],
        max_tokens: 1048576,
        enabled: true,
        param_overrides: null,
      },
      {
        provider_id: "builtin-gemini",
        model_id: "gemini-2.5-flash",
        name: "gemini-2.5-flash",
        capabilities: ["TextGeneration", "Vision", "FunctionCalling"],
        max_tokens: 1048576,
        enabled: true,
        param_overrides: null,
      },
      {
        provider_id: "builtin-gemini",
        model_id: "gemini-2.5-pro",
        name: "gemini-2.5-pro",
        capabilities: ["TextGeneration", "Vision", "FunctionCalling", "Reasoning"],
        max_tokens: 1048576,
        enabled: false,
        param_overrides: null,
      },
    ],
    keys: [],
    proxy_config: null,
    sort_order: 2,
    created_at: 1700000000000,
    updated_at: 1700000000000,
  },
  {
    id: "builtin-anthropic",
    name: "Claude",
    provider_type: "anthropic",
    api_host: "https://api.anthropic.com",
    api_path: null,
    enabled: true,
    models: [
      {
        provider_id: "builtin-anthropic",
        model_id: "claude-sonnet-4-6",
        name: "claude-sonnet-4-6",
        capabilities: ["TextGeneration", "Vision", "FunctionCalling"],
        max_tokens: 200000,
        enabled: true,
        param_overrides: null,
      },
      {
        provider_id: "builtin-anthropic",
        model_id: "claude-haiku-4-5",
        name: "claude-haiku-4-5",
        capabilities: ["TextGeneration", "Vision", "FunctionCalling"],
        max_tokens: 200000,
        enabled: true,
        param_overrides: null,
      },
      {
        provider_id: "builtin-anthropic",
        model_id: "claude-opus-4-8",
        name: "claude-opus-4-8",
        capabilities: ["TextGeneration", "Vision", "FunctionCalling", "Reasoning"],
        max_tokens: 200000,
        enabled: false,
        param_overrides: null,
      },
    ],
    keys: [],
    proxy_config: null,
    sort_order: 3,
    created_at: 1700000000000,
    updated_at: 1700000000000,
  },
  {
    id: "builtin-deepseek",
    name: "DeepSeek",
    provider_type: "openai",
    api_host: "https://api.deepseek.com",
    api_path: null,
    enabled: true,
    models: [
      {
        provider_id: "builtin-deepseek",
        model_id: "deepseek-v4-flash",
        name: "deepseek-v4-flash",
        capabilities: ["TextGeneration", "FunctionCalling"],
        max_tokens: 1048576,
        enabled: true,
        param_overrides: null,
      },
      {
        provider_id: "builtin-deepseek",
        model_id: "deepseek-v4-pro",
        name: "deepseek-v4-pro",
        capabilities: ["TextGeneration", "Reasoning", "FunctionCalling"],
        max_tokens: 1048576,
        enabled: true,
        param_overrides: null,
      },
    ],
    keys: [],
    proxy_config: null,
    sort_order: 4,
    created_at: 1700000000000,
    updated_at: 1700000000000,
  },
  {
    id: "builtin-qwen",
    name: i18n.t("browserMock.tongyi"),
    provider_type: "openai",
    api_host: "https://dashscope.aliyuncs.com/compatible-mode/v1",
    api_path: null,
    enabled: true,
    models: [
      {
        provider_id: "builtin-qwen",
        model_id: "qwen3.7-max",
        name: "qwen3.7-max",
        capabilities: ["TextGeneration", "Vision", "FunctionCalling", "Reasoning"],
        max_tokens: 1048576,
        enabled: true,
        param_overrides: null,
      },
      {
        provider_id: "builtin-qwen",
        model_id: "qwen3.6-plus",
        name: "qwen3.6-plus",
        capabilities: ["TextGeneration", "Vision", "FunctionCalling", "Reasoning"],
        max_tokens: 1048576,
        enabled: true,
        param_overrides: null,
      },
      {
        provider_id: "builtin-qwen",
        model_id: "qwen3.6-flash",
        name: "qwen3.6-flash",
        capabilities: ["TextGeneration", "Vision", "FunctionCalling", "Reasoning"],
        max_tokens: 1048576,
        enabled: false,
        param_overrides: null,
      },
    ],
    keys: [],
    proxy_config: null,
    sort_order: 5,
    created_at: 1700000000000,
    updated_at: 1700000000000,
  },
  {
    id: "builtin-kimi",
    name: "Kimi",
    provider_type: "openai",
    api_host: "https://api.moonshot.cn/v1",
    api_path: null,
    enabled: true,
    models: [
      {
        provider_id: "builtin-kimi",
        model_id: "kimi-k2.6",
        name: "kimi-k2.6",
        capabilities: ["TextGeneration", "Vision", "FunctionCalling", "Reasoning"],
        max_tokens: 262144,
        enabled: true,
        param_overrides: null,
      },
      {
        provider_id: "builtin-kimi",
        model_id: "kimi-k2.5",
        name: "kimi-k2.5",
        capabilities: ["TextGeneration", "Vision", "FunctionCalling", "Reasoning"],
        max_tokens: 262144,
        enabled: false,
        param_overrides: null,
      },
    ],
    keys: [],
    proxy_config: null,
    sort_order: 6,
    created_at: 1700000000000,
    updated_at: 1700000000000,
  },
  {
    id: "builtin-doubao",
    name: i18n.t("browserMock.doubao"),
    provider_type: "openai",
    api_host: "https://ark.cn-beijing.volces.com/api/v3",
    api_path: null,
    enabled: true,
    models: [
      {
        provider_id: "builtin-doubao",
        model_id: "doubao-1.5-pro-256k",
        name: "doubao-1.5-pro-256k",
        capabilities: ["TextGeneration", "Vision", "FunctionCalling"],
        max_tokens: 262144,
        enabled: true,
        param_overrides: null,
      },
      {
        provider_id: "builtin-doubao",
        model_id: "doubao-1.5-lite-32k",
        name: "doubao-1.5-lite-32k",
        capabilities: ["TextGeneration", "FunctionCalling"],
        max_tokens: 32768,
        enabled: false,
        param_overrides: null,
      },
    ],
    keys: [],
    proxy_config: null,
    sort_order: 7,
    created_at: 1700000000000,
    updated_at: 1700000000000,
  },
  {
    id: "builtin-siliconflow",
    name: i18n.t("browserMock.siliconFlow"),
    provider_type: "openai",
    api_host: "https://api.siliconflow.cn/v1",
    api_path: null,
    enabled: true,
    models: [
      {
        provider_id: "builtin-siliconflow",
        model_id: "Pro/deepseek-ai/DeepSeek-R1",
        name: "Pro/deepseek-ai/DeepSeek-R1",
        capabilities: ["TextGeneration", "Reasoning", "FunctionCalling"],
        max_tokens: 65536,
        enabled: true,
        param_overrides: null,
      },
      {
        provider_id: "builtin-siliconflow",
        model_id: "Pro/deepseek-ai/DeepSeek-V3",
        name: "Pro/deepseek-ai/DeepSeek-V3",
        capabilities: ["TextGeneration", "FunctionCalling"],
        max_tokens: 65536,
        enabled: true,
        param_overrides: null,
      },
      {
        provider_id: "builtin-siliconflow",
        model_id: "Qwen/Qwen3-235B-A22B",
        name: "Qwen/Qwen3-235B-A22B",
        capabilities: ["TextGeneration", "Reasoning", "FunctionCalling"],
        max_tokens: 262144,
        enabled: false,
        param_overrides: null,
      },
      {
        provider_id: "builtin-siliconflow",
        model_id: "Qwen/Qwen3-32B",
        name: "Qwen/Qwen3-32B",
        capabilities: ["TextGeneration", "Reasoning", "FunctionCalling"],
        max_tokens: 262144,
        enabled: false,
        param_overrides: null,
      },
    ],
    keys: [],
    proxy_config: null,
    sort_order: 8,
    created_at: 1700000000000,
    updated_at: 1700000000000,
  },
  {
    id: "builtin-glm",
    name: "GLM",
    provider_type: "openai",
    api_host: "https://open.bigmodel.cn/api/paas",
    api_path: null,
    enabled: true,
    models: [
      {
        provider_id: "builtin-glm",
        model_id: "glm-5",
        name: "glm-5",
        capabilities: ["TextGeneration", "Reasoning", "FunctionCalling"],
        max_tokens: 128000,
        enabled: true,
        param_overrides: null,
      },
      {
        provider_id: "builtin-glm",
        model_id: "glm-4-plus",
        name: "glm-4-plus",
        capabilities: ["TextGeneration", "Vision", "FunctionCalling"],
        max_tokens: 128000,
        enabled: false,
        param_overrides: null,
      },
      {
        provider_id: "builtin-glm",
        model_id: "glm-4-flash",
        name: "glm-4-flash",
        capabilities: ["TextGeneration", "Vision"],
        max_tokens: 128000,
        enabled: false,
        param_overrides: null,
      },
    ],
    keys: [],
    proxy_config: null,
    sort_order: 6,
    created_at: 1700000000000,
    updated_at: 1700000000000,
  },
  {
    id: "builtin-minimax",
    name: "MiniMax",
    provider_type: "openai",
    api_host: "https://api.minimaxi.com",
    api_path: null,
    enabled: true,
    models: [
      {
        provider_id: "builtin-minimax",
        model_id: "MiniMax-M3",
        name: "MiniMax-M3",
        capabilities: ["TextGeneration", "Reasoning", "FunctionCalling"],
        max_tokens: 1000000,
        enabled: true,
        param_overrides: null,
      },
      {
        provider_id: "builtin-minimax",
        model_id: "MiniMax-S1",
        name: "MiniMax-S1",
        capabilities: ["TextGeneration"],
        max_tokens: 245760,
        enabled: false,
        param_overrides: null,
      },
    ],
    keys: [],
    proxy_config: null,
    sort_order: 7,
    created_at: 1700000000000,
    updated_at: 1700000000000,
  },
  {
    id: "builtin-nvidia",
    name: "NVIDIA",
    provider_type: "openai",
    api_host: "https://integrate.api.nvidia.com/v1",
    api_path: null,
    enabled: true,
    models: [
      {
        provider_id: "builtin-nvidia",
        model_id: "meta/llama-4-maverick-17b-128e-instruct",
        name: "Llama 4 Maverick",
        capabilities: ["TextGeneration", "FunctionCalling"],
        max_tokens: 128000,
        enabled: true,
        param_overrides: null,
      },
      {
        provider_id: "builtin-nvidia",
        model_id: "deepseek-ai/deepseek-v3",
        name: "DeepSeek V3",
        capabilities: ["TextGeneration", "Reasoning"],
        max_tokens: 128000,
        enabled: true,
        param_overrides: null,
      },
    ],
    keys: [],
    proxy_config: null,
    sort_order: 8,
    created_at: 1700000000000,
    updated_at: 1700000000000,
  },
];

function initProviders(): Record<string, unknown>[] {
  const existing = getStore<Record<string, unknown>[]>("providers", []);
  if (existing.length === 0) {
    setStore("providers", BUILT_IN_PROVIDERS);
    return [...BUILT_IN_PROVIDERS];
  }
  // Restore missing models for built-in providers (e.g. after a bad fetch_remote_models wipe)
  let dirty = false;
  const existingMap = new Map(existing.map((p) => [p.id, p]));
  for (const builtin of BUILT_IN_PROVIDERS) {
    const stored = existingMap.get(builtin.id) as
      | (Provider & { models?: Array<{ model_id: string; name: string }> })
      | undefined;
    if (stored && (!stored.models || stored.models.length === 0)) {
      stored.models = [...builtin.models] as typeof stored.models;
      dirty = true;
    }
  }
  if (dirty) {
    setStore("providers", existing);
  }
  return existing;
}

// ── Default Settings ────────────────────────────────────────────────────

const DEFAULT_SETTINGS = {
  theme_mode: "system",
  primary_color: "#17A93D",
  font_size: 14,
  language: "zh-CN",
  send_on_enter: true,
  stream_response: true,
  global_shortcut: "CmdOrCtrl+Shift+A",
  shortcut_toggle_current_window: "CmdOrCtrl+Shift+A",
  shortcut_toggle_all_windows: "CmdOrCtrl+Shift+Alt+A",
  shortcut_close_window: "CmdOrCtrl+Shift+W",
  shortcut_new_conversation: "CmdOrCtrl+N",
  shortcut_open_settings: "CmdOrCtrl+,",
  shortcut_toggle_model_selector: "CmdOrCtrl+Shift+M",
  shortcut_fill_last_message: "CmdOrCtrl+Shift+ArrowUp",
  shortcut_clear_context: "CmdOrCtrl+Shift+K",
  shortcut_clear_conversation_messages: "CmdOrCtrl+Shift+Backspace",
  shortcut_toggle_gateway: "CmdOrCtrl+Shift+G",
  global_shortcuts_enabled: true,
  shortcut_registration_logs_enabled: false,
  shortcut_trigger_toast_enabled: false,
  proxy_enabled: false,
  proxy_url: "",
  auto_backup: false,
  backup_interval_hours: 24,
  content_safety_enabled: true,
  last_selected_conversation_id: null,
  onboarding_completed: true,
  onboarding_wizard_dismissed: true,
  onboarding_tutorial_completed: true,
};

// ── Command Handler ─────────────────────────────────────────────────────

// ── 计划确认闸门（P0-2）浏览器模式模拟 ──
// agent_query 在开启 requirePlanApproval 时弹出计划草稿并挂起，直到
// agent_approve_plan 被调用（approve/reject）。用于 e2e 测试事件驱动流程。
const planDecisionResolvers = new Map<string, (decision: string) => void>();

function waitForPlanDecision(conversationId: string): Promise<string> {
  return new Promise<string>((resolve) => {
    planDecisionResolvers.set(conversationId, resolve);
  });
}

function resolvePlanDecision(conversationId: string, decision: string): void {
  const resolver = planDecisionResolvers.get(conversationId);
  if (resolver) {
    planDecisionResolvers.delete(conversationId);
    resolver(decision);
  }
}

// ── DynamicUI Mock 辅助函数 ──────────────────────────────────────
// i18n-exempt: Mock data keys for localStorage, not user-facing.
function loadMockDynamicUIData<T>(key: string, defaultValue: T): T {
  try {
    const data = localStorage.getItem(`axagent.mock.dynamicUI.${key}`);
    return data ? (JSON.parse(data) as T) : defaultValue;
  } catch {
    return defaultValue;
  }
}

function saveMockDynamicUIData<T>(key: string, data: T): void {
  try {
    localStorage.setItem(`axagent.mock.dynamicUI.${key}`, JSON.stringify(data));
  } catch (e) {
    console.warn(`Failed to write localStorage key: axagent.mock.dynamicUI.${key}`, e);
  }
}

/** 语义化版本号 patch 自增（不传 version 时使用） */
function bumpPatchVersion(version: string): string {
  const parts = version.split(".");
  if (parts.length !== 3) {
    return version;
  }
  const patch = parseInt(parts[2], 10);
  if (isNaN(patch)) {
    return version;
  }
  return `${parts[0]}.${parts[1]}.${patch + 1}`;
}

/** 生成一个简单的 mock UISchema JSON（Column + Text 结构） */
function buildMockUISchemaJSON(): string {
  return JSON.stringify({
    version: "1.0",
    id: "mock-root",
    type: "Column",
    props: {},
    children: [
      {
        version: "1.0",
        id: "mock-text",
        type: "Text",
        props: { content: "Mock dynamic UI content" },
      },
    ],
  });
}

export async function handleCommand<T>(
  cmd: string,
  args?: Record<string, unknown>,
): Promise<T> {
  await new Promise((r) => setTimeout(r, 5));

  switch (cmd) {
    // ── Settings ──────────────────────────────────────────────────────
    case "get_settings":
      return getStore("settings", DEFAULT_SETTINGS) as T;
    // AxInvest: 数据源健康仪表盘 — 后端返回 Vec<VendorHealth>，mock 返回空数组
    case "get_vendor_health_all":
      return [] as unknown as T;
    // AxInvest: 管道历史 — 后端返回 Vec<PipelineRun>，mock 返回空数组
    case "get_pipeline_history":
      return [] as unknown as T;
    case "save_settings": {
      const settings = (args as { settings?: Partial<Settings> }).settings ?? {};
      const current = getStore<Settings>(
        "settings",
        DEFAULT_SETTINGS as Settings,
      );
      const merged = { ...current, ...settings };
      setStore("settings", merged);
      return merged as T;
    }

    // ── Providers ─────────────────────────────────────────────────────
    case "list_providers":
      return initProviders() as T;
    case "create_provider": {
      const input = (args as { input?: Partial<Provider> }).input
        ?? ({} as Partial<Provider>);
      const id = genId();
      const now = nowTs();
      const provider: Provider = {
        id,
        name: input.name ?? "",
        provider_type: input.provider_type ?? "",
        api_host: input.api_host ?? "",
        enabled: input.enabled ?? true,
        models: input.models ?? [],
        keys: [],
        proxy_config: null,
        created_at: now,
        updated_at: now,
      };
      const providers = getStore<Provider[]>("providers", []);
      providers.push(provider);
      setStore("providers", providers);
      return provider as T;
    }
    case "update_provider": {
      const { id, input } = args as { id?: string; input?: Partial<Provider> };
      const providers = getStore<Provider[]>("providers", []);
      const idx = providers.findIndex((p) => p.id === id);
      if (idx === -1) {
        throw new Error("Provider not found");
      }
      if (input?.name !== undefined) {
        providers[idx].name = input.name;
      }
      if (input?.provider_type !== undefined) {
        providers[idx].provider_type = input.provider_type;
      }
      if (input?.api_host !== undefined) {
        providers[idx].api_host = input.api_host;
      }
      if (input?.enabled !== undefined) {
        providers[idx].enabled = input.enabled;
      }
      if (input?.api_path !== undefined) {
        providers[idx].api_path = input.api_path;
      }
      if (input?.sort_order !== undefined) {
        providers[idx].sort_order = input.sort_order;
      }
      providers[idx].updated_at = nowTs();
      setStore("providers", providers);
      return providers[idx] as T;
    }
    case "delete_provider": {
      const { id } = args as { id?: string };
      const providers = getStore<Provider[]>("providers", []).filter(
        (p) => p.id !== id,
      );
      setStore("providers", providers);
      return undefined as T;
    }
    case "reorder_providers": {
      const { providerIds } = args as { providerIds?: string[] };
      const providers = getStore<Provider[]>("providers", []);
      if (providerIds) {
        const providerMap = new Map(providers.map((p) => [p.id, p]));
        for (let i = 0; i < providerIds.length; i++) {
          const p = providerMap.get(providerIds[i]);
          if (p) {
            p.sort_order = i;
          }
        }
        providers.sort((a, b) => (a.sort_order ?? 0) - (b.sort_order ?? 0));
        setStore("providers", providers);
      }
      return undefined as T;
    }
    case "toggle_provider": {
      const { id, enabled } = args as { id?: string; enabled?: boolean };
      const providers = getStore<Provider[]>("providers", []);
      const idx = providers.findIndex((p) => p.id === id);
      if (idx !== -1) {
        providers[idx].enabled = enabled ?? false;
        providers[idx].updated_at = nowTs();
        setStore("providers", providers);
      }
      return undefined as T;
    }
    case "add_provider_key": {
      const { providerId, rawKey } = args as {
        providerId?: string;
        rawKey?: string;
      };
      // SECURITY (S5): 浏览器 mock 模式下，对 API Key 进行 base64 编码存储，防止明文泄露
      const encodedKey = rawKey ? btoa(rawKey) : "";
      console.warn(
        "[browserMock] SECURITY: API key is stored with obfuscation in localStorage. Do NOT use browser mock mode in production.",
      );
      const key: ProviderKey = {
        id: genId(),
        provider_id: providerId ?? "",
        key_encrypted: encodedKey,
        key_prefix: (rawKey ?? "").substring(0, 8) + "...",
        enabled: true,
        last_validated_at: null,
        last_error: null,
        rotation_index: 0,
        created_at: nowTs(),
      };
      const providers = getStore<Provider[]>("providers", []);
      const idx = providers.findIndex((p) => p.id === providerId);
      if (idx !== -1) {
        providers[idx].keys.push(key);
        setStore("providers", providers);
      }
      return key as T;
    }
    case "delete_provider_key": {
      const { keyId } = args as { keyId?: string };
      const providers = getStore<Provider[]>("providers", []);
      for (const p of providers) {
        p.keys = p.keys.filter((k) => k.id !== keyId);
      }
      setStore("providers", providers);
      return undefined as T;
    }
    case "toggle_provider_key": {
      const { keyId, enabled } = args as { keyId?: string; enabled?: boolean };
      const providers = getStore<Provider[]>("providers", []);
      for (const p of providers) {
        for (const k of p.keys) {
          if (k.id === keyId) {
            k.enabled = enabled ?? true;
          }
        }
      }
      setStore("providers", providers);
      return undefined as T;
    }
    case "validate_provider_key":
      return true as T;
    case "save_models": {
      const { providerId, models } = args as {
        providerId?: string;
        models?: Array<{
          model_id: string;
          name: string;
          mode?: string;
          enabled?: boolean;
        }>;
      };
      const providers = getStore<Provider[]>("providers", []);
      const idx = providers.findIndex((p) => p.id === providerId);
      if (idx !== -1 && models) {
        providers[idx].models = models;
        setStore("providers", providers);
      }
      return undefined as T;
    }
    case "toggle_model": {
      const { providerId, modelId, enabled } = args as {
        providerId?: string;
        modelId?: string;
        enabled?: boolean;
      };
      const providers = getStore<Provider[]>("providers", []);
      const pIdx = providers.findIndex((p) => p.id === providerId);
      if (pIdx !== -1) {
        const model = providers[pIdx].models.find(
          (m) => m.model_id === modelId,
        );
        if (model) {
          model.enabled = enabled;
          setStore("providers", providers);
          return model as T;
        }
      }
      throw new Error("Model not found");
    }
    case "update_model_params": {
      const { providerId, modelId, overrides } = args as {
        providerId?: string;
        modelId?: string;
        overrides?: Record<string, unknown>;
      };
      const providers = getStore<Provider[]>("providers", []);
      const pIdx = providers.findIndex((p) => p.id === providerId);
      if (pIdx !== -1) {
        const model = providers[pIdx].models.find(
          (m) => m.model_id === modelId,
        );
        if (model) {
          (model as Record<string, unknown>).param_overrides = overrides;
          setStore("providers", providers);
          return model as T;
        }
      }
      throw new Error("Model not found");
    }
    case "fetch_remote_models": {
      const providers = getStore<Provider[]>("providers", []);
      const target = providers.find(
        (p) => p.id === (args as { providerId?: string }).providerId,
      );
      return (target?.models ?? []) as T;
    }

    // ── Conversations ─────────────────────────────────────────────────
    case "list_conversations":
      return getStore<Conversation[]>("conversations", []).filter(
        (c) => !c.is_archived,
      ) as T;
    case "list_archived_conversations":
      return getStore<Conversation[]>("conversations", []).filter(
        (c) => c.is_archived,
      ) as T;
    case "create_conversation": {
      const { title, modelId, providerId, systemPrompt } = args as Record<
        string,
        unknown
      >;
      const conv = {
        id: genId(),
        title,
        model_id: modelId,
        provider_id: providerId,
        system_prompt: systemPrompt || null,
        temperature: null,
        max_tokens: null,
        top_p: null,
        frequency_penalty: null,
        search_enabled: false,
        search_provider_id: null,
        thinking_budget: null,
        enabled_mcp_server_ids: [],
        enabled_knowledge_base_ids: [],
        enabled_memory_namespace_ids: [],
        message_count: 0,
        is_pinned: false,
        is_archived: false,
        created_at: nowTs(),
        updated_at: nowTs(),
      };
      const convs = getStore<Record<string, unknown>[]>("conversations", []);
      convs.push(conv);
      setStore("conversations", convs);
      return conv as T;
    }
    case "update_conversation": {
      const { id, input } = args as {
        id?: string;
        input?: Partial<Conversation>;
      };
      const convs = getStore<Conversation[]>("conversations", []);
      const idx = convs.findIndex((c) => c.id === id);
      if (idx !== -1 && input) {
        if (input.title !== undefined) {
          convs[idx].title = input.title;
        }
        if (input.category_id !== undefined) {
          convs[idx].category_id = input.category_id;
        }
        if (input.provider_id !== undefined) {
          convs[idx].provider_id = input.provider_id;
        }
        if (input.model_id !== undefined) {
          convs[idx].model_id = input.model_id;
        }
        if (input.temperature !== undefined) {
          convs[idx].temperature = input.temperature;
        }
        if (input.max_tokens !== undefined) {
          convs[idx].max_tokens = input.max_tokens;
        }
        if (input.top_p !== undefined) {
          convs[idx].top_p = input.top_p;
        }
        if (input.frequency_penalty !== undefined) {
          convs[idx].frequency_penalty = input.frequency_penalty;
        }
        convs[idx].updated_at = nowTs();
        setStore("conversations", convs);
        return convs[idx] as T;
      }
      throw new Error("Conversation not found");
    }
    case "delete_conversation": {
      const { id } = args as { id?: string };
      const convs = getStore<Conversation[]>("conversations", []).filter(
        (c) => c.id !== id,
      );
      setStore("conversations", convs);
      const msgs = getStore<Message[]>("messages", []).filter(
        (m) => m.conversation_id !== id,
      );
      setStore("messages", msgs);
      return undefined as T;
    }
    case "toggle_pin_conversation": {
      const { id } = args as { id?: string };
      const convs = getStore<Conversation[]>("conversations", []);
      const idx = convs.findIndex((c) => c.id === id);
      if (idx !== -1) {
        convs[idx].is_pinned = !convs[idx].is_pinned;
        convs[idx].updated_at = nowTs();
        setStore("conversations", convs);
        return convs[idx] as T;
      }
      throw new Error("Conversation not found");
    }
    case "toggle_archive_conversation": {
      const { id } = args as { id?: string };
      const convs = getStore<Conversation[]>("conversations", []);
      const idx = convs.findIndex((c) => c.id === id);
      if (idx !== -1) {
        convs[idx].is_archived = !convs[idx].is_archived;
        convs[idx].updated_at = nowTs();
        setStore("conversations", convs);
        return convs[idx] as T;
      }
      throw new Error("Conversation not found");
    }
    case "list_conversation_categories":
      return getStore<ConversationCategory[]>(
        "conversation_categories",
        [],
      ) as T;
    case "create_conversation_category": {
      const { input } = args as { input: ConversationCategory };
      const cats = getStore<ConversationCategory[]>(
        "conversation_categories",
        [],
      );
      const maxOrder = cats.reduce(
        (m: number, c) => Math.max(m, c.sort_order ?? 0),
        -1,
      );
      const cat: ConversationCategory = {
        id: genId(),
        name: input.name,
        icon_type: input.icon_type ?? null,
        icon_value: input.icon_value ?? null,
        system_prompt: input.system_prompt ?? null,
        default_provider_id: input.default_provider_id ?? null,
        default_model_id: input.default_model_id ?? null,
        default_temperature: input.default_temperature ?? null,
        default_max_tokens: input.default_max_tokens ?? null,
        default_top_p: input.default_top_p ?? null,
        default_frequency_penalty: input.default_frequency_penalty ?? null,
        sort_order: maxOrder + 1,
        is_collapsed: true,
        created_at: nowTs(),
        updated_at: nowTs(),
      };
      cats.push(cat);
      setStore("conversation_categories", cats);
      return cat as T;
    }
    case "update_conversation_category": {
      const { id, input } = args as {
        id: string;
        input: Partial<ConversationCategory>;
      };
      const cats = getStore<ConversationCategory[]>(
        "conversation_categories",
        [],
      );
      const idx = cats.findIndex((c) => c.id === id);
      if (idx !== -1) {
        if (input.name !== undefined) {
          cats[idx].name = input.name;
        }
        if (input.icon_type !== undefined) {
          cats[idx].icon_type = input.icon_type;
        }
        if (input.icon_value !== undefined) {
          cats[idx].icon_value = input.icon_value;
        }
        if (input.system_prompt !== undefined) {
          cats[idx].system_prompt = input.system_prompt;
        }
        if (input.default_provider_id !== undefined) {
          cats[idx].default_provider_id = input.default_provider_id;
        }
        if (input.default_model_id !== undefined) {
          cats[idx].default_model_id = input.default_model_id;
        }
        if (input.default_temperature !== undefined) {
          cats[idx].default_temperature = input.default_temperature;
        }
        if (input.default_max_tokens !== undefined) {
          cats[idx].default_max_tokens = input.default_max_tokens;
        }
        if (input.default_top_p !== undefined) {
          cats[idx].default_top_p = input.default_top_p;
        }
        if (input.default_frequency_penalty !== undefined) {
          cats[idx].default_frequency_penalty = input.default_frequency_penalty;
        }
        cats[idx].updated_at = nowTs();
        setStore("conversation_categories", cats);
        return cats[idx] as T;
      }
      throw new Error("Category not found");
    }
    case "delete_conversation_category": {
      const { id } = args as { id: string };
      const cats = getStore<ConversationCategory[]>(
        "conversation_categories",
        [],
      ).filter((c) => c.id !== id);
      setStore("conversation_categories", cats);
      const convs = getStore<Conversation[]>("conversations", []);
      convs.forEach((c) => {
        if (c.category_id === id) {
          c.category_id = null;
        }
      });
      setStore("conversations", convs);
      return undefined as T;
    }
    case "reorder_conversation_categories": {
      const { categoryIds } = args as { categoryIds: string[] };
      const cats = getStore<ConversationCategory[]>(
        "conversation_categories",
        [],
      );
      const catMap = new Map(cats.map((c) => [c.id, c]));
      for (let i = 0; i < categoryIds.length; i++) {
        const c = catMap.get(categoryIds[i]);
        if (c) {
          c.sort_order = i;
        }
      }
      cats.sort((a, b) => (a.sort_order ?? 0) - (b.sort_order ?? 0));
      setStore("conversation_categories", cats);
      return undefined as T;
    }
    case "set_conversation_category_collapsed": {
      const { id, collapsed } = args as { id?: string; collapsed?: boolean };
      const cats = getStore<ConversationCategory[]>(
        "conversation_categories",
        [],
      );
      const idx = cats.findIndex((c) => c.id === id);
      if (idx !== -1) {
        cats[idx].is_collapsed = collapsed ?? false;
        cats[idx].updated_at = nowTs();
        setStore("conversation_categories", cats);
      }
      return undefined as T;
    }
    case "agent_get_session": {
      // 浏览器 mock：与会话层持久化对齐后端 upsert 语义（有则返回，无则创建）
      const { conversationId } = (args ?? {}) as { conversationId?: string };
      if (!conversationId) {
        return null as T;
      }
      const sessions = getStore<MockAgentSession[]>("agent_sessions", []);
      const existing = sessions.find((s) => s.conversationId === conversationId);
      if (existing) {
        existing.updatedAt = nowTs();
        setStore("agent_sessions", sessions);
        return {
          conversationId: existing.conversationId,
          name: existing.name,
          metadata: existing.metadata,
          createdAt: existing.createdAt,
          lastActiveAt: existing.updatedAt,
        } as T;
      }
      const createdAt = nowTs();
      const created: MockAgentSession = {
        conversationId,
        name: null,
        metadata: null,
        cwd: null,
        permissionMode: "default",
        createdAt,
        updatedAt: createdAt,
      };
      setStore("agent_sessions", [...sessions, created]);
      return {
        conversationId: created.conversationId,
        name: created.name,
        metadata: created.metadata,
        createdAt: created.createdAt,
        lastActiveAt: created.updatedAt,
      } as T;
    }
    case "agent_update_session": {
      // 浏览器 mock：持久化会话字段（cwd / permissionMode / name / metadata），模拟后端 upsert
      const { conversationId, cwd, permissionMode, name, metadata } = (args ?? {}) as {
        conversationId?: string;
        cwd?: string | null;
        permissionMode?: string | null;
        name?: string | null;
        metadata?: Record<string, unknown> | null;
      };
      if (!conversationId) {
        return {
          conversationId: "",
          name: null,
          metadata: null,
          cwd: null,
          permissionMode: "default",
        } as T;
      }
      const sessions = getStore<MockAgentSession[]>("agent_sessions", []);
      const now = nowTs();
      let target = sessions.find((s) => s.conversationId === conversationId);
      if (target) {
        if (cwd !== undefined) {
          target.cwd = cwd;
        }
        if (permissionMode !== undefined) {
          target.permissionMode = permissionMode ?? "default";
        }
        if (name !== undefined) {
          target.name = name;
        }
        if (metadata !== undefined) {
          target.metadata = metadata;
        }
        target.updatedAt = now;
      } else {
        target = {
          conversationId,
          name: name ?? null,
          metadata: metadata ?? null,
          cwd: cwd ?? null,
          permissionMode: permissionMode ?? "default",
          createdAt: now,
          updatedAt: now,
        };
        sessions.push(target);
      }
      setStore("agent_sessions", sessions);
      return {
        conversationId: target.conversationId,
        name: target.name,
        metadata: target.metadata,
        cwd: target.cwd,
        permissionMode: target.permissionMode,
      } as T;
    }
    case "agent_ensure_workspace": {
      const workspacePath = "/mock/workspace/" + Date.now();
      return { workspacePath } as T;
    }
    case "list_agency_experts": {
      return [] as T;
    }
    case "agent_query": {
      const req = (args as {
        request?: { requirePlanApproval?: boolean; input?: string; conversationId?: string };
      } | undefined)?.request;
      // P0-2：开启计划确认时，模拟后端判定为复杂任务并弹出计划草稿等待用户确认
      if (req?.requirePlanApproval) {
        const conversationId = req.conversationId ?? `mock-${genId()}`;
        const input = req.input ?? "";
        emitBrowserEvent("agent-plan-ready-for-approval", {
          conversationId,
          plan: JSON.stringify({
            task_preview: input.slice(0, 200),
            selected_engine: "ReactEngine",
            features: {
              node_count: 3,
              estimated_tool_rounds: 2,
              requires_verification: true,
              has_branches: false,
              has_conditions: false,
            },
            note: i18n.t("browserMock.complexTaskNote"),
          }),
        });
        const decision = await waitForPlanDecision(conversationId);
        if (decision !== "approve") {
          return { conversationId, assistantMessageId: "", status: "rejected" } as T;
        }
        // 批准：模拟流式完成，使前端 eventPromise 正常 resolve
        const assistantId = genId();
        emitBrowserEvent("agent-message-id", { conversationId, assistantMessageId: assistantId });
        emitBrowserEvent("agent-done", {
          conversationId,
          assistantMessageId: assistantId,
          text: i18n.t("browserMock.planCompleted"),
          thinking: "",
          usage: { input_tokens: 1, output_tokens: 1 },
        });
        return { conversationId, assistantMessageId: "", status: undefined } as T;
      }
      // Browser mock: return immediately without error
      return undefined as T;
    }
    case "agent_approve_plan": {
      const req = (args as {
        request?: { conversationId?: string; decision?: string };
      } | undefined)?.request;
      if (req?.conversationId && req?.decision) {
        resolvePlanDecision(req.conversationId, req.decision);
      }
      return undefined as T;
    }
    // ── agent 运行控制 mock（浏览器回退模式，避免抛错）──
    case "agent_pause":
    case "agent_resume":
    case "agent_cancel":
    case "agent_steer":
    case "agent_approve":
    case "agent_respond_ask":
    case "agent_backup_and_clear_sdk_context":
    case "agent_restore_sdk_context_from_backup": {
      return undefined as T;
    }
    case "agent_is_running":
    case "agent_is_paused": {
      return false as T;
    }
    case "agent_runtime_stats": {
      return {
        conversationId: (args as { conversationId?: string } | undefined)?.conversationId ?? "",
        running: false,
        paused: false,
        activeSessions: 0,
        pendingPermissions: 0,
        pendingAskUser: 0,
        activeToolCalls: 0,
        executionProgress: null,
      } as T;
    }
    case "agent_resolve_model": {
      return "gpt-4o-mini" as T;
    }
    case "simple_chat_completion": {
      // 模拟 LLM 生成智能体配置（AgentGeneratorModal 用）
      const mockJson = JSON.stringify({
        agent_type: "general-assistant",
        display_name: i18n.t("agentGenerator.mock.displayName"),
        description: i18n.t("agentGenerator.mock.description"),
        system_prompt: i18n.t("agentGenerator.mock.systemPrompt"),
        permissions: ["read", "write"],
        preferred_model: "gpt-4o-mini",
      });
      return mockJson as T;
    }
    case "plan_list": {
      return [] as T;
    }
    case "plan_generate": {
      // Browser mock: return a draft plan for review
      const { conversationId, content } = args as { conversationId: string; content: string };
      const planId = genId();
      const now = Date.now();
      return {
        id: planId,
        conversation_id: conversationId,
        user_message_id: genId(),
        title: content?.slice(0, 60) || "Mock Plan",
        steps: [
          {
            id: genId(),
            title: i18n.t("browserMock.analyzeRequirements"),
            description: i18n.t("browserMock.understandGoal"),
            status: "pending",
            estimated_tools: ["Read"],
            result: null,
          },
          {
            id: genId(),
            title: i18n.t("browserMock.designPlan"),
            description: i18n.t("browserMock.implementSteps"),
            status: "pending",
            estimated_tools: ["Write"],
            result: null,
          },
          {
            id: genId(),
            title: i18n.t("browserMock.verifyResult"),
            description: i18n.t("browserMock.confirmCompletion"),
            status: "pending",
            estimated_tools: ["Bash"],
            result: null,
          },
        ],
        status: "reviewing",
        is_active: true,
        created_under_strategy: "plan",
        created_at: now,
        updated_at: now,
      } as T;
    }
    case "plan_execute": {
      // Browser mock: return immediately, execution handled by events
      return undefined as T;
    }
    case "plan_cancel": {
      return undefined as T;
    }
    case "plan_activate": {
      const { planId } = args as { planId: string; conversationId: string };
      const now = Date.now();
      return {
        id: planId,
        conversation_id: (args as { conversationId: string }).conversationId,
        user_message_id: genId(),
        title: "Restored Plan",
        steps: [],
        status: "reviewing",
        is_active: true,
        created_under_strategy: "plan",
        created_at: now,
        updated_at: now,
      } as T;
    }
    case "plan_modify_step": {
      return undefined as T;
    }
    case "send_message": {
      const raw = (args as { params?: unknown }).params ?? args;
      const { conversationId, content, attachments, options } = raw as {
        conversationId: string;
        content: string;
        attachments?: unknown[];
        options?: { requirePlanApproval?: boolean };
      };
      // P0-2 plan approval gate (browser mock): hang until user approves/rejects
      if (options?.requirePlanApproval) {
        emitBrowserEvent("agent-plan-ready-for-approval", {
          conversationId,
          plan: JSON.stringify({
            task_preview: content.slice(0, 200),
            selected_engine: "ReactEngine",
            features: {
              node_count: 3,
              estimated_tool_rounds: 2,
              requires_verification: true,
              has_branches: false,
              has_conditions: false,
            },
            note: i18n.t("browserMock.complexTaskNote"),
          }),
        });
        const decision = await waitForPlanDecision(conversationId);
        if (decision !== "approve") {
          return { rejected: true, conversationId } as T;
        }
      }
      const userMsgId = genId();
      const userMsg = {
        id: userMsgId,
        conversation_id: conversationId,
        role: "user",
        content,
        thinking: null,
        attachments: attachments || [],
        created_at: nowTs(),
        parent_message_id: null,
        version_index: 0,
        is_active: true,
      };
      const msgs = getStore<Record<string, unknown>[]>("messages", []);
      msgs.push(userMsg);

      // Generate a simulated AI response in browser mode
      const aiMsg = {
        id: genId(),
        conversation_id: conversationId,
        role: "assistant",
        content: generateBrowserResponse(content),
        thinking: null,
        attachments: [],
        created_at: nowTs() + 1,
        parent_message_id: userMsgId,
        version_index: 0,
        is_active: true,
      };
      msgs.push(aiMsg);
      setStore("messages", msgs);
      return userMsg as T;
    }
    case "list_messages": {
      const { conversationId } = args as { conversationId?: string };
      const msgs = getStore<Message[]>("messages", []).filter(
        (m) => m.conversation_id === conversationId,
      );
      return msgs as T;
    }
    case "list_messages_page": {
      const {
        conversationId,
        limit = 10,
        beforeMessageId = null,
      } = args as {
        conversationId: string;
        limit?: number;
        beforeMessageId?: string | null;
      };
      const allMessages = getStore<Message[]>("messages", [])
        .filter((m) => m.conversation_id === conversationId)
        .sort((a, b) => a.created_at - b.created_at);
      const cursorIndex = beforeMessageId
        ? allMessages.findIndex((m) => m.id === beforeMessageId)
        : allMessages.length;
      const endIndex = cursorIndex >= 0 ? cursorIndex : allMessages.length;
      const startIndex = Math.max(0, endIndex - (limit ?? 10));
      const pageMessages = allMessages.slice(startIndex, endIndex);
      return {
        messages: pageMessages,
        has_older: startIndex > 0,
        oldest_message_id: pageMessages[0]?.id ?? null,
      } as T;
    }
    case "search_conversations": {
      const { query } = args as { query: string };
      const convs = getStore<Conversation[]>("conversations", []);
      const results = convs.flatMap((c) =>
        c.title.toLowerCase().includes(query.toLowerCase())
          ? [{ conversation_id: c.id, title: c.title, snippet: "" }]
          : []
      );
      return results as T;
    }
    case "regenerate_message": {
      const regenRaw = (args as { params?: unknown }).params ?? args;
      const { conversationId: regenConvId, userMessageId: regenUserMsgId } = regenRaw as {
        conversationId?: string;
        userMessageId?: string;
      };
      const regenMsgs = getStore<Message[]>("messages", []);
      const convMsgs = regenMsgs.filter(
        (m) => m.conversation_id === regenConvId,
      );
      let lastUserMsg: Message | null = null;
      if (regenUserMsgId) {
        // Fix: when userMessageId is explicitly specified, only use that exact message;
        // do NOT silently fall back to the last user message if not found.
        lastUserMsg = convMsgs.find(
          (m) => m.id === regenUserMsgId && m.role === "user",
        ) ?? null;
      } else {
        // Fallback: no specific userMessageId → find the last user message
        for (let i = convMsgs.length - 1; i >= 0; i--) {
          if (convMsgs[i].role === "user") {
            lastUserMsg = convMsgs[i];
            break;
          }
        }
      }
      if (lastUserMsg) {
        const existingVersions = regenMsgs.filter(
          (m) => m.parent_message_id === lastUserMsg!.id && m.role === "assistant",
        );
        const nextVersion = existingVersions.length;
        for (const m of regenMsgs) {
          if (
            m.parent_message_id === lastUserMsg!.id
            && m.role === "assistant"
          ) {
            m.is_active = false;
          }
        }
        // Create new AI version
        const newAiMsg: Message = {
          id: genId(),
          conversation_id: regenConvId!,
          role: "assistant",
          content: generateBrowserResponse(lastUserMsg!.content),
          provider_id: null,
          model_id: null,
          token_count: null,
          thinking: null,
          attachments: [],
          tool_calls_json: null,
          tool_call_id: null,
          created_at: nowTs(),
          parent_message_id: lastUserMsg!.id,
          version_index: nextVersion,
          is_active: true,
          status: "complete",
        };
        regenMsgs.push(newAiMsg);
        setStore("messages", regenMsgs);
      }
      return undefined as T;
    }
    case "list_message_versions": {
      const { parentMessageId } = args as { parentMessageId?: string };
      const allMsgs = getStore<Message[]>("messages", []);
      return allMsgs.filter(
        (m) => m.parent_message_id === parentMessageId,
      ) as T;
    }
    case "switch_message_version": {
      const { parentMessageId: switchParent, messageId: switchTarget } = args as {
        parentMessageId?: string;
        messageId?: string;
      };
      const switchMsgs = getStore<Message[]>("messages", []);
      for (const m of switchMsgs) {
        if (m.parent_message_id === switchParent && m.role === "assistant") {
          m.is_active = m.id === switchTarget;
        }
      }
      setStore("messages", switchMsgs);
      return undefined as T;
    }
    case "delete_message_group": {
      const { userMessageId } = args as { userMessageId?: string };
      const delMsgs = getStore<Message[]>("messages", []);
      const filtered = delMsgs.filter(
        (m) => m.id !== userMessageId && m.parent_message_id !== userMessageId,
      );
      setStore("messages", filtered);
      return undefined as T;
    }

    // ── Gateway ───────────────────────────────────────────────────────
    case "list_gateway_keys":
      return getStore<GatewayKey[]>("gateway_keys", []) as T;
    case "create_gateway_key": {
      const input = (args as { input?: Partial<GatewayKey> }).input ?? {};
      const key: GatewayKey = {
        id: genId(),
        name: input.name ?? "",
        key_hash: "",
        key_prefix: "",
        enabled: input.enabled ?? true,
        created_at: nowTs(),
        last_used_at: null,
        has_encrypted_key: true,
      };
      const keys = getStore<GatewayKey[]>("gateway_keys", []);
      keys.push(key);
      setStore("gateway_keys", keys);
      return {
        gateway_key: key,
        plain_key: `sk-mock-plain-key-${genId().substring(0, 8)}`,
      } as T;
    }
    case "delete_gateway_key": {
      const { id } = args as { id?: string };
      const keys = getStore<GatewayKey[]>("gateway_keys", []).filter(
        (k) => k.id !== id,
      );
      setStore("gateway_keys", keys);
      return undefined as T;
    }
    case "toggle_gateway_key": {
      const { id, enabled } = args as { id?: string; enabled?: boolean };
      const keys = getStore<GatewayKey[]>("gateway_keys", []);
      const idx = keys.findIndex((k) => k.id === id);
      if (idx !== -1) {
        keys[idx].enabled = enabled ?? false;
        setStore("gateway_keys", keys);
      }
      return undefined as T;
    }
    case "get_gateway_metrics":
      return {
        total_requests: 0,
        successful_requests: 0,
        failed_requests: 0,
        avg_latency_ms: 0,
        requests_per_minute: 0,
        active_keys: 0,
        uptime_seconds: 0,
      } as T;
    case "get_gateway_usage_by_key":
    case "get_gateway_usage_by_provider":
    case "get_gateway_usage_by_day":
      return [] as T;
    case "get_gateway_status":
      return {
        is_running: false,
        listen_address: "127.1.0.0",
        port: 3000,
        ssl_enabled: false,
        started_at: null,
        https_port: null,
        force_ssl: false,
      } as T;
    case "get_connected_programs":
      return [] as T;
    case "start_gateway":
    case "stop_gateway":
      return undefined as T;

    // ── Data management ───────────────────────────────────────────────
    case "export_data":
      return { path: "export.json" } as T;
    case "import_data":
      return undefined as T;
    case "clear_data":
      localStorage.clear();
      return undefined as T;

    // ── Phase 2: Search Providers ──────────────────────────────────────
    case "list_search_providers":
      return getStore("search_providers", []) as T;
    case "create_search_provider": {
      const sps = getStore<SearchProvider[]>("search_providers", []);
      const spInput = (args as { input?: CreateSearchProviderInput }).input
        ?? ({} as CreateSearchProviderInput);
      const sp: SearchProvider = {
        id: genId(),
        name: spInput.name,
        providerType: spInput.providerType,
        endpoint: spInput.endpoint,
        hasApiKey: !!spInput.apiKey,
        enabled: spInput.enabled ?? true,
        resultLimit: spInput.resultLimit ?? 10,
        timeoutMs: spInput.timeoutMs ?? 5000,
      };
      sps.push(sp);
      setStore("search_providers", sps);
      return sp as T;
    }
    case "update_search_provider": {
      const sps2 = getStore<SearchProvider[]>("search_providers", []);
      const spUpdateId = (args as { id?: string }).id;
      const spInput = (args as { input?: Partial<CreateSearchProviderInput> }).input ?? {};
      const spi = sps2.findIndex((s) => s.id === spUpdateId);
      if (spi >= 0) {
        if (spInput.name !== undefined) {
          sps2[spi].name = spInput.name;
        }
        if (spInput.endpoint !== undefined) {
          sps2[spi].endpoint = spInput.endpoint;
        }
        if (spInput.enabled !== undefined) {
          sps2[spi].enabled = spInput.enabled;
        }
        if (spInput.region !== undefined) {
          sps2[spi].region = spInput.region;
        }
        if (spInput.language !== undefined) {
          sps2[spi].language = spInput.language;
        }
        if (spInput.safeSearch !== undefined) {
          sps2[spi].safeSearch = spInput.safeSearch;
        }
        if (spInput.resultLimit !== undefined) {
          sps2[spi].resultLimit = spInput.resultLimit;
        }
        if (spInput.timeoutMs !== undefined) {
          sps2[spi].timeoutMs = spInput.timeoutMs;
        }
        if (spInput.apiKey !== undefined) {
          sps2[spi].hasApiKey = !!spInput.apiKey;
        }
        setStore("search_providers", sps2);
        return sps2[spi] as T;
      }
      return undefined as T;
    }
    case "delete_search_provider": {
      const sps3 = getStore<SearchProvider[]>("search_providers", []);
      setStore(
        "search_providers",
        sps3.filter((s) => s.id !== (args as { id?: string })?.id),
      );
      return undefined as T;
    }
    case "test_search_provider":
      return { ok: true, latency_ms: 120 } as T;

    // ── Phase 2: MCP Servers ──────────────────────────────────────────
    case "list_local_tools":
      return [
        {
          groupId: "builtin-file-read",
          groupName: i18n.t("browserMock.fileRead"),
          description: i18n.t("browserMock.fileReadDesc"),
          enabled: true,
          tools: [
            {
              name: "FileRead",
              description: i18n.t("browserMock.fileReadDesc"),
              category: "file_read",
              isDestructive: false,
              isReadOnly: true,
              isConcurrencySafe: true,
              enabled: true,
            },
            {
              name: "Glob",
              description: i18n.t("browserMock.globDesc"),
              category: "file_read",
              isDestructive: false,
              isReadOnly: true,
              isConcurrencySafe: true,
              enabled: true,
            },
            {
              name: "Grep",
              description: i18n.t("browserMock.grepDesc"),
              category: "file_read",
              isDestructive: false,
              isReadOnly: true,
              isConcurrencySafe: true,
              enabled: true,
            },
          ],
        },
        {
          groupId: "builtin-file-write",
          groupName: i18n.t("browserMock.fileWrite"),
          description: i18n.t("browserMock.fileWriteDesc"),
          enabled: true,
          tools: [
            {
              name: "FileWrite",
              description: i18n.t("browserMock.fileWriteDesc"),
              category: "file_write",
              isDestructive: true,
              isReadOnly: false,
              isConcurrencySafe: false,
              enabled: true,
            },
            {
              name: "FileEdit",
              description: i18n.t("browserMock.fileEditDesc"),
              category: "file_write",
              isDestructive: true,
              isReadOnly: false,
              isConcurrencySafe: false,
              enabled: false,
            },
            {
              name: "DeleteFile",
              description: i18n.t("browserMock.deleteFileDesc"),
              category: "file_write",
              isDestructive: true,
              isReadOnly: false,
              isConcurrencySafe: false,
              enabled: true,
            },
          ],
        },
        {
          groupId: "builtin-shell",
          groupName: i18n.t("browserMock.shellCommand"),
          description: i18n.t("browserMock.shellCommandDesc"),
          enabled: false,
          tools: [
            {
              name: "Bash",
              description: i18n.t("browserMock.bashDesc"),
              category: "shell",
              isDestructive: true,
              isReadOnly: false,
              isConcurrencySafe: false,
              enabled: true,
            },
          ],
        },
      ] as T;
    case "toggle_local_tool_group":
      return {
        groupId: (args as Record<string, unknown>)?.groupId,
        groupName: "",
        description: "",
        enabled: true,
        tools: [],
      } as T;
    case "toggle_single_tool":
      return [] as T;
    case "list_mcp_servers":
      return getStore("mcp_servers", []) as T;
    case "create_mcp_server": {
      const mcps = getStore<Record<string, unknown>[]>("mcp_servers", []);
      const mcp = {
        id: genId(),
        ...(args as Record<string, unknown>),
        status: "disconnected",
        created_at: nowTs(),
        updated_at: nowTs(),
      };
      mcps.push(mcp);
      setStore("mcp_servers", mcps);
      return mcp as T;
    }
    case "update_mcp_server": {
      const mcps2 = getStore<Record<string, unknown>[]>("mcp_servers", []);
      const mi = mcps2.findIndex(
        (m) => m.id === (args as Record<string, unknown>)?.id,
      );
      if (mi >= 0) {
        Object.assign(mcps2[mi], args, { updated_at: nowTs() });
        setStore("mcp_servers", mcps2);
        return mcps2[mi] as T;
      }
      return undefined as T;
    }
    case "delete_mcp_server": {
      const mcps3 = getStore<Record<string, unknown>[]>("mcp_servers", []);
      setStore(
        "mcp_servers",
        mcps3.filter((m) => m.id !== (args as Record<string, unknown>)?.id),
      );
      return undefined as T;
    }
    case "connect_mcp_server":
      return { status: "connected" } as T;
    case "disconnect_mcp_server":
      return { status: "disconnected" } as T;
    case "list_mcp_tools":
      return [
        { name: "web_search", description: "Search the web", parameters: {} },
        {
          name: "calculator",
          description: "Evaluate math expressions",
          parameters: {},
        },
      ] as T;
    case "execute_tool":
      return {
        success: true,
        output: `Mock result for tool "${(args as Record<string, unknown>)?.tool_name ?? "unknown"}"`,
      } as T;
    case "test_mcp_server":
      return { ok: true, error: undefined } as T;
    case "list_tool_executions":
      return [] as T;

    // ── Phase 2: Knowledge Base ───────────────────────────────────────
    case "list_knowledge_bases":
      return getStore<KnowledgeBase[]>("knowledge_bases", []) as T;
    case "create_knowledge_base": {
      const input = (args as { input?: CreateKnowledgeBaseInput }).input
        ?? ({} as CreateKnowledgeBaseInput);
      const kbs = getStore<KnowledgeBase[]>("knowledge_bases", []);
      const kb: KnowledgeBase & {
        documents: KnowledgeDocument[];
        created_at: number;
        updated_at: number;
      } = {
        id: genId(),
        name: input.name,
        description: input.description,
        embeddingProvider: input.embeddingProvider,
        enabled: input.enabled ?? true,
        sortOrder: kbs.length,
        documents: [],
        created_at: nowTs(),
        updated_at: nowTs(),
      };
      kbs.push(kb);
      setStore("knowledge_bases", kbs);
      return kb as T;
    }
    case "update_knowledge_base": {
      const kbs2 = getStore<KnowledgeBase[]>("knowledge_bases", []);
      const { id, input } = args as {
        id: string;
        input?: Partial<KnowledgeBase>;
      };
      const ki = kbs2.findIndex((k) => k.id === id);
      if (ki >= 0) {
        if (input?.name !== undefined) {
          kbs2[ki].name = input.name;
        }
        if (input?.description !== undefined) {
          kbs2[ki].description = input.description;
        }
        if (input?.enabled !== undefined) {
          kbs2[ki].enabled = input.enabled;
        }
        setStore("knowledge_bases", kbs2);
        return kbs2[ki] as T;
      }
      return undefined as T;
    }
    case "delete_knowledge_base": {
      const kbs3 = getStore<KnowledgeBase[]>("knowledge_bases", []);
      setStore(
        "knowledge_bases",
        kbs3.filter((k) => k.id !== (args as { id?: string })?.id),
      );
      return undefined as T;
    }
    case "add_knowledge_document": {
      const kbs4 = getStore<
        (KnowledgeBase & {
          documents: KnowledgeDocument[];
          updated_at: number;
        })[]
      >("knowledge_bases", []);
      const { baseId, ...docInput } = args as {
        baseId?: string;
        title?: string;
        sourcePath?: string;
      };
      const kbi = kbs4.findIndex((k) => k.id === baseId);
      if (kbi >= 0) {
        const doc: KnowledgeDocument = {
          id: genId(),
          knowledgeBaseId: baseId!,
          title: docInput.title ?? "Untitled",
          sourcePath: docInput.sourcePath ?? "",
          mimeType: "text/plain",
          sizeBytes: 0,
          indexingStatus: "pending",
          docType: "document",
        };
        kbs4[kbi].documents = [...(kbs4[kbi].documents || []), doc];
        kbs4[kbi].updated_at = nowTs();
        setStore("knowledge_bases", kbs4);
        return doc as T;
      }
      return undefined as T;
    }
    case "list_knowledge_documents": {
      const kbs5 = getStore<
        (KnowledgeBase & {
          documents: KnowledgeDocument[];
          updated_at: number;
        })[]
      >("knowledge_bases", []);
      const target = kbs5.find(
        (k) => k.id === (args as { baseId?: string })?.baseId,
      );
      return (target?.documents ?? []) as T;
    }
    case "delete_knowledge_document": {
      const kbs6 = getStore<
        (KnowledgeBase & {
          documents: KnowledgeDocument[];
          updated_at: number;
        })[]
      >("knowledge_bases", []);
      const delDocId = (args as { id?: string })?.id;
      for (const kb of kbs6) {
        const docs = kb.documents || [];
        const filtered = docs.filter((d) => d.id !== delDocId);
        if (filtered.length !== docs.length) {
          kb.documents = filtered;
          kb.updated_at = nowTs();
          break;
        }
      }
      setStore("knowledge_bases", kbs6);
      return undefined as T;
    }
    case "query_knowledge":
    case "search_knowledge_base":
      return [] as T;
    case "rebuild_knowledge_index":
    case "clear_knowledge_index":
      return undefined as T;
    case "extract_entities_for_kb":
    case "extract_entities_from_documents":
      return {
        status: "success",
        documentCount: 0,
        taskCount: 0,
        message: "实体抽取仅在桌面模式下可用（浏览器模式 mock）",
      } as T;
    case "graph_enhanced_search":
      return {
        query: args?.query ?? "",
        entities: [],
        relations: [],
        chunks: [],
      } as T;

    // ── Phase 2: Memory ───────────────────────────────────────────────
    case "list_memory_namespaces":
      return getStore<MemoryNamespace[]>("memory_namespaces", []) as T;
    case "create_memory_namespace": {
      const input = (args as { input?: CreateMemoryNamespaceInput }).input
        ?? ({} as CreateMemoryNamespaceInput);
      const mns = getStore<MemoryNamespace[]>("memory_namespaces", []);
      const mn: MemoryNamespace & {
        items: MemoryItem[];
        created_at: number;
        updated_at: number;
      } = {
        id: genId(),
        name: input.name,
        scope: input.scope ?? "global",
        embeddingProvider: input.embeddingProvider,
        sortOrder: mns.length,
        items: [],
        created_at: nowTs(),
        updated_at: nowTs(),
      };
      mns.push(mn);
      setStore("memory_namespaces", mns);
      return mn as T;
    }
    case "delete_memory_namespace": {
      const mns2 = getStore<MemoryNamespace[]>("memory_namespaces", []);
      setStore(
        "memory_namespaces",
        mns2.filter((n) => n.id !== (args as { id?: string })?.id),
      );
      return undefined as T;
    }
    case "add_memory_item": {
      const mns3 = getStore<
        (MemoryNamespace & { items: MemoryItem[]; updated_at: number })[]
      >("memory_namespaces", []);
      const inputMem = (args as { input?: CreateMemoryItemInput }).input
        ?? ({} as CreateMemoryItemInput);
      const mni = mns3.findIndex((n) => n.id === inputMem?.namespaceId);
      if (mni >= 0) {
        const item: MemoryItem = {
          id: genId(),
          namespaceId: inputMem.namespaceId!,
          title: inputMem.title ?? "",
          content: inputMem.content ?? "",
          source: inputMem.source ?? "manual",
          indexStatus: "pending",
          tier: "working",
          importance: 0.5,
          memoryNature: "semantic",
          tags: [],
          accessCount: 0,
          decayRate: 0.02,
          updatedAt: new Date().toISOString(),
          applicabilityTags: inputMem.applicabilityTags ?? [],
          confirmed: inputMem.confirmed ?? 0,
        };
        mns3[mni].items = [...(mns3[mni].items || []), item];
        mns3[mni].updated_at = nowTs();
        setStore("memory_namespaces", mns3);
        return item as T;
      }
      return undefined as T;
    }
    case "list_memory_items": {
      const mns4 = getStore<(MemoryNamespace & { items: MemoryItem[] })[]>(
        "memory_namespaces",
        [],
      );
      const ns = mns4.find(
        (n) => n.id === (args as { namespaceId?: string })?.namespaceId,
      );
      return (ns?.items ?? []) as T;
    }
    case "delete_memory_item": {
      const mns5 = getStore<
        (MemoryNamespace & { items: MemoryItem[]; updated_at: number })[]
      >("memory_namespaces", []);
      const delItemId = (args as { id?: string })?.id;
      for (const mns of mns5) {
        const items = mns.items || [];
        const filtered = items.filter((i) => i.id !== delItemId);
        if (filtered.length !== items.length) {
          mns.items = filtered;
          mns.updated_at = nowTs();
          break;
        }
      }
      setStore("memory_namespaces", mns5);
      return undefined as T;
    }
    case "recall_memory":
    case "search_memory":
      return [] as T;
    case "rebuild_memory_index":
    case "clear_memory_index":
      return undefined as T;

    // ── Fleet (办公室) ─────────────────────────────────────────────────
    case "fleet_list": {
      return getStore<Fleet[]>("fleets", []) as T;
    }
    case "fleet_get": {
      const fleetId = (args as { fleetId?: string }).fleetId ?? "";
      const fleet = getStore<Fleet[]>("fleets", []).find((f) => f.id === fleetId);
      return (fleet ?? null) as T;
    }
    case "fleet_create": {
      const input = (args as { input?: Partial<Fleet> }).input ?? {};
      const fleets = getStore<Fleet[]>("fleets", []);
      const fleet: Fleet = {
        id: genId(),
        name: input.name ?? "New Fleet",
        sceneTemplateSlug: (input as Record<string, unknown>).sceneTemplateSlug as string ?? "default",
        status: "active",
        createdAt: nowTs(),
        updatedAt: nowTs(),
        metadata: {},
      };
      fleets.push(fleet);
      setStore("fleets", fleets);
      return fleet as T;
    }
    case "fleet_update_status": {
      const { fleetId, status } = args as { fleetId?: string; status?: string };
      if (fleetId && status) {
        const fleets = getStore<Fleet[]>("fleets", []);
        setStore(
          "fleets",
          fleets.map((f) => (f.id === fleetId ? { ...f, status, updatedAt: nowTs() } : f)),
        );
      }
      return undefined as T;
    }
    case "fleet_delete": {
      const fleetId = (args as { fleetId?: string }).fleetId ?? "";
      setStore("fleets", getStore<Fleet[]>("fleets", []).filter((f) => f.id !== fleetId));
      // 级联删除成员缓存
      for (let i = localStorage.length - 1; i >= 0; i--) {
        const key = localStorage.key(i);
        if (key && key.startsWith(`axagent_fleet_members:${fleetId}`)) {
          localStorage.removeItem(key);
        }
      }
      return undefined as T;
    }
    case "fleet_reset_daily_tokens": {
      const fleetId = (args as { fleetId?: string }).fleetId ?? "";
      const members = getStore<FleetMember[]>(`fleet_members:${fleetId}`, []);
      setStore(
        `fleet_members:${fleetId}`,
        members.map((m) => ({ ...m, todayTokens: 0 })),
      );
      return undefined as T;
    }
    case "fleet_list_members": {
      const fleetId = (args as { fleetId?: string }).fleetId ?? "";
      return getStore<FleetMember[]>(`fleet_members:${fleetId}`, []) as T;
    }
    case "fleet_get_member": {
      const memberId = (args as { memberId?: string }).memberId ?? "";
      for (let i = localStorage.length - 1; i >= 0; i--) {
        const key = localStorage.key(i);
        if (!key || !key.startsWith("axagent_fleet_members:")) {
          continue;
        }
        const found = getStore<FleetMember[]>(key.replace("axagent_", ""), [])
          .find((m) => m.id === memberId);
        if (found) {
          return found as T;
        }
      }
      return null as T;
    }
    case "fleet_add_member": {
      const input = (args as { input?: Partial<FleetMember> }).input ?? {};
      const fleetId = input.fleetId ?? "";
      const members = getStore<FleetMember[]>(`fleet_members:${fleetId}`, []);
      // 与后端一致：同舰队内 slug 必须唯一（路由与事件回写的键）
      const slug = (input.agentSlug ?? "assistant").trim();
      if (members.some((m) => m.agentSlug === slug)) {
        throw new Error(
          JSON.stringify({
            code: "FLEET_SLUG_EXISTS",
            params: { slug },
          }),
        );
      }
      const member: FleetMember = {
        id: genId(),
        fleetId,
        agentId: input.agentId ?? genId(),
        agentSlug: slug,
        displayName: input.displayName ?? "Assistant",
        role: input.role ?? "",
        agentProfileId: input.agentProfileId ?? undefined,
        roomId: input.roomId ?? "workspace",
        status: "idle",
        joinedAt: nowTs(),
        todayTokens: 0,
        totalTokens: 0,
      };
      members.push(member);
      setStore(`fleet_members:${fleetId}`, members);
      return member as T;
    }
    case "fleet_remove_member": {
      const { memberId } = args as { memberId?: string };
      if (memberId) {
        // 遍历所有 fleet_members 存储，移除对应成员
        for (let i = localStorage.length - 1; i >= 0; i--) {
          const key = localStorage.key(i);
          if (!key || !key.startsWith("axagent_fleet_members:")) {
            continue;
          }
          const members = getStore<FleetMember[]>(key.replace("axagent_", ""), []);
          const next = members.filter((m) => m.id !== memberId);
          setStore(key.replace("axagent_", ""), next);
        }
      }
      return undefined as T;
    }
    case "fleet_update_member_status": {
      const { memberId, status } = args as { memberId?: string; status?: string };
      if (memberId && status) {
        for (let i = localStorage.length - 1; i >= 0; i--) {
          const key = localStorage.key(i);
          if (!key || !key.startsWith("axagent_fleet_members:")) {
            continue;
          }
          const members = getStore<FleetMember[]>(key.replace("axagent_", ""), []);
          const next = members.map((m) => (m.id === memberId ? { ...m, status } : m));
          setStore(key.replace("axagent_", ""), next);
        }
      }
      return undefined as T;
    }
    case "fleet_dispatch":
    case "fleet_direct_message": {
      const input = (args as { input?: { fleetId?: string; userMessage?: string; agentSlug?: string } })
        .input ?? {};
      const onEvent = (args as { onEvent?: MockChannel }).onEvent;
      const fleetId = input.fleetId ?? "";
      const members = getStore<FleetMember[]>(`fleet_members:${fleetId}`, []);
      const push = (evt: unknown) => onEvent?.onmessage?.(evt);
      if (members.length === 0) {
        push({ type: "error", message: i18n.t("browserMock.fleetNoMembers") });
        return undefined as T;
      }
      // 直接 DM 时定位目标成员，否则取第一个
      const target = (args as { input?: { agentSlug?: string } }).input?.agentSlug
        ? members.find((m) => m.agentSlug === (args as { input?: { agentSlug?: string } }).input!.agentSlug)
          ?? members[0]
        : members[0];
      const msg = input.userMessage ?? "";
      push({
        type: "routing",
        agentSlug: target.agentSlug,
        agentId: target.agentId,
        roomId: target.roomId,
        taskSummary: msg,
      });
      push({ type: "agent_status", agentSlug: target.agentSlug, agentId: target.agentId, status: "busy" });
      push({
        type: "agent_message",
        agentSlug: target.agentSlug,
        agentId: target.agentId,
        content: i18n.t("browserMock.fleetMessageReceived", {
          name: target.displayName,
          message: msg,
        }),
      });
      push({
        type: "token_usage",
        agentSlug: target.agentSlug,
        agentId: target.agentId,
        inputTokens: 12,
        outputTokens: 34,
      });
      push({ type: "agent_status", agentSlug: target.agentSlug, agentId: target.agentId, status: "idle" });
      push({ type: "complete" });
      return undefined as T;
    }

    // ── Phase 2: Artifacts ────────────────────────────────────────────
    case "list_artifacts": {
      const allArtifacts = getStore<Artifact[]>("artifacts", []);
      const convId = (args as { conversationId?: string })?.conversationId;
      return (
        convId
          ? allArtifacts.filter((a) => a.conversationId === convId)
          : allArtifacts
      ) as T;
    }
    case "create_artifact": {
      const input = (args as { input?: Partial<Artifact> }).input ?? {};
      const arts = getStore<Artifact[]>("artifacts", []);
      const art: Artifact = {
        id: genId(),
        conversationId: input.conversationId ?? "",
        title: input.title ?? "Untitled",
        content: input.content ?? "",
        kind: input.kind ?? "note",
        format: input.format ?? "text",
        language: input.language,
        previewMode: input.previewMode,
        metadata: input.metadata,
        pinned: false,
        updatedAt: new Date().toISOString(),
      };
      arts.push(art);
      setStore("artifacts", arts);
      return art as T;
    }
    case "update_artifact": {
      const arts2 = getStore<Artifact[]>("artifacts", []);
      const artInput = (args as { id?: string; input?: Partial<Artifact> })
        .input;
      const ai = arts2.findIndex((a) => a.id === (args as { id?: string }).id);
      if (ai >= 0 && artInput) {
        if (artInput.title !== undefined) {
          arts2[ai].title = artInput.title;
        }
        if (artInput.content !== undefined) {
          arts2[ai].content = artInput.content;
        }
        if (artInput.format !== undefined) {
          arts2[ai].format = artInput.format;
        }
        if (artInput.language !== undefined) {
          arts2[ai].language = artInput.language;
        }
        if (artInput.previewMode !== undefined) {
          arts2[ai].previewMode = artInput.previewMode;
        }
        if (artInput.pinned !== undefined) {
          arts2[ai].pinned = artInput.pinned;
        }
        arts2[ai].updatedAt = new Date().toISOString();
        setStore("artifacts", arts2);
        return arts2[ai] as T;
      }
      return undefined as T;
    }
    case "delete_artifact": {
      const arts3 = getStore<Artifact[]>("artifacts", []);
      setStore(
        "artifacts",
        arts3.filter((a) => a.id !== (args as { id?: string })?.id),
      );
      return undefined as T;
    }

    // ── Phase 2: Conversation Branching ───────────────────────────────
    case "fork_conversation": {
      const convs = getStore<Record<string, unknown>[]>("conversations", []);
      const source = convs.find(
        (c) => c.id === (args as Record<string, unknown>)?.conversationId,
      );
      if (source) {
        const forked = {
          ...JSON.parse(JSON.stringify(source)),
          id: genId(),
          parent_id: source.id,
          title: (args as Record<string, unknown>)?.title
            ?? `Fork of ${source.title}`,
          created_at: nowTs(),
          updated_at: nowTs(),
        };
        convs.push(forked);
        setStore("conversations", convs);
        return forked as T;
      }
      return undefined as T;
    }
    case "list_branches": {
      const convs2 = getStore<Record<string, unknown>[]>("conversations", []);
      const parentId = (args as Record<string, unknown>)?.conversationId;
      return convs2.filter(
        (c) => c.parent_id === parentId || c.id === parentId,
      ) as T;
    }
    case "compare_branches": {
      const brA = (args as Record<string, unknown>)?.branchA;
      const brB = (args as Record<string, unknown>)?.branchB;
      return { branch_a: brA, branch_b: brB, differences: [] } as T;
    }

    // ── Phase 2: Context Sources ──────────────────────────────────────
    case "list_context_sources":
      return getStore("context_sources", []) as T;
    case "add_context_source": {
      const css = getStore<Record<string, unknown>[]>("context_sources", []);
      const cs = {
        id: genId(),
        ...(args as Record<string, unknown>),
        enabled: true,
        created_at: nowTs(),
        updated_at: nowTs(),
      };
      css.push(cs);
      setStore("context_sources", css);
      return cs as T;
    }
    case "remove_context_source": {
      const css2 = getStore<Record<string, unknown>[]>("context_sources", []);
      setStore(
        "context_sources",
        css2.filter((c) => c.id !== (args as Record<string, unknown>)?.id),
      );
      return undefined as T;
    }
    case "toggle_context_source": {
      const css3 = getStore<Record<string, unknown>[]>("context_sources", []);
      const csi = css3.findIndex(
        (c) => c.id === (args as Record<string, unknown>)?.id,
      );
      if (csi >= 0) {
        css3[csi].enabled = !css3[csi].enabled;
        css3[csi].updated_at = nowTs();
        setStore("context_sources", css3);
        return css3[csi] as T;
      }
      return undefined as T;
    }

    // ── Phase 2: Backup ──────────────────────────────────────────────
    case "create_backup": {
      const bkps = getStore<Record<string, unknown>[]>("backups", []);
      const bkp = {
        id: genId(),
        version: (args as Record<string, unknown>)?.format || "json",
        createdAt: new Date().toISOString(),
        encrypted: false,
        checksum: "mock-checksum",
        objectCountsJson: "{}",
        sourceAppVersion: "0.1.0",
        filePath: "/mock/path/axagent-backup.json",
        fileSize: 1024,
      };
      bkps.push(bkp);
      setStore("backups", bkps);
      return bkp as T;
    }
    case "list_backups":
      return getStore<BackupManifest[]>("backups", []) as T;
    case "delete_backup": {
      const backups = getStore<BackupManifest[]>("backups", []);
      const bkpId = (args as { backupId?: string })?.backupId;
      setStore(
        "backups",
        backups.filter((b) => b.id !== bkpId),
      );
      return undefined as T;
    }
    case "batch_delete_backups": {
      const allBkps = getStore<BackupManifest[]>("backups", []);
      const idsToDelete = (args as { backupIds?: string[] })?.backupIds || [];
      setStore(
        "backups",
        allBkps.filter((b) => !idsToDelete.includes(b.id)),
      );
      return undefined as T;
    }
    case "restore_backup":
      return undefined as T;
    case "get_backup_settings":
      return {
        enabled: false,
        intervalHours: 24,
        maxCount: 10,
        backupDir: "/mock/backups",
      } as T;
    case "update_backup_settings":
      return undefined as T;

    // ── Files Page ─────────────────────────────────────────────────────
    case "list_files_page_entries": {
      const category = (args as { category?: string })?.category;
      if (category === "backups") {
        const backups = getStore<BackupManifest[]>("backups", []);
        return backups.map((backup) => ({
          id: `backup_manifest::${backup.id}`,
          name: backup.filePath?.split("/").pop()
            || `backup-${backup.createdAt}.${backup.version}`,
          path: backup.filePath || "",
          size: backup.fileSize,
          createdAt: backup.createdAt,
          category: "backups",
          hasThumbnail: false,
          missing: !backup.filePath,
        })) as T;
      }
      return [] as T;
    }
    case "open_files_page_entry":
    case "reveal_files_page_entry":
      return undefined as T;
    case "cleanup_missing_files_page_entry": {
      const entryId = (args as { entryId?: string })?.entryId;
      if (entryId?.startsWith("backup_manifest::")) {
        const backupId = entryId.slice("backup_manifest::".length);
        const backups = getStore<BackupManifest[]>("backups", []);
        setStore(
          "backups",
          backups.filter((b) => b.id !== backupId),
        );
      }
      return undefined as T;
    }

    // ── Phase 2: Program Policies ─────────────────────────────────────
    case "list_program_policies":
      return getStore<ProgramPolicy[]>("program_policies", []) as T;
    case "get_program_policies":
      return getStore<ProgramPolicy[]>("program_policies", []) as T;
    case "save_program_policy": {
      const sppList = getStore<ProgramPolicy[]>("program_policies", []);
      const sppInput = (args as { input?: SaveProgramPolicyInput }).input
        ?? ({} as SaveProgramPolicyInput);
      const sppIdx = sppList.findIndex(
        (p) => p.programName === sppInput.programName,
      );
      if (sppIdx >= 0) {
        sppList[sppIdx] = {
          ...sppList[sppIdx],
          allowedProviderIdsJson: JSON.stringify(
            sppInput.allowedProviderIds ?? [],
          ),
          allowedModelIdsJson: JSON.stringify(sppInput.allowedModelIds ?? []),
          defaultProviderId: sppInput.defaultProviderId,
          defaultModelId: sppInput.defaultModelId,
          rateLimitPerMinute: sppInput.rateLimitPerMinute,
        };
        setStore("program_policies", sppList);
        return sppList[sppIdx] as T;
      }
      const sppNew: ProgramPolicy = {
        id: genId(),
        programName: sppInput.programName,
        allowedProviderIdsJson: JSON.stringify(
          sppInput.allowedProviderIds ?? [],
        ),
        allowedModelIdsJson: JSON.stringify(sppInput.allowedModelIds ?? []),
        defaultProviderId: sppInput.defaultProviderId,
        defaultModelId: sppInput.defaultModelId,
        rateLimitPerMinute: sppInput.rateLimitPerMinute,
      };
      sppList.push(sppNew);
      setStore("program_policies", sppList);
      return sppNew as T;
    }
    case "create_program_policy": {
      const pps = getStore<ProgramPolicy[]>("program_policies", []);
      const ppInput = args as {
        programName?: string;
        allowedProviderIds?: string[];
        allowedModelIds?: string[];
      };
      const pp: ProgramPolicy = {
        id: genId(),
        programName: ppInput.programName ?? "",
        allowedProviderIdsJson: JSON.stringify(
          ppInput.allowedProviderIds ?? [],
        ),
        allowedModelIdsJson: JSON.stringify(ppInput.allowedModelIds ?? []),
      };
      pps.push(pp);
      setStore("program_policies", pps);
      return pp as T;
    }
    case "update_program_policy": {
      const pps2 = getStore<ProgramPolicy[]>("program_policies", []);
      const { id, ...ppInput } = args as {
        id?: string;
        programName?: string;
        allowedProviderIds?: string[];
        allowedModelIds?: string[];
        defaultProviderId?: string;
        defaultModelId?: string;
        rateLimitPerMinute?: number;
      };
      const ppi = pps2.findIndex((p) => p.id === id);
      if (ppi >= 0) {
        if (ppInput.programName !== undefined) {
          pps2[ppi].programName = ppInput.programName;
        }
        if (ppInput.allowedProviderIds !== undefined) {
          pps2[ppi].allowedProviderIdsJson = JSON.stringify(
            ppInput.allowedProviderIds,
          );
        }
        if (ppInput.allowedModelIds !== undefined) {
          pps2[ppi].allowedModelIdsJson = JSON.stringify(
            ppInput.allowedModelIds,
          );
        }
        if (ppInput.defaultProviderId !== undefined) {
          pps2[ppi].defaultProviderId = ppInput.defaultProviderId;
        }
        if (ppInput.defaultModelId !== undefined) {
          pps2[ppi].defaultModelId = ppInput.defaultModelId;
        }
        if (ppInput.rateLimitPerMinute !== undefined) {
          pps2[ppi].rateLimitPerMinute = ppInput.rateLimitPerMinute;
        }
        setStore("program_policies", pps2);
        return pps2[ppi] as T;
      }
      return undefined as T;
    }
    case "delete_program_policy": {
      const pps3 = getStore<ProgramPolicy[]>("program_policies", []);
      setStore(
        "program_policies",
        pps3.filter((p) => p.id !== (args as { id?: string })?.id),
      );
      return undefined as T;
    }

    // ── Phase 2: Gateway Diagnostics & Templates ──────────────────────
    case "get_gateway_diagnostics":
      return [
        {
          id: "1",
          category: "port",
          status: "ok",
          message: "Gateway port is available",
          createdAt: nowTs(),
        },
        {
          id: "2",
          category: "auth",
          status: "ok",
          message: "Authentication configured",
          createdAt: nowTs(),
        },
        {
          id: "3",
          category: "proxy",
          status: "ok",
          message: "Proxy settings valid",
          createdAt: nowTs(),
        },
        {
          id: "4",
          category: "provider_latency",
          status: "warning",
          message: "No providers configured",
          createdAt: nowTs(),
        },
      ] as T;
    case "list_gateway_templates":
      return getStore("gateway_templates", [
        {
          id: "tpl-cursor",
          name: "Cursor IDE",
          target: "cursor",
          format: "json",
          content: '{\n  "openai.apiKey": "{{key}}",\n  "openai.apiBaseUrl": "http://localhost:{{port}}/v1"\n}',
          copyHint: i18n.t("browserMock.copyHintCursor"),
          created_at: nowTs(),
          updated_at: nowTs(),
        },
        {
          id: "tpl-vscode",
          name: "VS Code Continue",
          target: "vscode",
          format: "json",
          content:
            '{\n  "models": [{\n    "provider": "openai",\n    "apiBase": "http://localhost:{{port}}/v1",\n    "apiKey": "{{key}}"\n  }]\n}',
          copyHint: i18n.t("browserMock.copyHintContinue"),
          created_at: nowTs(),
          updated_at: nowTs(),
        },
        {
          id: "tpl-claude",
          name: "Claude Code CLI",
          target: "claude_code",
          format: "text",
          content: "ANTHROPIC_BASE_URL=http://localhost:{{port}}/v1\nANTHROPIC_AUTH_TOKEN={{key}}",
          copyHint: i18n.t("browserMock.copyHintEnv"),
          created_at: nowTs(),
          updated_at: nowTs(),
        },
        {
          id: "tpl-openai",
          name: "OpenAI Compatible",
          target: "openai_compatible",
          format: "text",
          content: "API Base: http://localhost:{{port}}/v1\nAPI Key: {{key}}",
          copyHint: i18n.t("browserMock.copyHintOpenAI"),
          created_at: nowTs(),
          updated_at: nowTs(),
        },
      ]) as T;
    case "create_gateway_template": {
      const gts = getStore<Record<string, unknown>[]>("gateway_templates", []);
      const gt = {
        id: genId(),
        ...(args as Record<string, unknown>),
        created_at: nowTs(),
        updated_at: nowTs(),
      };
      gts.push(gt);
      setStore("gateway_templates", gts);
      return gt as T;
    }
    case "delete_gateway_template": {
      const gts2 = getStore<Record<string, unknown>[]>("gateway_templates", []);
      setStore(
        "gateway_templates",
        gts2.filter((g) => g.id !== (args as Record<string, unknown>)?.id),
      );
      return undefined as T;
    }
    case "copy_gateway_template": {
      const cgtList = getStore<Record<string, unknown>[]>(
        "gateway_templates",
        [],
      );
      const cgtMatch = cgtList.find(
        (t) => t.id === (args as Record<string, unknown>)?.templateId,
      );
      return (cgtMatch?.content
        ?? "# Gateway Template Configuration\n\nNo template found.") as T;
    }
    case "apply_gateway_template":
      return { success: true, applied_at: nowTs() } as T;

    // ── Phase 2: Desktop Integration ──────────────────────────────────
    case "get_desktop_capabilities":
      return [
        { key: "tray", supported: false },
        { key: "global_shortcut", supported: true },
        { key: "protocol_handler", supported: false },
        { key: "mini_window", supported: false },
        { key: "notification", supported: "Notification" in globalThis },
      ] as T;
    case "get_window_state":
      return {
        width: globalThis.innerWidth ?? 1280,
        height: globalThis.innerHeight ?? 800,
        focused: true,
        fullscreen: false,
      } as T;
    case "send_desktop_notification": {
      if (
        typeof Notification !== "undefined"
        && Notification.permission === "granted"
      ) {
        new Notification((args as { title?: string })?.title ?? "AxAgent", {
          body: (args as { body?: string })?.body ?? "",
        });
      }
      return undefined as T;
    }
    case "set_always_on_top":
      return undefined as T;
    case "set_close_to_tray":
      return undefined as T;
    case "apply_startup_settings":
      return undefined as T;
    case "set_tray_actions":
      return undefined as T;
    case "handle_protocol_launch":
      return undefined as T;

    // ── Phase 2: Workspace Snapshot ────────────────────────────────────
    case "get_workspace_snapshot": {
      const convId = (args as Record<string, unknown>)?.conversationId as string;
      const branches = getStore<ConversationBranch[]>(`branches_${convId}`, []);
      return {
        searchPolicy: { enabled: false, queryMode: "manual", resultLimit: 10 },
        toolBinding: { serverIds: [], approvalMode: "inherit" },
        knowledgeBinding: { knowledgeBaseIds: [], autoAttach: false },
        memoryPolicy: { enabled: false, writeBack: false },
        toggles: {
          searchEnabled: false,
          enabledKnowledgeBaseIds: [],
          enabledMcpServerIds: [],
          enabledWikiIds: [],
          memoryEnabled: false,
          memoryWriteBack: false,
        },
        researchMode: false,
        pinnedArtifactIds: [],
        branches,
        activeBranchId: null,
      } as T;
    }
    case "update_workspace_snapshot":
      return undefined as T;

    // ── Proxy Test ────────────────────────────────────────────────────────
    case "test_proxy": {
      const addr = (args as Record<string, unknown>)?.proxyAddress;
      if (!addr) {
        return { ok: false, error: "No address" } as T;
      }
      await new Promise((r) => setTimeout(r, 500));
      return {
        ok: true,
        latency_ms: 120 + Math.floor(Math.random() * 200),
      } as T;
    }

    // ── Skills ────────────────────────────────────────────────────────
    case "list_skills":
      return [
        {
          name: "superpowers:brainstorming",
          description: "You MUST use this before any creative work",
          author: "AxAgent",
          version: "1.0.0",
          source: "builtin",
          sourcePath: "builtin://superpowers-brainstorming",
          enabled: true,
          hasUpdate: false,
          userInvocable: true,
          argumentHint: null,
          whenToUse: null,
          group: "superpowers",
          frontend: null,
        },
        {
          name: "superpowers:systematic-debugging",
          description: "Use when encountering any bug, test failure, or unexpected behavior",
          author: "AxAgent",
          version: "1.0.0",
          source: "builtin",
          sourcePath: "builtin://superpowers-debugging",
          enabled: true,
          hasUpdate: false,
          userInvocable: true,
          argumentHint: null,
          whenToUse: null,
          group: "superpowers",
          frontend: null,
        },
        {
          name: "superpowers:writing-plans",
          description: "Use when you have a spec or requirements for a multi-step task",
          author: "AxAgent",
          version: "1.0.0",
          source: "builtin",
          sourcePath: "builtin://superpowers-writing-plans",
          enabled: true,
          hasUpdate: false,
          userInvocable: true,
          argumentHint: null,
          whenToUse: null,
          group: "superpowers",
          frontend: null,
        },
        {
          name: "superpowers:test-driven-development",
          description: "Use when implementing any feature or bugfix, before writing implementation code",
          author: "AxAgent",
          version: "1.0.0",
          source: "builtin",
          sourcePath: "builtin://superpowers-tdd",
          enabled: true,
          hasUpdate: false,
          userInvocable: true,
          argumentHint: null,
          whenToUse: null,
          group: "superpowers",
          frontend: null,
        },
      ] as T;

    case "get_skill":
      return {
        info: {
          name: (args as Record<string, unknown>)?.name || "example",
          description: "Example skill",
          source: "axagent",
          sourcePath: "/mock/path",
          enabled: true,
          hasUpdate: false,
          userInvocable: true,
        },
        content: "# Example Skill\n\nThis is a mock skill for browser preview.",
        files: ["SKILL.md"],
        manifest: null,
      } as T;
      break;

    case "toggle_skill":
      return undefined as T;

    case "install_skill":
      return ((args as Record<string, unknown>)?.source
        || "installed-skill") as T;

    case "uninstall_skill":
      return undefined as T;

    case "uninstall_skill_group":
      return undefined as T;

    case "open_skills_dir":
      return undefined as T;

    case "open_skill_dir":
      return undefined as T;

    case "search_marketplace":
      return [] as T;

    case "check_skill_updates":
      return [] as T;

    case "get_webdav_sync_status":
      return { status: "disabled", lastSync: null, error: null } as T;

    case "restart_webdav_sync":
      return undefined as T;

    // ── Workflow Templates ────────────────────────────────────────────
    case "seed_preset_templates": {
      const existingTemplates = getStore<Record<string, unknown>[]>(
        "workflow_templates",
        [],
      );
      if (existingTemplates.length > 0) {
        return existingTemplates.length as T;
      }
      const presetTemplates = [
        {
          id: "docs",
          name: "Documentation",
          description: "Generate comprehensive documentation",
          icon: "BookOpen",
          tags: ["docs", "api", "readme"],
          version: 1,
          is_preset: true,
          is_editable: false,
          is_public: false,
          trigger_config: { trigger_type: "manual", config: {} },
          nodes: [],
          edges: [],
          input_schema: null,
          output_schema: null,
          variables: [],
          error_config: null,
          created_at: nowTs(),
          updated_at: nowTs(),
        },
        {
          id: "test-gen",
          name: "Test Generation",
          description: "Generate comprehensive test suites",
          icon: "TestTube",
          tags: ["testing", "tdd", "coverage"],
          version: 1,
          is_preset: true,
          is_editable: false,
          is_public: false,
          trigger_config: { trigger_type: "manual", config: {} },
          nodes: [],
          edges: [],
          input_schema: null,
          output_schema: null,
          variables: [],
          error_config: null,
          created_at: nowTs(),
          updated_at: nowTs(),
        },
        {
          id: "refactor",
          name: "Code Refactor",
          description: "Systematic code refactoring with behavior preservation",
          icon: "GitBranch",
          tags: ["refactor", "clean-code", "patterns"],
          version: 1,
          is_preset: true,
          is_editable: false,
          is_public: false,
          trigger_config: { trigger_type: "manual", config: {} },
          nodes: [],
          edges: [],
          input_schema: null,
          output_schema: null,
          variables: [],
          error_config: null,
          created_at: nowTs(),
          updated_at: nowTs(),
        },
        {
          id: "perf-opt",
          name: "Performance Optimization",
          description: "Identify and optimize performance bottlenecks",
          icon: "Zap",
          tags: ["performance", "optimization", "profiling"],
          version: 1,
          is_preset: true,
          is_editable: true,
          is_public: false,
          trigger_config: { trigger_type: "manual", config: {} },
          nodes: [],
          edges: [],
          input_schema: null,
          output_schema: null,
          variables: [],
          error_config: null,
          created_at: nowTs(),
          updated_at: nowTs(),
        },
        {
          id: "migration",
          name: "Migration",
          description: "Framework and language migration workflows",
          icon: "Ship",
          tags: ["migration", "upgrade", "compatibility"],
          version: 1,
          is_preset: true,
          is_editable: true,
          is_public: false,
          trigger_config: { trigger_type: "manual", config: {} },
          nodes: [],
          edges: [],
          input_schema: null,
          output_schema: null,
          variables: [],
          error_config: null,
          created_at: nowTs(),
          updated_at: nowTs(),
        },
        {
          id: "api-design",
          name: "API Design",
          description: "Design and document RESTful APIs",
          icon: "Cloud",
          tags: ["api", "rest", "design"],
          version: 1,
          is_preset: true,
          is_editable: true,
          is_public: false,
          trigger_config: { trigger_type: "manual", config: {} },
          nodes: [],
          edges: [],
          input_schema: null,
          output_schema: null,
          variables: [],
          error_config: null,
          created_at: nowTs(),
          updated_at: nowTs(),
        },
        {
          id: "env-debug",
          name: "Environment Debug",
          description: "Debug and diagnose environment issues",
          icon: "Bug",
          tags: ["debug", "troubleshoot", "environment"],
          version: 1,
          is_preset: true,
          is_editable: true,
          is_public: false,
          trigger_config: { trigger_type: "manual", config: {} },
          nodes: [],
          edges: [],
          input_schema: null,
          output_schema: null,
          variables: [],
          error_config: null,
          created_at: nowTs(),
          updated_at: nowTs(),
        },
        {
          id: "feature-impl",
          name: "Feature Implementation",
          description: "Implement new features with AI assistance",
          icon: "Sparkles",
          tags: ["feature", "ai", "implementation"],
          version: 1,
          is_preset: true,
          is_editable: true,
          is_public: false,
          trigger_config: { trigger_type: "manual", config: {} },
          nodes: [],
          edges: [],
          input_schema: null,
          output_schema: null,
          variables: [],
          error_config: null,
          created_at: nowTs(),
          updated_at: nowTs(),
        },
        {
          id: "knowledge-extract",
          name: "Knowledge Extraction",
          description: "Extract structured knowledge from documents",
          icon: "Brain",
          tags: ["knowledge", "extraction", "nlp"],
          version: 1,
          is_preset: true,
          is_editable: true,
          is_public: false,
          trigger_config: { trigger_type: "manual", config: {} },
          nodes: [],
          edges: [],
          input_schema: null,
          output_schema: null,
          variables: [],
          error_config: null,
          created_at: nowTs(),
          updated_at: nowTs(),
        },
        {
          id: "knowledge-to-code",
          name: "Knowledge to Code",
          description: "Convert knowledge into executable code",
          icon: "Code",
          tags: ["knowledge", "code", "generation"],
          version: 1,
          is_preset: true,
          is_editable: true,
          is_public: false,
          trigger_config: { trigger_type: "manual", config: {} },
          nodes: [],
          edges: [],
          input_schema: null,
          output_schema: null,
          variables: [],
          error_config: null,
          created_at: nowTs(),
          updated_at: nowTs(),
        },
        {
          id: "custom-1",
          name: "My Custom Workflow",
          description: "A custom workflow created by user",
          icon: "Star",
          tags: ["custom", "user"],
          version: 1,
          is_preset: false,
          is_editable: true,
          is_public: false,
          trigger_config: { trigger_type: "manual", config: {} },
          nodes: [],
          edges: [],
          input_schema: null,
          output_schema: null,
          variables: [],
          error_config: null,
          created_at: nowTs(),
          updated_at: nowTs(),
        },
        {
          id: "workflow-finance-invest",
          name: "金融投资分析",
          description: "财务分析 → 投资建议（CFO 视角）",
          icon: "LineChart",
          tags: ["opc", "finance", "investment"],
          version: 1,
          is_preset: true,
          is_editable: true,
          is_public: false,
          trigger_config: { trigger_type: "manual", config: {} },
          nodes: [],
          edges: [],
          input_schema: null,
          output_schema: null,
          variables: [],
          error_config: null,
          created_at: nowTs(),
          updated_at: nowTs(),
        },
        {
          id: "workflow-finance-stock-research",
          name: "个股投资研究",
          description: "A股个股深度研究：行情→财报→资金流→投资结论（astock-data 工具链）",
          icon: "TrendingUp",
          tags: ["opc", "finance", "stock", "astock"],
          version: 1,
          is_preset: true,
          is_editable: true,
          is_public: false,
          trigger_config: { trigger_type: "manual", config: {} },
          nodes: [],
          edges: [],
          input_schema: null,
          output_schema: null,
          variables: [],
          error_config: null,
          created_at: nowTs(),
          updated_at: nowTs(),
        },
        {
          id: "workflow-ai-research",
          name: "AI 科技研究报告",
          description: "AI 技术调研、原型验证与报告",
          icon: "Robot",
          tags: ["opc", "ai", "research"],
          version: 1,
          is_preset: true,
          is_editable: true,
          is_public: false,
          trigger_config: { trigger_type: "manual", config: {} },
          nodes: [],
          edges: [],
          input_schema: null,
          output_schema: null,
          variables: [],
          error_config: null,
          created_at: nowTs(),
          updated_at: nowTs(),
        },
        {
          id: "workflow-software-dev",
          name: "软件开发流程",
          description: "需求→架构→编码→测试→部署",
          icon: "Code",
          tags: ["opc", "dev", "software"],
          version: 1,
          is_preset: true,
          is_editable: true,
          is_public: false,
          trigger_config: { trigger_type: "manual", config: {} },
          nodes: [],
          edges: [],
          input_schema: null,
          output_schema: null,
          variables: [],
          error_config: null,
          created_at: nowTs(),
          updated_at: nowTs(),
        },
        {
          id: "workflow-accounting",
          name: "会计财务流程",
          description: "发票创建→审批→通知→KPI 记录",
          icon: "Calculator",
          tags: ["opc", "finance", "accounting"],
          version: 1,
          is_preset: true,
          is_editable: true,
          is_public: false,
          trigger_config: { trigger_type: "manual", config: {} },
          nodes: [],
          edges: [],
          input_schema: null,
          output_schema: null,
          variables: [],
          error_config: null,
          created_at: nowTs(),
          updated_at: nowTs(),
        },
        {
          id: "workflow-ecommerce",
          name: "电商运营流程",
          description: "选品→店铺→营销→订单",
          icon: "ShoppingCart",
          tags: ["opc", "ecommerce", "brand"],
          version: 1,
          is_preset: true,
          is_editable: true,
          is_public: false,
          trigger_config: { trigger_type: "manual", config: {} },
          nodes: [],
          edges: [],
          input_schema: null,
          output_schema: null,
          variables: [],
          error_config: null,
          created_at: nowTs(),
          updated_at: nowTs(),
        },
        {
          id: "workflow-education",
          name: "教育培训流程",
          description: "课程设计→内容制作→学员管理→评估",
          icon: "GraduationCap",
          tags: ["opc", "education", "training"],
          version: 1,
          is_preset: true,
          is_editable: true,
          is_public: false,
          trigger_config: { trigger_type: "manual", config: {} },
          nodes: [],
          edges: [],
          input_schema: null,
          output_schema: null,
          variables: [],
          error_config: null,
          created_at: nowTs(),
          updated_at: nowTs(),
        },
        {
          id: "prod-startup-mvp",
          name: "Startup MVP 流水线",
          description: "从想法到 MVP 的完整生产流程",
          icon: "Rocket",
          tags: ["opc", "production", "startup", "mvp"],
          version: 1,
          is_preset: true,
          is_editable: true,
          is_public: false,
          trigger_config: { trigger_type: "manual", config: {} },
          nodes: [],
          edges: [],
          input_schema: null,
          output_schema: null,
          variables: [],
          error_config: null,
          created_at: nowTs(),
          updated_at: nowTs(),
        },
        {
          id: "prod-landing-page",
          name: "落地页营销工作流",
          description: "创建→发布→跟踪落地页效果",
          icon: "FileText",
          tags: ["opc", "production", "landing"],
          version: 1,
          is_preset: true,
          is_editable: true,
          is_public: false,
          trigger_config: { trigger_type: "manual", config: {} },
          nodes: [],
          edges: [],
          input_schema: null,
          output_schema: null,
          variables: [],
          error_config: null,
          created_at: nowTs(),
          updated_at: nowTs(),
        },
      ];
      setStore("workflow_templates", presetTemplates);
      return presetTemplates.length as T;
    }
    case "list_workflow_templates": {
      const is_preset = (args as { is_preset?: boolean })?.is_preset;
      let templates = getStore<WorkflowTemplate[]>("workflow_templates", []);
      if (is_preset !== undefined) {
        templates = templates.filter((t) => t.is_preset === is_preset);
      }
      return templates as T;
    }

    // ── Gateway Links ─────────────────────────────────────────────────
    case "list_gateway_links":
      return getStore("gateway_links", []) as T;

    // ── Workflow Templates ────────────────────────────────────────────
    case "get_workflow_template": {
      const id = (args as { id?: string })?.id;
      const templates = getStore<WorkflowTemplate[]>("workflow_templates", []);
      return (templates.find((t) => t.id === id) || null) as T;
    }
    case "create_workflow_template": {
      const input = (args as { input?: CreateWorkflowTemplateInput }).input ?? {};
      const newId = genId();
      const now = nowTs();
      const template: WorkflowTemplate = {
        id: newId,
        name: input.name || "Unnamed Workflow",
        description: input.description || "",
        icon: "Bot",
        tags: input.tags || [],
        version: 1,
        is_preset: false,
        is_editable: true,
        is_public: false,
        trigger_config: { type: "manual", config: {} },
        nodes: input.nodes?.length
          ? input.nodes
          : [
            {
              id: genId(),
              type: "trigger",
              label: i18n.t("browserMock.triggerLabel"),
              config: { trigger_type: "manual" },
              position: { x: 100, y: 100 },
            },
          ],
        edges: input.edges || [],
        created_at: now,
        updated_at: now,
      };
      const templates = getStore<WorkflowTemplate[]>("workflow_templates", []);
      templates.push(template);
      setStore("workflow_templates", templates);
      return newId as T;
    }
    case "update_workflow_template": {
      const updateId = (args as { id?: string }).id;
      const updateInput = (args as { input?: UpdateWorkflowTemplateInput }).input ?? {};
      const templates = getStore<WorkflowTemplate[]>("workflow_templates", []);
      const idx = templates.findIndex((t) => t.id === updateId);
      if (idx >= 0) {
        if (updateInput.name !== undefined) {
          templates[idx].name = updateInput.name;
        }
        if (updateInput.description !== undefined) {
          templates[idx].description = updateInput.description;
        }
        if (updateInput.tags !== undefined) {
          templates[idx].tags = updateInput.tags;
        }
        if (updateInput.nodes !== undefined) {
          templates[idx].nodes = updateInput.nodes;
        }
        if (updateInput.edges !== undefined) {
          templates[idx].edges = updateInput.edges;
        }
        templates[idx].updated_at = nowTs();
        setStore("workflow_templates", templates);
      }
      return true as T;
    }
    case "delete_workflow_template": {
      const deleteId = (args as { id?: string }).id;
      const templates = getStore<WorkflowTemplate[]>("workflow_templates", []);
      setStore(
        "workflow_templates",
        templates.filter((t) => t.id !== deleteId),
      );
      return undefined as T;
    }

    // Platform / Message Channel commands
    case "get_platform_config": {
      return (getStore<PlatformConfig | null>("platform_config", null) ?? {
        telegram_enabled: false,
        telegram_bot_token: null,
        telegram_webhook_url: null,
        telegram_webhook_secret: null,
        telegram_allowed_users: null,
        discord_enabled: false,
        discord_bot_token: null,
        discord_webhook_url: null,
        discord_allowed_channels: null,
        slack_enabled: false,
        slack_bot_token: null,
        slack_signing_secret: null,
        slack_workspace_id: null,
        slack_app_token: null,
        whatsapp_enabled: false,
        whatsapp_phone_number_id: null,
        whatsapp_access_token: null,
        whatsapp_business_account_id: null,
        whatsapp_webhook_verify_token: null,
        whatsapp_api_version: null,
        wechat_enabled: false,
        wechat_app_id: null,
        wechat_app_secret: null,
        wechat_token: null,
        wechat_encoding_aes_key: null,
        wechat_original_id: null,
        wechat_mode: null,
        feishu_enabled: false,
        feishu_app_id: null,
        feishu_app_secret: null,
        feishu_verification_token: null,
        feishu_encrypt_key: null,
        qq_enabled: false,
        qq_bot_app_id: null,
        qq_bot_token: null,
        qq_bot_secret: null,
        dingtalk_enabled: false,
        dingtalk_app_key: null,
        dingtalk_app_secret: null,
        dingtalk_agent_id: null,
        dingtalk_robot_code: null,
        api_server_enabled: false,
        api_server_port: 8080,
        auto_sync_messages: false,
        max_history_per_session: 100,
      }) as T;
    }
    case "update_platform_config": {
      const input = args as Partial<PlatformConfig>;
      const existing = getStore<PlatformConfig | null>("platform_config", null)
        ?? ({} as PlatformConfig);
      const merged = { ...existing, ...input };
      setStore("platform_config", merged);
      return undefined as T;
    }
    case "get_platform_statuses": {
      const config = getStore<PlatformConfig | null>("platform_config", null);
      if (!config) {
        return [] as T;
      }
      const keys: { key: keyof PlatformConfig; name: string }[] = [
        { key: "telegram_enabled", name: "Telegram" },
        { key: "discord_enabled", name: "Discord" },
        { key: "slack_enabled", name: "Slack" },
        { key: "whatsapp_enabled", name: "WhatsApp" },
        { key: "wechat_enabled", name: "WeChat" },
        { key: "feishu_enabled", name: "Feishu" },
        { key: "qq_enabled", name: "QQ" },
        { key: "dingtalk_enabled", name: "DingTalk" },
      ];
      return keys.map(({ key, name }) => ({
        name,
        enabled: !!config[key],
        connected: false,
        last_activity: null,
        active_sessions: 0,
      })) as T;
    }
    case "reconcile_platforms": {
      return { started: [], stopped: [], errors: [] } as T;
    }
    case "get_active_sessions": {
      return getStore<PlatformSession[]>("platform_sessions", []) as T;
    }
    case "create_platform_session": {
      const input = args as { platform: string; chat_id: string };
      const sessions = getStore<PlatformSession[]>("platform_sessions", []);
      const session: PlatformSession = {
        session_id: `mock-${input.platform}-${Date.now()}`,
        platform: input.platform,
        user_id: input.chat_id,
        username: null,
        is_active: true,
        last_activity: Date.now(),
      };
      sessions.push(session);
      setStore("platform_sessions", sessions);
      return session as T;
    }
    case "deactivate_platform_session": {
      const input = args as { sessionId: string };
      const sessions = getStore<PlatformSession[]>("platform_sessions", []);
      setStore(
        "platform_sessions",
        sessions.map((s) => s.session_id === input.sessionId ? { ...s, is_active: false } : s),
      );
      return undefined as T;
    }
    case "send_platform_message": {
      return { ok: true, message_id: `mock-msg-${Date.now()}` } as T;
    }
    case "process_telegram_message":
    case "process_discord_message":
    case "process_wechat_message":
    case "process_feishu_message":
    case "process_qq_message":
    case "process_dingtalk_message":
    case "process_slack_message":
    case "process_whatsapp_message": {
      return { success: true, reply_sent: false } as T;
    }
    case "start_api_server": {
      setStore("api_server_running", true);
      return { port: (args as { port?: number }).port ?? 8080 } as T;
    }
    case "stop_api_server": {
      setStore("api_server_running", false);
      return undefined as T;
    }

    // ── Plugins (OpenClaw) ─────────────────────────────────────────────
    case "plugin_list": {
      const plugins = getStore<Array<Record<string, unknown>>>("plugins", []);
      return plugins as T;
    }
    case "plugin_validate_source": {
      const source = (args?.source as string) || "";
      return {
        name: source.split("/").pop() || source,
        version: "0.0.0",
        description: `Plugin from ${source}`,
        permissions: [],
        default_enabled: true,
        hooks: {},
        tools: [],
        mcp_servers: [],
        skills: [],
      } as T;
    }
    case "plugin_install": {
      const plugins = getStore<Array<Record<string, unknown>>>("plugins", []);
      const source = (args?.source as string) || "";
      const id = `plugin-${plugins.length + 1}`;
      plugins.push({
        id,
        name: source.split("/").pop() || source,
        version: "0.0.0",
        description: `Plugin from ${source}`,
        kind: "openclaw",
        enabled: true,
        tool_names: [],
        mcp_server_names: [],
        skill_names: [],
      });
      setStore("plugins", plugins);
      return {
        plugin_id: id,
        version: "0.0.0",
        install_path: `/mock/plugins/${id}`,
      } as T;
    }
    case "plugin_enable":
    case "plugin_disable": {
      const allPlugins = getStore<Array<Record<string, unknown>>>(
        "plugins",
        [],
      );
      const pluginId = (args?.pluginId as string) || "";
      const idx = allPlugins.findIndex((p) => p.id === pluginId);
      if (idx !== -1) {
        allPlugins[idx] = {
          ...allPlugins[idx],
          enabled: cmd === "plugin_enable",
        };
        setStore("plugins", allPlugins);
      }
      return undefined as T;
    }
    case "plugin_uninstall": {
      let allPlugins = getStore<Array<Record<string, unknown>>>("plugins", []);
      const pluginId = (args?.pluginId as string) || "";
      allPlugins = allPlugins.filter((p) => p.id !== pluginId);
      setStore("plugins", allPlugins);
      return undefined as T;
    }
    case "plugin_update": {
      return {
        plugin_id: (args?.pluginId as string) || "",
        version: "0.0.0",
        install_path: "",
      } as T;
    }

    // ── Agent Profiles (mock) ──────────────────────────────────────
    case "list_agent_profiles":
    case "list_agent_roles":
      return [] as T;

    // ── Dashboard Plugins (mock) ────────────────────────────────────
    case "dashboard_list_plugins":
      return [] as T;

    // ── Prompt Templates (mock) ─────────────────────────────────────
    case "list_prompt_templates":
      return [] as T;

    // ── PTY Terminal (mock) ──────────────────────────────────────────
    case "pty_create_session":
      return `pty-mock-${Date.now()}` as T;
    case "pty_write":
    case "pty_resize":
    case "pty_kill_session":
    case "pty_remove_session":
    case "pty_analyze_output":
    case "pty_clear_output":
      return null as T;

    // ── NL-to-Workflow ────────────────────────────────────────────────
    case "generate_workflow_from_prompt": {
      return {
        nodes: [
          {
            id: "trigger-1",
            type: "trigger",
            label: i18n.t("browserMock.triggerLabel"),
            config: { trigger_type: "manual" },
            position: { x: 100, y: 100 },
          },
          {
            id: "action-1",
            type: "tool",
            label: i18n.t("browserMock.executeLabel"),
            config: { tool_name: "mock_tool" },
            position: { x: 300, y: 100 },
          },
        ],
        edges: [
          { id: "e1", source: "trigger-1", target: "action-1" },
        ],
        explanation: i18n.t("browserMock.nlWorkflowExplanation", {
          prompt: (args as { prompt?: string })?.prompt ?? "",
        }),
      } as T;
    }

    // ── DynamicUI Schema CRUD ──────────────────────────────────────────
    case "list_dynamic_ui_schemas": {
      const { category } = (args as { category?: string | null }) ?? {};
      const schemas = loadMockDynamicUIData<DynamicUISchemaRecord[]>("schemas", []);
      const filtered = category
        ? schemas.filter((s) => s.category === category)
        : schemas;
      return filtered as T;
    }
    case "get_dynamic_ui_schema": {
      const { id } = args as { id?: string };
      const schemas = loadMockDynamicUIData<DynamicUISchemaRecord[]>("schemas", []);
      const schema = schemas.find((s) => s.id === id) ?? null;
      return schema as T;
    }
    case "create_dynamic_ui_schema": {
      const req = (args as { req?: CreateDynamicUISchemaParams }).req;
      if (!req) {
        throw new Error("Missing req parameter");
      }
      const now = new Date().toISOString();
      const schema: DynamicUISchemaRecord = {
        id: genId(),
        title: req.title,
        description: req.description,
        schema_json: req.schema_json,
        category: req.category,
        tags: req.tags,
        version: "1.0.0",
        is_builtin: false,
        created_at: now,
        updated_at: now,
      };
      const schemas = loadMockDynamicUIData<DynamicUISchemaRecord[]>("schemas", []);
      schemas.push(schema);
      saveMockDynamicUIData("schemas", schemas);
      // 创建初始版本记录
      const versions = loadMockDynamicUIData<DynamicUISchemaVersion[]>("versions", []);
      versions.push({
        id: Date.now(),
        schema_id: schema.id,
        version: schema.version,
        title: schema.title,
        description: schema.description,
        schema_json: schema.schema_json,
        category: schema.category,
        tags: schema.tags,
        change_log: "initial create",
        created_at: Date.now(),
      });
      saveMockDynamicUIData("versions", versions);
      return schema as T;
    }
    case "update_dynamic_ui_schema": {
      const { id, req } = args as {
        id?: string;
        req?: UpdateDynamicUISchemaParams;
      };
      if (!id || !req) {
        throw new Error("Missing id or req parameter");
      }
      const schemas = loadMockDynamicUIData<DynamicUISchemaRecord[]>("schemas", []);
      const idx = schemas.findIndex((s) => s.id === id);
      if (idx === -1) {
        throw new Error("Schema not found");
      }
      const old = schemas[idx];
      // 版本号：传了用传入值，否则 patch 自增
      const newVersion = req.version ?? bumpPatchVersion(old.version);
      const updated: DynamicUISchemaRecord = {
        ...old,
        title: req.title ?? old.title,
        description: req.description ?? old.description,
        schema_json: req.schema_json ?? old.schema_json,
        category: req.category ?? old.category,
        tags: req.tags ?? old.tags,
        version: newVersion,
        updated_at: new Date().toISOString(),
      };
      schemas[idx] = updated;
      saveMockDynamicUIData("schemas", schemas);
      // 创建版本记录
      const versions = loadMockDynamicUIData<DynamicUISchemaVersion[]>("versions", []);
      versions.push({
        id: Date.now(),
        schema_id: updated.id,
        version: updated.version,
        title: updated.title,
        description: updated.description,
        schema_json: updated.schema_json,
        category: updated.category,
        tags: updated.tags,
        change_log: req.change_log ?? "update",
        created_at: Date.now(),
      });
      saveMockDynamicUIData("versions", versions);
      return updated as T;
    }
    case "delete_dynamic_ui_schema": {
      const { id } = args as { id?: string };
      const schemas = loadMockDynamicUIData<DynamicUISchemaRecord[]>("schemas", []);
      saveMockDynamicUIData(
        "schemas",
        schemas.filter((s) => s.id !== id),
      );
      // 同步清理版本记录
      const versions = loadMockDynamicUIData<DynamicUISchemaVersion[]>("versions", []);
      saveMockDynamicUIData(
        "versions",
        versions.filter((v) => v.schema_id !== id),
      );
      return undefined as T;
    }

    // ── DynamicUI 表单数据持久化 ──────────────────────────────────────
    case "save_dynamic_ui_form_data": {
      const req = (args as { req?: SaveDynamicUIFormDataParams }).req;
      if (!req) {
        throw new Error("Missing req parameter");
      }
      const instanceKey = req.instance_key ?? "__default__";
      const records = loadMockDynamicUIData<DynamicUIFormDataRecord[]>("formData", []);
      const idx = records.findIndex(
        (r) => r.schema_id === req.schema_id && r.instance_key === instanceKey,
      );
      const now = new Date().toISOString();
      const record: DynamicUIFormDataRecord = {
        id: idx !== -1 ? records[idx].id : genId(),
        schema_id: req.schema_id,
        instance_key: instanceKey,
        form_data_json: req.form_data_json,
        updated_at: now,
      };
      if (idx !== -1) {
        records[idx] = record;
      } else {
        records.push(record);
      }
      saveMockDynamicUIData("formData", records);
      return record as T;
    }
    case "get_dynamic_ui_form_data": {
      const { schema_id, instance_key } = args as {
        schema_id?: string;
        instance_key?: string | null;
      };
      const instanceKey = instance_key ?? "__default__";
      const records = loadMockDynamicUIData<DynamicUIFormDataRecord[]>("formData", []);
      const record = records.find(
        (r) => r.schema_id === schema_id && r.instance_key === instanceKey,
      ) ?? null;
      return record as T;
    }
    case "delete_dynamic_ui_form_data": {
      const { schema_id, instance_key } = args as {
        schema_id?: string;
        instance_key?: string | null;
      };
      const instanceKey = instance_key ?? "__default__";
      const records = loadMockDynamicUIData<DynamicUIFormDataRecord[]>("formData", []);
      saveMockDynamicUIData(
        "formData",
        records.filter(
          (r) => !(r.schema_id === schema_id && r.instance_key === instanceKey),
        ),
      );
      return undefined as T;
    }

    // ── DynamicUI 钉入配置 ────────────────────────────────────────────
    case "list_dynamic_ui_pins": {
      const pins = loadMockDynamicUIData<DynamicUIPinRecord[]>("pins", []);
      return pins as T;
    }
    case "pin_dynamic_ui_schema": {
      const { schema_id, title, group_name, position } = args as {
        schema_id?: string;
        title?: string;
        group_name?: string;
        position?: number | null;
      };
      if (!schema_id) {
        throw new Error("Missing schema_id");
      }
      const pins = loadMockDynamicUIData<DynamicUIPinRecord[]>("pins", []);
      const idx = pins.findIndex((p) => p.schema_id === schema_id);
      const now = new Date().toISOString();
      const pos = position
        ?? (pins.length > 0 ? Math.max(...pins.map((p) => p.position)) + 1 : 0);
      const record: DynamicUIPinRecord = {
        schema_id,
        title: title ?? "",
        group_name: group_name ?? "",
        position: pos,
        created_at: idx !== -1 ? pins[idx].created_at : now,
        updated_at: now,
      };
      if (idx !== -1) {
        pins[idx] = record;
      } else {
        pins.push(record);
      }
      saveMockDynamicUIData("pins", pins);
      return record as T;
    }
    case "unpin_dynamic_ui_schema": {
      const { schema_id } = args as { schema_id?: string };
      const pins = loadMockDynamicUIData<DynamicUIPinRecord[]>("pins", []);
      saveMockDynamicUIData(
        "pins",
        pins.filter((p) => p.schema_id !== schema_id),
      );
      return undefined as T;
    }

    // ── DynamicUI 版本管理 ────────────────────────────────────────────
    case "list_dynamic_ui_schema_versions": {
      const { schema_id } = args as { schema_id?: string };
      const versions = loadMockDynamicUIData<DynamicUISchemaVersion[]>("versions", []);
      const filtered = versions.filter((v) => v.schema_id === schema_id);
      const schemas = loadMockDynamicUIData<DynamicUISchemaRecord[]>("schemas", []);
      const schema = schemas.find((s) => s.id === schema_id);
      const response: ListVersionsResponse = {
        versions: filtered,
        current_version: schema?.version ?? "",
      };
      return response as T;
    }
    case "get_dynamic_ui_schema_version": {
      const { version_id } = args as { version_id?: number };
      const versions = loadMockDynamicUIData<DynamicUISchemaVersion[]>("versions", []);
      const version = versions.find((v) => v.id === version_id) ?? null;
      return version as T;
    }
    case "restore_dynamic_ui_schema_version": {
      const { schema_id, version_id } = args as {
        schema_id?: string;
        version_id?: number;
      };
      const versions = loadMockDynamicUIData<DynamicUISchemaVersion[]>("versions", []);
      const version = versions.find(
        (v) => v.id === version_id && v.schema_id === schema_id,
      );
      if (!version) {
        throw new Error("Version not found");
      }
      const schemas = loadMockDynamicUIData<DynamicUISchemaRecord[]>("schemas", []);
      const idx = schemas.findIndex((s) => s.id === schema_id);
      if (idx === -1) {
        throw new Error("Schema not found");
      }
      const restored: DynamicUISchemaRecord = {
        id: schemas[idx].id,
        title: version.title,
        description: version.description,
        schema_json: version.schema_json,
        category: version.category,
        tags: version.tags,
        version: version.version,
        is_builtin: schemas[idx].is_builtin,
        created_at: schemas[idx].created_at,
        updated_at: new Date().toISOString(),
      };
      schemas[idx] = restored;
      saveMockDynamicUIData("schemas", schemas);
      return restored as T;
    }

    // ── DynamicUI 自然语言生成 ────────────────────────────────────────
    case "edit_dynamic_ui_schema_nl": {
      const { prompt } = args as { existing_schema?: string; prompt?: string };
      const mockSchema = buildMockUISchemaJSON();
      return {
        schema: mockSchema,
        description: `Mock: edited example schema per instruction "${(prompt ?? "").slice(0, 50)}"`,
      } as T;
    }
    case "generate_dynamic_ui_schema_nl": {
      const { prompt } = args as { prompt?: string };
      const mockSchema = buildMockUISchemaJSON();
      return {
        schema: mockSchema,
        title: "Mock Dynamic UI",
        description: `Mock: generated example schema per description "${(prompt ?? "").slice(0, 50)}"`,
      } as T;
    }

    // ── DevTools: Tracer (轨迹追踪) ───────────────────────────────────
    case "tracer_start_span":
      return genId() as T;
    case "tracer_end_span":
    case "tracer_record_error":
    case "tracer_record_span":
    case "tracer_delete_trace":
    case "tracer_submit_feedback":
    case "telemetry_report_error":
      return undefined as T;
    case "tracer_list_traces":
    case "tracer_get_feedback":
    case "tracer_generate_suggestions":
      return [] as T;
    case "tracer_get_trace":
    case "tracer_get_span":
    case "tracer_get_metrics":
      return null as T;
    case "tracer_export_traces":
      // 后端返回 Vec<u8>，浏览器 mock 用空数组占位
      return [] as unknown as T;
    case "tracer_delete_old_traces":
      return 0 as T;
    case "tracer_get_bottlenecks":
      return { bottlenecks: [], summary: {} } as T;

    // ── DevTools: Evaluator (评估器) ──────────────────────────────────
    case "evaluator_list_benchmarks":
    case "evaluator_list_datasets":
      return [] as T;
    case "evaluator_get_benchmark":
    case "evaluator_get_ab_results":
      return null as T;
    case "evaluator_run_benchmark":
      return {
        benchmark_id: "",
        status: "completed",
        results: [],
        started_at: nowTs(),
        completed_at: nowTs(),
      } as T;
    case "evaluator_generate_report":
      return {
        id: genId(),
        benchmark_id: "",
        generated_at: nowTs(),
        summary: {},
        metrics: {},
      } as T;
    case "evaluator_import_dataset":
      return {
        id: genId(),
        name: "Mock Dataset",
        source_path: (args as { path?: string })?.path ?? "",
        size: 0,
        created_at: nowTs(),
      } as T;
    case "evaluator_export_report":
      return "" as T;
    case "evaluator_run_ab_test":
      return {
        test_id: genId(),
        status: "completed",
        variants: {},
        started_at: nowTs(),
        completed_at: nowTs(),
      } as T;

    // ── DevTools: RL Training (强化学习) ──────────────────────────────
    case "start_rl_training":
    case "stop_rl_training":
    case "load_checkpoint":
    case "delete_checkpoint":
      return undefined as T;
    case "get_training_metrics":
      return {
        episode: 0,
        reward: 0,
        loss: 0,
        steps: 0,
        avg_reward: 0,
        epsilon: 0,
        learning_rate: 0,
      } as T;
    case "save_checkpoint":
      return {
        id: genId(),
        episode: 0,
        reward: 0,
        created_at: nowTs(),
        path: "/mock/checkpoint.pt",
      } as T;
    case "list_checkpoints":
      return [] as T;
    case "run_rl_training_step":
      return { step: 0, reward: 0, loss: 0, done: false } as T;

    // ── DevTools: Fine-tune (微调) ────────────────────────────────────
    case "list_datasets":
      return [] as T;
    case "get_dataset":
      return {
        id: "",
        name: "",
        description: "",
        sample_count: 0,
        created_at: 0,
      } as T;
    case "create_dataset":
      return {
        id: genId(),
        name: (args as { name?: string })?.name ?? "Mock Dataset",
        description: (args as { description?: string })?.description ?? "",
        sample_count: 0,
        created_at: nowTs(),
      } as T;
    case "add_sample":
    case "delete_dataset":
    case "cancel_training_job":
    case "delete_training_job":
    case "start_training_job":
    case "set_active_model":
      return undefined as T;
    case "list_training_jobs":
      return [] as T;
    case "get_training_job":
      return {
        id: "",
        status: "pending",
        progress: 0,
        created_at: 0,
      } as T;
    case "create_training_job":
      return {
        id: genId(),
        dataset_id: (args as { dataset_id?: string })?.dataset_id ?? "",
        base_model: (args as { base_model?: string })?.base_model ?? "",
        status: "pending",
        progress: 0,
        created_at: nowTs(),
      } as T;
    case "get_training_stats":
      return {
        total_jobs: 0,
        active_jobs: 0,
        completed_jobs: 0,
        failed_jobs: 0,
        total_samples: 0,
      } as T;
    case "list_base_models":
    case "list_lora_adapters":
      return [] as T;
    case "get_active_model":
      return null as T;

    // ── DevTools: Agent Analytics (智能体分析) ────────────────────────
    case "trajectory_stats":
      return {
        total_trajectories: 0,
        total_steps: 0,
        avg_steps: 0,
        avg_quality: 0,
        success_rate: 0,
      } as T;
    case "trajectory_list":
      return [] as T;
    case "get_trajectory_detail":
      return null as T;
    case "pattern_stats":
      return {
        total_patterns: 0,
        high_value_patterns: 0,
        failure_patterns: 0,
        avg_success_rate: 0,
      } as T;
    case "closed_loop_status":
      return {
        closed_loop_running: false,
        nudge_count: 0,
        pattern_count: 0,
        insight_count: 0,
      } as T;
    case "rl_config":
      return { config: {}, weights: {} } as T;
    case "rl_export_training_data":
      return [] as T;
    case "rl_compute_rewards":
      return {
        trajectory_id: (args as { trajectory_id?: string })?.trajectory_id
          ?? "",
        reward_count: 0,
        total_reward: 0,
        value_count: 0,
        advantage_count: 0,
      } as T;
    case "record_feedback":
      return undefined as T;

    // ── DevTools: Evolution (进化) ────────────────────────────────────
    case "pattern_list":
    case "cross_session_insights":
      return [] as T;
    case "skill_evolution_start":
      return {
        skill_id: (args as { skill_id?: string })?.skill_id ?? "",
        improved: false,
        reason: "Mock: no evolution result generated",
        confidence: 0,
      } as T;
    case "skill_evolution_status":
      return { is_running: false, stats: {} } as T;
    case "adaptation_status":
      return {
        response_style: "",
        content_adjustments: [],
        skill_suggestions: [],
        memory_priorities: [],
      } as T;

    // ── DevTools: Evolution Engine (进化引擎) ─────────────────────────
    case "get_all_engine_status":
      return [] as T;
    case "start_engine":
    case "stop_engine":
      return undefined as T;
    case "update_engine_config":
      return { success: true } as T;
    case "get_engine_logs":
      return [] as T;
    case "run_skill_evolution_generation":
    case "run_text_grad_optimize":
    case "run_dream_consolidation":
    case "run_auto_tool_create":
    case "run_process_reward_analysis":
    case "run_intrinsic_motivation_analysis":
    case "run_coevolution_cycle":
    case "run_sandbox_validate_step":
      return { success: false, reason: "Mock mode not enabled", stats: {} } as T;
    case "get_coevolution_status":
      return { is_running: false, generation: 0, stats: {} } as T;
    case "get_sandbox_policy":
      return { enabled: false, rules: [] } as T;
    case "extract_entities_for_kb":
    case "extract_entities_from_documents":
    case "batch_upsert_entities_and_relations":
      return {
        newEntities: [],
        updatedEntities: [],
        newRelations: [],
        skippedChunks: 0,
        elapsedMs: 0,
      } as T;

    // ── OPC Industries ──────────────────────────────────────────────
    case "opc_get_industry_pack": {
      const industryId = (args as { industry_id?: string })?.industry_id || "";
      const industryManifests: Record<string, {
        id: string;
        name: string;
        icon: string;
        description: string;
        version: number;
        enabled: boolean;
      }> = {
        "ai-research": {
          id: "ai_research",
          name: "人工智能研究",
          icon: "🤖",
          description: "AI 技术与应用研究报告：需求分析 → 技术调研 → 原型验证 → 报告输出",
          version: 1,
          enabled: true,
        },
        "software-dev": {
          id: "software_dev",
          name: "软件开发",
          icon: "💻",
          description: "软件开发全流程：需求分析 → 架构设计 → 编码实现 → 测试部署",
          version: 1,
          enabled: true,
        },
        "finance-invest": {
          id: "finance_invest",
          name: "金融投资",
          icon: "📈",
          description: "投资分析与决策：市场分析 → 财务建模 → 风险评估 → 投资建议",
          version: 1,
          enabled: true,
        },
        "sales-growth": {
          id: "sales_growth",
          name: "销售增长",
          icon: "🚀",
          description: "销售增长策略：市场定位 → 客户画像 → 获客策略 → 转化优化",
          version: 1,
          enabled: true,
        },
        "content-media": {
          id: "content_media",
          name: "内容媒体",
          icon: "📝",
          description: "内容创作与传播：选题策划 → 内容生产 → 多平台分发 → 效果追踪",
          version: 1,
          enabled: true,
        },
        "industry-consulting": {
          id: "industry_consulting",
          name: "行业咨询",
          icon: "💼",
          description: "行业洞察与咨询：行业扫描 → 竞品分析 → 战略建议 → 落地规划",
          version: 1,
          enabled: true,
        },
        "accounting": {
          id: "accounting",
          name: "会计",
          icon: "💰",
          description: "财务会计与税务：账务处理 → 税务申报 → 财务报表 → 审计支持",
          version: 1,
          enabled: true,
        },
        "ecommerce": {
          id: "ecommerce",
          name: "电商",
          icon: "🛒",
          description: "电商运营全流程：选品策略 → 店铺运营 → 营销推广 → 客户管理",
          version: 1,
          enabled: true,
        },
        "education": {
          id: "education",
          name: "教育",
          icon: "🎓",
          description: "教育产品与服务：课程设计 → 内容开发 → 学员运营 → 效果评估",
          version: 1,
          enabled: true,
        },
      };

      const manifest = industryManifests[industryId] || industryManifests["ai-research"];
      const workflows = getStore<any[]>("workflow_templates", []).filter(
        (w: any) => w.tags?.includes("opc") && w.tags?.some((t: string) => t.includes(industryId.replace("-", ""))),
      );

      return {
        manifest,
        workflows: workflows.length > 0 ? workflows : [
          {
            id: `workflow-${industryId}`,
            name: `${manifest.name}分析流程`,
            description: `${manifest.description}`,
            version: "1",
          },
        ],
      } as T;
    }

    case "opc_list_industries": {
      const industries = [
        {
          id: "ai-research",
          name: "人工智能研究",
          icon: "🤖",
          description: "AI 技术调研、模型评测、应用场景分析",
          actionCount: 4,
          workflowCount: 3,
        },
        {
          id: "software-dev",
          name: "软件开发",
          icon: "💻",
          description: "代码审查、架构设计、API 文档、Bug 分析",
          actionCount: 4,
          workflowCount: 3,
        },
        {
          id: "finance-invest",
          name: "金融投资",
          icon: "📈",
          description: "个股分析、财报解读、估值计算、风险评估",
          actionCount: 4,
          workflowCount: 3,
        },
        {
          id: "sales-growth",
          name: "销售增长",
          icon: "🚀",
          description: "客户画像、转化漏斗、销售文案、竞品策略",
          actionCount: 4,
          workflowCount: 3,
        },
        {
          id: "content-media",
          name: "内容媒体",
          icon: "📝",
          description: "文章写作、SEO 优化、视频脚本、内容日历",
          actionCount: 4,
          workflowCount: 3,
        },
        {
          id: "industry-consulting",
          name: "行业咨询",
          icon: "💼",
          description: "行业报告、市场预测、进入策略、竞品分析",
          actionCount: 4,
          workflowCount: 3,
        },
        {
          id: "accounting",
          name: "会计",
          icon: "💰",
          description: "税务筹划、报表解读、成本分析、预算规划",
          actionCount: 4,
          workflowCount: 3,
        },
        {
          id: "ecommerce",
          name: "电商",
          icon: "🛒",
          description: "选品分析、定价策略、营销方案、店铺诊断",
          actionCount: 4,
          workflowCount: 3,
        },
        {
          id: "education",
          name: "教育",
          icon: "🎓",
          description: "课程设计、知识图谱、学习路径、教材生成",
          actionCount: 4,
          workflowCount: 3,
        },
      ];
      return industries as unknown as T;
    }

    case "opc_build_industry_prompt": {
      const industryId = (args as { industry_id?: string })?.industry_id || "ai-research";
      const actionKey = (args as { action_key?: string })?.action_key || "ai-paper";

      const promptConfigs: Record<string, Record<string, { systemPrompt: string; userPrompt: string; label: string }>> =
        {
          "ai-research": {
            "ai-paper": {
              systemPrompt: "你是一位资深的 AI 研究员，擅长追踪前沿技术论文。",
              userPrompt: "请帮我调研最近一个月 AI 领域的重要论文和技术突破。",
              label: "AI 论文调研",
            },
            "ai-benchmark": {
              systemPrompt: "你是一位 AI 模型评测专家。",
              userPrompt: "请对比分析主流大模型在中文场景下的性能表现。",
              label: "模型性能对比",
            },
            "ai-application": {
              systemPrompt: "你是一位 AI 应用咨询顾问。",
              userPrompt: "请分析 AI 技术在特定行业的应用机会。",
              label: "AI 应用场景分析",
            },
            "ai-report": {
              systemPrompt: "你是一位专业的 AI 行业分析师。",
              userPrompt: "请生成一份 AI 技术研究报告。",
              label: "生成 AI 研究报告",
            },
          },
          "software-dev": {
            "sd-code-review": {
              systemPrompt: "你是一位资深代码审查专家。",
              userPrompt: "请帮我审查代码，关注代码质量和最佳实践。",
              label: "代码审查",
            },
            "sd-architecture": {
              systemPrompt: "你是一位资深软件架构师。",
              userPrompt: "请对系统架构进行评估并提供改进建议。",
              label: "架构设计咨询",
            },
            "sd-api-doc": {
              systemPrompt: "你是一位技术文档撰写专家。",
              userPrompt: "请为代码生成完整的 API 文档。",
              label: "API 文档生成",
            },
            "sd-bug": {
              systemPrompt: "你是一位资深调试专家。",
              userPrompt: "请帮我分析错误信息，定位根因并提供修复方案。",
              label: "Bug 分析修复",
            },
          },
        };

      const industryPrompts = promptConfigs[industryId] || promptConfigs["ai-research"];
      const action = industryPrompts[actionKey] || industryPrompts["ai-paper"];

      return {
        systemPrompt: action.systemPrompt,
        userPrompt: action.userPrompt,
        actionKey,
        actionLabel: action.label,
        industryId,
      } as unknown as T;
    }

    case "opc_list_industry_actions":
    case "opc_list_industry_workflows":
    case "opc_get_action_config":
    case "opc_get_workflow_config": {
      return [] as unknown as T;
    }

    // ── Knowledge Base Advanced (知识库高级) ────────────────────────
    case "kb_connect_vault":
      return { success: true, vault_id: (args as { vault_id?: string })?.vault_id ?? "" } as T;
    case "kb_disconnect_vault":
      return { success: true } as T;
    case "kb_list_vaults":
      return [] as T;
    case "kb_semantic_search":
      return [] as T;
    case "kb_hybrid_search":
      return [] as T;
    case "kb_rag_pipeline":
      return { chunks: [], answer: "", sources: [] } as T;
    case "kb_create_rag_pipeline":
      return { pipeline_id: (args as { name?: string })?.name ?? "", created: true } as T;
    case "kb_list_rag_pipelines":
      return [] as T;
    case "kb_delete_rag_pipeline":
      return { success: true } as T;
    case "kb_get_embedding_config":
      return { model: "text-embedding-3-small", dimension: 1536 } as T;
    case "kb_update_embedding_config":
      return { success: true } as T;
    case "kb_reindex":
      return { job_id: "mock-reindex", status: "started" } as T;
    case "kb_get_index_status":
      return { status: "ready", progress: 100, indexed_chunks: 0 } as T;
    case "kb_cancel_index":
      return { success: true } as T;
    case "kb_optimize_index":
      return { success: true, optimized_chunks: 0 } as T;

    // ── Workflow Engine (工作流引擎) ────────────────────────────────
    case "workflow_execute":
      return { execution_id: "mock-exec", status: "completed", outputs: {} } as T;
    case "workflow_cancel":
      return { success: true } as T;
    case "workflow_get_execution_status":
      return {
        execution_id: (args as { execution_id?: string })?.execution_id ?? "",
        status: "completed",
        current_node: null,
        progress: 100,
        node_states: {},
      } as T;
    case "workflow_list_executions":
      return [] as T;
    case "workflow_get_execution_history":
      return [] as T;
    case "workflow_get_node_trace":
      return { node_id: (args as { node_id?: string })?.node_id ?? "", inputs: {}, outputs: {}, duration_ms: 0 } as T;
    case "workflow_resume_execution":
      return { success: true } as T;
    case "workflow_pause_execution":
      return { success: true } as T;
    case "workflow_set_breakpoint":
      return { success: true } as T;
    case "workflow_remove_breakpoint":
      return { success: true } as T;
    case "workflow_list_breakpoints":
      return [] as T;

    // ── RL Training (RL 训练) ───────────────────────────────────────
    case "rl_train_start":
      return { training_id: "mock-rl-train", status: "started" } as T;
    case "rl_train_stop":
      return { success: true } as T;
    case "rl_train_status":
      return {
        training_id: (args as { training_id?: string })?.training_id ?? "",
        status: "idle",
        step: 0,
        total_steps: 0,
        reward: 0,
        loss: 0,
      } as T;
    case "rl_list_training_jobs":
      return [] as T;
    case "rl_get_training_config":
      return {} as T;
    case "rl_update_training_config":
      return { success: true } as T;
    case "rl_save_checkpoint":
      return { checkpoint_id: "mock-checkpoint", saved: true } as T;
    case "rl_load_checkpoint":
      return { checkpoint_id: (args as { checkpoint_id?: string })?.checkpoint_id ?? "", config: {} } as T;
    case "rl_list_checkpoints":
      return [] as T;
    case "rl_delete_checkpoint":
      return { success: true } as T;
    case "rl_compute_advantages":
      return { advantages: [], values: [] } as T;
    case "rl_generate_trajectories":
      return { trajectories: [], total_steps: 0 } as T;
    case "rl_rollout_policy":
      return { actions: [], log_probs: [], values: [] } as T;
    case "rl_get_policy_weights":
      return { weights: {} } as T;
    case "rl_update_policy_weights":
      return { success: true } as T;

    // ── Trajectory System (轨迹系统) ───────────────────────────────
    case "trajectory_record_start":
      return { trajectory_id: "mock-traj", started: true } as T;
    case "trajectory_record_step":
      return { recorded: true } as T;
    case "trajectory_record_end":
      return { trajectory_id: (args as { trajectory_id?: string })?.trajectory_id ?? "", total_steps: 0 } as T;
    case "trajectory_batch_record":
      return { batch_id: "mock-batch", count: 0 } as T;
    case "trajectory_replay":
      return { steps: [], total_score: 0 } as T;
    case "trajectory_sample_batch":
      return { trajectories: [] } as T;
    case "trajectory_export":
      return { format: "json", url: "" } as T;
    case "trajectory_delete":
      return { success: true } as T;
    case "trajectory_get_stats":
      return { total: 0, avg_reward: 0, avg_length: 0 } as T;

    // ── Tools System (工具系统) ─────────────────────────────────────
    case "tool_list":
      return [] as T;
    case "tool_execute":
      return { success: true, result: null, duration_ms: 0 } as T;
    case "tool_validate":
      return { valid: true, errors: [] } as T;
    case "tool_get_dependencies":
      return [] as T;
    case "tool_warmup":
      return { success: true, warmed_up: false } as T;

    default: {
      console.warn(`[BrowserMock] Unhandled command: ${cmd}`, args);
      // SAFE: browser mock fallback for unhandled commands — returns empty placeholder matching generic T
      if (cmd.startsWith("list_") || cmd.endsWith("_list") || cmd.includes("_list_") || cmd.endsWith("s")) {
        return [] as unknown as T;
      }
      if (cmd.startsWith("get_")) {
        return {} as unknown as T;
      }
      return undefined as T;
    }
  }
}

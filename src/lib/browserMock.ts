/**
 * Browser-mode mock backend using localStorage.
 * Activated when the app runs outside Tauri (e.g. `pnpm dev` in browser).
 * Provides CRUD operations for providers, conversations, apps, settings, and gateway.
 */

import type {
  Conversation,
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
  localStorage.setItem(`axagent_${key}`, JSON.stringify(value));
}

function generateBrowserResponse(userContent: string): string {
  const greeting = /^(你好|hi|hello|hey|嗨)/i.test(userContent.trim());
  if (greeting) {
    return "你好！我是 AxAgent 的浏览器预览模式。在此模式下，我无法连接真实的 AI 服务，但你可以体验完整的聊天界面交互。\n\n如需真实 AI 对话，请通过 `cargo tauri dev` 启动 Tauri 后端。";
  }
  return `收到你的消息：「${
    userContent.length > 50 ? userContent.slice(0, 50) + "..." : userContent
  }」\n\n当前为浏览器预览模式，无法调用真实 AI 接口。此模式用于 UI 开发和体验测试。\n\n如需 AI 回复，请使用 \`cargo tauri dev\` 启动完整应用。`;
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
        model_id: "gpt-4o",
        name: "gpt-4o",
        capabilities: ["TextGeneration", "Vision", "FunctionCalling"],
        max_tokens: 128000,
        enabled: true,
        param_overrides: null,
      },
      {
        provider_id: "builtin-openai",
        model_id: "gpt-4o-mini",
        name: "gpt-4o-mini",
        capabilities: ["TextGeneration", "Vision", "FunctionCalling"],
        max_tokens: 128000,
        enabled: true,
        param_overrides: null,
      },
      {
        provider_id: "builtin-openai",
        model_id: "o3-mini",
        name: "o3-mini",
        capabilities: ["TextGeneration", "Reasoning"],
        max_tokens: 200000,
        enabled: false,
        param_overrides: null,
      },
      {
        provider_id: "builtin-openai",
        model_id: "gpt-4.1",
        name: "gpt-4.1",
        capabilities: ["TextGeneration", "Vision", "FunctionCalling"],
        max_tokens: 1047576,
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
        model_id: "gpt-4o",
        name: "gpt-4o",
        capabilities: ["TextGeneration", "Vision", "FunctionCalling"],
        max_tokens: 128000,
        enabled: true,
        param_overrides: null,
      },
      {
        provider_id: "builtin-openai-responses",
        model_id: "gpt-4o-mini",
        name: "gpt-4o-mini",
        capabilities: ["TextGeneration", "Vision", "FunctionCalling"],
        max_tokens: 128000,
        enabled: true,
        param_overrides: null,
      },
      {
        provider_id: "builtin-openai-responses",
        model_id: "o3-mini",
        name: "o3-mini",
        capabilities: ["TextGeneration", "Reasoning"],
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
        model_id: "gemini-2.5-flash",
        name: "gemini-2.5-flash",
        capabilities: ["TextGeneration", "Vision", "Reasoning"],
        max_tokens: 1048576,
        enabled: true,
        param_overrides: null,
      },
      {
        provider_id: "builtin-gemini",
        model_id: "gemini-2.5-pro",
        name: "gemini-2.5-pro",
        capabilities: ["TextGeneration", "Vision", "Reasoning"],
        max_tokens: 1048576,
        enabled: true,
        param_overrides: null,
      },
      {
        provider_id: "builtin-gemini",
        model_id: "gemini-2.0-flash",
        name: "gemini-2.0-flash",
        capabilities: ["TextGeneration", "Vision"],
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
        model_id: "claude-sonnet-4-20250514",
        name: "claude-sonnet-4-20250514",
        capabilities: ["TextGeneration", "Vision", "FunctionCalling"],
        max_tokens: 200000,
        enabled: true,
        param_overrides: null,
      },
      {
        provider_id: "builtin-anthropic",
        model_id: "claude-3-5-haiku-20241022",
        name: "claude-3-5-haiku-20241022",
        capabilities: ["TextGeneration"],
        max_tokens: 200000,
        enabled: true,
        param_overrides: null,
      },
      {
        provider_id: "builtin-anthropic",
        model_id: "claude-opus-4-20250514",
        name: "claude-opus-4-20250514",
        capabilities: ["TextGeneration", "Vision", "FunctionCalling"],
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
        model_id: "deepseek-chat",
        name: "deepseek-chat",
        capabilities: ["TextGeneration"],
        max_tokens: 128000,
        enabled: true,
        param_overrides: null,
      },
      {
        provider_id: "builtin-deepseek",
        model_id: "deepseek-reasoner",
        name: "deepseek-reasoner",
        capabilities: ["TextGeneration", "Reasoning"],
        max_tokens: 128000,
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
    id: "builtin-xai",
    name: "xAI",
    provider_type: "openai",
    api_host: "https://api.x.ai",
    api_path: null,
    enabled: true,
    models: [
      {
        provider_id: "builtin-xai",
        model_id: "grok-3",
        name: "grok-3",
        capabilities: ["TextGeneration", "Vision", "FunctionCalling"],
        max_tokens: 131072,
        enabled: true,
        param_overrides: null,
      },
      {
        provider_id: "builtin-xai",
        model_id: "grok-3-mini",
        name: "grok-3-mini",
        capabilities: ["TextGeneration", "Vision", "Reasoning"],
        max_tokens: 131072,
        enabled: true,
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
    id: "builtin-glm",
    name: "GLM",
    provider_type: "openai",
    api_host: "https://open.bigmodel.cn/api/paas",
    api_path: null,
    enabled: true,
    models: [
      {
        provider_id: "builtin-glm",
        model_id: "glm-4-plus",
        name: "glm-4-plus",
        capabilities: ["TextGeneration", "Vision", "FunctionCalling"],
        max_tokens: 128000,
        enabled: true,
        param_overrides: null,
      },
      {
        provider_id: "builtin-glm",
        model_id: "glm-4-flash",
        name: "glm-4-flash",
        capabilities: ["TextGeneration", "Vision"],
        max_tokens: 128000,
        enabled: true,
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
        model_id: "MiniMax-M1",
        name: "MiniMax-M1",
        capabilities: ["TextGeneration", "Reasoning"],
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
        enabled: true,
        param_overrides: null,
      },
    ],
    keys: [],
    proxy_config: null,
    sort_order: 7,
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
  for (const builtin of BUILT_IN_PROVIDERS) {
    const stored = existing.find((p) => p.id === builtin.id) as
      | (Provider & { models?: Array<{ model_id: string; name: string }> })
      | undefined;
    if (stored && (!stored.models || stored.models.length === 0)) {
      stored.models = [...builtin.models] as typeof stored.models;
      dirty = true;
    }
  }
  if (dirty) { setStore("providers", existing); }
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
};

// ── Command Handler ─────────────────────────────────────────────────────

export async function handleCommand<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  await new Promise((r) => setTimeout(r, 5));

  switch (cmd) {
    // ── Settings ──────────────────────────────────────────────────────
    case "get_settings":
      return getStore("settings", DEFAULT_SETTINGS) as T;
    case "save_settings": {
      const settings = (args as { settings?: Partial<Settings> }).settings ?? {};
      const current = getStore<Settings>("settings", DEFAULT_SETTINGS as Settings);
      const merged = { ...current, ...settings };
      setStore("settings", merged);
      return merged as T;
    }

    // ── Providers ─────────────────────────────────────────────────────
    case "list_providers":
      return initProviders() as T;
    case "create_provider": {
      const input = (args as { input?: Partial<Provider> }).input ?? {} as Partial<Provider>;
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
      if (idx === -1) { throw new Error("Provider not found"); }
      if (input?.name !== undefined) { providers[idx].name = input.name; }
      if (input?.provider_type !== undefined) { providers[idx].provider_type = input.provider_type; }
      if (input?.api_host !== undefined) { providers[idx].api_host = input.api_host; }
      if (input?.enabled !== undefined) { providers[idx].enabled = input.enabled; }
      if (input?.api_path !== undefined) { providers[idx].api_path = input.api_path; }
      if (input?.sort_order !== undefined) { providers[idx].sort_order = input.sort_order; }
      providers[idx].updated_at = nowTs();
      setStore("providers", providers);
      return providers[idx] as T;
    }
    case "delete_provider": {
      const { id } = args as { id?: string };
      const providers = getStore<Provider[]>("providers", []).filter((p) => p.id !== id);
      setStore("providers", providers);
      return undefined as T;
    }
    case "reorder_providers": {
      const { providerIds } = args as { providerIds?: string[] };
      const providers = getStore<Provider[]>("providers", []);
      if (providerIds) {
        for (let i = 0; i < providerIds.length; i++) {
          const p = providers.find((p) => p.id === providerIds[i]);
          if (p) { p.sort_order = i; }
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
      const { providerId, rawKey } = args as { providerId?: string; rawKey?: string };
      const key: ProviderKey = {
        id: genId(),
        provider_id: providerId ?? "",
        key_encrypted: rawKey ?? "",
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
        const key = p.keys.find((k) => k.id === keyId);
        if (key) { key.enabled = enabled ?? true; }
      }
      setStore("providers", providers);
      return undefined as T;
    }
    case "validate_provider_key":
      return true as T;
    case "save_models": {
      const { providerId, models } = args as {
        providerId?: string;
        models?: Array<{ model_id: string; name: string; mode?: string; enabled?: boolean }>;
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
      const { providerId, modelId, enabled } = args as { providerId?: string; modelId?: string; enabled?: boolean };
      const providers = getStore<Provider[]>("providers", []);
      const pIdx = providers.findIndex((p) => p.id === providerId);
      if (pIdx !== -1) {
        const model = providers[pIdx].models.find((m) => m.model_id === modelId);
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
        const model = providers[pIdx].models.find((m) => m.model_id === modelId);
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
      const target = providers.find((p) => p.id === (args as { providerId?: string }).providerId);
      return (target?.models ?? []) as T;
    }

    // ── Conversations ─────────────────────────────────────────────────
    case "list_conversations":
      return getStore<Conversation[]>("conversations", []).filter((c) => !c.is_archived) as T;
    case "list_archived_conversations":
      return getStore<Conversation[]>("conversations", []).filter((c) => c.is_archived) as T;
    case "create_conversation": {
      const { title, modelId, providerId, systemPrompt } = args as Record<string, unknown>;
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
      const { id, input } = args as { id?: string; input?: Partial<Conversation> };
      const convs = getStore<Conversation[]>("conversations", []);
      const idx = convs.findIndex((c) => c.id === id);
      if (idx !== -1 && input) {
        if (input.title !== undefined) { convs[idx].title = input.title; }
        if (input.category_id !== undefined) { convs[idx].category_id = input.category_id; }
        if (input.provider_id !== undefined) { convs[idx].provider_id = input.provider_id; }
        if (input.model_id !== undefined) { convs[idx].model_id = input.model_id; }
        if (input.temperature !== undefined) { convs[idx].temperature = input.temperature; }
        if (input.max_tokens !== undefined) { convs[idx].max_tokens = input.max_tokens; }
        if (input.top_p !== undefined) { convs[idx].top_p = input.top_p; }
        if (input.frequency_penalty !== undefined) { convs[idx].frequency_penalty = input.frequency_penalty; }
        convs[idx].updated_at = nowTs();
        setStore("conversations", convs);
        return convs[idx] as T;
      }
      throw new Error("Conversation not found");
    }
    case "delete_conversation": {
      const { id } = args as { id?: string };
      const convs = getStore<Conversation[]>("conversations", []).filter((c) => c.id !== id);
      setStore("conversations", convs);
      const msgs = getStore<Message[]>("messages", []).filter((m) => m.conversation_id !== id);
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
      return getStore<ConversationCategory[]>("conversation_categories", []) as T;
    case "create_conversation_category": {
      const { input } = args as { input: ConversationCategory };
      const cats = getStore<ConversationCategory[]>("conversation_categories", []);
      const maxOrder = cats.reduce((m: number, c) => Math.max(m, c.sort_order ?? 0), -1);
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
      const { id, input } = args as { id: string; input: Partial<ConversationCategory> };
      const cats = getStore<ConversationCategory[]>("conversation_categories", []);
      const idx = cats.findIndex((c) => c.id === id);
      if (idx !== -1) {
        if (input.name !== undefined) { cats[idx].name = input.name; }
        if (input.icon_type !== undefined) { cats[idx].icon_type = input.icon_type; }
        if (input.icon_value !== undefined) { cats[idx].icon_value = input.icon_value; }
        if (input.system_prompt !== undefined) { cats[idx].system_prompt = input.system_prompt; }
        if (input.default_provider_id !== undefined) { cats[idx].default_provider_id = input.default_provider_id; }
        if (input.default_model_id !== undefined) { cats[idx].default_model_id = input.default_model_id; }
        if (input.default_temperature !== undefined) { cats[idx].default_temperature = input.default_temperature; }
        if (input.default_max_tokens !== undefined) { cats[idx].default_max_tokens = input.default_max_tokens; }
        if (input.default_top_p !== undefined) { cats[idx].default_top_p = input.default_top_p; }
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
      const cats = getStore<ConversationCategory[]>("conversation_categories", []).filter((c) => c.id !== id);
      setStore("conversation_categories", cats);
      const convs = getStore<Conversation[]>("conversations", []);
      convs.forEach((c) => {
        if (c.category_id === id) { c.category_id = null; }
      });
      setStore("conversations", convs);
      return undefined as T;
    }
    case "reorder_conversation_categories": {
      const { categoryIds } = args as { categoryIds: string[] };
      const cats = getStore<ConversationCategory[]>("conversation_categories", []);
      for (let i = 0; i < categoryIds.length; i++) {
        const c = cats.find((c) => c.id === categoryIds[i]);
        if (c) { c.sort_order = i; }
      }
      cats.sort((a, b) => (a.sort_order ?? 0) - (b.sort_order ?? 0));
      setStore("conversation_categories", cats);
      return undefined as T;
    }
    case "set_conversation_category_collapsed": {
      const { id, collapsed } = args as { id?: string; collapsed?: boolean };
      const cats = getStore<ConversationCategory[]>("conversation_categories", []);
      const idx = cats.findIndex((c) => c.id === id);
      if (idx !== -1) {
        cats[idx].is_collapsed = collapsed ?? false;
        cats[idx].updated_at = nowTs();
        setStore("conversation_categories", cats);
      }
      return undefined as T;
    }
    case "agent_get_session": {
      // Browser mock: return null session (not yet initialized)
      return null as T;
    }
    case "agent_update_session": {
      // Browser mock: return no cwd (will trigger workspace creation)
      return { cwd: null } as T;
    }
    case "agent_ensure_workspace": {
      const workspacePath = "/mock/workspace/" + Date.now();
      return { workspacePath } as T;
    }
    case "list_agency_experts": {
      return [] as T;
    }
    case "agent_query": {
      // Browser mock: return immediately without error
      return undefined as T;
    }
    case "plan_list": {
      return [] as T;
    }
    case "send_message": {
      const { conversationId, content, attachments } = args as {
        conversationId: string;
        content: string;
        attachments?: unknown[];
      };
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
      const { conversationId, limit = 10, beforeMessageId = null } = args as {
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
      const results = convs
        .filter((c) => c.title.toLowerCase().includes(query.toLowerCase()))
        .map((c) => ({ conversation_id: c.id, title: c.title, snippet: "" }));
      return results as T;
    }
    case "regenerate_message": {
      const { conversationId: regenConvId } = args as { conversationId?: string };
      const regenMsgs = getStore<Message[]>("messages", []);
      const convMsgs = regenMsgs.filter((m) => m.conversation_id === regenConvId);
      let lastUserMsg: Message | null = null;
      for (let i = convMsgs.length - 1; i >= 0; i--) {
        if (convMsgs[i].role === "user") {
          lastUserMsg = convMsgs[i];
          break;
        }
      }
      if (lastUserMsg) {
        const existingVersions = regenMsgs.filter(
          (m) => m.parent_message_id === lastUserMsg!.id && m.role === "assistant",
        );
        const nextVersion = existingVersions.length;
        for (const m of regenMsgs) {
          if (m.parent_message_id === lastUserMsg!.id && m.role === "assistant") {
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
      return allMsgs.filter((m) => m.parent_message_id === parentMessageId) as T;
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
      return { gateway_key: key, plain_key: `sk-mock-plain-key-${genId().substring(0, 8)}` } as T;
    }
    case "delete_gateway_key": {
      const { id } = args as { id?: string };
      const keys = getStore<GatewayKey[]>("gateway_keys", []).filter((k) => k.id !== id);
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
      const spInput = (args as { input?: CreateSearchProviderInput }).input ?? ({} as CreateSearchProviderInput);
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
      const spi = sps2.findIndex(s => s.id === spUpdateId);
      if (spi >= 0) {
        if (spInput.name !== undefined) { sps2[spi].name = spInput.name; }
        if (spInput.endpoint !== undefined) { sps2[spi].endpoint = spInput.endpoint; }
        if (spInput.enabled !== undefined) { sps2[spi].enabled = spInput.enabled; }
        if (spInput.region !== undefined) { sps2[spi].region = spInput.region; }
        if (spInput.language !== undefined) { sps2[spi].language = spInput.language; }
        if (spInput.safeSearch !== undefined) { sps2[spi].safeSearch = spInput.safeSearch; }
        if (spInput.resultLimit !== undefined) { sps2[spi].resultLimit = spInput.resultLimit; }
        if (spInput.timeoutMs !== undefined) { sps2[spi].timeoutMs = spInput.timeoutMs; }
        if (spInput.apiKey !== undefined) { sps2[spi].hasApiKey = !!spInput.apiKey; }
        setStore("search_providers", sps2);
        return sps2[spi] as T;
      }
      return undefined as T;
    }
    case "delete_search_provider": {
      const sps3 = getStore<SearchProvider[]>("search_providers", []);
      setStore("search_providers", sps3.filter(s => s.id !== (args as { id?: string })?.id));
      return undefined as T;
    }
    case "test_search_provider":
      return { ok: true, latency_ms: 120 } as T;

    // ── Phase 2: MCP Servers ──────────────────────────────────────────
    case "list_local_tools":
      return [] as T;
    case "toggle_local_tool":
      return { groupId: (args as Record<string, unknown>)?.groupId, groupName: "", enabled: true, tools: [] } as T;
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
      const mi = mcps2.findIndex(m => m.id === (args as Record<string, unknown>)?.id);
      if (mi >= 0) {
        Object.assign(mcps2[mi], args, { updated_at: nowTs() });
        setStore("mcp_servers", mcps2);
        return mcps2[mi] as T;
      }
      return undefined as T;
    }
    case "delete_mcp_server": {
      const mcps3 = getStore<Record<string, unknown>[]>("mcp_servers", []);
      setStore("mcp_servers", mcps3.filter(m => m.id !== (args as Record<string, unknown>)?.id));
      return undefined as T;
    }
    case "connect_mcp_server":
      return { status: "connected" } as T;
    case "disconnect_mcp_server":
      return { status: "disconnected" } as T;
    case "list_mcp_tools":
      return [
        { name: "web_search", description: "Search the web", parameters: {} },
        { name: "calculator", description: "Evaluate math expressions", parameters: {} },
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
      const input = (args as { input?: CreateKnowledgeBaseInput }).input ?? ({} as CreateKnowledgeBaseInput);
      const kbs = getStore<KnowledgeBase[]>("knowledge_bases", []);
      const kb: KnowledgeBase & { documents: KnowledgeDocument[]; created_at: number; updated_at: number } = {
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
      const { id, input } = args as { id: string; input?: Partial<KnowledgeBase> };
      const ki = kbs2.findIndex(k => k.id === id);
      if (ki >= 0) {
        if (input?.name !== undefined) { kbs2[ki].name = input.name; }
        if (input?.description !== undefined) { kbs2[ki].description = input.description; }
        if (input?.enabled !== undefined) { kbs2[ki].enabled = input.enabled; }
        setStore("knowledge_bases", kbs2);
        return kbs2[ki] as T;
      }
      return undefined as T;
    }
    case "delete_knowledge_base": {
      const kbs3 = getStore<KnowledgeBase[]>("knowledge_bases", []);
      setStore("knowledge_bases", kbs3.filter(k => k.id !== (args as { id?: string })?.id));
      return undefined as T;
    }
    case "add_knowledge_document": {
      const kbs4 = getStore<(KnowledgeBase & { documents: KnowledgeDocument[]; updated_at: number })[]>(
        "knowledge_bases",
        [],
      );
      const { baseId, ...docInput } = args as { baseId?: string; title?: string; sourcePath?: string };
      const kbi = kbs4.findIndex(k => k.id === baseId);
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
      const kbs5 = getStore<(KnowledgeBase & { documents: KnowledgeDocument[]; updated_at: number })[]>(
        "knowledge_bases",
        [],
      );
      const target = kbs5.find(k => k.id === (args as { baseId?: string })?.baseId);
      return (target?.documents ?? []) as T;
    }
    case "delete_knowledge_document": {
      const kbs6 = getStore<(KnowledgeBase & { documents: KnowledgeDocument[]; updated_at: number })[]>(
        "knowledge_bases",
        [],
      );
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

    // ── Phase 2: Memory ───────────────────────────────────────────────
    case "list_memory_namespaces":
      return getStore<MemoryNamespace[]>("memory_namespaces", []) as T;
    case "create_memory_namespace": {
      const input = (args as { input?: CreateMemoryNamespaceInput }).input ?? ({} as CreateMemoryNamespaceInput);
      const mns = getStore<MemoryNamespace[]>("memory_namespaces", []);
      const mn: MemoryNamespace & { items: MemoryItem[]; created_at: number; updated_at: number } = {
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
      setStore("memory_namespaces", mns2.filter(n => n.id !== (args as { id?: string })?.id));
      return undefined as T;
    }
    case "add_memory_item": {
      const mns3 = getStore<(MemoryNamespace & { items: MemoryItem[]; updated_at: number })[]>("memory_namespaces", []);
      const inputMem = (args as { input?: CreateMemoryItemInput }).input ?? ({} as CreateMemoryItemInput);
      const mni = mns3.findIndex(n => n.id === inputMem?.namespaceId);
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
          nature: "semantic",
          tags: [],
          accessCount: 0,
          updatedAt: new Date().toISOString(),
        };
        mns3[mni].items = [...(mns3[mni].items || []), item];
        mns3[mni].updated_at = nowTs();
        setStore("memory_namespaces", mns3);
        return item as T;
      }
      return undefined as T;
    }
    case "list_memory_items": {
      const mns4 = getStore<(MemoryNamespace & { items: MemoryItem[] })[]>("memory_namespaces", []);
      const ns = mns4.find(n => n.id === (args as { namespaceId?: string })?.namespaceId);
      return (ns?.items ?? []) as T;
    }
    case "delete_memory_item": {
      const mns5 = getStore<(MemoryNamespace & { items: MemoryItem[]; updated_at: number })[]>("memory_namespaces", []);
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

    // ── Phase 2: Artifacts ────────────────────────────────────────────
    case "list_artifacts": {
      const allArtifacts = getStore<Artifact[]>("artifacts", []);
      const convId = (args as { conversationId?: string })?.conversationId;
      return (convId ? allArtifacts.filter((a) => a.conversationId === convId) : allArtifacts) as T;
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
      const artInput = (args as { id?: string; input?: Partial<Artifact> }).input;
      const ai = arts2.findIndex(a => a.id === (args as { id?: string }).id);
      if (ai >= 0 && artInput) {
        if (artInput.title !== undefined) { arts2[ai].title = artInput.title; }
        if (artInput.content !== undefined) { arts2[ai].content = artInput.content; }
        if (artInput.format !== undefined) { arts2[ai].format = artInput.format; }
        if (artInput.language !== undefined) { arts2[ai].language = artInput.language; }
        if (artInput.previewMode !== undefined) { arts2[ai].previewMode = artInput.previewMode; }
        if (artInput.pinned !== undefined) { arts2[ai].pinned = artInput.pinned; }
        arts2[ai].updatedAt = new Date().toISOString();
        setStore("artifacts", arts2);
        return arts2[ai] as T;
      }
      return undefined as T;
    }
    case "delete_artifact": {
      const arts3 = getStore<Artifact[]>("artifacts", []);
      setStore("artifacts", arts3.filter(a => a.id !== (args as { id?: string })?.id));
      return undefined as T;
    }

    // ── Phase 2: Conversation Branching ───────────────────────────────
    case "fork_conversation": {
      const convs = getStore<Record<string, unknown>[]>("conversations", []);
      const source = convs.find(c => c.id === (args as Record<string, unknown>)?.conversationId);
      if (source) {
        const forked = {
          ...JSON.parse(JSON.stringify(source)),
          id: genId(),
          parent_id: source.id,
          title: (args as Record<string, unknown>)?.title ?? `Fork of ${source.title}`,
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
      return convs2.filter(c => c.parent_id === parentId || c.id === parentId) as T;
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
      setStore("context_sources", css2.filter(c => c.id !== (args as Record<string, unknown>)?.id));
      return undefined as T;
    }
    case "toggle_context_source": {
      const css3 = getStore<Record<string, unknown>[]>("context_sources", []);
      const csi = css3.findIndex(c => c.id === (args as Record<string, unknown>)?.id);
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
      setStore("backups", backups.filter((b) => b.id !== bkpId));
      return undefined as T;
    }
    case "batch_delete_backups": {
      const allBkps = getStore<BackupManifest[]>("backups", []);
      const idsToDelete = (args as { backupIds?: string[] })?.backupIds || [];
      setStore("backups", allBkps.filter((b) => !idsToDelete.includes(b.id)));
      return undefined as T;
    }
    case "restore_backup":
      return undefined as T;
    case "get_backup_settings":
      return { enabled: false, intervalHours: 24, maxCount: 10, backupDir: "/mock/backups" } as T;
    case "update_backup_settings":
      return undefined as T;

    // ── Files Page ─────────────────────────────────────────────────────
    case "list_files_page_entries": {
      const category = (args as { category?: string })?.category;
      if (category === "backups") {
        const backups = getStore<BackupManifest[]>("backups", []);
        return backups.map((backup) => ({
          id: `backup_manifest::${backup.id}`,
          name: backup.filePath?.split("/").pop() || `backup-${backup.createdAt}.${backup.version}`,
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
        setStore("backups", backups.filter((b) => b.id !== backupId));
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
      const sppInput = (args as { input?: SaveProgramPolicyInput }).input ?? ({} as SaveProgramPolicyInput);
      const sppIdx = sppList.findIndex(p => p.programName === sppInput.programName);
      if (sppIdx >= 0) {
        sppList[sppIdx] = {
          ...sppList[sppIdx],
          allowedProviderIdsJson: JSON.stringify(sppInput.allowedProviderIds ?? []),
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
        allowedProviderIdsJson: JSON.stringify(sppInput.allowedProviderIds ?? []),
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
      const ppInput = args as { programName?: string; allowedProviderIds?: string[]; allowedModelIds?: string[] };
      const pp: ProgramPolicy = {
        id: genId(),
        programName: ppInput.programName ?? "",
        allowedProviderIdsJson: JSON.stringify(ppInput.allowedProviderIds ?? []),
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
      const ppi = pps2.findIndex(p => p.id === id);
      if (ppi >= 0) {
        if (ppInput.programName !== undefined) { pps2[ppi].programName = ppInput.programName; }
        if (ppInput.allowedProviderIds !== undefined) {
          pps2[ppi].allowedProviderIdsJson = JSON.stringify(ppInput.allowedProviderIds);
        }
        if (ppInput.allowedModelIds !== undefined) {
          pps2[ppi].allowedModelIdsJson = JSON.stringify(ppInput.allowedModelIds);
        }
        if (ppInput.defaultProviderId !== undefined) { pps2[ppi].defaultProviderId = ppInput.defaultProviderId; }
        if (ppInput.defaultModelId !== undefined) { pps2[ppi].defaultModelId = ppInput.defaultModelId; }
        if (ppInput.rateLimitPerMinute !== undefined) { pps2[ppi].rateLimitPerMinute = ppInput.rateLimitPerMinute; }
        setStore("program_policies", pps2);
        return pps2[ppi] as T;
      }
      return undefined as T;
    }
    case "delete_program_policy": {
      const pps3 = getStore<ProgramPolicy[]>("program_policies", []);
      setStore("program_policies", pps3.filter(p => p.id !== (args as { id?: string })?.id));
      return undefined as T;
    }

    // ── Phase 2: Gateway Diagnostics & Templates ──────────────────────
    case "get_gateway_diagnostics":
      return [
        { id: "1", category: "port", status: "ok", message: "Gateway port is available", createdAt: nowTs() },
        { id: "2", category: "auth", status: "ok", message: "Authentication configured", createdAt: nowTs() },
        { id: "3", category: "proxy", status: "ok", message: "Proxy settings valid", createdAt: nowTs() },
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
          copyHint: "添加到 Cursor User settings.json",
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
          copyHint: "添加到 .continue/config.json 的 models 数组",
          created_at: nowTs(),
          updated_at: nowTs(),
        },
        {
          id: "tpl-claude",
          name: "Claude Code CLI",
          target: "claude_code",
          format: "text",
          content: "ANTHROPIC_BASE_URL=http://localhost:{{port}}/v1\nANTHROPIC_AUTH_TOKEN={{key}}",
          copyHint: "添加到环境变量或 .env 文件",
          created_at: nowTs(),
          updated_at: nowTs(),
        },
        {
          id: "tpl-openai",
          name: "OpenAI Compatible",
          target: "openai_compatible",
          format: "text",
          content: "API Base: http://localhost:{{port}}/v1\nAPI Key: {{key}}",
          copyHint: "适用于任何支持 OpenAI API 的客户端",
          created_at: nowTs(),
          updated_at: nowTs(),
        },
      ]) as T;
    case "create_gateway_template": {
      const gts = getStore<Record<string, unknown>[]>("gateway_templates", []);
      const gt = { id: genId(), ...(args as Record<string, unknown>), created_at: nowTs(), updated_at: nowTs() };
      gts.push(gt);
      setStore("gateway_templates", gts);
      return gt as T;
    }
    case "delete_gateway_template": {
      const gts2 = getStore<Record<string, unknown>[]>("gateway_templates", []);
      setStore("gateway_templates", gts2.filter(g => g.id !== (args as Record<string, unknown>)?.id));
      return undefined as T;
    }
    case "copy_gateway_template": {
      const cgtList = getStore<Record<string, unknown>[]>("gateway_templates", []);
      const cgtMatch = cgtList.find(t => t.id === (args as Record<string, unknown>)?.templateId);
      return (cgtMatch?.content ?? "# Gateway Template Configuration\n\nNo template found.") as T;
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
      if (typeof Notification !== "undefined" && Notification.permission === "granted") {
        new Notification((args as { title?: string })?.title ?? "AxAgent", {
          body: (args as { body?: string })?.body ?? "",
        });
      }
      return undefined as T;
    }
    case "set_always_on_top":
      console.log("[Mock] set_always_on_top:", (args as Record<string, unknown>)?.enabled);
      return undefined as T;
    case "set_close_to_tray":
      console.log("[Mock] set_close_to_tray:", (args as Record<string, unknown>)?.enabled);
      return undefined as T;
    case "apply_startup_settings":
      console.log("[Mock] apply_startup_settings:", args);
      return undefined as T;
    case "set_tray_actions":
      return undefined as T;
    case "handle_protocol_launch":
      return undefined as T;

    // ── Phase 2: Workspace Snapshot ────────────────────────────────────
    case "get_workspace_snapshot":
      return { conversations: [], providers: [], settings: {}, captured_at: nowTs() } as T;
    case "update_workspace_snapshot":
      return undefined as T;

    // ── Proxy Test ────────────────────────────────────────────────────────
    case "test_proxy": {
      const addr = (args as Record<string, unknown>)?.proxyAddress;
      if (!addr) { return { ok: false, error: "No address" } as T; }
      await new Promise(r => setTimeout(r, 500));
      return { ok: true, latency_ms: 120 + Math.floor(Math.random() * 200) } as T;
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
      return ((args as Record<string, unknown>)?.source || "installed-skill") as T;

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
      const existingTemplates = getStore<Record<string, unknown>[]>("workflow_templates", []);
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
        nodes: input.nodes || [],
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
        if (updateInput.name !== undefined) { templates[idx].name = updateInput.name; }
        if (updateInput.description !== undefined) { templates[idx].description = updateInput.description; }
        if (updateInput.tags !== undefined) { templates[idx].tags = updateInput.tags; }
        if (updateInput.nodes !== undefined) { templates[idx].nodes = updateInput.nodes; }
        if (updateInput.edges !== undefined) { templates[idx].edges = updateInput.edges; }
        templates[idx].updated_at = nowTs();
        setStore("workflow_templates", templates);
      }
      return true as T;
    }
    case "delete_workflow_template": {
      const deleteId = (args as { id?: string }).id;
      const templates = getStore<WorkflowTemplate[]>("workflow_templates", []);
      setStore("workflow_templates", templates.filter((t) => t.id !== deleteId));
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
      const existing = getStore<PlatformConfig | null>("platform_config", null) ?? {} as PlatformConfig;
      const merged = { ...existing, ...input };
      setStore("platform_config", merged);
      return undefined as T;
    }
    case "get_platform_statuses": {
      const config = getStore<PlatformConfig | null>("platform_config", null);
      if (!config) { return [] as T; }
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
      return (getStore<PlatformSession[]>("platform_sessions", [])) as T;
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

    default:
      console.warn(`[BrowserMock] Unhandled command: ${cmd}`, args);
      return undefined as T;
  }
}

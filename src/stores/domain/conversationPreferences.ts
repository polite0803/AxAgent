import type { Conversation, UpdateConversationInput } from "@/types";

// Sequence counter to prevent stale preference saves
export const _conversationPreferenceSaveSeq = new Map<string, number>();

// ── Staged preferences (localStorage) for when no conversation is active ──
const STAGED_PREFS_KEY = "axagent:staged-prefs";

function loadStagedPrefs(): Record<string, unknown> {
  try {
    const raw = localStorage.getItem(STAGED_PREFS_KEY);
    return raw ? JSON.parse(raw) : {};
  } catch {
    return {};
  }
}
function saveStagedPrefs(prefs: Record<string, unknown>) {
  try {
    localStorage.setItem(STAGED_PREFS_KEY, JSON.stringify(prefs));
  } catch { /* ignore */ }
}
export function clearStagedPrefs() {
  try {
    localStorage.removeItem(STAGED_PREFS_KEY);
  } catch { /* ignore */ }
}

/** Save the current preference value to staged storage (no-conversation fallback). */
export function stagePreference(key: string, value: unknown) {
  const prefs = loadStagedPrefs();
  prefs[key] = value;
  saveStagedPrefs(prefs);
}

/** Apply staged preferences to a new conversation's update input. */
export function getStagedPreferenceUpdate(): Partial<UpdateConversationInput> {
  const staged = loadStagedPrefs();
  const update: Record<string, unknown> = {};
  if (staged.searchEnabled !== undefined) { update.search_enabled = staged.searchEnabled; }
  if (staged.searchProviderId !== undefined) { update.search_provider_id = staged.searchProviderId; }
  if (staged.enabledMcpServerIds) { update.enabled_mcp_server_ids = staged.enabledMcpServerIds; }
  if (staged.enabledKnowledgeBaseIds) { update.enabled_knowledge_base_ids = staged.enabledKnowledgeBaseIds; }
  if (staged.activeMemoryNamespaceId) {
    update.enabled_memory_namespace_ids = [
      staged.activeMemoryNamespaceId as string,
    ];
  }
  if (staged.enabledWikiIds) { update.enabled_wiki_ids = staged.enabledWikiIds; }
  if (staged.thinkingBudget !== undefined) { update.thinking_budget = staged.thinkingBudget; }
  return update as Partial<UpdateConversationInput>;
}

export type ConversationPreferenceState = {
  searchEnabled: boolean;
  searchProviderId: string | null;
  thinkingBudget: number | null;
  mcpMode: "auto" | "manual" | "disabled";
  enabledMcpServerIds: string[];
  enabledKnowledgeBaseIds: string[];
  activeMemoryNamespaceId: string | null;
  enabledWikiIds: string[];
};

export function conversationPreferenceStateFromConversation(
  conversation?: Conversation | null,
): ConversationPreferenceState {
  return {
    searchEnabled: conversation?.search_enabled ?? false,
    searchProviderId: conversation?.search_provider_id ?? null,
    thinkingBudget: conversation?.thinking_budget ?? null,
    mcpMode: ((conversation as Record<string, unknown> | null | undefined)?.mcp_mode as "auto" | "disabled" | "manual")
      ?? "auto",
    enabledMcpServerIds: [...(conversation?.enabled_mcp_server_ids ?? [])],
    enabledKnowledgeBaseIds: [...(conversation?.enabled_knowledge_base_ids ?? [])],
    activeMemoryNamespaceId: (conversation?.enabled_memory_namespace_ids ?? [])[0] ?? null,
    enabledWikiIds: [...(conversation?.enabled_wiki_ids ?? [])],
  };
}

export function conversationPreferenceUpdateFromState(
  state: Pick<
    ConversationPreferenceState,
    | "searchEnabled"
    | "searchProviderId"
    | "thinkingBudget"
    | "enabledMcpServerIds"
    | "enabledKnowledgeBaseIds"
    | "activeMemoryNamespaceId"
    | "enabledWikiIds"
  >,
): Pick<
  UpdateConversationInput,
  | "search_enabled"
  | "search_provider_id"
  | "thinking_budget"
  | "enabled_mcp_server_ids"
  | "enabled_knowledge_base_ids"
  | "enabled_memory_namespace_ids"
  | "enabled_wiki_ids"
> {
  return {
    search_enabled: state.searchEnabled,
    search_provider_id: state.searchProviderId,
    thinking_budget: state.thinkingBudget,
    enabled_mcp_server_ids: [...state.enabledMcpServerIds],
    enabled_knowledge_base_ids: [...state.enabledKnowledgeBaseIds],
    enabled_memory_namespace_ids: state.activeMemoryNamespaceId ? [state.activeMemoryNamespaceId] : [],
    enabled_wiki_ids: [...state.enabledWikiIds],
  };
}

export function nextConversationPreferenceSaveSeq(conversationId: string): number {
  const next = (_conversationPreferenceSaveSeq.get(conversationId) ?? 0) + 1;
  _conversationPreferenceSaveSeq.set(conversationId, next);
  return next;
}

export function isLatestConversationPreferenceSave(conversationId: string, seq: number): boolean {
  return (_conversationPreferenceSaveSeq.get(conversationId) ?? 0) === seq;
}

export function preferenceStateMatches(
  state: ConversationPreferenceState,
  expected: Partial<ConversationPreferenceState>,
): boolean {
  return Object.entries(expected).every(([key, value]) => {
    const currentValue = state[key as keyof ConversationPreferenceState];
    if (Array.isArray(currentValue) && Array.isArray(value)) {
      return JSON.stringify(currentValue) === JSON.stringify(value);
    }
    return currentValue === value;
  });
}

export function mergeConversationCollections(
  conversations: Conversation[],
  archivedConversations: Conversation[],
  updated: Conversation,
) {
  return {
    conversations: conversations.map((conversation) => (
      conversation.id === updated.id ? updated : conversation
    )),
    archivedConversations: archivedConversations.map((conversation) => (
      conversation.id === updated.id ? updated : conversation
    )),
  };
}

export function categoryTemplateUpdateFromCategory(
  category?: {
    id: string;
    system_prompt?: string | null;
    default_temperature?: number | null;
    default_max_tokens?: number | null;
    default_top_p?: number | null;
    default_frequency_penalty?: number | null;
  } | null,
): Pick<
  UpdateConversationInput,
  | "category_id"
  | "system_prompt"
  | "temperature"
  | "max_tokens"
  | "top_p"
  | "frequency_penalty"
> {
  if (!category) {
    return {};
  }

  return {
    category_id: category.id,
    system_prompt: category.system_prompt ?? undefined,
    temperature: category.default_temperature,
    max_tokens: category.default_max_tokens,
    top_p: category.default_top_p,
    frequency_penalty: category.default_frequency_penalty,
  };
}

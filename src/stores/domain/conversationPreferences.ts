// SPDX-License-Identifier: AGPL-3.0-only

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
  } catch {
    /* ignore */
  }
}
export function clearStagedPrefs() {
  try {
    localStorage.removeItem(STAGED_PREFS_KEY);
  } catch {
    /* ignore */
  }
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
  if (staged.searchEnabled !== undefined) {
    update.searchEnabled = staged.searchEnabled;
  }
  if (staged.searchProviderId !== undefined) {
    update.searchProviderId = staged.searchProviderId;
  }
  if (staged.enabledMcpServerIds) {
    update.enabledMcpServerIds = staged.enabledMcpServerIds;
  }
  if (staged.enabledKnowledgeBaseIds) {
    update.enabledKnowledgeBaseIds = staged.enabledKnowledgeBaseIds;
  }
  if (staged.activeMemoryNamespaceId) {
    update.enabledMemoryNamespaceIds = [
      staged.activeMemoryNamespaceId as string,
    ];
  }
  if (staged.enabledWikiIds) {
    update.enabledWikiIds = staged.enabledWikiIds;
  }
  if (staged.thinkingBudget !== undefined) {
    update.thinkingBudget = staged.thinkingBudget;
  }
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
    searchEnabled: conversation?.searchEnabled ?? false,
    searchProviderId: conversation?.searchProviderId ?? null,
    thinkingBudget: conversation?.thinkingBudget ?? null,
    mcpMode: ((conversation as Record<string, unknown> | null | undefined)
      ?.mcp_mode as "auto" | "disabled" | "manual") ?? "auto",
    enabledMcpServerIds: [...(conversation?.enabledMcpServerIds ?? [])],
    enabledKnowledgeBaseIds: [
      ...(conversation?.enabledKnowledgeBaseIds ?? []),
    ],
    activeMemoryNamespaceId: (conversation?.enabledMemoryNamespaceIds ?? [])[0] ?? null,
    enabledWikiIds: [...(conversation?.enabledWikiIds ?? [])],
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
  | "searchEnabled"
  | "searchProviderId"
  | "thinkingBudget"
  | "enabledMcpServerIds"
  | "enabledKnowledgeBaseIds"
  | "enabledMemoryNamespaceIds"
  | "enabledWikiIds"
> {
  return {
    searchEnabled: state.searchEnabled,
    searchProviderId: state.searchProviderId,
    thinkingBudget: state.thinkingBudget,
    enabledMcpServerIds: [...state.enabledMcpServerIds],
    enabledKnowledgeBaseIds: [...state.enabledKnowledgeBaseIds],
    enabledMemoryNamespaceIds: state.activeMemoryNamespaceId
      ? [state.activeMemoryNamespaceId]
      : [],
    enabledWikiIds: [...state.enabledWikiIds],
  };
}

export function nextConversationPreferenceSaveSeq(
  conversationId: string,
): number {
  const next = (_conversationPreferenceSaveSeq.get(conversationId) ?? 0) + 1;
  _conversationPreferenceSaveSeq.set(conversationId, next);
  return next;
}

export function isLatestConversationPreferenceSave(
  conversationId: string,
  seq: number,
): boolean {
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
    conversations: conversations.map((conversation) => conversation.id === updated.id ? updated : conversation),
    archivedConversations: archivedConversations.map((conversation) =>
      conversation.id === updated.id ? updated : conversation
    ),
  };
}

export function categoryTemplateUpdateFromCategory(
  category?: {
    id: string;
    systemPrompt?: string | null;
    defaultTemperature?: number | null;
    defaultMaxTokens?: number | null;
    defaultTopP?: number | null;
    defaultFrequencyPenalty?: number | null;
  } | null,
): Pick<
  UpdateConversationInput,
  | "categoryId"
  | "systemPrompt"
  | "temperature"
  | "maxTokens"
  | "topP"
  | "frequencyPenalty"
> {
  if (!category) {
    return {};
  }

  return {
    categoryId: category.id,
    systemPrompt: category.systemPrompt ?? undefined,
    temperature: category.defaultTemperature,
    maxTokens: category.defaultMaxTokens,
    topP: category.defaultTopP,
    frequencyPenalty: category.defaultFrequencyPenalty,
  };
}

import { invoke } from "@/lib/invoke";
import { findModelByIds, supportsReasoning } from "@/lib/modelCapabilities";
import type { Conversation, UpdateConversationInput } from "@/types";
import { create } from "zustand";
import { useProviderStore } from "../feature/providerStore";
import { useConversationStore } from "./conversationStore";

// Sequence counter to prevent stale preference saves
const _conversationPreferenceSaveSeq = new Map<string, number>();

type ConversationPreferenceState = Pick<
  PreferenceState,
  | "searchEnabled"
  | "searchProviderId"
  | "thinkingBudget"
  | "enabledMcpServerIds"
  | "enabledKnowledgeBaseIds"
  | "enabledMemoryNamespaceIds"
>;

function conversationPreferenceStateFromConversation(
  conversation?: Conversation | null,
): ConversationPreferenceState {
  return {
    searchEnabled: conversation?.search_enabled ?? false,
    searchProviderId: conversation?.search_provider_id ?? null,
    thinkingBudget: conversation?.thinking_budget ?? null,
    enabledMcpServerIds: [...(conversation?.enabled_mcp_server_ids ?? [])],
    enabledKnowledgeBaseIds: [...(conversation?.enabled_knowledge_base_ids ?? [])],
    enabledMemoryNamespaceIds: [...(conversation?.enabled_memory_namespace_ids ?? [])],
  };
}

function conversationPreferenceUpdateFromState(
  state: Pick<
    PreferenceState,
    | "searchEnabled"
    | "searchProviderId"
    | "thinkingBudget"
    | "enabledMcpServerIds"
    | "enabledKnowledgeBaseIds"
    | "enabledMemoryNamespaceIds"
  >,
): Pick<
  UpdateConversationInput,
  | "search_enabled"
  | "search_provider_id"
  | "thinking_budget"
  | "enabled_mcp_server_ids"
  | "enabled_knowledge_base_ids"
  | "enabled_memory_namespace_ids"
> {
  return {
    search_enabled: state.searchEnabled,
    search_provider_id: state.searchProviderId,
    thinking_budget: state.thinkingBudget,
    enabled_mcp_server_ids: [...state.enabledMcpServerIds],
    enabled_knowledge_base_ids: [...state.enabledKnowledgeBaseIds],
    enabled_memory_namespace_ids: [...state.enabledMemoryNamespaceIds],
  };
}

function nextConversationPreferenceSaveSeq(conversationId: string): number {
  const next = (_conversationPreferenceSaveSeq.get(conversationId) ?? 0) + 1;
  _conversationPreferenceSaveSeq.set(conversationId, next);
  return next;
}

function isLatestConversationPreferenceSave(conversationId: string, seq: number): boolean {
  return (_conversationPreferenceSaveSeq.get(conversationId) ?? 0) === seq;
}

function preferenceStateMatches(
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

function mergeConversationCollections(
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

async function persistConversationPreferences(
  conversationId: string,
  input: Partial<UpdateConversationInput>,
  optimisticState: Partial<ConversationPreferenceState>,
  rollbackState: Partial<ConversationPreferenceState>,
) {
  const requestSeq = nextConversationPreferenceSaveSeq(conversationId);
  try {
    const updated = await invoke<Conversation>("update_conversation", { id: conversationId, input });
    if (!isLatestConversationPreferenceSave(conversationId, requestSeq)) { return; }

    const convState = useConversationStore.getState();
    useConversationStore.setState((state) => ({
      ...mergeConversationCollections(state.conversations, state.archivedConversations, updated),
      ...(state.activeConversationId === conversationId
        ? {} // preferenceStore will handle its own state
        : {}),
      error: null,
    }));

    // Update preferenceStore state
    if (convState.activeConversationId === conversationId) {
      usePreferenceStore.setState(conversationPreferenceStateFromConversation(updated));
    }
  } catch (error) {
    if (!isLatestConversationPreferenceSave(conversationId, requestSeq)) { return; }

    const prefState = usePreferenceStore.getState();
    const convState = useConversationStore.getState();
    if (
      convState.activeConversationId !== conversationId
      || !preferenceStateMatches({
        searchEnabled: prefState.searchEnabled,
        searchProviderId: prefState.searchProviderId,
        thinkingBudget: prefState.thinkingBudget,
        enabledMcpServerIds: prefState.enabledMcpServerIds,
        enabledKnowledgeBaseIds: prefState.enabledKnowledgeBaseIds,
        enabledMemoryNamespaceIds: prefState.enabledMemoryNamespaceIds,
      }, optimisticState)
    ) {
      useConversationStore.setState({ error: String(error) });
      return;
    }

    usePreferenceStore.setState(rollbackState);
    useConversationStore.setState({ error: String(error) });
  }
}

export function getEffectiveThinkingBudget(conversationId: string): number | undefined {
  const thinkingBudget = usePreferenceStore.getState().thinkingBudget;
  if (thinkingBudget === null) { return undefined; }

  const conversation = useConversationStore.getState().conversations.find((item) => item.id === conversationId);
  if (!conversation) { return thinkingBudget; }

  const providers = useProviderStore.getState().providers;
  const model = findModelByIds(providers, conversation.provider_id, conversation.model_id);
  if (!model) { return thinkingBudget; }
  return supportsReasoning(model) ? thinkingBudget : undefined;
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

// Re-export for use in conversationStore's setActiveConversation
export {
  conversationPreferenceStateFromConversation,
  conversationPreferenceUpdateFromState,
  mergeConversationCollections,
};

interface PreferenceState {
  searchEnabled: boolean;
  searchProviderId: string | null;
  enabledMcpServerIds: string[];
  thinkingBudget: number | null;
  enabledKnowledgeBaseIds: string[];
  enabledMemoryNamespaceIds: string[];
  setSearchEnabled: (enabled: boolean) => void;
  setSearchProviderId: (id: string | null) => void;
  setEnabledMcpServerIds: (ids: string[]) => void;
  toggleMcpServer: (id: string) => void;
  setThinkingBudget: (budget: number | null) => void;
  setEnabledKnowledgeBaseIds: (ids: string[]) => void;
  toggleKnowledgeBase: (id: string) => void;
  setEnabledMemoryNamespaceIds: (ids: string[]) => void;
  toggleMemoryNamespace: (id: string) => void;
}

export const usePreferenceStore = create<PreferenceState>((set, get) => ({
  searchEnabled: false,
  searchProviderId: null,
  enabledMcpServerIds: [],
  thinkingBudget: null,
  enabledKnowledgeBaseIds: [],
  enabledMemoryNamespaceIds: [],

  setSearchEnabled: (enabled) => {
    const previous = get().searchEnabled;
    const conversationId = useConversationStore.getState().activeConversationId;
    set({ searchEnabled: enabled });
    if (conversationId) {
      void persistConversationPreferences(
        conversationId,
        { search_enabled: enabled },
        { searchEnabled: enabled },
        { searchEnabled: previous },
      );
    }
  },
  setSearchProviderId: (id) => {
    const previous = get().searchProviderId;
    const conversationId = useConversationStore.getState().activeConversationId;
    set({ searchProviderId: id });
    if (conversationId) {
      void persistConversationPreferences(
        conversationId,
        { search_provider_id: id },
        { searchProviderId: id },
        { searchProviderId: previous },
      );
    }
  },
  setEnabledMcpServerIds: (ids) => {
    const previous = get().enabledMcpServerIds;
    const conversationId = useConversationStore.getState().activeConversationId;
    const nextIds = [...ids];
    set({ enabledMcpServerIds: nextIds });
    if (conversationId) {
      void persistConversationPreferences(
        conversationId,
        { enabled_mcp_server_ids: nextIds },
        { enabledMcpServerIds: nextIds },
        { enabledMcpServerIds: previous },
      );
    }
  },
  toggleMcpServer: (id) => {
    const previous = get().enabledMcpServerIds;
    const nextIds = previous.includes(id)
      ? previous.filter((serverId) => serverId !== id)
      : [...previous, id];
    const conversationId = useConversationStore.getState().activeConversationId;
    set({ enabledMcpServerIds: nextIds });
    if (conversationId) {
      void persistConversationPreferences(
        conversationId,
        { enabled_mcp_server_ids: nextIds },
        { enabledMcpServerIds: nextIds },
        { enabledMcpServerIds: previous },
      );
    }
  },
  setThinkingBudget: (budget) => {
    const previous = get().thinkingBudget;
    const conversationId = useConversationStore.getState().activeConversationId;
    set({ thinkingBudget: budget });
    if (conversationId) {
      void persistConversationPreferences(
        conversationId,
        { thinking_budget: budget },
        { thinkingBudget: budget },
        { thinkingBudget: previous },
      );
    }
  },
  setEnabledKnowledgeBaseIds: (ids) => {
    const previous = get().enabledKnowledgeBaseIds;
    const conversationId = useConversationStore.getState().activeConversationId;
    const nextIds = [...ids];
    set({ enabledKnowledgeBaseIds: nextIds });
    if (conversationId) {
      void persistConversationPreferences(
        conversationId,
        { enabled_knowledge_base_ids: nextIds },
        { enabledKnowledgeBaseIds: nextIds },
        { enabledKnowledgeBaseIds: previous },
      );
    }
  },
  toggleKnowledgeBase: (id) => {
    const previous = get().enabledKnowledgeBaseIds;
    const nextIds = previous.includes(id)
      ? previous.filter((knowledgeBaseId) => knowledgeBaseId !== id)
      : [...previous, id];
    const conversationId = useConversationStore.getState().activeConversationId;
    set({ enabledKnowledgeBaseIds: nextIds });
    if (conversationId) {
      void persistConversationPreferences(
        conversationId,
        { enabled_knowledge_base_ids: nextIds },
        { enabledKnowledgeBaseIds: nextIds },
        { enabledKnowledgeBaseIds: previous },
      );
    }
  },
  setEnabledMemoryNamespaceIds: (ids) => {
    const previous = get().enabledMemoryNamespaceIds;
    const conversationId = useConversationStore.getState().activeConversationId;
    const nextIds = [...ids];
    set({ enabledMemoryNamespaceIds: nextIds });
    if (conversationId) {
      void persistConversationPreferences(
        conversationId,
        { enabled_memory_namespace_ids: nextIds },
        { enabledMemoryNamespaceIds: nextIds },
        { enabledMemoryNamespaceIds: previous },
      );
    }
  },
  toggleMemoryNamespace: (id) => {
    const previous = get().enabledMemoryNamespaceIds;
    const nextIds = previous.includes(id)
      ? previous.filter((memoryNamespaceId) => memoryNamespaceId !== id)
      : [...previous, id];
    const conversationId = useConversationStore.getState().activeConversationId;
    set({ enabledMemoryNamespaceIds: nextIds });
    if (conversationId) {
      void persistConversationPreferences(
        conversationId,
        { enabled_memory_namespace_ids: nextIds },
        { enabledMemoryNamespaceIds: nextIds },
        { enabledMemoryNamespaceIds: previous },
      );
    }
  },
}));

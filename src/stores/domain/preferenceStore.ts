import { invoke } from "@/lib/invoke";
import { findModelByIds, supportsReasoning } from "@/lib/modelCapabilities";
import type { Conversation, UpdateConversationInput } from "@/types";
import { create } from "zustand";
import { useMcpStore } from "../feature/mcpStore";
import { useProviderStore } from "../feature/providerStore";
import {
  clearStagedPrefs,
  type ConversationPreferenceState,
  conversationPreferenceStateFromConversation,
  conversationPreferenceUpdateFromState,
  getStagedPreferenceUpdate,
  isLatestConversationPreferenceSave,
  mergeConversationCollections,
  nextConversationPreferenceSaveSeq,
  preferenceStateMatches,
  stagePreference,
} from "./conversationPreferences";
import { useConversationStore } from "./conversationStore";

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
        mcpMode: prefState.mcpMode,
        enabledMcpServerIds: prefState.enabledMcpServerIds,
        enabledKnowledgeBaseIds: prefState.enabledKnowledgeBaseIds,
        activeMemoryNamespaceId: prefState.activeMemoryNamespaceId,
        enabledWikiIds: prefState.enabledWikiIds,
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

// Re-export for use in conversationStore's setActiveConversation
export {
  clearStagedPrefs,
  conversationPreferenceStateFromConversation,
  conversationPreferenceUpdateFromState,
  getStagedPreferenceUpdate,
  mergeConversationCollections,
} from "./conversationPreferences";
export { categoryTemplateUpdateFromCategory } from "./conversationPreferences";

interface PreferenceState {
  searchEnabled: boolean;
  searchProviderId: string | null;
  enabledMcpServerIds: string[];
  mcpMode: "auto" | "manual" | "disabled";
  thinkingBudget: number | null;
  enabledKnowledgeBaseIds: string[];
  activeMemoryNamespaceId: string | null;
  enabledWikiIds: string[];
  setSearchEnabled: (enabled: boolean) => void;
  setSearchProviderId: (id: string | null) => void;
  setEnabledMcpServerIds: (ids: string[]) => void;
  toggleMcpServer: (id: string) => void;
  setMcpMode: (mode: "auto" | "manual" | "disabled") => void;
  setThinkingBudget: (budget: number | null) => void;
  setEnabledKnowledgeBaseIds: (ids: string[]) => void;
  toggleKnowledgeBase: (id: string) => void;
  setActiveMemoryNamespaceId: (id: string | null) => void;
  setActiveMemoryNamespace: (id: string | null) => void;
  setEnabledWikiIds: (ids: string[]) => void;
  toggleWiki: (id: string) => void;
}

export const usePreferenceStore = create<PreferenceState>((set, get) => ({
  searchEnabled: false,
  searchProviderId: null,
  enabledMcpServerIds: [],
  mcpMode: "auto",
  thinkingBudget: null,
  enabledKnowledgeBaseIds: [],
  activeMemoryNamespaceId: null,
  enabledWikiIds: [],

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
    } else {
      stagePreference("searchEnabled", enabled);
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
    } else {
      stagePreference("searchProviderId", id);
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
    } else {
      stagePreference("enabledMcpServerIds", nextIds);
    }
  },

  setMcpMode: (mode) => {
    set({ mcpMode: mode });
    const conversationId = useConversationStore.getState().activeConversationId;
    if (conversationId) {
      const allBuiltinIds = useMcpStore.getState().servers
        .filter((s) => s.source === "builtin" && s.enabled)
        .map((s) => s.id);
      if (mode === "auto") {
        set({ enabledMcpServerIds: allBuiltinIds });
        void persistConversationPreferences(
          conversationId,
          { enabled_mcp_server_ids: allBuiltinIds },
          { enabledMcpServerIds: allBuiltinIds, mcpMode: mode },
          { enabledMcpServerIds: [], mcpMode: "manual" },
        );
      } else if (mode === "disabled") {
        set({ enabledMcpServerIds: [] });
        void persistConversationPreferences(
          conversationId,
          { enabled_mcp_server_ids: [] },
          { enabledMcpServerIds: [], mcpMode: mode },
          { enabledMcpServerIds: allBuiltinIds, mcpMode: "auto" },
        );
      } else {
        void persistConversationPreferences(
          conversationId,
          {},
          { mcpMode: mode },
          { mcpMode: "auto" },
        );
      }
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
    } else {
      stagePreference("thinkingBudget", budget);
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
    } else {
      stagePreference("enabledKnowledgeBaseIds", nextIds);
    }
  },
  setActiveMemoryNamespaceId: (id) => {
    const previous = get().activeMemoryNamespaceId;
    const conversationId = useConversationStore.getState().activeConversationId;
    set({ activeMemoryNamespaceId: id });
    if (conversationId) {
      void persistConversationPreferences(
        conversationId,
        { enabled_memory_namespace_ids: id ? [id] : [] },
        { activeMemoryNamespaceId: id },
        { activeMemoryNamespaceId: previous },
      );
    }
  },
  setActiveMemoryNamespace: (id) => {
    const previous = get().activeMemoryNamespaceId;
    const nextId = previous === id ? null : id;
    const conversationId = useConversationStore.getState().activeConversationId;
    set({ activeMemoryNamespaceId: nextId });
    if (conversationId) {
      void persistConversationPreferences(
        conversationId,
        { enabled_memory_namespace_ids: nextId ? [nextId] : [] },
        { activeMemoryNamespaceId: nextId },
        { activeMemoryNamespaceId: previous },
      );
    } else {
      stagePreference("activeMemoryNamespaceId", nextId);
    }
  },
  setEnabledWikiIds: (ids) => {
    const previous = get().enabledWikiIds;
    const conversationId = useConversationStore.getState().activeConversationId;
    const nextIds = [...ids];
    set({ enabledWikiIds: nextIds });
    if (conversationId) {
      void persistConversationPreferences(
        conversationId,
        { enabled_wiki_ids: nextIds },
        { enabledWikiIds: nextIds },
        { enabledWikiIds: previous },
      );
    }
  },
  toggleWiki: (id) => {
    const previous = get().enabledWikiIds;
    const nextIds = previous.includes(id)
      ? previous.filter((wikiId) => wikiId !== id)
      : [...previous, id];
    const conversationId = useConversationStore.getState().activeConversationId;
    set({ enabledWikiIds: nextIds });
    if (conversationId) {
      void persistConversationPreferences(
        conversationId,
        { enabled_wiki_ids: nextIds },
        { enabledWikiIds: nextIds },
        { enabledWikiIds: previous },
      );
    } else {
      stagePreference("enabledWikiIds", nextIds);
    }
  },
}));

import { invoke, isTauri, listen, type UnlistenFn } from "@/lib/invoke";
import { buildKnowledgeTag, buildMemoryTag, type RagContextRetrievedEvent } from "@/lib/memoryUtils";
import { buildSearchTag, formatSearchContent } from "@/lib/searchUtils";
import { useSearchStore } from "@/stores";
import type {
  AgentDoneEvent,
  AgentErrorEvent,
  AgentStreamTextEvent,
  AgentStreamThinkingEvent,
  AttachmentInput,
  ChatStreamErrorEvent,
  ChatStreamEvent,
  CompareResponsesResult,
  Conversation,
  ConversationBranch,
  ConversationSearchResult,
  ConversationWorkspaceSnapshot,
  Message,
  MessagePage,
  UpdateConversationInput,
  WorkflowEvent,
  WorkflowCompleteEvent,
} from "@/types";
import { create } from "zustand";
import { useCategoryStore } from "../feature/categoryStore";
import { mergeOlderPages, mergePreservedMessages, MESSAGE_PAGE_SIZE } from "./messageStore";
import {
  categoryTemplateUpdateFromCategory,
  conversationPreferenceStateFromConversation,
  conversationPreferenceUpdateFromState,
  getEffectiveThinkingBudget,
  mergeConversationCollections,
  usePreferenceStore,
} from "./preferenceStore";
import {
  _activeMessageLoadSeq,
  _isMultiModelActive,
  _listenerGen,
  _multiModelDoneResolve,
  _multiModelFirstMessageId,
  _multiModelFirstModelId,
  _multiModelTotalRemaining,
  _pendingConversationRefresh,
  _pendingUiChunk,
  _streamBuffer,
  _streamPrefix,
  _streamUiFlushTimer,
  // Module-level variable accessors
  _unlisten,
  _userManuallySelectedVersion,
  addPendingConversationRefresh,
  appendStreamChunk,
  clearPendingConversationRefresh,
  decrementMultiModelTotalRemaining,
  deletePendingConversationRefresh,
  flushPendingStreamChunk,
  getStreamingMessageId,
  incrementActiveMessageLoadSeq,
  incrementListenerGen,
  isConversationStreaming as isConvStreaming,
  rebuildMessageIndex,
  registerConversationStoreRef,
  resetMultiModelState,
  setIsMultiModelActive,
  setMultiModelDoneResolve,
  setMultiModelFirstMessageId,
  setMultiModelFirstModelId,
  setMultiModelTotalRemaining,
  setPendingUiChunk,
  setStreamBuffer,
  setStreamPrefix,
  setStreamUiFlushTimer,
  // Setter functions
  setUnlisten,
  setUserManuallySelectedVersion,
  startConversationStream,
  stopConversationStream,
  STREAM_UI_FLUSH_INTERVAL_MS,
  useStreamStore,
} from "./streamStore";

// ─── Fallback model chain ───
//
// When the primary model fails (rate limit, timeout, provider error),
// we iterate through a chain of fallback models instead of immediately
// showing an error. The chain is built from the user's configured providers.
// This increases reliability significantly for long-running sessions.

interface FallbackModel {
  providerId: string;
  model_id: string;
}

/** Build a fallback model chain from available providers, excluding the current model.
 *  Prioritizes models from the same provider, then the user's default model, then others. */
function buildFallbackChain(
  currentProviderId: string,
  currentModelId: string,
): FallbackModel[] {
  const chain: FallbackModel[] = [];
  try {
    // Access provider store — dynamic import avoids circular dependency
    const { useProviderStore } = require("@/stores");
    const providers = useProviderStore.getState().providers ?? [];
    const { usePreferenceStore } = require("@/stores/domain/preferenceStore");
    const { defaultProviderId, defaultModelId } = usePreferenceStore.getState();

    for (const p of providers) {
      for (const m of p.models ?? []) {
        const key = `${p.id}:${m.model_id}`;
        if (key === `${currentProviderId}:${currentModelId}`) continue;

        const entry: FallbackModel = { providerId: p.id, model_id: m.model_id };

        // Same provider, different model — highest priority
        if (p.id === currentProviderId) {
          chain.unshift(entry);
        } else if (p.id === defaultProviderId && m.model_id === defaultModelId) {
          // User's default model — second priority
          chain.push(entry);
        } else {
          chain.push(entry);
        }
      }
    }
  } catch {
    // If stores aren't available, return empty chain
  }
  return chain.slice(0, 3); // Max 3 fallback attempts
}

interface ConversationState {
  conversations: Conversation[];
  activeConversationId: string | null;
  messages: Message[];
  loading: boolean;
  loadingOlder: boolean;
  hasOlderMessages: boolean;
  totalActiveCount: number;
  oldestLoadedMessageId: string | null;
  error: string | null;
  /** Current streaming message ID (for streamStore compatibility) */
  streamingMessageId: string | null;
  /** Insert a context-clear marker into the conversation */
  insertContextClear: () => Promise<void>;
  /** Remove a context-clear marker */
  removeContextClear: (messageId: string) => Promise<void>;
  /** Clear all messages in the active conversation */
  clearAllMessages: () => Promise<void>;
  fetchConversations: () => Promise<void>;
  setActiveConversation: (id: string | null) => void;
  createConversation: (
    title: string,
    model_id: string,
    providerId: string,
    options?: { categoryId?: string | null; scenario?: string | null },
  ) => Promise<Conversation>;
  updateConversation: (id: string, input: UpdateConversationInput) => Promise<void>;
  renameConversation: (id: string, title: string) => Promise<void>;
  deleteConversation: (id: string) => Promise<void>;
  branchConversation: (
    conversationId: string,
    untilMessageId: string,
    asChild: boolean,
    title?: string,
  ) => Promise<Conversation>;
  togglePin: (id: string) => Promise<void>;
  toggleArchive: (id: string) => Promise<void>;
  archiveToKnowledgeBase: (id: string, knowledgeBaseId: string) => Promise<void>;
  archivedConversations: Conversation[];
  fetchArchivedConversations: () => Promise<void>;
  batchDelete: (ids: string[]) => Promise<void>;
  batchArchive: (ids: string[]) => Promise<void>;
  sendMessage: (content: string, attachments?: AttachmentInput[], searchProviderId?: string | null) => Promise<void>;
  /** Send a message in agent mode (non-streaming MVP) */
  sendAgentMessage: (content: string, attachments?: AttachmentInput[]) => Promise<void>;
  regenerateMessage: (targetMessageId?: string) => Promise<void>;
  regenerateWithModel: (targetMessageId: string, providerId: string, model_id: string) => Promise<void>;
  deleteMessage: (messageId: string) => Promise<void>;
  fetchMessages: (conversationId: string, preserveMessageIds?: string[]) => Promise<void>;
  loadOlderMessages: () => Promise<void>;
  searchConversations: (query: string) => Promise<ConversationSearchResult[]>;
  startStreamListening: () => Promise<void>;
  switchMessageVersion: (conversationId: string, parentMessageId: string, messageId: string) => Promise<void>;
  listMessageVersions: (conversationId: string, parentMessageId: string) => Promise<Message[]>;
  updateMessageContent: (messageId: string, content: string) => Promise<void>;
  deleteMessageGroup: (conversationId: string, userMessageId: string) => Promise<void>;
  workspaceSnapshot: ConversationWorkspaceSnapshot | null;
  loadWorkspaceSnapshot: (conversationId: string) => Promise<ConversationWorkspaceSnapshot | null>;
  updateWorkspaceSnapshot: (conversationId: string, snapshot: Partial<ConversationWorkspaceSnapshot>) => Promise<void>;
  forkConversation: (conversationId: string, fromMessageId?: string) => Promise<ConversationBranch | null>;
  compareResponses: (leftMessageId: string, rightMessageId: string) => Promise<CompareResponsesResult | null>;
  /** Conversation ID currently generating an AI title (null if none) */
  titleGeneratingConversationId: string | null;
  /** Regenerate the title of a conversation using AI */
  regenerateTitle: (conversationId: string) => Promise<void>;
  /** Companion models pending or currently streaming (for multi-model simultaneous response) */
  pendingCompanionModels: Array<{ providerId: string; model_id: string }>;
  /** User message ID of the current multi-model request (for scoping UI indicators) */
  multiModelParentId: string | null;
  /** Message IDs of models that have completed their streams (for per-model loading indicators) */
  multiModelDoneMessageIds: string[];
  /** Send a message and generate responses from multiple companion models */
  sendMultiModelMessage: (
    content: string,
    companionModels: Array<{ providerId: string; model_id: string }>,
    attachments?: AttachmentInput[],
    searchProviderId?: string | null,
  ) => Promise<void>;
  /** Pending prompt text from welcome cards — InputArea picks it up and sends with companion awareness */
  pendingPromptText: string | null;
  setPendingPromptText: (text: string | null) => void;
  searchEnabled: boolean;
  searchProviderId: string | null;
  thinkingBudget: number | null;
  enabledMcpServerIds: string[];
  enabledKnowledgeBaseIds: string[];
  enabledMemoryNamespaceIds: string[];
  setSearchEnabled: (enabled: boolean) => void;
  setSearchProviderId: (id: string | null) => void;
  toggleMcpServer: (id: string) => void;
  setThinkingBudget: (budget: number | null) => void;
  toggleKnowledgeBase: (id: string) => void;
  toggleMemoryNamespace: (id: string) => void;
}

export const useConversationStore = create<ConversationState>((set, get) => ({
  conversations: [],
  activeConversationId: null,
  messages: [],
  loading: false,
  loadingOlder: false,
  hasOlderMessages: false,
  totalActiveCount: 0,
  oldestLoadedMessageId: null,
  error: null,
  streamingMessageId: null,
  titleGeneratingConversationId: null,
  pendingCompanionModels: [],
  multiModelParentId: null,
  multiModelDoneMessageIds: [],
  pendingPromptText: null,
  setPendingPromptText: (text) => set({ pendingPromptText: text }),
  searchEnabled: usePreferenceStore.getState().searchEnabled,
  searchProviderId: usePreferenceStore.getState().searchProviderId,
  thinkingBudget: usePreferenceStore.getState().thinkingBudget,
  enabledMcpServerIds: usePreferenceStore.getState().enabledMcpServerIds,
  enabledKnowledgeBaseIds: usePreferenceStore.getState().enabledKnowledgeBaseIds,
  enabledMemoryNamespaceIds: usePreferenceStore.getState().enabledMemoryNamespaceIds,
  setSearchEnabled: (enabled) => {
    usePreferenceStore.getState().setSearchEnabled(enabled);
    set({ searchEnabled: enabled });
  },
  setSearchProviderId: (id) => {
    usePreferenceStore.getState().setSearchProviderId(id);
    set({ searchProviderId: id });
  },
  toggleMcpServer: async (id) => {
    const current = get().enabledMcpServerIds;
    const next = current.includes(id) ? current.filter((s) => s !== id) : [...current, id];
    set({ enabledMcpServerIds: next });
    try {
      await usePreferenceStore.getState().toggleMcpServer(id);
    } catch (e) {
      set({ enabledMcpServerIds: current });
      throw e;
    }
  },
  setThinkingBudget: (budget) => {
    usePreferenceStore.getState().setThinkingBudget(budget);
    set({ thinkingBudget: budget });
  },
  toggleKnowledgeBase: (id) => {
    const current = get().enabledKnowledgeBaseIds;
    const next = current.includes(id) ? current.filter((s) => s !== id) : [...current, id];
    usePreferenceStore.getState().toggleKnowledgeBase(id);
    set({ enabledKnowledgeBaseIds: next });
  },
  toggleMemoryNamespace: (id) => {
    const current = get().enabledMemoryNamespaceIds;
    const next = current.includes(id) ? current.filter((s) => s !== id) : [...current, id];
    usePreferenceStore.getState().toggleMemoryNamespace(id);
    set({ enabledMemoryNamespaceIds: next });
  },
  insertContextClear: async () => {
    const conversationId = get().activeConversationId;
    if (!conversationId) { return; }
    try {
      const msg = await invoke<Message>("send_system_message", {
        conversationId,
        content: "<!-- context-clear -->",
      });
      set((s) => ({ messages: [...s.messages, msg] }));
      // Backup and clear agent SDK context (no-op if no agent session exists)
      await invoke("agent_backup_and_clear_sdk_context", { conversationId }).catch(() => {});
    } catch {
      // If backend command doesn't exist yet, add optimistic local message
      const localMsg: Message = {
        id: `ctx-clear-${Date.now()}`,
        conversation_id: conversationId,
        role: "system",
        content: "<!-- context-clear -->",
        provider_id: null,
        model_id: null,
        token_count: null,
        attachments: [],
        thinking: null,
        tool_calls_json: null,
        tool_call_id: null,
        created_at: Date.now(),
        parent_message_id: null,
        version_index: 0,
        is_active: true,
        status: "complete",
      };
      set((s) => ({ messages: [...s.messages, localMsg] }));
    }
  },
  removeContextClear: async (messageId) => {
    const conversationId = get().activeConversationId;
    if (messageId.startsWith("ctx-clear-") || messageId.startsWith("temp-")) {
      set((s) => ({ messages: s.messages.filter((m) => m.id !== messageId) }));
      return;
    }

    try {
      await invoke("delete_message", { id: messageId });
      set((s) => ({ messages: s.messages.filter((m) => m.id !== messageId) }));
      // Restore agent SDK context from backup (no-op if no agent session or no backup)
      if (conversationId) {
        await invoke("agent_restore_sdk_context_from_backup", { conversationId }).catch(() => {});
      }
    } catch (e) {
      set({ error: String(e) });
      throw e;
    }
  },

  clearAllMessages: async () => {
    const conversationId = get().activeConversationId;
    if (!conversationId) { return; }
    try {
      await invoke("clear_conversation_messages", { conversationId });
      set({
        messages: [],
        hasOlderMessages: false,
        totalActiveCount: 0,
        oldestLoadedMessageId: null,
        loadingOlder: false,
      });
    } catch (e) {
      console.error("Failed to clear messages:", e);
    }
  },

  fetchConversations: async () => {
    set({ loading: true });
    try {
      const conversations = await invoke<Conversation[]>("list_conversations");
      set({ conversations, loading: false, error: null });
    } catch (e) {
      set({ error: String(e), loading: false });
    }
  },

  setActiveConversation: (id) => {
    if (id === get().activeConversationId && (!id || !_pendingConversationRefresh.has(id))) {
      return;
    }
    incrementActiveMessageLoadSeq();
    if (!id) {
      if (get().activeConversationId === null) { return; }
      set({
        activeConversationId: null,
        messages: [],
        loading: false,
        loadingOlder: false,
        hasOlderMessages: false,
        totalActiveCount: 0,
        oldestLoadedMessageId: null,
      });
      return;
    }

    const conversation = get().conversations.find((item) => item.id === id)
      ?? get().archivedConversations.find((item) => item.id === id);
    const requestSeq = _activeMessageLoadSeq;

    // Check if this conversation had a stream complete while we were away
    const needsRefreshAfterStreamDone = _pendingConversationRefresh.has(id);
    if (needsRefreshAfterStreamDone) {
      deletePendingConversationRefresh(id);
    }

    const prefState = conversationPreferenceStateFromConversation(conversation);
    set({
      activeConversationId: id,
      messages: [],
      loading: true,
      loadingOlder: false,
      hasOlderMessages: false,
      totalActiveCount: 0,
      oldestLoadedMessageId: null,
      error: null,
      searchEnabled: prefState.searchEnabled,
      searchProviderId: prefState.searchProviderId,
      thinkingBudget: prefState.thinkingBudget,
      enabledMcpServerIds: prefState.enabledMcpServerIds,
      enabledKnowledgeBaseIds: prefState.enabledKnowledgeBaseIds,
      enabledMemoryNamespaceIds: prefState.enabledMemoryNamespaceIds,
    });
    // Sync preference state from the conversation (direct setState to avoid triggering persistence)
    usePreferenceStore.setState(prefState);
    get().fetchMessages(id).then(() => {
      if (requestSeq !== _activeMessageLoadSeq || get().activeConversationId !== id) {
        return;
      }
      // If there's an active stream for this conversation, inject buffered content
      if (
        _streamBuffer && _streamBuffer.conversationId === id
        && isConvStreaming(useStreamStore.getState().activeStreams, id)
      ) {
        const realId = _streamBuffer.resolvedId ?? _streamBuffer.messageId;
        set((s) => {
          const exists = s.messages.some((m) => m.id === realId);
          if (exists) {
            // Message already fetched from backend — replace with buffered content (more up-to-date)
            useStreamStore.setState({ streamingMessageId: realId });
            return {
              messages: s.messages.map((m) =>
                m.id === realId
                  ? { ...m, content: _streamBuffer!.content, thinking: _streamBuffer!.thinking || null }
                  : m
              ),
            };
          }
          // Message not yet in backend — create from buffer
          const newMessage: Message = {
            id: realId,
            conversation_id: id,
            role: "assistant",
            content: _streamBuffer!.content,
            provider_id: null,
            model_id: null,
            token_count: null,
            attachments: [],
            thinking: _streamBuffer!.thinking || null,
            tool_calls_json: null,
            tool_call_id: null,
            created_at: Date.now(),
            parent_message_id: null,
            version_index: 0,
            is_active: true,
            status: "partial",
          };
          useStreamStore.setState({ streamingMessageId: realId });
          return {
            messages: [...s.messages, newMessage],
          };
        });
      } else if (_streamBuffer && _streamBuffer.conversationId === id && needsRefreshAfterStreamDone) {
        // Stream completed while user was away — buffer still has final content.
        // fetchMessages already loaded the completed message from DB, but inject
        // buffer content in case the DB response is slightly behind.
        const realId = _streamBuffer.resolvedId ?? _streamBuffer.messageId;
        set((s) => {
          const exists = s.messages.some((m) => m.id === realId);
          if (exists) {
            return {
              messages: s.messages.map((m) =>
                m.id === realId
                  ? { ...m, content: _streamBuffer!.content, thinking: _streamBuffer!.thinking || null }
                  : m
              ),
            };
          }
          return {};
        });
        setStreamBuffer(null);
      } else if (needsRefreshAfterStreamDone) {
        // Stream completed while away and buffer was already consumed — the
        // fetchMessages above should have loaded the final message from DB.
        // Clear any stale buffer reference.
        setStreamBuffer(null);
      }
    });
  },

  createConversation: async (title, model_id, providerId, options) => {
    try {
      const category = options?.categoryId
        ? useCategoryStore.getState().categories.find((item) => item.id === options.categoryId) ?? null
        : null;
      const templateProviderId = category?.default_provider_id ?? providerId;
      const templateModelId = category?.default_model_id ?? model_id;
      if (!templateModelId || !templateProviderId) {
        throw new Error(
          "Cannot create conversation: model_id and provider_id are required. Please configure a provider and model first.",
        );
      }
      const createdConversation = await invoke<Conversation>("create_conversation", {
        title,
        modelId: templateModelId,
        providerId: templateProviderId,
        systemPrompt: category?.system_prompt ?? undefined,
      });
      let conversation = createdConversation;
      try {
        conversation = await invoke<Conversation>("update_conversation", {
          id: createdConversation.id,
          input: {
            ...categoryTemplateUpdateFromCategory(category),
            ...conversationPreferenceUpdateFromState(usePreferenceStore.getState()),
            scenario: options?.scenario,
          },
        });
      } catch (preferenceError) {
        set({ error: String(preferenceError) });
      }
      set((s) => ({
        conversations: [conversation, ...s.conversations],
        activeConversationId: conversation.id,
        messages: [],
        error: null,
      }));
      // Sync preference state from the created conversation
      usePreferenceStore.setState(conversationPreferenceStateFromConversation(conversation));
      return conversation;
    } catch (e) {
      set({ error: String(e) });
      throw e;
    }
  },

  updateConversation: async (id, input) => {
    try {
      const updated = await invoke<Conversation>("update_conversation", { id, input });
      set((s) => ({
        ...mergeConversationCollections(s.conversations, s.archivedConversations, updated),
        error: null,
      }));
      // Sync preference state if this is the active conversation
      if (get().activeConversationId === id) {
        usePreferenceStore.setState(conversationPreferenceStateFromConversation(updated));
      }
    } catch (e) {
      set({ error: String(e) });
      throw e;
    }
  },

  renameConversation: async (id, title) => {
    await get().updateConversation(id, { title });
  },

  regenerateTitle: async (conversationId) => {
    try {
      await invoke("regenerate_conversation_title", { conversationId });
    } catch (e) {
      set({ error: String(e) });
    }
  },

  deleteConversation: async (id) => {
    try {
      await invoke("delete_conversation", { id });
      const state = get();
      set({
        conversations: state.conversations.filter((c) => c.id !== id),
        activeConversationId: state.activeConversationId === id ? null : state.activeConversationId,
        messages: state.activeConversationId === id ? [] : state.messages,
        error: null,
      });
    } catch (e) {
      set({ error: String(e) });
      throw e;
    }
  },

  branchConversation: async (conversationId, untilMessageId, asChild, title) => {
    try {
      const newConv = await invoke<Conversation>("branch_conversation", {
        conversationId,
        untilMessageId,
        asChild,
        title: title || null,
      });
      set((s) => ({
        conversations: [newConv, ...s.conversations],
        activeConversationId: newConv.id,
        messages: [],
        error: null,
      }));
      // Load the branched messages
      const msgs = await invoke<Message[]>("list_messages", { conversationId: newConv.id });
      set({ messages: msgs });
      return newConv;
    } catch (e) {
      set({ error: String(e) });
      throw e;
    }
  },

  togglePin: async (id) => {
    try {
      const updated = await invoke<Conversation>("toggle_pin_conversation", { id });
      set((s) => ({
        conversations: s.conversations.map((c) => (c.id === id ? updated : c)),
        error: null,
      }));
    } catch (e) {
      set({ error: String(e) });
      throw e;
    }
  },

  archivedConversations: [],

  toggleArchive: async (id) => {
    try {
      const updated = await invoke<Conversation>("toggle_archive_conversation", { id });
      if (updated.is_archived) {
        // Moved to archive — remove from active list, add to archived
        set((s) => ({
          conversations: s.conversations.filter((c) => c.id !== id),
          archivedConversations: [updated, ...s.archivedConversations],
          activeConversationId: s.activeConversationId === id ? null : s.activeConversationId,
          messages: s.activeConversationId === id ? [] : s.messages,
          error: null,
        }));
      } else {
        // Unarchived — remove from archived, add to active
        set((s) => ({
          conversations: [updated, ...s.conversations],
          archivedConversations: s.archivedConversations.filter((c) => c.id !== id),
          error: null,
        }));
      }
    } catch (e) {
      set({ error: String(e) });
      throw e;
    }
  },

  archiveToKnowledgeBase: async (id, knowledgeBaseId) => {
    try {
      const updated = await invoke<Conversation>("archive_conversation_to_knowledge_base", {
        id,
        knowledgeBaseId,
      });
      // Archive succeeded — move from active list to archived list
      set((s) => ({
        conversations: s.conversations.filter((c) => c.id !== id),
        archivedConversations: [updated, ...s.archivedConversations],
        activeConversationId: s.activeConversationId === id ? null : s.activeConversationId,
        messages: s.activeConversationId === id ? [] : s.messages,
        error: null,
      }));
    } catch (e) {
      set({ error: String(e) });
      throw e;
    }
  },

  fetchArchivedConversations: async () => {
    try {
      const archived = await invoke<Conversation[]>("list_archived_conversations");
      set({ archivedConversations: archived, error: null });
    } catch (e) {
      set({ error: String(e) });
    }
  },

  batchDelete: async (ids) => {
    const errors: string[] = [];
    for (const id of ids) {
      try {
        await invoke("delete_conversation", { id });
      } catch (e) {
        errors.push(String(e));
      }
    }
    set((s) => ({
      conversations: s.conversations.filter((c) => !ids.includes(c.id)),
      activeConversationId: ids.includes(s.activeConversationId ?? "") ? null : s.activeConversationId,
      messages: ids.includes(s.activeConversationId ?? "") ? [] : s.messages,
      error: errors.length ? errors.join("; ") : null,
    }));
  },

  batchArchive: async (ids) => {
    const archived: Conversation[] = [];
    for (const id of ids) {
      try {
        const updated = await invoke<Conversation>("toggle_archive_conversation", { id });
        if (updated.is_archived) { archived.push(updated); }
      } catch (_) { /* skip */ }
    }
    set((s) => ({
      conversations: s.conversations.filter((c) => !ids.includes(c.id)),
      archivedConversations: [...archived, ...s.archivedConversations],
      activeConversationId: ids.includes(s.activeConversationId ?? "") ? null : s.activeConversationId,
      messages: ids.includes(s.activeConversationId ?? "") ? [] : s.messages,
      error: null,
    }));
  },

  sendMessage: async (content, attachments = [], searchProviderId = null) => {
    const conversationId = get().activeConversationId;
    if (!conversationId) { throw new Error("No active conversation"); }

    // Guard: prevent duplicate sends while a stream is already active for this conversation
    if (isConvStreaming(useStreamStore.getState().activeStreams, conversationId)) {
      console.warn("[sendMessage] Ignoring duplicate send — stream already active for", conversationId);
      return;
    }

    // Optimistically add user message BEFORE backend call
    const optimisticUserMsg: Message = {
      id: `temp-user-${Date.now()}`,
      conversation_id: conversationId,
      role: "user",
      content,
      provider_id: null,
      model_id: null,
      token_count: null,
      attachments: attachments.map((a) => ({
        id: `temp-att-${Date.now()}`,
        file_name: a.file_name,
        file_type: a.file_type,
        file_path: "",
        file_size: a.file_size,
        data: a.data,
      })),
      thinking: null,
      tool_calls_json: null,
      tool_call_id: null,
      created_at: Date.now(),
      parent_message_id: null,
      version_index: 0,
      is_active: true,
      status: "complete",
    };

    // Create assistant placeholder upfront (for search status or streaming)
    const tempAssistantId = `temp-assistant-${Date.now()}`;
    const kbIds = usePreferenceStore.getState().enabledKnowledgeBaseIds;
    const memIds = usePreferenceStore.getState().enabledMemoryNamespaceIds;
    const hasKnowledgeRag = kbIds.length > 0;
    const hasMemoryRag = memIds.length > 0;
    const hasAnyRag = hasKnowledgeRag || hasMemoryRag;
    let placeholderContent = "";
    if (searchProviderId) { placeholderContent += buildSearchTag("searching"); }
    if (hasKnowledgeRag) { placeholderContent += buildKnowledgeTag("searching"); }
    if (hasMemoryRag) { placeholderContent += buildMemoryTag("searching"); }
    const placeholderAssistant: Message = {
      id: tempAssistantId,
      conversation_id: conversationId,
      role: "assistant",
      content: placeholderContent,
      provider_id: null,
      model_id: null,
      token_count: null,
      attachments: [],
      thinking: null,
      tool_calls_json: null,
      tool_call_id: null,
      created_at: Date.now(),
      parent_message_id: optimisticUserMsg.id,
      version_index: 0,
      is_active: true,
      status: "partial",
    };

    set((s) => ({
      messages: [...s.messages, optimisticUserMsg, placeholderAssistant],
    }));
    useStreamStore.setState((s) => ({
      ...startConversationStream(s.activeStreams, conversationId, tempAssistantId),
      streamingStartTimestamps: { ...s.streamingStartTimestamps, [conversationId]: Date.now() },
      thinkingActiveMessageIds: new Set<string>(),
    }));
    setPendingUiChunk(null);
    if (_streamUiFlushTimer !== null) {
      clearTimeout(_streamUiFlushTimer);
      setStreamUiFlushTimer(null);
    }

    try {
      // If web search is enabled, execute search before sending to backend
      let finalContent = content;
      if (searchProviderId) {
        let searchResultTag = "";
        try {
          const searchResult = await useSearchStore.getState().executeSearch(searchProviderId, content);
          if (searchResult?.ok && searchResult.results.length > 0) {
            finalContent = formatSearchContent(searchResult.results, content);
            searchResultTag = buildSearchTag("done", searchResult.results);
          }
        } catch (e) {
          // Search failed, continue without search results
        }
        // Replace searching tag with results, keep RAG searching tags if present
        const kbPart = hasKnowledgeRag ? buildKnowledgeTag("searching") : "";
        const memPart = hasMemoryRag ? buildMemoryTag("searching") : "";
        setStreamPrefix(searchResultTag + kbPart + memPart);
        set((s) => ({
          messages: s.messages.map(m =>
            m.id === tempAssistantId ? { ...m, content: searchResultTag + kbPart + memPart } : m
          ),
        }));
      } else if (hasAnyRag) {
        // RAG only — set prefix so searching tags flow into stream buffer
        const kbPart = hasKnowledgeRag ? buildKnowledgeTag("searching") : "";
        const memPart = hasMemoryRag ? buildMemoryTag("searching") : "";
        setStreamPrefix(kbPart + memPart);
      }

      const mcpIds = usePreferenceStore.getState().enabledMcpServerIds;
      const thinkingBudget = getEffectiveThinkingBudget(conversationId);
      const kbIds = usePreferenceStore.getState().enabledKnowledgeBaseIds;
      const memIds = usePreferenceStore.getState().enabledMemoryNamespaceIds;
      const userMessage = await invoke<Message>("send_message", {
        conversationId,
        content: finalContent,
        attachments,
        enabledMcpServerIds: mcpIds.length > 0 ? mcpIds : undefined,
        thinkingBudget,
        enabledKnowledgeBaseIds: kbIds.length > 0 ? kbIds : undefined,
        enabledMemoryNamespaceIds: memIds.length > 0 ? memIds : undefined,
      });

      // Replace optimistic user msg with real one, update placeholder parent
      set((s) => ({
        messages: s.messages.map(m => {
          if (m.id === optimisticUserMsg.id) { return userMessage; }
          if (m.id === tempAssistantId) { return { ...m, parent_message_id: userMessage.id }; }
          return m;
        }),
      }));

      // In browser mode, simulate brief loading then fetch the mock AI response
      if (!isTauri()) {
        await new Promise((r) => setTimeout(r, 600));
        useStreamStore.setState((s) => ({
          ...stopConversationStream(s.activeStreams, conversationId),
          streamingStartTimestamps: (() => {
            const t = { ...s.streamingStartTimestamps };
            delete t[conversationId];
            return t;
          })(),
          thinkingActiveMessageIds: new Set<string>(),
        }));
        get().fetchMessages(conversationId);
      }
    } catch (e) {
      console.error("[sendMessage] error:", e);
      const errMsg = String(e);

      // Determine whether this error is retryable (transient) vs permanent.
      // Only attempt fallback for network, rate limit, timeout, and provider errors.
      const isRetryable = true
        && !errMsg.includes("invalid_request_error")   // bad request
        && !errMsg.includes("authentication")           // auth error
        && !errMsg.includes("insufficient_quota")       // billing
        && !errMsg.includes("invalid_api_key")          // auth
        && !errMsg.includes("context_length_exceeded"); // context too long

      // Try fallback models before showing error
      if (isRetryable) {
        const conversation = get().conversations.find(c => c.id === conversationId);
        const currentProviderId = conversation?.provider_id;
        const currentModelId = conversation?.model_id;

        if (currentProviderId && currentModelId) {
          const fallbackChain = buildFallbackChain(currentProviderId, currentModelId);
          for (let i = 0; i < fallbackChain.length; i++) {
            const fb = fallbackChain[i];
            try {
              // Switch conversation to fallback model
              await get().updateConversation(conversationId, {
                provider_id: fb.providerId,
                model_id: fb.model_id,
              });

              // Re-check streaming guard
              const currentActiveStreams = useStreamStore.getState().activeStreams;
              if (isConvStreaming(currentActiveStreams, conversationId)) {
                return; // Another stream started, abort fallback
              }

              // Reset placeholder for retry
              set((s) => {
                const filtered = s.messages.filter(m =>
                  m.id !== currentStreamingMessageId
                  && m.id !== `temp-error-${Date.now()}`
                  && !(m.status === "error" && m.role === "assistant" && m.content === errMsg)
                );
                return { messages: filtered };
              });

              // Retry sendMessage — uses the conversation's now-updated model
              await get().sendMessage(content, attachments, searchProviderId);
              return; // Success! Fallback worked.
            } catch (fallbackError) {
              console.warn(
                `[sendMessage] Fallback ${i + 1}/${fallbackChain.length} (${fb.model_id}) also failed:`,
                fallbackError,
              );
              // Continue to next fallback
            }
          }
        }
      }

      // All fallbacks exhausted or error not retryable — show error
      const currentStreamingMessageId = getStreamingMessageId(useStreamStore.getState().activeStreams, conversationId);
      useStreamStore.setState((s) => ({
        ...stopConversationStream(s.activeStreams, conversationId),
        streamingStartTimestamps: (() => {
          const t = { ...s.streamingStartTimestamps };
          delete t[conversationId];
          return t;
        })(),
        thinkingActiveMessageIds: new Set<string>(),
      }));
      set((s) => ({
        messages: currentStreamingMessageId
          ? s.messages.map(m =>
            m.id === currentStreamingMessageId
              ? { ...m, content: errMsg, status: "error" as const }
              : m
          )
          : [...s.messages, {
            id: `temp-error-${Date.now()}`,
            conversation_id: conversationId,
            role: "assistant" as const,
            content: errMsg,
            provider_id: null,
            model_id: null,
            token_count: null,
            attachments: [],
            thinking: null,
            tool_calls_json: null,
            tool_call_id: null,
            created_at: Date.now(),
            parent_message_id: null,
            version_index: 0,
            is_active: true,
            status: "error" as const,
          }],
      }));
      // Sync messages from DB so temp- prefixed user messages get replaced
      // with real backend IDs, enabling regenerate after a send failure.
      window.setTimeout(() => {
        void get().fetchMessages(conversationId);
      }, 120);
    }
  },

  sendAgentMessage: async (content, attachments = []) => {
    const conversationId = get().activeConversationId;
    if (!conversationId) { throw new Error("No active conversation"); }

    const conversation = get().conversations.find((c) => c.id === conversationId);
    if (!conversation) { throw new Error("Conversation not found"); }

    // Guard: prevent duplicate sends while a stream is already active for this conversation
    if (isConvStreaming(useStreamStore.getState().activeStreams, conversationId)) {
      console.warn("[sendAgentMessage] Ignoring duplicate send — stream already active for", conversationId);
      return;
    }

    const providerId = conversation.provider_id;
    const model_id = conversation.model_id;

    // Optimistic user message
    const optimisticUserMsg: Message = {
      id: `temp-user-${Date.now()}`,
      conversation_id: conversationId,
      role: "user",
      content,
      provider_id: null,
      model_id: null,
      token_count: null,
      attachments: attachments.map((a) => ({
        id: `temp-att-${Date.now()}`,
        file_name: a.file_name,
        file_type: a.file_type,
        file_path: "",
        file_size: a.file_size,
        data: a.data,
      })),
      thinking: null,
      tool_calls_json: null,
      tool_call_id: null,
      created_at: Date.now(),
      parent_message_id: null,
      version_index: 0,
      is_active: true,
      status: "complete",
    };

    // Placeholder assistant message
    let currentMsgId = `temp-agent-${Date.now()}`;
    const placeholderAssistant: Message = {
      id: currentMsgId,
      conversation_id: conversationId,
      role: "assistant",
      content: "",
      provider_id: providerId,
      model_id: model_id,
      token_count: null,
      attachments: [],
      thinking: null,
      tool_calls_json: null,
      tool_call_id: null,
      created_at: Date.now(),
      parent_message_id: optimisticUserMsg.id,
      version_index: 0,
      is_active: true,
      status: "partial",
    };

    set((s) => ({
      messages: [...s.messages, optimisticUserMsg, placeholderAssistant],
    }));
    useStreamStore.setState((s) => ({
      ...startConversationStream(s.activeStreams, conversationId, currentMsgId),
      streamingStartTimestamps: { ...s.streamingStartTimestamps, [conversationId]: Date.now() },
    }));

    let unlistenDone: UnlistenFn | null = null;
    let unlistenError: UnlistenFn | null = null;
    let unlistenStreamText: UnlistenFn | null = null;
    let unlistenStreamThinking: UnlistenFn | null = null;
    let unlistenMessageId: UnlistenFn | null = null;
    let unlistenWorkflowComplete: UnlistenFn | null = null;

    // ── Agent stream buffering (same pattern as Q&A _pendingUiChunk) ──
    let _agentPendingText = "";
    let _agentPendingThinking = "";
    let _agentFlushTimer: ReturnType<typeof setTimeout> | null = null;

    const flushAgentStreamChunks = () => {
      if (_agentFlushTimer !== null) {
        clearTimeout(_agentFlushTimer);
        _agentFlushTimer = null;
      }
      const textChunk = _agentPendingText;
      const thinkingChunk = _agentPendingThinking;
      _agentPendingText = "";
      _agentPendingThinking = "";
      if (!textChunk && !thinkingChunk) { return; }

      set((s) => {
        const wasThinking = useStreamStore.getState().thinkingActiveMessageIds.has(currentMsgId);
        let nextThinkingIds = useStreamStore.getState().thinkingActiveMessageIds;

        const updatedMessages = s.messages.map((m) => {
          if (m.id !== currentMsgId) { return m; }

          let content = m.content || "";
          let thinking = m.thinking || "";

          // 1. Process buffered thinking chunks first
          if (thinkingChunk) {
            if (!wasThinking) {
              content += '<think data-axagent="1">\n';
            }
            content += thinkingChunk;
            thinking += thinkingChunk;
            nextThinkingIds = new Set([...nextThinkingIds, currentMsgId]);
          }

          // 2. Process buffered text chunks (closes thinking block if needed)
          if (textChunk) {
            const isCurrentlyThinking = thinkingChunk ? true : wasThinking;
            if (isCurrentlyThinking) {
              content += "\n</think>\n\n";
              const n = new Set(nextThinkingIds);
              n.delete(currentMsgId);
              nextThinkingIds = n;
            }
            content += textChunk;
          }

          return { ...m, content, thinking };
        });

        useStreamStore.setState({ thinkingActiveMessageIds: nextThinkingIds });
        return {
          messages: updatedMessages,
        };
      });
    };

    const scheduleAgentFlush = () => {
      if (_agentFlushTimer === null) {
        _agentFlushTimer = setTimeout(flushAgentStreamChunks, STREAM_UI_FLUSH_INTERVAL_MS);
      }
    };

    const handleWorkflowEvent = (event: WorkflowEvent) => {
      const text = formatWorkflowEventAsText(event);
      if (text) {
        _agentPendingText += text;
        scheduleAgentFlush();
      }
    };

    const formatWorkflowEventAsText = (event: WorkflowEvent): string => {
      switch (event.type) {
        case "workflow_start":
          return `\n[Workflow Started: ${event.workflowId}]\n`;
        case "workflow_step_start":
          return `\n[Step Start] ${event.agentRole}: ${event.stepGoal}\n`;
        case "workflow_step_complete":
          return `[Step Complete] ${event.stepGoal}: ${event.result}\n`;
        case "workflow_step_error":
          return `[Step Error] ${event.stepId}: ${event.error}\n`;
        default:
          return "";
      }
    };

    const clearAgentStreamBuffer = () => {
      if (_agentFlushTimer !== null) {
        clearTimeout(_agentFlushTimer);
        _agentFlushTimer = null;
      }
      _agentPendingText = "";
      _agentPendingThinking = "";
    };

    const cleanup = () => {
      clearAgentStreamBuffer();
      unlistenStreamText?.();
      unlistenStreamThinking?.();
      unlistenDone?.();
      unlistenError?.();
      unlistenMessageId?.();
      unlistenWorkflowComplete?.();
      unlistenStreamText = null;
      unlistenStreamThinking = null;
      unlistenDone = null;
      unlistenError = null;
      unlistenMessageId = null;
      unlistenWorkflowComplete = null;
    };

    try {
      const eventPromise = new Promise<void>((resolve, reject) => {
        // Listen for the real assistant message ID from the backend
        // This replaces the temp ID so tool call events can be matched
        listen<{ conversationId: string; assistantMessageId: string }>("agent-message-id", (event) => {
          if (event.payload.conversationId !== conversationId) { return; }
          // Flush pending buffer before switching IDs
          flushAgentStreamChunks();
          const realId = event.payload.assistantMessageId;
          const oldId = currentMsgId;
          currentMsgId = realId;
          useStreamStore.setState((s) => ({
            ...startConversationStream(s.activeStreams, conversationId, realId),
            streamingMessageId: realId,
          }));
          set((s) => ({
            messages: s.messages.map((m) => m.id === oldId ? { ...m, id: realId } : m),
          }));
        }).then((fn) => {
          unlistenMessageId = fn;
        });

        // Listen for incremental text chunks — buffer and flush periodically
        listen<AgentStreamTextEvent | WorkflowEvent>("agent-stream-text", (event) => {
          if (event.payload.conversationId !== conversationId) { return; }

          // Check if this is a workflow event
          if ("type" in event.payload) {
            handleWorkflowEvent(event.payload as WorkflowEvent);
            return;
          }

          // Regular text event
          _agentPendingText += event.payload.text;
          scheduleAgentFlush();
        }).then((fn) => {
          unlistenStreamText = fn;
        });

        // Listen for incremental thinking chunks — buffer and flush periodically
        listen<AgentStreamThinkingEvent>("agent-stream-thinking", (event) => {
          if (event.payload.conversationId !== conversationId) { return; }
          _agentPendingThinking += event.payload.thinking;
          scheduleAgentFlush();
        }).then((fn) => {
          unlistenStreamThinking = fn;
        });

        // Listen for agent-done — correction overwrite with final content
        listen<AgentDoneEvent>("agent-done", (event) => {
          if (event.payload.conversationId !== conversationId) { return; }
          // Clear pending buffer (done event overwrites with final content)
          clearAgentStreamBuffer();
          // Skip if streaming was already cancelled (avoid stale fetchMessages re-render)
          const isStillStreaming = isConvStreaming(useStreamStore.getState().activeStreams, conversationId);
          if (!isStillStreaming) {
            cleanup();
            resolve();
            return;
          }

          useStreamStore.setState((s) => ({
            ...stopConversationStream(s.activeStreams, conversationId),
            streamingStartTimestamps: (() => {
              const t = { ...s.streamingStartTimestamps };
              delete t[conversationId];
              return t;
            })(),
            thinkingActiveMessageIds: (() => {
              const next = new Set(s.thinkingActiveMessageIds);
              next.delete(currentMsgId);
              return next;
            })(),
          }));
          set((s) => ({
            messages: s.messages.map((m) => {
              if (m.id === currentMsgId) {
                // Reconstruct content with thinking wrapped in <think> tags,
                // matching the format used during streaming (flushAgentStreamChunks).
                let finalContent = "";
                const thinkingText = event.payload.thinking;
                if (thinkingText) {
                  finalContent = `<think data-axagent="1">\n${thinkingText}\n</think>\n\n`;
                }
                finalContent += event.payload.text;

                return {
                  ...m,
                  id: event.payload.assistantMessageId || m.id,
                  content: finalContent,
                  thinking: thinkingText || m.thinking,
                  status: "complete" as const,
                  prompt_tokens: event.payload.usage?.input_tokens ?? null,
                  completion_tokens: event.payload.usage?.output_tokens ?? null,
                  blocks: event.payload.blocks ?? m.blocks,
                };
              }
              return m;
            }),
          }));

          cleanup();
          // Fetch messages to fully sync with backend (real user message ID, etc.)
          get().fetchMessages(conversationId);
          resolve();
        }).then((fn) => {
          unlistenDone = fn;
        });

        // Listen for workflow-complete
        listen<WorkflowCompleteEvent>("workflow-complete", (event) => {
          if (event.payload.conversationId !== conversationId) { return; }
          const text = event.payload.success
            ? `\n[Workflow Complete: ${event.payload.workflowId}]\n`
            : `\n[Workflow Failed: ${event.payload.workflowId}]\n`;
          _agentPendingText += text;
          scheduleAgentFlush();
        }).then((fn) => {
          unlistenWorkflowComplete = fn;
        });

        // Listen for agent-error
        listen<AgentErrorEvent>("agent-error", (event) => {
          if (event.payload.conversationId !== conversationId) { return; }
          // Clear pending buffer (error event overwrites content)
          clearAgentStreamBuffer();
          // Skip if streaming was already cancelled
          const isStillStreaming = isConvStreaming(useStreamStore.getState().activeStreams, conversationId);
          if (!isStillStreaming) {
            cleanup();
            resolve();
            return;
          }

          useStreamStore.setState((s) => ({
            ...stopConversationStream(s.activeStreams, conversationId),
            streamingStartTimestamps: (() => {
              const t = { ...s.streamingStartTimestamps };
              delete t[conversationId];
              return t;
            })(),
            thinkingActiveMessageIds: (() => {
              const next = new Set(s.thinkingActiveMessageIds);
              next.delete(currentMsgId);
              return next;
            })(),
          }));
          set((s) => ({
            messages: s.messages.map((m) => {
              if (m.id === currentMsgId) {
                return {
                  ...m,
                  content: event.payload.message,
                  status: "error" as const,
                };
              }
              return m;
            }),
          }));

          // Sync messages from DB so temp- prefixed user messages get replaced
          // with real backend IDs, enabling regenerate after an agent error.
          get().fetchMessages(conversationId);
          cleanup();
          reject(new Error(event.payload.message));
        }).then((fn) => {
          unlistenError = fn;
        });
      });

      // Invoke the backend command (this creates the real user message in DB)
      // agent_query can run for a very long time (10+ minutes for complex tasks).
      // We must NOT use the default 5-minute invoke timeout — the backend continues
      // running and we rely on agent-done/agent-error events for completion.
      // Setting timeoutMs=0 disables the invoke-level timeout entirely.
      await invoke("agent_query", {
        request: {
          conversationId,
          input: content,
          providerId,
          model_id,
        },
      }, 0);

      // Wait for agent-done or agent-error event
      await eventPromise;
    } catch (e) {
      // Safeguard: ensure listeners are always cleaned up, even if cleanup() itself throws
      try {
        cleanup();
      } catch (_) { /* ignore cleanup errors */ }
      const errMsg = String(e);
      console.error("[sendAgentMessage] error:", errMsg);

      // If streaming is still true, the error came from invoke itself (not an event)
      if (isConvStreaming(useStreamStore.getState().activeStreams, conversationId)) {
        useStreamStore.setState((s) => ({
          ...stopConversationStream(s.activeStreams, conversationId),
          streamingStartTimestamps: (() => {
            const t = { ...s.streamingStartTimestamps };
            delete t[conversationId];
            return t;
          })(),
        }));
        set((s) => ({
          messages: s.messages.map((m) =>
            m.id === currentMsgId
              ? { ...m, content: errMsg, status: "error" as const }
              : m
          ),
        }));
      }
      // Sync messages from DB so temp- prefixed user messages get replaced
      // with real backend IDs, enabling regenerate after an agent send failure.
      window.setTimeout(() => {
        void get().fetchMessages(conversationId);
      }, 120);
    }
  },

  regenerateMessage: async (targetMessageId?: string) => {
    const conversationId = get().activeConversationId;
    if (!conversationId) { throw new Error("No active conversation"); }

    // Guard: prevent duplicate sends while a stream is already active for this conversation
    if (isConvStreaming(useStreamStore.getState().activeStreams, conversationId)) {
      console.warn("[regenerateMessage] Ignoring duplicate send — stream already active for", conversationId);
      return;
    }

    const msgs = get().messages;
    // Find the user message (either specific or last one)
    let userMsg: Message | undefined;
    if (targetMessageId) {
      // Find the AI message, then its parent user message
      const aiMsg = msgs.find(m => m.id === targetMessageId);
      if (aiMsg?.parent_message_id) {
        userMsg = msgs.find(m => m.id === aiMsg.parent_message_id);
      }
    }
    if (!userMsg) {
      for (let i = msgs.length - 1; i >= 0; i--) {
        if (msgs[i].role === "user") {
          userMsg = msgs[i];
          break;
        }
      }
    }
    if (!userMsg) { throw new Error("No user message found"); }

    // Guard: reject temp IDs that haven't been persisted to the backend yet
    if (userMsg.id.startsWith("temp-")) {
      throw new Error("Message is still being sent. Please wait and try again.");
    }

    // Create placeholder for new version, preserving original created_at for position
    const tempAssistantId = `temp-assistant-${Date.now()}`;
    const parentId = userMsg.id;

    // Find the original active AI message to preserve its created_at
    const originalAiMsg = msgs.find(m => m.parent_message_id === parentId && m.is_active);
    const placeholderAssistant: Message = {
      id: tempAssistantId,
      conversation_id: conversationId,
      role: "assistant",
      content: "",
      provider_id: originalAiMsg?.provider_id ?? null,
      model_id: originalAiMsg?.model_id ?? null,
      token_count: null,
      attachments: [],
      thinking: null,
      tool_calls_json: null,
      tool_call_id: null,
      created_at: originalAiMsg?.created_at ?? Date.now(),
      parent_message_id: userMsg.id,
      version_index: 0,
      is_active: true,
      status: "partial",
    };

    // Replace the active AI message in-place with placeholder (preserve position)
    set((s) => {
      let inserted = false;
      const updated: Message[] = [];
      for (const m of s.messages) {
        if (m.parent_message_id === parentId && m.is_active) {
          updated.push({ ...m, is_active: false });
          if (!inserted) {
            updated.push(placeholderAssistant);
            inserted = true;
          }
        } else {
          updated.push(m);
        }
      }
      if (!inserted) {
        updated.push(placeholderAssistant);
      }
      return {
        messages: updated,
      };
    });
    useStreamStore.setState((s) => ({
      ...startConversationStream(s.activeStreams, conversationId, tempAssistantId),
      streamingStartTimestamps: { ...s.streamingStartTimestamps, [conversationId]: Date.now() },
      thinkingActiveMessageIds: new Set<string>(),
    }));
    setPendingUiChunk(null);
    if (_streamUiFlushTimer !== null) {
      clearTimeout(_streamUiFlushTimer);
      setStreamUiFlushTimer(null);
    }

    try {
      const rMcpIds = usePreferenceStore.getState().enabledMcpServerIds;
      const rThinkingBudget = getEffectiveThinkingBudget(conversationId);
      const rKbIds = usePreferenceStore.getState().enabledKnowledgeBaseIds;
      const rMemIds = usePreferenceStore.getState().enabledMemoryNamespaceIds;
      await invoke("regenerate_message", {
        conversationId,
        userMessageId: userMsg.id,
        enabledMcpServerIds: rMcpIds.length > 0 ? rMcpIds : undefined,
        thinkingBudget: rThinkingBudget,
        enabledKnowledgeBaseIds: rKbIds.length > 0 ? rKbIds : undefined,
        enabledMemoryNamespaceIds: rMemIds.length > 0 ? rMemIds : undefined,
      });

      // In browser mode, simulate brief loading then fetch the mock AI response
      if (!isTauri()) {
        await new Promise((r) => setTimeout(r, 600));
        useStreamStore.setState((s) => ({
          ...stopConversationStream(s.activeStreams, conversationId),
          streamingStartTimestamps: (() => {
            const t = { ...s.streamingStartTimestamps };
            delete t[conversationId];
            return t;
          })(),
          thinkingActiveMessageIds: new Set<string>(),
        }));
        get().fetchMessages(conversationId);
      }
    } catch (e) {
      console.error("[regenerateMessage] error:", e);
      const errMsg = String(e);
      const currentStreamingMessageId = getStreamingMessageId(useStreamStore.getState().activeStreams, conversationId);
      useStreamStore.setState((s) => ({
        ...stopConversationStream(s.activeStreams, conversationId),
        streamingStartTimestamps: (() => {
          const t = { ...s.streamingStartTimestamps };
          delete t[conversationId];
          return t;
        })(),
        thinkingActiveMessageIds: new Set<string>(),
      }));
      set((s) => ({
        messages: currentStreamingMessageId
          ? s.messages.map(m =>
            m.id === currentStreamingMessageId
              ? { ...m, content: errMsg, status: "error" as const }
              : m
          )
          : s.messages,
      }));
    }
  },

  regenerateWithModel: async (targetMessageId: string, providerId: string, model_id: string) => {
    const conversationId = get().activeConversationId;
    if (!conversationId) { throw new Error("No active conversation"); }

    const msgs = get().messages;
    // Find the AI message, then its parent user message
    const aiMsg = msgs.find(m => m.id === targetMessageId);
    if (!aiMsg?.parent_message_id) { throw new Error("Cannot find parent user message"); }
    const userMsg = msgs.find(m => m.id === aiMsg.parent_message_id);
    if (!userMsg) { throw new Error("User message not found"); }

    const parentId = userMsg.id;
    const originalAiMsg = msgs.find(m => m.parent_message_id === parentId && m.is_active);

    // Create placeholder with the target model info
    const tempAssistantId = `temp-assistant-${Date.now()}`;
    const placeholderAssistant: Message = {
      id: tempAssistantId,
      conversation_id: conversationId,
      role: "assistant",
      content: "",
      provider_id: providerId,
      model_id: model_id,
      token_count: null,
      attachments: [],
      thinking: null,
      tool_calls_json: null,
      tool_call_id: null,
      created_at: originalAiMsg?.created_at ?? Date.now(),
      parent_message_id: userMsg.id,
      version_index: 0,
      is_active: true,
      status: "partial",
    };

    // Replace the active AI message in-place with placeholder
    set((s) => {
      let inserted = false;
      const updated: Message[] = [];
      for (const m of s.messages) {
        if (m.parent_message_id === parentId && m.is_active) {
          updated.push({ ...m, is_active: false });
          if (!inserted) {
            updated.push(placeholderAssistant);
            inserted = true;
          }
        } else {
          updated.push(m);
        }
      }
      if (!inserted) {
        updated.push(placeholderAssistant);
      }
      return {
        messages: updated,
      };
    });
    useStreamStore.setState((s) => ({
      ...startConversationStream(s.activeStreams, conversationId, tempAssistantId),
      streamingStartTimestamps: { ...s.streamingStartTimestamps, [conversationId]: Date.now() },
      thinkingActiveMessageIds: new Set<string>(),
    }));
    setPendingUiChunk(null);
    if (_streamUiFlushTimer !== null) {
      clearTimeout(_streamUiFlushTimer);
      setStreamUiFlushTimer(null);
    }

    try {
      const rMcpIds = usePreferenceStore.getState().enabledMcpServerIds;
      const rThinkingBudget = getEffectiveThinkingBudget(conversationId);
      const rKbIds = usePreferenceStore.getState().enabledKnowledgeBaseIds;
      const rMemIds = usePreferenceStore.getState().enabledMemoryNamespaceIds;
      await invoke("regenerate_with_model", {
        conversationId,
        userMessageId: userMsg.id,
        targetProviderId: providerId,
        targetModelId: model_id,
        enabledMcpServerIds: rMcpIds.length > 0 ? rMcpIds : undefined,
        thinkingBudget: rThinkingBudget,
        enabledKnowledgeBaseIds: rKbIds.length > 0 ? rKbIds : undefined,
        enabledMemoryNamespaceIds: rMemIds.length > 0 ? rMemIds : undefined,
      });

      if (!isTauri()) {
        await new Promise((r) => setTimeout(r, 600));
        useStreamStore.setState((s) => ({
          ...stopConversationStream(s.activeStreams, conversationId),
          streamingStartTimestamps: (() => {
            const t = { ...s.streamingStartTimestamps };
            delete t[conversationId];
            return t;
          })(),
          thinkingActiveMessageIds: new Set<string>(),
        }));
        get().fetchMessages(conversationId);
      }
    } catch (e) {
      console.error("[regenerateWithModel] error:", e);
      const errMsg = String(e);
      const currentStreamingMessageId = getStreamingMessageId(useStreamStore.getState().activeStreams, conversationId);
      useStreamStore.setState((s) => ({
        ...stopConversationStream(s.activeStreams, conversationId),
        streamingStartTimestamps: (() => {
          const t = { ...s.streamingStartTimestamps };
          delete t[conversationId];
          return t;
        })(),
        thinkingActiveMessageIds: new Set<string>(),
      }));
      set((s) => ({
        messages: currentStreamingMessageId
          ? s.messages.map(m =>
            m.id === currentStreamingMessageId
              ? { ...m, content: errMsg, status: "error" as const }
              : m
          )
          : s.messages,
      }));
    }
  },

  sendMultiModelMessage: async (content, companionModels, attachments = [], searchProviderId = null) => {
    const conversationId = get().activeConversationId;
    if (!conversationId || companionModels.length === 0) { return; }

    // Guard: prevent duplicate sends while a stream is already active for this conversation
    if (isConvStreaming(useStreamStore.getState().activeStreams, conversationId)) {
      console.warn("[sendMultiModelMessage] Ignoring duplicate send — stream already active for", conversationId);
      return;
    }

    // Save original conversation model to restore later
    const conv = get().conversations.find((c) => c.id === conversationId);
    const originalProviderId = conv?.provider_id;
    const originalModelId = conv?.model_id;

    // Track ALL models (first + companions) in a unified counter
    setIsMultiModelActive(true);
    setMultiModelTotalRemaining(companionModels.length);
    setMultiModelFirstModelId(companionModels[0].model_id);
    set({ pendingCompanionModels: [...companionModels] });

    // Switch to the first selected model and send
    const firstModel = companionModels[0];
    try {
      await get().updateConversation(conversationId, {
        provider_id: firstModel.providerId,
        model_id: firstModel.model_id,
      });
    } catch (e) {
      console.error("[sendMultiModelMessage] failed to switch model:", e);
      resetMultiModelState();
      set({ pendingCompanionModels: [], multiModelParentId: null, multiModelDoneMessageIds: [] });
      return;
    }

    // sendMessage returns after invoke (message created in DB), stream continues in background
    await get().sendMessage(content, attachments, searchProviderId);

    // Find the user message that was just created
    const msgs = get().messages;
    const lastUserMsg = [...msgs].reverse().find((m) => m.role === "user");
    if (!lastUserMsg) {
      resetMultiModelState();
      set({ pendingCompanionModels: [], multiModelParentId: null, multiModelDoneMessageIds: [] });
      if (originalProviderId && originalModelId) {
        void get().updateConversation(conversationId, { provider_id: originalProviderId, model_id: originalModelId });
      }
      return;
    }

    // Scope loading indicators to this message and set parent_message_id
    // on the streaming placeholder so ModelTags renders immediately
    set((s) => ({
      multiModelParentId: lastUserMsg.id,
      messages: s.messages.map((m) =>
        m.id === useStreamStore.getState().streamingMessageId && m.role === "assistant"
          ? { ...m, parent_message_id: lastUserMsg.id }
          : m
      ),
    }));

    // Create a unified promise for ALL models (first model stream already running)
    const allDone = new Promise<void>((resolve) => {
      // If first model already finished before we set up the promise, check immediately
      if (_multiModelTotalRemaining === 0) {
        resolve();
        return;
      }
      setMultiModelDoneResolve(resolve);
    });

    // Fire remaining companions in PARALLEL (concurrent with first model's stream)
    const remaining = companionModels.slice(1);
    if (remaining.length > 0) {
      setStreamBuffer(null);

      const mcpIds = usePreferenceStore.getState().enabledMcpServerIds;
      const thinkingBudget = getEffectiveThinkingBudget(conversationId);
      const kbIds = usePreferenceStore.getState().enabledKnowledgeBaseIds;
      const memIds = usePreferenceStore.getState().enabledMemoryNamespaceIds;

      const invocations = remaining.map((model) =>
        invoke("regenerate_with_model", {
          conversationId,
          userMessageId: lastUserMsg.id,
          targetProviderId: model.providerId,
          targetModelId: model.model_id,
          enabledMcpServerIds: mcpIds.length > 0 ? mcpIds : undefined,
          thinkingBudget,
          enabledKnowledgeBaseIds: kbIds.length > 0 ? kbIds : undefined,
          enabledMemoryNamespaceIds: memIds.length > 0 ? memIds : undefined,
          isCompanion: true,
        }).then(async () => {
          // Each invoke returns after message creation — immediately enrich the store
          // so ModelTags can render this companion as clickable right away.
          if (!_isMultiModelActive) { return; }
          try {
            const versions = await get().listMessageVersions(conversationId, lastUserMsg.id);
            if (versions.length > 0 && _isMultiModelActive) {
              set((s) => {
                const existingIds = new Set(s.messages.map((m) => m.id));
                const dbVersionMap = new Map(versions.map((v) => [v.id, v]));

                const currentStreamingMessageId = useStreamStore.getState().streamingMessageId;
                let resolvedFirstModelId: string | null = null;
                if (currentStreamingMessageId?.startsWith("temp-") && _multiModelFirstModelId) {
                  const firstDbVersion = versions.find(
                    (v) => v.model_id === _multiModelFirstModelId && !existingIds.has(v.id),
                  );
                  if (firstDbVersion) {
                    resolvedFirstModelId = firstDbVersion.id;
                    existingIds.delete(currentStreamingMessageId);
                    existingIds.add(firstDbVersion.id);
                    useStreamStore.setState({ streamingMessageId: firstDbVersion.id });
                  }
                }

                const newVersions = versions
                  .filter((v) => !existingIds.has(v.id))
                  .map((v) => ({ ...v, is_active: false as const }));
                let enriched = false;
                const updatedMessages = s.messages.map((m) => {
                  if (resolvedFirstModelId && m.id === currentStreamingMessageId) {
                    const dbVersion = dbVersionMap.get(resolvedFirstModelId);
                    enriched = true;
                    return {
                      ...m,
                      id: resolvedFirstModelId,
                      model_id: dbVersion?.model_id ?? m.model_id,
                      provider_id: dbVersion?.provider_id ?? m.provider_id,
                    };
                  }
                  const dbVersion = dbVersionMap.get(m.id);
                  if (dbVersion && (!m.model_id || !m.provider_id)) {
                    enriched = true;
                    return { ...m, model_id: dbVersion.model_id, provider_id: dbVersion.provider_id };
                  }
                  return m;
                });
                if (newVersions.length === 0 && !enriched && resolvedFirstModelId === null) { return {}; }
                return { messages: [...updatedMessages, ...newVersions] };
              });
            }
          } catch (e) {
            console.warn("[sendMultiModelMessage] failed to enrich companion:", e);
          }
        }).catch((e) => {
          console.error(`[sendMultiModelMessage] companion ${model.model_id} invoke failed:`, e);
          // Invoke failed — no stream will start, so decrement counter here
          decrementMultiModelTotalRemaining();
          if (_multiModelTotalRemaining <= 0 && _multiModelDoneResolve) {
            const r = _multiModelDoneResolve;
            setMultiModelDoneResolve(null);
            useStreamStore.setState({
              streaming: false,
              streamingMessageId: null,
              streamingConversationId: null,
              thinkingActiveMessageIds: new Set<string>(),
            });
            r();
          }
        })
      );

      // Don't await invocations — they return after message creation, streams run in background
      // Enrichment now happens per-invocation (see .then() above).
      void Promise.allSettled(invocations);
    }

    // Wait for ALL streams to complete (first + companions)
    await allDone;

    // All done — cleanup
    setIsMultiModelActive(false);
    setMultiModelFirstModelId(null);
    set({ pendingCompanionModels: [], multiModelDoneMessageIds: [] });

    // Restore original conversation model
    if (originalProviderId && originalModelId) {
      try {
        await get().updateConversation(conversationId, {
          provider_id: originalProviderId,
          model_id: originalModelId,
        });
      } catch (e) {
        console.error("[sendMultiModelMessage] failed to restore model:", e);
      }
    }

    // Final fetch for consistency
    if (get().activeConversationId === conversationId) {
      const parentId = get().multiModelParentId;

      // Determine which version to show: if user manually selected a version, respect that choice
      const userSelectedMessageId = _userManuallySelectedVersion
        ? get().messages.find(
          (m) => m.parent_message_id === parentId && m.role === "assistant" && m.is_active,
        )?.id ?? null
        : null;

      if (parentId && !_userManuallySelectedVersion) {
        // No manual selection — switch to the first model's version
        const firstModelId = companionModels[0].model_id;
        let targetMessageId = _multiModelFirstMessageId;
        if (!targetMessageId) {
          const localMatch = get().messages.find(
            (m) => m.parent_message_id === parentId && m.role === "assistant" && m.model_id === firstModelId,
          );
          targetMessageId = localMatch?.id ?? null;
        }
        if (targetMessageId) {
          await invoke("switch_message_version", {
            conversation_id: conversationId,
            parent_message_id: parentId,
            message_id: targetMessageId,
          }).catch(() => {});
        }
      } else if (parentId && userSelectedMessageId) {
        // User manually selected a version — sync that to backend
        await invoke("switch_message_version", {
          conversation_id: conversationId,
          parent_message_id: parentId,
          message_id: userSelectedMessageId,
        }).catch(() => {});
      }

      await get().fetchMessages(conversationId);

      // Ensure only one version is shown locally
      if (parentId) {
        const refreshedMsgs = get().messages;

        // Determine which version to display
        let displayVersion: Message | null = null;
        if (_userManuallySelectedVersion && userSelectedMessageId) {
          displayVersion = refreshedMsgs.find((m) => m.id === userSelectedMessageId) ?? null;
        }
        if (!displayVersion) {
          const firstModelId = companionModels[0].model_id;
          displayVersion = _multiModelFirstMessageId
            ? refreshedMsgs.find((m) => m.id === _multiModelFirstMessageId) ?? null
            : null;
          if (!displayVersion) {
            displayVersion = refreshedMsgs.find(
              (m) => m.parent_message_id === parentId && m.role === "assistant" && m.model_id === firstModelId,
            ) ?? null;
          }
        }

        if (displayVersion) {
          set((s) => {
            let kept = false;
            return {
              messages: s.messages.reduce<Message[]>((acc, m) => {
                if (m.parent_message_id === parentId && m.role === "assistant") {
                  if (!kept) {
                    acc.push({ ...displayVersion, is_active: true });
                    kept = true;
                  }
                } else {
                  acc.push(m);
                }
                return acc;
              }, []),
            };
          });
        }
      }
    }

    setMultiModelFirstMessageId(null);
    setUserManuallySelectedVersion(false);
    set({ multiModelParentId: null, multiModelDoneMessageIds: [] });
  },

  deleteMessage: async (messageId) => {
    const conversationId = get().activeConversationId;
    if (!conversationId) { return; }
    // Client-only messages (temp IDs) — just remove locally
    if (messageId.startsWith("temp-")) {
      set((s) => ({
        messages: s.messages.filter((m) => m.id !== messageId),
      }));
      return;
    }
    try {
      await invoke("delete_message", { id: messageId });
      set((s) => ({
        messages: s.messages.filter((m) => m.id !== messageId),
      }));
    } catch (e) {
      set({ error: String(e) });
    }
  },

  fetchMessages: async (conversationId, preserveMessageIds = []) => {
    const requestSeq = _activeMessageLoadSeq;
    set({ loading: true });
    try {
      const page = await invoke<MessagePage>("list_messages_page", {
        conversationId,
        limit: MESSAGE_PAGE_SIZE,
        beforeMessageId: null,
      });
      if (requestSeq !== _activeMessageLoadSeq || get().activeConversationId !== conversationId) {
        return;
      }

      set((s) => {
        const messages = mergePreservedMessages(page.messages, preserveMessageIds, s.messages);
        return {
          messages,
          loading: false,
          loadingOlder: false,
          hasOlderMessages: page.has_older,
          totalActiveCount: page.total_active_count,
          oldestLoadedMessageId: messages[0]?.id ?? page.oldest_message_id,
          error: null,
        };
      });
    } catch (e) {
      if (requestSeq !== _activeMessageLoadSeq || get().activeConversationId !== conversationId) {
        return;
      }
      const errorMessage = String(e);
      if (errorMessage.includes("Not found: Conversation")) {
        console.warn("Conversation no longer exists on backend, clearing active selection:", conversationId);
        await get().fetchConversations().catch(() => {});
        const nextConversation = get().conversations[0] ?? get().archivedConversations[0] ?? null;
        if (nextConversation) {
          get().setActiveConversation(nextConversation.id);
          return;
        }
        set({
          activeConversationId: null,
          messages: [],
          loading: false,
          loadingOlder: false,
          hasOlderMessages: false,
          totalActiveCount: 0,
          oldestLoadedMessageId: null,
          error: errorMessage,
        });
        return;
      }
      set({ error: errorMessage, loading: false, loadingOlder: false });
    }
  },

  loadOlderMessages: async () => {
    const { activeConversationId, oldestLoadedMessageId, hasOlderMessages, loading, loadingOlder } = get();
    if (!activeConversationId || !oldestLoadedMessageId || !hasOlderMessages || loading || loadingOlder) {
      return;
    }

    const requestSeq = _activeMessageLoadSeq;
    set({ loadingOlder: true, error: null });
    try {
      const page = await invoke<MessagePage>("list_messages_page", {
        conversationId: activeConversationId,
        limit: MESSAGE_PAGE_SIZE,
        beforeMessageId: oldestLoadedMessageId,
      });
      if (requestSeq !== _activeMessageLoadSeq || get().activeConversationId !== activeConversationId) {
        return;
      }

      set((s) => ({
        messages: mergeOlderPages(page.messages, s.messages),
        loadingOlder: false,
        hasOlderMessages: page.has_older,
        totalActiveCount: page.total_active_count,
        oldestLoadedMessageId: page.oldest_message_id ?? s.oldestLoadedMessageId,
        error: null,
      }));
    } catch (e) {
      if (requestSeq !== _activeMessageLoadSeq || get().activeConversationId !== activeConversationId) {
        return;
      }
      set({ error: String(e), loadingOlder: false });
    }
  },

  searchConversations: async (query) => {
    try {
      return await invoke<ConversationSearchResult[]>("search_conversations", { query });
    } catch (e) {
      set({ error: String(e) });
      throw e;
    }
  },

  startStreamListening: async () => {
    // Increment generation and clean up previous listeners
    const gen = incrementListenerGen();
    if (_unlisten) {
      _unlisten();
      setUnlisten(null);
    }

    const chunkUnsub = await listen<ChatStreamEvent>("chat-stream-chunk", (event) => {
      if (_listenerGen !== gen) { return; // stale listener
       }
      if (!useStreamStore.getState().streaming) { return; // cancelled
       }
      const { conversation_id, message_id, chunk, model_id: evt_model_id, provider_id: evt_provider_id } =
        event.payload;

      if (chunk.done) {
        if (chunk.is_final === false) {
          // Append any remaining content in the done chunk (e.g. closing </think> tag)
          if (chunk.content) {
            appendStreamChunk(set, get, message_id, chunk.content, conversation_id, evt_model_id, evt_provider_id);
          }
          flushPendingStreamChunk(set, get);
          // Clear thinking state — this iteration is done
          if (useStreamStore.getState().thinkingActiveMessageIds.has(message_id)) {
            useStreamStore.setState((s) => {
              const next = new Set(s.thinkingActiveMessageIds);
              next.delete(message_id);
              return { thinkingActiveMessageIds: next };
            });
          }
          return;
        }

        // Unified multi-model handler: applies to ALL models (first + companions)
        if (_isMultiModelActive) {
          decrementMultiModelTotalRemaining();
          flushPendingStreamChunk(set, get);
          setStreamBuffer(null);

          // Clear streamingMessageId and mark completed message as 'complete'
          const currentStreamingMessageId = useStreamStore.getState().streamingMessageId;
          const currentThinkingIds = useStreamStore.getState().thinkingActiveMessageIds;
          const streamUpdates: { streamingMessageId?: string | null; thinkingActiveMessageIds?: Set<string> } = {};
          if (currentStreamingMessageId === message_id) {
            // This is the first model finishing — save its message_id for later version switching
            setMultiModelFirstMessageId(message_id);
            streamUpdates.streamingMessageId = null;
          }
          // Clear thinking state for this completed model
          if (currentThinkingIds.has(message_id)) {
            const nextThinking = new Set(currentThinkingIds);
            nextThinking.delete(message_id);
            streamUpdates.thinkingActiveMessageIds = nextThinking;
          }
          if (Object.keys(streamUpdates).length > 0) {
            useStreamStore.setState(streamUpdates);
          }
          set((s) => {
            const updated: Partial<ConversationState> = {};
            updated.conversations = s.conversations.map((c) =>
              c.id === conversation_id ? { ...c, message_count: c.message_count + 1 } : c
            );
            // Update completed message status to prevent "主动停止" tag
            updated.messages = s.messages.map((m) => m.id === message_id ? { ...m, status: "complete" } : m);
            // Track per-model completion for individual loading indicators
            updated.multiModelDoneMessageIds = [...s.multiModelDoneMessageIds, message_id];
            return updated;
          });

          if (_multiModelTotalRemaining <= 0) {
            // All models done
            useStreamStore.setState({
              streaming: false,
              streamingMessageId: null,
              streamingConversationId: null,
              thinkingActiveMessageIds: new Set<string>(),
            });
            if (_multiModelDoneResolve) {
              const resolve = _multiModelDoneResolve;
              setMultiModelDoneResolve(null);
              resolve();
            }
          }
          return;
        }

        const placeholderMessageId = useStreamStore.getState().streamingMessageId;
        flushPendingStreamChunk(set, get);
        const flushedMessageId = useStreamStore.getState().streamingMessageId ?? message_id;
        // Only preserve real backend IDs — temp placeholders (temp-assistant-*)
        // must NOT be preserved alongside the DB message, otherwise both the
        // unresolved placeholder and the DB row survive the merge (different
        // ids, same parent_message_id → duplicate bubble + React key collision).
        const preserveMessageIds = Array.from(
          new Set(
            [placeholderMessageId, flushedMessageId, message_id].filter(
              (value): value is string => typeof value === "string" && value.length > 0 && !value.startsWith("temp-"),
            ),
          ),
        );
        useStreamStore.setState({
          streaming: false,
          streamingMessageId: null,
          streamingConversationId: null,
          thinkingActiveMessageIds: new Set<string>(),
        });
        set((s) => ({
          conversations: s.conversations.map((c) =>
            c.id === conversation_id
              ? { ...c, message_count: c.message_count + 1 }
              : c
          ),
          // Update completed message status immediately to prevent "主动停止" tag flash
          messages: s.messages.map((m) =>
            preserveMessageIds.includes(m.id) ? { ...m, status: "complete" as const } : m
          ),
        }));
        if (get().activeConversationId === conversation_id) {
          // Active conversation — refresh messages then clear buffer
          setStreamBuffer(null);
          window.setTimeout(() => {
            void get().fetchMessages(
              conversation_id,
              preserveMessageIds,
            );
          }, 120);
        } else {
          // User is viewing a different conversation — keep buffer alive and
          // schedule a refresh so the completed message loads from DB when
          // the user switches back.
          addPendingConversationRefresh(conversation_id);
        }
        return;
      }

      if (
        chunk.thinking !== undefined && chunk.thinking !== null
        && !useStreamStore.getState().thinkingActiveMessageIds.has(message_id)
      ) {
        useStreamStore.setState((s) => ({
          thinkingActiveMessageIds: new Set([...s.thinkingActiveMessageIds, message_id]),
        }));
      }
      if (
        chunk.content && useStreamStore.getState().thinkingActiveMessageIds.has(message_id)
        && (chunk.thinking === undefined || chunk.thinking === null)
      ) {
        useStreamStore.setState((s) => {
          const next = new Set(s.thinkingActiveMessageIds);
          next.delete(message_id);
          return { thinkingActiveMessageIds: next };
        });
      }

      appendStreamChunk(set, get, message_id, chunk.content, conversation_id, evt_model_id, evt_provider_id);
    });

    const errorUnsub = await listen<ChatStreamErrorEvent>("chat-stream-error", (event) => {
      if (_listenerGen !== gen) { return; // stale listener
       }
      if (!useStreamStore.getState().streaming) { return; // cancelled
       }
      const { conversation_id, message_id, error: errMsg } = event.payload;

      flushPendingStreamChunk(set, get);
      setStreamBuffer(null); // Clear buffer on error

      // Multi-model: treat error as stream completion for this model
      if (_isMultiModelActive) {
        decrementMultiModelTotalRemaining();
        console.error(`[multi-model] stream error:`, errMsg);
        // Mark this model as done so ModelTags stops showing loading indicator
        set((s) => ({
          multiModelDoneMessageIds: [...s.multiModelDoneMessageIds, message_id],
          messages: s.messages.map((m) => m.id === message_id ? { ...m, status: "error" as const } : m),
        }));
        if (_multiModelTotalRemaining <= 0) {
          useStreamStore.setState({
            streaming: false,
            streamingMessageId: null,
            streamingConversationId: null,
            thinkingActiveMessageIds: new Set<string>(),
          });
          if (_multiModelDoneResolve) {
            const r = _multiModelDoneResolve;
            setMultiModelDoneResolve(null);
            r();
          }
        }
        return;
      }

      // Only show error if still on the same conversation
      if (get().activeConversationId !== conversation_id) {
        useStreamStore.setState({
          streaming: false,
          streamingMessageId: null,
          streamingConversationId: null,
          thinkingActiveMessageIds: new Set<string>(),
        });
        return;
      }

      // Update the streaming message to show error inline
      const currentStreamingMessageId = useStreamStore.getState().streamingMessageId;
      useStreamStore.setState({
        streaming: false,
        streamingMessageId: null,
        streamingConversationId: null,
        thinkingActiveMessageIds: new Set<string>(),
      });
      set((s) => ({
        messages: s.messages.map(m =>
          m.id === message_id || m.id === currentStreamingMessageId
            ? { ...m, content: errMsg, status: "error" as const }
            : m
        ),
      }));
      // Sync messages from DB so temp- prefixed user messages get replaced
      // with real backend IDs, enabling regenerate after a stream error.
      if (get().activeConversationId === conversation_id) {
        window.setTimeout(() => {
          void get().fetchMessages(conversation_id);
        }, 120);
      }
    });

    const titleUnsub = await listen<{ conversation_id: string; title: string }>(
      "conversation-title-updated",
      (event) => {
        if (_listenerGen !== gen) { return; }
        const { conversation_id, title } = event.payload;
        set((s) => ({
          conversations: s.conversations.map((c) => c.id === conversation_id ? { ...c, title } : c),
        }));
      },
    );

    const titleGenUnsub = await listen<{ conversation_id: string; generating: boolean; error: string | null }>(
      "conversation-title-generating",
      (event) => {
        if (_listenerGen !== gen) { return; }
        const { conversation_id, generating, error } = event.payload;
        set({ titleGeneratingConversationId: generating ? conversation_id : null });
        if (!generating && error) {
          console.error("[title-gen] AI title generation failed:", error);
          set({ error });
        }
      },
    );

    const ragUnsub = await listen<RagContextRetrievedEvent>("rag-context-retrieved", (event) => {
      if (_listenerGen !== gen) { return; }
      if (!useStreamStore.getState().streaming) { return; }
      const { conversation_id, sources } = event.payload;

      // Split sources by type and build separate tags
      const knowledgeSources = sources.filter(s => s.source_type === "knowledge");
      const memorySources = sources.filter(s => s.source_type === "memory");

      const kbSearching = buildKnowledgeTag("searching");
      const memSearching = buildMemoryTag("searching");
      const kbDone = knowledgeSources.length > 0 ? buildKnowledgeTag("done", knowledgeSources) : "";
      const memDone = memorySources.length > 0 ? buildMemoryTag("done", memorySources) : "";

      // Replace each searching tag with its done counterpart (or remove if empty)
      const replaceTag = (content: string, searching: string, done: string) => {
        if (content.includes(searching)) { return content.replace(searching, done); }
        if (done) { return done + content; }
        return content;
      };

      if (_streamBuffer && _streamBuffer.conversationId === conversation_id) {
        const buf = _streamBuffer;
        setStreamBuffer({
          ...buf,
          content: replaceTag(replaceTag(buf.content, kbSearching, kbDone), memSearching, memDone),
        });
      } else {
        setStreamPrefix(replaceTag(replaceTag(_streamPrefix, kbSearching, kbDone), memSearching, memDone));
      }

      // Update UI immediately
      if (get().activeConversationId === conversation_id) {
        const msgId = useStreamStore.getState().streamingMessageId;
        if (msgId) {
          set((s) => ({
            messages: s.messages.map(m => {
              if (m.id !== msgId) { return m; }
              let updated = m.content;
              updated = replaceTag(updated, kbSearching, kbDone);
              updated = replaceTag(updated, memSearching, memDone);
              return { ...m, content: updated };
            }),
          }));
        }
      }
    });

    // If generation changed while awaiting, this listener set is stale
    if (_listenerGen !== gen) {
      chunkUnsub();
      errorUnsub();
      titleUnsub();
      titleGenUnsub();
      ragUnsub();
      return;
    }

    setUnlisten(() => {
      chunkUnsub();
      errorUnsub();
      titleUnsub();
      titleGenUnsub();
      ragUnsub();
    });
  },

  stopStreamListening: () => {
    incrementListenerGen();
    if (_unlisten) {
      _unlisten();
      setUnlisten(null);
    }
  },

  cancelCurrentStream: () => {
    flushPendingStreamChunk(set, get);
    setPendingUiChunk(null);
    setStreamBuffer(null);
    clearPendingConversationRefresh();
    // Clean up multi-model state on cancel
    if (_isMultiModelActive) {
      resetMultiModelState();
      if (_multiModelDoneResolve) {
        const r = _multiModelDoneResolve;
        setMultiModelDoneResolve(null);
        r();
      }
      set({ pendingCompanionModels: [], multiModelParentId: null, multiModelDoneMessageIds: [] });
    }
    if (_streamUiFlushTimer !== null) {
      clearTimeout(_streamUiFlushTimer);
      setStreamUiFlushTimer(null);
    }
    // Tell the backend to cancel the stream — fire and forget
    const streamState = useStreamStore.getState();
    const conversationId = streamState.streamingConversationId ?? get().activeConversationId;
    if (conversationId && isTauri()) {
      invoke("cancel_stream", { conversationId }).catch(() => {});
      // Also cancel the agent if in agent mode
      const conv = get().conversations.find((c) => c.id === conversationId);
      if (conv?.mode === "agent") {
        invoke("agent_cancel", { request: { conversationId } }).catch(() => {});
      }
    }
    if (!conversationId) { return; }
    // Mark the current streaming message as partial
    const streamMsgId = getStreamingMessageId(streamState.activeStreams, conversationId);
    useStreamStore.setState((s) => ({
      ...stopConversationStream(s.activeStreams, conversationId),
      streamingStartTimestamps: (() => {
        const t = { ...s.streamingStartTimestamps };
        delete t[conversationId];
        return t;
      })(),
      thinkingActiveMessageIds: new Set<string>(),
    }));
    if (streamMsgId) {
      set((s) => ({
        messages: s.messages.map(m => m.id === streamMsgId ? { ...m, status: "partial" as const } : m),
      }));
    }
  },

  switchMessageVersion: async (conversationId, parentMessageId, messageId) => {
    try {
      if (_isMultiModelActive) {
        // During multi-model streaming, skip the backend call entirely to avoid:
        // 1. Race conditions with concurrent regenerate_with_model calls
        // 2. invoke delay causing stale content display
        // 3. Potential invoke failures during active streaming
        // Just swap is_active flags in-memory; backend will be synced during cleanup.
        setUserManuallySelectedVersion(true);
        set((s) => {
          const targetExists = s.messages.some(
            (m) => m.id === messageId && m.parent_message_id === parentMessageId && m.role === "assistant",
          );
          if (!targetExists) { return {}; // Target not in memory yet, no-op
           }
          return {
            messages: s.messages.map((m) => {
              if (m.parent_message_id !== parentMessageId || m.role !== "assistant") { return m; }
              return m.id === messageId
                ? { ...m, is_active: true }
                : { ...m, is_active: false };
            }),
          };
        });
        return;
      }

      await invoke("switch_message_version", { conversation_id: conversationId, parent_message_id: parentMessageId, message_id: messageId });

      // Normal path: fetch all versions from DB and keep them all in store
      // with correct is_active flags. This preserves multi-model detection
      // (multiModelResponseParents) which needs multiple versions visible.
      const versions = await get().listMessageVersions(conversationId, parentMessageId);
      if (versions.length > 0) {
        set((s) => {
          const versionMap = new Map(versions.map(v => [v.id, v]));
          const existingIds = new Set(
            s.messages
              .filter(m => m.parent_message_id === parentMessageId && m.role === "assistant")
              .map(m => m.id),
          );
          // Update existing versions in-place
          const updatedMessages = s.messages.map((m) => {
            if (m.parent_message_id !== parentMessageId || m.role !== "assistant") { return m; }
            const dbVersion = versionMap.get(m.id);
            if (dbVersion) {
              return { ...dbVersion, is_active: m.id === messageId };
            }
            return { ...m, is_active: m.id === messageId };
          });
          // Add any DB versions not already in store
          for (const v of versions) {
            if (!existingIds.has(v.id)) {
              updatedMessages.push({ ...v, is_active: v.id === messageId });
            }
          }
          return { messages: updatedMessages };
        });
      }
    } catch (e) {
      set({ error: String(e) });
      await get().fetchMessages(conversationId);
    }
  },

  listMessageVersions: async (conversationId, parentMessageId) => {
    try {
      return await invoke<Message[]>("list_message_versions", { conversationId, parentMessageId });
    } catch (e) {
      set({ error: String(e) });
      return [];
    }
  },

  updateMessageContent: async (messageId, content) => {
    try {
      const updated = await invoke<Message>("update_message_content", { id: messageId, content });
      set((s) => ({
        messages: s.messages.map((m) => (m.id === messageId ? { ...m, content: updated.content } : m)),
      }));
    } catch (e) {
      set({ error: String(e) });
      throw e;
    }
  },

  deleteMessageGroup: async (conversationId, userMessageId) => {
    // Client-only messages (temp IDs) — just remove locally
    if (userMessageId.startsWith("temp-")) {
      set((s) => ({
        messages: s.messages.filter(m => m.id !== userMessageId && m.parent_message_id !== userMessageId),
      }));
      return;
    }
    try {
      await invoke("delete_message_group", { conversation_id: conversationId, user_message_id: userMessageId });
      set((s) => ({
        messages: s.messages.filter(m => m.id !== userMessageId && m.parent_message_id !== userMessageId),
      }));
    } catch (e) {
      set({ error: String(e) });
    }
  },

  workspaceSnapshot: null,

  loadWorkspaceSnapshot: async (conversationId) => {
    try {
      const snapshot = await invoke<ConversationWorkspaceSnapshot>("get_workspace_snapshot", {
        conversationId: conversationId,
      });
      set({ workspaceSnapshot: snapshot });
      return snapshot;
    } catch {
      set({ workspaceSnapshot: null });
      return null;
    }
  },

  updateWorkspaceSnapshot: async (conversationId, snapshot) => {
    try {
      await invoke("update_workspace_snapshot", {
        conversation_id: conversationId,
        ...snapshot,
      });
      set((s) => ({
        workspaceSnapshot: s.workspaceSnapshot
          ? { ...s.workspaceSnapshot, ...snapshot }
          : null,
      }));
    } catch (e) {
      console.error("Failed to update workspace snapshot:", e);
    }
  },

  forkConversation: async (conversationId, fromMessageId?) => {
    try {
      const branch = await invoke<ConversationBranch>("fork_conversation", {
        conversationId: conversationId,
        messageId: fromMessageId,
      });
      const { fetchConversations } = get();
      await fetchConversations();
      return branch;
    } catch (e) {
      set({ error: String(e) });
      return null;
    }
  },

  compareResponses: async (leftMessageId, rightMessageId) => {
    try {
      return await invoke<CompareResponsesResult>("compare_branches", {
        branchA: leftMessageId,
        branchB: rightMessageId,
      });
    } catch {
      return null;
    }
  },
}));

// Register conversationStore reference so streamStore can call back
registerConversationStoreRef({
  getState: () => useConversationStore.getState(),
  setState: (partial) => useConversationStore.setState(partial),
});

// Auto-rebuild message index on every messages replacement to keep O(1) streaming fast.
// Uses shallow comparison (=== on the messages array reference), so it only fires
// when the entire messages array is replaced — not on every streaming chunk flush,
// since those create new arrays too. The rebuild is O(n) but n is typically <1000;
// at 50ms flush intervals this adds negligible overhead (<1ms for 1000 messages).
useConversationStore.subscribe(
  (state) => state.messages,
  (messages) => {
    rebuildMessageIndex(messages);
  },
);

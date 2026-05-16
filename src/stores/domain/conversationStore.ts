import { invoke } from "@/lib/invoke";
import { mergeOlderPages, mergePreservedMessages, MESSAGE_PAGE_SIZE } from "@/lib/messageUtils";
import { useProviderStore } from "@/stores/feature/providerStore";
import type {
  AttachmentInput,
  CompareResponsesResult,
  Conversation,
  ConversationBranch,
  ConversationSearchResult,
  ConversationWorkspaceSnapshot,
  Message,
  MessagePage,
  UpdateConversationInput,
} from "@/types";
import { create } from "zustand";
import { useAgentStore } from "../feature/agentStore";
import { useCategoryStore } from "../feature/categoryStore";
import { useExecutionStore } from "../feature/executionStore";
import { usePlanStore } from "../feature/planStore";
import { useTrajectoryStore } from "../feature/trajectoryStore";
import { tempId } from "./conversationHelpers";
import { createEventMethods } from "./conversationStoreEvents";
import { createSendMethods } from "./conversationStoreSend";
import { useMultiModelStore } from "./multiModelStore";
import {
  categoryTemplateUpdateFromCategory,
  conversationPreferenceStateFromConversation,
  conversationPreferenceUpdateFromState,
  getStagedPreferenceUpdate,
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
  deletePendingConversationRefresh,
  getStreamingMessageId,
  incrementActiveMessageLoadSeq,
  isConversationStreaming as isConvStreaming,
  rebuildMessageIndex,
  registerConversationStoreRef,
  setStreamBuffer,
  // Setter functions
  setUserManuallySelectedVersion,
  useStreamStore,
} from "./streamStore";

export interface ConversationState {
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
  /** Switch the active conversation to a different model by keyword (e.g. "opus", "sonnet", "haiku") */
  switchModel: (modelKeyword: string) => Promise<void>;
  fetchConversations: () => Promise<void>;
  setActiveConversation: (id: string | null) => void;
  createConversation: (
    title: string,
    model_id: string,
    providerId: string,
    options?: {
      categoryId?: string | null;
      scenario?: string | null;
      mode?: string;
      expert_role_id?: string;
      agent_profile_id?: string;
      workflow_template_id?: string;
      system_prompt?: string;
    },
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
  sendAgentMessage: (
    content: string,
    attachments?: AttachmentInput[],
    searchProviderId?: string | null,
  ) => Promise<void>;
  /** Send a message in plan mode: generates plan first, awaits approval, then executes */
  sendPlanMessage: (
    content: string,
    attachments?: AttachmentInput[],
    searchProviderId?: string | null,
  ) => Promise<void>;
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
  mcpMode: "auto" | "manual" | "disabled";
  enabledMcpServerIds: string[];
  enabledKnowledgeBaseIds: string[];
  activeMemoryNamespaceId: string | null;
  enabledWikiIds: string[];
  setSearchEnabled: (enabled: boolean) => void;
  setSearchProviderId: (id: string | null) => void;
  toggleMcpServer: (id: string) => void;
  setMcpMode: (mode: "auto" | "manual" | "disabled") => void;
  setThinkingBudget: (budget: number | null) => void;
  toggleKnowledgeBase: (id: string) => void;
  setActiveMemoryNamespace: (id: string | null) => void;
  toggleWiki: (id: string) => void;
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
  setPendingPromptText: (text) => {
    useMultiModelStore.getState().setPendingPromptText(text);
    set({ pendingPromptText: text });
  },
  searchEnabled: usePreferenceStore.getState().searchEnabled,
  searchProviderId: usePreferenceStore.getState().searchProviderId,
  thinkingBudget: usePreferenceStore.getState().thinkingBudget,
  mcpMode: usePreferenceStore.getState().mcpMode,
  enabledMcpServerIds: usePreferenceStore.getState().enabledMcpServerIds,
  enabledKnowledgeBaseIds: usePreferenceStore.getState().enabledKnowledgeBaseIds,
  activeMemoryNamespaceId: usePreferenceStore.getState().activeMemoryNamespaceId,
  enabledWikiIds: usePreferenceStore.getState().enabledWikiIds,
  setMcpMode: (mode: "auto" | "manual" | "disabled") => {
    usePreferenceStore.getState().setMcpMode(mode);
    set({ mcpMode: mode });
  },
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
  setActiveMemoryNamespace: (id) => {
    const current = get().activeMemoryNamespaceId;
    const nextId = current === id ? null : id;
    usePreferenceStore.getState().setActiveMemoryNamespace(id);
    set({ activeMemoryNamespaceId: nextId });
  },
  toggleWiki: (id) => {
    const current = get().enabledWikiIds;
    const next = current.includes(id) ? current.filter((s) => s !== id) : [...current, id];
    usePreferenceStore.getState().toggleWiki(id);
    set({ enabledWikiIds: next });
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
      await invoke("agent_backup_and_clear_sdk_context", { conversationId }).catch((e: unknown) => {
        console.warn("[IPC]", e);
      });
    } catch {
      // If backend command doesn't exist yet, add optimistic local message
      const localMsg: Message = {
        id: tempId("ctx-clear-"),
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
        await invoke("agent_restore_sdk_context_from_backup", { conversationId }).catch((e: unknown) => {
          console.warn("[IPC]", e);
        });
      }
    } catch (e) {
      set({ error: String(e) });
      throw e;
    }
  },

  clearAllMessages: async () => {
    const conversationId = get().activeConversationId;
    if (!conversationId) { return; }
    // Guard: cancel any active stream before clearing messages.
    // Otherwise the backend stream task would try to update a deleted
    // placeholder message in DB, producing errors and orphaned chunks.
    if (isConvStreaming(useStreamStore.getState().activeStreams, conversationId)) {
      useStreamStore.getState().cancelCurrentStream(conversationId);
    }
    try {
      await invoke("clear_conversation_messages", { conversationId });
      // Stale guard: don't wipe messages if user switched conversations
      if (get().activeConversationId !== conversationId) { return; }
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

  switchModel: async (modelKeyword: string) => {
    const conversationId = get().activeConversationId;
    const conversation = get().conversations.find((c) => c.id === conversationId);
    if (!conversationId || !conversation) { return; }

    try {
      const providers = useProviderStore.getState().providers;
      const keyword = modelKeyword.toLowerCase();

      // 优先精确匹配，其次同 provider 子串匹配，最后跨 provider 子串匹配
      let bestProviderId: string | null = null;
      let bestModelId: string | null = null;
      // 评分: 3=精确+同provider, 2=精确+跨provider, 1=子串+同provider, 0=子串+跨provider
      let bestScore = 0;

      for (const p of providers) {
        for (const m of p.models) {
          if (!m.enabled) { continue; }
          const modelLower = m.model_id.toLowerCase();
          const exact = modelLower === keyword;
          const contains = modelLower.includes(keyword);
          if (!exact && !contains) { continue; }
          const sameProvider = p.id === conversation.provider_id;
          const score = exact ? (sameProvider ? 3 : 2) : (sameProvider ? 1 : 0);
          if (score > bestScore) {
            bestScore = score;
            bestProviderId = p.id;
            bestModelId = m.model_id;
          }
        }
      }

      if (bestProviderId && bestModelId) {
        await get().updateConversation(conversationId, {
          provider_id: bestProviderId,
          model_id: bestModelId,
        });
      }
    } catch (e) {
      console.error("Failed to switch model:", e);
    }
  },

  fetchConversations: async () => {
    set({ loading: true });
    try {
      // 15s timeout — session list is a lightweight DB query, should be fast
      const conversations = await invoke<Conversation[]>("list_conversations", undefined, 15_000);
      set({ conversations, loading: false, error: null });
    } catch (e) {
      set({ error: String(e), loading: false });
    }
  },

  setActiveConversation: (id) => {
    if (id === get().activeConversationId && (!id || !_pendingConversationRefresh.has(id))) {
      return;
    }
    const prevId = get().activeConversationId;
    incrementActiveMessageLoadSeq();
    if (!id) {
      if (prevId === null) { return; }
      if (prevId) {
        if (isConvStreaming(useStreamStore.getState().activeStreams, prevId)) {
          useStreamStore.getState().cancelCurrentStream(prevId);
        }
        useAgentStore.getState().clearConversation(prevId);
        useExecutionStore.getState().clearConversation(prevId);
        usePlanStore.getState().clearActivePlan(prevId);
        useTrajectoryStore.getState().clearConversation(prevId);
      }
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

    if (prevId && prevId !== id) {
      // Cancel any active stream for the conversation being left
      if (isConvStreaming(useStreamStore.getState().activeStreams, prevId)) {
        useStreamStore.getState().cancelCurrentStream(prevId);
      }
      useAgentStore.getState().clearConversation(prevId);
      useExecutionStore.getState().clearConversation(prevId);
      usePlanStore.getState().clearActivePlan(prevId);
      useTrajectoryStore.getState().clearConversation(prevId);
    }

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
      mcpMode: prefState.mcpMode,
      enabledMcpServerIds: prefState.enabledMcpServerIds,
      enabledKnowledgeBaseIds: prefState.enabledKnowledgeBaseIds,
      activeMemoryNamespaceId: prefState.activeMemoryNamespaceId,
      enabledWikiIds: prefState.enabledWikiIds,
    });
    // Sync preference state from the conversation (direct setState to avoid triggering persistence)
    usePreferenceStore.setState(prefState);
    // 保留尚未持久化的 temp- 消息，防止被服务端返回的列表覆盖丢失
    const tempIds = get().messages.filter(m => m.id.startsWith("temp-")).map(m => m.id);
    get().fetchMessages(id, tempIds).then(() => {
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
        systemPrompt: options?.system_prompt ?? category?.system_prompt ?? undefined,
      });
      let conversation = createdConversation;
      try {
        conversation = await invoke<Conversation>("update_conversation", {
          id: createdConversation.id,
          input: {
            ...categoryTemplateUpdateFromCategory(category),
            ...conversationPreferenceUpdateFromState(usePreferenceStore.getState()),
            scenario: options?.scenario,
            expert_role_id: options?.expert_role_id,
            agent_profile_id: options?.agent_profile_id,
            workflow_template_id: options?.workflow_template_id,
            mode: options?.mode,
            ...getStagedPreferenceUpdate(),
          },
        }, 10_000);
      } catch (preferenceError) {
        // 非致命：对话已创建，偏好设置未应用，使用默认值
        console.warn("[createConversation] 偏好设置更新失败，使用默认值:", preferenceError);
      }
      // Clean up the previous active conversation's stores before switching.
      // createConversation bypassed setActiveConversation, which would normally
      // handle this cleanup. Without it, agent/execution/plan state from the
      // old conversation leaks into the new one.
      const prevId = get().activeConversationId;
      if (prevId && prevId !== conversation.id) {
        useAgentStore.getState().clearConversation(prevId);
        useExecutionStore.getState().clearConversation(prevId);
        usePlanStore.getState().clearActivePlan(prevId);
        useTrajectoryStore.getState().clearConversation(prevId);
      }
      set((s) => ({
        conversations: [conversation, ...s.conversations],
        activeConversationId: conversation.id,
        messages: [],
        loading: true,
        loadingOlder: false,
        hasOlderMessages: false,
        totalActiveCount: 0,
        oldestLoadedMessageId: null,
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
      // If the conversation is currently streaming, cancel it first to clean up stream state
      if (isConvStreaming(useStreamStore.getState().activeStreams, id)) {
        useStreamStore.getState().cancelCurrentStream(id);
      }
      await invoke("delete_conversation", { id });
      // Clean up other stores for this conversation
      useAgentStore.getState().clearConversation(id);
      useExecutionStore.getState().clearConversation(id);
      usePlanStore.getState().clearActivePlan(id);
      useTrajectoryStore.getState().clearConversation(id);
      // dreamStore is global, no per-conversation cleanup needed
      // Clean up stream buffer and pending refresh if they reference this conversation
      if (_streamBuffer?.conversationId === id) {
        setStreamBuffer(null);
      }
      deletePendingConversationRefresh(id);
      const state = get();
      // When deleting the active conversation, suppress the sidebar auto-select
      // so the ChatView shows the welcome screen instead of jumping to another
      // conversation. The flag is reset by ChatSidebar on next render.
      if (state.activeConversationId === id) {
        setSidebarAutoSelectSuppression();
      }
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
      // Clean up old conversation's stores before switching to branch
      const branchPrevId = get().activeConversationId;
      if (branchPrevId && branchPrevId !== newConv.id) {
        useAgentStore.getState().clearConversation(branchPrevId);
        useExecutionStore.getState().clearConversation(branchPrevId);
        usePlanStore.getState().clearActivePlan(branchPrevId);
        useTrajectoryStore.getState().clearConversation(branchPrevId);
      }
      set((s) => ({
        conversations: [newConv, ...s.conversations],
        activeConversationId: newConv.id,
        messages: [],
        loading: true,
        loadingOlder: false,
        hasOlderMessages: false,
        totalActiveCount: 0,
        oldestLoadedMessageId: null,
        error: null,
      }));
      // Load the branched messages
      const msgs = await invoke<Message[]>("list_messages", { conversationId: newConv.id });
      // Stale guard: if user switched away, discard messages to prevent cross-conversation pollution
      if (get().activeConversationId !== newConv.id) { return newConv; }
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

  toggleArchive: async (id, feedback?: string) => {
    try {
      // If the conversation is currently streaming, cancel it first
      if (isConvStreaming(useStreamStore.getState().activeStreams, id)) {
        useStreamStore.getState().cancelCurrentStream(id);
      }
      const conv = get().conversations.find((c) => c.id === id)
        ?? get().archivedConversations.find((c) => c.id === id);

      const isAlreadyArchived = conv?.is_archived ?? false;
      const isWorkflow = conv?.session_type === "workflow";

      const command = isWorkflow && !isAlreadyArchived
        ? "archive_workflow_session"
        : "toggle_archive_conversation";
      const params = isWorkflow && !isAlreadyArchived
        ? { conversationId: id, feedback }
        : { id };

      const updated = await invoke<Conversation>(command, params);
      // Clean up other stores when archiving
      useAgentStore.getState().clearConversation(id);
      useExecutionStore.getState().clearConversation(id);
      usePlanStore.getState().clearActivePlan(id);
      useTrajectoryStore.getState().clearConversation(id);
      if (updated.is_archived) {
        // When archiving the active conversation, suppress sidebar auto-select
        if (get().activeConversationId === id) {
          setSidebarAutoSelectSuppression();
        }
        set((s) => ({
          conversations: s.conversations.filter((c) => c.id !== id),
          archivedConversations: [updated, ...s.archivedConversations],
          activeConversationId: s.activeConversationId === id ? null : s.activeConversationId,
          messages: s.activeConversationId === id ? [] : s.messages,
          error: null,
        }));
      } else {
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
        knowledge_base_id: knowledgeBaseId,
      });
      // Archive succeeded — move from active list to archived list
      // When archiving the active conversation, suppress sidebar auto-select
      if (get().activeConversationId === id) {
        setSidebarAutoSelectSuppression();
      }
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
    // Cancel any active streams for the conversations being deleted
    for (const id of ids) {
      if (isConvStreaming(useStreamStore.getState().activeStreams, id)) {
        useStreamStore.getState().cancelCurrentStream(id);
      }
    }
    await invoke("batch_delete_conversations", { ids });
    // Clean up other stores for all deleted conversations
    for (const id of ids) {
      useAgentStore.getState().clearConversation(id);
      useExecutionStore.getState().clearConversation(id);
      usePlanStore.getState().clearActivePlan(id);
      useTrajectoryStore.getState().clearConversation(id);
    }
    set((s) => ({
      conversations: s.conversations.filter((c) => !ids.includes(c.id)),
      activeConversationId: ids.includes(s.activeConversationId ?? "") ? null : s.activeConversationId,
      messages: ids.includes(s.activeConversationId ?? "") ? [] : s.messages,
      error: null,
    }));
  },

  batchArchive: async (ids) => {
    // Cancel any active streams for the conversations being archived
    for (const id of ids) {
      if (isConvStreaming(useStreamStore.getState().activeStreams, id)) {
        useStreamStore.getState().cancelCurrentStream(id);
      }
    }
    // 并行归档所有对话（无依赖关系）
    const results = await Promise.allSettled(
      ids.map(async (id) => {
        const conv = get().conversations.find((c) => c.id === id);
        const command = conv?.session_type === "workflow"
          ? "archive_workflow_session"
          : "toggle_archive_conversation";
        const params = conv?.session_type === "workflow"
          ? { conversationId: id }
          : { id };
        return invoke<Conversation>(command, params);
      }),
    );
    const archived: Conversation[] = [];
    for (const r of results) {
      if (r.status === "fulfilled" && r.value.is_archived) {
        archived.push(r.value);
      }
    }
    // Clean up other stores for all archived conversations
    for (const id of ids) {
      useAgentStore.getState().clearConversation(id);
      useExecutionStore.getState().clearConversation(id);
      usePlanStore.getState().clearActivePlan(id);
      useTrajectoryStore.getState().clearConversation(id);
    }
    set((s) => ({
      conversations: s.conversations.filter((c) => !ids.includes(c.id)),
      archivedConversations: [...archived, ...s.archivedConversations],
      activeConversationId: ids.includes(s.activeConversationId ?? "") ? null : s.activeConversationId,
      messages: ids.includes(s.activeConversationId ?? "") ? [] : s.messages,
      error: null,
    }));
  },
  ...createSendMethods(set, get) as any,
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
    // If the message is currently streaming, cancel the stream first
    const currentStreamingMessageId = getStreamingMessageId(
      useStreamStore.getState().activeStreams,
      conversationId,
    );
    if (currentStreamingMessageId === messageId) {
      useStreamStore.getState().cancelCurrentStream(conversationId);
    }
    try {
      await invoke("delete_message", { id: messageId });
      // Stale guard: don't filter messages if user switched conversations
      if (get().activeConversationId !== conversationId) { return; }
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
        await get().fetchConversations().catch((e: unknown) => {
          console.warn("[IPC]", e);
        });
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
  ...createEventMethods(set, get) as any,
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

      await invoke("switch_message_version", {
        conversation_id: conversationId,
        parent_message_id: parentMessageId,
        message_id: messageId,
      });

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
      await invoke("delete_message_group", { conversationId: conversationId, userMessageId: userMessageId });
      // Stale guard: don't filter messages if user switched conversations
      if (get().activeConversationId !== conversationId) { return; }
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

// ─── Sidebar auto-select suppression ───
//
// When deleteConversation or toggleArchive removes the active conversation,
// ChatSidebar's useEffect would normally auto-select the next conversation.
// Setting this flag to true tells the sidebar to skip auto-select for one
// render cycle, keeping the ChatView on the welcome screen.
export let _suppressSidebarAutoSelect = false;

let _sideBarSuppressTimer: ReturnType<typeof setTimeout> | null = null;

/** Reset the sidebar auto-select suppression flag (called by ChatSidebar after consuming). */
export function resetSidebarAutoSelectSuppression() {
  _suppressSidebarAutoSelect = false;
  if (_sideBarSuppressTimer) {
    clearTimeout(_sideBarSuppressTimer);
    _sideBarSuppressTimer = null;
  }
}

// 设置 sidebar 自动选择抑制，带超时保护防止永久抑制
export function setSidebarAutoSelectSuppression() {
  setSidebarAutoSelectSuppression();
  if (_sideBarSuppressTimer) { clearTimeout(_sideBarSuppressTimer); }
  _sideBarSuppressTimer = setTimeout(() => {
    _suppressSidebarAutoSelect = false;
    _sideBarSuppressTimer = null;
    // 5s 安全超时，防止 ChatSidebar 未挂载导致永久抑制
  }, 5000);
}

// Auto-rebuild message index on every messages replacement to keep O(1) streaming fast.
// Subscribes to all state changes but only rebuilds when the messages array reference
// changes (Zustand shallow merge creates new references on every set).
// The rebuild is O(n) but n is typically <1000; at 50ms flush intervals this adds
// negligible overhead (<1ms for 1000 messages).
useConversationStore.subscribe((state, prev) => {
  if (state.messages !== prev.messages) {
    rebuildMessageIndex(state.messages);
  }
});

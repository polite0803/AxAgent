// SPDX-License-Identifier: AGPL-3.0-only

import { invoke, logIpcError } from "@/lib/invoke";
import { mergeOlderPages, mergePreservedMessages, MESSAGE_PAGE_SIZE } from "@/lib/messageUtils";
import { useProviderStore } from "@/stores/feature/providerStore";
import { useSettingsStore } from "@/stores/feature/settingsStore";
import type {
  AttachmentInput,
  CognitiveClarification,
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
import { useTabStore } from "../shared/tabStore";

// ── 惰性 PreferenceStore 访问器（打破 conversation ↔ preference 循环依赖）──
// preferenceStore 在自身初始化后主动注入引用；conversationStore 不 import preferenceStore。
// ConversationPreferenceState 来自 conversationPreferences.ts，无循环依赖风险。
interface PrefStoreState extends ConversationPreferenceState {
  setMcpMode: (mode: "auto" | "manual" | "disabled") => void;
  setSearchEnabled: (enabled: boolean) => void;
  setSearchProviderId: (id: string | null) => void;
  toggleMcpServer: (id: string) => void;
  setThinkingBudget: (budget: number | null) => void;
  toggleKnowledgeBase: (id: string) => void;
  setActiveMemoryNamespaceId: (id: string | null) => void;
  toggleWiki: (id: string) => void;
}
interface PrefStoreHandle {
  getState(): PrefStoreState;
  setState(state: unknown): void;
}

// 所有状态存放在 globalThis，避免两个问题：
// 1) HMR 重载 conversationStore 模块时模块级变量重置，但 globalThis 不会丢失
// 2) conversationStore ↔ preferenceStore 循环依赖导致 preferenceStore 在
//    conversationStore 模块体执行到 let/const/var 赋值前调用 _injectPreferenceStore，
//    此时模块级变量（包括 var，因为赋值不会 hoist）均不可用。
//    _injectPreferenceStore 直接用字符串字面量，不依赖任何模块级变量。
const _PREF_STORE_KEY = "__axagent_prefStore__";
const _PREF_SYNCED_KEY = "__axagent_prefSynced__";

function _g(): Record<string, unknown> {
  return globalThis as Record<string, unknown>;
}

function _getPrefStore(): PrefStoreHandle | null {
  return (_g()[_PREF_STORE_KEY] as PrefStoreHandle | null | undefined) ?? null;
}
function _getPrefSynced(): boolean {
  return (_g()[_PREF_SYNCED_KEY] as boolean | undefined) ?? false;
}

/**
 * 注入 preferenceStore 引用。在测试中也可直接调用以注入 mock。
 *
 * 注意：由于存在 conversationStore ↔ preferenceStore 的循环依赖，
 * preferenceStore 模块体可能在 conversationStore 模块体执行到
 * let/const 声明之前就调用本函数。因此本函数不得访问任何模块级变量，
 * 只能直接使用 globalThis 字面量和字符串字面量。
 */
export function _injectPreferenceStore(store: PrefStoreHandle): void {
  const g = globalThis as Record<string, unknown>;
  g["__axagent_prefStore__"] = store;
  g["__axagent_prefSynced__"] = false;
}

function syncPrefState(): void {
  if (_getPrefSynced()) { return; }
  const store = _getPrefStore();
  if (!store) { return; }
  _g()[_PREF_SYNCED_KEY] = true;
  (async () => {
    try {
      const prefState = store.getState();
      useConversationStore.setState({
        searchEnabled: prefState.searchEnabled,
        searchProviderId: prefState.searchProviderId,
        thinkingBudget: prefState.thinkingBudget,
        mcpMode: prefState.mcpMode,
        enabledMcpServerIds: prefState.enabledMcpServerIds,
        enabledKnowledgeBaseIds: prefState.enabledKnowledgeBaseIds,
        activeMemoryNamespaceId: prefState.activeMemoryNamespaceId,
        enabledWikiIds: prefState.enabledWikiIds,
      });
    } catch {
      /* module still initializing */
    }
  })();
}

function getPref(): PrefStoreHandle {
  const store = _getPrefStore();
  if (!store) {
    // 循环依赖导致 preferenceStore 顶层执行时调用本模块，此时还未注入；
    // HMR 重载场景下 globalThis 会保留之前的引用，不会走到这里。
    // 走到这里说明是首次模块加载竞态，抛错由调用方 try/catch 处理。
    throw new Error("preferenceStore_not_initialized");
  }
  if (!_getPrefSynced()) {
    syncPrefState();
  }
  return store;
}

// 单调递增计数器，与 Date.now() 组合防止同毫秒 ID 重复
let _idSeq = 0;
export function tempId(prefix: string): string {
  return `${prefix}${Date.now()}-${++_idSeq}`;
}
import {
  categoryTemplateUpdateFromCategory,
  type ConversationPreferenceState,
  conversationPreferenceStateFromConversation,
  conversationPreferenceUpdateFromState,
  getStagedPreferenceUpdate,
  mergeConversationCollections,
} from "./conversationPreferences";
import { createEventMethods } from "./conversationStoreEvents";
import { createSendMethods, type SendModeHint } from "./conversationStoreSend";
import { useMultiModelStore } from "./multiModelStore";
import {
  _activeMessageLoadSeq,
  _isMultiModelActive,
  _pendingConversationRefresh,
  deletePendingConversationRefresh,
  getSession,
  getStreamingMessageId,
  incrementActiveMessageLoadSeq,
  isConversationStreaming as isConvStreaming,
  rebuildMessageIndex,
  registerConversationStoreRef,
  removeSession,
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
  /** 抑制侧栏自动选择 (防止删除/归档后自动跳转) */
  sidebarAutoSelectSuppressed: boolean;
  /** 抑制自动选择的计时器清理函数 */
  sidebarSuppressTimer: ReturnType<typeof setTimeout> | null;
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
    modelId: string,
    providerId: string,
    options?: {
      categoryId?: string | null;
      scenario?: string | null;
      mode?: string;
      workStrategy?: string;
      agentProfileId?: string;
      workflowTemplateId?: string;
      systemPrompt?: string;
    },
  ) => Promise<Conversation>;
  updateConversation: (
    id: string,
    input: UpdateConversationInput,
  ) => Promise<void>;
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
  archiveToKnowledgeBase: (
    id: string,
    knowledgeBaseId: string,
  ) => Promise<void>;
  archivedConversations: Conversation[];
  fetchArchivedConversations: () => Promise<void>;
  batchDelete: (ids: string[]) => Promise<void>;
  batchArchive: (ids: string[]) => Promise<void>;
  sendMessage: (
    content: string,
    attachments?: AttachmentInput[],
    searchProviderId?: string | null,
    quotedMessageId?: string | null,
    modeHint?: SendModeHint,
    disabledTools?: string[],
    resumeClarify?: {
      capabilityId: string;
      userMessageId: string;
    },
  ) => Promise<void>;
  /** Clarify 二次执行：用户选中候选后携带 capabilityId 重新调用 cognitive_query */
  executeClarify: (capabilityId: string) => Promise<void>;
  /** 认知编排澄清候选（Clarify 分支）待用户选择；null 表示无待选 */
  pendingClarification: CognitiveClarification | null;
  setPendingClarification: (c: CognitiveClarification | null) => void;
  regenerateMessage: (targetMessageId?: string) => Promise<void>;
  regenerateWithModel: (
    targetMessageId: string,
    providerId: string,
    modelId: string,
  ) => Promise<void>;
  deleteMessage: (messageId: string) => Promise<void>;
  fetchMessages: (
    conversationId: string,
    preserveMessageIds?: string[],
  ) => Promise<void>;
  loadOlderMessages: () => Promise<void>;
  searchConversations: (query: string) => Promise<ConversationSearchResult[]>;
  startStreamListening: () => Promise<void>;
  switchMessageVersion: (
    conversationId: string,
    parentMessageId: string,
    messageId: string,
  ) => Promise<void>;
  listMessageVersions: (
    conversationId: string,
    parentMessageId: string,
  ) => Promise<Message[]>;
  updateMessageContent: (messageId: string, content: string) => Promise<void>;
  deleteMessageGroup: (
    conversationId: string,
    userMessageId: string,
  ) => Promise<void>;
  workspaceSnapshot: ConversationWorkspaceSnapshot | null;
  loadWorkspaceSnapshot: (
    conversationId: string,
  ) => Promise<ConversationWorkspaceSnapshot | null>;
  updateWorkspaceSnapshot: (
    conversationId: string,
    snapshot: Partial<ConversationWorkspaceSnapshot>,
  ) => Promise<void>;
  forkConversation: (
    conversationId: string,
    fromMessageId?: string,
  ) => Promise<ConversationBranch | null>;
  compareResponses: (
    leftMessageId: string,
    rightMessageId: string,
  ) => Promise<CompareResponsesResult | null>;
  /** Conversation ID currently generating an AI title (null if none) */
  titleGeneratingConversationId: string | null;
  /** Regenerate the title of a conversation using AI */
  regenerateTitle: (conversationId: string) => Promise<void>;
  /** Companion models pending or currently streaming (for multi-model simultaneous response) */
  pendingCompanionModels: Array<{ providerId: string; modelId: string }>;
  /** User message ID of the current multi-model request (for scoping UI indicators) */
  multiModelParentId: string | null;
  /** Message IDs of models that have completed their streams (for per-model loading indicators) */
  multiModelDoneMessageIds: string[];
  /** Send a message and generate responses from multiple companion models */
  sendMultiModelMessage: (
    content: string,
    companionModels: Array<{ providerId: string; modelId: string }>,
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
  pendingClarification: null,
  setPendingClarification: (c) => set({ pendingClarification: c }),
  sidebarAutoSelectSuppressed: false,
  sidebarSuppressTimer: null,
  titleGeneratingConversationId: null,
  pendingCompanionModels: [],
  multiModelParentId: null,
  multiModelDoneMessageIds: [],
  pendingPromptText: null,
  setPendingPromptText: (text) => {
    useMultiModelStore.getState().setPendingPromptText(text);
    set({ pendingPromptText: text });
  },
  // 偏好设置在 store 初始化后从 preferenceStore 同步，此时用 null 占位避免循环依赖
  searchEnabled: null as unknown as boolean,
  searchProviderId: null as unknown as string,
  thinkingBudget: null as unknown as number,
  mcpMode: null as unknown as "manual" | "auto" | "disabled",
  enabledMcpServerIds: [] as string[],
  enabledKnowledgeBaseIds: [] as string[],
  activeMemoryNamespaceId: null,
  enabledWikiIds: [] as string[],
  syncPreferencesFromStore: () => {
    try {
      const p = getPref().getState();
      set({
        searchEnabled: p.searchEnabled,
        searchProviderId: p.searchProviderId,
        thinkingBudget: p.thinkingBudget,
        mcpMode: p.mcpMode,
        enabledMcpServerIds: p.enabledMcpServerIds,
        enabledKnowledgeBaseIds: p.enabledKnowledgeBaseIds,
        activeMemoryNamespaceId: p.activeMemoryNamespaceId,
        enabledWikiIds: p.enabledWikiIds,
      });
    } catch {
      // preferenceStore 尚未就绪，稍后重试
    }
  },
  setMcpMode: (mode: "auto" | "manual" | "disabled") => {
    getPref().getState().setMcpMode(mode);
    set({ mcpMode: mode });
  },
  setSearchEnabled: (enabled) => {
    getPref().getState().setSearchEnabled(enabled);
    set({ searchEnabled: enabled });
  },
  setSearchProviderId: (id) => {
    getPref().getState().setSearchProviderId(id);
    set({ searchProviderId: id });
  },
  toggleMcpServer: async (id) => {
    const current = get().enabledMcpServerIds;
    const next = current.includes(id)
      ? current.filter((s) => s !== id)
      : [...current, id];
    set({ enabledMcpServerIds: next });
    try {
      await getPref().getState().toggleMcpServer(id);
    } catch (e) {
      set({ enabledMcpServerIds: current });
      throw e;
    }
  },
  setThinkingBudget: (budget) => {
    getPref().getState().setThinkingBudget(budget);
    set({ thinkingBudget: budget });
  },
  toggleKnowledgeBase: async (id) => {
    const current = get().enabledKnowledgeBaseIds;
    const next = current.includes(id)
      ? current.filter((s) => s !== id)
      : [...current, id];
    set({ enabledKnowledgeBaseIds: next });
    try {
      await getPref().getState().toggleKnowledgeBase(id);
    } catch (e) {
      set({ enabledKnowledgeBaseIds: current });
      throw e;
    }
  },
  setActiveMemoryNamespace: (id) => {
    const current = get().activeMemoryNamespaceId;
    const nextId = current === id ? null : id;
    getPref().getState().setActiveMemoryNamespaceId(nextId);
    set({ activeMemoryNamespaceId: nextId });
  },
  toggleWiki: (id) => {
    const current = get().enabledWikiIds;
    const next = current.includes(id)
      ? current.filter((s) => s !== id)
      : [...current, id];
    getPref().getState().toggleWiki(id);
    set({ enabledWikiIds: next });
  },
  insertContextClear: async () => {
    const conversationId = get().activeConversationId;
    if (!conversationId) {
      return;
    }
    try {
      const msg = await invoke<Message>("send_system_message", {
        conversationId,
        content: "<!-- context-clear -->",
      });
      set((s) => ({ messages: [...s.messages, msg] }));
      // Backup and clear agent SDK context (no-op if no agent session exists)
      await invoke("agent_backup_and_clear_sdk_context", {
        conversationId,
      }).catch(logIpcError("agent_backup_and_clear_sdk_context"));
    } catch {
      // If backend command doesn't exist yet, add optimistic local message
      const localMsg: Message = {
        id: tempId("ctx-clear-"),
        conversationId: conversationId,
        role: "system",
        content: "<!-- context-clear -->",
        providerId: null,
        modelId: null,
        tokenCount: null,
        attachments: [],
        thinking: null,
        toolCallsJson: null,
        toolCallId: null,
        createdAt: Math.floor(Date.now() / 1000),
        parentMessageId: null,
        versionIndex: 0,
        isActive: true,
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
        await invoke("agent_restore_sdk_context_from_backup", {
          conversationId,
        }).catch(logIpcError("agent_restore_sdk_context_from_backup"));
      }
    } catch (e) {
      set({ error: String(e) });
      throw e;
    }
  },

  clearAllMessages: async () => {
    const conversationId = get().activeConversationId;
    if (!conversationId) {
      return;
    }
    // Guard: cancel any active stream before clearing messages.
    // Otherwise the backend stream task would try to update a deleted
    // placeholder message in DB, producing errors and orphaned chunks.
    if (
      isConvStreaming(useStreamStore.getState().activeStreams, conversationId)
    ) {
      useStreamStore.getState().cancelCurrentStream(conversationId);
    }
    try {
      await invoke("clear_conversation_messages", { conversationId });
      // Stale guard: don't wipe messages if user switched conversations
      if (get().activeConversationId !== conversationId) {
        return;
      }
      set({
        messages: [],
        hasOlderMessages: false,
        totalActiveCount: 0,
        oldestLoadedMessageId: null,
        loadingOlder: false,
      });
    } catch (e) {
      logIpcError("clearAllMessages")(e);
      set({ error: String(e) });
    }
  },

  switchModel: async (modelKeyword: string) => {
    const conversationId = get().activeConversationId;
    const conversation = get().conversations.find(
      (c) => c.id === conversationId,
    );
    if (!conversationId || !conversation) {
      return;
    }

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
          if (!m.enabled) {
            continue;
          }
          const modelLower = m.modelId.toLowerCase();
          const exact = modelLower === keyword;
          // js-set-map-lookups: 子串匹配无法用 Set.has 替代
          const contains = modelLower.includes(keyword);
          if (!exact && !contains) {
            continue;
          }
          const sameProvider = p.id === conversation.providerId;
          const score = exact ? (sameProvider ? 3 : 2) : sameProvider ? 1 : 0;
          if (score > bestScore) {
            bestScore = score;
            bestProviderId = p.id;
            bestModelId = m.modelId;
          }
        }
      }

      if (bestProviderId && bestModelId) {
        await get().updateConversation(conversationId, {
          providerId: bestProviderId,
          modelId: bestModelId,
        });
      }
    } catch (e) {
      logIpcError("switchModel")(e);
      set({ error: String(e) });
    }
  },

  fetchConversations: async () => {
    set({ loading: true });
    try {
      const conversations = await invoke<Conversation[]>(
        "list_conversations",
        undefined,
        15_000,
      );
      set({ conversations, loading: false, error: null });
    } catch (e) {
      set({ error: String(e), loading: false });
    }
  },

  setActiveConversation: (id) => {
    if (
      id === get().activeConversationId
      && (!id || !_pendingConversationRefresh.has(id))
    ) {
      return;
    }
    const prevId = get().activeConversationId;
    incrementActiveMessageLoadSeq();
    if (!id) {
      if (prevId === null) {
        return;
      }
      if (prevId) {
        const prevStreaming = isConvStreaming(useStreamStore.getState().activeStreams, prevId);
        if (!prevStreaming) {
          useAgentStore.getState().clearConversation(prevId);
          useExecutionStore.getState().clearConversation(prevId);
        } else {
          useAgentStore.getState().clearConversationUI(prevId);
          useExecutionStore.getState().clearConversationUI(prevId);
        }
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
      const prevStreaming = isConvStreaming(useStreamStore.getState().activeStreams, prevId);
      if (!prevStreaming) {
        useAgentStore.getState().clearConversation(prevId);
        useExecutionStore.getState().clearConversation(prevId);
      } else {
        useAgentStore.getState().clearConversationUI(prevId);
        useExecutionStore.getState().clearConversationUI(prevId);
      }
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
    // 同步偏好状态到 preferenceStore（两个不同 store 的 setState，不能合并）
    getPref().setState(prefState);
    // 保留尚未持久化的 temp- 消息，防止被服务端返回的列表覆盖丢失
    const tempIds = get().messages.flatMap((m) => m.id.startsWith("temp-") ? [m.id] : []);
    get()
      .fetchMessages(id, tempIds)
      .then(() => {
        if (
          requestSeq !== _activeMessageLoadSeq
          || get().activeConversationId !== id
        ) {
          return;
        }
        // FE-I11 修复：使用会话级 streamBuffer（getSession）替代全局 _streamBuffer，
        // 避免跨会话切换时 global buffer 被其他会话的流劫持导致串扰。
        const session = getSession(id);
        const sessionBuffer = session?.streamBuffer ?? null;
        // If there's an active stream for this conversation, inject buffered content
        if (
          sessionBuffer
          && sessionBuffer.conversationId === id
          && isConvStreaming(useStreamStore.getState().activeStreams, id)
        ) {
          const realId = sessionBuffer.resolvedId ?? sessionBuffer.messageId;
          set((s) => {
            const exists = s.messages.some((m) => m.id === realId);
            if (exists) {
              // Message already fetched from backend — replace with buffered content (more up-to-date)
              useStreamStore.setState({ streamingMessageId: realId });
              return {
                messages: s.messages.map((m) =>
                  m.id === realId
                    ? {
                      ...m,
                      content: sessionBuffer!.content,
                      thinking: sessionBuffer!.thinking || null,
                    }
                    : m
                ),
              };
            }
            // Message not yet in backend — create from buffer
            const newMessage: Message = {
              id: realId,
              conversationId: id,
              role: "assistant",
              content: sessionBuffer!.content,
              providerId: null,
              modelId: null,
              tokenCount: null,
              attachments: [],
              thinking: sessionBuffer!.thinking || null,
              toolCallsJson: null,
              toolCallId: null,
              createdAt: Math.floor(Date.now() / 1000),
              parentMessageId: null,
              versionIndex: 0,
              isActive: true,
              status: "partial",
            };
            useStreamStore.setState({ streamingMessageId: realId });
            return {
              messages: [...s.messages, newMessage],
            };
          });
        } else if (
          sessionBuffer
          && sessionBuffer.conversationId === id
          && needsRefreshAfterStreamDone
        ) {
          // Stream completed while user was away — buffer still has final content.
          // fetchMessages already loaded the completed message from DB, but inject
          // buffer content in case the DB response is slightly behind.
          const realId = sessionBuffer.resolvedId ?? sessionBuffer.messageId;
          set((s) => {
            const exists = s.messages.some((m) => m.id === realId);
            if (exists) {
              return {
                messages: s.messages.map((m) =>
                  m.id === realId
                    ? {
                      ...m,
                      content: sessionBuffer!.content,
                      thinking: sessionBuffer!.thinking || null,
                    }
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

  createConversation: async (title, modelId, providerId, options) => {
    try {
      const category = options?.categoryId
        ? (useCategoryStore
          .getState()
          .categories.find((item) => item.id === options.categoryId) ?? null)
        : null;

      // ── 模型解析：按优先级逐级获取有效的 providerId + modelId ──
      // 优先级顺序：
      //   1. 分类默认值（若指定了 categoryId 且分类有默认值）— 覆盖显式参数
      //   2. 显式传入的参数（若非空）
      //   3. 设置中的全局默认值（settings.defaultModel?.a / settings.defaultModel?.b）
      //   4. 第一个已启用的 provider 下的第一个已启用模型
      //   5. 任意有模型的 provider + 其第一个模型（最终兜底，永远不会空）

      let templateProviderId = category?.defaultModel?.a ?? providerId ?? "";
      let templateModelId = category?.defaultModel?.b ?? modelId ?? "";

      if (!templateModelId || !templateProviderId) {
        const settings = useSettingsStore.getState().settings;

        // 加载 providers（如果尚未加载）
        let providers = useProviderStore.getState().providers;
        if (providers.length === 0) {
          try {
            await useProviderStore.getState().fetchProviders();
            providers = useProviderStore.getState().providers;
          } catch {
            // 加载失败则继续，后续兜底逻辑会处理
          }
        }

        // 优先级 3: 设置中的全局默认值
        if (!templateProviderId && settings.defaultModel?.a) {
          templateProviderId = settings.defaultModel.a;
        }
        if (!templateModelId && settings.defaultModel?.b) {
          templateModelId = settings.defaultModel.b;
        }

        // 优先级 4: 匹配指定 provider 或取第一个 enabled provider + enabled model
        if (!templateModelId || !templateProviderId) {
          const provider = providers.find(
            (p) => p.id === templateProviderId && p.enabled,
          ) ?? providers.find(
            (p) => p.enabled && p.models.some((m) => m.enabled),
          );
          if (provider) {
            templateProviderId = templateProviderId || provider.id;
            const model = provider.models.find((m) => m.enabled)
              ?? provider.models[0];
            if (model) {
              templateModelId = templateModelId || model.modelId;
            }
          }
        }

        // 优先级 5（最终兜底）: 任意有模型的 provider，取其第一个模型
        if (!templateModelId || !templateProviderId) {
          const fallback = providers.find((p) => p.models.length > 0);
          if (fallback) {
            templateProviderId = templateProviderId || fallback.id;
            templateModelId = templateModelId || fallback.models[0].modelId;
          }
        }
      }

      // 最终检查 — 理论上不应到达此分支，除非 providers 列表完全为空
      // （浏览器 mock 至少有 2 个 builtin provider）
      if (!templateModelId || !templateProviderId) {
        console.error("[createConversation] Failed to resolve valid provider/model, using fallback", {
          modelId,
          providerId,
          settingsDefaultProviderId: useSettingsStore.getState().settings.defaultModel?.a,
          settingsDefaultModelId: useSettingsStore.getState().settings.defaultModel?.b,
          providersCount: useProviderStore.getState().providers.length,
        });
        // 终极兜底 — 使用硬编码的 builtin provider/model
        templateProviderId = templateProviderId || "builtin-openai";
        templateModelId = templateModelId || "gpt-4o";
      }
      const createdConversation = await invoke<Conversation>(
        "create_conversation",
        {
          title,
          modelId: templateModelId,
          providerId: templateProviderId,
          systemPrompt: options?.systemPrompt ?? category?.systemPrompt ?? undefined,
        },
      );
      let conversation = createdConversation;
      try {
        conversation = await invoke<Conversation>(
          "update_conversation",
          {
            id: createdConversation.id,
            input: {
              ...categoryTemplateUpdateFromCategory(category),
              ...conversationPreferenceUpdateFromState(
                getPref().getState(),
              ),
              scenario: options?.scenario,
              agentProfileId: options?.agentProfileId,
              workflowTemplateId: options?.workflowTemplateId,
              mode: options?.mode,
              workStrategy: options?.workStrategy,
              ...getStagedPreferenceUpdate(),
            },
          },
          10_000,
        );
      } catch (preferenceError) {
        // 非致命：对话已创建，偏好设置未应用，使用默认值
        logIpcError("createConversation.preferenceUpdate")(preferenceError);
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
      getPref().setState(
        conversationPreferenceStateFromConversation(conversation),
      );
      return conversation;
    } catch (e) {
      set({ error: String(e) });
      throw e;
    }
  },

  updateConversation: async (id, input) => {
    try {
      const updated = await invoke<Conversation>("update_conversation", {
        id,
        input,
      });
      set((s) => ({
        ...mergeConversationCollections(
          s.conversations,
          s.archivedConversations,
          updated,
        ),
        error: null,
      }));
      // Sync preference state if this is the active conversation
      if (get().activeConversationId === id) {
        getPref().setState(
          conversationPreferenceStateFromConversation(updated),
        );
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
      // FE-I11 修复：删除会话时同步清理其流式 session（含 buffer 与 flush timer）
      removeSession(id);
      deletePendingConversationRefresh(id);
      const state = get();
      // 清理关联的 tab，防止会话删除后 tab 残留
      useTabStore.getState().removeTabsByConversationId(id);
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

  branchConversation: async (
    conversationId,
    untilMessageId,
    asChild,
    title,
  ) => {
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
      const msgs = await invoke<Message[]>("list_messages", {
        conversationId: newConv.id,
      });
      // Stale guard: if user switched away, discard messages to prevent cross-conversation pollution
      if (get().activeConversationId !== newConv.id) {
        return newConv;
      }
      set({ messages: msgs });
      return newConv;
    } catch (e) {
      set({ error: String(e) });
      throw e;
    }
  },

  togglePin: async (id) => {
    try {
      const updated = await invoke<Conversation>("toggle_pin_conversation", {
        id,
      });
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

  toggleArchive: async (id: string) => {
    try {
      // If the conversation is currently streaming, cancel it first
      if (isConvStreaming(useStreamStore.getState().activeStreams, id)) {
        useStreamStore.getState().cancelCurrentStream(id);
      }

      const command = "toggle_archive_conversation";
      const params = { id };

      const updated = await invoke<Conversation>(command, params);
      // Clean up other stores when archiving
      useAgentStore.getState().clearConversation(id);
      useExecutionStore.getState().clearConversation(id);
      usePlanStore.getState().clearActivePlan(id);
      useTrajectoryStore.getState().clearConversation(id);
      if (updated.isArchived) {
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
          archivedConversations: s.archivedConversations.filter(
            (c) => c.id !== id,
          ),
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
      const updated = await invoke<Conversation>(
        "archive_conversation_to_knowledge_base",
        {
          id,
          knowledgeBaseId,
        },
      );
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
      const archived = await invoke<Conversation[]>(
        "list_archived_conversations",
      );
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
      activeConversationId: ids.includes(s.activeConversationId ?? "")
        ? null
        : s.activeConversationId,
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
        return invoke<Conversation>("toggle_archive_conversation", { id });
      }),
    );
    const archived: Conversation[] = [];
    for (const r of results) {
      if (r.status === "fulfilled" && r.value.isArchived) {
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
      activeConversationId: ids.includes(s.activeConversationId ?? "")
        ? null
        : s.activeConversationId,
      messages: ids.includes(s.activeConversationId ?? "") ? [] : s.messages,
      error: null,
    }));
  },
  ...createSendMethods(set, get),
  deleteMessage: async (messageId) => {
    const conversationId = get().activeConversationId;
    if (!conversationId) {
      return;
    }
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
      if (get().activeConversationId !== conversationId) {
        return;
      }
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
      if (
        requestSeq !== _activeMessageLoadSeq
        || get().activeConversationId !== conversationId
      ) {
        return;
      }

      set((s) => {
        const messages = mergePreservedMessages(
          page.messages,
          preserveMessageIds,
          s.messages,
        );
        return {
          messages,
          loading: false,
          loadingOlder: false,
          hasOlderMessages: page.hasOlder,
          totalActiveCount: page.totalActiveCount,
          oldestLoadedMessageId: messages[0]?.id ?? page.oldestMessageId,
          error: null,
        };
      });
    } catch (e) {
      if (
        requestSeq !== _activeMessageLoadSeq
        || get().activeConversationId !== conversationId
      ) {
        return;
      }
      const errorMessage = String(e);
      if (errorMessage.includes("Not found: Conversation")) {
        await get()
          .fetchConversations()
          .catch(logIpcError("fetchConversations.afterConversationNotFound"));
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
    const {
      activeConversationId,
      oldestLoadedMessageId,
      hasOlderMessages,
      loading,
      loadingOlder,
    } = get();
    if (
      !activeConversationId
      || !oldestLoadedMessageId
      || !hasOlderMessages
      || loading
      || loadingOlder
    ) {
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
      if (
        requestSeq !== _activeMessageLoadSeq
        || get().activeConversationId !== activeConversationId
      ) {
        return;
      }

      set((s) => ({
        messages: mergeOlderPages(page.messages, s.messages),
        loadingOlder: false,
        hasOlderMessages: page.hasOlder,
        totalActiveCount: page.totalActiveCount,
        oldestLoadedMessageId: page.oldestMessageId ?? s.oldestLoadedMessageId,
        error: null,
      }));
    } catch (e) {
      if (
        requestSeq !== _activeMessageLoadSeq
        || get().activeConversationId !== activeConversationId
      ) {
        return;
      }
      set({ error: String(e), loadingOlder: false });
    }
  },

  searchConversations: async (query) => {
    try {
      return await invoke<ConversationSearchResult[]>("search_conversations", {
        query,
      });
    } catch (e) {
      set({ error: String(e) });
      throw e;
    }
  },
  ...createEventMethods(set, get),
  switchMessageVersion: async (conversationId, parentMessageId, messageId) => {
    try {
      if (_isMultiModelActive) {
        // During multi-model streaming, skip the backend call entirely to avoid:
        // 1. Race conditions with concurrent regenerate_with_model calls
        // 2. invoke delay causing stale content display
        // 3. Potential invoke failures during active streaming
        // Just swap isActive flags in-memory; backend will be synced during cleanup.
        setUserManuallySelectedVersion(true);
        // 多模型路径：仅在内存中切换 isActive（与下方正常路径 + catch 路径互斥）
        set((s) => {
          const targetExists = s.messages.some(
            (m) =>
              m.id === messageId
              && m.parentMessageId === parentMessageId
              && m.role === "assistant",
          );
          if (!targetExists) {
            return {}; // Target not in memory yet, no-op
          }
          return {
            messages: s.messages.map((m) => {
              if (
                m.parentMessageId !== parentMessageId
                || m.role !== "assistant"
              ) {
                return m;
              }
              return m.id === messageId
                ? { ...m, isActive: true }
                : { ...m, isActive: false };
            }),
          };
        });
        return;
      }

      await invoke("switch_message_version", {
        conversationId,
        parentMessageId,
        messageId,
      });

      // Normal path: fetch all versions from DB and keep them all in store
      // with correct isActive flags. This preserves multi-model detection
      // (multiModelResponseParents) which needs multiple versions visible.
      const versions = await get().listMessageVersions(
        conversationId,
        parentMessageId,
      );
      if (versions.length > 0) {
        // 正常路径：从 DB 获取版本更新 store（与上方多模型路径 + 下方 catch 路径互斥）
        set((s) => {
          const versionMap = new Map(versions.map((v) => [v.id, v]));
          const existingIds = new Set(
            s.messages.flatMap((m) =>
              m.parentMessageId === parentMessageId && m.role === "assistant"
                ? [m.id]
                : []
            ),
          );
          // Update existing versions in-place
          const updatedMessages = s.messages.map((m) => {
            if (
              m.parentMessageId !== parentMessageId
              || m.role !== "assistant"
            ) {
              return m;
            }
            const dbVersion = versionMap.get(m.id);
            if (dbVersion) {
              return { ...dbVersion, isActive: m.id === messageId };
            }
            return { ...m, isActive: m.id === messageId };
          });
          // Add any DB versions not already in store
          for (const v of versions) {
            if (!existingIds.has(v.id)) {
              updatedMessages.push({ ...v, isActive: v.id === messageId });
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
      return await invoke<Message[]>("list_message_versions", {
        conversationId,
        parentMessageId,
      });
    } catch (e) {
      set({ error: String(e) });
      return [];
    }
  },

  updateMessageContent: async (messageId, content) => {
    try {
      const updated = await invoke<Message>("update_message_content", {
        id: messageId,
        content,
      });
      set((s) => ({
        messages: s.messages.map((m) => m.id === messageId ? { ...m, content: updated.content } : m),
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
        messages: s.messages.filter(
          (m) => m.id !== userMessageId && m.parentMessageId !== userMessageId,
        ),
      }));
      return;
    }
    try {
      await invoke("delete_message_group", {
        conversationId: conversationId,
        userMessageId: userMessageId,
      });
      // Stale guard: don't filter messages if user switched conversations
      if (get().activeConversationId !== conversationId) {
        return;
      }
      set((s) => ({
        messages: s.messages.filter(
          (m) => m.id !== userMessageId && m.parentMessageId !== userMessageId,
        ),
      }));
    } catch (e) {
      set({ error: String(e) });
    }
  },

  workspaceSnapshot: null,

  loadWorkspaceSnapshot: async (conversationId) => {
    try {
      const snapshot = await invoke<ConversationWorkspaceSnapshot>(
        "get_workspace_snapshot",
        {
          conversationId: conversationId,
        },
      );
      set({ workspaceSnapshot: snapshot });
      return snapshot;
    } catch (e) {
      // C-P1-2: 记录错误日志,避免静默吞错导致问题难以排查
      logIpcError("loadWorkspaceSnapshot")(e);
      set({ workspaceSnapshot: null });
      return null;
    }
  },

  updateWorkspaceSnapshot: async (conversationId, snapshot) => {
    try {
      await invoke("update_workspace_snapshot", {
        conversationId,
        ...snapshot,
      });
      set((s) => ({
        workspaceSnapshot: s.workspaceSnapshot
          ? { ...s.workspaceSnapshot, ...snapshot }
          : null,
      }));
    } catch (e) {
      logIpcError("updateWorkspaceSnapshot")(e);
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
    } catch (e) {
      // C-P1-2: 记录错误日志,避免静默吞错
      logIpcError("compareResponses")(e);
      return null;
    }
  },
}));

// Register conversationStore reference so streamStore can call back
registerConversationStoreRef({
  getState: () => useConversationStore.getState(),
  setState: (partial) =>
    useConversationStore.setState(
      partial as unknown as Parameters<typeof useConversationStore.setState>[0],
    ), /* SAFE: external store update via dynamic type */
});

// ─── Sidebar auto-select suppression ───
//
// When deleteConversation or toggleArchive removes the active conversation,
// ChatSidebar's useEffect would normally auto-select the next conversation.
// Setting this flag to true tells the sidebar to skip auto-select for one
// render cycle, keeping the ChatView on the welcome screen.
//
// ── Sidebar Auto-Select Suppression ──────────────────────────────────
// 防止删除/归档后侧栏自动跳转。使用 Zustand store 替代模块级可变状态，
// 避免多测试/SSR 场景下的状态污染。
// 本质上是临时绕过方案；长期应改用更清晰的 UI 状态机。

export function isSidebarAutoSelectSuppressed(): boolean {
  return useConversationStore.getState().sidebarAutoSelectSuppressed;
}

export function setSidebarAutoSelectSuppressed(val: boolean): void {
  useConversationStore.setState({ sidebarAutoSelectSuppressed: val });
}

/** Reset the sidebar auto-select suppression flag (called by ChatSidebar after consuming). */
export function resetSidebarAutoSelectSuppression() {
  const state = useConversationStore.getState();
  if (state.sidebarSuppressTimer) {
    clearTimeout(state.sidebarSuppressTimer);
  }
  useConversationStore.setState({ sidebarAutoSelectSuppressed: false, sidebarSuppressTimer: null });
}

export function setSidebarAutoSelectSuppression() {
  const state = useConversationStore.getState();
  if (state.sidebarSuppressTimer) {
    clearTimeout(state.sidebarSuppressTimer);
  }
  const timer = setTimeout(() => {
    useConversationStore.setState({ sidebarAutoSelectSuppressed: false, sidebarSuppressTimer: null });
  }, 5000);
  useConversationStore.setState({ sidebarAutoSelectSuppressed: true, sidebarSuppressTimer: timer });
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

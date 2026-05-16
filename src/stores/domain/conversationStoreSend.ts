// @ts-nocheck - S-20 extracted module, types verified at composition point
// S-20: Send method factory extracted from conversationStore

import i18n from "@/i18n";
import { invoke, isTauri, listen, logIpcError, type UnlistenFn } from "@/lib/invoke";
import { buildKnowledgeTag, buildMemoryTag, buildWikiTag, type RagContextRetrievedEvent } from "@/lib/memoryUtils";
import { mergeOlderPages, mergePreservedMessages, MESSAGE_PAGE_SIZE } from "@/lib/messageUtils";
import { buildSearchTag, formatSearchContent } from "@/lib/searchUtils";
import { useSearchStore } from "@/stores";
import { useProviderStore } from "@/stores/feature/providerStore";
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
  WorkflowCompleteEvent,
  WorkflowEvent,
} from "@/types";
import { create } from "zustand";
import { useAgentStore } from "../feature/agentStore";
import { useCategoryStore } from "../feature/categoryStore";
import { useExecutionStore } from "../feature/executionStore";
import { usePlanStore } from "../feature/planStore";
import { useTrajectoryStore } from "../feature/trajectoryStore";
import { useMultiModelStore } from "./multiModelStore";
import {
  categoryTemplateUpdateFromCategory,
  conversationPreferenceStateFromConversation,
  conversationPreferenceUpdateFromState,
  getEffectiveThinkingBudget,
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
  setMultiModelDoneResolve,
  setMultiModelFirstMessageId,
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

// 单调递增计数器，与 Date.now() 组合防止同毫秒 ID 重复
import { tempId } from "./conversationHelpers";

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
    const providers = useProviderStore.getState().providers ?? [];
    // TODO: 从 providerStore 读取用户配置的默认模型。当前 store 尚未实现
    // 默认模型优先级选择功能，保留 undefined 占位以维持 fallback 链结构。
    const defaultProviderId: string | undefined = undefined;
    const defaultModelId: string | undefined = undefined;

    for (const p of providers) {
      for (const m of p.models ?? []) {
        const key = `${p.id}:${m.model_id}`;
        if (key === `${currentProviderId}:${currentModelId}`) { continue; }

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

import type { ConversationState } from "./conversationStore";

export function createSendMethods(
  set: (fn: (s: ConversationState) => Partial<ConversationState>) => void,
  get: () => ConversationState,
) {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  return {
    sendMessage: async (content, attachments = [], searchProviderId = null) => {
      const conversationId = get().activeConversationId;
      if (!conversationId) { throw new Error("No active conversation"); }

      // Guard: prevent duplicate sends while a stream is already active for this conversation
      if (isConvStreaming(useStreamStore.getState().activeStreams, conversationId)) {
        console.warn("[sendMessage] Ignoring duplicate send — stream already active for", conversationId);
        return;
      }

      // Hoisted variables used by both try and catch blocks
      let finalContent = content;
      let kbIds: string[] = [];
      let memIds: string[] = [];
      let mcpIds: string[] = [];
      let thinkingBudget: number | undefined;

      // Optimistically add user message BEFORE backend call
      const optimisticUserMsg: Message = {
        id: tempId("temp-user-"),
        conversation_id: conversationId,
        role: "user",
        content,
        provider_id: null,
        model_id: null,
        token_count: null,
        attachments: attachments.map((a) => ({
          id: tempId("temp-att-"),
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
      const tempAssistantId = tempId("temp-assistant-");
      kbIds = usePreferenceStore.getState().enabledKnowledgeBaseIds;
      const activeMemId1 = usePreferenceStore.getState().activeMemoryNamespaceId;
      memIds = activeMemId1 ? [activeMemId1] : [];
      const wikiIds = usePreferenceStore.getState().enabledWikiIds;
      const hasKnowledgeRag = kbIds.length > 0;
      const hasMemoryRag = memIds.length > 0;
      const hasWikiRag = wikiIds.length > 0;
      const hasAnyRag = hasKnowledgeRag || hasMemoryRag || hasWikiRag;
      let placeholderContent = "";
      if (searchProviderId) { placeholderContent += buildSearchTag("searching"); }
      if (hasKnowledgeRag) { placeholderContent += buildKnowledgeTag("searching"); }
      if (hasMemoryRag) { placeholderContent += buildMemoryTag("searching"); }
      if (hasWikiRag) { placeholderContent += buildWikiTag("searching"); }
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
          const wikiPart = hasWikiRag ? buildWikiTag("searching") : "";
          setStreamPrefix(searchResultTag + kbPart + memPart + wikiPart);
          set((s) => ({
            messages: s.messages.map(m =>
              m.id === tempAssistantId ? { ...m, content: searchResultTag + kbPart + memPart + wikiPart } : m
            ),
          }));
        } else if (hasAnyRag) {
          // RAG only — set prefix so searching tags flow into stream buffer
          const kbPart = hasKnowledgeRag ? buildKnowledgeTag("searching") : "";
          const memPart = hasMemoryRag ? buildMemoryTag("searching") : "";
          const wikiPart = hasWikiRag ? buildWikiTag("searching") : "";
          setStreamPrefix(kbPart + memPart + wikiPart);
        }

        mcpIds = usePreferenceStore.getState().enabledMcpServerIds;
        thinkingBudget = getEffectiveThinkingBudget(conversationId);
        kbIds = usePreferenceStore.getState().enabledKnowledgeBaseIds;
        const activeMemNsIdForSend = usePreferenceStore.getState().activeMemoryNamespaceId;
        memIds = activeMemNsIdForSend ? [activeMemNsIdForSend] : [];
        const wikiIdsForSend = usePreferenceStore.getState().enabledWikiIds;
        const userMessage = await invoke<Message>("send_message", {
          conversationId,
          content: finalContent,
          attachments,
          enabledMcpServerIds: mcpIds.length > 0 ? mcpIds : undefined,
          thinkingBudget,
          enabledKnowledgeBaseIds: kbIds.length > 0 ? kbIds : undefined,
          enabledMemoryNamespaceIds: memIds.length > 0 ? memIds : undefined,
          enabledWikiIds: wikiIdsForSend.length > 0 ? wikiIdsForSend : undefined,
        });

        // Stale guard: if user switched conversations while send was in-flight,
        // discard the response to prevent cross-conversation message pollution.
        if (get().activeConversationId !== conversationId) { return; }

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
          && !errMsg.includes("invalid_request_error") // bad request
          && !errMsg.includes("authentication") // auth error
          && !errMsg.includes("insufficient_quota") // billing
          && !errMsg.includes("invalid_api_key") // auth
          && !errMsg.includes("context_length_exceeded"); // context too long

        // Try fallback models before showing error (use loop, not recursion)
        if (isRetryable) {
          const conversation = get().conversations.find(c => c.id === conversationId);
          const currentProviderId = conversation?.provider_id;
          const currentModelId = conversation?.model_id;

          if (currentProviderId && currentModelId) {
            const fallbackChain = buildFallbackChain(currentProviderId, currentModelId);
            let fallbackSucceeded = false;
            // 保存原始 provider/model，全部 fallback 失败后恢复
            const originalProviderId = currentProviderId;
            const originalModelId = currentModelId;
            for (let i = 0; i < fallbackChain.length; i++) {
              const fb = fallbackChain[i];
              try {
                await get().updateConversation(conversationId, { provider_id: fb.providerId, model_id: fb.model_id });
                const currentActiveStreams = useStreamStore.getState().activeStreams;
                if (isConvStreaming(currentActiveStreams, conversationId)) { return; }
                // Remove error placeholder
                const currentMsgId = getStreamingMessageId(useStreamStore.getState().activeStreams, conversationId);
                set((s) => ({
                  messages: s.messages.filter(m =>
                    m.id !== currentMsgId && !(m.status === "error" && m.role === "assistant" && m.content === errMsg)
                  ),
                }));
                // Re-invoke send_message directly (not recursive sendMessage)
                await invoke("send_message", {
                  conversationId,
                  content: finalContent,
                  attachments,
                  enabledMcpServerIds: mcpIds.length > 0 ? mcpIds : undefined,
                  thinkingBudget,
                  enabledKnowledgeBaseIds: kbIds.length > 0 ? kbIds : undefined,
                  enabledMemoryNamespaceIds: memIds.length > 0 ? memIds : undefined,
                  enabledWikiIds: usePreferenceStore.getState().enabledWikiIds.length > 0
                    ? usePreferenceStore.getState().enabledWikiIds
                    : undefined,
                });
                // Re-start stream
                const newTempId = tempId("temp-assistant-");
                useStreamStore.setState((s) => ({
                  ...startConversationStream(s.activeStreams, conversationId, newTempId),
                  streamingStartTimestamps: { ...s.streamingStartTimestamps, [conversationId]: Date.now() },
                  thinkingActiveMessageIds: new Set<string>(),
                }));
                fallbackSucceeded = true;
                break;
              } catch (_fallbackErr) { /* continue to next */ }
            }
            if (fallbackSucceeded) { return; }

            // 全部 fallback 失败，恢复原始 provider/model
            await get().updateConversation(conversationId, {
              provider_id: originalProviderId,
              model_id: originalModelId,
            }).catch(logIpcError("恢复原始模型配置"));
          }
        }

        // All fallbacks exhausted or error not retryable — show error
        const currentStreamingMessageId = getStreamingMessageId(
          useStreamStore.getState().activeStreams,
          conversationId,
        );
        useStreamStore.setState((s) => ({
          ...stopConversationStream(s.activeStreams, conversationId),
          streamingStartTimestamps: (() => {
            const t = { ...s.streamingStartTimestamps };
            delete t[conversationId];
            return t;
          })(),
          thinkingActiveMessageIds: new Set<string>(),
        }));
        // Generate error message ID upfront so it can be preserved across fetchMessages
        const tempErrorId = tempId("temp-error-");
        set((s) => ({
          messages: currentStreamingMessageId
            ? s.messages.map(m =>
              m.id === currentStreamingMessageId
                ? { ...m, content: errMsg, status: "error" as const }
                : m
            )
            : [...s.messages, {
              id: tempErrorId,
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
        // Preserve the temp-error message AND the optimistic user message so they
        // aren't silently dropped when invoke("send_message") failed entirely.
        // (If the DB also has the real user message, mergePreservedMessages keeps both;
        // a duplicate user bubble is much less harmful than losing the user's input.)
        const errorPreserveIds = [optimisticUserMsg.id, tempErrorId, currentStreamingMessageId].filter(
          (value): value is string => typeof value === "string" && value.length > 0,
        );
        window.setTimeout(() => {
          void get().fetchMessages(conversationId, errorPreserveIds);
        }, 120);
      }
    },

    sendAgentMessage: async (content, attachments = [], searchProviderId: string | null = null) => {
      const conversationId = get().activeConversationId;
      if (!conversationId) { throw new Error("No active conversation"); }

      const conversation = get().conversations.find((c) => c.id === conversationId);
      if (!conversation) { throw new Error("Conversation not found"); }

      // Guard: prevent duplicate sends while a stream is already active for this conversation
      if (isConvStreaming(useStreamStore.getState().activeStreams, conversationId)) {
        console.warn("[sendAgentMessage] Ignoring duplicate send — stream already active for", conversationId);
        return;
      }

      // Agent 模式仅在 Tauri 桌面端可用，浏览器模式不支持
      if (!isTauri()) {
        set((s) => ({
          error: i18n.t("agentMode.requiresTauri"),
          messages: [
            ...s.messages,
            {
              id: tempId("temp-user-"),
              conversation_id: conversationId,
              role: "user",
              content,
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
            },
            {
              id: tempId("temp-agent-error-"),
              conversation_id: conversationId,
              role: "assistant",
              content: i18n.t("agentMode.requiresTauriDetail"),
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
            },
          ],
        }));
        return;
      }

      const providerId = conversation.provider_id;
      const model_id = conversation.model_id;

      // Optimistic user message
      const optimisticUserMsg: Message = {
        id: tempId("temp-user-"),
        conversation_id: conversationId,
        role: "user",
        content,
        provider_id: null,
        model_id: null,
        token_count: null,
        attachments: attachments.map((a) => ({
          id: tempId("temp-att-"),
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
        content: i18n.t("agentMode.thinking"),
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
      let unlistenStatus: UnlistenFn | null = null;

      // Agent 超时保护：10 分钟无响应则报错
      const AGENT_TIMEOUT_MS = 10 * 60 * 1000;
      // Hoist promise reject so the timeout can reject the promise
      let _agentReject: ((reason: Error) => void) | null = null;
      let timeoutId: ReturnType<typeof setTimeout> | null = setTimeout(() => {
        if (!isConvStreaming(useStreamStore.getState().activeStreams, conversationId)) { return; }
        cleanup();
        set((s) => ({
          messages: s.messages.map((m) =>
            m.id === currentMsgId
              ? { ...m, content: i18n.t("agentMode.timeout"), status: "error" as const }
              : m
          ),
        }));
        useStreamStore.setState((s) => ({
          ...stopConversationStream(s.activeStreams, conversationId),
          streamingStartTimestamps: (() => {
            const t = { ...s.streamingStartTimestamps };
            delete t[conversationId];
            return t;
          })(),
        }));
        // Reject the event promise so the await doesn't hang forever
        if (_agentReject) {
          _agentReject(new Error(i18n.t("agentMode.timeout")));
        }
      }, AGENT_TIMEOUT_MS);

      // ── Agent stream buffering (same pattern as Q&A _pendingUiChunk) ──
      let _agentPendingText = "";
      let _agentPendingThinking = "";
      // ── Agent stream buffer & flush (priority-tiered) ──
      //
      // Agent events produce text, thinking, and workflow updates concurrently.
      // Rendering everything at the same 50ms cadence creates unnecessary re-renders
      // for low-urgency content (thinking, workflow steps). We split into two timers:
      //
      //   P1 (text):     50ms flush — user-visible text must feel responsive
      //   P2 (thinking): 200ms flush — thinking is background context, low urgency
      //   P3 (workflow): piggybacks on text flush — no independent timer
      //
      // Tool-call events (P0) are handled by agentStore.ts separately; they trigger
      // immediate UI updates without buffering.

      const AGENT_THINKING_FLUSH_MS = 200;

      let _agentFlushTimer: ReturnType<typeof setTimeout> | null = null;
      let _agentThinkingFlushTimer: ReturnType<typeof setTimeout> | null = null;

      const flushAgentTextChunks = () => {
        if (_agentFlushTimer !== null) {
          clearTimeout(_agentFlushTimer);
          _agentFlushTimer = null;
        }
        const textChunk = _agentPendingText;
        _agentPendingText = "";
        if (!textChunk) { return; }

        // Guard: don't update messages if user switched to a different conversation
        if (get().activeConversationId !== conversationId) { return; }

        set((s) => {
          const wasThinking = useStreamStore.getState().thinkingActiveMessageIds.has(currentMsgId);
          let nextThinkingIds = useStreamStore.getState().thinkingActiveMessageIds;

          const updatedMessages = s.messages.map((m) => {
            if (m.id !== currentMsgId) { return m; }
            let content = m.content || "";

            // Close thinking block if we were in thinking mode
            if (wasThinking) {
              content += "\n</think>\n\n";
              const n = new Set(nextThinkingIds);
              n.delete(currentMsgId);
              nextThinkingIds = n;
            }
            content += textChunk;
            return { ...m, content };
          });

          useStreamStore.setState({ thinkingActiveMessageIds: nextThinkingIds });
          return { messages: updatedMessages };
        });
      };

      const flushAgentThinkingChunks = () => {
        if (_agentThinkingFlushTimer !== null) {
          clearTimeout(_agentThinkingFlushTimer);
          _agentThinkingFlushTimer = null;
        }
        const thinkingChunk = _agentPendingThinking;
        _agentPendingThinking = "";
        if (!thinkingChunk) { return; }

        // Guard: don't update messages if user switched to a different conversation
        if (get().activeConversationId !== conversationId) { return; }

        set((s) => {
          const wasThinking = useStreamStore.getState().thinkingActiveMessageIds.has(currentMsgId);
          let nextThinkingIds = useStreamStore.getState().thinkingActiveMessageIds;

          const updatedMessages = s.messages.map((m) => {
            if (m.id !== currentMsgId) { return m; }
            let content = m.content || "";
            let thinking = m.thinking || "";

            if (!wasThinking) {
              content += '<think data-axagent="1">\n';
            }
            content += thinkingChunk;
            thinking += thinkingChunk;
            nextThinkingIds = new Set([...nextThinkingIds, currentMsgId]);

            return { ...m, content, thinking };
          });

          useStreamStore.setState({ thinkingActiveMessageIds: nextThinkingIds });
          return { messages: updatedMessages };
        });
      };

      const scheduleAgentFlush = () => {
        if (_agentFlushTimer === null) {
          _agentFlushTimer = setTimeout(flushAgentTextChunks, STREAM_UI_FLUSH_INTERVAL_MS);
        }
      };

      const scheduleAgentThinkingFlush = () => {
        if (_agentThinkingFlushTimer === null) {
          _agentThinkingFlushTimer = setTimeout(flushAgentThinkingChunks, AGENT_THINKING_FLUSH_MS);
        }
      };

      const handleWorkflowEvent = (event: WorkflowEvent) => {
        const text = formatWorkflowEventAsText(event);
        if (text) {
          _agentPendingText += text;
          // P3: Workflow events are lazy — they piggyback on the next text/thinking flush.
          // No independent timer; they render when text content triggers a flush.
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
        if (_agentThinkingFlushTimer !== null) {
          clearTimeout(_agentThinkingFlushTimer);
          _agentThinkingFlushTimer = null;
        }
        _agentPendingText = "";
        _agentPendingThinking = "";
      };

      const cleanup = () => {
        clearAgentStreamBuffer();
        if (timeoutId !== null) {
          clearTimeout(timeoutId);
          timeoutId = null;
        }
        unlistenStreamText?.();
        unlistenStreamThinking?.();
        unlistenDone?.();
        unlistenError?.();
        unlistenMessageId?.();
        unlistenWorkflowComplete?.();
        unlistenStatus?.();
        unlistenStreamText = null;
        unlistenStreamThinking = null;
        unlistenDone = null;
        unlistenError = null;
        unlistenMessageId = null;
        unlistenWorkflowComplete = null;
        unlistenStatus = null;
      };

      try {
        const eventPromise = new Promise<void>((resolve, reject) => {
          _agentReject = reject;
          // Listen for the real assistant message ID from the backend
          // This replaces the temp ID so tool call events can be matched
          listen<{ conversationId: string; assistantMessageId: string }>("agent-message-id", (event) => {
            if (event.payload.conversationId !== conversationId) { return; }
            // Flush pending buffers before switching IDs (both text and thinking)
            flushAgentTextChunks();
            flushAgentThinkingChunks();
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
            scheduleAgentThinkingFlush(); // P2: 200ms cadence
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

                  // eslint-disable-next-line @typescript-eslint/no-explicit-any
                  return {
                    ...m,
                    id: event.payload.assistantMessageId || m.id,
                    content: finalContent,
                    thinking: thinkingText || m.thinking,
                    status: "complete" as const,
                    prompt_tokens: event.payload.usage?.input_tokens ?? null,
                    completion_tokens: event.payload.usage?.output_tokens ?? null,
                    blocks: event.payload.blocks ?? m.blocks,
                  } as Message;
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
            // P3: Lazy — piggybacks on next text flush, no independent timer
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
                  // eslint-disable-next-line @typescript-eslint/no-explicit-any
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
            // Preserve the optimistic user message — if agent_query failed before
            // persisting it, fetchMessages would otherwise drop the user's input.
            get().fetchMessages(conversationId, [optimisticUserMsg.id]);
            cleanup();
            reject(new Error(event.payload.message));
          }).then((fn) => {
            unlistenError = fn;
          });
        });

        // Listen for agent status updates — update placeholder message to show progress
        listen<{ conversationId: string; phase: string; message: string }>("agent-status", (event) => {
          if (event.payload.conversationId !== conversationId) { return; }
          // Reset timeout on each status event
          if (timeoutId !== null) { clearTimeout(timeoutId); }
          timeoutId = setTimeout(() => {
            if (!isConvStreaming(useStreamStore.getState().activeStreams, conversationId)) { return; }
            cleanup();
            set((s) => ({
              messages: s.messages.map((m) =>
                m.id === currentMsgId
                  ? { ...m, content: i18n.t("agentMode.timeoutShort"), status: "error" as const }
                  : m
              ),
            }));
            useStreamStore.setState((s) => ({
              ...stopConversationStream(s.activeStreams, conversationId),
              streamingStartTimestamps: (() => {
                const t = { ...s.streamingStartTimestamps };
                delete t[conversationId];
                return t;
              })(),
            }));
            if (_agentReject) {
              _agentReject(new Error(i18n.t("agentMode.timeoutShort")));
            }
          }, AGENT_TIMEOUT_MS);
          set((s) => ({
            messages: s.messages.map((m) =>
              m.id === currentMsgId
                ? { ...m, thinking: `🔄 ${event.payload.message}` }
                : m
            ),
          }));
        }).then((fn) => {
          unlistenStatus = fn;
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
            expertRoleId: conversation.expert_role_id ?? undefined,
            agentProfileId: conversation.agent_profile_id ?? undefined,
            systemPrompt: conversation.system_prompt ?? undefined,
            searchProviderId: searchProviderId ?? undefined,
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

        // Stale guard: user switched conversations while agent was running
        if (get().activeConversationId !== conversationId) { return; }

        // Only set error state if the message doesn't already have an error state
        // (agent-error event listener may have already set it with the backend message)
        const currentMsgs = useConversationStore.getState().messages;
        const msgAlreadyHasError = currentMsgs.some(
          (m) => m.id === currentMsgId && m.status === "error",
        );
        if (msgAlreadyHasError) {
          // agent-error event already handled the failure — no duplicate needed
          return;
        }

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
        // Clean up agent/execution state for this conversation since the send failed.
        // The conversation itself is not being deleted — just the execution attempt failed.
        useAgentStore.getState().clearStatus(conversationId);
        useExecutionStore.getState().clearConversation(conversationId);

        // Sync messages from DB so temp- prefixed user messages get replaced
        // with real backend IDs, enabling regenerate after an agent send failure.
        // Preserve the optimistic user message to prevent it from being dropped
        // when agent_query failed before persisting the user message.
        window.setTimeout(() => {
          void get().fetchMessages(conversationId, [optimisticUserMsg.id]);
        }, 120);
      }
    },

    sendPlanMessage: async (content, attachments = [], _searchProviderId: string | null = null) => {
      const conversationId = get().activeConversationId;
      if (!conversationId) { throw new Error("No active conversation"); }

      const conversation = get().conversations.find((c) => c.id === conversationId);
      if (!conversation) { throw new Error("Conversation not found"); }

      // Guard: prevent duplicate sends while a stream is already active
      if (isConvStreaming(useStreamStore.getState().activeStreams, conversationId)) {
        console.warn("[sendPlanMessage] Ignoring duplicate send — stream already active for", conversationId);
        return;
      }

      const providerId = conversation.provider_id;
      const model_id = conversation.model_id;

      // Optimistic user message
      const optimisticUserMsg: Message = {
        id: tempId("temp-user-"),
        conversation_id: conversationId,
        role: "user",
        content,
        provider_id: null,
        model_id: null,
        token_count: null,
        attachments: attachments.map((a) => ({
          id: tempId("temp-att-"),
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

      // Placeholder assistant message (will be replaced by PlanCard rendering)
      let currentMsgId = `temp-plan-${Date.now()}`;
      const placeholderAssistant: Message = {
        id: currentMsgId,
        conversation_id: conversationId,
        role: "assistant",
        content: i18n.t("agentMode.generatingPlan"),
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

      try {
        // Trigger plan generation on the backend - it emits plan-generated event via SSE
        await invoke("plan_generate", {
          request: { conversationId, content },
        }, 0);

        // Plan generation is async - the plan-generated event will trigger PlanCard rendering
        // End the initial text stream so InputArea unblocks
        useStreamStore.setState((s) => ({
          ...stopConversationStream(s.activeStreams, conversationId),
          streamingStartTimestamps: (() => {
            const t = { ...s.streamingStartTimestamps };
            delete t[conversationId];
            return t;
          })(),
        }));

        // Update placeholder message to indicate plan is ready for review
        set((s) => ({
          messages: s.messages.map((m) =>
            m.id === currentMsgId
              ? { ...m, content: i18n.t("agentMode.planGenerated"), status: "complete" as const }
              : m
          ),
        }));

        // Refresh messages after a short delay to get real IDs
        window.setTimeout(() => {
          void get().fetchMessages(conversationId, [optimisticUserMsg.id]);
        }, 120);
      } catch (e) {
        useStreamStore.setState((s) => ({
          ...stopConversationStream(s.activeStreams, conversationId),
          streamingStartTimestamps: (() => {
            const t = { ...s.streamingStartTimestamps };
            delete t[conversationId];
            return t;
          })(),
        }));
        const errMsg = String(e);
        console.error("[sendPlanMessage] error:", errMsg);
        set((s) => ({
          messages: s.messages.map((m) =>
            m.id === currentMsgId
              ? { ...m, content: errMsg, status: "error" as const }
              : m
          ),
        }));
        // Preserve the optimistic user message — plan_generate doesn't persist it,
        // so fetchMessages without preservation would drop the user's input entirely.
        window.setTimeout(() => {
          void get().fetchMessages(conversationId, [optimisticUserMsg.id]);
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
      const tempAssistantId = tempId("temp-assistant-");
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
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
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
        const rMemNsId = usePreferenceStore.getState().activeMemoryNamespaceId;
        const rMemIds = rMemNsId ? [rMemNsId] : [];
        const rWikiIds = usePreferenceStore.getState().enabledWikiIds;
        await invoke("regenerate_message", {
          conversationId,
          userMessageId: userMsg.id,
          enabledMcpServerIds: rMcpIds.length > 0 ? rMcpIds : undefined,
          thinkingBudget: rThinkingBudget,
          enabledKnowledgeBaseIds: rKbIds.length > 0 ? rKbIds : undefined,
          enabledMemoryNamespaceIds: rMemIds.length > 0 ? rMemIds : undefined,
          enabledWikiIds: rWikiIds.length > 0 ? rWikiIds : undefined,
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
        const currentStreamingMessageId = getStreamingMessageId(
          useStreamStore.getState().activeStreams,
          conversationId,
        );
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
      const tempAssistantId = tempId("temp-assistant-");
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
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
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
        const rMemNsId2 = usePreferenceStore.getState().activeMemoryNamespaceId;
        const rMemIds = rMemNsId2 ? [rMemNsId2] : [];
        const rWikiIds = usePreferenceStore.getState().enabledWikiIds;
        await invoke("regenerate_with_model", {
          conversationId,
          userMessageId: userMsg.id,
          targetProviderId: providerId,
          targetModelId: model_id,
          enabledMcpServerIds: rMcpIds.length > 0 ? rMcpIds : undefined,
          thinkingBudget: rThinkingBudget,
          enabledKnowledgeBaseIds: rKbIds.length > 0 ? rKbIds : undefined,
          enabledMemoryNamespaceIds: rMemIds.length > 0 ? rMemIds : undefined,
          enabledWikiIds: rWikiIds.length > 0 ? rWikiIds : undefined,
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
        const currentStreamingMessageId = getStreamingMessageId(
          useStreamStore.getState().activeStreams,
          conversationId,
        );
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

    sendMultiModelMessage: (content, companionModels, attachments, searchProviderId) => {
      // 委托给 multiModelStore 实现
      return useMultiModelStore.getState().sendMultiModelMessage(
        content,
        companionModels,
        attachments,
        searchProviderId,
      );
    },
  };
}

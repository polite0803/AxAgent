// SPDX-License-Identifier: AGPL-3.0-only

import { invoke, isTauri, listen, logIpcError } from "@/lib/invoke";
import { buildKnowledgeTag, buildMemoryTag, buildWikiTag, type RagContextRetrievedEvent } from "@/lib/memoryUtils";
import type { ChatStreamErrorEvent, ChatStreamEvent } from "@/types";
import type { ConversationState } from "./conversationStore";
import { useConversationStore } from "./conversationStore";
import { usePreferenceStore } from "./preferenceStore";
import {
  _isMultiModelActive,
  _multiModelDoneResolve,
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  _multiModelFirstMessageId,
  _multiModelTotalRemaining,
  _streamBuffer,
  _streamPrefix,
  _streamUiFlushTimer,
  addPendingConversationRefresh,
  appendStreamChunk,
  clearPendingConversationRefresh,
  decrementMultiModelTotalRemaining,
  flushPendingStreamChunk,
  getListenerGen,
  getStreamingMessageId,
  getUnlisten,
  incrementListenerGen,
  resetMultiModelState,
  setMultiModelDoneResolve,
  setMultiModelFirstMessageId,
  setPendingUiChunk,
  setStreamBuffer,
  setStreamPrefix,
  setStreamUiFlushTimer,
  setUnlisten,
  stopConversationStream,
  useStreamStore,
} from "./streamStore";

export interface EventMethods {
  startStreamListening: () => Promise<void>;
  stopStreamListening: () => void;
  cancelCurrentStream: () => void;
}

export function createEventMethods(
  set: (
    partial:
      | Partial<ConversationState>
      | ((s: ConversationState) => Partial<ConversationState>),
  ) => void,
  get: () => ConversationState,
): EventMethods {
  return {
    startStreamListening: async () => {
      // Increment generation and clean up previous listeners
      const gen = incrementListenerGen();
      const currentUnlisten = getUnlisten();
      if (currentUnlisten) {
        currentUnlisten();
        setUnlisten(null);
      }

      const [chunkUnsub, errorUnsub, titleUnsub, titleGenUnsub, ragUnsub] = await Promise.all([
        listen<ChatStreamEvent>("chat-stream-chunk", (event) => {
          if (getListenerGen() !== gen) {
            return; // stale listener
          }
          if (!useStreamStore.getState().streaming) {
            return; // cancelled
          }
          const {
            conversation_id,
            message_id,
            chunk,
            model_id: evt_model_id,
            provider_id: evt_provider_id,
          } = event.payload;

          if (typeof conversation_id !== "string" || !conversation_id) {
            return;
          }

          if (chunk.done) {
            if (chunk.is_final === false) {
              // Append any remaining content in the done chunk (e.g. closing </think> tag)
              if (chunk.content) {
                appendStreamChunk(
                  set,
                  get,
                  message_id,
                  chunk.content,
                  conversation_id,
                  evt_model_id,
                  evt_provider_id,
                );
              }
              flushPendingStreamChunk(set, get);
              // Clear thinking state — this iteration is done
              if (
                useStreamStore
                  .getState()
                  .thinkingActiveMessageIds.has(message_id)
              ) {
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
              const streamUpdates: {
                streamingMessageId?: string | null;
                thinkingActiveMessageIds?: Set<string>;
              } = {};
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
                  c.id === conversation_id
                    ? { ...c, message_count: c.message_count + 1 }
                    : c
                );
                // Update completed message status to prevent "主动停止" tag
                updated.messages = s.messages.map((m) => m.id === message_id ? { ...m, status: "complete" } : m);
                // Track per-model completion for individual loading indicators
                updated.multiModelDoneMessageIds = [
                  ...s.multiModelDoneMessageIds,
                  message_id,
                ];
                return updated;
              });

              if (_multiModelTotalRemaining <= 0) {
                // All models done
                useStreamStore.setState((s) => ({
                  ...stopConversationStream(s.activeStreams, conversation_id),
                  streamingStartTimestamps: (() => {
                    const t = { ...s.streamingStartTimestamps };
                    delete t[conversation_id];
                    return t;
                  })(),
                  thinkingActiveMessageIds: new Set<string>(),
                }));
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
                  (value): value is string =>
                    typeof value === "string"
                    && value.length > 0
                    && !value.startsWith("temp-"),
                ),
              ),
            );
            useStreamStore.setState((s) => ({
              // Must use stopConversationStream to ALSO clean up activeStreams,
              // otherwise InputArea sees the stale entry and keeps the stop button.
              ...stopConversationStream(s.activeStreams, conversation_id),
              streamingStartTimestamps: (() => {
                const t = { ...s.streamingStartTimestamps };
                delete t[conversation_id];
                return t;
              })(),
              thinkingActiveMessageIds: new Set<string>(),
            }));
            set((s) => ({
              conversations: s.conversations.map((c) =>
                c.id === conversation_id
                  ? { ...c, message_count: c.message_count + 1 }
                  : c
              ),
              // Update completed message status immediately to prevent "主动停止" tag flash
              messages: s.messages.map((m) =>
                preserveMessageIds.includes(m.id)
                  ? { ...m, status: "complete" as const }
                  : m
              ),
            }));
            if (get().activeConversationId === conversation_id) {
              // Active conversation — refresh messages then clear buffer
              setStreamBuffer(null);
              window.setTimeout(() => {
                void get().fetchMessages(conversation_id, preserveMessageIds);
              }, 120);
            } else {
              // User is viewing a different conversation — keep buffer alive and
              // schedule a refresh so the completed message loads from DB when
              // the user switches back.
              addPendingConversationRefresh(conversation_id);
            }

            // Auto incremental memory extraction after stream completes
            // Delayed + staggered to avoid competing with the main LLM for API quota.
            // Skip entirely if an agent execution is still active for this conversation.
            Promise.all([
              import("@/lib/invoke"),
              import("@/stores/feature/executionStore"),
            ])
              .then(([{ invoke }, { useExecutionStore }]) => {
                const isAgentActive = useExecutionStore
                  .getState()
                  .isActive(conversation_id);
                if (isAgentActive) {
                  return;
                }

                const scheduledConvId = conversation_id;
                const memNsId = usePreferenceStore.getState().activeMemoryNamespaceId;
                setTimeout(() => {
                  const currentConvId = useConversationStore.getState().activeConversationId;
                  if (currentConvId !== scheduledConvId) {
                    return;
                  }
                  const stillActive = useExecutionStore
                    .getState()
                    .isActive(conversation_id);
                  if (stillActive) {
                    return;
                  }
                  void invoke("auto_extract_incremental_memories", {
                    conversationId: conversation_id,
                    namespaceId: memNsId ?? null,
                  }).catch(logIpcError("auto_extract_memories"));
                }, 5000);
                setTimeout(() => {
                  const currentConvId = useConversationStore.getState().activeConversationId;
                  if (currentConvId !== scheduledConvId) {
                    return;
                  }
                  const stillActive = useExecutionStore
                    .getState()
                    .isActive(conversation_id);
                  if (stillActive) {
                    return;
                  }
                  void invoke("extract_conversation_entities", {
                    conversationId: conversation_id,
                  }).catch(logIpcError("extract_entities"));
                }, 8000);
              })
              .catch(logIpcError("dynamic_import_invoke"));

            return;
          }

          if (
            chunk.thinking !== undefined
            && chunk.thinking !== null
            && !useStreamStore
              .getState()
              .thinkingActiveMessageIds.has(message_id)
          ) {
            useStreamStore.setState((s) => ({
              thinkingActiveMessageIds: new Set([
                ...s.thinkingActiveMessageIds,
                message_id,
              ]),
            }));
          }
          if (
            chunk.content
            && useStreamStore
              .getState()
              .thinkingActiveMessageIds.has(message_id)
            && (chunk.thinking === undefined || chunk.thinking === null)
          ) {
            useStreamStore.setState((s) => {
              const next = new Set(s.thinkingActiveMessageIds);
              next.delete(message_id);
              return { thinkingActiveMessageIds: next };
            });
          }

          appendStreamChunk(
            set,
            get,
            message_id,
            chunk.content,
            conversation_id,
            evt_model_id,
            evt_provider_id,
          );
        }),
        listen<ChatStreamErrorEvent>("chat-stream-error", (event) => {
          if (getListenerGen() !== gen) {
            return; // stale listener
          }
          if (!useStreamStore.getState().streaming) {
            return; // cancelled
          }
          const {
            conversation_id,
            message_id,
            error: errMsg,
          } = event.payload;

          flushPendingStreamChunk(set, get);
          setStreamBuffer(null); // Clear buffer on error

          // Multi-model: treat error as stream completion for this model
          if (_isMultiModelActive) {
            decrementMultiModelTotalRemaining();
            logIpcError("multi-model.streamError")(errMsg);
            // Mark this model as done so ModelTags stops showing loading indicator.
            // Include error message in content so the user sees diagnostic info.
            set((s) => ({
              multiModelDoneMessageIds: [
                ...s.multiModelDoneMessageIds,
                message_id,
              ],
              messages: s.messages.map((m) =>
                m.id === message_id
                  ? {
                    ...m,
                    content: errMsg || m.content,
                    status: "error" as const,
                  }
                  : m
              ),
            }));
            if (_multiModelTotalRemaining <= 0) {
              useStreamStore.setState((s) => ({
                ...stopConversationStream(s.activeStreams, conversation_id),
                streamingStartTimestamps: (() => {
                  const t = { ...s.streamingStartTimestamps };
                  delete t[conversation_id];
                  return t;
                })(),
                thinkingActiveMessageIds: new Set<string>(),
              }));
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
            useStreamStore.setState((s) => ({
              ...stopConversationStream(s.activeStreams, conversation_id),
              streamingStartTimestamps: (() => {
                const t = { ...s.streamingStartTimestamps };
                delete t[conversation_id];
                return t;
              })(),
              thinkingActiveMessageIds: new Set<string>(),
            }));
            return;
          }

          // Update the streaming message to show error inline
          const currentStreamingMessageId = useStreamStore.getState().streamingMessageId;
          useStreamStore.setState((s) => ({
            ...stopConversationStream(s.activeStreams, conversation_id),
            streamingStartTimestamps: (() => {
              const t = { ...s.streamingStartTimestamps };
              delete t[conversation_id];
              return t;
            })(),
            thinkingActiveMessageIds: new Set<string>(),
          }));
          set((s) => ({
            messages: s.messages.map((m) =>
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
        }),
        listen<{ conversation_id: string; title: string }>(
          "conversation-title-updated",
          (event) => {
            if (getListenerGen() !== gen) {
              return;
            }
            const { conversation_id, title } = event.payload;
            set((s) => ({
              conversations: s.conversations.map((c) => c.id === conversation_id ? { ...c, title } : c),
            }));
          },
        ),
        listen<{
          conversation_id: string;
          generating: boolean;
          error: string | null;
        }>("conversation-title-generating", (event) => {
          if (getListenerGen() !== gen) {
            return;
          }
          const { conversation_id, generating, error } = event.payload;
          set({
            titleGeneratingConversationId: generating
              ? conversation_id
              : null,
          });
          if (!generating && error) {
            logIpcError("title-gen")(error);
            set({ error });
          }
        }),
        listen<RagContextRetrievedEvent>("rag-context-retrieved", (event) => {
          if (getListenerGen() !== gen) {
            return;
          }
          if (!useStreamStore.getState().streaming) {
            return;
          }
          const { conversation_id, sources } = event.payload;

          // Split sources by type and build separate tags
          const knowledgeSources = sources.filter(
            (s) => s.source_type === "knowledge",
          );
          const memorySources = sources.filter(
            (s) => s.source_type === "memory",
          );
          const wikiSources = sources.filter((s) => s.source_type === "wiki");

          const kbSearching = buildKnowledgeTag("searching");
          const memSearching = buildMemoryTag("searching");
          const wikiSearching = buildWikiTag("searching");
          const kbDone = knowledgeSources.length > 0
            ? buildKnowledgeTag("done", knowledgeSources)
            : "";
          const memDone = memorySources.length > 0
            ? buildMemoryTag("done", memorySources)
            : "";
          const wikiDone = wikiSources.length > 0 ? buildWikiTag("done", wikiSources) : "";

          // Replace each searching tag with its done counterpart (or remove if empty)
          const replaceTag = (
            content: string,
            searching: string,
            done: string,
          ) => {
            if (content.includes(searching)) {
              return content.replace(searching, done);
            }
            if (done) {
              return done + content;
            }
            return content;
          };

          if (
            _streamBuffer
            && _streamBuffer.conversationId === conversation_id
          ) {
            const buf = _streamBuffer;
            setStreamBuffer({
              ...buf,
              content: replaceTag(
                replaceTag(
                  replaceTag(buf.content, kbSearching, kbDone),
                  memSearching,
                  memDone,
                ),
                wikiSearching,
                wikiDone,
              ),
            });
          } else {
            setStreamPrefix(
              replaceTag(
                replaceTag(
                  replaceTag(_streamPrefix, kbSearching, kbDone),
                  memSearching,
                  memDone,
                ),
                wikiSearching,
                wikiDone,
              ),
            );
          }

          // Update UI immediately
          if (get().activeConversationId === conversation_id) {
            const msgId = useStreamStore.getState().streamingMessageId;
            if (msgId) {
              set((s) => ({
                messages: s.messages.map((m) => {
                  if (m.id !== msgId) {
                    return m;
                  }
                  let updated = m.content;
                  updated = replaceTag(updated, kbSearching, kbDone);
                  updated = replaceTag(updated, memSearching, memDone);
                  updated = replaceTag(updated, wikiSearching, wikiDone);
                  return { ...m, content: updated };
                }),
              }));
            }
          }
        }),
      ]);

      // If generation changed while awaiting, this listener set is stale
      if (getListenerGen() !== gen) {
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
      const currentUnlisten = getUnlisten();
      if (currentUnlisten) {
        currentUnlisten();
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
        set({
          pendingCompanionModels: [],
          multiModelParentId: null,
          multiModelDoneMessageIds: [],
        });
      }
      if (_streamUiFlushTimer !== null) {
        clearTimeout(_streamUiFlushTimer);
        setStreamUiFlushTimer(null);
      }
      // Tell the backend to cancel the stream — fire and forget
      const streamState = useStreamStore.getState();
      const activeConvId = get().activeConversationId;
      const conversationId = (activeConvId && activeConvId in streamState.activeStreams)
        ? activeConvId
        : streamState.streamingConversationId;
      if (conversationId && isTauri()) {
        invoke("cancel_stream", { conversationId }).catch(logIpcError("cancel_stream"));
        // Also cancel the agent if in agent mode
        const conv = get().conversations.find((c) => c.id === conversationId);
        if (conv?.mode === "agent") {
          invoke("agent_cancel", { request: { conversationId } }).catch(
            logIpcError("agent_cancel"),
          );
        }
      }
      if (!conversationId) {
        return;
      }
      // Mark the current streaming message as partial
      const streamMsgId = getStreamingMessageId(
        streamState.activeStreams,
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
      if (streamMsgId) {
        set((s) => ({
          messages: s.messages.map((m) => m.id === streamMsgId ? { ...m, status: "partial" as const } : m),
        }));
      }
    },
  };
}

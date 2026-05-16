import i18n from "@/i18n";
import { invoke, isTauri, type UnlistenFn } from "@/lib/invoke";
import type { Conversation, Message } from "@/types";
import { create } from "zustand";

// ─── Module-level variables (exported for use by conversationStore) ───

/** Tauri event unlisten handle for the current stream listener set */
export let _unlisten: UnlistenFn | null = null;
/** Generation counter to prevent stale listeners from processing events
 *  (fixes React StrictMode double-effect causing duplicate stream processing) */
export let _listenerGen = 0;

// ─── 卡住的流看门狗 ───

// 流超过此毫秒数无更新则视为卡住，自动取消
const STUCK_STREAM_TIMEOUT_MS = 5 * 60 * 1000; // 5 分钟
/** 看门狗检查间隔 */
// 看门狗检查间隔: 30 秒
const WATCHDOG_INTERVAL_MS = 30 * 1000;

let _watchdogTimer: ReturnType<typeof setInterval> | null = null;

// 启动流看门狗：定期检查 streamingStartTimestamps，
// 超过 STUCK_STREAM_TIMEOUT_MS 未结束则自动取消并标记错误。
export function startStreamWatchdog() {
  if (_watchdogTimer !== null) { return; }

  _watchdogTimer = setInterval(() => {
    const state = useStreamStore.getState();
    const now = Date.now();
    const stuckConversationIds: string[] = [];

    for (const [convId, startTime] of Object.entries(state.streamingStartTimestamps)) {
      if (now - startTime > STUCK_STREAM_TIMEOUT_MS) {
        stuckConversationIds.push(convId);
      }
    }

    for (const convId of stuckConversationIds) {
      console.warn(
        `[StreamWatchdog] stream stuck: conversationId=${convId}, running ${
          Math.round((now - state.streamingStartTimestamps[convId]) / 1000)
        }s, auto-cancelling`,
      );

      const msgId = state.activeStreams[convId];
      if (msgId && _conversationStoreRef) {
        _conversationStoreRef.setState((s) => ({
          messages: s.messages.map((m: Message) =>
            m.id === msgId && m.status === "partial"
              ? {
                ...m,
                content: m.content + "\n\n> " + i18n.t("stream.timeout"),
                status: "error" as const,
              }
              : m
          ),
        }));
      }

      state.cancelCurrentStream(convId);
    }
  }, WATCHDOG_INTERVAL_MS);
}

// 停止流看门狗（应用退出时调用）
export function stopStreamWatchdog() {
  if (_watchdogTimer !== null) {
    clearInterval(_watchdogTimer);
    _watchdogTimer = null;
  }
}

// ─── Stream buffer ───

/** Buffer for streaming content — persists across conversation switches
 *  so chunks arriving while viewing another conversation aren't lost */
export interface StreamBuffer {
  messageId: string;
  conversationId: string;
  content: string;
  /** The real message ID resolved from the backend (may differ from initial placeholder) */
  resolvedId: string | null;
  /** Accumulated thinking/reasoning content */
  thinking: string | null;
}

/**
 * Preserved buffers from conversations the user navigated away from while streaming.
 * When switching back, these are used instead of the live buffer (which may have
 * been hijacked by a new send in a different conversation).
 * Keyed by conversationId.
 */
const _orphanedBuffers = new Map<string, StreamBuffer>();

/** Conversations whose stream completed while the user was viewing a different
 *  conversation.  When the user switches back we trigger a fetchMessages so the
 *  final AI response is loaded from the backend. */
export const _pendingConversationRefresh = new Set<string>();

// ─── UI flush batching ───
//
// Performance tuning (2026-04-30):
// - FLUSH_INTERVAL: 16→50ms (~20fps instead of ~60fps). Text streaming doesn't
//   benefit from 60fps; human reading speed is the bottleneck. This cuts
//   React setState calls by ~67% while maintaining perceived smoothness.
// - MAX_CHUNK_SIZE: 100→500 chars. Fewer but larger updates reduce the
//   React reconciliation overhead per batch. 500 chars ≈ one paragraph,
//   which maps naturally to human reading cadence.

export const STREAM_UI_FLUSH_INTERVAL_MS = 50;
export const STREAM_MAX_CHUNK_SIZE = 500;

let _flushTimer: ReturnType<typeof setTimeout> | null = null;

export function cancelScheduledFlush() {
  if (_flushTimer !== null) {
    clearTimeout(_flushTimer);
    _flushTimer = null;
  }
}

export interface PendingUiChunk {
  messageId: string;
  conversationId: string;
  content: string;
  model_id?: string;
  providerId?: string;
}

// ─── Per-conversation stream session state ───
//
// Encapsulates all mutable state that was previously stored in module-level
// variables.  Each conversation gets its own session so that concurrent streams
// don't corrupt each other's data (race-condition fix).

const DEFAULT_SESSION_KEY = "__default__";

export interface StreamSessionState {
  streamBuffer: StreamBuffer | null;
  streamPrefix: string;
  pendingUiChunk: PendingUiChunk | null;
  streamUiFlushTimer: ReturnType<typeof setTimeout> | null;
  isMultiModelActive: boolean;
  multiModelTotalRemaining: number;
  multiModelDoneResolve: (() => void) | null;
  multiModelFirstModelId: string | null;
  multiModelFirstMessageId: string | null;
  userManuallySelectedVersion: boolean;
  multiModelBuffers: Map<string, StreamBuffer>;
}

const _streamSessions = new Map<string, StreamSessionState>();

function resolveSessionKey(conversationId?: string): string {
  return conversationId ?? DEFAULT_SESSION_KEY;
}

export function getOrCreateSession(conversationId: string): StreamSessionState {
  let session = _streamSessions.get(conversationId);
  if (!session) {
    session = {
      streamBuffer: null,
      streamPrefix: "",
      pendingUiChunk: null,
      streamUiFlushTimer: null,
      isMultiModelActive: false,
      multiModelTotalRemaining: 0,
      multiModelDoneResolve: null,
      multiModelFirstModelId: null,
      multiModelFirstMessageId: null,
      userManuallySelectedVersion: false,
      multiModelBuffers: new Map(),
    };
    _streamSessions.set(conversationId, session);
  }
  return session;
}

export function removeSession(conversationId: string): void {
  const session = _streamSessions.get(conversationId);
  if (session) {
    if (session.streamUiFlushTimer !== null) {
      clearTimeout(session.streamUiFlushTimer);
    }
    _streamSessions.delete(conversationId);
  }
}

// ─── Backward-compatible exported variables ───
// These mirror the default session's state for callers that don't pass
// conversationId.  Prefer using session-based APIs (getOrCreateSession) in
// new code.

export let _streamBuffer: StreamBuffer | null = null;
export let _streamPrefix = "";
export let _pendingUiChunk: PendingUiChunk | null = null;
export let _streamUiFlushTimer: ReturnType<typeof setTimeout> | null = null;

/** Monotonically increasing counter to ignore stale fetchMessages after conversation switch */
export let _activeMessageLoadSeq = 0;

// ─── Message index for O(1) streaming updates ───
//
// During streaming, we receive chunks every ~50ms that update a single message.
// Without an index, each update does a full messages.find() + messages.map() — O(n).
// With this index, we do a direct array slot mutation in O(1), saving significant
// CPU in long conversations (1000+ messages).
//
// The index is rebuilt on every full message replacement (fetchMessages, setActiveConversation)
// and remains valid as long as only the streaming message is being mutated in place.

/** Maps messageId → its position in the conversationStore.messages array.
 *  Rebuilt on full message loads; consulted during streaming for O(1) lookups. */
const _messageIndex = new Map<string, number>();

/** Rebuild the index from a fresh messages array. Call after any operation
 *  that replaces the entire messages list (fetchMessages, switchConversation). */
export function rebuildMessageIndex(messages: ReadonlyArray<{ id: string }>) {
  _messageIndex.clear();
  for (let i = 0; i < messages.length; i++) {
    _messageIndex.set(messages[i].id, i);
  }
}

/** Update the index after a message ID changes (e.g. temp- placeholder → real ID). */
function updateMessageIdInIndex(oldId: string, newId: string) {
  const idx = _messageIndex.get(oldId);
  if (idx !== undefined) {
    _messageIndex.delete(oldId);
    _messageIndex.set(newId, idx);
  }
}

// ─── Multi-model parallel tracking (exported for conversationStore) ───

/** Counts ALL models (including first) */
export let _multiModelTotalRemaining = 0;
export let _multiModelDoneResolve: (() => void) | null = null;
export let _isMultiModelActive = false;
/** model_id of the first selected model (for auto-switch) */
export let _multiModelFirstModelId: string | null = null;
/** actual DB message_id of the first model's response */
export let _multiModelFirstMessageId: string | null = null;
/** tracks if user manually switched version during multi-model streaming */
export let _userManuallySelectedVersion = false;

// ─── Setter functions for module-level variables ───
// conversationStore needs to mutate these from outside this module.
// All setters accept an optional conversationId to target a specific session.
// When omitted, the default session AND the backward-compatible exported
// variable are updated.

export function setUnlisten(value: UnlistenFn | null) {
  _unlisten = value;
}
export function incrementListenerGen() {
  return ++_listenerGen;
}
export function setStreamBuffer(value: StreamBuffer | null, conversationId?: string) {
  const key = resolveSessionKey(conversationId);
  if (value) {
    getOrCreateSession(key).streamBuffer = value;
  } else {
    const session = _streamSessions.get(key);
    if (session) { session.streamBuffer = value; }
  }
  if (key === DEFAULT_SESSION_KEY) {
    _streamBuffer = value;
  }
}
export function setStreamPrefix(value: string, conversationId?: string) {
  const key = resolveSessionKey(conversationId);
  getOrCreateSession(key).streamPrefix = value;
  if (key === DEFAULT_SESSION_KEY) {
    _streamPrefix = value;
  }
}
export function clearPendingConversationRefresh() {
  _pendingConversationRefresh.clear();
}
export function addPendingConversationRefresh(id: string) {
  _pendingConversationRefresh.add(id);
}
export function deletePendingConversationRefresh(id: string) {
  _pendingConversationRefresh.delete(id);
}
export function setPendingUiChunk(value: PendingUiChunk | null, conversationId?: string) {
  const key = resolveSessionKey(conversationId);
  if (value) {
    getOrCreateSession(key).pendingUiChunk = value;
  } else {
    const session = _streamSessions.get(key);
    if (session) { session.pendingUiChunk = value; }
  }
  if (key === DEFAULT_SESSION_KEY) {
    _pendingUiChunk = value;
  }
}
export function setStreamUiFlushTimer(value: ReturnType<typeof setTimeout> | null, conversationId?: string) {
  const key = resolveSessionKey(conversationId);
  const session = _streamSessions.get(key);
  if (session) { session.streamUiFlushTimer = value; }
  if (key === DEFAULT_SESSION_KEY) {
    _streamUiFlushTimer = value;
  }
}
export function incrementActiveMessageLoadSeq() {
  return ++_activeMessageLoadSeq;
}

export function setMultiModelTotalRemaining(value: number, conversationId?: string) {
  const key = resolveSessionKey(conversationId);
  getOrCreateSession(key).multiModelTotalRemaining = value;
  if (key === DEFAULT_SESSION_KEY) {
    _multiModelTotalRemaining = value;
  }
}
export function decrementMultiModelTotalRemaining(conversationId?: string) {
  const key = resolveSessionKey(conversationId);
  const session = _streamSessions.get(key);
  if (session) {
    session.multiModelTotalRemaining--;
    if (key === DEFAULT_SESSION_KEY) {
      _multiModelTotalRemaining = session.multiModelTotalRemaining;
    }
  }
}
export function setMultiModelDoneResolve(value: (() => void) | null, conversationId?: string) {
  const key = resolveSessionKey(conversationId);
  const session = _streamSessions.get(key);
  if (session) { session.multiModelDoneResolve = value; }
  if (key === DEFAULT_SESSION_KEY) {
    _multiModelDoneResolve = value;
  }
}
export function setIsMultiModelActive(value: boolean, conversationId?: string) {
  const key = resolveSessionKey(conversationId);
  getOrCreateSession(key).isMultiModelActive = value;
  if (key === DEFAULT_SESSION_KEY) {
    _isMultiModelActive = value;
  }
}
export function setMultiModelFirstModelId(value: string | null, conversationId?: string) {
  const key = resolveSessionKey(conversationId);
  const session = _streamSessions.get(key);
  if (session) { session.multiModelFirstModelId = value; }
  if (key === DEFAULT_SESSION_KEY) {
    _multiModelFirstModelId = value;
  }
}
export function setMultiModelFirstMessageId(value: string | null, conversationId?: string) {
  const key = resolveSessionKey(conversationId);
  const session = _streamSessions.get(key);
  if (session) { session.multiModelFirstMessageId = value; }
  if (key === DEFAULT_SESSION_KEY) {
    _multiModelFirstMessageId = value;
  }
}
export function setUserManuallySelectedVersion(value: boolean, conversationId?: string) {
  const key = resolveSessionKey(conversationId);
  const session = _streamSessions.get(key);
  if (session) { session.userManuallySelectedVersion = value; }
  if (key === DEFAULT_SESSION_KEY) {
    _userManuallySelectedVersion = value;
  }
}

/** Reset all multi-model module-level state to defaults */
export function resetMultiModelState(conversationId?: string) {
  const key = resolveSessionKey(conversationId);
  const session = _streamSessions.get(key);
  if (session) {
    session.isMultiModelActive = false;
    session.multiModelTotalRemaining = 0;
    session.multiModelFirstModelId = null;
    session.multiModelFirstMessageId = null;
    session.userManuallySelectedVersion = false;
    session.multiModelDoneResolve = null;
  }
  if (key === DEFAULT_SESSION_KEY) {
    _isMultiModelActive = false;
    _multiModelTotalRemaining = 0;
    _multiModelFirstModelId = null;
    _multiModelFirstMessageId = null;
    _userManuallySelectedVersion = false;
    _multiModelDoneResolve = null;
  }
}

export function preserveOrphanedBuffer(conversationId?: string) {
  const key = resolveSessionKey(conversationId);
  const session = _streamSessions.get(key);
  if (session && session.streamBuffer && session.streamBuffer.content) {
    _orphanedBuffers.set(session.streamBuffer.conversationId, { ...session.streamBuffer });
  }
}

export function takeOrphanedBuffer(conversationId: string): StreamBuffer | undefined {
  const buf = _orphanedBuffers.get(conversationId);
  if (buf) {
    _orphanedBuffers.delete(conversationId);
  }
  return buf;
}

export function clearOrphanedBuffer(conversationId: string) {
  _orphanedBuffers.delete(conversationId);
}

// 完整重置所有模块级可变状态。仅供测试使用。
// 这些变量放在模块级而非 Zustand store 中，
// 因为流式处理每 50ms 到达一个 chunk，
// Zustand 不可变更新周期会增加 CPU 开销。
// resetStreamRuntime 保证测试隔离。
export function resetStreamRuntime() {
  _unlisten?.();
  _unlisten = null;
  _listenerGen = 0;
  _streamBuffer = null;
  _streamPrefix = "";
  _pendingConversationRefresh.clear();
  cancelScheduledFlush();
  _pendingUiChunk = null;
  if (_streamUiFlushTimer !== null) {
    clearTimeout(_streamUiFlushTimer);
    _streamUiFlushTimer = null;
  }
  _activeMessageLoadSeq = 0;
  _messageIndex.clear();
  _conversationStoreRef = null;
  resetMultiModelState();
  _orphanedBuffers.clear();
  for (const session of _streamSessions.values()) {
    if (session.streamUiFlushTimer !== null) {
      clearTimeout(session.streamUiFlushTimer);
    }
  }
  _streamSessions.clear();
}

// ─── Generic type for set/get that can update messages ───
// These helpers need to update `messages` which lives in conversationStore,
// so they accept set/get typed generically rather than tied to StreamState.

type GenericSet<T> = (fn: (s: T) => Partial<T>) => void;
type GenericGet<T> = () => T;

// Minimal shape required by the helper functions from the calling store
interface ConversationStoreLike {
  activeConversationId: string | null;
  messages: Message[];
  multiModelParentId: string | null;
  streamingMessageId: string | null;
}

// ─── Helper functions ───

function scheduleFlush(set: GenericSet<unknown>, get: GenericGet<unknown>, conversationId?: string) {
  if (_flushTimer === null) {
    _flushTimer = setTimeout(() => {
      _flushTimer = null;
      flushPendingStreamChunk(
        set as GenericSet<ConversationStoreLike>,
        get as GenericGet<ConversationStoreLike>,
        conversationId,
      );
    }, 0);
  }
}

export function appendStreamChunk<T extends ConversationStoreLike>(
  set: GenericSet<T>,
  get: GenericGet<T>,
  messageId: string,
  content: string | null,
  conversationId: string,
  model_id?: string,
  providerId?: string,
) {
  const session = getOrCreateSession(conversationId);

  // 单模型模式用会话缓冲区，多模型模式用 per-messageId 缓冲防止互相覆盖
  if (!session.isMultiModelActive) {
    if (!session.streamBuffer || session.streamBuffer.conversationId !== conversationId) {
      // Preserve the previous conversation's buffer before overwriting
      if (
        session.streamBuffer && session.streamBuffer.conversationId !== conversationId && session.streamBuffer.content
      ) {
        preserveOrphanedBuffer(session.streamBuffer.conversationId);
      }
      session.streamBuffer = {
        messageId,
        conversationId,
        content: session.streamPrefix,
        resolvedId: null,
        thinking: null,
      };
      session.streamPrefix = ""; // consumed
    }
    session.streamBuffer.content += content ?? "";
    // Track ID resolution (placeholder → real ID)
    if (session.streamBuffer.messageId !== messageId && !session.streamBuffer.resolvedId) {
      session.streamBuffer.resolvedId = messageId;
    }
  } else {
    // 多模型模式：每个 messageId 独立缓冲
    let buf = session.multiModelBuffers.get(messageId);
    if (!buf) {
      buf = { messageId, conversationId, content: session.streamPrefix, resolvedId: null, thinking: null };
      session.streamPrefix = "";
      session.multiModelBuffers.set(messageId, buf);
    }
    buf.content += content ?? "";
    if (buf.messageId !== messageId && !buf.resolvedId) {
      buf.resolvedId = messageId;
    }
  }

  // Sync backward-compatible exported variables
  _streamBuffer = session.streamBuffer;
  _streamPrefix = session.streamPrefix;
  _isMultiModelActive = session.isMultiModelActive;

  // Only update messages in UI if this is the active conversation
  if (get().activeConversationId !== conversationId) { return; }

  if (
    session.pendingUiChunk && (
      session.pendingUiChunk.conversationId !== conversationId
      || session.pendingUiChunk.messageId !== messageId
    )
  ) {
    flushPendingStreamChunk(set, get, conversationId);
  }

  if (!session.pendingUiChunk) {
    session.pendingUiChunk = {
      messageId,
      conversationId,
      content: "",
      model_id,
      providerId,
    };
  }

  session.pendingUiChunk.content += content ?? "";
  _pendingUiChunk = session.pendingUiChunk;

  const contentLength = session.pendingUiChunk.content.length;
  if (contentLength >= STREAM_MAX_CHUNK_SIZE) {
    flushPendingStreamChunk(set, get, conversationId);
  } else if (session.streamUiFlushTimer === null) {
    session.streamUiFlushTimer = setTimeout(() => {
      session.streamUiFlushTimer = null;
      scheduleFlush(set as GenericSet<unknown>, get as GenericGet<unknown>, conversationId);
    }, STREAM_UI_FLUSH_INTERVAL_MS);
    _streamUiFlushTimer = session.streamUiFlushTimer;
  }
}

export function flushPendingStreamChunk<T extends ConversationStoreLike>(
  set: GenericSet<T>,
  get: GenericGet<T>,
  conversationId?: string,
) {
  const key = resolveSessionKey(conversationId);
  const session = _streamSessions.get(key);
  if (!session) { return; }

  if (session.streamUiFlushTimer !== null) {
    clearTimeout(session.streamUiFlushTimer);
    session.streamUiFlushTimer = null;
  }
  cancelScheduledFlush();

  const pending = session.pendingUiChunk;
  session.pendingUiChunk = null;
  _pendingUiChunk = null;
  _streamUiFlushTimer = null;
  if (!pending) { return; }

  const { messageId, content, conversationId: chunkConvId, model_id: chunkModelId, providerId: chunkProviderId } =
    pending;
  if (get().activeConversationId !== chunkConvId) { return; }

  set((s) => {
    // 1. Direct ID match via index — O(1) append to existing message
    const existingIdx = _messageIndex.get(messageId);
    if (existingIdx !== undefined && existingIdx < s.messages.length && s.messages[existingIdx].id === messageId) {
      const updated = [...s.messages];
      updated[existingIdx] = {
        ...updated[existingIdx],
        content: updated[existingIdx].content + (content ?? ""),
        // Enrich model info from chunk if missing
        model_id: updated[existingIdx].model_id ?? chunkModelId ?? null,
        provider_id: updated[existingIdx].provider_id ?? chunkProviderId ?? null,
      };
      return {
        messages: updated,
      } as Partial<T>;
    }
    // Fallback: index miss — do a linear scan (should be rare, only on index invalidation)
    const existing = s.messages.find((m) => m.id === messageId);
    if (existing) {
      return {
        messages: s.messages.map((m) =>
          m.id === messageId
            ? {
              ...m,
              content: m.content + (content ?? ""),
              model_id: m.model_id ?? chunkModelId ?? null,
              provider_id: m.provider_id ?? chunkProviderId ?? null,
            }
            : m
        ),
      } as Partial<T>;
    }

    // 2. ID mismatch but placeholder exists — replace placeholder ID with real one
    // In multi-model mode, only resolve temp-* placeholders (first model's initial
    // chunk resolving the placeholder to its real DB ID). Once resolved,
    // streamingMessageId is a real ID and companion chunks must NOT hijack it —
    // they fall through to case 3 and create their own message entries.
    if (s.streamingMessageId && s.streamingMessageId !== messageId) {
      if (!session.isMultiModelActive || s.streamingMessageId.startsWith("temp-")) {
        const placeholderIdx = _messageIndex.get(s.streamingMessageId);
        if (
          placeholderIdx !== undefined && placeholderIdx < s.messages.length
          && s.messages[placeholderIdx].id === s.streamingMessageId
        ) {
          const updated = [...s.messages];
          updated[placeholderIdx] = {
            ...updated[placeholderIdx],
            id: messageId,
            content: updated[placeholderIdx].content + (content ?? ""),
          };
          updateMessageIdInIndex(s.streamingMessageId, messageId);
          return {
            messages: updated,
            streamingMessageId: messageId,
          } as Partial<T>;
        }
        // Fallback: linear scan
        const placeholder = s.messages.find((m) => m.id === s.streamingMessageId);
        if (placeholder) {
          return {
            messages: s.messages.map((m) =>
              m.id === s.streamingMessageId
                ? {
                  ...m,
                  id: messageId,
                  content: m.content + (content ?? ""),
                }
                : m
            ),
            streamingMessageId: messageId,
          } as Partial<T>;
        }
      }
    }

    // 3. No placeholder found — create new assistant message with full buffered content
    const isMultiModel = session.isMultiModelActive;
    const bufferContent = isMultiModel
      ? session.multiModelBuffers.get(messageId)?.content
      : session.streamBuffer?.content;
    const newMessage: Message = {
      id: messageId,
      conversation_id: chunkConvId,
      role: "assistant",
      content: bufferContent ?? (content ?? ""),
      provider_id: chunkProviderId ?? null,
      model_id: chunkModelId ?? null,
      token_count: null,
      attachments: [],
      thinking: null,
      tool_calls_json: null,
      tool_call_id: null,
      created_at: Date.now(),
      // In multi-model mode: group under the same parent and hide from main view
      // (only ModelTags pending animation is shown; fetchMessages after completion loads proper data)
      parent_message_id: isMultiModel ? s.multiModelParentId : null,
      version_index: 0,
      is_active: !isMultiModel,
      status: "partial",
    };
    // Register the new message in our index for O(1) future updates
    _messageIndex.set(messageId, s.messages.length);
    return {
      messages: [...s.messages, newMessage],
      // Don't overwrite streamingMessageId in multi-model mode
      streamingMessageId: isMultiModel ? s.streamingMessageId : messageId,
    } as Partial<T>;
  });
}

// ─── Lazy reference to conversationStore (avoids circular import) ───
// Set by conversationStore during its initialization.

interface ConversationStoreRefState {
  activeConversationId: string | null;
  conversations: Conversation[];
  messages: Message[];
  streamingMessageId: string | null;
  multiModelParentId: string | null;
  pendingCompanionModels: Array<{ providerId: string; model_id: string }>;
  multiModelDoneMessageIds: string[];
}

interface ConversationStoreRef {
  getState: () => ConversationStoreRefState;
  setState: (
    partial:
      | Partial<ConversationStoreRefState>
      | ((state: ConversationStoreRefState) => Partial<ConversationStoreRefState>),
  ) => void;
}

let _conversationStoreRef: ConversationStoreRef | null = null;

/** Register the conversationStore reference so cancelCurrentStream can call it.
 *  Called once by conversationStore during module initialization. */
export function registerConversationStoreRef(ref: ConversationStoreRef) {
  _conversationStoreRef = ref;
}

// ─── Multi-conversation stream tracking ───

/**
 * Derive legacy fields from activeStreams and return a partial state update.
 * When no conversations are streaming: { streaming: false, streamingMessageId: null, streamingConversationId: null }
 * When one is streaming: copies that conversation's values.
 * When multiple are streaming: streamingMessageId is from the FIRST active stream.
 */
export function deriveLegacyStreamFields(activeStreams: Record<string, string>) {
  const convIds = Object.keys(activeStreams);
  const streaming = convIds.length > 0;
  if (streaming) {
    const firstConvId = convIds[0];
    return {
      streaming: true,
      streamingMessageId: activeStreams[firstConvId],
      streamingConversationId: firstConvId,
    };
  }
  return {
    streaming: false,
    streamingMessageId: null,
    streamingConversationId: null,
  };
}

/** Start streaming for a conversation. Returns a partial state update. */
export function startConversationStream(
  activeStreams: Record<string, string>,
  conversationId: string,
  messageId: string,
) {
  const updated = { ...activeStreams, [conversationId]: messageId };
  return {
    activeStreams: updated,
    ...deriveLegacyStreamFields(updated),
  };
}

/** Stop streaming for a conversation (if it was active). Returns a partial state update. */
export function stopConversationStream(
  activeStreams: Record<string, string>,
  conversationId: string,
) {
  if (!(conversationId in activeStreams)) { return { activeStreams }; }
  const { [conversationId]: _removed, ...rest } = activeStreams;
  return {
    activeStreams: rest,
    ...deriveLegacyStreamFields(rest),
  };
}

export function getStreamingMessageId(
  activeStreams: Record<string, string>,
  conversationId: string,
): string | null {
  return activeStreams[conversationId] ?? null;
}

export function isConversationStreaming(
  activeStreams: Record<string, string>,
  conversationId: string,
): boolean {
  return conversationId in activeStreams;
}

// ─── Stream Store ───

interface StreamState {
  /** Per-conversation streaming state: conversationId → messageId */
  activeStreams: Record<string, string>;
  /** Legacy: true iff any conversation is streaming */
  streaming: boolean;
  /** Legacy: messageId of the first active stream */
  streamingMessageId: string | null;
  /** Legacy: conversationId of the first active stream */
  streamingConversationId: string | null;
  /** Per-conversation start timestamps for stuck recovery */
  streamingStartTimestamps: Record<string, number>;
  thinkingActiveMessageIds: Set<string>;
  stopStreamListening: () => void;
  cancelCurrentStream: (conversationId?: string) => void;
  isConversationStreaming: (conversationId: string) => boolean;
}

export const useStreamStore = create<StreamState>((set, get) => ({
  activeStreams: {},
  streaming: false,
  streamingMessageId: null,
  streamingConversationId: null,
  streamingStartTimestamps: {},
  thinkingActiveMessageIds: new Set<string>(),

  stopStreamListening: () => {
    _listenerGen++;
    if (_unlisten) {
      _unlisten();
      _unlisten = null;
    }
  },

  cancelCurrentStream: (conversationId?: string) => {
    const convRef = _conversationStoreRef;
    const state = get();
    const activeConvId = conversationId ?? state.streamingConversationId ?? convRef?.getState().activeConversationId;

    // If no specific conversation, cancel ALL active streams
    if (!activeConvId) { return; }

    const session = _streamSessions.get(activeConvId);

    // Flush pending UI chunk through conversationStore if available
    if (convRef) {
      flushPendingStreamChunk(
        (fn) => {
          convRef.setState(fn(convRef.getState()));
        },
        () => convRef.getState(),
        activeConvId,
      );
    }

    // Clean up session state
    if (session) {
      session.pendingUiChunk = null;
      session.streamBuffer = null;
      // 清理被取消对话的多模型缓冲
      if (session.isMultiModelActive) {
        for (const [msgId, buf] of session.multiModelBuffers) {
          if (buf.conversationId === activeConvId) {
            session.multiModelBuffers.delete(msgId);
          }
        }
      }
    }

    // Sync backward-compatible variables
    _pendingUiChunk = null;
    _streamBuffer = null;

    // Only clear the pending refresh for the conversation being cancelled,
    // not all conversations (other conversations may have completed streams
    // that need to be loaded when the user switches back to them).
    if (activeConvId) {
      _pendingConversationRefresh.delete(activeConvId);
    }

    // Clean up multi-model state on cancel.
    // Must resolve _multiModelDoneResolve BEFORE resetMultiModelState sets it to null,
    // otherwise sendMultiModelMessage hangs forever at `await allDone`.
    if (session?.isMultiModelActive) {
      if (session.multiModelDoneResolve) {
        const r = session.multiModelDoneResolve;
        session.multiModelDoneResolve = null;
        r();
      }
      resetMultiModelState(activeConvId);
      if (convRef) {
        convRef.setState({ pendingCompanionModels: [], multiModelParentId: null, multiModelDoneMessageIds: [] });
      }
    }

    if (session && session.streamUiFlushTimer !== null) {
      clearTimeout(session.streamUiFlushTimer);
      session.streamUiFlushTimer = null;
    }
    _streamUiFlushTimer = null;

    // Tell the backend to cancel the stream — fire and forget
    if (isTauri()) {
      invoke("cancel_stream", { conversationId: activeConvId }).catch((e: unknown) => {
        console.warn("[IPC]", e);
      });
      // Also cancel the agent if in agent mode
      const conv = convRef?.getState().conversations?.find((c) => c.id === activeConvId);
      if (conv?.mode === "agent") {
        invoke("agent_cancel", { request: { conversationId: activeConvId } }).catch((e: unknown) => {
          console.warn("[IPC]", e);
        });
      }
    }

    // Mark the message as partial
    const streamMsgId = getStreamingMessageId(state.activeStreams, activeConvId);
    const { activeStreams, streamingStartTimestamps, thinkingActiveMessageIds } = get();
    const { [activeConvId]: _msgId, ...restStreams } = activeStreams;
    const { [activeConvId]: _ts, ...restTimestamps } = streamingStartTimestamps;
    // Only remove thinking indicators for the cancelled conversation's messages,
    // leaving other conversations' thinking state intact.
    const updatedThinking = new Set(thinkingActiveMessageIds);
    if (_msgId) { updatedThinking.delete(_msgId); }
    set({
      activeStreams: restStreams,
      ...deriveLegacyStreamFields(restStreams),
      streamingStartTimestamps: restTimestamps,
      thinkingActiveMessageIds: updatedThinking,
    });

    if (streamMsgId && convRef) {
      convRef.setState((s) => ({
        messages: s.messages.map((m: Message) => m.id === streamMsgId ? { ...m, status: "partial" as const } : m),
      }));
    }

    // Remove the session after all cleanup is done
    removeSession(activeConvId);
  },

  isConversationStreaming: (conversationId: string) => {
    return conversationId in get().activeStreams;
  },
}));

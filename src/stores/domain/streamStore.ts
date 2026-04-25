import { create } from 'zustand';
import { invoke, type UnlistenFn, isTauri } from '@/lib/invoke';
import type { Message } from '@/types';

// ─── Module-level variables (exported for use by conversationStore) ───

/** Tauri event unlisten handle for the current stream listener set */
export let _unlisten: UnlistenFn | null = null;
/** Generation counter to prevent stale listeners from processing events
 *  (fixes React StrictMode double-effect causing duplicate stream processing) */
export let _listenerGen = 0;

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

export let _streamBuffer: StreamBuffer | null = null;
/**
 * Preserved buffers from conversations the user navigated away from while streaming.
 * When switching back, these are used instead of the live buffer (which may have
 * been hijacked by a new send in a different conversation).
 * Keyed by conversationId.
 */
const _orphanedBuffers = new Map<string, StreamBuffer>();

export function preserveOrphanedBuffer() {
  if (_streamBuffer && _streamBuffer.content) {
    _orphanedBuffers.set(_streamBuffer.conversationId, { ..._streamBuffer });
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
/** Prefix injected before streaming content (e.g., search result tags) */
export let _streamPrefix = '';
/** Conversations whose stream completed while the user was viewing a different
 *  conversation.  When the user switches back we trigger a fetchMessages so the
 *  final AI response is loaded from the backend. */
export const _pendingConversationRefresh = new Set<string>();

// ─── UI flush batching ───

export const STREAM_UI_FLUSH_INTERVAL_MS = 16;
export const STREAM_MAX_CHUNK_SIZE = 100;

let _rafId: number | null = null;
let _needsFlush = false;

function scheduleFlush(set: GenericSet<unknown>, get: GenericGet<unknown>) {
  _needsFlush = true;
  if (_rafId === null) {
    _rafId = requestAnimationFrame(() => {
      _rafId = null;
      if (_needsFlush) {
        _needsFlush = false;
        flushPendingStreamChunk(set as GenericSet<ConversationStoreLike>, get as GenericGet<ConversationStoreLike>);
      }
    });
  }
}

export function cancelScheduledFlush() {
  _needsFlush = false;
  if (_rafId !== null) {
    cancelAnimationFrame(_rafId);
    _rafId = null;
  }
}

export interface PendingUiChunk {
  messageId: string;
  conversationId: string;
  content: string;
  model_id?: string;
  providerId?: string;
}

export let _pendingUiChunk: PendingUiChunk | null = null;
export let _streamUiFlushTimer: ReturnType<typeof setTimeout> | null = null;

/** Monotonically increasing counter to ignore stale fetchMessages after conversation switch */
export let _activeMessageLoadSeq = 0;

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

export function setUnlisten(value: UnlistenFn | null) { _unlisten = value; }
export function incrementListenerGen() { return ++_listenerGen; }
export function setStreamBuffer(value: StreamBuffer | null) { _streamBuffer = value; }
export function setStreamPrefix(value: string) { _streamPrefix = value; }
export function clearPendingConversationRefresh() { _pendingConversationRefresh.clear(); }
export function addPendingConversationRefresh(id: string) { _pendingConversationRefresh.add(id); }
export function deletePendingConversationRefresh(id: string) { _pendingConversationRefresh.delete(id); }
export function setPendingUiChunk(value: PendingUiChunk | null) { _pendingUiChunk = value; }
export function setStreamUiFlushTimer(value: ReturnType<typeof setTimeout> | null) { _streamUiFlushTimer = value; }
export function incrementActiveMessageLoadSeq() { return ++_activeMessageLoadSeq; }

export function setMultiModelTotalRemaining(value: number) { _multiModelTotalRemaining = value; }
export function decrementMultiModelTotalRemaining() { _multiModelTotalRemaining--; }
export function setMultiModelDoneResolve(value: (() => void) | null) { _multiModelDoneResolve = value; }
export function setIsMultiModelActive(value: boolean) { _isMultiModelActive = value; }
export function setMultiModelFirstModelId(value: string | null) { _multiModelFirstModelId = value; }
export function setMultiModelFirstMessageId(value: string | null) { _multiModelFirstMessageId = value; }
export function setUserManuallySelectedVersion(value: boolean) { _userManuallySelectedVersion = value; }

/** Reset all multi-model module-level state to defaults */
export function resetMultiModelState() {
  _isMultiModelActive = false;
  _multiModelTotalRemaining = 0;
  _multiModelFirstModelId = null;
  _multiModelFirstMessageId = null;
  _userManuallySelectedVersion = false;
  _multiModelDoneResolve = null;
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

export function appendStreamChunk<T extends ConversationStoreLike>(
  set: GenericSet<T>,
  get: GenericGet<T>,
  messageId: string,
  content: string | null,
  conversationId: string,
  model_id?: string,
  providerId?: string,
) {

  // Accumulate into stream buffer only in single-stream mode
  // (parallel multi-model streams would corrupt the shared buffer)
  if (!_isMultiModelActive) {
    if (!_streamBuffer || _streamBuffer.conversationId !== conversationId) {
      // Preserve the previous conversation's buffer before overwriting
      if (_streamBuffer && _streamBuffer.conversationId !== conversationId && _streamBuffer.content) {
        preserveOrphanedBuffer();
      }
      _streamBuffer = { messageId, conversationId, content: _streamPrefix, resolvedId: null, thinking: null };
      _streamPrefix = ''; // consumed
    }
    _streamBuffer.content += content ?? '';
    // Track ID resolution (placeholder → real ID)
    if (_streamBuffer.messageId !== messageId && !_streamBuffer.resolvedId) {
      _streamBuffer.resolvedId = messageId;
    }
  }

  // Only update messages in UI if this is the active conversation
  if (get().activeConversationId !== conversationId) return;

  if (_pendingUiChunk && (
    _pendingUiChunk.conversationId !== conversationId
    || _pendingUiChunk.messageId !== messageId
  )) {
    flushPendingStreamChunk(set, get);
  }

  if (!_pendingUiChunk) {
    _pendingUiChunk = {
      messageId,
      conversationId,
      content: '',
      model_id,
      providerId,
    };
  }

  _pendingUiChunk.content += content ?? '';

  const contentLength = _pendingUiChunk.content.length;
  if (contentLength >= STREAM_MAX_CHUNK_SIZE) {
    flushPendingStreamChunk(set, get);
  } else if (_streamUiFlushTimer === null) {
    _streamUiFlushTimer = setTimeout(() => {
      _streamUiFlushTimer = null;
      scheduleFlush(set as GenericSet<unknown>, get as GenericGet<unknown>);
    }, STREAM_UI_FLUSH_INTERVAL_MS);
  }
}

export function flushPendingStreamChunk<T extends ConversationStoreLike>(
  set: GenericSet<T>,
  get: GenericGet<T>,
) {
  if (_streamUiFlushTimer !== null) {
    clearTimeout(_streamUiFlushTimer);
    _streamUiFlushTimer = null;
  }
  cancelScheduledFlush();

  const pending = _pendingUiChunk;
  _pendingUiChunk = null;
  if (!pending) return;

  const { messageId, content, conversationId, model_id: chunkModelId, providerId: chunkProviderId } = pending;
  if (get().activeConversationId !== conversationId) return;

  set((s) => {
    // 1. Direct ID match — append to existing message
    const existing = s.messages.find((m) => m.id === messageId);
    if (existing) {
      return {
        messages: s.messages.map((m) =>
          m.id === messageId
            ? {
                ...m,
                content: m.content + (content ?? ''),
                // Enrich model info from chunk if missing
                model_id: m.model_id ?? chunkModelId ?? null,
                provider_id: m.provider_id ?? chunkProviderId ?? null,
              }
            : m,
        ),
      } as Partial<T>;
    }

    // 2. ID mismatch but placeholder exists — replace placeholder ID with real one
    // In multi-model mode, only resolve temp-* placeholders (first model's initial
    // chunk resolving the placeholder to its real DB ID). Once resolved,
    // streamingMessageId is a real ID and companion chunks must NOT hijack it —
    // they fall through to case 3 and create their own message entries.
    if (s.streamingMessageId && s.streamingMessageId !== messageId) {
      if (!_isMultiModelActive || s.streamingMessageId.startsWith('temp-')) {
        const placeholder = s.messages.find((m) => m.id === s.streamingMessageId);
        if (placeholder) {
          return {
            messages: s.messages.map((m) =>
              m.id === s.streamingMessageId
                ? {
                    ...m,
                    id: messageId,
                    content: m.content + (content ?? ''),
                  }
                : m,
            ),
            streamingMessageId: messageId,
          } as Partial<T>;
        }
      }
    }

    // 3. No placeholder found — create new assistant message with full buffered content
    const isMultiModel = _isMultiModelActive;
    const newMessage: Message = {
      id: messageId,
      conversation_id: conversationId,
      role: 'assistant',
      content: _streamBuffer?.content ?? (content ?? ''),
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
      status: 'partial',
    };
    return {
      messages: [...s.messages, newMessage],
      // Don't overwrite streamingMessageId in multi-model mode
      streamingMessageId: isMultiModel ? s.streamingMessageId : messageId,
    } as Partial<T>;
  });
}

// ─── Lazy reference to conversationStore (avoids circular import) ───
// Set by conversationStore during its initialization.
let _conversationStoreRef: {
  getState: () => any;
  setState: (partial: any) => void;
} | null = null;

/** Register the conversationStore reference so cancelCurrentStream can call it.
 *  Called once by conversationStore during module initialization. */
export function registerConversationStoreRef(ref: typeof _conversationStoreRef) {
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
  if (!(conversationId in activeStreams)) return { activeStreams };
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
    if (!activeConvId) return;

    // Flush pending UI chunk through conversationStore if available
    if (convRef) {
      flushPendingStreamChunk(
        (fn) => { convRef.setState(fn(convRef.getState())); },
        () => convRef.getState(),
      );
    }

    _pendingUiChunk = null;
    _streamBuffer = null;
    _pendingConversationRefresh.clear();

    // Clean up multi-model state on cancel
    if (_isMultiModelActive) {
      resetMultiModelState();
      if (convRef) {
        convRef.setState({ pendingCompanionModels: [], multiModelParentId: null, multiModelDoneMessageIds: [] });
      }
    }

    if (_streamUiFlushTimer !== null) {
      clearTimeout(_streamUiFlushTimer);
      _streamUiFlushTimer = null;
    }

    // Tell the backend to cancel the stream — fire and forget
    if (isTauri()) {
      invoke('cancel_stream', { conversationId: activeConvId }).catch(() => {});
      // Also cancel the agent if in agent mode
      const conv = convRef?.getState().conversations?.find((c: any) => c.id === activeConvId);
      if (conv?.mode === 'agent') {
        invoke('agent_cancel', { request: { conversationId: activeConvId } }).catch(() => {});
      }
    }

    // Mark the message as partial
    const streamMsgId = getStreamingMessageId(state.activeStreams, activeConvId);
    const { activeStreams, streamingStartTimestamps } = get();
    const { [activeConvId]: _msgId, ...restStreams } = activeStreams;
    const { [activeConvId]: _ts, ...restTimestamps } = streamingStartTimestamps;
    set({
      activeStreams: restStreams,
      ...deriveLegacyStreamFields(restStreams),
      streamingStartTimestamps: restTimestamps,
      thinkingActiveMessageIds: new Set<string>(),
    });

    if (streamMsgId && convRef) {
      convRef.setState((s: any) => ({
        messages: s.messages.map((m: Message) =>
          m.id === streamMsgId ? { ...m, status: 'partial' as const } : m
        ),
      }));
    }
  },

  isConversationStreaming: (conversationId: string) => {
    return conversationId in get().activeStreams;
  },
}));

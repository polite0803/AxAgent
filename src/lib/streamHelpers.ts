/**
 * Stream processing helper utilities.
 *
 * Consolidates common patterns used across sendMessage, sendAgentMessage,
 * regenerateMessage, and regenerateWithModel to reduce code duplication
 * and ensure consistent error handling.
 */
import { useStreamStore } from "@/stores/domain/streamStore";
import {
  _streamUiFlushTimer,
  setPendingUiChunk,
  setStreamBuffer,
  setStreamUiFlushTimer,
} from "@/stores/domain/streamStore";
import type { Message } from "@/types";

// ─── Types ───

export interface OptimisticMessages {
  userMessage: Message;
  assistantMessage: Message;
  tempAssistantId: string;
}

export interface StreamErrorState {
  conversationId: string;
  tempAssistantId: string;
  errorMessage: string;
}

// ─── Stream setup / teardown helpers ───

/**
 * Create an optimistic user message with temp- prefix.
 */
export function createOptimisticUserMessage(
  conversationId: string,
  content: string,
  attachments: { file_name: string; file_type: string; file_size?: number; data?: string }[] = [],
): Message {
  return {
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
      file_size: a.file_size ?? 0,
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
}

/**
 * Create a placeholder assistant message with temp- prefix.
 */
export function createPlaceholderAssistant(
  conversationId: string,
  parentId: string,
  content = "",
  providerId?: string | null,
  modelId?: string | null,
): Message {
  return {
    id: `temp-assistant-${Date.now()}`,
    conversation_id: conversationId,
    role: "assistant",
    content,
    provider_id: providerId ?? null,
    model_id: modelId ?? null,
    token_count: null,
    attachments: [],
    thinking: null,
    tool_calls_json: null,
    tool_call_id: null,
    created_at: Date.now(),
    parent_message_id: parentId,
    version_index: 0,
    is_active: true,
    status: "partial",
  };
}

/**
 * Initialize streaming state in both conversationStore and streamStore.
 * Clears any pending UI chunk and flush timer to prevent stale state.
 */
export function initStreamingState(
  conversationId: string,
  tempAssistantId: string,
  setMessages: (messages: Message[]) => void,
  newMessages: Message[],
) {
  // Clear any pending flush state from previous streams
  setPendingUiChunk(null);
  if (_streamUiFlushTimer !== null) {
    clearTimeout(_streamUiFlushTimer);
    setStreamUiFlushTimer(null);
  }
  setStreamBuffer(null);

  // Update messages and streaming state atomically
  setMessages(newMessages);
  useStreamStore.setState({
    streaming: true,
    streamingConversationId: conversationId,
    streamingMessageId: tempAssistantId,
    thinkingActiveMessageIds: new Set<string>(),
  });
}

/**
 * Handle a streaming error — updates the placeholder message with error
 * content and resets streaming state.
 */
export function handleStreamError(
  state: StreamErrorState,
  setMessages: (messages: Message[]) => void,
  currentMessages: Message[],
) {
  useStreamStore.setState({
    streaming: false,
    streamingMessageId: null,
    streamingConversationId: null,
    thinkingActiveMessageIds: new Set<string>(),
  });

  setMessages(
    currentMessages.map((m) =>
      m.id === state.tempAssistantId
        ? { ...m, content: state.errorMessage, status: "error" as const }
        : m
    ),
  );
}

/**
 * Replace the active assistant message for a given parent, deactivating
 * the old one and inserting a new placeholder version.
 */
export function replaceActiveVersion(
  messages: Message[],
  parentId: string,
  placeholder: Message,
): { updatedMessages: Message[]; inserted: boolean } {
  let inserted = false;
  const updated: Message[] = [];
  for (const m of messages) {
    if (m.parent_message_id === parentId && m.is_active) {
      updated.push({ ...m, is_active: false });
      if (!inserted) {
        updated.push(placeholder);
        inserted = true;
      }
    } else {
      updated.push(m);
    }
  }
  if (!inserted) {
    updated.push(placeholder);
  }
  return { updatedMessages: updated, inserted };
}

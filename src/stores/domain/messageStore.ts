/**
 * messageStore.ts — 消息管理 Zustand Store
 *
 * 管理消息的发送、流式接收、分页加载、版本切换等。
 * 与 conversationStore 协作：conversationStore 保留 messages 等状态字段以保证向后兼容，
 * messageStore 提供辅助函数和独立的 store 实例。
 */

import { invoke } from "@/lib/invoke";
import type { Message, MessagePage } from "@/types";
import { create } from "zustand";

// ─── Constants ───

export const MESSAGE_PAGE_SIZE = 50;

// ─── Helper functions ───

export function mergePreservedMessages(
  pageMessages: Message[],
  preserveMessageIds: string[],
  currentMessages: Message[],
): Message[] {
  if (preserveMessageIds.length === 0) {
    return pageMessages;
  }

  const merged = new Map(pageMessages.map((message) => [message.id, message]));
  for (const messageId of preserveMessageIds) {
    const localMessage = currentMessages.find((message) => message.id === messageId);
    if (localMessage) {
      const dbMessage = merged.get(messageId);
      if (dbMessage) {
        merged.set(messageId, {
          ...dbMessage,
          content: localMessage.content,
          status: localMessage.status,
        });
      } else {
        merged.set(messageId, localMessage);
      }
    }
  }

  return Array.from(merged.values()).sort(
    (left, right) => left.created_at - right.created_at || left.id.localeCompare(right.id),
  );
}

export function mergeOlderPages(olderMessages: Message[], currentMessages: Message[]): Message[] {
  const merged = new Map<string, Message>();
  for (const message of olderMessages) {
    merged.set(message.id, message);
  }
  for (const message of currentMessages) {
    merged.set(message.id, message);
  }
  return Array.from(merged.values()).sort(
    (left, right) => left.created_at - right.created_at || left.id.localeCompare(right.id),
  );
}

// ─── Message Store State ───

interface MessageState {
  /** 当前会话的消息列表（独立副本，与 conversationStore 并行维护） */
  messages: Message[];
  /** 是否正在加载消息 */
  loading: boolean;
  /** 是否正在加载更早的消息 */
  loadingOlder: boolean;
  /** 是否有更早的消息可加载 */
  hasOlderMessages: boolean;
  /** 当前会话的总消息数 */
  totalActiveCount: number;
  /** 已加载的最早消息 ID */
  oldestLoadedMessageId: string | null;
  /** 错误信息 */
  error: string | null;

  /** 加载消息（分页） */
  loadMessages: (conversationId: string, preserveMessageIds?: string[]) => Promise<Message[]>;
  /** 加载更早的消息 */
  loadOlderMessages: (
    conversationId: string,
    oldestLoadedMessageId: string,
  ) => Promise<{ messages: Message[]; hasOlder: boolean; totalActiveCount: number; oldestId: string | null }>;
  /** 搜索消息 */
  searchMessages: (conversationId: string, query: string) => Promise<Message[]>;
  /** 删除消息 */
  deleteMessage: (messageId: string) => Promise<void>;
  /** 删除消息组（用户消息及其所有回复） */
  deleteMessageGroup: (conversationId: string, userMessageId: string) => Promise<void>;
}

export const useMessageStore = create<MessageState>((set) => ({
  messages: [],
  loading: false,
  loadingOlder: false,
  hasOlderMessages: false,
  totalActiveCount: 0,
  oldestLoadedMessageId: null,
  error: null,

  loadMessages: async (conversationId, preserveMessageIds = []) => {
    set({ loading: true });
    try {
      const page = await invoke<MessagePage>("list_messages_page", {
        conversationId,
        limit: MESSAGE_PAGE_SIZE,
        beforeMessageId: null,
      });

      const messages = mergePreservedMessages(page.messages, preserveMessageIds, []);
      set({
        messages,
        loading: false,
        loadingOlder: false,
        hasOlderMessages: page.has_older,
        totalActiveCount: page.total_active_count,
        oldestLoadedMessageId: messages[0]?.id ?? page.oldest_message_id,
        error: null,
      });
      return messages;
    } catch (e) {
      const errorMessage = String(e);
      set({ error: errorMessage, loading: false, loadingOlder: false });
      throw e;
    }
  },

  loadOlderMessages: async (conversationId, oldestLoadedMessageId) => {
    set({ loadingOlder: true, error: null });
    try {
      const page = await invoke<MessagePage>("list_messages_page", {
        conversationId,
        limit: MESSAGE_PAGE_SIZE,
        beforeMessageId: oldestLoadedMessageId,
      });

      set((s) => ({
        messages: mergeOlderPages(page.messages, s.messages),
        loadingOlder: false,
        hasOlderMessages: page.has_older,
        totalActiveCount: page.total_active_count,
        oldestLoadedMessageId: page.oldest_message_id ?? s.oldestLoadedMessageId,
        error: null,
      }));
      return {
        messages: page.messages,
        hasOlder: page.has_older,
        totalActiveCount: page.total_active_count,
        oldestId: page.oldest_message_id,
      };
    } catch (e) {
      set({ error: String(e), loadingOlder: false });
      throw e;
    }
  },

  searchMessages: async (conversationId, query) => {
    try {
      const page = await invoke<MessagePage>("search_messages", {
        conversationId,
        query,
        limit: MESSAGE_PAGE_SIZE,
        beforeMessageId: null,
      });
      set({ messages: page.messages });
      return page.messages;
    } catch (e) {
      set({ error: String(e) });
      throw e;
    }
  },

  deleteMessage: async (messageId) => {
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

  deleteMessageGroup: async (conversationId, userMessageId) => {
    if (userMessageId.startsWith("temp-")) {
      set((s) => ({
        messages: s.messages.filter(
          (m) => m.id !== userMessageId && m.parent_message_id !== userMessageId,
        ),
      }));
      return;
    }
    try {
      await invoke("delete_message_group", { conversationId, userMessageId });
      set((s) => ({
        messages: s.messages.filter(
          (m) => m.id !== userMessageId && m.parent_message_id !== userMessageId,
        ),
      }));
    } catch (e) {
      set({ error: String(e) });
    }
  },
}));

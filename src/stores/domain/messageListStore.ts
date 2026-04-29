import { invoke } from "@/lib/invoke";
import type { Message, MessagePage } from "@/types";
import { create } from "zustand";
import { MESSAGE_PAGE_SIZE, mergeOlderPages, mergePreservedMessages } from "./messageStore";

interface MessageListState {
  messages: Message[];
  loading: boolean;
  loadingOlder: boolean;
  hasOlderMessages: boolean;
  totalActiveCount: number;
  oldestLoadedMessageId: string | null;
  error: string | null;

  fetchMessages: (conversationId: string, preserveMessageIds?: string[]) => Promise<void>;
  loadOlderMessages: () => Promise<void>;
  deleteMessage: (messageId: string) => Promise<void>;
  deleteMessageGroup: (conversationId: string, userMessageId: string) => Promise<void>;
  updateMessageContent: (messageId: string, content: string) => Promise<void>;
  switchMessageVersion: (conversationId: string, parentMessageId: string, messageId: string) => Promise<void>;
  listMessageVersions: (conversationId: string, parentMessageId: string) => Promise<Message[]>;
  setMessages: (messages: Message[]) => void;
  appendMessage: (message: Message) => void;
  updateMessage: (messageId: string, updates: Partial<Message>) => void;
  removeMessage: (messageId: string) => void;
  clearMessages: () => void;
  clearError: () => void;
}

export const useMessageListStore = create<MessageListState>((set, get) => ({
  messages: [],
  loading: false,
  loadingOlder: false,
  hasOlderMessages: false,
  totalActiveCount: 0,
  oldestLoadedMessageId: null,
  error: null,

  fetchMessages: async (conversationId, preserveMessageIds) => {
    set({ loading: true, error: null });
    try {
      const page = await invoke<MessagePage>("list_messages", {
        conversationId,
        limit: MESSAGE_PAGE_SIZE,
        beforeId: null,
      });
      const messages = preserveMessageIds
        ? mergePreservedMessages(page.messages, preserveMessageIds, get().messages)
        : page.messages;
      set({
        messages,
        hasOlderMessages: page.has_older,
        oldestLoadedMessageId: messages.length > 0 ? messages[0].id : null,
        totalActiveCount: page.total_active_count,
        loading: false,
      });
    } catch (e: any) {
      set({ error: String(e), loading: false });
    }
  },

  loadOlderMessages: async () => {
    const { oldestLoadedMessageId, loadingOlder, hasOlderMessages } = get();
    if (loadingOlder || !hasOlderMessages || !oldestLoadedMessageId) return;

    set({ loadingOlder: true });
    try {
      const activeConvId = (await import("./conversationStore")).useConversationStore
        .getState()
        .activeConversationId;
      if (!activeConvId) return;

      const page = await invoke<MessagePage>("list_messages", {
        conversationId: activeConvId,
        limit: MESSAGE_PAGE_SIZE,
        beforeId: oldestLoadedMessageId,
      });
      set((state) => ({
        messages: mergeOlderPages(state.messages, page.messages),
        hasOlderMessages: page.has_older,
        oldestLoadedMessageId: page.messages.length > 0 ? page.messages[0].id : state.oldestLoadedMessageId,
        loadingOlder: false,
      }));
    } catch (e: any) {
      set({ error: String(e), loadingOlder: false });
    }
  },

  deleteMessage: async (messageId) => {
    try {
      await invoke("delete_message", { id: messageId });
      set((state) => ({
        messages: state.messages.filter((m) => m.id !== messageId),
      }));
    } catch (e: any) {
      set({ error: String(e) });
    }
  },

  deleteMessageGroup: async (conversationId, userMessageId) => {
    try {
      await invoke("delete_message_group", { conversation_id: conversationId, user_message_id: userMessageId });
      set((state) => ({
        messages: state.messages.filter(
          (m) => m.id !== userMessageId && m.parent_message_id !== userMessageId,
        ),
      }));
    } catch (e: any) {
      set({ error: String(e) });
    }
  },

  updateMessageContent: async (messageId, content) => {
    try {
      await invoke("update_message_content", { id: messageId, content });
      set((state) => ({
        messages: state.messages.map((m) =>
          m.id === messageId ? { ...m, content } : m,
        ),
      }));
    } catch (e: any) {
      set({ error: String(e) });
    }
  },

  switchMessageVersion: async (conversationId, parentMessageId, messageId) => {
    try {
      await invoke("switch_message_version", { conversation_id: conversationId, parent_message_id: parentMessageId, message_id: messageId });
      const page = await invoke<MessagePage>("list_messages", {
        conversationId,
        limit: MESSAGE_PAGE_SIZE,
        beforeId: null,
      });
      set({ messages: page.messages });
    } catch (e: any) {
      set({ error: String(e) });
    }
  },

  listMessageVersions: async (conversationId, parentMessageId) => {
    try {
      return await invoke<Message[]>("list_message_versions", {
        conversationId,
        parentMessageId,
      });
    } catch (e: any) {
      set({ error: String(e) });
      return [];
    }
  },

  setMessages: (messages) => set({ messages }),

  appendMessage: (message) => {
    set((state) => ({
      messages: [...state.messages, message],
    }));
  },

  updateMessage: (messageId, updates) => {
    set((state) => ({
      messages: state.messages.map((m) =>
        m.id === messageId ? { ...m, ...updates } : m,
      ),
    }));
  },

  removeMessage: (messageId) => {
    set((state) => ({
      messages: state.messages.filter((m) => m.id !== messageId),
    }));
  },

  clearMessages: () => set({ messages: [], hasOlderMessages: false, oldestLoadedMessageId: null }),

  clearError: () => set({ error: null }),
}));

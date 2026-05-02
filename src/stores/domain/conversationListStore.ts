import { invoke } from "@/lib/invoke";
import type { Conversation, ConversationSearchResult } from "@/types";
import { create } from "zustand";

interface ConversationListState {
  conversations: Conversation[];
  archivedConversations: Conversation[];
  totalActiveCount: number;
  loading: boolean;
  error: string | null;

  fetchConversations: () => Promise<void>;
  fetchArchivedConversations: () => Promise<void>;
  searchConversations: (query: string) => Promise<ConversationSearchResult[]>;
  renameConversation: (id: string, title: string) => Promise<void>;
  deleteConversation: (id: string) => Promise<void>;
  togglePin: (id: string) => Promise<void>;
  toggleArchive: (id: string) => Promise<void>;
  archiveToKnowledgeBase: (id: string, knowledgeBaseId: string) => Promise<void>;
  batchDelete: (ids: string[]) => Promise<void>;
  batchArchive: (ids: string[]) => Promise<void>;
  setConversations: (conversations: Conversation[]) => void;
  updateConversationInList: (id: string, updates: Partial<Conversation>) => void;
  removeConversationFromList: (id: string) => void;
  addConversationToList: (conversation: Conversation) => void;
  clearError: () => void;
}

export const useConversationListStore = create<ConversationListState>((set, get) => ({
  conversations: [],
  archivedConversations: [],
  totalActiveCount: 0,
  loading: false,
  error: null,

  fetchConversations: async () => {
    set({ loading: true, error: null });
    try {
      const result = await invoke<{ conversations: Conversation[]; total_count: number }>(
        "list_conversations",
        { limit: 200, offset: 0 },
      );
      set({
        conversations: result.conversations,
        totalActiveCount: result.total_count,
        loading: false,
      });
    } catch (e: unknown) {
      set({ error: String(e), loading: false });
    }
  },

  fetchArchivedConversations: async () => {
    try {
      const result = await invoke<Conversation[]>("list_archived_conversations");
      set({ archivedConversations: result });
    } catch (e: unknown) {
      set({ error: String(e) });
    }
  },

  searchConversations: async (query) => {
    try {
      return await invoke<ConversationSearchResult[]>("search_conversations", { query });
    } catch (e: unknown) {
      set({ error: String(e) });
      return [];
    }
  },

  renameConversation: async (id, title) => {
    try {
      await invoke("update_conversation", { id, input: { title } });
      set((state) => ({
        conversations: state.conversations.map((c) => c.id === id ? { ...c, title } : c),
      }));
    } catch (e: unknown) {
      set({ error: String(e) });
    }
  },

  deleteConversation: async (id) => {
    try {
      await invoke("delete_conversation", { id });
      set((state) => ({
        conversations: state.conversations.filter((c) => c.id !== id),
        totalActiveCount: Math.max(0, state.totalActiveCount - 1),
      }));
    } catch (e: unknown) {
      set({ error: String(e) });
    }
  },

  togglePin: async (id) => {
    try {
      await invoke("toggle_pin_conversation", { id });
      const { conversations } = get();
      const conv = conversations.find((c) => c.id === id);
      if (conv) {
        set({
          conversations: conversations.map((c) => c.id === id ? { ...c, is_pinned: !c.is_pinned } : c),
        });
      }
    } catch (e: unknown) {
      set({ error: String(e) });
    }
  },

  toggleArchive: async (id) => {
    try {
      await invoke("toggle_archive_conversation", { id });
      set((state) => ({
        conversations: state.conversations.filter((c) => c.id !== id),
        totalActiveCount: Math.max(0, state.totalActiveCount - 1),
      }));
    } catch (e: unknown) {
      set({ error: String(e) });
    }
  },

  archiveToKnowledgeBase: async (id, knowledgeBaseId) => {
    try {
      await invoke("archive_conversation_to_knowledge_base", { id, knowledge_base_id: knowledgeBaseId });
    } catch (e: unknown) {
      set({ error: String(e) });
    }
  },

  batchDelete: async (ids) => {
    try {
      await invoke("batch_delete_conversations", { ids });
      set((state) => ({
        conversations: state.conversations.filter((c) => !ids.includes(c.id)),
        totalActiveCount: Math.max(0, state.totalActiveCount - ids.length),
      }));
    } catch (e: unknown) {
      set({ error: String(e) });
    }
  },

  batchArchive: async (ids) => {
    try {
      for (const id of ids) {
        await invoke("toggle_archive_conversation", { id });
      }
      set((state) => ({
        conversations: state.conversations.filter((c) => !ids.includes(c.id)),
        totalActiveCount: Math.max(0, state.totalActiveCount - ids.length),
      }));
    } catch (e: unknown) {
      set({ error: String(e) });
    }
  },

  setConversations: (conversations) => set({ conversations }),

  updateConversationInList: (id, updates) => {
    set((state) => ({
      conversations: state.conversations.map((c) => c.id === id ? { ...c, ...updates } : c),
    }));
  },

  removeConversationFromList: (id) => {
    set((state) => ({
      conversations: state.conversations.filter((c) => c.id !== id),
    }));
  },

  addConversationToList: (conversation) => {
    set((state) => ({
      conversations: [conversation, ...state.conversations],
      totalActiveCount: state.totalActiveCount + 1,
    }));
  },

  clearError: () => set({ error: null }),
}));

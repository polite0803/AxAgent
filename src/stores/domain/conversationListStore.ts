import { create } from 'zustand';
import { invoke } from '@/lib/invoke';
import type { Conversation, UpdateConversationInput } from '@/types';

interface ConversationListState {
  conversations: Conversation[];
  activeConversationId: string | null;
  totalActiveCount: number;
  archivedConversations: Conversation[];
  loading: boolean;
  error: string | null;
  fetchConversations: () => Promise<void>;
  setActiveConversation: (id: string | null) => void;
  createConversation: (
    title: string,
    model_id: string,
    providerId: string,
    options?: { categoryId?: string | null; scenario?: string | null },
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
  fetchArchivedConversations: () => Promise<void>;
  batchDelete: (ids: string[]) => Promise<void>;
  batchArchive: (ids: string[]) => Promise<void>;
}

export const useConversationListStore = create<ConversationListState>((set, get) => ({
  conversations: [],
  activeConversationId: null,
  totalActiveCount: 0,
  archivedConversations: [],
  loading: false,
  error: null,

  fetchConversations: async () => {
    set({ loading: true, error: null });
    try {
      const conversations = await invoke<Conversation[]>('list_conversations');
      const active = conversations.filter((c) => !c.is_archived);
      set({
        conversations: active,
        totalActiveCount: conversations.filter((c) => c.is_pinned).length,
        loading: false,
      });
    } catch (error) {
      set({ error: String(error), loading: false });
    }
  },

  setActiveConversation: (id) => set({ activeConversationId: id }),

  createConversation: async (title, model_id, providerId, options) => {
    const conversation = await invoke<Conversation>('create_conversation', {
      title,
      model_id,
      provider_id: providerId,
      category_id: options?.categoryId,
      scenario: options?.scenario,
    });
    set((state) => ({
      conversations: [conversation, ...state.conversations],
    }));
    return conversation;
  },

  updateConversation: async (id, input) => {
    await invoke('update_conversation', { id, input });
    set((state) => ({
      conversations: state.conversations.map((c) =>
        c.id === id ? { ...c, ...input } : c
      ),
    }));
  },

  renameConversation: async (id, title) => {
    await get().updateConversation(id, { title });
  },

  deleteConversation: async (id) => {
    await invoke('delete_conversation', { id });
    set((state) => ({
      conversations: state.conversations.filter((c) => c.id !== id),
      activeConversationId:
        state.activeConversationId === id ? null : state.activeConversationId,
    }));
  },

  branchConversation: async (conversationId, untilMessageId, asChild, title) => {
    const newConv = await invoke<Conversation>('branch_conversation', {
      conversationId,
      untilMessageId,
      asChild,
      title: title || null,
    });
    set((state) => ({
      conversations: [newConv, ...state.conversations],
      activeConversationId: newConv.id,
    }));
    return newConv;
  },

  togglePin: async (id) => {
    const updated = await invoke<Conversation>('toggle_pin_conversation', { id });
    set((state) => ({
      conversations: state.conversations.map((c) => (c.id === id ? updated : c)),
    }));
  },

  toggleArchive: async (id) => {
    const updated = await invoke<Conversation>('toggle_archive_conversation', { id });
    if (updated.is_archived) {
      set((state) => ({
        conversations: state.conversations.filter((c) => c.id !== id),
        archivedConversations: [updated, ...state.archivedConversations],
        activeConversationId:
          state.activeConversationId === id ? null : state.activeConversationId,
      }));
    } else {
      set((state) => ({
        conversations: [updated, ...state.conversations],
        archivedConversations: state.archivedConversations.filter((c) => c.id !== id),
      }));
    }
  },

  archiveToKnowledgeBase: async (id, knowledgeBaseId) => {
    const updated = await invoke<Conversation>('archive_conversation_to_knowledge_base', {
      id,
      knowledgeBaseId,
    });
    set((state) => ({
      conversations: state.conversations.filter((c) => c.id !== id),
      archivedConversations: [updated, ...state.archivedConversations],
    }));
  },

  fetchArchivedConversations: async () => {
    try {
      const archived = await invoke<Conversation[]>('list_archived_conversations');
      set({ archivedConversations: archived });
    } catch (error) {
      set({ error: String(error) });
    }
  },

  batchDelete: async (ids) => {
    await invoke('batch_delete_conversations', { ids });
    set((state) => ({
      conversations: state.conversations.filter((c) => !ids.includes(c.id)),
      archivedConversations: state.archivedConversations.filter((c) => !ids.includes(c.id)),
    }));
  },

  batchArchive: async (ids) => {
    await invoke('batch_archive_conversations', { ids });
    set((state) => {
      const archived = state.conversations.filter((c) => ids.includes(c.id));
      return {
        conversations: state.conversations.filter((c) => !ids.includes(c.id)),
        archivedConversations: [...archived, ...state.archivedConversations],
      };
    });
  },
}));

interface ConversationState {
  conversations: Conversation[];
  activeConversationId: string | null;
  totalActiveCount: number;
  archivedConversations: Conversation[];
  fetchConversations: () => Promise<void>;
  setActiveConversation: (id: string | null) => void;
  createConversation: (
    title: string,
    model_id: string,
    providerId: string,
    options?: { categoryId?: string | null; scenario?: string | null },
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
  fetchArchivedConversations: () => Promise<void>;
  batchDelete: (ids: string[]) => Promise<void>;
  batchArchive: (ids: string[]) => Promise<void>;
}

export const useConversationStore = create<ConversationState>((set) => ({
  conversations: [],
  activeConversationId: null,
  totalActiveCount: 0,
  archivedConversations: [],

  fetchConversations: () => useConversationListStore.getState().fetchConversations(),
  setActiveConversation: (id) => set({ activeConversationId: id }),
  createConversation: (title, model_id, providerId, options) =>
    useConversationListStore.getState().createConversation(title, model_id, providerId, options),
  updateConversation: (id, input) =>
    useConversationListStore.getState().updateConversation(id, input),
  renameConversation: (id, title) =>
    useConversationListStore.getState().renameConversation(id, title),
  deleteConversation: (id) =>
    useConversationListStore.getState().deleteConversation(id),
  branchConversation: (conversationId, untilMessageId, asChild, title) =>
    useConversationListStore.getState().branchConversation(conversationId, untilMessageId, asChild, title),
  togglePin: (id) => useConversationListStore.getState().togglePin(id),
  toggleArchive: (id) => useConversationListStore.getState().toggleArchive(id),
  archiveToKnowledgeBase: (id, knowledgeBaseId) =>
    useConversationListStore.getState().archiveToKnowledgeBase(id, knowledgeBaseId),
  fetchArchivedConversations: () =>
    useConversationListStore.getState().fetchArchivedConversations(),
  batchDelete: (ids) => useConversationListStore.getState().batchDelete(ids),
  batchArchive: (ids) => useConversationListStore.getState().batchArchive(ids),
}));

export const useConversationStoreDel = useConversationStore;

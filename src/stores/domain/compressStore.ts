import { create } from 'zustand';
import { invoke } from '@/lib/invoke';
import { useConversationStore } from './conversationStore';
import type { ConversationSummary, MessagePage } from '@/types';

interface CompressState {
  compressing: boolean;
  compressContext: () => Promise<void>;
  getCompressionSummary: (conversationId: string) => Promise<ConversationSummary | null>;
  deleteCompression: () => Promise<void>;
}

export const useCompressStore = create<CompressState>((set) => ({
  compressing: false,

  compressContext: async () => {
    const conversationId = useConversationStore.getState().activeConversationId;
    if (!conversationId) return;
    set({ compressing: true });
    try {
      await invoke<ConversationSummary>('compress_context', { conversationId });
      // Reload messages to get the new compression marker
      const page = await invoke<MessagePage>('list_messages_page', {
        conversationId,
        limit: 100,
        beforeMessageId: null,
      });
      useConversationStore.setState({
        messages: page.messages,
        hasOlderMessages: page.has_older,
        totalActiveCount: page.total_active_count,
        oldestLoadedMessageId: page.messages.length > 0 ? page.messages[0].id : null,
      });
      set({ compressing: false });
    } catch (e) {
      set({ compressing: false });
      console.error('Failed to compress context:', e);
      throw e;
    }
  },

  getCompressionSummary: async (conversationId: string) => {
    try {
      return await invoke<ConversationSummary | null>('get_compression_summary', { conversationId });
    } catch (e) {
      console.error('Failed to get compression summary:', e);
      return null;
    }
  },

  deleteCompression: async () => {
    const conversationId = useConversationStore.getState().activeConversationId;
    if (!conversationId) return;
    try {
      await invoke('delete_compression', { conversationId });
      // Reload messages to remove the compression marker
      const page = await invoke<MessagePage>('list_messages_page', {
        conversationId,
        limit: 100,
        beforeMessageId: null,
      });
      useConversationStore.setState({
        messages: page.messages,
        hasOlderMessages: page.has_older,
        totalActiveCount: page.total_active_count,
        oldestLoadedMessageId: page.messages.length > 0 ? page.messages[0].id : null,
      });
    } catch (e) {
      console.error('Failed to delete compression:', e);
      throw e;
    }
  },
}));

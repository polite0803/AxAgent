import { create } from 'zustand';
import { invoke } from '@/lib/invoke';
import { _activeMessageLoadSeq } from './streamStore';
import type {
  Message,
  MessagePage,
  ConversationWorkspaceSnapshot,
  ConversationBranch,
  CompareResponsesResult,
} from '@/types';

// ─── Constants ───

export const MESSAGE_PAGE_SIZE = 10;

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

// ─── Lazy reference to conversationStore (avoids circular import) ───

let _conversationStoreRef: {
  getState: () => any;
  setState: (partial: any) => void;
} | null = null;

/** Register the conversationStore reference so messageStore can access activeConversationId etc. */
export function registerMessageStoreConvRef(ref: typeof _conversationStoreRef) {
  _conversationStoreRef = ref;
}

// ─── Message Store ───

interface MessageState {
  messages: Message[];
  loading: boolean;
  loadingOlder: boolean;
  hasOlderMessages: boolean;
  totalActiveCount: number;
  oldestLoadedMessageId: string | null;
  error: string | null;

  // Message CRUD
  insertContextClear: () => Promise<void>;
  removeContextClear: (messageId: string) => Promise<void>;
  clearAllMessages: () => Promise<void>;
  deleteMessage: (messageId: string) => Promise<void>;
  fetchMessages: (conversationId: string, preserveMessageIds?: string[]) => Promise<void>;
  loadOlderMessages: () => Promise<void>;
  switchMessageVersion: (conversationId: string, parentMessageId: string, messageId: string) => Promise<void>;
  listMessageVersions: (conversationId: string, parentMessageId: string) => Promise<Message[]>;
  updateMessageContent: (messageId: string, content: string) => Promise<void>;
  deleteMessageGroup: (conversationId: string, userMessageId: string) => Promise<void>;

  // Workspace / Fork
  workspaceSnapshot: ConversationWorkspaceSnapshot | null;
  loadWorkspaceSnapshot: (conversationId: string) => Promise<ConversationWorkspaceSnapshot | null>;
  updateWorkspaceSnapshot: (conversationId: string, snapshot: Partial<ConversationWorkspaceSnapshot>) => Promise<void>;
  forkConversation: (conversationId: string, fromMessageId?: string) => Promise<ConversationBranch | null>;
  compareResponses: (leftMessageId: string, rightMessageId: string) => Promise<CompareResponsesResult | null>;
}

export const useMessageStore = create<MessageState>((set, get) => ({
  messages: [],
  loading: false,
  loadingOlder: false,
  hasOlderMessages: false,
  totalActiveCount: 0,
  oldestLoadedMessageId: null,
  error: null,

  insertContextClear: async () => {
    const conversationId = _conversationStoreRef?.getState().activeConversationId;
    if (!conversationId) return;
    try {
      const msg = await invoke<Message>('send_system_message', {
        conversationId,
        content: '<!-- context-clear -->',
      });
      set((s) => ({ messages: [...s.messages, msg] }));
      await invoke('agent_backup_and_clear_sdk_context', { conversationId }).catch(() => {});
    } catch {
      const localMsg: Message = {
        id: `ctx-clear-${Date.now()}`,
        conversation_id: conversationId,
        role: 'system',
        content: '<!-- context-clear -->',
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
        status: 'complete',
      };
      set((s) => ({ messages: [...s.messages, localMsg] }));
    }
  },

  removeContextClear: async (messageId) => {
    const conversationId = _conversationStoreRef?.getState().activeConversationId;
    if (messageId.startsWith('ctx-clear-') || messageId.startsWith('temp-')) {
      set((s) => ({ messages: s.messages.filter((m) => m.id !== messageId) }));
      return;
    }

    try {
      await invoke('delete_message', { id: messageId });
      set((s) => ({ messages: s.messages.filter((m) => m.id !== messageId) }));
      if (conversationId) {
        await invoke('agent_restore_sdk_context_from_backup', { conversationId }).catch(() => {});
      }
    } catch (e) {
      set({ error: String(e) });
      throw e;
    }
  },

  clearAllMessages: async () => {
    const conversationId = _conversationStoreRef?.getState().activeConversationId;
    if (!conversationId) return;
    try {
      await invoke('clear_conversation_messages', { conversationId });
      set({ messages: [], hasOlderMessages: false, totalActiveCount: 0, oldestLoadedMessageId: null, loadingOlder: false });
    } catch (e) {
      console.error('Failed to clear messages:', e);
    }
  },

  deleteMessage: async (messageId) => {
    const conversationId = _conversationStoreRef?.getState().activeConversationId;
    if (!conversationId) return;
    if (messageId.startsWith('temp-')) {
      set((s) => ({
        messages: s.messages.filter((m) => m.id !== messageId),
      }));
      return;
    }
    try {
      await invoke('delete_message', { id: messageId });
      set((s) => ({
        messages: s.messages.filter((m) => m.id !== messageId),
      }));
    } catch (e) {
      set({ error: String(e) });
    }
  },

  fetchMessages: async (conversationId, preserveMessageIds = []) => {
    const requestSeq = _activeMessageLoadSeq;
    set({ loading: true });
    try {
      const page = await invoke<MessagePage>('list_messages_page', {
        conversationId,
        limit: MESSAGE_PAGE_SIZE,
        beforeMessageId: null,
      });
      if (requestSeq !== _activeMessageLoadSeq || _conversationStoreRef?.getState().activeConversationId !== conversationId) {
        return;
      }

      set((s) => {
        const messages = mergePreservedMessages(page.messages, preserveMessageIds, s.messages);
        return {
          messages,
          loading: false,
          loadingOlder: false,
          hasOlderMessages: page.has_older,
          totalActiveCount: page.total_active_count,
          oldestLoadedMessageId: messages[0]?.id ?? page.oldest_message_id,
          error: null,
        };
      });
    } catch (e) {
      if (requestSeq !== _activeMessageLoadSeq || _conversationStoreRef?.getState().activeConversationId !== conversationId) {
        return;
      }
      set({ error: String(e), loading: false, loadingOlder: false });
    }
  },

  loadOlderMessages: async () => {
    const { activeConversationId } = _conversationStoreRef?.getState() ?? {};
    const { oldestLoadedMessageId, hasOlderMessages, loading, loadingOlder } = get();
    if (!activeConversationId || !oldestLoadedMessageId || !hasOlderMessages || loading || loadingOlder) {
      return;
    }

    const requestSeq = _activeMessageLoadSeq;
    set({ loadingOlder: true, error: null });
    try {
      const page = await invoke<MessagePage>('list_messages_page', {
        conversationId: activeConversationId,
        limit: MESSAGE_PAGE_SIZE,
        beforeMessageId: oldestLoadedMessageId,
      });
      if (requestSeq !== _activeMessageLoadSeq || _conversationStoreRef?.getState().activeConversationId !== activeConversationId) {
        return;
      }

      set((s) => ({
        messages: mergeOlderPages(page.messages, s.messages),
        loadingOlder: false,
        hasOlderMessages: page.has_older,
        totalActiveCount: page.total_active_count,
        oldestLoadedMessageId: page.oldest_message_id ?? s.oldestLoadedMessageId,
        error: null,
      }));
    } catch (e) {
      if (requestSeq !== _activeMessageLoadSeq || _conversationStoreRef?.getState().activeConversationId !== activeConversationId) {
        return;
      }
      set({ error: String(e), loadingOlder: false });
    }
  },

  switchMessageVersion: async (conversationId, parentMessageId, messageId) => {
    // Note: multi-model check is handled by conversationStore which calls this
    // after checking _isMultiModelActive. The multi-model branch stays in
    // conversationStore because it needs to call setUserManuallySelectedVersion.
    try {
      await invoke('switch_message_version', { conversationId, parentMessageId, messageId });

      const versions = await get().listMessageVersions(conversationId, parentMessageId);
      if (versions.length > 0) {
        set((s) => {
          const versionMap = new Map(versions.map(v => [v.id, v]));
          const existingIds = new Set(
            s.messages
              .filter(m => m.parent_message_id === parentMessageId && m.role === 'assistant')
              .map(m => m.id),
          );
          const updatedMessages = s.messages.map((m) => {
            if (m.parent_message_id !== parentMessageId || m.role !== 'assistant') return m;
            const dbVersion = versionMap.get(m.id);
            if (dbVersion) {
              return { ...dbVersion, is_active: m.id === messageId };
            }
            return { ...m, is_active: m.id === messageId };
          });
          for (const v of versions) {
            if (!existingIds.has(v.id)) {
              updatedMessages.push({ ...v, is_active: v.id === messageId });
            }
          }
          return { messages: updatedMessages };
        });
      }
    } catch (e) {
      set({ error: String(e) });
      await get().fetchMessages(conversationId);
    }
  },

  listMessageVersions: async (conversationId, parentMessageId) => {
    try {
      return await invoke<Message[]>('list_message_versions', { conversationId, parentMessageId });
    } catch (e) {
      set({ error: String(e) });
      return [];
    }
  },

  updateMessageContent: async (messageId, content) => {
    try {
      const updated = await invoke<Message>('update_message_content', { id: messageId, content });
      set((s) => ({
        messages: s.messages.map((m) => (m.id === messageId ? { ...m, content: updated.content } : m)),
      }));
    } catch (e) {
      set({ error: String(e) });
      throw e;
    }
  },

  deleteMessageGroup: async (conversationId, userMessageId) => {
    if (userMessageId.startsWith('temp-')) {
      set((s) => ({
        messages: s.messages.filter(m =>
          m.id !== userMessageId && m.parent_message_id !== userMessageId
        ),
      }));
      return;
    }
    try {
      await invoke('delete_message_group', { conversationId, userMessageId });
      set((s) => ({
        messages: s.messages.filter(m =>
          m.id !== userMessageId && m.parent_message_id !== userMessageId
        ),
      }));
    } catch (e) {
      set({ error: String(e) });
    }
  },

  workspaceSnapshot: null,

  loadWorkspaceSnapshot: async (conversationId) => {
    try {
      const snapshot = await invoke<ConversationWorkspaceSnapshot>('get_workspace_snapshot', {
        conversationId: conversationId,
      });
      set({ workspaceSnapshot: snapshot });
      return snapshot;
    } catch {
      set({ workspaceSnapshot: null });
      return null;
    }
  },

  updateWorkspaceSnapshot: async (conversationId, snapshot) => {
    try {
      await invoke('update_workspace_snapshot', {
        conversationId: conversationId,
        ...snapshot,
      });
      set((s) => ({
        workspaceSnapshot: s.workspaceSnapshot
          ? { ...s.workspaceSnapshot, ...snapshot }
          : null,
      }));
    } catch (e) {
      console.error('Failed to update workspace snapshot:', e);
    }
  },

  forkConversation: async (conversationId, fromMessageId?) => {
    try {
      const branch = await invoke<ConversationBranch>('fork_conversation', {
        conversationId: conversationId,
        messageId: fromMessageId,
      });
      // Refresh conversations list via conversationStore
      const { fetchConversations } = _conversationStoreRef?.getState() ?? {};
      if (fetchConversations) await fetchConversations();
      return branch;
    } catch (e) {
      set({ error: String(e) });
      return null;
    }
  },

  compareResponses: async (leftMessageId, rightMessageId) => {
    try {
      return await invoke<CompareResponsesResult>('compare_branches', {
        branchA: leftMessageId,
        branchB: rightMessageId,
      });
    } catch {
      return null;
    }
  },
}));

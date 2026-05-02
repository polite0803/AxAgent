import { invoke } from "@/lib/invoke";
import type { CompareResponsesResult, ConversationBranch, ConversationWorkspaceSnapshot } from "@/types";
import { create } from "zustand";

interface WorkspaceState {
  workspaceSnapshot: ConversationWorkspaceSnapshot | null;
  loading: boolean;
  error: string | null;

  loadWorkspaceSnapshot: (conversationId: string) => Promise<ConversationWorkspaceSnapshot | null>;
  updateWorkspaceSnapshot: (conversationId: string, snapshot: Partial<ConversationWorkspaceSnapshot>) => Promise<void>;
  forkConversation: (conversationId: string, fromMessageId?: string) => Promise<ConversationBranch | null>;
  compareResponses: (leftMessageId: string, rightMessageId: string) => Promise<CompareResponsesResult | null>;
  clearError: () => void;
}

export const useWorkspaceStore = create<WorkspaceState>((set) => ({
  workspaceSnapshot: null,
  loading: false,
  error: null,

  loadWorkspaceSnapshot: async (conversationId) => {
    set({ loading: true, error: null });
    try {
      const snapshot = await invoke<ConversationWorkspaceSnapshot | null>(
        "get_workspace_snapshot",
        { conversation_id: conversationId },
      );
      set({ workspaceSnapshot: snapshot, loading: false });
      return snapshot;
    } catch (e: unknown) {
      set({ error: String(e), loading: false });
      return null;
    }
  },

  updateWorkspaceSnapshot: async (conversationId, snapshot) => {
    try {
      await invoke("update_workspace_snapshot", { conversation_id: conversationId, snapshot });
      set((state) => ({
        workspaceSnapshot: state.workspaceSnapshot
          ? { ...state.workspaceSnapshot, ...snapshot }
          : null,
      }));
    } catch (e: unknown) {
      set({ error: String(e) });
    }
  },

  forkConversation: async (conversationId, fromMessageId) => {
    try {
      return await invoke<ConversationBranch | null>("fork_conversation", {
        conversationId,
        fromMessageId: fromMessageId ?? null,
      });
    } catch (e: unknown) {
      set({ error: String(e) });
      return null;
    }
  },

  compareResponses: async (leftMessageId, rightMessageId) => {
    try {
      return await invoke<CompareResponsesResult | null>("compare_responses", {
        leftMessageId,
        rightMessageId,
      });
    } catch (e: unknown) {
      set({ error: String(e) });
      return null;
    }
  },

  clearError: () => set({ error: null }),
}));

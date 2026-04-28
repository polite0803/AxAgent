import { invoke } from "@/lib/invoke";
import type { AttachmentInput } from "@/types";
import { create } from "zustand";

interface MultiModelState {
  pendingCompanionModels: Array<{ providerId: string; model_id: string }>;
  multiModelParentId: string | null;
  multiModelDoneMessageIds: string[];
  loading: boolean;
  error: string | null;

  sendMultiModelMessage: (
    conversationId: string,
    content: string,
    companionModels: Array<{ providerId: string; model_id: string }>,
    attachments?: AttachmentInput[],
    searchProviderId?: string | null,
  ) => Promise<void>;
  addDoneMessageId: (messageId: string) => void;
  resetMultiModel: () => void;
  clearError: () => void;
}

export const useMultiModelStore = create<MultiModelState>((set) => ({
  pendingCompanionModels: [],
  multiModelParentId: null,
  multiModelDoneMessageIds: [],
  loading: false,
  error: null,

  sendMultiModelMessage: async (
    conversationId,
    content,
    companionModels,
    attachments,
    searchProviderId,
  ) => {
    set({ loading: true, error: null, pendingCompanionModels: companionModels, multiModelDoneMessageIds: [] });
    try {
      await invoke("send_multi_model_message", {
        conversationId,
        content,
        companionModels,
        attachments: attachments ?? [],
        searchProviderId: searchProviderId ?? null,
      });
    } catch (e: any) {
      set({ error: String(e), loading: false });
    }
  },

  addDoneMessageId: (messageId) => {
    set((state) => ({
      multiModelDoneMessageIds: [...state.multiModelDoneMessageIds, messageId],
    }));
  },

  resetMultiModel: () => {
    set({
      pendingCompanionModels: [],
      multiModelParentId: null,
      multiModelDoneMessageIds: [],
      loading: false,
    });
  },

  clearError: () => set({ error: null }),
}));

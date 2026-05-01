import { invoke } from "@/lib/invoke";
import type { AttachmentInput, Message } from "@/types";
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
    _searchProviderId,
  ) => {
    set({
      loading: true,
      error: null,
      pendingCompanionModels: companionModels,
      multiModelDoneMessageIds: [],
    });

    try {
      // 步骤 1: 创建用户消息并启动主模型响应
      const userMessage = await invoke<Message>("send_message", {
        conversationId,
        content,
        attachments: attachments ?? [],
        enabledMcpServerIds: null,
        thinkingBudget: null,
        enabledKnowledgeBaseIds: null,
        enabledMemoryNamespaceIds: null,
      });

      // 步骤 2: 并发启动所有伴随模型响应
      const companionPromises = companionModels.map((companion) =>
        invoke("regenerate_with_model", {
          conversationId,
          userMessageId: userMessage.id,
          targetProviderId: companion.providerId,
          targetModelId: companion.model_id,
          enabledMcpServerIds: null,
          thinkingBudget: null,
          enabledKnowledgeBaseIds: null,
          enabledMemoryNamespaceIds: null,
          isCompanion: true,
        })
      );

      await Promise.allSettled(companionPromises);

      set({ loading: false });
    } catch (e: unknown) {
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

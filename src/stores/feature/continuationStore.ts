// SPDX-License-Identifier: AGPL-3.0-only

// 消息续写状态管理
import { invoke, logIpcError } from "@/lib/invoke";
import { useConversationStore } from "@/stores/domain/conversationStore";
import { create } from "zustand";

interface ContinuationStore {
  continuing: Record<string, boolean>;
  continuableMessages: Record<
    string,
    Array<{
      id: string;
      parentMessageId: string;
      status: string;
      contentPreview: string;
      createdAt: number;
    }>
  >;

  loadContinuable: (conversationId: string) => Promise<void>;
  startContinue: (
    conversationId: string,
    messageId: string,
    branch: boolean,
  ) => Promise<void>;
}

export const useContinuationStore = create<ContinuationStore>((set) => ({
  continuing: {},
  continuableMessages: {},

  loadContinuable: async (conversationId: string) => {
    try {
      const result = await invoke<
        Array<{
          id: string;
          parentMessageId: string;
          status: string;
          contentPreview: string;
          createdAt: number;
        }>
      >("list_continuable_messages", { conversationId });
      set((s) => ({
        continuableMessages: {
          ...s.continuableMessages,
          [conversationId]: result,
        },
      }));
    } catch (e) {
      logIpcError("continuationStore: 加载可续写消息失败")(e);
    }
  },

  startContinue: async (conversationId, messageId, branch) => {
    // 临时占位消息（temp- 前缀）不存在于数据库中，无法续写
    if (messageId.startsWith("temp-")) {
      return;
    }
    set((s) => ({ continuing: { ...s.continuing, [messageId]: true } }));

    try {
      await invoke("continue_message", { conversationId, messageId, branch });
      const convStore = useConversationStore.getState();
      await convStore.regenerateMessage(messageId);
    } catch (e) {
      logIpcError("continuationStore: 续写失败")(e);
    } finally {
      set((s) => ({ continuing: { ...s.continuing, [messageId]: false } }));
    }
  },
}));

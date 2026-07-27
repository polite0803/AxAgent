// SPDX-License-Identifier: AGPL-3.0-only

import { invoke, logIpcError } from "@/lib/invoke";
import type { ContextSource } from "@/types";
import { create } from "zustand";

interface ContextSourceState {
  /** 当前活动会话的 context sources 列表（含 doc_ids 过滤状态） */
  sources: ContextSource[];
  /** 当前已加载的 conversationId，用于判断是否需要重新加载 */
  loadedConversationId: string | null;
  loading: boolean;
  error: string | null;

  /** 加载指定会话的 context sources；若已是同一会话则跳过 */
  loadSources: (conversationId: string, force?: boolean) => Promise<void>;
  /** 切换会话时清空本地缓存 */
  clear: () => void;
  /**
   * 多文档协同：更新指定容器的 doc_ids 过滤集合。
   * 调用后端 set_context_source_doc_ids 并同步本地状态。
   * 若对应容器行不存在（用户尚未启用该 KB/wiki/memory），后端会返回错误，前端忽略。
   */
  setDocIds: (
    conversationId: string,
    sourceType: "knowledge" | "memory" | "wiki",
    refId: string,
    docIds: string[],
  ) => Promise<void>;
}

export const useContextSourceStore = create<ContextSourceState>((set, get) => ({
  sources: [],
  loadedConversationId: null,
  loading: false,
  error: null,

  loadSources: async (conversationId, force = false) => {
    if (!force && get().loadedConversationId === conversationId) {
      return;
    }
    set({ loading: true, error: null });
    try {
      const sources = await invoke<ContextSource[]>("list_context_sources", {
        conversationId,
      });
      set({
        sources: Array.isArray(sources) ? sources : [],
        loadedConversationId: conversationId,
        loading: false,
      });
    } catch (e) {
      logIpcError("contextSourceStore.loadSources")(e);
      set({ loading: false, error: String(e) });
    }
  },

  clear: () => {
    set({ sources: [], loadedConversationId: null, loading: false, error: null });
  },

  setDocIds: async (conversationId, sourceType, refId, docIds) => {
    try {
      await invoke<ContextSource>("set_context_source_doc_ids", {
        conversationId,
        sourceType,
        refId,
        docIds,
      });
      // 乐观更新本地缓存
      set((s) => ({
        sources: s.sources.map((src) =>
          src.type === sourceType && src.refId === refId
            ? { ...src, docIds: [...docIds] }
            : src
        ),
      }));
    } catch (e) {
      // 容器行可能尚未创建（用户未启用该 KB），静默降级
      logIpcError("contextSourceStore.setDocIds")(e);
      set({ error: String(e) });
    }
  },
}));

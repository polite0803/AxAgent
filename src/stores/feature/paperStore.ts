// SPDX-License-Identifier: AGPL-3.0-only

// 论文概览 / 论文问答 Store
// 对接后端 commands::paper_overview 与 commands::paper_qa 模块

import { invoke } from "@/lib/invoke";
import type {
  CreatePaperOverviewInput,
  PaperOverview,
  PaperQAPreparedContext,
  UpdatePaperOverviewInput,
} from "@/types";
import { create } from "zustand";

interface PaperState {
  /** 当前知识库下的论文概览列表 */
  overviews: PaperOverview[];
  /** 当前查看的论文概览（按文档加载） */
  currentOverview: PaperOverview | null;
  /** 最近一次论文 QA 准备好的上下文 */
  preparedQaContext: PaperQAPreparedContext | null;
  loading: boolean;
  error: string | null;

  /** 加载指定知识库下的全部论文概览 */
  loadOverviewsByKb: (kbId: string) => Promise<void>;
  /** 按文档 ID 拉取论文概览，结果写入 currentOverview（无概览时为 null） */
  getOverviewByDocument: (documentId: string) => Promise<PaperOverview | null>;
  /** 新建论文概览 */
  createOverview: (input: CreatePaperOverviewInput) => Promise<PaperOverview | null>;
  /** 更新论文概览 */
  updateOverview: (id: string, input: UpdatePaperOverviewInput) => Promise<PaperOverview | null>;
  /** 按文档 upsert 论文概览（前端首选入口，LLM 返回 JSON 后调用） */
  upsertOverviewByDocument: (input: CreatePaperOverviewInput) => Promise<PaperOverview | null>;
  /** 删除论文概览 */
  deleteOverview: (id: string) => Promise<void>;
  /** 生成论文概览 Prompt（供用户复制到聊天调用 LLM） */
  generateOverviewPrompt: (documentId: string, maxChars?: number) => Promise<string>;
  /** 准备论文 QA 上下文 */
  preparePaperQaContext: (
    documentId: string,
    question: string,
    topK?: number,
  ) => Promise<PaperQAPreparedContext | null>;
  /** 清空当前概览与 QA 上下文（关闭面板时调用） */
  clearCurrent: () => void;
  /** 清空错误状态 */
  clearError: () => void;
}

export const usePaperStore = create<PaperState>((set) => ({
  overviews: [],
  currentOverview: null,
  preparedQaContext: null,
  loading: false,
  error: null,

  loadOverviewsByKb: async (kbId) => {
    set({ loading: true, error: null });
    try {
      const overviews = await invoke<PaperOverview[]>(
        "list_paper_overviews_by_kb",
        { knowledgeBaseId: kbId },
      );
      set({ overviews: Array.isArray(overviews) ? overviews : [], loading: false });
    } catch (e) {
      set({ error: String(e), loading: false });
    }
  },

  getOverviewByDocument: async (documentId) => {
    set({ loading: true, error: null });
    try {
      const overview = await invoke<PaperOverview | null>(
        "get_paper_overview_by_document",
        { documentId },
      );
      set({ currentOverview: overview ?? null, loading: false });
      return overview;
    } catch (e) {
      set({ error: String(e), loading: false });
      return null;
    }
  },

  createOverview: async (input) => {
    try {
      const overview = await invoke<PaperOverview>("create_paper_overview", { input });
      set((s) => ({
        overviews: [...s.overviews, overview],
        currentOverview: overview,
        error: null,
      }));
      return overview;
    } catch (e) {
      set({ error: String(e) });
      return null;
    }
  },

  updateOverview: async (id, input) => {
    try {
      const overview = await invoke<PaperOverview>("update_paper_overview", { id, input });
      set((s) => ({
        overviews: s.overviews.map((o) => (o.id === id ? overview : o)),
        currentOverview: s.currentOverview?.id === id ? overview : s.currentOverview,
        error: null,
      }));
      return overview;
    } catch (e) {
      set({ error: String(e) });
      return null;
    }
  },

  upsertOverviewByDocument: async (input) => {
    try {
      const overview = await invoke<PaperOverview>("upsert_paper_overview_by_document", { input });
      set((s) => ({
        // 若列表已有同 id，替换；否则追加
        overviews: s.overviews.some((o) => o.id === overview.id)
          ? s.overviews.map((o) => (o.id === overview.id ? overview : o))
          : [...s.overviews, overview],
        currentOverview: overview,
        error: null,
      }));
      return overview;
    } catch (e) {
      set({ error: String(e) });
      return null;
    }
  },

  deleteOverview: async (id) => {
    try {
      await invoke("delete_paper_overview", { id });
      set((s) => ({
        overviews: s.overviews.filter((o) => o.id !== id),
        currentOverview: s.currentOverview?.id === id ? null : s.currentOverview,
        error: null,
      }));
    } catch (e) {
      set({ error: String(e) });
      throw e;
    }
  },

  generateOverviewPrompt: async (documentId, maxChars) => {
    try {
      const prompt = await invoke<string>("generate_paper_overview_prompt", {
        documentId,
        maxChars,
      });
      return prompt;
    } catch (e) {
      set({ error: String(e) });
      throw e;
    }
  },

  preparePaperQaContext: async (documentId, question, topK) => {
    set({ loading: true, error: null });
    try {
      const ctx = await invoke<PaperQAPreparedContext>("prepare_paper_qa_context", {
        documentId,
        question,
        topK,
      });
      set({ preparedQaContext: ctx, loading: false });
      return ctx;
    } catch (e) {
      set({ error: String(e), loading: false });
      return null;
    }
  },

  clearCurrent: () => {
    set({ currentOverview: null, preparedQaContext: null, error: null });
  },

  clearError: () => {
    set({ error: null });
  },
}));

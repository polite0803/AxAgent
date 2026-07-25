// SPDX-License-Identifier: AGPL-3.0-only
/**
 * G6 截图持仓诊断（Screenshot Diagnosis）Zustand store
 *
 * 负责：
 * - 列表 / 详情的 IPC 调用与缓存
 * - 上传截图自动诊断（OCR + 结构化 + 风险诊断 + 持久化）
 * - 一键转为模拟观察组合（G2 联动）
 * - loading / error 状态管理
 *
 * 命令清单（与后端 commands/screenshot_diagnosis.rs 对齐）：
 * - screenshot_diagnosis_create_from_image（桌面端，移动端不支持）
 * - screenshot_diagnosis_create / get / list_recent / list_by_status
 * - screenshot_diagnosis_archive / update
 * - screenshot_diagnosis_to_paper_portfolio
 */

import { invoke } from "@/lib/invoke";
import type { PaperPortfolio } from "@/types/paper-portfolio";
import type {
  ConvertToPaperPortfolioInput,
  CreateDiagnosisFromImageInput,
  ScreenshotDiagnosis,
  UpdateDiagnosisInput,
} from "@/types/screenshot-diagnosis";
import { create } from "zustand";

interface ScreenshotDiagnosisState {
  // ── 数据 ──
  /** 最近的诊断列表（按 created_at 降序） */
  recentDiagnoses: ScreenshotDiagnosis[];
  /** 按状态过滤的诊断列表 */
  filteredDiagnoses: ScreenshotDiagnosis[];
  /** 当前选中的诊断详情 */
  currentDiagnosis: ScreenshotDiagnosis | null;

  // ── 状态 ──
  loadingList: boolean;
  loadingDetail: boolean;
  submitting: boolean;
  converting: boolean;
  error: string | null;

  // ── Actions ──
  /** 列出最近 N 条诊断（默认 20） */
  fetchRecent: (limit?: number) => Promise<void>;
  /** 按状态过滤诊断 */
  fetchByStatus: (status: string) => Promise<void>;
  /** 获取单个诊断详情 */
  fetchDiagnosis: (diagnosisId: string) => Promise<void>;
  /** 上传截图自动诊断（OCR + 结构化 + 风险诊断 + 持久化） */
  createFromImage: (input: CreateDiagnosisFromImageInput) => Promise<ScreenshotDiagnosis>;
  /** 归档诊断 */
  archiveDiagnosis: (diagnosisId: string) => Promise<void>;
  /** 更新诊断字段 */
  updateDiagnosis: (input: UpdateDiagnosisInput) => Promise<void>;
  /** 一键转为模拟观察组合 */
  convertToPaperPortfolio: (
    input: ConvertToPaperPortfolioInput,
  ) => Promise<PaperPortfolio>;
  /** 清空当前详情 */
  clearCurrentDetail: () => void;
  /** 清空错误 */
  clearError: () => void;
}

export const useScreenshotDiagnosisStore = create<ScreenshotDiagnosisState>((set, get) => ({
  recentDiagnoses: [],
  filteredDiagnoses: [],
  currentDiagnosis: null,

  loadingList: false,
  loadingDetail: false,
  submitting: false,
  converting: false,
  error: null,

  fetchRecent: async (limit?: number) => {
    set({ loadingList: true, error: null });
    try {
      const list = await invoke<ScreenshotDiagnosis[]>(
        "screenshot_diagnosis_list_recent",
        { limit: limit ?? 20 },
      );
      set({ recentDiagnoses: list, loadingList: false });
    } catch (e) {
      set({ loadingList: false, error: String(e) });
    }
  },

  fetchByStatus: async (status: string) => {
    set({ loadingList: true, error: null });
    try {
      const list = await invoke<ScreenshotDiagnosis[]>(
        "screenshot_diagnosis_list_by_status",
        { status },
      );
      set({ filteredDiagnoses: list, loadingList: false });
    } catch (e) {
      set({ loadingList: false, error: String(e) });
    }
  },

  fetchDiagnosis: async (diagnosisId: string) => {
    set({ loadingDetail: true, error: null });
    try {
      const detail = await invoke<ScreenshotDiagnosis | null>(
        "screenshot_diagnosis_get",
        { diagnosisId },
      );
      set({ currentDiagnosis: detail, loadingDetail: false });
    } catch (e) {
      set({ loadingDetail: false, error: String(e) });
    }
  },

  createFromImage: async (input: CreateDiagnosisFromImageInput) => {
    set({ submitting: true, error: null });
    try {
      const diagnosis = await invoke<ScreenshotDiagnosis>(
        "screenshot_diagnosis_create_from_image",
        {
          imageBase64: input.imageBase64,
          sourceApp: input.sourceApp ?? null,
          providerId: input.providerId,
          modelId: input.modelId,
        },
      );
      // 把新诊断加入列表头部
      const cur = get().recentDiagnoses;
      set({
        recentDiagnoses: [diagnosis, ...cur.filter((d) => d.id !== diagnosis.id)],
        currentDiagnosis: diagnosis,
        submitting: false,
      });
      return diagnosis;
    } catch (e) {
      set({ submitting: false, error: String(e) });
      throw e;
    }
  },

  archiveDiagnosis: async (diagnosisId: string) => {
    try {
      const updated = await invoke<ScreenshotDiagnosis>(
        "screenshot_diagnosis_archive",
        { diagnosisId },
      );
      const cur = get().recentDiagnoses;
      set({
        recentDiagnoses: cur.map((d) => (d.id === updated.id ? updated : d)),
        currentDiagnosis: get().currentDiagnosis?.id === updated.id ? updated : get().currentDiagnosis,
      });
    } catch (e) {
      set({ error: String(e) });
      throw e;
    }
  },

  updateDiagnosis: async (input: UpdateDiagnosisInput) => {
    try {
      const updated = await invoke<ScreenshotDiagnosis>(
        "screenshot_diagnosis_update",
        { input },
      );
      const cur = get().recentDiagnoses;
      set({
        recentDiagnoses: cur.map((d) => (d.id === updated.id ? updated : d)),
        currentDiagnosis: get().currentDiagnosis?.id === updated.id ? updated : get().currentDiagnosis,
      });
    } catch (e) {
      set({ error: String(e) });
      throw e;
    }
  },

  convertToPaperPortfolio: async (input: ConvertToPaperPortfolioInput) => {
    set({ converting: true, error: null });
    try {
      const portfolio = await invoke<PaperPortfolio>(
        "screenshot_diagnosis_to_paper_portfolio",
        {
          diagnosisId: input.diagnosisId,
          name: input.name,
          sourceEvent: input.sourceEvent,
        },
      );
      set({ converting: false });
      return portfolio;
    } catch (e) {
      set({ converting: false, error: String(e) });
      throw e;
    }
  },

  clearCurrentDetail: () => set({ currentDiagnosis: null }),
  clearError: () => set({ error: null }),
}));

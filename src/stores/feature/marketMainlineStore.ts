// SPDX-License-Identifier: AGPL-3.0-only
/**
 * G4 市场主线（Market Mainline）Zustand store
 *
 * 负责：
 * - 列表 / 详情的 IPC 调用与缓存
 * - 创建 / 更新 / 归档等操作
 * - loading / error 状态管理
 *
 * 命令清单（与后端 commands/market_mainline.rs 对齐）：
 * - market_mainline_create / get / list_by_date / list_recent / list_by_status / list_by_category
 * - market_mainline_update / archive
 * - market_mainline_batch_upsert / delete_by_date
 */

import { invoke } from "@/lib/invoke";
import type {
  BatchUpsertInput,
  BatchUpsertResult,
  CreateMainlineInput,
  MarketMainline,
  UpdateMainlineInput,
} from "@/types";
import { create } from "zustand";

interface MarketMainlineState {
  // ── 数据 ──
  /** 最近 N 天主线（Dashboard 用） */
  recentMainlines: MarketMainline[];
  /** 当前选中日期的主线列表 */
  dateMainlines: MarketMainline[];
  /** 当前选中的单条主线（详情用） */
  currentMainline: MarketMainline | null;

  // ── 状态 ──
  loadingRecent: boolean;
  loadingDate: boolean;
  loadingDetail: boolean;
  submitting: boolean;
  error: string | null;

  // ── Actions ──
  /** 列出最近 N 天的主线（默认 7 天） */
  fetchRecentMainlines: (days?: number) => Promise<void>;
  /** 列出某日所有主线 */
  fetchMainlinesByDate: (date: string) => Promise<void>;
  /** 按状态过滤主线 */
  fetchMainlinesByStatus: (status: string) => Promise<MarketMainline[]>;
  /** 按主题大类过滤主线 */
  fetchMainlinesByCategory: (category: string) => Promise<MarketMainline[]>;
  /** 获取单条主线 */
  fetchMainline: (mainlineId: string) => Promise<void>;
  /** 创建主线 */
  createMainline: (input: CreateMainlineInput) => Promise<MarketMainline>;
  /** 更新主线（部分字段） */
  updateMainline: (input: UpdateMainlineInput) => Promise<void>;
  /** 归档主线 */
  archiveMainline: (mainlineId: string) => Promise<void>;
  /** 批量 upsert 主线（工作流用） */
  batchUpsertMainlines: (input: BatchUpsertInput) => Promise<BatchUpsertResult>;
  /** 清除某日所有主线 */
  deleteMainlinesByDate: (date: string) => Promise<number>;
  /** 清空当前详情 */
  clearCurrentMainline: () => void;
  /** 清空错误 */
  clearError: () => void;
}

export const useMarketMainlineStore = create<MarketMainlineState>((set, get) => ({
  recentMainlines: [],
  dateMainlines: [],
  currentMainline: null,

  loadingRecent: false,
  loadingDate: false,
  loadingDetail: false,
  submitting: false,
  error: null,

  fetchRecentMainlines: async (days?: number) => {
    set({ loadingRecent: true, error: null });
    try {
      const data = await invoke<MarketMainline[]>("market_mainline_list_recent", {
        days: days ?? 7,
      });
      set({ recentMainlines: data, loadingRecent: false });
    } catch (e) {
      set({ loadingRecent: false, error: String(e) });
    }
  },

  fetchMainlinesByDate: async (date: string) => {
    set({ loadingDate: true, error: null });
    try {
      const data = await invoke<MarketMainline[]>("market_mainline_list_by_date", {
        mainlineDate: date,
      });
      set({ dateMainlines: data, loadingDate: false });
    } catch (e) {
      set({ loadingDate: false, error: String(e) });
    }
  },

  fetchMainlinesByStatus: async (status: string) => {
    set({ error: null });
    try {
      return await invoke<MarketMainline[]>("market_mainline_list_by_status", { status });
    } catch (e) {
      set({ error: String(e) });
      return [];
    }
  },

  fetchMainlinesByCategory: async (category: string) => {
    set({ error: null });
    try {
      return await invoke<MarketMainline[]>("market_mainline_list_by_category", {
        themeCategory: category,
      });
    } catch (e) {
      set({ error: String(e) });
      return [];
    }
  },

  fetchMainline: async (mainlineId: string) => {
    set({ loadingDetail: true, error: null });
    try {
      const data = await invoke<MarketMainline | null>("market_mainline_get", {
        mainlineId,
      });
      set({ currentMainline: data, loadingDetail: false });
    } catch (e) {
      set({ loadingDetail: false, error: String(e) });
    }
  },

  createMainline: async (input: CreateMainlineInput) => {
    set({ submitting: true, error: null });
    try {
      const created = await invoke<MarketMainline>("market_mainline_create", { input });
      set({ submitting: false });
      // 创建后刷新最近列表
      await get().fetchRecentMainlines();
      return created;
    } catch (e) {
      set({ submitting: false, error: String(e) });
      throw e;
    }
  },

  updateMainline: async (input: UpdateMainlineInput) => {
    set({ submitting: true, error: null });
    try {
      await invoke("market_mainline_update", { input });
      set({ submitting: false });
      // 更新后刷新最近列表
      await get().fetchRecentMainlines();
    } catch (e) {
      set({ submitting: false, error: String(e) });
      throw e;
    }
  },

  archiveMainline: async (mainlineId: string) => {
    set({ submitting: true, error: null });
    try {
      await invoke("market_mainline_archive", { mainlineId });
      set({ submitting: false });
      await get().fetchRecentMainlines();
    } catch (e) {
      set({ submitting: false, error: String(e) });
      throw e;
    }
  },

  batchUpsertMainlines: async (input: BatchUpsertInput) => {
    set({ submitting: true, error: null });
    try {
      const result = await invoke<BatchUpsertResult>("market_mainline_batch_upsert", { input });
      set({ submitting: false });
      await get().fetchRecentMainlines();
      return result;
    } catch (e) {
      set({ submitting: false, error: String(e) });
      throw e;
    }
  },

  deleteMainlinesByDate: async (date: string) => {
    set({ submitting: true, error: null });
    try {
      const affected = await invoke<number>("market_mainline_delete_by_date", {
        mainlineDate: date,
      });
      set({ submitting: false });
      await get().fetchRecentMainlines();
      return affected;
    } catch (e) {
      set({ submitting: false, error: String(e) });
      throw e;
    }
  },

  clearCurrentMainline: () => set({ currentMainline: null }),
  clearError: () => set({ error: null }),
}));

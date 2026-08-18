// SPDX-License-Identifier: AGPL-3.0-only

import { create } from "zustand";

import {
  getLearningConfig,
  getRLStats,
  listLearningConfigs,
  recordRLExperience,
  triggerAutoLearning,
  triggerRLOptimization,
} from "@/lib/opcLearning";
import type {
  AutoLearningResult,
  ExperiencePoolStats,
  IndustryLearningConfig,
  IndustryLearningConfigSummary,
  RLPolicyUpdate,
} from "@/types";

interface IndustryLearningState {
  /** 所有行业学习配置缓存 */
  configs: Map<string, IndustryLearningConfig>;
  /** 配置列表摘要 */
  summaries: IndustryLearningConfigSummary[];
  /** 加载状态 */
  loading: boolean;
  /** 错误信息 */
  error: string | null;
  /** 最后加载时间 */
  lastLoadedAt: number | null;

  /** RL 经验池统计（按行业 ID 缓存） */
  rlStats: Map<string, ExperiencePoolStats>;
  /** 全局 RL 统计（跨行业汇总） */
  rlGlobalStats: ExperiencePoolStats | null;
  /** RL 策略优化结果（按行业 ID 缓存） */
  rlPolicyUpdates: Map<string, RLPolicyUpdate>;
  /** 自动学习闭环触发历史 */
  autoLearningHistory: AutoLearningResult[];
  /** RL 操作加载状态 */
  rlLoading: boolean;

  /** 加载指定行业的学习配置 */
  loadConfig: (industryId: string) => Promise<IndustryLearningConfig | null>;
  /** 加载所有行业学习配置列表 */
  loadAllConfigs: () => Promise<void>;
  /** 从缓存获取配置 */
  getConfig: (industryId: string) => IndustryLearningConfig | undefined;
  /** 清除缓存 */
  clearCache: () => void;

  /** 获取指定行业的 RL 经验池统计 */
  loadRLStats: (industryId?: string) => Promise<ExperiencePoolStats | null>;
  /** 记录 RL 经验 */
  recordExperience: (params: {
    industryId: string;
    workflowId: string;
    qualityScore: number;
    workflowResult: Record<string, unknown>;
  }) => Promise<boolean>;
  /** 触发 RL 策略优化 */
  triggerOptimization: (industryId: string) => Promise<RLPolicyUpdate | null>;
  /** 触发自动学习闭环 */
  triggerAutoLearning: (params: {
    industryId: string;
    workflowId: string;
    workflowResult: Record<string, unknown>;
  }) => Promise<AutoLearningResult | null>;
  /** 获取最近的自动学习结果 */
  getLatestAutoLearning: () => AutoLearningResult | undefined;
}

const CACHE_TTL_MS = 5 * 60 * 1000;

export const useIndustryLearningStore = create<IndustryLearningState>(
  (set, get) => ({
    configs: new Map(),
    summaries: [],
    loading: false,
    error: null,
    lastLoadedAt: null,

    rlStats: new Map(),
    rlGlobalStats: null,
    rlPolicyUpdates: new Map(),
    autoLearningHistory: [],
    rlLoading: false,

    loadConfig: async (industryId: string) => {
      const state = get();
      const cached = state.configs.get(industryId);
      if (cached && state.lastLoadedAt && Date.now() - state.lastLoadedAt < CACHE_TTL_MS) {
        return cached;
      }

      set({ loading: true, error: null });
      try {
        const config = await getLearningConfig(industryId);
        const newConfigs = new Map(state.configs);
        newConfigs.set(industryId, config);
        set({ configs: newConfigs, loading: false, lastLoadedAt: Date.now() });
        return config;
      } catch (e) {
        set({ error: String(e), loading: false });
        return null;
      }
    },

    loadAllConfigs: async () => {
      set({ loading: true, error: null });
      try {
        const summaries = await listLearningConfigs();
        set({ summaries, loading: false, lastLoadedAt: Date.now() });
      } catch (e) {
        set({ error: String(e), loading: false });
      }
    },

    getConfig: (industryId: string) => {
      return get().configs.get(industryId);
    },

    clearCache: () => {
      set({
        configs: new Map(),
        summaries: [],
        lastLoadedAt: null,
        rlStats: new Map(),
        rlGlobalStats: null,
        rlPolicyUpdates: new Map(),
      });
    },

    loadRLStats: async (industryId?: string) => {
      set({ rlLoading: true });
      try {
        const stats = await getRLStats(industryId);
        if (industryId) {
          const newStats = new Map(get().rlStats);
          newStats.set(industryId, stats);
          set({ rlStats: newStats, rlLoading: false });
        } else {
          set({ rlGlobalStats: stats, rlLoading: false });
        }
        return stats;
      } catch (e) {
        set({ error: String(e), rlLoading: false });
        return null;
      }
    },

    recordExperience: async (params: {
      industryId: string;
      workflowId: string;
      qualityScore: number;
      workflowResult: Record<string, unknown>;
    }) => {
      set({ rlLoading: true });
      try {
        const result = await recordRLExperience(params);
        set({ rlLoading: false });
        return result.success;
      } catch (e) {
        set({ error: String(e), rlLoading: false });
        return false;
      }
    },

    triggerOptimization: async (industryId: string) => {
      set({ rlLoading: true });
      try {
        const update = await triggerRLOptimization({ industryId });
        const newUpdates = new Map(get().rlPolicyUpdates);
        newUpdates.set(industryId, update);
        set({ rlPolicyUpdates: newUpdates, rlLoading: false });
        return update;
      } catch (e) {
        set({ error: String(e), rlLoading: false });
        return null;
      }
    },

    triggerAutoLearning: async (params) => {
      set({ rlLoading: true });
      try {
        const result = await triggerAutoLearning(params);
        const history = [result, ...get().autoLearningHistory].slice(0, 50);
        set({ autoLearningHistory: history, rlLoading: false });
        return result;
      } catch (e) {
        set({ error: String(e), rlLoading: false });
        return null;
      }
    },

    getLatestAutoLearning: () => {
      return get().autoLearningHistory[0];
    },
  }),
);

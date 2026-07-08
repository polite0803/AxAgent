// =====================================================================
// Quant: backtestStore
// =====================================================================
//
// 持有：
//  - 当前 run 配置表单（受控状态）
//  - 已提交的 run 历史（轻量：仅存 run + 顶层指标 + WF 摘要）
//  - 当前 run 详情（result_json 完整加载）
//  - 对比结果（runIds 列表 + runs 详情 + bestBy）
//
// 通过 invoke 调用 `quant_backtest_run` / `quant_metrics_compare`。
// =====================================================================

import { create } from "zustand";

import { invoke } from "@/lib/invoke";
import type {
  BacktestResult,
  BacktestRunRequest,
  BacktestRunResponse,
  MetricsCompareResponse,
  QuantRun,
} from "@/types";

interface BacktestState {
  // 表单（受控）
  draftRequest: Partial<BacktestRunRequest>;

  // 当前 run
  currentRun: BacktestRunResponse | null;
  currentBacktestResult: BacktestResult | null;
  isRunning: boolean;
  isLoadingResult: boolean;

  // 历史（轻量）
  recentRuns: QuantRun[];

  // 对比
  compare: MetricsCompareResponse | null;
  isComparing: boolean;

  error: string | null;

  // actions
  setDraft: (patch: Partial<BacktestRunRequest>) => void;
  resetDraft: () => void;
  runBacktest: (request: BacktestRunRequest) => Promise<BacktestRunResponse>;
  loadRun: (runId: string) => Promise<BacktestResult | null>;
  setRecentRuns: (runs: QuantRun[]) => void;
  compareRuns: (runIds: string[]) => Promise<MetricsCompareResponse>;
  clearCompare: () => void;
  reset: () => void;
}

const INITIAL: Omit<
  BacktestState,
  | "setDraft"
  | "resetDraft"
  | "runBacktest"
  | "loadRun"
  | "setRecentRuns"
  | "compareRuns"
  | "clearCompare"
  | "reset"
> = {
  draftRequest: {
    initialCash: 1_000_000,
    walkForwardEnabled: true,
    walkForwardForceOff: false,
    matcherConfig: null,
    params: {},
    name: null,
  },
  currentRun: null,
  currentBacktestResult: null,
  isRunning: false,
  isLoadingResult: false,
  recentRuns: [],
  compare: null,
  isComparing: false,
  error: null,
};

export const useBacktestStore = create<BacktestState>((set, get) => ({
  ...INITIAL,

  setDraft: (patch) => set((s) => ({ draftRequest: { ...s.draftRequest, ...patch } })),

  resetDraft: () => set({ draftRequest: INITIAL.draftRequest }),

  runBacktest: async (request: BacktestRunRequest) => {
    set({ isRunning: true, error: null });
    try {
      const response = await invoke<BacktestRunResponse>("quant_backtest_run", {
        request,
      });
      // 缓存 result（解 result_json）
      const result: BacktestResult | null = response.run.resultJson
        ? (() => {
          try {
            return JSON.parse(response.run.resultJson) as BacktestResult;
          } catch {
            return null;
          }
        })()
        : null;
      // 记录 recent
      const recent = [response.run, ...get().recentRuns].slice(0, 20);
      set({
        currentRun: response,
        currentBacktestResult: result,
        recentRuns: recent,
        isRunning: false,
      });
      return response;
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      set({ isRunning: false, error: msg });
      throw new Error(msg, { cause: e });
    }
  },

  loadRun: async (runId: string) => {
    set({ isLoadingResult: true, error: null });
    try {
      const run = await invoke<QuantRun | null>("quant_run_get", { runId });
      if (!run) {
        set({ isLoadingResult: false });
        return null;
      }
      const result: BacktestResult | null = run.resultJson
        ? JSON.parse(run.resultJson)
        : null;
      set({
        currentBacktestResult: result,
        isLoadingResult: false,
      });
      return result;
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      set({ isLoadingResult: false, error: msg });
      return null;
    }
  },

  setRecentRuns: (runs) => set({ recentRuns: runs }),

  compareRuns: async (runIds: string[]) => {
    set({ isComparing: true, error: null });
    try {
      const compare = await invoke<MetricsCompareResponse>("quant_metrics_compare", {
        runIds,
      });
      set({ compare, isComparing: false });
      return compare;
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      set({ isComparing: false, error: msg });
      throw new Error(msg, { cause: e });
    }
  },

  clearCompare: () => set({ compare: null }),

  reset: () => set(INITIAL),
}));

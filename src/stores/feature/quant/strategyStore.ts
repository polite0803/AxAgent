// =====================================================================
// Quant: strategyStore
// =====================================================================
//
// 持有：
//  - 策略元数据列表（DB 已注册 + 内置 5 项默认）
//  - Rhai 编辑器草稿（与 store 解耦，组件级 useState 即可）
//  - 注册 / 刷新 actions
//
// 通过 invoke 调用 Rust `quant_strategies_list` / `quant_strategy_register_rhai`。
// =====================================================================

import { create } from "zustand";

import { invoke } from "@/lib/invoke";
import type { RegisterRhaiRequest, StrategyMeta } from "@/types";

interface StrategyState {
  strategies: StrategyMeta[];
  isLoading: boolean;
  isRegistering: boolean;
  error: string | null;
  lastFetchedAt: number | null;

  loadStrategies: (force?: boolean) => Promise<void>;
  registerRhai: (request: RegisterRhaiRequest) => Promise<StrategyMeta>;
  getStrategyById: (id: string) => StrategyMeta | undefined;
  reset: () => void;
}

const INITIAL: Omit<StrategyState, "loadStrategies" | "registerRhai" | "getStrategyById" | "reset"> = {
  strategies: [],
  isLoading: false,
  isRegistering: false,
  error: null,
  lastFetchedAt: null,
};

export const useStrategyStore = create<StrategyState>((set, get) => ({
  ...INITIAL,

  loadStrategies: async (force = false) => {
    const state = get();
    if (state.isLoading) { return; }
    if (!force && state.strategies.length > 0) { return; }
    set({ isLoading: true, error: null });
    try {
      const result = await invoke<StrategyMeta[]>("quant_strategies_list");
      set({
        strategies: result,
        isLoading: false,
        lastFetchedAt: Date.now(),
      });
    } catch (e) {
      set({
        isLoading: false,
        error: e instanceof Error ? e.message : String(e),
      });
    }
  },

  registerRhai: async (request: RegisterRhaiRequest) => {
    set({ isRegistering: true, error: null });
    try {
      const newStrategy = await invoke<StrategyMeta>(
        "quant_strategy_register_rhai",
        { request },
      );
      // 追加 / 替换到列表
      const list = get().strategies;
      const idx = list.findIndex((s) => s.id === newStrategy.id);
      if (idx >= 0) {
        const next = [...list];
        next[idx] = newStrategy;
        set({ strategies: next, isRegistering: false });
      } else {
        set({
          strategies: [newStrategy, ...list],
          isRegistering: false,
        });
      }
      return newStrategy;
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      set({ isRegistering: false, error: msg });
      throw new Error(msg, { cause: e });
    }
  },

  getStrategyById: (id: string) => get().strategies.find((s) => s.id === id),

  reset: () => set(INITIAL),
}));

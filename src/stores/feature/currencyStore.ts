// SPDX-License-Identifier: AGPL-3.0-only

import { useCallback } from "react";
import { create } from "zustand";
import { persist } from "zustand/middleware";

/** 人民币货币符号 */
const CNY_SYMBOL = "¥";

/** 默认 USD → CNY 汇率（约 2025 年均值） */
const DEFAULT_USD_TO_CNY_RATE = 7.2;

interface CurrencyState {
  /** 1 USD 折合多少人民币 */
  usdToCnyRate: number;
  setUsdToCnyRate: (rate: number) => void;
}

/**
 * 货币显示偏好：后端成本字段以 USD 计价，前端统一按此汇率换算为人民币展示。
 * 汇率持久化在 localStorage，用户可在偏好设置中调整。
 */
export const useCurrencyStore = create<CurrencyState>()(
  persist(
    (set) => ({
      usdToCnyRate: DEFAULT_USD_TO_CNY_RATE,
      setUsdToCnyRate: (rate) => {
        // 仅接受正数汇率，避免误置零导致成本归零
        if (Number.isFinite(rate) && rate > 0) {
          set({ usdToCnyRate: rate });
        }
      },
    }),
    {
      name: "axagent-currency-storage",
      partialize: (state) => ({ usdToCnyRate: state.usdToCnyRate }),
    },
  ),
);

/**
 * 将 USD 金额格式化为人民币字符串（纯函数，读取当前 store 状态）。
 * 用于非 React 场景或不需要响应汇率变化的场合。
 */
export function formatCny(usd: number, decimals = 2): string {
  const rate = useCurrencyStore.getState().usdToCnyRate;
  return `${CNY_SYMBOL}${(usd * rate).toFixed(decimals)}`;
}

/**
 * React Hook：返回一个格式化函数，汇率变化时自动重算。
 * 组件中使用以确保用户调整汇率后展示即时刷新。
 */
export function useFormatCny(): (usd: number, decimals?: number) => string {
  const rate = useCurrencyStore((s) => s.usdToCnyRate);
  return useCallback(
    (usd: number, decimals = 2) => `${CNY_SYMBOL}${(usd * rate).toFixed(decimals)}`,
    [rate],
  );
}

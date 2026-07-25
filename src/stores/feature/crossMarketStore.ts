// SPDX-License-Identifier: AGPL-3.0-only
/**
 * G1 跨市场数据接入 Zustand store
 *
 * 负责：
 * - 美股 / 港股行情与 K 线拉取
 * - 国际基准指数 K 线（标普 500 / 纳指 / 恒生 / 上证等）
 * - 外汇 K 线（USD/CNY、HKD/CNY 等）
 *
 * 命令清单（与后端 commands/stock_analysis.rs 对齐）：
 * - get_international_stock_quote / get_international_stock_kline
 * - get_benchmark_kline
 * - get_forex_kline
 */

import { invoke } from "@/lib/invoke";
import type { KLine, StockQuote } from "@/types";
import { create } from "zustand";

export interface CrossMarketState {
  // 国际股票行情缓存（key: stock_code）
  intlQuotes: Record<string, StockQuote>;
  // 国际股票 K 线缓存（key: `${code}:${period}:${limit}`）
  intlKlines: Record<string, KLine[]>;
  // 基准指数 K 线缓存（key: `${benchmark}:${period}:${limit}`）
  benchmarkKlines: Record<string, KLine[]>;
  // 外汇 K 线缓存（key: `${pair}:${period}:${limit}`）
  forexKlines: Record<string, KLine[]>;

  loadingQuote: boolean;
  loadingKline: boolean;
  loadingBenchmark: boolean;
  loadingForex: boolean;
  error: string | null;

  // ── Actions ──
  fetchIntlQuote: (stockCode: string, force?: boolean) => Promise<StockQuote | null>;
  fetchIntlKline: (
    stockCode: string,
    period?: string,
    limit?: number,
  ) => Promise<KLine[] | null>;
  fetchBenchmarkKline: (
    benchmarkCode: string,
    period?: string,
    limit?: number,
  ) => Promise<KLine[] | null>;
  fetchForexKline: (
    pair: string,
    period?: string,
    limit?: number,
  ) => Promise<KLine[] | null>;
  clearError: () => void;
}

export const useCrossMarketStore = create<CrossMarketState>((set, get) => ({
  intlQuotes: {},
  intlKlines: {},
  benchmarkKlines: {},
  forexKlines: {},

  loadingQuote: false,
  loadingKline: false,
  loadingBenchmark: false,
  loadingForex: false,
  error: null,

  fetchIntlQuote: async (stockCode, force) => {
    if (!force && get().intlQuotes[stockCode]) {
      return get().intlQuotes[stockCode];
    }
    set({ loadingQuote: true, error: null });
    try {
      const quote = await invoke<StockQuote>("get_international_stock_quote", {
        stockCode,
      });
      set((s) => ({
        intlQuotes: { ...s.intlQuotes, [stockCode]: quote },
        loadingQuote: false,
      }));
      return quote;
    } catch (e) {
      set({ loadingQuote: false, error: String(e) });
      return null;
    }
  },

  fetchIntlKline: async (stockCode, period = "daily", limit = 120) => {
    const key = `${stockCode}:${period}:${limit}`;
    if (get().intlKlines[key]) {
      return get().intlKlines[key];
    }
    set({ loadingKline: true, error: null });
    try {
      const klines = await invoke<KLine[]>("get_international_stock_kline", {
        stockCode,
        period,
        limit,
      });
      set((s) => ({
        intlKlines: { ...s.intlKlines, [key]: klines },
        loadingKline: false,
      }));
      return klines;
    } catch (e) {
      set({ loadingKline: false, error: String(e) });
      return null;
    }
  },

  fetchBenchmarkKline: async (benchmarkCode, period = "daily", limit = 120) => {
    const key = `${benchmarkCode}:${period}:${limit}`;
    if (get().benchmarkKlines[key]) {
      return get().benchmarkKlines[key];
    }
    set({ loadingBenchmark: true, error: null });
    try {
      const klines = await invoke<KLine[]>("get_benchmark_kline", {
        benchmarkCode,
        period,
        limit,
      });
      set((s) => ({
        benchmarkKlines: { ...s.benchmarkKlines, [key]: klines },
        loadingBenchmark: false,
      }));
      return klines;
    } catch (e) {
      set({ loadingBenchmark: false, error: String(e) });
      return null;
    }
  },

  fetchForexKline: async (pair, period = "daily", limit = 120) => {
    const key = `${pair}:${period}:${limit}`;
    if (get().forexKlines[key]) {
      return get().forexKlines[key];
    }
    set({ loadingForex: true, error: null });
    try {
      const klines = await invoke<KLine[]>("get_forex_kline", {
        pair,
        period,
        limit,
      });
      set((s) => ({
        forexKlines: { ...s.forexKlines, [key]: klines },
        loadingForex: false,
      }));
      return klines;
    } catch (e) {
      set({ loadingForex: false, error: String(e) });
      return null;
    }
  },

  clearError: () => set({ error: null }),
}));

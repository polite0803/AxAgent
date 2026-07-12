// SPDX-License-Identifier: AGPL-3.0-only

import { invoke } from "@/lib/invoke";
import type { PipelineResult, PipelineRun, PipelineStepEvent } from "@/types";
import { create } from "zustand";

interface PipelineState {
  pipelineRuns: PipelineRun[];
  currentRun: PipelineResult | null;
  isRunning: boolean;
  stepEvents: PipelineStepEvent[];
  fetchHistory: (limit?: number) => Promise<void>;
  runPipeline: (asOfDate?: string) => Promise<PipelineResult>;
  getRunDetail: (runId: string) => Promise<PipelineResult>;
  addStepEvent: (event: PipelineStepEvent) => void;
  clearStepEvents: () => void;
}

export const usePipelineStore = create<PipelineState>((set, get) => ({
  pipelineRuns: [],
  currentRun: null,
  isRunning: false,
  stepEvents: [],
  fetchHistory: async (limit = 20) => {
    try {
      const runs = await invoke<PipelineRun[]>("get_pipeline_history", { limit });
      set({ pipelineRuns: runs });
    } catch (e) {
      console.error("[pipelineStore] 获取历史失败:", e);
    }
  },
  runPipeline: async (asOfDate?: string) => {
    set({ isRunning: true, stepEvents: [] });
    try {
      const result = await invoke<PipelineResult>("run_stock_pipeline", {
        asOfDate: asOfDate ?? null,
      });
      set({ currentRun: result, isRunning: false });
      await get().fetchHistory();
      return result;
    } catch (e) {
      set({ isRunning: false });
      throw e;
    }
  },
  getRunDetail: async (runId: string) => {
    return await invoke<PipelineResult>("get_pipeline_run_detail", { runId });
  },
  addStepEvent: (event) => {
    set((state) => ({
      stepEvents: [...state.stepEvents, event],
    }));
  },
  clearStepEvents: () => {
    set({ stepEvents: [] });
  },
}));

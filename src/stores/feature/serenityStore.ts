import { create } from "zustand";

// ── 类型 ──

export type StepStage =
  | "loading"
  | "scanning"
  | "decomposing"
  | "identifying"
  | "mapping"
  | "saving"
  | "done"
  | "error";

export interface Catalyst {
  type: string;
  description: string;
  expected_timeframe: string;
  confidence: number;
  trigger_condition?: string;
}

export interface ExitSignals {
  technology_disruption_risk?: string;
  capacity_oversupply_risk?: string;
  new_entrant_risk?: string;
  demand_slowdown_risk?: string;
  overall_exit_urgency?: string;
}

export interface AttentionMetrics {
  coverage_change_3m?: string;
  search_heat?: string;
  relative_volume?: string;
  consensus_gap?: string;
  attention_score?: number;
}

export interface SerenityCandidate {
  stockCode?: string;
  stock_name?: string;
  stockName?: string;
  stock_code?: string;
  relevance?: string;
  serenityScore?: number;
  serenity_score?: number;
  confidence?: number;
  bottleneckProduct?: string;
  bottleneck_product?: string;
  primaryRisk?: string;
  primary_risk?: string;
  catalysts?: Catalyst[];
  exit_signals?: ExitSignals;
  exitSignals?: ExitSignals;
  attention_metrics?: AttentionMetrics;
  attentionMetrics?: AttentionMetrics;
}

export interface TrendInfo {
  trendName?: string;
  trend_name?: string;
  bottleneck_candidate?: string;
  confidence?: number;
}

/// 单个节点的执行日志
export interface StepLog {
  nodeId: string;
  status: string;
  output?: unknown;
  error?: string;
  elapsedMs?: number;
  totalNodes?: number;
  completedNodes?: number;
  timestamp: number;
}

// ── Store ──

interface SerenityState {
  running: boolean;
  stage: StepStage;
  candidates: SerenityCandidate[];
  trends: TrendInfo[];
  error: string | null;
  completedNodes: number;
  totalNodes: number;
  steps: StepLog[];
  currentNodeId: string | null;
  /**
   * 工作流结束时，a-candidate-mapper 节点返回的"为什么没有候选"原因。
   * 当上游三个瓶颈节点均返回 data_gaps=true 时，模型会在 arguments.summary
   * 给出反幻觉说明；此字段在 candidates 为空时供前端展示，避免用户看到
   * 一个无解释的 Empty 占位。
   */
  emptyReason: string | null;

  setRunning: (v: boolean) => void;
  setStage: (s: StepStage) => void;
  setCandidates: (c: SerenityCandidate[]) => void;
  setTrends: (t: TrendInfo[]) => void;
  setError: (e: string | null) => void;
  setCompletedNodes: (n: number) => void;
  setTotalNodes: (n: number) => void;
  addStep: (log: StepLog) => void;
  setCurrentNode: (id: string | null) => void;
  setEmptyReason: (r: string | null) => void;
  clearSteps: () => void;
  reset: () => void;
}

const initialState = {
  running: false,
  stage: "done" as StepStage,
  candidates: [] as SerenityCandidate[],
  trends: [] as TrendInfo[],
  error: null as string | null,
  completedNodes: 0,
  totalNodes: 0,
  steps: [] as StepLog[],
  currentNodeId: null as string | null,
  emptyReason: null as string | null,
};

export const useSerenityStore = create<SerenityState>((set) => ({
  ...initialState,
  setRunning: (v) => set({ running: v }),
  setStage: (s) => set({ stage: s }),
  setCandidates: (c) => set({ candidates: c }),
  setTrends: (t) => set({ trends: t }),
  setError: (e) => set({ error: e }),
  setCompletedNodes: (n) => set({ completedNodes: n }),
  setTotalNodes: (n) => set({ totalNodes: n }),
  addStep: (log) =>
    set((s) => {
      const idx = s.steps.findIndex((item) => item.nodeId === log.nodeId);
      if (idx >= 0) {
        // upsert：同节点更新状态 + output/error/elapsedMs，不追加重复
        const updated = [...s.steps];
        updated[idx] = { ...updated[idx], ...log };
        return { steps: updated };
      }
      return { steps: [...s.steps, log] };
    }),
  setCurrentNode: (id) => set({ currentNodeId: id }),
  setEmptyReason: (r) => set({ emptyReason: r }),
  clearSteps: () => set({ steps: [], currentNodeId: null }),
  reset: () => set(initialState),
}));

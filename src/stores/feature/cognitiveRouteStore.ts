// SPDX-License-Identifier: AGPL-3.0-only

import type {
  CognitiveCandidateSummary,
  CognitiveExecutionMode,
  CognitiveQueryResponse,
  CognitiveRouteStageView,
  CognitiveSelectedAgentProfile,
} from "@/types";
import { create } from "zustand";

/**
 * 认知编排路由观测状态
 *
 * 保存最近一次 cognitive_query 的三层路由决策结果（stageRecords / routePath /
 * circuitBroken / totalElapsedMs / isLlmFallback 等），供右侧边栏「路由观测」
 * 面板展示。仅保留最近一次，切换会话/再次发送时覆盖。
 */
export interface CognitiveRouteObservation {
  /** 所属会话 */
  conversationId: string;
  /** 三层路由地址（确定性路径），如 "invest/stock_analysis/tech" */
  routePath: string;
  /** 业务域 */
  domain: string;
  /** 功能集群 */
  cluster: string;
  /** 具体能力/工作流 ID */
  capabilityId: string;
  /** 路由置信度（0.0 - 1.0） */
  confidence: number;
  /** 是否通过 LLM 兜底 */
  isLlmFallback: boolean;
  /** 是否触发熔断 */
  circuitBroken: boolean;
  /** 熔断原因 */
  circuitBreakReason: string | null;
  /** 备选路径 */
  fallbackPath: string | null;
  /** 候选列表（Top-K） */
  candidates: string[];
  /** 候选能力详情（含名称/描述/置信度/种类，供前端展示） */
  candidateDetails: CognitiveCandidateSummary[];
  /** 熔断过滤数量（RAR 原始候选数 - 最终候选数，0 表示无过滤） */
  filteredCount: number;
  /** 执行模式 */
  executionMode: CognitiveExecutionMode;
  /** 选中工作流的可读名称（未命中工作流时为 null） */
  selectedWorkflowName: string | null;
  /** 选中的执行专家（Agent 执行路径；未走 Agent 路径时为 null） */
  selectedAgentProfile: CognitiveSelectedAgentProfile | null;
  /** 各阶段执行记录 */
  stageRecords: CognitiveRouteStageView[];
  /** 总耗时（毫秒） */
  totalElapsedMs: number;
  /** 记录时间戳 */
  recordedAt: number;
}

interface CognitiveRouteState {
  /** 最近一次认知路由观测（null 表示尚无观测） */
  observation: CognitiveRouteObservation | null;
  /** 记录一次观测（覆盖旧值） */
  recordObservation: (
    conversationId: string,
    response: CognitiveQueryResponse,
  ) => void;
  /** 清空观测 */
  reset: () => void;
}

export const useCognitiveRouteStore = create<CognitiveRouteState>((set) => ({
  observation: null,

  recordObservation: (conversationId, response) => {
    set({
      observation: {
        conversationId,
        routePath: response.routePath,
        domain: response.domain,
        cluster: response.cluster,
        capabilityId: response.capabilityId,
        confidence: response.confidence,
        isLlmFallback: response.isLlmFallback,
        circuitBroken: response.circuitBroken,
        circuitBreakReason: response.circuitBreakReason ?? null,
        fallbackPath: response.fallbackPath ?? null,
        candidates: response.candidates ?? [],
        candidateDetails: response.candidateDetails ?? [],
        filteredCount: response.filteredCount ?? 0,
        executionMode: response.executionMode,
        selectedWorkflowName: response.selectedWorkflowName ?? null,
        selectedAgentProfile: response.selectedAgentProfile ?? null,
        stageRecords: response.stageRecords ?? [],
        totalElapsedMs: response.totalElapsedMs,
        recordedAt: Date.now(),
      },
    });
  },

  reset: () => set({ observation: null }),
}));

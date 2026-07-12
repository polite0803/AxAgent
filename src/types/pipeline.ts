// SPDX-License-Identifier: AGPL-3.0-only

/** 管道执行结果 */
export interface PipelineResult {
  runId: string;
  runDate: string;
  status: string;
  candidates: string[];
  newAnalyses: PipelineAnalysisSummary[];
  reassessed: PipelineAnalysisSummary[];
  summary: PipelineSummary;
  error: string | null;
}

/**
 * 单只股票分析摘要（管道上下文）。
 * 与 stock-analysis.ts 中的 AnalysisSummary 字段不同，故独立命名为 PipelineAnalysisSummary。
 */
export interface PipelineAnalysisSummary {
  stockCode: string;
  stockName: string;
  status: string;
  analysisId: string | null;
  action: string | null;
  confidence: number | null;
  error: string | null;
}

/** 管道汇总报告 */
export interface PipelineSummary {
  pipelineDate: string;
  discovery: { candidatesFound: number };
  analysis: {
    newAnalyzed: number;
    newFailed: number;
    reassessed: number;
    reassessFailed: number;
  };
  decisions: {
    buy: number;
    hold: number;
    watch: number;
    sell: number;
  };
  reflectionScheduled: number;
  note: string;
}

/** 管道历史记录（列表项） */
export interface PipelineRun {
  id: string;
  runDate: string;
  asOfDate: string | null;
  status: string;
  startedAt: number;
  completedAt: number | null;
  errorMessage: string | null;
  summary: PipelineSummary | null;
}

/** 管道进度事件 */
export interface PipelineStepEvent {
  step: string;
  detail: string;
  timestamp: number;
}

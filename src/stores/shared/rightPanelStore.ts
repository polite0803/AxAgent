// SPDX-License-Identifier: AGPL-3.0-only

import type { ChartData } from "@/components/chat/ChartInterpreter";
import type { Citation } from "@/types";
import { create } from "zustand";

/** UISnapshotViewer 元素 */
export interface SnapshotElement {
  element_type: string;
  label: string | null;
  bounding_box: { x: number; y: number; width: number; height: number } | null;
  actionable: boolean;
}

/** ResearchSources 数据（匹配 researchUtils.ts 的 SearchResult） */
export interface ResearchSourceItem {
  id: string;
  sourceType: string;
  url: string;
  title: string;
  snippet: string;
  credibilityScore: number | null;
  relevanceScore: number;
}

/** 研究报告数据（匹配 ReportViewer 的 ResearchReport 类型） */
export interface ResearchReport {
  id: string;
  topic: string;
  content: string;
  citations: Citation[];
  summary: string;
  createdAt?: string;
}

/** 右侧面板跨组件共享数据 */
interface RightPanelState {
  chartData: ChartData | null;
  chartRawAnalysis: string;
  setChartResult: (data: ChartData | null, rawAnalysis: string) => void;

  snapshotElements: SnapshotElement[];
  snapshotDescription: string;
  setSnapshotResult: (elements: SnapshotElement[], description: string) => void;

  researchSources: ResearchSourceItem[];
  setResearchSources: (sources: ResearchSourceItem[]) => void;

  report: ResearchReport | null;
  setReport: (report: ResearchReport | null) => void;
}

export const useRightPanelStore = create<RightPanelState>((set) => ({
  chartData: null,
  chartRawAnalysis: "",
  setChartResult: (chartData, chartRawAnalysis) => set({ chartData, chartRawAnalysis }),

  snapshotElements: [],
  snapshotDescription: "",
  setSnapshotResult: (snapshotElements, snapshotDescription) => set({ snapshotElements, snapshotDescription }),

  researchSources: [],
  setResearchSources: (researchSources) => set({ researchSources }),

  report: null,
  setReport: (report) => set({ report }),
}));

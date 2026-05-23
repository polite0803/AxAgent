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

/** 右侧面板跨组件共享数据 */
interface RightPanelState {
  chartData: Record<string, unknown> | null;
  chartRawAnalysis: string;
  setChartResult: (data: Record<string, unknown> | null, rawAnalysis: string) => void;

  snapshotElements: SnapshotElement[];
  snapshotDescription: string;
  setSnapshotResult: (elements: SnapshotElement[], description: string) => void;

  researchSources: ResearchSourceItem[];
  setResearchSources: (sources: ResearchSourceItem[]) => void;

  predictionContext: Record<string, unknown>;
  setPredictionContext: (ctx: Record<string, unknown>) => void;
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

  predictionContext: {},
  setPredictionContext: (predictionContext) => set({ predictionContext }),
}));

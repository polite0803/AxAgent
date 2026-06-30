// SPDX-License-Identifier: AGPL-3.0-only

import type { Citation, PageKey, SettingsSection } from "@/types";
import type { ChartData } from "@/components/chat/ChartInterpreter";
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

/** 桌面分辨率布局模式 */
export type DeviceLayout = "mobile" | "tablet" | "desktop";

interface UIState {
  activePage: PageKey;
  previousPage: PageKey;
  sidebarCollapsed: boolean;
  settingsSection: SettingsSection;
  selectedProviderId: string | null;
  workflowEditorOpen: boolean;
  /** 根据窗口宽度自动检测的布局模式 */
  deviceLayout: DeviceLayout;
  setActivePage: (page: PageKey) => void;
  enterSettings: () => void;
  exitSettings: () => void;
  toggleSidebar: () => void;
  setSettingsSection: (section: SettingsSection) => void;
  setSelectedProviderId: (id: string | null) => void;
  openWorkflowEditor: () => void;
  closeWorkflowEditor: () => void;
  /** 设置布局模式（启动时由 useResponsive hook 自动调用） */
  setDeviceLayout: (layout: DeviceLayout) => void;

  // --- Right Panel (merged from rightPanelStore) ---
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

  // --- Chat Workspace (merged from chatWorkspaceStore) ---
  selectedArtifactId: string | null;
  comparedMessageIds: [string, string] | null;

  selectArtifact: (id: string | null) => void;
  startCompare: (messageIds: [string, string]) => void;
  clearCompare: () => void;
}

/** 根据窗口宽度解析布局模式 */
export function resolveDeviceLayout(width: number): DeviceLayout {
  if (width < 600) { return "mobile"; }
  if (width < 900) { return "tablet"; }
  return "desktop";
}

export const useUIStore = create<UIState>((set, get) => ({
  activePage: "chat",
  previousPage: "chat",
  sidebarCollapsed: true,
  settingsSection: "general",
  selectedProviderId: null,
  workflowEditorOpen: false,
  deviceLayout: resolveDeviceLayout(window.innerWidth),

  // --- Right Panel state ---
chartData: null,
  chartRawAnalysis: "",
  setChartResult: (chartData, chartRawAnalysis) => set({ chartData, chartRawAnalysis }),

  snapshotElements: [],
  snapshotDescription: "",
  setSnapshotResult: (snapshotElements, snapshotDescription) => set({ snapshotElements, snapshotDescription }),

  researchSources: [],
  setResearchSources: (researchSources) => set({ researchSources }),

  report: null,
  setReport: (report) => set({ report }),,

  // --- Chat Workspace state ---
selectedArtifactId: null,
  comparedMessageIds: null,

  selectArtifact: (id) => set({ selectedArtifactId: id }),
  startCompare: (messageIds) => set({ comparedMessageIds: messageIds }),
  clearCompare: () => set({ comparedMessageIds: null }),
  setActivePage: (page) => set({ activePage: page }),
  enterSettings: () => {
    const current = get().activePage;
    if (current !== "settings") {
      set({ previousPage: current, activePage: "settings" });
    }
  },
  exitSettings: () => {
    const prev = get().previousPage;
    set({ activePage: prev });
  },
  toggleSidebar: () => set((s) => ({ sidebarCollapsed: !s.sidebarCollapsed })),
  setSettingsSection: (section) => set({ settingsSection: section }),
  setSelectedProviderId: (id) => set({ selectedProviderId: id }),
  openWorkflowEditor: () => {
    set({ settingsSection: "workflow", workflowEditorOpen: true });
    const current = get().activePage;
    if (current !== "settings") {
      set({ previousPage: current, activePage: "settings" });
    }
  },
  closeWorkflowEditor: () => set({ workflowEditorOpen: false }),
  setDeviceLayout: (layout) => {
    set((s) => {
      const updates: Partial<UIState> = { deviceLayout: layout };
      // 布局模式切换时 → 小屏自动折叠，大屏自动展开
      if (layout !== s.deviceLayout) {
        updates.sidebarCollapsed = layout === "mobile" || layout === "tablet";
      }
      return updates;
    });
  },
}));

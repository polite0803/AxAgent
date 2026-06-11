// SPDX-License-Identifier: AGPL-3.0-only

import type { PageKey, SettingsSection } from "@/types";
import { create } from "zustand";

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

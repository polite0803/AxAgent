// SPDX-License-Identifier: AGPL-3.0-only

import { create } from "zustand";
import { persist } from "zustand/middleware";

/** 工作区视图类型（桌面端 6 视图，移动端 4 核心 + more） */
export type WorkspaceView =
  | "analysis"
  | "monitor"
  | "trade"
  | "backtest"
  | "compare"
  | "review"
  | "more";

/** 用户信息密度模式 */
export type UserMode = "simple" | "professional";

/** 最近访问的股票记录 */
export interface RecentStock {
  code: string;
  name: string;
  /** 最后访问时间戳（ms） */
  visitedAt: number;
}

interface WorkspaceState {
  /** 当前股票代码（跨视图共享） */
  currentStockCode: string | null;
  /** 当前股票名称 */
  currentStockName: string | null;
  /** 当前视图 Tab */
  currentView: WorkspaceView;
  /** 用户模式（简洁/专业），默认 simple */
  userMode: UserMode;
  /** 左栏（股票切换器）折叠状态 — 桌面端默认折叠 */
  leftSidebarCollapsed: boolean;
  /** 右栏（上下文侧栏）折叠状态 — 桌面端默认折叠 */
  rightSidebarCollapsed: boolean;
  /** 最近访问的股票列表（最多 10 条） */
  recentStocks: RecentStock[];

  // ── actions ──
  /** 设置当前股票，并写入最近访问列表 */
  setCurrentStock: (code: string, name: string) => void;
  /** 清空当前股票 */
  clearCurrentStock: () => void;
  /** 切换视图 Tab */
  setCurrentView: (view: WorkspaceView) => void;
  /** 设置用户模式 */
  setUserMode: (mode: UserMode) => void;
  /** 切换用户模式 */
  toggleUserMode: () => void;
  /** 切换左栏折叠 */
  toggleLeftSidebar: () => void;
  /** 切换右栏折叠 */
  toggleRightSidebar: () => void;
  /** 从最近列表移除指定股票 */
  removeRecentStock: (code: string) => void;
}

const MAX_RECENT = 10;

export const useWorkspaceStore = create<WorkspaceState>()(
  persist(
    (set) => ({
      currentStockCode: null,
      currentStockName: null,
      currentView: "analysis",
      userMode: "simple",
      leftSidebarCollapsed: true,
      rightSidebarCollapsed: true,
      recentStocks: [],

      setCurrentStock: (code, name) =>
        set((state) => {
          // 更新最近列表：去重 + 置顶 + 截断
          const filtered = state.recentStocks.filter((s) => s.code !== code);
          const newRecent: RecentStock[] = [
            { code, name, visitedAt: Date.now() },
            ...filtered,
          ].slice(0, MAX_RECENT);
          return {
            currentStockCode: code,
            currentStockName: name,
            recentStocks: newRecent,
          };
        }),

      clearCurrentStock: () => set({ currentStockCode: null, currentStockName: null }),

      setCurrentView: (view) => set({ currentView: view }),

      setUserMode: (mode) => set({ userMode: mode }),

      toggleUserMode: () =>
        set((state) => ({
          userMode: state.userMode === "simple" ? "professional" : "simple",
        })),

      toggleLeftSidebar: () => set((state) => ({ leftSidebarCollapsed: !state.leftSidebarCollapsed })),

      toggleRightSidebar: () => set((state) => ({ rightSidebarCollapsed: !state.rightSidebarCollapsed })),

      removeRecentStock: (code) =>
        set((state) => ({
          recentStocks: state.recentStocks.filter((s) => s.code !== code),
        })),
    }),
    {
      name: "axagent-stock-workspace",
      // 只持久化用户偏好，不持久化瞬态的 currentStockCode/Name（由路由参数驱动）
      partialize: (state) => ({
        userMode: state.userMode,
        recentStocks: state.recentStocks,
        leftSidebarCollapsed: state.leftSidebarCollapsed,
        rightSidebarCollapsed: state.rightSidebarCollapsed,
      }),
    },
  ),
);

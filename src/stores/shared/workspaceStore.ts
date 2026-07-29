// SPDX-License-Identifier: AGPL-3.0-only

import { create } from "zustand";

/**
 * 工作台功能 Tab 类型。
 * 对话页作为核心枢纽，其他功能以 Tab 形式嵌入。
 */
export type WorkspaceTab =
  | "chat"
  | "dashboard"
  | "workflow"
  | "terminal"
  | "knowledge"
  | "files";

interface WorkspaceState {
  /** 当前激活的功能 Tab */
  activeTab: WorkspaceTab;
  /** 切换功能 Tab */
  setActiveTab: (tab: WorkspaceTab) => void;
}

/**
 * 工作台 Tab 状态。
 * 管理 /chat 路由下功能 Tab 的切换（对话/仪表盘/工作流/终端/知识源/文件）。
 */
export const useWorkspaceStore = create<WorkspaceState>((set) => ({
  activeTab: "chat",
  setActiveTab: (tab) => set({ activeTab: tab }),
}));

// SPDX-License-Identifier: AGPL-3.0-only

import { create } from "zustand";

/** Agent Panel 当前活跃标签页 */
export type AgentPanelTab = "chat" | "execution" | "skill" | "nl-generation";

/** 页面选中内容的元数据 */
export interface AgentSelection {
  type: "file" | "node" | "edge" | "document" | "memory" | "setting" | "conversation";
  id: string;
  label: string;
  metadata?: Record<string, unknown>;
}

/** 最近用户操作记录 */
export interface AgentRecentAction {
  action: string;
  timestamp: number;
  detail?: string;
}

/** Agent 页面上下文 */
export interface AgentContext {
  /** 当前页面标识 */
  page: string;
  /** 当前 URL */
  url: string;
  /** 当前选中内容（可选） */
  selection?: AgentSelection;
  /** 最近操作记录 */
  recentActions?: AgentRecentAction[];
}

/** 面板最小/最大/默认宽度 */
export const PANEL_MIN_WIDTH = 320;
export const PANEL_MAX_WIDTH = 600;
const PANEL_DEFAULT_WIDTH = 400;

/** localStorage 持久化的键 */
const STORAGE_KEY_WIDTH = "axagent:agentPanel:width";
const STORAGE_KEY_MINI = "axagent:agentPanel:miniMode";
const STORAGE_KEY_TAB = "axagent:agentPanel:activeTab";

/** 从 localStorage 读取持久化值 */
function loadPersistedWidth(): number {
  try {
    const raw = localStorage.getItem(STORAGE_KEY_WIDTH);
    if (raw !== null) {
      const val = Number(raw);
      if (!Number.isNaN(val) && val >= PANEL_MIN_WIDTH && val <= PANEL_MAX_WIDTH) {
        return val;
      }
    }
  } catch {
    // localStorage 不可用，忽略
  }
  return PANEL_DEFAULT_WIDTH;
}

function loadPersistedMiniMode(): boolean {
  try {
    return localStorage.getItem(STORAGE_KEY_MINI) === "true";
  } catch {
    return false;
  }
}

function loadPersistedTab(): AgentPanelTab {
  try {
    const raw = localStorage.getItem(STORAGE_KEY_TAB);
    if (raw === "chat" || raw === "execution" || raw === "skill" || raw === "nl-generation") {
      return raw;
    }
  } catch {
    // 忽略
  }
  return "chat";
}

function persistWidth(w: number): void {
  try {
    localStorage.setItem(STORAGE_KEY_WIDTH, String(w));
  } catch {
    // 忽略
  }
}

function persistMiniMode(m: boolean): void {
  try {
    localStorage.setItem(STORAGE_KEY_MINI, String(m));
  } catch {
    // 忽略
  }
}

function persistTab(tab: AgentPanelTab): void {
  try {
    localStorage.setItem(STORAGE_KEY_TAB, tab);
  } catch {
    // 忽略
  }
}

interface AgentPanelState {
  /** 面板是否展开 */
  isOpen: boolean;

  /** 当前活跃标签页 */
  activeTab: AgentPanelTab;

  /** 面板宽度 (px)，范围 320-600 */
  panelWidth: number;

  /** 迷你模式开关 */
  isMiniMode: boolean;

  /** 是否正在拖拽调整宽度（拖拽时禁用 transition） */
  isDragging: boolean;

  /** Agent 页面上下文 */
  agentContext: AgentContext | null;

  // ── 方法 ──

  toggle(): void;
  open(): void;
  close(): void;
  setTab(tab: AgentPanelTab): void;
  setWidth(w: number): void;
  setDragging(dragging: boolean): void;
  toggleMiniMode(): void;
  setAgentContext(ctx: AgentContext): void;
  clearAgentContext(): void;
}

export const useAgentPanelStore = create<AgentPanelState>((set, get) => ({
  isOpen: false,
  activeTab: loadPersistedTab(),
  panelWidth: loadPersistedWidth(),
  isMiniMode: loadPersistedMiniMode(),
  isDragging: false,
  agentContext: null,

  toggle() {
    const { isOpen } = get();
    set({ isOpen: !isOpen });
  },

  open() {
    set({ isOpen: true });
  },

  close() {
    set({ isOpen: false });
  },

  setTab(tab) {
    set({ activeTab: tab });
    persistTab(tab);
    if (!get().isOpen) {
      set({ isOpen: true });
    }
  },

  setWidth(w) {
    const clamped = Math.min(PANEL_MAX_WIDTH, Math.max(PANEL_MIN_WIDTH, Math.round(w)));
    set({ panelWidth: clamped });
    persistWidth(clamped);
  },

  setDragging(dragging) {
    set({ isDragging: dragging });
  },

  toggleMiniMode() {
    const next = !get().isMiniMode;
    set({ isMiniMode: next });
    persistMiniMode(next);
  },

  setAgentContext(ctx) {
    set({ agentContext: ctx });
  },

  clearAgentContext() {
    set({ agentContext: null });
  },
}));

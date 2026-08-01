// SPDX-License-Identifier: AGPL-3.0-only

import { invoke } from "@/lib/invoke";
import { create } from "zustand";

/** Agent Panel 当前活跃标签页 */
export type AgentPanelTab = "chat" | "execution" | "skill" | "ui" | "nl-generation";

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

/** Agent 快捷操作 — 页面暴露给 Agent 的可执行操作 */
export interface AgentQuickAction {
  /** 操作唯一标识符 */
  id: string;
  /** 操作描述（给 Agent 看的） */
  description: string;
  /** 可选的参数定义 */
  params?: Record<string, unknown>;
  /** 是否需要用户确认（危险操作） */
  requireConfirmation?: boolean;
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
  /** 页面暴露给 Agent 的快捷操作列表 */
  quickActions?: AgentQuickAction[];
  /** 页面数据快照（供 Agent 参考的序列化数据） */
  data?: Record<string, unknown>;
  /** 上下文更新时间戳 */
  updatedAt?: number;
}

/** Agent 待确认的写操作（PermissionGate） */
export interface PendingConfirmation {
  id: string;
  /** 工具名称 */
  toolName: string;
  /** 操作描述 */
  description: string;
  /** 参数摘要 */
  paramsSummary?: string;
  /** 是否允许"本次不再询问" */
  allowBypass?: boolean;
  /** 超时时间戳（ms since epoch） */
  expiresAt?: number;
}

/** Agent 动态渲染的 UI Schema 条目 */
export interface AgentUISchemaEntry {
  /** Schema 唯一 ID */
  id: string;
  /** 完整的 UISchema JSON */
  schema: Record<string, unknown>;
  /** 渲染目标容器 ID */
  targetId?: string;
  /** 创建时间戳 */
  createdAt: number;
  /** 更新时间戳 */
  updatedAt: number;
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
    if (raw === "chat" || raw === "execution" || raw === "skill" || raw === "ui" || raw === "nl-generation") {
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

  /** 待确认的写操作队列 */
  pendingConfirmations: PendingConfirmation[];

  /** Agent 动态渲染的 UI Schema 列表 */
  agentUISchemas: AgentUISchemaEntry[];

  // ── 方法 ──

  toggle(): void;
  open(): void;
  close(): void;
  setTab(tab: AgentPanelTab): void;
  setWidth(w: number): void;
  setDragging(dragging: boolean): void;
  toggleMiniMode(): void;
  setAgentContext(ctx: AgentContext): void;
  /** 增量合并 Agent 上下文（保留未覆盖的字段） */
  mergeAgentContext(ctx: Partial<AgentContext>): void;
  clearAgentContext(): void;

  // ── PermissionGate ──

  /** 添加待确认的写操作 */
  addPendingConfirmation(confirmation: PendingConfirmation): void;
  /** 批准待确认操作 */
  resolveConfirmation(id: string, approved: boolean, bypass?: boolean): void;
  /** 清空所有待确认操作 */
  clearPendingConfirmations(): void;

  // ── Agent UI 渲染 ──

  /** 渲染新的 Agent UI Schema（或替换已存在的同名组件） */
  renderAgentUI(schema: Record<string, unknown>, targetId?: string, replace?: boolean): void;
  /** 更新已存在的 Agent UI Schema */
  updateAgentUI(
    schemaId: string,
    operation: "replace" | "append" | "remove",
    newSchema?: Record<string, unknown>,
    path?: string,
  ): void;
  /** 移除指定的 Agent UI Schema */
  removeAgentUI(schemaId: string): void;
  /** 清空所有 Agent UI Schema */
  clearAgentUI(): void;
}

export const useAgentPanelStore = create<AgentPanelState>((set, get) => ({
  isOpen: false,
  activeTab: loadPersistedTab(),
  panelWidth: loadPersistedWidth(),
  isMiniMode: loadPersistedMiniMode(),
  isDragging: false,
  agentContext: null,
  pendingConfirmations: [],
  agentUISchemas: [],

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
    set({ agentContext: { ...ctx, updatedAt: Date.now() } });
  },

  mergeAgentContext(ctx) {
    const current = get().agentContext;
    if (current) {
      set({ agentContext: { ...current, ...ctx, updatedAt: Date.now() } });
    } else {
      set({ agentContext: { ...ctx, page: "", url: "", updatedAt: Date.now() } });
    }
  },

  clearAgentContext() {
    set({ agentContext: null });
  },

  // ── PermissionGate ──

  addPendingConfirmation(confirmation) {
    const { pendingConfirmations } = get();
    set({
      pendingConfirmations: [...pendingConfirmations, confirmation],
    });
  },

  resolveConfirmation(id, approved, _bypass) {
    const { pendingConfirmations } = get();
    const target = pendingConfirmations.find((c) => c.id === id);
    set({
      pendingConfirmations: pendingConfirmations.filter((c) => c.id !== id),
    });

    // 通知后端权限确认结果
    if (target) {
      invoke<void>("agent_permission_response", {
        requestId: id,
        approved,
      }).catch((err) => {
        console.error("[agent_permission_response] IPC 调用失败:", err);
      });
    }
  },

  clearPendingConfirmations() {
    set({ pendingConfirmations: [] });
  },

  // ── Agent UI 渲染 ──

  renderAgentUI(schema, targetId, replace = true) {
    const { agentUISchemas } = get();
    const schemaId = String(schema.id ?? crypto.randomUUID());
    const now = Date.now();

    if (replace) {
      const existingIndex = agentUISchemas.findIndex((e) => e.id === schemaId);
      if (existingIndex >= 0) {
        const updated = [...agentUISchemas];
        updated[existingIndex] = {
          ...updated[existingIndex],
          schema,
          targetId,
          updatedAt: now,
        };
        set({ agentUISchemas: updated });
        return;
      }
    }

    set({
      agentUISchemas: [
        ...agentUISchemas,
        { id: schemaId, schema, targetId, createdAt: now, updatedAt: now },
      ],
    });
  },

  updateAgentUI(schemaId, operation, newSchema, _path) {
    const { agentUISchemas } = get();
    const now = Date.now();

    if (operation === "remove") {
      set({
        agentUISchemas: agentUISchemas.filter((e) => e.id !== schemaId),
      });
      return;
    }

    const existingIndex = agentUISchemas.findIndex((e) => e.id === schemaId);
    if (existingIndex < 0) { return; }

    if (operation === "replace" && newSchema) {
      const updated = [...agentUISchemas];
      updated[existingIndex] = {
        ...updated[existingIndex],
        schema: newSchema,
        updatedAt: now,
      };
      set({ agentUISchemas: updated });
    } else if (operation === "append" && newSchema) {
      const updated = [...agentUISchemas];
      const current = updated[existingIndex];
      const currentChildren = Array.isArray(current.schema.children)
        ? [...current.schema.children]
        : [];
      updated[existingIndex] = {
        ...current,
        schema: {
          ...current.schema,
          children: [...currentChildren, newSchema],
        },
        updatedAt: now,
      };
      set({ agentUISchemas: updated });
    }
  },

  removeAgentUI(schemaId) {
    const { agentUISchemas } = get();
    set({
      agentUISchemas: agentUISchemas.filter((e) => e.id !== schemaId),
    });
  },

  clearAgentUI() {
    set({ agentUISchemas: [] });
  },
}));

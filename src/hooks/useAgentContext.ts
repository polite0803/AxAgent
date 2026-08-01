// SPDX-License-Identifier: AGPL-3.0-only

import { useAgentPanelStore } from "@/stores/shared/agentPanelStore";
import type { AgentContext, AgentQuickAction, AgentSelection } from "@/stores/shared/agentPanelStore";
import { useCallback, useEffect } from "react";

/** useAgentContext Hook 的配置参数 */
export interface UseAgentContextOptions {
  /** 页面标识（如 "settings", "conversation", "workflow-editor"） */
  page: string;
  /** 页面 URL 或路由路径 */
  url?: string;
  /** 初始快捷操作列表 */
  quickActions?: AgentQuickAction[];
  /** 初始数据快照 */
  data?: Record<string, unknown>;
  /** 是否在组件卸载时清除上下文（默认 false，页面切换时由路由处理） */
  cleanupOnUnmount?: boolean;
}

/** useAgentContext 返回的 API */
export interface UseAgentContextReturn {
  /** 当前 Agent 上下文 */
  context: AgentContext | null;
  /** 设置页面数据快照 */
  setData: (data: Record<string, unknown>) => void;
  /** 设置选中内容 */
  setSelection: (selection: AgentSelection | null) => void;
  /** 添加或更新快捷操作 */
  upsertQuickAction: (action: AgentQuickAction) => void;
  /** 移除快捷操作 */
  removeQuickAction: (actionId: string) => void;
  /** 刷新上下文（更新时间戳） */
  refresh: () => void;
  /** 清除上下文 */
  clear: () => void;
}

/**
 * 页面级 Agent 上下文 Hook
 *
 * 使用示例：
 * ```tsx
 * const { setData, setSelection, upsertQuickAction } = useAgentContext({
 *   page: "settings",
 *   url: "/settings",
 *   quickActions: [
 *     { id: "save", description: "保存当前设置" },
 *     { id: "reset", description: "重置为默认设置", requireConfirmation: true },
 *   ],
 * });
 * ```
 *
 * 设计原则：
 * - 页面负责告知 Agent "我是谁" 和 "我能做什么"
 * - Agent 在侧边栏通过上下文理解当前环境
 * - 上下文通过 Zustand store 共享给 Agent 面板
 */
export function useAgentContext(options: UseAgentContextOptions): UseAgentContextReturn {
  const { page, url, quickActions: initialQuickActions, data: initialData, cleanupOnUnmount } = options;

  const agentContext = useAgentPanelStore((s) => s.agentContext);
  const setAgentContext = useAgentPanelStore((s) => s.setAgentContext);
  const mergeAgentContext = useAgentPanelStore((s) => s.mergeAgentContext);
  const clearAgentContext = useAgentPanelStore((s) => s.clearAgentContext);

  // 组件挂载时初始化上下文
  useEffect(() => {
    const initial: AgentContext = {
      page,
      url: url ?? "",
      quickActions: initialQuickActions ?? [],
      data: initialData ?? {},
      updatedAt: Date.now(),
    };
    setAgentContext(initial);

    return () => {
      if (cleanupOnUnmount) {
        clearAgentContext();
      }
    };
    // 仅在挂载时执行一次
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const setData = useCallback(
    (data: Record<string, unknown>) => {
      mergeAgentContext({ data });
    },
    [mergeAgentContext],
  );

  const setSelection = useCallback(
    (selection: AgentSelection | null) => {
      mergeAgentContext({ selection: selection ?? undefined });
    },
    [mergeAgentContext],
  );

  const upsertQuickAction = useCallback(
    (action: AgentQuickAction) => {
      const current = useAgentPanelStore.getState().agentContext;
      const existing = current?.quickActions ?? [];
      const idx = existing.findIndex((a) => a.id === action.id);
      const next = idx >= 0 ? existing.map((a) => (a.id === action.id ? action : a)) : [...existing, action];
      mergeAgentContext({ quickActions: next });
    },
    [mergeAgentContext],
  );

  const removeQuickAction = useCallback(
    (actionId: string) => {
      const current = useAgentPanelStore.getState().agentContext;
      const existing = current?.quickActions ?? [];
      const next = existing.filter((a) => a.id !== actionId);
      mergeAgentContext({ quickActions: next });
    },
    [mergeAgentContext],
  );

  const refresh = useCallback(() => {
    mergeAgentContext({});
  }, [mergeAgentContext]);

  const clear = useCallback(() => {
    clearAgentContext();
  }, [clearAgentContext]);

  return {
    context: agentContext,
    setData,
    setSelection,
    upsertQuickAction,
    removeQuickAction,
    refresh,
    clear,
  };
}

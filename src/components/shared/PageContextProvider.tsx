// SPDX-License-Identifier: AGPL-3.0-only
/* eslint-disable react-refresh/only-export-components */

import { useEvolutionStore } from "@/stores/feature/evolutionStore";
import { useWorkflowEditorStore } from "@/stores/feature/workflowEditorStore";
import { useWorkflowStore } from "@/stores/feature/workflowStore";
import { useAgentPanelStore } from "@/stores/shared/agentPanelStore";
import type { AgentSelection } from "@/stores/shared/agentPanelStore";
import React, { createContext, useContext, useEffect, useMemo } from "react";
import { useLocation } from "react-router-dom";

// ── Page Context (React Context) ──

export interface PageContextValue {
  /** 当前页面标识 */
  page: string;
  /** 当前 URL */
  url: string;
  /** 当前工作流 ID（仅 workflow 页） */
  currentWorkflowId: string | null;
  /** 活跃节点数（仅 workflow 页） */
  activeNodes: number;
  /** 进化状态摘要（workflow/evolution 页） */
  evolutionStatus: {
    runningEngines: string[];
    totalEvolutions: number;
  };
  /** 最近执行痕迹 */
  recentTraces: {
    count: number;
    latestName?: string;
    latestDuration?: number;
  };
  /** 当前选中内容（可选） */
  selection?: AgentSelection;
}

const defaultPageContext: PageContextValue = {
  page: "",
  url: "",
  currentWorkflowId: null,
  activeNodes: 0,
  evolutionStatus: { runningEngines: [], totalEvolutions: 0 },
  recentTraces: { count: 0 },
};

const PageContext = createContext<PageContextValue>(defaultPageContext);

/** 在子组件中获取当前页面上下文 */
export function usePageContext(): PageContextValue {
  return useContext(PageContext);
}

// ── Provider Props ──

export interface PageContextProviderProps {
  /** 页面标识（如 "knowledge"、"workflow"、"settings"） */
  page: string;
  /** 当前选中内容（可选） */
  selection?: AgentSelection;
  /** 子组件 */
  children: React.ReactNode;
}

/**
 * 页面上下文提供者
 *
 * 双重注入：
 * 1. 挂载时向 agentPanelStore 注入 Agent 上下文
 * 2. 创建 React Context 供子组件消费（usePageContext()）
 *
 * 使用方式：在每个页面的 Route 外层包裹此组件。
 * ```tsx
 * <PageContextProvider page="knowledge" selection={selectedDoc}>
 *   <KnowledgeHubPage />
 * </PageContextProvider>
 * ```
 */
export function PageContextProvider({
  page,
  selection,
  children,
}: PageContextProviderProps) {
  const location = useLocation();
  const setAgentContext = useAgentPanelStore((s) => s.setAgentContext);
  const clearAgentContext = useAgentPanelStore((s) => s.clearAgentContext);

  // ── 注入 Agent 面板上下文 ──

  useEffect(() => {
    setAgentContext({
      page,
      url: location.pathname + location.search,
      selection: selection,
    });

    return () => {
      clearAgentContext();
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [page]);

  // 监听 URL 变化，更新 context.url
  useEffect(() => {
    const currentCtx = useAgentPanelStore.getState().agentContext;
    if (currentCtx && currentCtx.page === page) {
      setAgentContext({
        ...currentCtx,
        url: location.pathname + location.search,
      });
    }
  }, [location.pathname, location.search, page, setAgentContext]);

  // ── React Context: 从各 store 聚合页面上下文 ──

  const pageContext = useMemo<PageContextValue>(() => {
    const base: PageContextValue = {
      ...defaultPageContext,
      page,
      url: location.pathname + location.search,
      selection,
    };

    // workflow 页：读取工作流编辑器 store
    if (page === "workflow") {
      try {
        const wfState = useWorkflowEditorStore.getState();
        base.activeNodes = wfState.nodes.length;
        base.currentWorkflowId = wfState.currentTemplate?.id ?? useWorkflowStore.getState().currentWorkflowId ?? null;
      } catch { /* store unavailable */ }
    }

    // evolution 上下文（workflow 页）
    if (page === "workflow") {
      try {
        const evoState = useEvolutionStore.getState();
        const runningEngines = Object.values(evoState.engines)
          .filter((e) => e.running)
          .map((e) => e.displayName);
        base.evolutionStatus = {
          runningEngines,
          totalEvolutions: evoState.evolutionHistory.length,
        };
      } catch { /* store unavailable */ }
    }

    // 最近执行痕迹
    try {
      const wfState = useWorkflowEditorStore.getState();
      base.recentTraces = {
        count: wfState.nodes.length > 0 ? 1 : 0,
      };
    } catch { /* store unavailable */ }

    return base;
  }, [page, location.pathname, location.search, selection]);

  return (
    <PageContext.Provider value={pageContext}>
      {children}
    </PageContext.Provider>
  );
}

// SPDX-License-Identifier: AGPL-3.0-only
// 集成测试辅助函数

import { PageContextProvider } from "@/components/shared/PageContextProvider";
import type { WorkflowEdge, WorkflowNode } from "@/components/workflow/types/workflow.types";
import { useWorkflowEditorStore } from "@/stores/feature/workflowEditorStore";
import type { AiChatMessage } from "@/stores/feature/workflowEditorStore";
import { render, type RenderOptions } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import React from "react";
import { MemoryRouter } from "react-router-dom";

// ── 类型 ──

export interface RenderWithProvidersOptions extends Omit<RenderOptions, "wrapper"> {
  /** 初始路由 */
  route?: string;
  /** 页面上下文 */
  pageContext?: string;
  /** 预注入 store 数据 */
  storeData?: Partial<ReturnType<typeof useWorkflowEditorStore.getState>>;
}

// ── Mock 数据工厂 ──

export function createMockNode(overrides: Partial<WorkflowNode> = {}): WorkflowNode {
  return {
    id: `node-${Math.random().toString(36).slice(2, 6)}`,
    type: "action",
    label: "Test Node",
    config: {},
    position: { x: 100, y: 100 },
    inputs: [],
    outputs: [],
    ...overrides,
  };
}

export function createMockEdge(overrides: Partial<WorkflowEdge> = {}): WorkflowEdge {
  return {
    id: `edge-${Math.random().toString(36).slice(2, 6)}`,
    source: "node-src",
    target: "node-tgt",
    ...overrides,
  };
}

export function createMockChatMessage(overrides: Partial<AiChatMessage> = {}): AiChatMessage {
  return {
    id: `msg-${Math.random().toString(36).slice(2, 6)}`,
    role: "user",
    content: "Hello",
    timestamp: Date.now(),
    ...overrides,
  };
}

// ── 渲染工具 ──

/**
 * 用所有必要的 Provider 渲染组件，并在组件外暴露 store 以便断言。
 * 返回 render 结果 + user + 便捷 store getter。
 */
export function renderWithProviders(
  ui: React.ReactElement,
  options: RenderWithProvidersOptions = {},
) {
  const { route = "/workflow", pageContext = "workflow", storeData, ...renderOptions } = options;

  // 注入 store 数据
  if (storeData) {
    useWorkflowEditorStore.setState({ ...useWorkflowEditorStore.getState(), ...storeData });
  }

  const Wrapper: React.FC<{ children: React.ReactNode }> = ({ children }) => (
    <MemoryRouter initialEntries={[route]}>
      <PageContextProvider page={pageContext}>
        {children}
      </PageContextProvider>
    </MemoryRouter>
  );

  const user = userEvent.setup();
  const result = render(ui, { wrapper: Wrapper, ...renderOptions });

  return {
    ...result,
    user,
    /** 直接获取当前 workflowEditorStore 状态 */
    getStore: () => useWorkflowEditorStore.getState(),
  };
}

/**
 * 从 useWorkflowEditorStore state 中读取 nodes/edges
 */
export function getStoreNodes(): WorkflowNode[] {
  return useWorkflowEditorStore.getState().nodes;
}

export function getStoreEdges(): WorkflowEdge[] {
  return useWorkflowEditorStore.getState().edges;
}

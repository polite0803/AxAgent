// SPDX-License-Identifier: AGPL-3.0-only
// 集成测试：工作流创建流程 —— 添加节点 → 连接 → 保存

import { useWorkflowEditorStore } from "@/stores/feature/workflowEditorStore";
import { act, renderHook } from "@testing-library/react";
import React from "react";
import { MemoryRouter } from "react-router-dom";
import { describe, expect, it } from "vitest";
import type { WorkflowNode, WorkflowEdge } from "@/components/workflow/types/workflow.types";

// 使用 react-hooks 的 renderHook 直接测试 store 逻辑
// （ReactFlow 画布渲染依赖 DOM 布局，集成测试专注于 store 状态正确性）

function createTestNode(id: string, overrides: Partial<WorkflowNode> = {}): WorkflowNode {
  return {
    id,
    type: "action",
    label: `Node ${id}`,
    config: {},
    position: { x: 100, y: 100 },
    inputs: [],
    outputs: [],
    ...overrides,
  };
}

function createTestEdge(id: string, source: string, target: string): WorkflowEdge {
  return { id, source, target };
}

describe("Workflow Creation Flow (Store)", () => {
  it("adds a node and verifies store state", () => {
    const { result } = renderHook(() => useWorkflowEditorStore(), {
      wrapper: ({ children }: { children: React.ReactNode }) => (
        <MemoryRouter>{children}</MemoryRouter>
      ),
    });

    const node = createTestNode("n1");

    act(() => {
      result.current.addNode(node);
    });

    expect(result.current.nodes).toHaveLength(1);
    expect(result.current.nodes[0].id).toBe("n1");
    expect(result.current.nodes[0].label).toBe("Node n1");
  });

  it("adds multiple nodes sequentially", () => {
    const { result } = renderHook(() => useWorkflowEditorStore(), {
      wrapper: ({ children }: { children: React.ReactNode }) => (
        <MemoryRouter>{children}</MemoryRouter>
      ),
    });

    act(() => {
      result.current.addNode(createTestNode("trigger-1", { type: "trigger" }));
      result.current.addNode(createTestNode("action-1", { type: "action" }));
      result.current.addNode(createTestNode("output-1", { type: "output" }));
    });

    expect(result.current.nodes).toHaveLength(3);
    const types = result.current.nodes.map((n) => n.type);
    expect(types).toContain("trigger");
    expect(types).toContain("action");
    expect(types).toContain("output");
  });

  it("connects nodes with edges", () => {
    const { result } = renderHook(() => useWorkflowEditorStore(), {
      wrapper: ({ children }: { children: React.ReactNode }) => (
        <MemoryRouter>{children}</MemoryRouter>
      ),
    });

    act(() => {
      result.current.addNode(createTestNode("n1"));
      result.current.addNode(createTestNode("n2"));
    });

    act(() => {
      result.current.addEdge(createTestEdge("e1", "n1", "n2"));
    });

    expect(result.current.edges).toHaveLength(1);
    expect(result.current.edges[0].source).toBe("n1");
    expect(result.current.edges[0].target).toBe("n2");
  });

  it("removes a node and its connected edges", () => {
    const { result } = renderHook(() => useWorkflowEditorStore(), {
      wrapper: ({ children }: { children: React.ReactNode }) => (
        <MemoryRouter>{children}</MemoryRouter>
      ),
    });

    act(() => {
      result.current.addNode(createTestNode("n1"));
      result.current.addNode(createTestNode("n2"));
      result.current.addEdge(createTestEdge("e1", "n1", "n2"));
    });

    expect(result.current.nodes).toHaveLength(2);
    expect(result.current.edges).toHaveLength(1);

    act(() => {
      result.current.deleteNode("n1");
    });

    expect(result.current.nodes).toHaveLength(1);
    // edges connected to deleted node should also be removed
    expect(result.current.edges.every((e) => e.source !== "n1")).toBe(true);
  });

  it("updates a node's config", () => {
    const { result } = renderHook(() => useWorkflowEditorStore(), {
      wrapper: ({ children }: { children: React.ReactNode }) => (
        <MemoryRouter>{children}</MemoryRouter>
      ),
    });

    act(() => {
      result.current.addNode(createTestNode("n1"));
    });

    act(() => {
      result.current.updateNode("n1", { label: "Updated Node", config: { url: "https://api.test" } });
    });

    const updated = result.current.nodes.find((n) => n.id === "n1");
    expect(updated?.label).toBe("Updated Node");
    expect(updated?.config).toEqual({ url: "https://api.test" });
  });

  it("clears all nodes and edges", () => {
    const { result } = renderHook(() => useWorkflowEditorStore(), {
      wrapper: ({ children }: { children: React.ReactNode }) => (
        <MemoryRouter>{children}</MemoryRouter>
      ),
    });

    act(() => {
      result.current.addNode(createTestNode("n1"));
      result.current.addNode(createTestNode("n2"));
      result.current.addEdge(createTestEdge("e1", "n1", "n2"));
    });

    expect(result.current.nodes).toHaveLength(2);

    act(() => {
      // reset to empty
      useWorkflowEditorStore.setState({ nodes: [], edges: [] });
    });

    expect(result.current.nodes).toHaveLength(0);
    expect(result.current.edges).toHaveLength(0);
  });
});

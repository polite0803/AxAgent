// SPDX-License-Identifier: AGPL-3.0-only

import type { WorkflowNode } from "@/components/workflow/types";
import { beforeEach, describe, expect, it, vi } from "vitest";

const invokeMock = vi.fn();
const listenMock = vi.fn();

vi.mock("@/lib/invoke", () => ({
  invoke: invokeMock,
  listen: listenMock,
  isTauri: () => false,
}));

vi.mock("zustand/middleware/immer", () => ({
  immer: (config: any) => (set: any, get: any, api: any) =>
    config(
      (partial: any) => {
        if (typeof partial === "function") {
          partial(get());
          set(get());
        } else {
          set(partial);
        }
      },
      get,
      api,
    ),
}));

function makeNode(id: string): WorkflowNode {
  return {
    id,
    type: "agent",
    title: id,
    description: "",
    position: { x: 0, y: 0 },
    config: {},
    retry: { enabled: false, max_retries: 0, backoff_type: "Fixed", base_delay_ms: 0, max_delay_ms: 0 },
    enabled: true,
  } as unknown as WorkflowNode;
}

describe("BatchEdit undo behavior - #6.8", () => {
  beforeEach(async () => {
    vi.clearAllMocks();
    invokeMock.mockReset();
    const { useWorkflowEditorStore } = await import("@/stores/feature/workflowEditorStore");
    (useWorkflowEditorStore as any).setState({
      nodes: [],
      edges: [],
      past: [],
      future: [],
      _lastUndoRecordTime: 0,
    });
  });

  it("a series of updateNode calls produces a single undo entry that reverts all changes", async () => {
    const { useWorkflowEditorStore } = await import("@/stores/feature/workflowEditorStore");
    const store = useWorkflowEditorStore.getState() as any;

    store.setNodes([makeNode("a"), makeNode("b"), makeNode("c")]);

    // 模拟 BatchEditPanel：先等待 1ms 越过 1 秒 coalesce 窗口，让首个 updateNode 能产生快照
    await new Promise((r) => setTimeout(r, 1100));

    const beforeBatch = useWorkflowEditorStore.getState() as any;
    expect(beforeBatch.nodes.every((n: any) => n.enabled === true)).toBe(true);
    const pastBefore = beforeBatch.past.length;

    for (const node of beforeBatch.nodes) {
      store.updateNode(node.id, { enabled: false });
    }

    const afterApply = useWorkflowEditorStore.getState() as any;
    expect(afterApply.past.length).toBe(pastBefore + 1);
    expect(afterApply.nodes.every((n: any) => n.enabled === false)).toBe(true);

    store.undo();

    const afterUndo = useWorkflowEditorStore.getState() as any;
    expect(afterUndo.nodes.every((n: any) => n.enabled === true)).toBe(true);
  });
});

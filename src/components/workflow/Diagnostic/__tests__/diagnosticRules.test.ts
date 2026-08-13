// SPDX-License-Identifier: AGPL-3.0-only

import { describe, expect, it } from "vitest";

import type { WorkflowEdge, WorkflowNode } from "@/components/workflow/types";
import { runDiagnosticRules } from "../diagnosticRules";

// ── 工厂函数 ──────────────────────────────────────────────────

function n(
  id: string,
  type: string,
  parentId?: string,
): WorkflowNode {
  return {
    id,
    type: type as WorkflowNode["type"],
    title: id,
    position: { x: 0, y: 0 },
    retry: { enabled: false, max_retries: 0, backoff_type: "Fixed", base_delay_ms: 0, max_delay_ms: 0 },
    enabled: true,
    parentId,
    config: {},
  } as WorkflowNode;
}

function e(id: string, source: string, target: string): WorkflowEdge {
  return { id, source, target, sourceHandle: "out", targetHandle: "in", edge_type: "direct" };
}

function orphanIds(report: ReturnType<typeof runDiagnosticRules>): string[] {
  return report.issues.filter((i) => i.id === "orphan_node").flatMap((i) => i.node_ids);
}

// ── 测试 ──────────────────────────────────────────────────────

describe("runDiagnosticRules — orphan_node", () => {
  it("正常链路无孤立节点", () => {
    const nodes = [n("a", "trigger"), n("b", "agent"), n("c", "end")];
    const edges = [e("e1", "a", "b"), e("e2", "b", "c")];
    expect(orphanIds(runDiagnosticRules(nodes, edges))).toHaveLength(0);
  });

  it("独立未连接节点（非 trigger）被检出", () => {
    const nodes = [n("trigger-1", "trigger"), n("orphan", "llm")];
    const edges: WorkflowEdge[] = [];
    expect(orphanIds(runDiagnosticRules(nodes, edges))).toContain("orphan");
  });

  it("装饰节点（_phaseSeparator / groupFrame）不误报为孤立", () => {
    const nodes = [
      n("trigger-1", "trigger"),
      n("sep", "_phaseSeparator"),
      n("frame", "groupFrame"),
    ];
    const edges: WorkflowEdge[] = [];
    expect(orphanIds(runDiagnosticRules(nodes, edges))).toHaveLength(0);
  });

  it("容器子节点（parentId 归属父容器、不经边）不误报为孤立", () => {
    const nodes = [
      n("trigger-1", "trigger"),
      n("parallel-1", "parallel"),
      n("child-1", "agent", "parallel-1"),
      n("child-2", "agent", "parallel-1"),
    ];
    const edges = [e("e1", "trigger-1", "parallel-1")];
    expect(orphanIds(runDiagnosticRules(nodes, edges))).toHaveLength(0);
  });
});

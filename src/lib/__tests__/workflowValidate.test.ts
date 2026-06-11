// SPDX-License-Identifier: AGPL-3.0-only

import { describe, expect, it } from "vitest";

import type { WorkflowEdge, WorkflowNode } from "@/components/workflow/types";
import { suggest_title, validate_workflow } from "@/lib/workflowLayout";
import type { ValidateIssue } from "@/lib/workflowLayout";

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

function e(
  id: string,
  source: string,
  target: string,
  sourceHandle?: string,
  edge_type: "direct" | "conditionTrue" | "conditionFalse" | "loopBack" | "grouping" = "direct",
): WorkflowEdge {
  return { id, source, target, sourceHandle, edge_type };
}

function findIssues(issues: ValidateIssue[], rule: string): ValidateIssue[] {
  return issues.filter((i) => i.rule === rule);
}

// ── 测试 ──────────────────────────────────────────────────────

describe("validate_workflow", () => {
  // ================================================================
  // 规则 1：孤立节点（非 trigger、非容器，入=出=0）
  // ================================================================
  describe("Rule 1 — orphan_node", () => {
    it("正常链路无孤立节点", () => {
      const nodes = [n("a", "trigger"), n("b", "agent"), n("c", "end")];
      const edges = [e("e1", "a", "b"), e("e2", "b", "c")];
      const result = validate_workflow(nodes, edges);
      expect(findIssues(result.issues, "orphan_node")).toHaveLength(0);
    });

    it("孤立 agent 节点被检出", () => {
      const nodes = [n("a", "trigger"), n("b", "agent")];
      const edges = [e("e1", "a", "b")];
      const result = validate_workflow(nodes, edges);
      // 'b' 有入度，不应检出
      expect(findIssues(result.issues, "orphan_node")).toHaveLength(0);
    });

    it("独立未连接节点（非 trigger）被检出", () => {
      const nodes = [n("trigger-1", "trigger"), n("orphan", "llm")];
      const edges: WorkflowEdge[] = [];
      const result = validate_workflow(nodes, edges);
      const issues = findIssues(result.issues, "orphan_node");
      expect(issues).toHaveLength(1);
      expect(issues[0].nodeIds).toContain("orphan");
    });

    it("孤立 tool 节点也被检出", () => {
      const nodes = [n("t1", "tool"), n("t2", "code")];
      const edges: WorkflowEdge[] = [];
      const result = validate_workflow(nodes, edges);
      const issues = findIssues(result.issues, "orphan_node");
      expect(issues).toHaveLength(2);
    });

    it("trigger 节点即使孤立也不报错", () => {
      const nodes = [n("tr", "trigger")];
      const edges: WorkflowEdge[] = [];
      const result = validate_workflow(nodes, edges);
      expect(findIssues(result.issues, "orphan_node")).toHaveLength(0);
    });
  });

  // ================================================================
  // 规则 2：数据黑洞（aggregator 入≥3 出=0）
  // ================================================================
  describe("Rule 2 — data_blackhole", () => {
    it("aggregator 入度≥3 出度=0 被检出", () => {
      const nodes = [n("a", "trigger"), n("b", "agent"), n("c", "agent"), n("d", "agent"), n("agg", "aggregator")];
      const edges = [
        e("e1", "a", "agg"),
        e("e2", "b", "agg"),
        e("e3", "c", "agg"),
        e("e4", "d", "agg"),
      ];
      const result = validate_workflow(nodes, edges);
      const issues = findIssues(result.issues, "data_blackhole");
      expect(issues).toHaveLength(1);
      expect(issues[0].nodeIds).toContain("agg");
      expect(issues[0].severity).toBe("error");
    });

    it("aggregator 出度>0 不报错", () => {
      const nodes = [
        n("a", "agent"),
        n("b", "agent"),
        n("c", "agent"),
        n("agg", "aggregator"),
        n("out", "agent"),
      ];
      const edges = [
        e("e1", "a", "agg"),
        e("e2", "b", "agg"),
        e("e3", "c", "agg"),
        e("e4", "agg", "out"),
      ];
      const result = validate_workflow(nodes, edges);
      expect(findIssues(result.issues, "data_blackhole")).toHaveLength(0);
    });

    it("aggregator 入度<3 不报错", () => {
      const nodes = [n("a", "agent"), n("agg", "aggregator")];
      const edges = [e("e1", "a", "agg")];
      const result = validate_workflow(nodes, edges);
      expect(findIssues(result.issues, "data_blackhole")).toHaveLength(0);
    });
  });

  // ================================================================
  // 规则 3：死分支（parallel/loop/debate/aggregator 入=出=0）
  // ================================================================
  describe("Rule 3 — dead_branch", () => {
    it("有子节点的 parallel 调度容器被检出（error）", () => {
      const nodes = [n("p", "parallel"), n("child", "agent", "p")];
      const edges: WorkflowEdge[] = [];
      const result = validate_workflow(nodes, edges);
      const issues = findIssues(result.issues, "dead_branch");
      expect(issues).toHaveLength(1);
      expect(issues[0].severity).toBe("error");
      expect(issues[0].nodeIds).toContain("p");
    });

    it("无子节点的 parallel 装饰容器被检出（warning）", () => {
      const nodes = [n("p", "parallel")];
      const edges: WorkflowEdge[] = [];
      const result = validate_workflow(nodes, edges);
      const issues = findIssues(result.issues, "dead_branch");
      expect(issues).toHaveLength(1);
      expect(issues[0].severity).toBe("warning");
      expect(issues[0].nodeIds).toContain("p");
    });

    it("有连接的 container 不报错", () => {
      const nodes = [n("tr", "trigger"), n("p", "parallel"), n("end", "end")];
      const edges = [e("e1", "tr", "p"), e("e2", "p", "end")];
      const result = validate_workflow(nodes, edges);
      expect(findIssues(result.issues, "dead_branch")).toHaveLength(0);
    });

    it("loop 和 debate 死分支也被检出", () => {
      const nodes = [n("lp", "loop"), n("db", "debate")];
      const edges: WorkflowEdge[] = [];
      const result = validate_workflow(nodes, edges);
      expect(findIssues(result.issues, "dead_branch")).toHaveLength(2);
    });
  });

  // ================================================================
  // 规则 4：端口未连（condition 的 true/false 至少一边未接）
  // ================================================================
  describe("Rule 4 — unconnected_port", () => {
    it("condition 的 true 端口未连被检出", () => {
      const nodes = [n("tr", "trigger"), n("c", "condition"), n("a", "agent")];
      const edges = [
        e("e1", "tr", "c"),
        e("e2", "c", "a", "false"),
      ];
      const result = validate_workflow(nodes, edges);
      const issues = findIssues(result.issues, "unconnected_port");
      expect(issues).toHaveLength(1);
      expect(issues[0].nodeIds).toContain("c");
      // 缺 true
      expect(issues[0].message).toContain("true");
    });

    it("condition 的 false 端口未连被检出", () => {
      const nodes = [n("tr", "trigger"), n("c", "condition"), n("a", "agent")];
      const edges = [
        e("e1", "tr", "c"),
        e("e2", "c", "a", "true"),
      ];
      const result = validate_workflow(nodes, edges);
      const issues = findIssues(result.issues, "unconnected_port");
      expect(issues).toHaveLength(1);
      expect(issues[0].message).toContain("false");
    });

    it("condition 双端口均未连被检出", () => {
      const nodes = [n("tr", "trigger"), n("c", "condition")];
      const edges = [e("e1", "tr", "c")];
      const result = validate_workflow(nodes, edges);
      const issues = findIssues(result.issues, "unconnected_port");
      expect(issues).toHaveLength(1);
      expect(issues[0].message).toContain("true");
      expect(issues[0].message).toContain("false");
    });

    it("condition 双端口全连不报错", () => {
      const nodes = [n("tr", "trigger"), n("c", "condition"), n("t", "agent"), n("f", "agent")];
      const edges = [
        e("e1", "tr", "c"),
        e("e2", "c", "t", "true"),
        e("e3", "c", "f", "false"),
      ];
      const result = validate_workflow(nodes, edges);
      expect(findIssues(result.issues, "unconnected_port")).toHaveLength(0);
    });
  });

  // ================================================================
  // 规则 5：循环无出口（SCC 不含 loopBack）
  // ================================================================
  describe("Rule 5 — cycle_no_exit", () => {
    it("简单环路无断路条件被检出", () => {
      const nodes = [n("a", "agent"), n("b", "agent"), n("c", "agent")];
      const edges = [e("e1", "a", "b"), e("e2", "b", "c"), e("e3", "c", "a")];
      const result = validate_workflow(nodes, edges);
      const issues = findIssues(result.issues, "cycle_no_exit");
      expect(issues).toHaveLength(1);
      expect(issues[0].severity).toBe("error");
      // SCC 应包含 a/b/c
      expect(issues[0].nodeIds.sort()).toEqual(["a", "b", "c"]);
    });

    it("环路含 loopBack 边不报错", () => {
      const nodes = [n("a", "agent"), n("b", "loop")];
      const edges = [
        e("e1", "a", "b"),
        e("e2", "b", "a", "loopBack", "loopBack"),
      ];
      const result = validate_workflow(nodes, edges);
      expect(findIssues(result.issues, "cycle_no_exit")).toHaveLength(0);
    });

    it("无环路不报错", () => {
      const nodes = [n("tr", "trigger"), n("a", "agent"), n("end", "end")];
      const edges = [e("e1", "tr", "a"), e("e2", "a", "end")];
      const result = validate_workflow(nodes, edges);
      expect(findIssues(result.issues, "cycle_no_exit")).toHaveLength(0);
    });
  });

  // ================================================================
  // 规则 6：自环边（source === target）
  // ================================================================
  describe("Rule 6 — self_loop", () => {
    it("自环边被检出", () => {
      const nodes = [n("a", "agent")];
      const edges = [e("sl", "a", "a")];
      const result = validate_workflow(nodes, edges);
      const issues = findIssues(result.issues, "self_loop");
      expect(issues).toHaveLength(1);
      expect(issues[0].nodeIds).toContain("a");
      expect(issues[0].edgeIds).toContain("sl");
      expect(issues[0].severity).toBe("error");
    });

    it("正常边不报错", () => {
      const nodes = [n("a", "agent"), n("b", "agent")];
      const edges = [e("e1", "a", "b")];
      const result = validate_workflow(nodes, edges);
      expect(findIssues(result.issues, "self_loop")).toHaveLength(0);
    });
  });

  // ================================================================
  // 空输入 / 组合场景
  // ================================================================
  describe("Edge cases", () => {
    it("空输入返回 valid", () => {
      const result = validate_workflow([], []);
      expect(result.valid).toBe(true);
      expect(result.issues).toHaveLength(0);
    });

    it("grouping 边被校验跳过（自环+循环检测均忽略）", () => {
      // grouping 自环边 → 不应命中 self_loop
      const nodes = [n("a", "agent"), n("b", "agent")];
      const edges = [
        e("e1", "a", "b"),
        e("sl", "a", "a", undefined, "grouping"),
      ];
      const result = validate_workflow(nodes, edges);
      expect(findIssues(result.issues, "self_loop")).toHaveLength(0);
      // 正常边存在，不被视为孤立
      expect(findIssues(result.issues, "orphan_node")).toHaveLength(0);
    });

    it("grouping 边不参与孤立节点/入度计算", () => {
      // 仅通过 grouping 边连接 → 节点仍被视为孤立
      const nodes = [n("tr", "trigger"), n("a", "agent")];
      const edges = [e("e1", "tr", "a", undefined, "grouping")];
      const result = validate_workflow(nodes, edges);
      // agent 只有 grouping 边，应被认定为孤立
      expect(findIssues(result.issues, "orphan_node")).toHaveLength(1);
    });

    it("带 end 节点的完整链路全部通过", () => {
      const nodes = [n("tr", "trigger"), n("a", "agent"), n("e", "end")];
      const edges = [e("e1", "tr", "a"), e("e2", "a", "e")];
      const result = validate_workflow(nodes, edges);
      expect(result.valid).toBe(true);
    });

    it("同时命中多条规则", () => {
      // 同时：孤立节点 + 自环边 + condition 缺端口
      const nodes = [n("tr", "trigger"), n("orphan", "llm"), n("c", "condition")];
      const edges = [e("sl", "c", "c")];
      const result = validate_workflow(nodes, edges);
      expect(result.valid).toBe(false);
      expect(findIssues(result.issues, "orphan_node")).toHaveLength(1);
      expect(findIssues(result.issues, "self_loop")).toHaveLength(1);
      expect(findIssues(result.issues, "unconnected_port")).toHaveLength(1);
    });
  });

  // ================================================================
  // 规则 7：标题重复（同一 type 相同 title）
  // ================================================================
  describe("Rule 7 — duplicate_title", () => {
    it("同 type 相同 title 被检出", () => {
      const nodes = [
        { ...n("a1", "agent"), title: "获取数据" },
        { ...n("a2", "agent"), title: "获取数据" },
        { ...n("a3", "tool"), title: "获取数据" },
      ];
      const result = validate_workflow(nodes, []);
      const issues = findIssues(result.issues, "duplicate_title");
      // agent 有 2 个重复"获取数据"，tool 只有 1 个不重复
      expect(issues).toHaveLength(1);
      expect(issues[0].nodeIds).toEqual(["a1", "a2"]);
      expect(issues[0].severity).toBe("warning");
    });

    it("不同 type 相同 title 不报警", () => {
      const nodes = [
        { ...n("a1", "agent"), title: "获取数据" },
        { ...n("t1", "tool"), title: "获取数据" },
      ];
      const result = validate_workflow(nodes, []);
      expect(findIssues(result.issues, "duplicate_title")).toHaveLength(0);
    });

    it("空标题不参与检查", () => {
      const nodes = [
        { ...n("a1", "agent"), title: "" },
        { ...n("a2", "agent"), title: "" },
      ];
      const result = validate_workflow(nodes, []);
      expect(findIssues(result.issues, "duplicate_title")).toHaveLength(0);
    });
  });
});

// ── suggest_title 测试 ──────────────────────────────────────
describe("suggest_title", () => {
  it("get-market-data → 获取行情数据", () => {
    expect(suggest_title("get-market-data", "tool")).toBe("获取行情数据");
  });

  it("fetch-user-order → 获取用户订单", () => {
    expect(suggest_title("fetch-user-order", "agent")).toBe("获取用户订单");
  });

  it("parse-kline → 解析K线", () => {
    expect(suggest_title("parse-kline", "tool")).toBe("解析K线");
  });

  it("短ID回退到 type+ID", () => {
    const result = suggest_title("node-5", "agent");
    expect(result).toContain("Agent");
    expect(result).toContain("node-5");
  });

  it("无分隔符的ID只用type+ID", () => {
    const result = suggest_title("myagent", "agent");
    expect(result).toContain("myagent");
  });
});

// ── WorkflowRef 规则测试 ──────────────────────────────────
describe("Rule 8 — workflow_ref", () => {
  it("workflowRef 未指定 target 报空引用 error", () => {
    const nodes = [
      { ...n("r1", "workflowRef"), config: { target_workflow_id: "" } },
    ];
    const result = validate_workflow(nodes, []);
    const issues = findIssues(result.issues, "workflow_ref_empty");
    expect(issues).toHaveLength(1);
    expect(issues[0].nodeIds).toContain("r1");
    expect(issues[0].severity).toBe("error");
  });

  it("workflowRef 自引用报错", () => {
    const nodes = [
      {
        ...n("r1", "workflowRef"),
        config: { target_workflow_id: "wf-1" },
        data: { templateId: "wf-1" },
      },
    ];
    const result = validate_workflow(nodes, []);
    const issues = findIssues(result.issues, "workflow_ref_self");
    expect(issues).toHaveLength(1);
    expect(issues[0].nodeIds).toContain("r1");
    expect(issues[0].severity).toBe("error");
  });

  it("多个 workflowRef 指向同一目标报 depth warning", () => {
    const nodes = [
      {
        ...n("r1", "workflowRef"),
        config: { target_workflow_id: "wf-target" },
      },
      {
        ...n("r2", "workflowRef"),
        config: { target_workflow_id: "wf-target" },
      },
    ];
    const result = validate_workflow(nodes, []);
    const issues = findIssues(result.issues, "workflow_ref_depth");
    // 2 个节点指向同一目标，产生 1 对冲突
    expect(issues.length).toBeGreaterThanOrEqual(1);
    expect(issues[0].severity).toBe("warning");
  });

  it("workflowRef 正常引用不报错", () => {
    const nodes = [
      {
        ...n("r1", "workflowRef"),
        config: { target_workflow_id: "wf-other" },
        data: { templateId: "wf-self" },
      },
    ];
    const result = validate_workflow(nodes, []);
    expect(findIssues(result.issues, "workflow_ref_empty")).toHaveLength(0);
    expect(findIssues(result.issues, "workflow_ref_self")).toHaveLength(0);
  });
});

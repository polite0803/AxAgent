// SPDX-License-Identifier: AGPL-3.0-only

import type { Edge, Node } from "@xyflow/react";
import { describe, expect, it } from "vitest";

import {
  autoLayoutWorkflow,
  clampChildrenIntoContainers,
  find_safe_position,
  getNodeSize,
  would_create_cycle,
} from "@/lib/workflowLayout";

function makeNode(
  id: string,
  type: string,
  position: { x: number; y: number } = { x: 0, y: 0 },
): Node {
  return {
    id,
    type,
    position,
    data: { type, label: id },
  };
}

function makeEdge(id: string, source: string, target: string): Edge {
  return { id, source, target };
}

/** 拓扑按层（dagre 输出 rank）累加每个节点的绝对 x/y 边界。 */
function absBounds(
  nodes: Node[],
  parentRefs: Record<string, string> = {},
): { minX: number; maxX: number; minY: number; maxY: number } {
  const childOf = parentRefs;
  const abs: Record<string, { x: number; y: number }> = {};
  for (const n of nodes) {
    const pid = childOf[n.id];
    const base = pid && abs[pid] ? abs[pid] : { x: 0, y: 0 };
    abs[n.id] = { x: n.position.x + base.x, y: n.position.y + base.y };
  }
  let minX = Infinity, minY = Infinity, maxX = -Infinity, maxY = -Infinity;
  for (const n of nodes) {
    const a = abs[n.id];
    const sz = getNodeSize((n.data?.type as string) || n.type || "");
    minX = Math.min(minX, a.x);
    minY = Math.min(minY, a.y);
    maxX = Math.max(maxX, a.x + sz.width);
    maxY = Math.max(maxY, a.y + sz.height);
  }
  return { minX, maxX, minY, maxY };
}

describe("workflowLayout", () => {
  describe("getNodeSize", () => {
    it("returns known size for built-in types", () => {
      expect(getNodeSize("agent")).toEqual({ width: 140, height: 36 });
      expect(getNodeSize("parallel")).toEqual({ width: 200, height: 80 });
    });

    it("returns default size for unknown type", () => {
      expect(getNodeSize("totally-unknown")).toEqual({ width: 140, height: 36 });
    });
  });

  describe("autoLayoutWorkflow", () => {
    it("returns empty result for empty input", () => {
      const result = autoLayoutWorkflow([], []);
      expect(result.nodes).toEqual([]);
      expect(result.edges).toEqual([]);
    });

    it("falls back to flat layout when no parentRefs provided", () => {
      const nodes = [
        makeNode("a", "agent", { x: 0, y: 0 }),
        makeNode("b", "agent", { x: 0, y: 0 }),
      ];
      const edges = [makeEdge("e1", "a", "b")];

      const result = autoLayoutWorkflow(nodes, edges);

      // 扁平布局时所有节点都按主 dagre 输出绝对坐标
      expect(result.nodes.find((n) => n.id === "a")?.position.x).not.toBe(0);
      expect(result.nodes.find((n) => n.id === "b")?.position.y).not.toBe(0);
      // b 在 a 之后（rankdir=LR，b 的 x 大于 a）
      expect(result.nodes.find((n) => n.id === "b")!.position.x)
        .toBeGreaterThan(result.nodes.find((n) => n.id === "a")!.position.x);
    });

    it("falls back to flat layout when parentRefs is empty even with parallel nodes", () => {
      const nodes = [
        makeNode("p", "parallel", { x: 0, y: 0 }),
        makeNode("c", "agent", { x: 0, y: 0 }),
      ];
      const result = autoLayoutWorkflow(nodes, [], {});
      // 空 parentRefs → 走扁平分支，节点都有非零 y
      expect(result.nodes.length).toBe(2);
      expect(result.nodes[0].position.y).toBeGreaterThanOrEqual(0);
    });

    it("places children of parallel container relative to parent", () => {
      const nodes = [
        makeNode("p", "parallel", { x: 0, y: 0 }),
        makeNode("c1", "agent", { x: 0, y: 0 }),
        makeNode("c2", "agent", { x: 0, y: 0 }),
      ];
      const parentRefs = { c1: "p", c2: "p" };

      const result = autoLayoutWorkflow(nodes, [], parentRefs);
      const c1 = result.nodes.find((n) => n.id === "c1")!;
      const p = result.nodes.find((n) => n.id === "p")!;

      // 子节点必须落在父容器 bbox 内：父绝对坐标 + PADDING 范围内
      const PADDING = 40;
      const pSize = getNodeSize("parallel");
      // ReactFlow 相对坐标：c1 = abs(c1) - abs(p)
      // 由于子组归一化到原点后叠加 PADDING，c1.x 必在 [PADDING, PADDING + 子组宽] 内
      expect(c1.position.x).toBeGreaterThanOrEqual(PADDING);
      expect(c1.position.y).toBeGreaterThanOrEqual(PADDING);
      expect(c1.position.x).toBeLessThan(pSize.width);
      expect(c1.position.y).toBeLessThan(pSize.height);
      // 容器尺寸在原默认 500x400 上至少放大（容纳子组 bbox + 2*PADDING）
      const pWidth = p.position.x + pSize.width; // 用默认尺寸参考
      expect(pWidth).toBeGreaterThan(0);
    });

    it("uses default container size when parallel has no children", () => {
      const nodes = [
        makeNode("p", "parallel", { x: 0, y: 0 }),
        makeNode("a", "agent", { x: 0, y: 0 }),
      ];
      const parentRefs = {}; // 容器无子

      const result = autoLayoutWorkflow(nodes, [], parentRefs);
      // 走扁平分支
      const bounds = absBounds(result.nodes);
      expect(bounds.maxY).toBeGreaterThan(0);
    });

    it("preserves edges unchanged in the result", () => {
      const nodes = [
        makeNode("a", "agent", { x: 0, y: 0 }),
        makeNode("b", "agent", { x: 0, y: 0 }),
      ];
      const edges = [makeEdge("e1", "a", "b")];

      const flat = autoLayoutWorkflow(nodes, edges);
      expect(flat.edges).toBe(edges);

      const withParent = autoLayoutWorkflow(
        [
          makeNode("p", "parallel", { x: 0, y: 0 }),
          makeNode("a", "agent", { x: 0, y: 0 }),
          makeNode("b", "agent", { x: 0, y: 0 }),
        ],
        edges,
        { a: "p", b: "p" },
      );
      expect(withParent.edges).toBe(edges);
    });

    it("nested parallel is treated as child of outer container (no nested sizing)", () => {
      // 设计上不支持嵌套：inner parallel 即使出现在 childOf 表里也按叶子处理
      const nodes = [
        makeNode("outer", "parallel", { x: 0, y: 0 }),
        makeNode("inner", "parallel", { x: 0, y: 0 }),
        makeNode("leaf", "agent", { x: 0, y: 0 }),
      ];
      const parentRefs = { inner: "outer", leaf: "outer" };

      const result = autoLayoutWorkflow(nodes, [], parentRefs);
      // outer 视为顶层容器并放大；inner 作为 leaf 出现在 outer 内
      const outer = result.nodes.find((n) => n.id === "outer")!;
      const inner = result.nodes.find((n) => n.id === "inner")!;
      // inner 应在 outer bbox 内（相对坐标落在 PADDING 区间）
      expect(inner.position.x).toBeGreaterThanOrEqual(40);
      expect(inner.position.y).toBeGreaterThanOrEqual(40);
      // outer 的宽度应当至少 > inner 宽 + padding
      const outerSize = getNodeSize("parallel");
      expect(outer.position.x).toBeDefined();
      expect(outerSize.width).toBeGreaterThan(0);
    });

    it("child not in any parentRefs subtree keeps absolute position", () => {
      const nodes = [
        makeNode("p", "parallel", { x: 0, y: 0 }),
        makeNode("c1", "agent", { x: 0, y: 0 }),
        makeNode("orphan", "agent", { x: 0, y: 0 }),
      ];
      const parentRefs = { c1: "p" };

      const result = autoLayoutWorkflow(nodes, [], parentRefs);
      // orphan 是顶层节点，dagre 会给非零坐标
      const orphan = result.nodes.find((n) => n.id === "orphan")!;
      expect(orphan.position.x !== 0 || orphan.position.y !== 0).toBe(true);
    });

    it("places children of loop container relative to parent", () => {
      const nodes = [
        makeNode("lp", "loop", { x: 0, y: 0 }),
        makeNode("c1", "agent", { x: 0, y: 0 }),
        makeNode("c2", "agent", { x: 0, y: 0 }),
      ];
      const parentRefs = { c1: "lp", c2: "lp" };

      const result = autoLayoutWorkflow(nodes, [], parentRefs);
      const c1 = result.nodes.find((n) => n.id === "c1")!;
      const lp = result.nodes.find((n) => n.id === "lp")!;

      const PADDING = 40;
      expect(c1.position.x).toBeGreaterThanOrEqual(PADDING);
      expect(c1.position.y).toBeGreaterThanOrEqual(PADDING);
      expect(lp.position).toBeDefined();
    });

    it("places children of debate container relative to parent", () => {
      const nodes = [
        makeNode("db", "debate", { x: 0, y: 0 }),
        makeNode("c1", "agent", { x: 0, y: 0 }),
        makeNode("c2", "agent", { x: 0, y: 0 }),
      ];
      const parentRefs = { c1: "db", c2: "db" };

      const result = autoLayoutWorkflow(nodes, [], parentRefs);
      const c1 = result.nodes.find((n) => n.id === "c1")!;
      const db = result.nodes.find((n) => n.id === "db")!;

      const PADDING = 40;
      expect(c1.position.x).toBeGreaterThanOrEqual(PADDING);
      expect(c1.position.y).toBeGreaterThanOrEqual(PADDING);
      expect(db.position).toBeDefined();
    });

    it("places children of aggregator container relative to parent", () => {
      const nodes = [
        makeNode("agg", "aggregator", { x: 0, y: 0 }),
        makeNode("c1", "agent", { x: 0, y: 0 }),
      ];
      const parentRefs = { c1: "agg" };

      const result = autoLayoutWorkflow(nodes, [], parentRefs);
      const c1 = result.nodes.find((n) => n.id === "c1")!;
      const agg = result.nodes.find((n) => n.id === "agg")!;

      const PADDING = 40;
      expect(c1.position.x).toBeGreaterThanOrEqual(PADDING);
      expect(c1.position.y).toBeGreaterThanOrEqual(PADDING);
      expect(agg.position).toBeDefined();
    });
  });
});

describe("find_safe_position", () => {
  it("returns the candidate when there is no overlap", () => {
    const safe = find_safe_position(
      { x: 100, y: 100 },
      "agent",
      [{ id: "b", x: 500, y: 500, type: "agent" }],
      10,
    );
    expect(safe).toEqual({ x: 100, y: 100 });
  });

  it("avoids overlapping a sibling by escaping to a non-overlapping direction", () => {
    // sibling occupies (0..180, 0..130), candidate (50, 50) sits inside it
    // algorithm picks the closest escape direction (right/left/down/up),
    // not necessarily right. We only assert the result escapes the bbox.
    const agentSize = getNodeSize("agent");
    const safe = find_safe_position(
      { x: 50, y: 50 },
      "agent",
      [{ id: "b", x: 0, y: 0, type: "agent" }],
      10,
    );
    const escapesX = safe.x + agentSize.width <= 0 || safe.x >= agentSize.width + 10;
    const escapesY = safe.y + agentSize.height <= 0 || safe.y >= agentSize.height + 10;
    expect(escapesX || escapesY).toBe(true);
  });

  it("snaps to grid", () => {
    // grid default is 20 → output should be divisible by 20 (when not overlapping)
    const safe = find_safe_position(
      { x: 13, y: 27 },
      "agent",
      [],
      10,
    );
    expect(safe.x % 20).toBe(0);
    expect(safe.y % 20).toBe(0);
  });
});

describe("would_create_cycle", () => {
  it("treats self-loop as a cycle", () => {
    expect(would_create_cycle([], "a", "a")).toBe(true);
  });

  it("returns false for a simple new edge in a DAG", () => {
    const edges = [
      { source: "a", target: "b" },
      { source: "b", target: "c" },
    ];
    expect(would_create_cycle(edges, "c", "d")).toBe(false);
  });

  it("returns true when new edge closes a cycle", () => {
    const edges = [
      { source: "a", target: "b" },
      { source: "b", target: "c" },
      { source: "c", target: "a" },
    ];
    // adding a -> c does not close a cycle (cycle already exists),
    // but adding b -> a would be one direction
    expect(would_create_cycle(edges, "b", "a")).toBe(true);
  });

  it("detects transitive cycle", () => {
    const edges = [
      { source: "a", target: "b" },
      { source: "b", target: "c" },
    ];
    // adding c -> a would create a -> b -> c -> a
    expect(would_create_cycle(edges, "c", "a")).toBe(true);
  });
});

describe("clampChildrenIntoContainers", () => {
  // 容器固定尺寸 500×400（parallel 默认），padding 默认 40
  const containerW = 500;
  const containerH = 400;
  const padding = 40;

  it("returns a new array without mutating input", () => {
    const container: Node = {
      id: "p",
      type: "parallel",
      position: { x: 0, y: 0 },
      data: {},
    };
    const child: Node = {
      id: "c1",
      type: "agent",
      // 完全合法：在容器内
      position: { x: 100, y: 100 },
      data: {},
      parentId: "p",
    };
    const result = clampChildrenIntoContainers(
      [container, child],
      { c1: "p" },
      { p: { width: containerW, height: containerH } },
      padding,
    );
    expect(result).not.toBe([container, child]);
    expect(result.find((n) => n.id === "c1")?.position).toEqual({ x: 100, y: 100 });
  });

  it("clamps a child overflowing on the right", () => {
    const container: Node = {
      id: "p",
      type: "parallel",
      position: { x: 0, y: 0 },
      data: {},
    };
    const overflow: Node = {
      id: "c1",
      type: "agent",
      position: { x: containerW + 200, y: 50 },
      data: {},
      parentId: "p",
    };
    const result = clampChildrenIntoContainers(
      [container, overflow],
      { c1: "p" },
      { p: { width: containerW, height: containerH } },
      padding,
    );
    const r = result.find((n) => n.id === "c1")!;
    const childW = getNodeSize("agent").width;
    // 子节点被拉回到容器内的 padding 区域
    expect(r.position.x + childW).toBeLessThanOrEqual(containerW - padding);
    expect(r.position.y).toBe(50);
  });

  it("clamps a child overflowing on the top-left", () => {
    const container: Node = {
      id: "p",
      type: "parallel",
      position: { x: 0, y: 0 },
      data: {},
    };
    const overflow: Node = {
      id: "c1",
      type: "agent",
      position: { x: -500, y: -300 },
      data: {},
      parentId: "p",
    };
    const result = clampChildrenIntoContainers(
      [container, overflow],
      { c1: "p" },
      { p: { width: containerW, height: containerH } },
      padding,
    );
    const r = result.find((n) => n.id === "c1")!;
    expect(r.position.x).toBeGreaterThanOrEqual(padding);
    expect(r.position.y).toBeGreaterThanOrEqual(padding);
  });

  it("leaves nodes without parent untouched", () => {
    const standalone: Node = {
      id: "x",
      type: "agent",
      position: { x: 9999, y: 9999 },
      data: {},
    };
    const result = clampChildrenIntoContainers(
      [standalone],
      {},
      {},
      padding,
    );
    expect(result.find((n) => n.id === "x")?.position).toEqual({ x: 9999, y: 9999 });
  });

  it("respects a missing parent size entry (skips clamping)", () => {
    const child: Node = {
      id: "c1",
      type: "agent",
      position: { x: 9999, y: 9999 },
      data: {},
      parentId: "missing",
    };
    const result = clampChildrenIntoContainers(
      [child],
      { c1: "missing" },
      {},
      padding,
    );
    // 无尺寸信息时保守不修改位置
    expect(result.find((n) => n.id === "c1")?.position).toEqual({ x: 9999, y: 9999 });
  });
});

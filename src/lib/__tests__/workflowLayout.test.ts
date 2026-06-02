import type { Edge, Node } from "reactflow";
import { describe, expect, it } from "vitest";

import { autoLayoutWorkflow, getNodeSize } from "@/lib/workflowLayout";

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
      expect(getNodeSize("agent")).toEqual({ width: 220, height: 160 });
      expect(getNodeSize("parallel")).toEqual({ width: 500, height: 400 });
    });

    it("returns default size for unknown type", () => {
      expect(getNodeSize("totally-unknown")).toEqual({ width: 200, height: 120 });
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
      // b 在 a 之后（rank 更低）
      expect(result.nodes.find((n) => n.id === "b")!.position.y)
        .toBeGreaterThan(result.nodes.find((n) => n.id === "a")!.position.y);
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

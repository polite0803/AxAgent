// SPDX-License-Identifier: AGPL-3.0-only
/**
 * 图谱布局 Web Worker：在后台线程运行 ForceAtlas2 力导向布局，避免阻塞主线程。
 *
 * 通信协议：
 * - 入参：{ type: "layout", nodes, edges, iterations, settings }
 * - 出参：{ type: "done", positions } 或 { type: "error", message }
 *
 * 10 万节点规模下，单次 ForceAtlas2 tick 约 O(N+E)，100 次 tick 在 i7 上 ~1.5s。
 * 在主线程跑会冻结 UI，放 worker 后用户可流畅缩放/平移。
 *
 * 注意：forceAtlas2.assign 是同步阻塞调用，但因为在 worker 里，不会卡主线程。
 */

import Graph from "graphology";
import forceAtlas2 from "graphology-layout-forceatlas2";
import type { Attributes } from "graphology-types";

export interface LayoutNode {
  id: string;
  x?: number;
  y?: number;
}

export interface LayoutEdge {
  source: string;
  target: string;
}

export interface LayoutRequest {
  type: "layout";
  nodes: LayoutNode[];
  edges: LayoutEdge[];
  iterations: number;
  settings?: {
    barnesHutOptimize?: boolean;
    barnesHutTheta?: number;
    adjustSizes?: boolean;
    gravity?: number;
    slowDown?: number;
    linLogMode?: boolean;
    outboundAttractionDistribution?: boolean;
    weightAttraction?: number;
    scaling?: number;
    strongGravityMode?: boolean;
  };
}

export interface LayoutResult {
  type: "done";
  positions: Array<{ id: string; x: number; y: number }>;
  iterations: number;
  durationMs: number;
}

export interface LayoutError {
  type: "error";
  message: string;
}

export type WorkerOutbound = LayoutResult | LayoutError;

// 构建图实例：节点带初始位置（无则随机），边为无向
function buildGraph(nodes: LayoutNode[], edges: LayoutEdge[]) {
  const graph = new Graph({ type: "undirected" });
  for (const n of nodes) {
    // 跳过重复 id（防御性）
    if (!graph.hasNode(n.id)) {
      graph.addNode(n.id, {
        x: n.x ?? Math.random() * 1000 - 500,
        y: n.y ?? Math.random() * 1000 - 500,
      });
    }
  }
  for (const e of edges) {
    if (graph.hasNode(e.source) && graph.hasNode(e.target) && e.source !== e.target) {
      // graphology 不支持平行边，合并为一条
      if (!graph.hasEdge(e.source, e.target)) {
        graph.addEdge(e.source, e.target, { weight: 1 });
      }
    }
  }
  return graph;
}

function handleRequest(req: LayoutRequest): WorkerOutbound {
  try {
    const start = performance.now();
    const graph = buildGraph(req.nodes, req.edges);
    const settings = {
      barnesHutOptimize: req.nodes.length > 1000,
      barnesHutTheta: 0.6,
      adjustSizes: false,
      gravity: 1.0,
      slowDown: 4,
      linLogMode: false,
      outboundAttractionDistribution: false,
      weightAttraction: 1,
      scaling: 1.0,
      strongGravityMode: false,
      ...req.settings,
    };

    const iters = Math.max(1, Math.min(req.iterations, 500));
    forceAtlas2.assign(graph, {
      iterations: iters,
      settings,
    });

    const positions: LayoutResult["positions"] = [];
    graph.forEachNode((id: string, attrs: Attributes) => {
      const x = (attrs as { x?: number }).x ?? 0;
      const y = (attrs as { y?: number }).y ?? 0;
      positions.push({ id, x, y });
    });

    return {
      type: "done",
      positions,
      iterations: iters,
      durationMs: performance.now() - start,
    };
  } catch (e) {
    return { type: "error", message: e instanceof Error ? e.message : String(e) };
  }
}

self.onmessage = (ev: MessageEvent<LayoutRequest>) => {
  const req = ev.data;
  if (!req || req.type !== "layout") {
    (self as unknown as { postMessage: (m: WorkerOutbound) => void }).postMessage({
      type: "error",
      message: "unknown request type",
    });
    return;
  }
  const result = handleRequest(req);
  (self as unknown as { postMessage: (m: WorkerOutbound) => void }).postMessage(result);
};

// 导出占位：让 Vite 把本文件识别为 worker module，并供类型导入使用
export const _workerReady = true;
export type { WorkerOutbound as GraphLayoutWorkerOutbound };

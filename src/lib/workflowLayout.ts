import dagre from "dagre";
import type { Edge, Node } from "reactflow";

/**
 * 节点尺寸估计（React Flow 画布坐标系，单位 px）。
 * 实际渲染尺寸可能不同，但 Dagre 只影响相对排列，偏差可接受。
 */
const NODE_SIZE: Record<string, { width: number; height: number }> = {
  trigger: { width: 200, height: 120 },
  agent: { width: 220, height: 160 },
  llm: { width: 220, height: 180 },
  condition: { width: 200, height: 140 },
  parallel: { width: 500, height: 400 },
  loop: { width: 480, height: 300 },
  debate: { width: 480, height: 260 },
  aggregator: { width: 320, height: 180 },
  merge: { width: 220, height: 120 },
  delay: { width: 180, height: 100 },
  tool: { width: 200, height: 140 },
  code: { width: 200, height: 140 },
  subWorkflow: { width: 220, height: 140 },
  documentParser: { width: 200, height: 120 },
  vectorRetrieve: { width: 200, height: 120 },
  validation: { width: 200, height: 120 },
  end: { width: 180, height: 80 },
};

const DEFAULT_SIZE = { width: 200, height: 120 };

/** 获取节点类型的尺寸估算（用于 hit-test / 布局） */
export function getNodeSize(type: string): { width: number; height: number } {
  return NODE_SIZE[type] || DEFAULT_SIZE;
}

/** 间距常量 */
const RANK_SEP = 80; // 层间垂直间距
const NODE_SEP = 50; // 同层节点水平间距
const MARGIN_X = 60; // 左边距
const MARGIN_Y = 60; // 上边距

/**
 * 使用 Dagre 对工作流节点进行自动布局。
 *
 * 策略：
 * - 拓扑排序后按层级自上而下排列
 * - 同一层节点水平均匀分布
 * - 尽量最小化边的交叉
 * - 特殊处理：condition 的 true/false 分支、loop 的回边
 *
 * @returns 更新了 position 的 nodes 和 edges（edges 不变）
 */
export function autoLayout(nodes: Node[], edges: Edge[]): { nodes: Node[]; edges: Edge[] } {
  if (nodes.length === 0) { return { nodes, edges }; }

  const g = new dagre.graphlib.Graph();
  g.setDefaultEdgeLabel(() => ({}));
  g.setGraph({
    rankdir: "TB",
    ranksep: RANK_SEP,
    nodesep: NODE_SEP,
    marginx: MARGIN_X,
    marginy: MARGIN_Y,
    edgesep: 20,
  });

  // 添加节点
  for (const node of nodes) {
    const nodeType = (node.data?.type || node.type || "") as string;
    const size = NODE_SIZE[nodeType] || DEFAULT_SIZE;
    g.setNode(node.id, { width: size.width, height: size.height });
  }

  // 添加边
  for (const edge of edges) {
    g.setEdge(edge.source, edge.target);
  }

  // 执行布局
  dagre.layout(g);

  // 应用位置
  const layoutedNodes = nodes.map((node) => {
    const dagreNode = g.node(node.id);
    if (!dagreNode) { return node; }

    const nodeType = (node.data?.type || node.type || "") as string;
    const size = NODE_SIZE[nodeType] || DEFAULT_SIZE;

    return {
      ...node,
      position: {
        x: dagreNode.x - size.width / 2,
        y: dagreNode.y - size.height / 2,
      },
    };
  });

  return { nodes: layoutedNodes, edges };
}

/**
 * 判断两个矩形是否重叠。
 */
function rectsOverlap(
  a: { x: number; y: number; w: number; h: number },
  b: { x: number; y: number; w: number; h: number },
): boolean {
  return !(a.x + a.w <= b.x || b.x + b.w <= a.x || a.y + a.h <= b.y || b.y + b.h <= a.y);
}

/**
 * 检测并修正节点重叠问题。
 * 对重叠的节点进行温和的位移，避免堆叠。
 */
export function resolveOverlaps(nodes: Node[]): Node[] {
  if (nodes.length < 2) { return nodes; }

  const result = [...nodes];
  const maxIterations = 100;
  let iteration = 0;

  while (iteration < maxIterations) {
    iteration++;
    let moved = false;

    for (let i = 0; i < result.length; i++) {
      for (let j = i + 1; j < result.length; j++) {
        const a = result[i];
        const b = result[j];
        const sizeA = NODE_SIZE[(a.data?.type as string) || ""] || DEFAULT_SIZE;
        const sizeB = NODE_SIZE[(b.data?.type as string) || ""] || DEFAULT_SIZE;

        if (
          rectsOverlap(
            { x: a.position.x, y: a.position.y, w: sizeA.width, h: sizeA.height },
            { x: b.position.x, y: b.position.y, w: sizeB.width, h: sizeB.height },
          )
        ) {
          // 计算重叠量并温和推开
          const overlapX = Math.min(a.position.x + sizeA.width, b.position.x + sizeB.width)
            - Math.max(a.position.x, b.position.x);
          const overlapY = Math.min(a.position.y + sizeA.height, b.position.y + sizeB.height)
            - Math.max(a.position.y, b.position.y);

          if (overlapX < overlapY) {
            // 水平推开
            const push = overlapX / 2 + 10;
            if (a.position.x <= b.position.x) {
              result[i] = { ...a, position: { ...a.position, x: a.position.x - push } };
              result[j] = { ...b, position: { ...b.position, x: b.position.x + push } };
            } else {
              result[i] = { ...a, position: { ...a.position, x: a.position.x + push } };
              result[j] = { ...b, position: { ...b.position, x: b.position.x - push } };
            }
          } else {
            // 垂直推开
            const push = overlapY / 2 + 10;
            if (a.position.y <= b.position.y) {
              result[i] = { ...a, position: { ...a.position, y: a.position.y - push } };
              result[j] = { ...b, position: { ...b.position, y: b.position.y + push } };
            } else {
              result[i] = { ...a, position: { ...a.position, y: a.position.y + push } };
              result[j] = { ...b, position: { ...b.position, y: b.position.y - push } };
            }
          }
          moved = true;
        }
      }
    }

    if (!moved) { break; }
  }

  return result;
}

/**
 * 完整的自动布局流程：Dagre 层级布局 + 重叠修正。
 *
 * @param parentRefs 容器子树映射（childId → parentId），用于让每个 parallel 节点
 *  内部的子节点先单独 dagre 排布，再随父容器整体定位。不传则退化为扁平布局。
 */
export function autoLayoutWorkflow(
  nodes: Node[],
  edges: Edge[],
  parentRefs: Record<string, string> = {},
): { nodes: Node[]; edges: Edge[] } {
  const childOf = parentRefs;
  const CONTAINER_TYPES = new Set(["parallel", "debate", "loop", "aggregator"]);
  const containers = nodes.filter((n) => CONTAINER_TYPES.has(n.type || "") && !childOf[n.id]);

  if (containers.length === 0 || Object.keys(childOf).length === 0) {
    const dagreResult = autoLayout(nodes, edges);
    const resolvedNodes = resolveOverlaps(dagreResult.nodes);
    return { nodes: resolvedNodes, edges: dagreResult.edges };
  }

  // 1. 反算每个节点的当前绝对坐标（input 是 ReactFlow 坐标系，子节点为相对父）
  const currentAbs: Record<string, { x: number; y: number }> = {};
  for (const n of nodes) {
    const pid = childOf[n.id];
    if (pid && currentAbs[pid]) {
      currentAbs[n.id] = { x: n.position.x + currentAbs[pid].x, y: n.position.y + currentAbs[pid].y };
    } else {
      currentAbs[n.id] = { x: n.position.x, y: n.position.y };
    }
  }

  // 2. 对每个 parallel 容器：单独 dagre 排子节点 + 量 bbox + 归一化到原点
  const PADDING = 40;
  const groupNorm: Record<string, { nodes: Node[]; bboxW: number; bboxH: number }> = {};
  const containerSizes: Record<string, { width: number; height: number }> = {};

  for (const c of containers) {
    const childIds = Object.keys(childOf).filter((cid) => childOf[cid] === c.id);
    const childNodesAbs = childIds
      .map((cid) => nodes.find((n) => n.id === cid))
      .filter((n): n is Node => !!n)
      .map((n) => ({ ...n, position: currentAbs[n.id] || n.position }));

    if (childNodesAbs.length === 0) {
      const size = getNodeSize(c.type || "");
      groupNorm[c.id] = { nodes: [], bboxW: 0, bboxH: 0 };
      containerSizes[c.id] = { width: size.width, height: size.height };
      continue;
    }

    const childEdges = edges.filter((e) => childIds.includes(e.source) && childIds.includes(e.target));
    const sub = autoLayout(childNodesAbs, childEdges);

    let minX = Infinity, minY = Infinity, maxX = -Infinity, maxY = -Infinity;
    for (const n of sub.nodes) {
      const sz = getNodeSize((n.data?.type as string) || n.type || "");
      minX = Math.min(minX, n.position.x);
      minY = Math.min(minY, n.position.y);
      maxX = Math.max(maxX, n.position.x + sz.width);
      maxY = Math.max(maxY, n.position.y + sz.height);
    }
    const bboxW = maxX - minX;
    const bboxH = maxY - minY;
    const normalized = sub.nodes.map((n) => ({
      ...n,
      position: { x: n.position.x - minX, y: n.position.y - minY },
    }));
    groupNorm[c.id] = { nodes: normalized, bboxW, bboxH };
    containerSizes[c.id] = { width: bboxW + PADDING * 2, height: bboxH + PADDING * 2 };
  }

  // 3. 主 dagre：只放顶层节点（容器节点 + 无父孤立节点）
  const topLevelIds = new Set<string>();
  for (const n of nodes) {
    if (CONTAINER_TYPES.has(n.type || "") || !childOf[n.id]) {
      topLevelIds.add(n.id);
    }
  }
  const topLevelNodes = nodes.filter((n) => topLevelIds.has(n.id));
  const topLevelEdges = edges.filter((e) => topLevelIds.has(e.source) && topLevelIds.has(e.target));

  const g = new dagre.graphlib.Graph();
  g.setDefaultEdgeLabel(() => ({}));
  g.setGraph({
    rankdir: "TB",
    ranksep: RANK_SEP,
    nodesep: NODE_SEP,
    marginx: MARGIN_X,
    marginy: MARGIN_Y,
    edgesep: 20,
  });
  for (const n of topLevelNodes) {
    const t = (n.data?.type as string) || n.type || "";
    const size = CONTAINER_TYPES.has(n.type || "")
      ? (containerSizes[n.id] ?? getNodeSize(t))
      : getNodeSize(t);
    g.setNode(n.id, { width: size.width, height: size.height });
  }
  for (const e of topLevelEdges) {
    g.setEdge(e.source, e.target);
  }
  dagre.layout(g);

  // 4. 写回绝对坐标
  const newAbs: Record<string, { x: number; y: number }> = {};
  for (const n of topLevelNodes) {
    const dagreNode = g.node(n.id);
    if (!dagreNode) { continue; }
    const t = (n.data?.type as string) || n.type || "";
    const size = CONTAINER_TYPES.has(n.type || "")
      ? (containerSizes[n.id] ?? getNodeSize(t))
      : getNodeSize(t);
    newAbs[n.id] = { x: dagreNode.x - size.width / 2, y: dagreNode.y - size.height / 2 };
  }
  for (const c of containers) {
    const cAbs = newAbs[c.id];
    if (!cAbs) { continue; }
    const group = groupNorm[c.id];
    for (const cn of group.nodes) {
      newAbs[cn.id] = {
        x: cAbs.x + PADDING + cn.position.x,
        y: cAbs.y + PADDING + cn.position.y,
      };
    }
  }

  // 5. 写回 ReactFlow 坐标系（子节点减回父节点位置，转为相对坐标）
  const result: Node[] = nodes.map((n) => {
    const abs = newAbs[n.id];
    if (!abs) { return n; }
    const pid = childOf[n.id];
    let final = abs;
    if (pid) {
      const parentAbs = newAbs[pid] || currentAbs[pid];
      if (parentAbs) {
        final = { x: abs.x - parentAbs.x, y: abs.y - parentAbs.y };
      }
    }
    return { ...n, position: final };
  });

  return { nodes: result, edges };
}

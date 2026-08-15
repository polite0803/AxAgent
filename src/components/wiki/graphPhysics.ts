// SPDX-License-Identifier: AGPL-3.0-only
/**
 * 图谱物理引擎：简洁可靠的力导向布局。
 *
 * 核心设计：
 * - 初始化时同步运行预热迭代（warmup），确保节点在首次渲染前已充分扩散
 * - 持续运行的低速模拟，节点永远在做微小的"呼吸"运动
 * - Barnes-Hut 四叉树加速多体排斥，O(n log n)
 * - 边用弹簧力（Hooke 定律），保持拓扑结构
 * - 稳定检测：全节点速度低于阈值时跳过物理
 */

export interface PhysicsNode {
  id: string;
  x: number;
  y: number;
  vx: number;
  vy: number;
  fx: number;
  fy: number;
  mass: number;
  fixed: boolean;
  kind: "note" | "concept" | "entity" | "source";
  idx: number;
}

export interface PhysicsEdge {
  source: string;
  target: string;
  restLength: number;
  stiffness: number;
  damping: number;
  sourceIdx: number;
  targetIdx: number;
}

interface QuadNode {
  x: number;
  y: number;
  mass: number;
  children: (QuadNode | null)[];
  nodeId: string | null;
  x0: number;
  y0: number;
  x1: number;
  y1: number;
}

const MAX_DEPTH = 50;

function createQuadNode(x0: number, y0: number, x1: number, y1: number): QuadNode {
  return {
    x: 0,
    y: 0,
    mass: 0,
    children: [null, null, null, null],
    nodeId: null,
    x0,
    y0,
    x1,
    y1,
  };
}

function getSubIndex(node: QuadNode, x: number, y: number): number {
  const mx = (node.x0 + node.x1) / 2;
  const my = (node.y0 + node.y1) / 2;
  const top = y < my ? 0 : 1;
  const left = x < mx ? 0 : 1;
  return top * 2 + left;
}

function insertQuad(root: QuadNode, id: string, x: number, y: number, mass: number, depth = 0): void {
  if (depth > MAX_DEPTH) { return; }

  if (root.nodeId === null && root.mass === 0 && root.children.every((c) => c === null)) {
    root.nodeId = id;
    root.x = x;
    root.y = y;
    root.mass = mass;
    return;
  }

  if (root.nodeId !== null) {
    const existingId = root.nodeId;
    const ex = root.x;
    const ey = root.y;
    const em = root.mass;
    root.nodeId = null;
    insertIntoChildren(root, existingId, ex, ey, em, depth);
    insertIntoChildren(root, id, x, y, mass, depth);
    root.mass += em;
    return;
  }

  insertIntoChildren(root, id, x, y, mass, depth);
  root.mass += mass;
}

function insertIntoChildren(root: QuadNode, id: string, x: number, y: number, mass: number, depth: number): void {
  const idx = getSubIndex(root, x, y);
  const child = getOrCreateChild(root, idx);
  insertQuad(child, id, x, y, mass, depth + 1);
}

function getOrCreateChild(root: QuadNode, idx: number): QuadNode {
  if (root.children[idx]) { return root.children[idx]!; }
  const mx = (root.x0 + root.x1) / 2;
  const my = (root.y0 + root.y1) / 2;
  let child: QuadNode;
  switch (idx) {
    case 0:
      child = createQuadNode(root.x0, root.y0, mx, my);
      break;
    case 1:
      child = createQuadNode(mx, root.y0, root.x1, my);
      break;
    case 2:
      child = createQuadNode(root.x0, my, mx, root.y1);
      break;
    default:
      child = createQuadNode(mx, my, root.x1, root.y1);
      break;
  }
  root.children[idx] = child;
  return child;
}

function computeCentroid(root: QuadNode): void {
  if (root.nodeId !== null) { return; }
  if (root.mass > 0) {
    let cx = 0;
    let cy = 0;
    for (const child of root.children) {
      if (child) {
        computeCentroid(child);
        cx += child.x * child.mass;
        cy += child.y * child.mass;
      }
    }
    root.x = cx / root.mass;
    root.y = cy / root.mass;
  }
}

function barnesHutForce(
  node: PhysicsNode,
  root: QuadNode,
  theta: number,
  repulsion: number,
): { fx: number; fy: number } {
  let fx = 0;
  let fy = 0;
  const stack: QuadNode[] = [root];

  while (stack.length > 0) {
    const current = stack.pop()!;
    if (current.mass === 0) { continue; }

    const dx = node.x - current.x;
    const dy = node.y - current.y;
    const distSq = dx * dx + dy * dy;

    if (distSq < 1) { continue; }

    const size = Math.max(current.x1 - current.x0, current.y1 - current.y0);
    const dist = Math.sqrt(distSq);

    if (size / dist < theta) {
      const force = (repulsion * current.mass * node.mass) / distSq;
      fx += (dx / dist) * force;
      fy += (dy / dist) * force;
    } else if (current.nodeId !== null && current.nodeId !== node.id) {
      const force = (repulsion * current.mass * node.mass) / distSq;
      fx += (dx / dist) * force;
      fy += (dy / dist) * force;
    } else {
      for (const child of current.children) {
        if (child) { stack.push(child); }
      }
    }
  }

  return { fx, fy };
}

function buildQuadTree(nodes: PhysicsNode[], startIdx: number, endIdx: number): QuadNode {
  let minX = Infinity, minY = Infinity, maxX = -Infinity, maxY = -Infinity;
  for (let i = startIdx; i < endIdx; i++) {
    const n = nodes[i];
    if (n.x < minX) { minX = n.x; }
    if (n.y < minY) { minY = n.y; }
    if (n.x > maxX) { maxX = n.x; }
    if (n.y > maxY) { maxY = n.y; }
  }
  const pad = 1;
  minX -= pad;
  minY -= pad;
  maxX += pad;
  maxY += pad;
  const root = createQuadNode(minX, minY, maxX, maxY);

  for (let i = startIdx; i < endIdx; i++) {
    const n = nodes[i];
    insertQuad(root, n.id, n.x, n.y, n.mass);
  }
  computeCentroid(root);
  return root;
}

export interface PhysicsConfig {
  theta: number;
  repulsion: number;
  gravity: number;
  damping: number;
  dt: number;
  springForce: number;
  springDamping: number;
  maxVelocity: number;
  clusterForce?: number;
}

export const DEFAULT_PHYSICS_CONFIG: PhysicsConfig = {
  theta: 0.6,
  repulsion: 30000,
  gravity: 0.002,
  damping: 0.85,
  dt: 0.4,
  springForce: 0.06,
  springDamping: 0.9,
  maxVelocity: 10,
};

export function computeCommunityCentroids(
  nodes: PhysicsNode[],
  communities: Map<string, number>,
): Map<number, { cx: number; cy: number; count: number }> {
  const buckets = new Map<number, { sx: number; sy: number; count: number }>();
  for (const node of nodes) {
    const cid = communities.get(node.id);
    if (cid === undefined) { continue; }
    const b = buckets.get(cid) ?? { sx: 0, sy: 0, count: 0 };
    b.sx += node.x;
    b.sy += node.y;
    b.count += 1;
    buckets.set(cid, b);
  }
  const result = new Map<number, { cx: number; cy: number; count: number }>();
  for (const [cid, b] of buckets) {
    result.set(cid, { cx: b.sx / b.count, cy: b.sy / b.count, count: b.count });
  }
  return result;
}

export type NeighborMap = Map<number, { targetIdx: number; rest: number; stiffness: number; damping: number }[]>;
export type NodeMap = Map<string, number>;

export function buildNodeMap(nodes: PhysicsNode[]): NodeMap {
  const map: NodeMap = new Map();
  for (let i = 0; i < nodes.length; i++) {
    map.set(nodes[i].id, i);
  }
  return map;
}

export function buildNeighborMap(edges: PhysicsEdge[]): NeighborMap {
  const map: NeighborMap = new Map();
  for (const edge of edges) {
    const sIdx = edge.sourceIdx;
    const tIdx = edge.targetIdx;
    if (!map.has(sIdx)) { map.set(sIdx, []); }
    if (!map.has(tIdx)) { map.set(tIdx, []); }
    map.get(sIdx)!.push({
      targetIdx: tIdx,
      rest: edge.restLength,
      stiffness: edge.stiffness,
      damping: edge.damping,
    });
    map.get(tIdx)!.push({
      targetIdx: sIdx,
      rest: edge.restLength,
      stiffness: edge.stiffness,
      damping: edge.damping,
    });
  }
  return map;
}

export function isSystemStable(nodes: PhysicsNode[], threshold = 0.3): boolean {
  for (let i = 0; i < nodes.length; i++) {
    const node = nodes[i];
    if (node.fixed) { continue; }
    const vx = node.vx;
    const vy = node.vy;
    if (vx * vx + vy * vy > threshold * threshold) { return false; }
  }
  return true;
}

/**
 * 执行一步物理模拟。
 * 使用索引化的节点数组，支持外部缓存的 neighborMap。
 */
export function stepPhysics(
  nodes: PhysicsNode[],
  edges: PhysicsEdge[],
  config: PhysicsConfig = DEFAULT_PHYSICS_CONFIG,
  bounds?: { x0: number; y0: number; x1: number; y1: number },
  communities?: Map<string, number>,
  communityCentroids?: Map<number, { cx: number; cy: number; count: number }>,
  cachedNeighborMap?: NeighborMap,
): void {
  if (nodes.length === 0) { return; }

  const n = nodes.length;
  const neighborMap = cachedNeighborMap ?? buildNeighborMap(edges);

  const fixedMask = new Uint8Array(n);
  const fxArr = new Float64Array(n);
  const fyArr = new Float64Array(n);

  let anyMoving = false;
  for (let i = 0; i < n; i++) {
    const node = nodes[i];
    if (node.fixed) {
      fixedMask[i] = 1;
      fxArr[i] = node.fx;
      fyArr[i] = node.fy;
    } else {
      const vx = node.vx;
      const vy = node.vy;
      if (!anyMoving && (vx * vx + vy * vy > 0.01)) { anyMoving = true; }
    }
  }

  if (!anyMoving && !communities) {
    for (let i = 0; i < n; i++) {
      const node = nodes[i];
      if (fixedMask[i]) { continue; }
      node.vx = 0;
      node.vy = 0;
    }
    return;
  }

  const quadRoot = buildQuadTree(nodes, 0, n);

  for (let i = 0; i < n; i++) {
    const node = nodes[i];
    if (fixedMask[i]) { continue; }

    let fx = 0;
    let fy = 0;

    const repForce = barnesHutForce(node, quadRoot, config.theta, config.repulsion);
    fx += repForce.fx;
    fy += repForce.fy;

    const distToCenter = Math.sqrt(node.x * node.x + node.y * node.y) || 1;
    fx += -config.gravity * node.x / distToCenter;
    fy += -config.gravity * node.y / distToCenter;

    const neighbors = neighborMap.get(i);
    if (neighbors) {
      for (let j = 0; j < neighbors.length; j++) {
        const nb = neighbors[j];
        const other = nodes[nb.targetIdx];
        const dx = other.x - node.x;
        const dy = other.y - node.y;
        const distSq = dx * dx + dy * dy;
        const dist = Math.sqrt(distSq) || 1;
        const displacement = dist - nb.rest;
        const springK = nb.stiffness * config.springForce;
        const dampK = nb.damping * config.springDamping;
        const f = springK * displacement;
        const dampF = dampK * ((other.vx - node.vx) * dx + (other.vy - node.vy) * dy) / dist;
        fx += (dx / dist) * (f + dampF);
        fy += (dy / dist) * (f + dampF);
      }
    }

    if (communities && communityCentroids && config.clusterForce) {
      const cid = communities.get(node.id);
      if (cid !== undefined) {
        const centroid = communityCentroids.get(cid);
        if (centroid) {
          const cdx = centroid.cx - node.x;
          const cdy = centroid.cy - node.y;
          const cdist = Math.sqrt(cdx * cdx + cdy * cdy) || 1;
          const clusterK = config.clusterForce;
          fx += (cdx / cdist) * clusterK;
          fy += (cdy / cdist) * clusterK;
        }
      }
    }

    fx += node.fx;
    fy += node.fy;
    fxArr[i] = fx;
    fyArr[i] = fy;
  }

  for (let i = 0; i < n; i++) {
    const node = nodes[i];
    if (fixedMask[i]) {
      node.vx = 0;
      node.vy = 0;
      continue;
    }

    const forceIdx = i;
    const ax = fxArr[forceIdx] / node.mass;
    const ay = fyArr[forceIdx] / node.mass;

    node.vx = (node.vx + ax * config.dt) * config.damping;
    node.vy = (node.vy + ay * config.dt) * config.damping;

    const speedSq = node.vx * node.vx + node.vy * node.vy;
    const maxV = config.maxVelocity;
    if (speedSq > maxV * maxV) {
      const speed = Math.sqrt(speedSq);
      const scale = maxV / speed;
      node.vx *= scale;
      node.vy *= scale;
    }

    node.x += node.vx * config.dt;
    node.y += node.vy * config.dt;

    if (bounds) {
      const margin = 50;
      if (node.x < bounds.x0 + margin) {
        node.x = bounds.x0 + margin;
        node.vx *= -0.5;
      }
      if (node.x > bounds.x1 - margin) {
        node.x = bounds.x1 - margin;
        node.vx *= -0.5;
      }
      if (node.y < bounds.y0 + margin) {
        node.y = bounds.y0 + margin;
        node.vy *= -0.5;
      }
      if (node.y > bounds.y1 - margin) {
        node.y = bounds.y1 - margin;
        node.vy *= -0.5;
      }
    }
  }
}

/**
 * 预热迭代：在渲染前运行多步物理，确保节点充分扩散。
 * 这是解决"节点堆成一团"的关键——在首次显示前就完成布局收敛。
 */
export function warmupPhysics(
  nodes: PhysicsNode[],
  edges: PhysicsEdge[],
  iterations: number = 80,
  config: PhysicsConfig = DEFAULT_PHYSICS_CONFIG,
  communities?: Map<string, number>,
): void {
  if (nodes.length === 0) { return; }

  const neighborMap = buildNeighborMap(edges);

  // 预热使用更大的斥力和更少的阻尼，帮助节点快速扩散
  const warmupConfig: PhysicsConfig = {
    ...config,
    repulsion: config.repulsion * 1.5,
    damping: 0.88, // 预热时较高阻尼，快速收敛
    maxVelocity: config.maxVelocity * 1.2,
  };

  const centroids = communities ? computeCommunityCentroids(nodes, communities) : undefined;

  for (let iter = 0; iter < iterations; iter++) {
    // 预热后期逐渐降低阻尼，让节点找到最终位置
    if (iter > iterations * 0.6) {
      // P10: 分母应为固定 iterations * 0.4，原式误用 iter 导致阻尼提前降到底
      const progress = (iter - iterations * 0.6) / (iterations * 0.4);
      warmupConfig.damping = 0.88 - progress * 0.03;
    }

    stepPhysics(
      nodes,
      edges,
      warmupConfig,
      undefined,
      communities,
      centroids,
      neighborMap,
    );

    // 每 10 步更新质心
    if (iter % 10 === 0 && communities) {
      const newCentroids = computeCommunityCentroids(nodes, communities);
      if (newCentroids) {
        for (const [k, v] of newCentroids) {
          centroids!.set(k, v);
        }
      }
    }
  }

  // 预热完成，清零速度，让节点稳定
  for (const node of nodes) {
    if (!node.fixed) {
      node.vx = 0;
      node.vy = 0;
    }
  }
}

/** 初始化节点位置（圆形均匀分布，足够大的初始范围） */
export function initializePositions(nodes: PhysicsNode[], width: number, height: number): void {
  const cx = 0;
  const cy = 0;
  const minDim = Math.min(width, height);
  const radius = Math.max(minDim * 0.6, nodes.length * 2);

  for (let i = 0; i < nodes.length; i++) {
    // 使用斐波那契螺旋分布，确保节点均匀填充圆盘
    const goldenAngle = Math.PI * (3 - Math.sqrt(5));
    const r = radius * Math.sqrt((i + 0.5) / nodes.length);
    const angle = i * goldenAngle + (Math.random() - 0.5) * 0.1;

    nodes[i].x = cx + r * Math.cos(angle);
    nodes[i].y = cy + r * Math.sin(angle);
    nodes[i].vx = 0;
    nodes[i].vy = 0;
    nodes[i].fx = 0;
    nodes[i].fy = 0;
    nodes[i].idx = i;
  }
}

/** 从邻接表构建物理边（带索引化） */
export function buildPhysicsEdges(
  adjacency: Map<string, Set<string>>,
  nodes: PhysicsNode[],
  avgDegree: number,
): PhysicsEdge[] {
  const idToIndex = new Map<string, number>();
  for (let i = 0; i < nodes.length; i++) {
    idToIndex.set(nodes[i].id, i);
  }

  const edges: PhysicsEdge[] = [];
  const seen = new Set<string>();

  for (const [source, targets] of adjacency) {
    const sourceIdx = idToIndex.get(source);
    if (sourceIdx === undefined) { continue; }

    for (const target of targets) {
      const targetIdx = idToIndex.get(target);
      if (targetIdx === undefined) { continue; }

      const key = sourceIdx < targetIdx ? `${sourceIdx}|${targetIdx}` : `${targetIdx}|${sourceIdx}`;
      if (seen.has(key)) { continue; }
      seen.add(key);

      const sourceDegree = adjacency.get(source)?.size ?? 1;
      const targetDegree = adjacency.get(target)?.size ?? 1;
      const degreeFactor = (sourceDegree + targetDegree) / (avgDegree * 2);
      const restLength = 80 + degreeFactor * 60;

      edges.push({
        source,
        target,
        restLength,
        stiffness: 0.7,
        damping: 0.5,
        sourceIdx,
        targetIdx,
      });
    }
  }

  return edges;
}

/** 根据节点 ID 快速设置力拖拽 */
export function setNodePositionById(nodes: PhysicsNode[], id: string, x: number, y: number, fixed: boolean): boolean {
  for (let i = 0; i < nodes.length; i++) {
    if (nodes[i].id === id) {
      nodes[i].x = x;
      nodes[i].y = y;
      nodes[i].vx = 0;
      nodes[i].vy = 0;
      nodes[i].fixed = fixed;
      return true;
    }
  }
  return false;
}

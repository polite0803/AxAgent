// SPDX-License-Identifier: AGPL-3.0-only
/**
 * 图谱物理引擎：Verlet 积分 + Barnes-Hut 近似 + 持续力模拟。
 *
 * 设计要点：
 * - 连续运行的低速模拟，节点永远在做微小的"呼吸"运动
 * - 拖拽节点时，物理引擎会实时响应，松手后弹性回弹
 * - Barnes-Hut 四叉树加速多体排斥，O(n log n)
 * - 边用弹簧力（Hooke 定律），保持拓扑结构
 *
 * 性能优化：
 * - 稳定检测 + 跳过物理：全节点速度低于阈值时跳过整个 step
 * - neighborMap / nodeMap 缓存复用：避免每帧重建
 * - 物理帧降频：稳定时每 6 帧才跑一次物理
 * - forces 用 Float64Array 存储：减少 Map 开销
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
  /** 在节点数组中的索引（快速定位，避免 Map 查找） */
  idx: number;
}

export interface PhysicsEdge {
  source: string;
  target: string;
  restLength: number;
  stiffness: number;
  damping: number;
  /** 在节点数组中的源/目标索引 */
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
  if (depth > MAX_DEPTH) {
    return;
  }

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
  if (root.children[idx]) {
    return root.children[idx]!;
  }
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
  if (root.nodeId !== null) {
    return;
  }
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
    if (current.mass === 0) {
      continue;
    }

    const dx = node.x - current.x;
    const dy = node.y - current.y;
    const distSq = dx * dx + dy * dy;

    if (distSq < 1) {
      continue;
    }

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
        if (child) {
          stack.push(child);
        }
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
  theta: 0.5,
  repulsion: 18000,
  gravity: 0.003,
  damping: 0.82,
  dt: 0.35,
  springForce: 0.08,
  springDamping: 0.85,
  maxVelocity: 8,
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

/** 邻居表类型：按节点索引分组，避免 Map 查找 */
export type NeighborMap = Map<number, { targetIdx: number; rest: number; stiffness: number; damping: number }[]>;

/** 节点索引映射（供 stepPhysics 按 ID 反查 idx） */
export type NodeMap = Map<string, number>;

/** 构建节点索引映射 */
export function buildNodeMap(nodes: PhysicsNode[]): NodeMap {
  const map: NodeMap = new Map();
  for (let i = 0; i < nodes.length; i++) {
    map.set(nodes[i].id, i);
  }
  return map;
}

/** 构建邻居表（按索引分组） */
export function buildNeighborMap(edges: PhysicsEdge[]): NeighborMap {
  const map: NeighborMap = new Map();
  for (const edge of edges) {
    const sIdx = edge.sourceIdx;
    const tIdx = edge.targetIdx;
    if (!map.has(sIdx)) {
      map.set(sIdx, []);
    }
    if (!map.has(tIdx)) {
      map.set(tIdx, []);
    }
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

/** 检查系统是否稳定（所有非固定节点速度低于阈值） */
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
 * - 使用索引化的节点数组，避免 Map 查找
 * - 稳定检测：如果全节点速度接近零，直接跳过
 * - 支持外部缓存的 neighborMap
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
  if (nodes.length === 0) {
    return;
  }

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
      if (!anyMoving && (vx * vx + vy * vy > 0.01)) {
        anyMoving = true;
      }
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

/** 初始化节点位置（圆形随机分布） */
export function initializePositions(nodes: PhysicsNode[], width: number, height: number): void {
  const cx = 0;
  const cy = 0;
  const radius = Math.min(width, height) * 0.5;
  const angleStep = (Math.PI * 2) / Math.max(nodes.length, 1);

  for (let i = 0; i < nodes.length; i++) {
    const r = radius * (0.15 + Math.random() * 0.85);
    const angle = i * angleStep + Math.random() * 1.0;
    nodes[i].x = cx + r * Math.cos(angle);
    nodes[i].y = cy + r * Math.sin(angle);
    nodes[i].vx = (Math.random() - 0.5) * 3;
    nodes[i].vy = (Math.random() - 0.5) * 3;
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
      const restLength = 60 + degreeFactor * 40;

      edges.push({
        source,
        target,
        restLength,
        stiffness: 0.8,
        damping: 0.6,
        sourceIdx,
        targetIdx,
      });
    }
  }

  return edges;
}

/** 根据节点 ID 快速设置力拖拽（避免每帧 find） */
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

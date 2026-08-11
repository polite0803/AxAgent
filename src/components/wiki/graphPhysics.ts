// SPDX-License-Identifier: AGPL-3.0-only
/**
 * 图谱物理引擎：Verlet 积分 + Barnes-Hut 近似 + 持续力模拟。
 *
 * 设计要点：
 * - 连续运行的低速模拟，节点永远在做微小的"呼吸"运动
 * - 拖拽节点时，物理引擎会实时响应，松手后弹性回弹
 * - Barnes-Hut 四叉树加速多体排斥，O(n log n)
 * - 边用弹簧力（Hooke 定律），保持拓扑结构
 */

export interface PhysicsNode {
  id: string;
  x: number;
  y: number;
  vx: number;
  vy: number;
  /** 外力（拖拽/固定节点使用） */
  fx: number;
  fy: number;
  mass: number;
  /** 是否固定位置（拖拽中或用户锁定） */
  fixed: boolean;
  /** 节点类型，用于差异化物理参数 */
  kind: "note" | "concept" | "entity" | "source";
}

export interface PhysicsEdge {
  source: string;
  target: string;
  /** 弹簧静止长度 */
  restLength: number;
  /** 弹簧刚度 */
  stiffness: number;
  /** 阻尼系数 */
  damping: number;
}

/** Barnes-Hut 四叉树节点 */
interface QuadNode {
  x: number;
  y: number;
  mass: number;
  children: (QuadNode | null)[];
  nodeId: string | null;
  /** 包围盒 */
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

  // 空叶子节点 → 直接存放
  if (root.nodeId === null && root.mass === 0 && root.children.every((c) => c === null)) {
    root.nodeId = id;
    root.x = x;
    root.y = y;
    root.mass = mass;
    return;
  }

  // 已被占用 → 分裂
  if (root.nodeId !== null) {
    const existingId = root.nodeId;
    const ex = root.x;
    const ey = root.y;
    const em = root.mass;
    root.nodeId = null;
    // 重新插入旧节点
    insertIntoChildren(root, existingId, ex, ey, em, depth);
    // 插入新节点
    insertIntoChildren(root, id, x, y, mass, depth);
    // 更新质心
    root.mass += em;
    return;
  }

  // 有子节点 → 直接插入
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

/** 计算四叉树的总质心（先有节点才能调用） */
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

/** Barnes-Hut 力计算 */
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

    // 远场近似
    if (size / dist < theta) {
      const force = (repulsion * current.mass * node.mass) / distSq;
      fx += (dx / dist) * force;
      fy += (dy / dist) * force;
    } else if (current.nodeId !== null && current.nodeId !== node.id) {
      // 叶子节点，直接计算
      const force = (repulsion * current.mass * node.mass) / distSq;
      fx += (dx / dist) * force;
      fy += (dy / dist) * force;
    } else {
      // 近场，递归
      for (const child of current.children) {
        if (child) {
          stack.push(child);
        }
      }
    }
  }

  return { fx, fy };
}

/** 构建 Barnes-Hut 四叉树 */
function buildQuadTree(nodes: PhysicsNode[]): QuadNode {
  let minX = Infinity, minY = Infinity, maxX = -Infinity, maxY = -Infinity;
  for (const n of nodes) {
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

  for (const n of nodes) {
    insertQuad(root, n.id, n.x, n.y, n.mass);
  }
  computeCentroid(root);
  return root;
}

export interface PhysicsConfig {
  /** Barnes-Hut 阈值，越小越精确但越慢 */
  theta: number;
  /** 全局斥力系数 */
  repulsion: number;
  /** 全局引力（向中心聚拢） */
  gravity: number;
  /** 阻尼系数（速度衰减） */
  damping: number;
  /** 积分步长 */
  dt: number;
  /** 弹簧力强度 */
  springForce: number;
  /** 弹簧阻尼 */
  springDamping: number;
  /** 速度裁剪上限 */
  maxVelocity: number;
  /** 社区聚类引力强度 */
  clusterForce?: number;
}

export const DEFAULT_PHYSICS_CONFIG: PhysicsConfig = {
  theta: 0.5,
  repulsion: 8000,
  gravity: 0.005,
  damping: 0.85,
  dt: 0.4,
  springForce: 0.05,
  springDamping: 0.9,
  maxVelocity: 8,
};

/** 社区聚类：计算每个社区的质心 */
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

/**
 * 执行一步物理模拟。
 * 使用 Verlet 积分 + Barnes-Hut + 弹簧力 + 社区聚类力。
 */
export function stepPhysics(
  nodes: PhysicsNode[],
  edges: PhysicsEdge[],
  config: PhysicsConfig = DEFAULT_PHYSICS_CONFIG,
  bounds?: { x0: number; y0: number; x1: number; y1: number },
  communities?: Map<string, number>,
  communityCentroids?: Map<number, { cx: number; cy: number; count: number }>,
): void {
  if (nodes.length === 0) {
    return;
  }

  // 1. 构建四叉树
  const quadRoot = buildQuadTree(nodes);

  // 2. 收集邻居表（弹簧力用）
  const neighborMap = new Map<string, { target: string; rest: number; stiffness: number; damping: number }[]>();
  for (const edge of edges) {
    if (!neighborMap.has(edge.source)) {
      neighborMap.set(edge.source, []);
    }
    if (!neighborMap.has(edge.target)) {
      neighborMap.set(edge.target, []);
    }
    neighborMap.get(edge.source)!.push({
      target: edge.target,
      rest: edge.restLength,
      stiffness: edge.stiffness,
      damping: edge.damping,
    });
    neighborMap.get(edge.target)!.push({
      target: edge.source,
      rest: edge.restLength,
      stiffness: edge.stiffness,
      damping: edge.damping,
    });
  }

  // 3. 计算所有力
  const forces = new Map<string, { fx: number; fy: number }>();
  const nodeMap = new Map<string, PhysicsNode>();
  for (const n of nodes) {
    nodeMap.set(n.id, n);
  }

  for (const node of nodes) {
    if (node.fixed) {
      forces.set(node.id, { fx: node.fx, fy: node.fy });
      continue;
    }

    let fx = 0;
    let fy = 0;

    // Barnes-Hut 排斥力
    const repForce = barnesHutForce(node, quadRoot, config.theta, config.repulsion);
    fx += repForce.fx;
    fy += repForce.fy;

    // 重力（向原点聚拢）
    const distToCenter = Math.sqrt(node.x * node.x + node.y * node.y) || 1;
    fx += -config.gravity * node.x / distToCenter;
    fy += -config.gravity * node.y / distToCenter;

    // 弹簧力
    const neighbors = neighborMap.get(node.id);
    if (neighbors) {
      for (const nb of neighbors) {
        const other = nodeMap.get(nb.target);
        if (!other) {
          continue;
        }
        const dx = other.x - node.x;
        const dy = other.y - node.y;
        const dist = Math.sqrt(dx * dx + dy * dy) || 1;
        // Hooke 定律 + 阻尼
        const displacement = dist - nb.rest;
        const springK = nb.stiffness * config.springForce;
        const dampK = nb.damping * config.springDamping;
        const f = springK * displacement;
        const dampF = dampK * ((other.vx - node.vx) * dx + (other.vy - node.vy) * dy) / dist;
        fx += (dx / dist) * (f + dampF);
        fy += (dy / dist) * (f + dampF);
      }
    }

    // 社区聚类引力（将节点拉向其社区质心）
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

    // 外力（拖拽）
    fx += node.fx;
    fy += node.fy;

    forces.set(node.id, { fx, fy });
  }

  // 4. Verlet 积分 + 边界约束
  for (const node of nodes) {
    if (node.fixed) {
      node.vx = 0;
      node.vy = 0;
      continue;
    }

    const force = forces.get(node.id);
    if (!force) {
      continue;
    }

    const ax = force.fx / node.mass;
    const ay = force.fy / node.mass;

    node.vx = (node.vx + ax * config.dt) * config.damping;
    node.vy = (node.vy + ay * config.dt) * config.damping;

    // 速度裁剪
    const speed = Math.sqrt(node.vx * node.vx + node.vy * node.vy);
    if (speed > config.maxVelocity) {
      const scale = config.maxVelocity / speed;
      node.vx *= scale;
      node.vy *= scale;
    }

    node.x += node.vx * config.dt;
    node.y += node.vy * config.dt;

    // 边界约束
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
  const radius = Math.min(width, height) * 0.3;
  const angleStep = (Math.PI * 2) / Math.max(nodes.length, 1);

  for (let i = 0; i < nodes.length; i++) {
    const r = radius * (0.3 + Math.random() * 0.7);
    const angle = i * angleStep + Math.random() * 0.5;
    nodes[i].x = cx + r * Math.cos(angle);
    nodes[i].y = cy + r * Math.sin(angle);
    nodes[i].vx = 0;
    nodes[i].vy = 0;
    nodes[i].fx = 0;
    nodes[i].fy = 0;
  }
}

/** 从邻接表构建物理边 */
export function buildPhysicsEdges(
  adjacency: Map<string, Set<string>>,
  avgDegree: number,
): PhysicsEdge[] {
  const edges: PhysicsEdge[] = [];
  const seen = new Set<string>();

  for (const [source, targets] of adjacency) {
    for (const target of targets) {
      const key = [source, target].sort().join("|");
      if (seen.has(key)) {
        continue;
      }
      seen.add(key);

      // 基于度的弹簧静止长度：度越高的节点连接越密，弹簧越长
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
      });
    }
  }

  return edges;
}

// SPDX-License-Identifier: AGPL-3.0-only
/**
 * 图谱物理引擎 Worker
 *
 * 将 Barnes-Hut 四叉树 + Verlet 积分物理模拟移至 Worker 线程，
 * 主线程仅需接收计算结果并渲染，彻底消除物理计算对 UI 的阻塞。
 *
 * 消息协议：
 * - INIT:    初始化节点/边数据
 * - STEP:    执行一步物理模拟（主线程按需请求）
 * - UPDATE:  拖拽/交互时的强制更新
 * - RESULT:  Worker 返回节点坐标
 */

export type WorkerMessage =
  | { type: "init"; payload: InitPayload }
  | { type: "step"; payload: StepPayload }
  | { type: "update"; payload: UpdatePayload }
  | { type: "reset"; payload: ResetPayload }
  | { type: "destroy" };

export type WorkerResponse =
  | { type: "ready" }
  | { type: "result"; payload: ResultPayload }
  | { type: "error"; message: string };

interface InitPayload {
  nodes: {
    id: string;
    x: number;
    y: number;
    vx: number;
    vy: number;
    fx: number;
    fy: number;
    mass: number;
    fixed: boolean;
    kind: string;
    idx: number;
  }[];
  edges: {
    source: string;
    target: string;
    restLength: number;
    stiffness: number;
    damping: number;
    sourceIdx: number;
    targetIdx: number;
  }[];
  config: PhysicsConfig;
  communities?: Record<string, number>;
}

interface StepPayload {
  config: PhysicsConfig;
  communities?: Record<string, number>;
  centroids?: Record<number, { cx: number; cy: number; count: number }>;
}

// 节点索引 → 社区 ID 映射（-1 表示无社区）
// 在 init 时根据 communities（键为 nodeId 字符串）+ nodeIds 构建一次，
// 避免 step 时每帧用 Number("note:xxx") → NaN 查询失败
let nodeIdxToCommunity: Int32Array | null = null;

interface UpdatePayload {
  nodeId: string;
  x: number;
  y: number;
  fixed: boolean;
  vx?: number;
  vy?: number;
}

// 重置布局：主线程重新随机分布后，把新坐标同步回 Worker
interface ResetPayload {
  positions: Float64Array; // x, y 交替，长度 = 节点数 * 2
}

interface ResultPayload {
  positions: Float64Array;
  velocities: Float64Array;
  stable: boolean;
  tick: number;
}

interface PhysicsConfig {
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

// ── 内部数据存储（Worker 内存中保持连续 TypedArray） ──
let nodePositions: Float64Array | null = null; // x, y 交替
let nodeVelocities: Float64Array | null = null; // vx, vy 交替
let nodeFx: Float64Array | null = null;
let nodeFy: Float64Array | null = null;
let nodeMass: Float64Array | null = null;
let nodeFixed: Uint8Array | null = null;
let nodeIds: string[] = [];
let nodeKinds: string[] = [];

// 邻居表（索引化）
type NeighborEntry = { targetIdx: number; rest: number; stiffness: number; damping: number };
let neighborMap: Map<number, NeighborEntry[]> = new Map();
let neighborEdgesCount = 0;

let tick = 0;
let initialized = false;

// ── 四叉树（与 graphPhysics.ts 保持一致的实现） ──

interface QuadNode {
  x: number;
  y: number;
  mass: number;
  children: (QuadNode | null)[];
  nodeId: number | null; // 存储索引而非字符串
  x0: number;
  y0: number;
  x1: number;
  y1: number;
}

const MAX_DEPTH = 50;

function createQuadNode(x0: number, y0: number, x1: number, y1: number): QuadNode {
  return { x: 0, y: 0, mass: 0, children: [null, null, null, null], nodeId: null, x0, y0, x1, y1 };
}

function insertQuad(root: QuadNode, idx: number, x: number, y: number, mass: number, depth = 0): void {
  if (depth > MAX_DEPTH) { return; }
  if (root.nodeId === null && root.mass === 0 && root.children.every((c) => c === null)) {
    root.nodeId = idx;
    root.x = x;
    root.y = y;
    root.mass = mass;
    return;
  }
  if (root.nodeId !== null) {
    const ex = root.x;
    const ey = root.y;
    const em = root.mass;
    const existingIdx = root.nodeId;
    root.nodeId = null;
    insertIntoChildren(root, existingIdx, ex, ey, em, depth);
    insertIntoChildren(root, idx, x, y, mass, depth);
    root.mass += em;
    return;
  }
  insertIntoChildren(root, idx, x, y, mass, depth);
  root.mass += mass;
}

function insertIntoChildren(root: QuadNode, idx: number, x: number, y: number, mass: number, depth: number): void {
  const mx = (root.x0 + root.x1) / 2;
  const my = (root.y0 + root.y1) / 2;
  const top = y < my ? 0 : 1;
  const left = x < mx ? 0 : 1;
  const cidx = top * 2 + left;
  if (!root.children[cidx]) {
    let child: QuadNode;
    switch (cidx) {
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
    root.children[cidx] = child;
  }
  insertQuad(root.children[cidx]!, idx, x, y, mass, depth + 1);
}

function computeCentroid(root: QuadNode): void {
  if (root.nodeId !== null) { return; }
  if (root.mass > 0) {
    let cx = 0, cy = 0;
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

function buildQuadTree(n: number): QuadNode {
  if (!nodePositions || n === 0) { return createQuadNode(0, 0, 1, 1); }
  let minX = Infinity, minY = Infinity, maxX = -Infinity, maxY = -Infinity;
  for (let i = 0; i < n; i++) {
    const x = nodePositions[i * 2];
    const y = nodePositions[i * 2 + 1];
    if (x < minX) { minX = x; }
    if (y < minY) { minY = y; }
    if (x > maxX) { maxX = x; }
    if (y > maxY) { maxY = y; }
  }
  minX -= 1;
  minY -= 1;
  maxX += 1;
  maxY += 1;
  const root = createQuadNode(minX, minY, maxX, maxY);
  for (let i = 0; i < n; i++) {
    insertQuad(root, i, nodePositions[i * 2], nodePositions[i * 2 + 1], nodeMass![i]);
  }
  computeCentroid(root);
  return root;
}

function barnesHutForce(nIdx: number, root: QuadNode, theta: number, repulsion: number): { fx: number; fy: number } {
  let fx = 0, fy = 0;
  const nx = nodePositions![nIdx * 2];
  const ny = nodePositions![nIdx * 2 + 1];
  const nm = nodeMass![nIdx];
  const stack: QuadNode[] = [root];

  while (stack.length > 0) {
    const current = stack.pop()!;
    if (current.mass === 0) { continue; }

    const dx = nx - current.x;
    const dy = ny - current.y;
    const distSq = dx * dx + dy * dy;

    if (distSq < 1) { continue; }

    const size = Math.max(current.x1 - current.x0, current.y1 - current.y0);
    const dist = Math.sqrt(distSq);

    if (size / dist < theta) {
      const force = (repulsion * current.mass * nm) / distSq;
      fx += (dx / dist) * force;
      fy += (dy / dist) * force;
    } else if (current.nodeId !== null && current.nodeId !== nIdx) {
      const force = (repulsion * current.mass * nm) / distSq;
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

// ── 物理步进（Worker 内部实现） ──

function stepPhysicsInternal(
  config: PhysicsConfig,
  centroids?: Map<number, { cx: number; cy: number; count: number }>,
): boolean {
  if (!nodePositions || !nodeVelocities || !nodeMass || !nodeFixed) { return false; }

  const n = nodePositions.length / 2;
  if (n === 0) { return false; }

  // 全量稳定检测：万级节点下一次循环开销可忽略，
  // 避免前 20 个节点恰好稳定但其余仍在剧烈运动时误判
  let stable = true;
  const stableThresholdSq = 0.04; // 0.2²
  for (let i = 0; i < n; i++) {
    if (nodeFixed[i]) { continue; }
    const vx = nodeVelocities[i * 2];
    const vy = nodeVelocities[i * 2 + 1];
    if (vx * vx + vy * vy > stableThresholdSq) {
      stable = false;
      break;
    }
  }
  if (stable) {
    // 低速持续运动：仍执行物理，但减小 dt
    config = { ...config, dt: config.dt * 0.3 };
  }

  // 构建四叉树
  const quadRoot = buildQuadTree(n);

  const fxArr = new Float64Array(n);
  const fyArr = new Float64Array(n);
  let anyMoving = false;

  for (let i = 0; i < n; i++) {
    if (nodeFixed[i]) {
      fxArr[i] = nodeFx![i];
      fyArr[i] = nodeFy![i];
      continue;
    }

    const vx = nodeVelocities[i * 2];
    const vy = nodeVelocities[i * 2 + 1];
    if (!anyMoving && (vx * vx + vy * vy > 0.01)) { anyMoving = true; }

    const { fx: repFx, fy: repFy } = barnesHutForce(i, quadRoot, config.theta, config.repulsion);
    let fx = repFx;
    let fy = repFy;

    const nx = nodePositions[i * 2];
    const ny = nodePositions[i * 2 + 1];
    const distToCenter = Math.sqrt(nx * nx + ny * ny) || 1;
    fx += -config.gravity * nx / distToCenter;
    fy += -config.gravity * ny / distToCenter;

    // 弹簧力（邻居）
    const neighbors = neighborMap.get(i);
    if (neighbors) {
      for (let j = 0; j < neighbors.length; j++) {
        const nb = neighbors[j];
        const oi = nb.targetIdx;
        const ox = nodePositions[oi * 2];
        const oy = nodePositions[oi * 2 + 1];
        const ovx = nodeVelocities[oi * 2];
        const ovy = nodeVelocities[oi * 2 + 1];

        const dx = ox - nx;
        const dy = oy - ny;
        const distSq = dx * dx + dy * dy;
        const dist = Math.sqrt(distSq) || 1;
        const displacement = dist - nb.rest;
        const springK = nb.stiffness * config.springForce;
        const dampK = nb.damping * config.springDamping;
        const f = springK * displacement;
        const dampF = dampK * ((ovx - vx) * dx + (ovy - vy) * dy) / dist;
        fx += (dx / dist) * (f + dampF);
        fy += (dy / dist) * (f + dampF);
      }
    }

    // 聚类力：使用 init 时构建的 nodeIdxToCommunity（Int32Array，O(1) 查询）
    if (nodeIdxToCommunity && centroids && config.clusterForce) {
      const cid = nodeIdxToCommunity[i];
      if (cid >= 0) {
        const centroid = centroids.get(cid);
        if (centroid) {
          const cdx = centroid.cx - nx;
          const cdy = centroid.cy - ny;
          const cdist = Math.sqrt(cdx * cdx + cdy * cdy) || 1;
          fx += (cdx / cdist) * config.clusterForce;
          fy += (cdy / cdist) * config.clusterForce;
        }
      }
    }

    fx += nodeFx![i];
    fy += nodeFy![i];
    fxArr[i] = fx;
    fyArr[i] = fy;
  }

  // Verlet 积分更新
  for (let i = 0; i < n; i++) {
    if (nodeFixed[i]) {
      nodeVelocities[i * 2] = 0;
      nodeVelocities[i * 2 + 1] = 0;
      continue;
    }

    const ax = fxArr[i] / nodeMass![i];
    const ay = fyArr[i] / nodeMass![i];

    nodeVelocities[i * 2] = (nodeVelocities[i * 2] + ax * config.dt) * config.damping;
    nodeVelocities[i * 2 + 1] = (nodeVelocities[i * 2 + 1] + ay * config.dt) * config.damping;

    // 速度截断
    const vx = nodeVelocities[i * 2];
    const vy = nodeVelocities[i * 2 + 1];
    const speedSq = vx * vx + vy * vy;
    if (speedSq > config.maxVelocity * config.maxVelocity) {
      const speed = Math.sqrt(speedSq);
      const scale = config.maxVelocity / speed;
      nodeVelocities[i * 2] *= scale;
      nodeVelocities[i * 2 + 1] *= scale;
    }

    nodePositions[i * 2] += nodeVelocities[i * 2] * config.dt;
    nodePositions[i * 2 + 1] += nodeVelocities[i * 2 + 1] * config.dt;
  }

  return stable;
}

// ── 主消息循环 ──

self.onmessage = (e: MessageEvent<WorkerMessage>) => {
  const msg = e.data;
  switch (msg.type) {
    case "init": {
      try {
        const n = msg.payload.nodes.length;
        nodePositions = new Float64Array(n * 2);
        nodeVelocities = new Float64Array(n * 2);
        nodeFx = new Float64Array(n);
        nodeFy = new Float64Array(n);
        nodeMass = new Float64Array(n);
        nodeFixed = new Uint8Array(n);
        nodeIdxToCommunity = new Int32Array(n).fill(-1);
        nodeIds = [];
        nodeKinds = [];

        for (let i = 0; i < n; i++) {
          const node = msg.payload.nodes[i];
          nodePositions[i * 2] = node.x;
          nodePositions[i * 2 + 1] = node.y;
          nodeVelocities[i * 2] = node.vx;
          nodeVelocities[i * 2 + 1] = node.vy;
          nodeFx[i] = node.fx;
          nodeFy[i] = node.fy;
          nodeMass[i] = node.mass;
          nodeFixed[i] = node.fixed ? 1 : 0;
          nodeIds.push(node.id);
          nodeKinds.push(node.kind);
        }

        // 构建邻居表
        neighborMap = new Map();
        neighborEdgesCount = 0;
        for (const edge of msg.payload.edges) {
          const sIdx = edge.sourceIdx;
          const tIdx = edge.targetIdx;
          if (!neighborMap.has(sIdx)) { neighborMap.set(sIdx, []); }
          if (!neighborMap.has(tIdx)) { neighborMap.set(tIdx, []); }
          neighborMap.get(sIdx)!.push({
            targetIdx: tIdx,
            rest: edge.restLength,
            stiffness: edge.stiffness,
            damping: edge.damping,
          });
          neighborMap.get(tIdx)!.push({
            targetIdx: sIdx,
            rest: edge.restLength,
            stiffness: edge.stiffness,
            damping: edge.damping,
          });
          neighborEdgesCount++;
        }

        // 构建节点索引 → 社区 ID 映射
        // init 时 communities 键是 nodeId 字符串，需通过 nodeIds 反查索引
        if (msg.payload.communities) {
          const idToIdx = new Map<string, number>();
          for (let i = 0; i < nodeIds.length; i++) {
            idToIdx.set(nodeIds[i], i);
          }
          for (const [nodeId, cid] of Object.entries(msg.payload.communities)) {
            const idx = idToIdx.get(nodeId);
            if (idx !== undefined) {
              nodeIdxToCommunity[idx] = cid;
            }
          }
        }

        tick = 0;
        initialized = true;

        (globalThis as unknown as Worker).postMessage({ type: "ready" } as WorkerResponse);
      } catch (err) {
        (globalThis as unknown as Worker).postMessage({ type: "error", message: String(err) } as WorkerResponse);
      }
      break;
    }

    case "step": {
      if (!initialized || !nodePositions) {
        (globalThis as unknown as Worker).postMessage(
          { type: "error", message: "Worker not initialized" } as WorkerResponse,
        );
        break;
      }
      try {
        const config = msg.payload.config;

        // communities 已在 init 时构建为 nodeIdxToCommunity（Int32Array），
        // step 时不再反序列化（避免 Number("note:xxx") → NaN 的键查询失败）
        // 仅当主线程在 step 时传入新的 communities 才热更新映射
        if (msg.payload.communities && nodeIdxToCommunity) {
          const idToIdx = new Map<string, number>();
          for (let i = 0; i < nodeIds.length; i++) {
            idToIdx.set(nodeIds[i], i);
          }
          nodeIdxToCommunity.fill(-1);
          for (const [nodeId, cid] of Object.entries(msg.payload.communities)) {
            const idx = idToIdx.get(nodeId);
            if (idx !== undefined) {
              nodeIdxToCommunity[idx] = cid;
            }
          }
        }

        // centroids 反序列化（键是数字字符串，转 number 安全）
        let centroids: Map<number, { cx: number; cy: number; count: number }> | undefined;
        if (msg.payload.centroids) {
          centroids = new Map();
          for (const [k, v] of Object.entries(msg.payload.centroids)) {
            centroids.set(Number(k), v);
          }
        }

        const stable = stepPhysicsInternal(config, centroids);
        tick++;

        const positionsCopy = new Float64Array(nodePositions!);
        const velocitiesCopy = new Float64Array(nodeVelocities!);

        const response: WorkerResponse = {
          type: "result",
          payload: { positions: positionsCopy, velocities: velocitiesCopy, stable, tick },
        };
        (globalThis as unknown as Worker).postMessage(response, [positionsCopy.buffer, velocitiesCopy.buffer]);
      } catch (err) {
        (globalThis as unknown as Worker).postMessage({ type: "error", message: String(err) } as WorkerResponse);
      }
      break;
    }

    case "update": {
      if (!initialized || !nodePositions) { break; }
      const ids = nodeIds;
      const i = ids.indexOf(msg.payload.nodeId);
      if (i >= 0) {
        nodePositions![i * 2] = msg.payload.x;
        nodePositions![i * 2 + 1] = msg.payload.y;
        nodeFixed![i] = msg.payload.fixed ? 1 : 0;
        if (msg.payload.vx !== undefined) { nodeVelocities![i * 2] = msg.payload.vx; }
        if (msg.payload.vy !== undefined) { nodeVelocities![i * 2 + 1] = msg.payload.vy; }
      }
      break;
    }

    case "reset": {
      if (!initialized || !nodePositions || !nodeVelocities || !nodeFixed) { break; }
      const positions = msg.payload.positions;
      const n = nodePositions.length / 2;
      const count = Math.min(n, positions.length / 2);
      for (let i = 0; i < count; i++) {
        nodePositions[i * 2] = positions[i * 2];
        nodePositions[i * 2 + 1] = positions[i * 2 + 1];
        nodeVelocities[i * 2] = 0;
        nodeVelocities[i * 2 + 1] = 0;
        nodeFixed[i] = 0;
      }
      tick = 0;
      // 同步主线程：重置后返回当前坐标，并让渲染循环立刻衔接新布局
      const positionsCopy = new Float64Array(nodePositions);
      const velocitiesCopy = new Float64Array(nodeVelocities!);
      (globalThis as unknown as Worker).postMessage(
        {
          type: "result",
          payload: { positions: positionsCopy, velocities: velocitiesCopy, stable: false, tick },
        } as WorkerResponse,
        [positionsCopy.buffer, velocitiesCopy.buffer],
      );
      break;
    }

    case "destroy": {
      nodePositions = null;
      nodeVelocities = null;
      nodeFx = null;
      nodeFy = null;
      nodeMass = null;
      nodeFixed = null;
      nodeIdxToCommunity = null;
      nodeIds = [];
      nodeKinds = [];
      neighborMap = new Map();
      initialized = false;
      break;
    }
  }
};

// SPDX-License-Identifier: AGPL-3.0-only
/**
 * Wiki 图谱视图 V2：Canvas 2D 自研渲染器 + 自研物理引擎。
 *
 * 向 Obsidian 图谱看齐的设计目标：
 * - 持续的力导向物理模拟，节点永远在做微小的"呼吸"运动
 * - 节点：径向渐变 glow + 脉动光晕 + 社区染色
 * - 边：基础线 + 粒子流动动画（沿边移动的光点）
 * - 交互：拖拽回弹、hover 邻居高亮、滚轮缩放、平移
 * - 性能：Canvas 2D 轻松处理万级节点
 */

import { Tooltip } from "@/components/layout/Tooltip";
import { Button, Card, Empty, Popover, theme, Typography } from "antd";
import { Download, Eye, Fullscreen, Maximize2, RefreshCw, SlidersHorizontal, ZoomIn, ZoomOut } from "lucide-react";
import {
  type CSSProperties,
  forwardRef,
  memo,
  type MouseEvent as ReactMouseEvent,
  useCallback,
  useEffect,
  useImperativeHandle,
  useRef,
  useState,
} from "react";
import { useTranslation } from "react-i18next";
import {
  buildNeighborMap,
  buildNodeMap,
  buildPhysicsEdges,
  computeCommunityCentroids,
  initializePositions,
  isSystemStable,
  type NeighborMap,
  type NodeMap,
  type PhysicsConfig,
  type PhysicsEdge,
  type PhysicsNode,
  stepPhysics,
} from "./graphPhysics";

// ─────────────────────────────────────────────────────────────────────────────
// 公共类型（保持向后兼容）
// ─────────────────────────────────────────────────────────────────────────────

export type GraphNodeType = "note" | "concept" | "entity" | "source";
export type GraphEdgeType = "link" | "backlink" | "reference" | "derived_from" | "contradicts" | "mapping";

export interface GraphNode {
  id: string;
  title: string;
  type: GraphNodeType;
  tags: string[];
  linkCount: number;
  backlinkCount: number;
  path: string;
  x?: number;
  y?: number;
}

export interface GraphEdge {
  source: string;
  target: string;
  type: GraphEdgeType;
}

export interface GraphData {
  nodes: GraphNode[];
  edges: GraphEdge[];
}

export type LayoutMode = "force" | "radial" | "hierarchy";

export interface GraphViewProps {
  data: GraphData;
  wikiId?: string;
  onNodeClick?: (nodeId: string) => void;
  onNodeDoubleClick?: (nodeId: string) => void;
  onNodeHover?: (nodeId: string | null) => void;
  onContextMenu?: (nodeId: string, position: { x: number; y: number }) => void;
  onDeleteNode?: (nodeId: string) => void;
  onDeselect?: () => void;
  highlightedNodeIds?: Set<string>;
  selectedNodeId?: string | null;
  communities?: Map<string, number>;
  showMinimap?: boolean;
}

export interface GraphViewHandle {
  focusOnNode: (nodeId: string) => void;
}

// ─────────────────────────────────────────────────────────────────────────────
// 常量与配色
// ─────────────────────────────────────────────────────────────────────────────

type TokenType = ReturnType<typeof theme.useToken>["token"];

const EMPTY_SET: ReadonlySet<string> = new Set<string>();

const communityPalette = [
  "#5B8FF9",
  "#61DDAA",
  "#65789B",
  "#F6BD16",
  "#7262FD",
  "#78D3F8",
  "#9661BC",
  "#F6903D",
  "#008685",
  "#F08BB4",
  "#1E90FF",
  "#32CD32",
];

const getNodeColorMap = (token: TokenType): Record<GraphNodeType, string> => ({
  note: token.colorPrimary,
  concept: token.colorSuccess,
  entity: "#FA8C16",
  source: "#EB2F96",
});

// ─────────────────────────────────────────────────────────────────────────────
// 布局持久化：localStorage 存储节点坐标
// ─────────────────────────────────────────────────────────────────────────────

const LAYOUT_STORAGE_PREFIX = "wiki_graph_layout_";

interface SavedLayout {
  positions: Record<string, { x: number; y: number }>;
  savedAt: number;
}

function saveLayout(wikiId: string, nodes: PhysicsNode[]): void {
  try {
    const positions: Record<string, { x: number; y: number }> = {};
    for (const node of nodes) {
      positions[node.id] = { x: node.x, y: node.y };
    }
    const layout: SavedLayout = {
      positions,
      savedAt: Date.now(),
    };
    localStorage.setItem(LAYOUT_STORAGE_PREFIX + wikiId, JSON.stringify(layout));
  } catch {
    // localStorage 可能已满，静默忽略
  }
}

function loadLayout(wikiId: string): SavedLayout | null {
  try {
    const raw = localStorage.getItem(LAYOUT_STORAGE_PREFIX + wikiId);
    if (!raw) { return null; }
    return JSON.parse(raw) as SavedLayout;
  } catch {
    return null;
  }
}

function applySavedLayout(nodes: PhysicsNode[], saved: SavedLayout): void {
  let matched = 0;
  for (const node of nodes) {
    const savedPos = saved.positions[node.id];
    if (savedPos) {
      node.x = savedPos.x;
      node.y = savedPos.y;
      matched++;
    }
  }
  // 匹配率低于 30% 时放弃：清空位置，交由 initializePositions 重新随机。
  // 注意：恢复位置不设 fixed —— 保存的布局只是"初始形态"，
  // 节点仍参与低速力导向模拟（保持呼吸动画），拖拽/重新布局可随时覆盖。
  if (matched < nodes.length * 0.3) {
    for (const node of nodes) {
      node.x = 0;
      node.y = 0;
    }
  }
}

const getEdgeTypeStylesMap = (
  token: TokenType,
): Record<GraphEdgeType, { color: string; width: number; animated: boolean }> => ({
  link: { color: token.colorBorderSecondary, width: 0.6, animated: true },
  backlink: { color: token.colorPrimary, width: 1.0, animated: true },
  reference: { color: "#52C41A", width: 1.6, animated: true },
  derived_from: { color: "#FA8C16", width: 1.2, animated: false },
  contradicts: { color: token.colorError, width: 1.8, animated: false },
  mapping: { color: "#722ED1", width: 1.0, animated: true },
});

const edgeTypeLabels: Record<GraphEdgeType, string> = {
  link: "wiki.graph.edgeType.link",
  backlink: "wiki.graph.edgeType.backlink",
  reference: "wiki.graph.edgeType.reference",
  derived_from: "wiki.graph.edgeType.derived",
  contradicts: "wiki.graph.edgeType.contradicts",
  mapping: "wiki.graph.edgeType.mapping",
};

// 节点颜色缓存：nodeId → color
function buildNodeColorCache(
  nodes: GraphNode[],
  communities?: Map<string, number>,
  token?: TokenType,
): Map<string, string> {
  const cache = new Map<string, string>();
  const typeMap = token ? getNodeColorMap(token) : {
    note: "#1890ff",
    concept: "#52c41a",
    entity: "#fa8c16",
    source: "#eb2f96",
  };
  for (const node of nodes) {
    if (communities && communities.has(node.id)) {
      const cid = communities.get(node.id)!;
      cache.set(node.id, communityPalette[cid % communityPalette.length]);
    } else {
      cache.set(node.id, typeMap[node.type] || typeMap.note);
    }
  }
  return cache;
}

function getNodeSize(node: GraphNode): number {
  const degree = node.linkCount + node.backlinkCount;
  if (node.type === "entity") { return Math.max(6, Math.min(22, 6 + degree * 0.8)); }
  if (node.type === "concept") { return Math.max(5, Math.min(18, 5 + degree * 0.6)); }
  return Math.max(4, Math.min(15, 4 + degree * 0.4));
}

// ─────────────────────────────────────────────────────────────────────────────
// 核心组件
// ─────────────────────────────────────────────────────────────────────────────

interface Particle {
  edgeIndex: number;
  progress: number; // 0..1
  speed: number;
  size: number;
  color: string;
}

const GraphViewInner = forwardRef<GraphViewHandle, GraphViewProps>(({
  data,
  wikiId,
  onNodeClick,
  onNodeDoubleClick,
  onNodeHover,
  onContextMenu,
  onDeleteNode,
  onDeselect,
  highlightedNodeIds,
  selectedNodeId,
  communities,
  showMinimap = true,
}, ref) => {
  const { token } = theme.useToken();
  const { t } = useTranslation();

  const canvasRef = useRef<HTMLCanvasElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const rafRef = useRef<number>(0);

  // 物理节点和边（在 ref 中持久化，不触发 React 重渲染）
  const physNodesRef = useRef<PhysicsNode[]>([]);
  const physEdgesRef = useRef<PhysicsEdge[]>([]);
  const particlesRef = useRef<Particle[]>([]);
  const nodeMetaRef = useRef<Map<string, GraphNode>>(new Map());
  const nodeColorRef = useRef<Map<string, string>>(new Map());
  const nodeSizeRef = useRef<Map<string, number>>(new Map());
  const nodeSpriteCacheRef = useRef<Map<string, HTMLCanvasElement>>(new Map());
  const edgeMetaRef = useRef<
    {
      source: string;
      target: string;
      type: GraphEdgeType;
      animated: boolean;
      color: string;
      width: number;
      sourceIdx: number;
      targetIdx: number;
    }[]
  >([]);

  // 预构建的邻居表和节点索引（缓存复用，避免每帧重建）
  const neighborMapCacheRef = useRef<NeighborMap>(new Map());
  const nodeMapCacheRef = useRef<NodeMap>(new Map());

  // 预渲染的背景画布（避免每帧重建渐变）
  const bgCacheRef = useRef<HTMLCanvasElement | null>(null);
  const bgCacheSizeRef = useRef({ w: 0, h: 0 });

  // 相机变换
  const cameraRef = useRef({ x: 0, y: 0, zoom: 1 });

  // 交互状态
  const dragRef = useRef<{ nodeId: string } | null>(null);
  const panRef = useRef<{ startX: number; startY: number; camX: number; camY: number } | null>(null);
  const hoverNodeRef = useRef<string | null>(null);
  const selectedNodeIdRef = useRef<string | null>(null);
  const highlightSetRef = useRef<Set<string> | undefined>(undefined);

  // 脉动相位
  const phaseRef = useRef(0);
  const frameCounterRef = useRef(0);
  const stableFrameCounterRef = useRef(0);
  const idleCounterRef = useRef(0);

  // 鱼眼 / 聚类 状态
  const fisheyeEnabledRef = useRef(true);
  const clusterModeRef = useRef(false);
  const mouseScreenRef = useRef({ x: 0, y: 0, active: false });
  const communityCentroidsRef = useRef<Map<number, { cx: number; cy: number; count: number }>>(new Map());

  const gridIndexRef = useRef<Map<string, string[]>>(new Map());
  const GRID_CELL_SIZE = 80;

  const minimapRef = useRef<HTMLCanvasElement>(null);
  const [minimapOpen, setMinimapOpen] = useState(true);
  const minimapDragRef = useRef(false);

  // wikiId ref，用于布局持久化
  const wikiIdRef = useRef<string | undefined>(wikiId);
  wikiIdRef.current = wikiId;

  // 渲染缓存：posMap 和预计算的邻居集合，避免每帧重建 O(N)/O(E)
  const posMapRef = useRef<Map<string, PhysicsNode>>(new Map());
  const neighborsRef = useRef<Map<string, Set<string>>>(new Map());

  const [fisheyeEnabled, setFisheyeEnabled] = useState(true);
  const [clusterMode, setClusterMode] = useState(false);

  // Tooltip: 节点内容用 useState (低频更新)，位置用 ref + DOM 操作 (高频更新)
  const [tooltipNodeIdState, setTooltipNodeIdState] = useState<string | null>(null);
  const tooltipNodeIdRef = useRef<string | null>(null);
  const tooltipPosRef = useRef({ x: 0, y: 0 });
  const tooltipVisibleRef = useRef(false);
  const tooltipRef = useRef<HTMLDivElement | null>(null);

  // 同步 tooltip 节点 ID 到 ref（供渲染循环使用，避免闭包过期）
  useEffect(() => {
    tooltipNodeIdRef.current = tooltipNodeIdState;
  }, [tooltipNodeIdState]);

  // 尺寸
  const [dimensions, setDimensions] = useState({ width: 800, height: 600 });
  const [isFullscreen, setIsFullscreen] = useState(false);
  const [statsOpen, setStatsOpen] = useState(false);
  const [legendOpen, setLegendOpen] = useState(false);

  // 边类型可见性筛选
  const [visibleEdgeTypes, setVisibleEdgeTypes] = useState<Set<GraphEdgeType>>(
    new Set(["link", "backlink", "reference", "derived_from", "contradicts", "mapping"]),
  );
  const visibleEdgeTypesRef = useRef(visibleEdgeTypes);
  visibleEdgeTypesRef.current = visibleEdgeTypes;

  const toggleEdgeType = useCallback((type: GraphEdgeType) => {
    setVisibleEdgeTypes((prev) => {
      const next = new Set(prev);
      if (next.has(type)) {
        next.delete(type);
      } else {
        next.add(type);
      }
      return next;
    });
  }, []);

  // 同步 selected/highlight 到 ref
  useEffect(() => {
    selectedNodeIdRef.current = selectedNodeId ?? null;
  }, [selectedNodeId]);
  useEffect(() => {
    highlightSetRef.current = highlightedNodeIds && highlightedNodeIds.size > 0 ? highlightedNodeIds : undefined;
  }, [highlightedNodeIds]);
  useEffect(() => {
    fisheyeEnabledRef.current = fisheyeEnabled;
  }, [fisheyeEnabled]);
  useEffect(() => {
    clusterModeRef.current = clusterMode;
  }, [clusterMode]);

  // 社区可见性筛选
  const [visibleCommunities, setVisibleCommunities] = useState<Set<number>>(new Set());
  const visibleCommunitiesRef = useRef(visibleCommunities);
  visibleCommunitiesRef.current = visibleCommunities;

  // 社区筛选预计算：缓存全量社区集合和筛选状态，避免每帧在绘制函数内重建
  const visibleCommunitiesAllSetRef = useRef<Set<number>>(new Set());
  const hasCommunityFilterRef = useRef(false);

  const toggleCommunity = useCallback((cid: number) => {
    setVisibleCommunities((prev) => {
      const next = new Set(prev);
      if (next.has(cid)) {
        next.delete(cid);
      } else {
        next.add(cid);
      }
      return next;
    });
  }, []);

  // 当 communities 数据变化时，初始化可见的社区
  useEffect(() => {
    if (!communities) { return; }
    const uniqueCommunities = new Set<number>();
    for (const cid of communities.values()) {
      uniqueCommunities.add(cid);
    }
    setVisibleCommunities(uniqueCommunities);
  }, [communities]);

  // 预计算社区筛选状态：全量社区集合 + 是否启用筛选
  useEffect(() => {
    if (!communities) {
      visibleCommunitiesAllSetRef.current = new Set();
      hasCommunityFilterRef.current = false;
      return;
    }
    const allCids = new Set<number>();
    for (const cid of communities.values()) {
      allCids.add(cid);
    }
    visibleCommunitiesAllSetRef.current = allCids;
    hasCommunityFilterRef.current = visibleCommunities.size < allCids.size;
  }, [communities, visibleCommunities]);

  // 选中节点时自动聚焦（搜索定位 / 点击导航）
  const prevSelectedRef = useRef<string | null>(null);
  // 画布交互（点击/拖拽/右键/触摸）触发的选中不聚焦——用户已在节点旁，
  // 相机突变会破坏拖拽手感；仅外部驱动（搜索定位/列表导航/笔记跳转）时聚焦
  const suppressAutoFocusRef = useRef(false);
  useEffect(() => {
    if (!selectedNodeId || selectedNodeId === prevSelectedRef.current) {
      return;
    }
    prevSelectedRef.current = selectedNodeId;
    if (suppressAutoFocusRef.current) {
      suppressAutoFocusRef.current = false;
      return;
    }
    // 延迟到下一帧，确保物理节点已就绪
    requestAnimationFrame(() => {
      const nodes = physNodesRef.current;
      const node = nodes.find((n) => n.id === selectedNodeId);
      if (!node) { return; }
      // 平滑移动相机到节点位置
      const targetZoom = Math.max(cameraRef.current.zoom, 1.5);
      cameraRef.current.x = -node.x * targetZoom;
      cameraRef.current.y = -node.y * targetZoom;
      cameraRef.current.zoom = targetZoom;
    });
  }, [selectedNodeId]);

  // 容器尺寸监听
  useEffect(() => {
    const el = containerRef.current;
    if (!el) { return; }
    const update = () => setDimensions({ width: el.clientWidth, height: el.clientHeight });
    update();
    const ro = new ResizeObserver(update);
    ro.observe(el);
    return () => ro.disconnect();
  }, []);

  // 全屏状态
  useEffect(() => {
    const handle = () => setIsFullscreen(!!document.fullscreenElement);
    document.addEventListener("fullscreenchange", handle);
    return () => document.removeEventListener("fullscreenchange", handle);
  }, []);

  // 数据变化 → 重建物理世界
  useEffect(() => {
    if (!data || data.nodes.length === 0) { return; }

    const colorCache = buildNodeColorCache(data.nodes, communities, token);
    nodeColorRef.current = colorCache;
    buildNodeSpriteCache();

    // 构建物理节点
    const pNodes: PhysicsNode[] = data.nodes.map((n, i) => ({
      id: n.id,
      x: n.x ?? 0,
      y: n.y ?? 0,
      vx: 0,
      vy: 0,
      fx: 0,
      fy: 0,
      mass: 1 + (n.linkCount + n.backlinkCount) * 0.2,
      fixed: false,
      kind: n.type,
      idx: i,
    }));

    // 首次布局：优先从 localStorage 加载已保存的布局
    if (wikiId) {
      const saved = loadLayout(wikiId);
      if (saved) {
        applySavedLayout(pNodes, saved);
      }
    }

    // 若无已保存布局或匹配率太低，则使用力导向布局
    const hasPositions = pNodes.some((n) => n.x !== 0 || n.y !== 0);
    if (!hasPositions) {
      initializePositions(pNodes, dimensions.width, dimensions.height);
    }

    // 邻接表 → 物理边
    const adjacency = new Map<string, Set<string>>();
    for (const n of data.nodes) { adjacency.set(n.id, new Set()); }
    for (const e of data.edges) {
      adjacency.get(e.source)?.add(e.target);
      adjacency.get(e.target)?.add(e.source);
    }
    const avgDegree = data.edges.length > 0 ? (data.edges.length * 2) / data.nodes.length : 1;
    const pEdges = buildPhysicsEdges(adjacency, pNodes, avgDegree);

    physNodesRef.current = pNodes;
    physEdgesRef.current = pEdges;

    // 构建渲染缓存：posMap (O(N) 一次性) + 邻居集合 (O(E) 一次性)
    const posMap = new Map<string, PhysicsNode>();
    for (const n of pNodes) { posMap.set(n.id, n); }
    posMapRef.current = posMap;
    neighborsRef.current = adjacency; // 已在上方构建

    // 构建物理引擎缓存：邻居表 + 节点索引（供 stepPhysics 复用，避免每帧重建）
    neighborMapCacheRef.current = buildNeighborMap(pEdges);
    nodeMapCacheRef.current = buildNodeMap(pNodes);

    // 重置稳定计数器，强制物理引擎重新运行
    stableFrameCounterRef.current = 0;

    // 构建网格空间索引
    const gridIndex = new Map<string, string[]>();
    for (const n of pNodes) {
      const gx = Math.floor(n.x / GRID_CELL_SIZE);
      const gy = Math.floor(n.y / GRID_CELL_SIZE);
      const key = `${gx},${gy}`;
      const bucket = gridIndex.get(key);
      if (bucket) {
        bucket.push(n.id);
      } else {
        gridIndex.set(key, [n.id]);
      }
    }
    gridIndexRef.current = gridIndex;

    // 节点元数据
    const metaMap = new Map<string, GraphNode>();
    const sizeMap = new Map<string, number>();
    for (const n of data.nodes) {
      metaMap.set(n.id, n);
      sizeMap.set(n.id, getNodeSize(n));
    }
    nodeMetaRef.current = metaMap;
    nodeSizeRef.current = sizeMap;

    // 边元数据（用于渲染），直接存储 sourceIdx/targetIdx 避免渲染循环中的 Map 查找
    const edgeStyles = getEdgeTypeStylesMap(token);
    const idToIdx = new Map<string, number>();
    for (let i = 0; i < pNodes.length; i++) {
      idToIdx.set(pNodes[i].id, i);
    }
    edgeMetaRef.current = data.edges.map((e) => {
      const style = edgeStyles[e.type] || edgeStyles.link;
      return {
        source: e.source,
        target: e.target,
        type: e.type,
        animated: style.animated,
        color: style.color,
        width: style.width,
        sourceIdx: idToIdx.get(e.source) ?? -1,
        targetIdx: idToIdx.get(e.target) ?? -1,
      };
    });

    // 粒子系统
    const particles: Particle[] = [];
    for (let i = 0; i < data.edges.length; i++) {
      const em = edgeMetaRef.current[i];
      if (em.animated) {
        // 每条动画边 1-2 个粒子
        const count = em.type === "reference" ? 2 : 1;
        for (let j = 0; j < count; j++) {
          particles.push({
            edgeIndex: i,
            progress: Math.random(),
            speed: 0.003 + Math.random() * 0.004,
            size: em.type === "reference" ? 2.5 : 1.8,
            color: em.color,
          });
        }
      }
    }
    particlesRef.current = particles;

    // 初始布局：用 requestAnimationFrame 异步迭代，避免阻塞主线程
    const config: PhysicsConfig = {
      theta: 0.5,
      repulsion: 6000,
      gravity: 0.01,
      damping: 0.82,
      dt: 0.5,
      springForce: 0.06,
      springDamping: 0.8,
      maxVelocity: 10,
    };
    let iter = 0;
    const MAX_ITER = 30;
    const runLayoutStep = () => {
      if (iter >= MAX_ITER) {
        return;
      }
      stepPhysics(pNodes, pEdges, config);
      iter++;
      // 每 3 帧让一帧给浏览器渲染
      if (iter % 3 === 0) {
        requestAnimationFrame(runLayoutStep);
      } else {
        runLayoutStep();
      }
    };
    requestAnimationFrame(runLayoutStep);
  }, [data, communities, token]);

  // 主动画循环
  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) { return; }

    const ctx = canvas.getContext("2d");
    if (!ctx) { return; }

    let running = true;

    // 预渲染背景到离屏画布
    const ensureBackground = (w: number, h: number) => {
      const cache = bgCacheRef.current;
      if (cache && bgCacheSizeRef.current.w === w && bgCacheSizeRef.current.h === h) {
        return cache;
      }
      const offscreen = document.createElement("canvas");
      offscreen.width = w;
      offscreen.height = h;
      const offCtx = offscreen.getContext("2d")!;

      // 绘制背景
      const grad = offCtx.createRadialGradient(w / 2, h / 2, 0, w / 2, h / 2, Math.max(w, h) * 0.7);
      grad.addColorStop(0, token.colorBgContainer);
      grad.addColorStop(1, token.colorBgElevated);
      offCtx.fillStyle = grad;
      offCtx.fillRect(0, 0, w, h);

      offCtx.fillStyle = hexToRgba(token.colorText, 0.03);
      const gridSize = 40;
      for (let x = gridSize; x < w; x += gridSize) {
        for (let y = gridSize; y < h; y += gridSize) {
          offCtx.beginPath();
          offCtx.arc(x, y, 1, 0, Math.PI * 2);
          offCtx.fill();
        }
      }

      bgCacheRef.current = offscreen;
      bgCacheSizeRef.current = { w, h };
      return offscreen;
    };

    const render = () => {
      if (!running) { return; }

      const dpr = window.devicePixelRatio || 1;
      const w = dimensions.width;
      const h = dimensions.height;

      if (canvas.width !== w * dpr || canvas.height !== h * dpr) {
        canvas.width = w * dpr;
        canvas.height = h * dpr;
        canvas.style.width = `${w}px`;
        canvas.style.height = `${h}px`;
        // 尺寸变化时重置背景缓存
        bgCacheRef.current = null;
      }

      ctx.setTransform(dpr, 0, 0, dpr, 0, 0);

      // 绘制缓存背景（一次性拷贝，避免每帧重建渐变）
      const bg = ensureBackground(w, h);
      ctx.drawImage(bg, 0, 0);

      // 相机变换
      const cam = cameraRef.current;
      ctx.save();
      ctx.translate(w / 2 + cam.x, h / 2 + cam.y);
      ctx.scale(cam.zoom, cam.zoom);

      const nodes = physNodesRef.current;
      const edges = physEdgesRef.current;
      frameCounterRef.current++;

      // ── 物理步进优化：稳定检测 + 缓存复用 + 降频 ──
      const hasDrag = !!dragRef.current;
      const hasInteraction = mouseScreenRef.current.active || !!panRef.current;

      if (nodes.length > 0 && !hasDrag) {
        const stable = isSystemStable(nodes, 0.15);

        if (stable && !hasInteraction) {
          idleCounterRef.current++;
        } else {
          idleCounterRef.current = 0;
        }

        // 稳定无交互时每 12 帧执行一次物理，拖拽/交互时全速
        const shouldRunPhysics = hasInteraction || !stable || idleCounterRef.current % 12 === 0;

        if (shouldRunPhysics) {
          const enableClusters = clusterModeRef.current && communities;
          let centroids = communityCentroidsRef.current;
          if (enableClusters && frameCounterRef.current % 3 === 0) {
            centroids = computeCommunityCentroids(nodes, communities!);
            communityCentroidsRef.current = centroids;
          }
          const config: PhysicsConfig = {
            theta: 0.5,
            repulsion: 6000,
            gravity: 0.01,
            damping: 0.92,
            dt: 0.25,
            springForce: 0.04,
            springDamping: 0.85,
            maxVelocity: 4,
            clusterForce: enableClusters ? 0.15 : undefined,
          };
          stepPhysics(
            nodes,
            edges,
            config,
            undefined,
            enableClusters ? communities : undefined,
            enableClusters ? centroids : undefined,
            neighborMapCacheRef.current,
          );

          // 更新网格空间索引（物理改变位置后）
          if (frameCounterRef.current % 3 === 0) {
            const gridIndex = new Map<string, string[]>();
            for (const n of nodes) {
              const gx = Math.floor(n.x / GRID_CELL_SIZE);
              const gy = Math.floor(n.y / GRID_CELL_SIZE);
              const key = `${gx},${gy}`;
              const bucket = gridIndex.get(key);
              if (bucket) {
                bucket.push(n.id);
              } else {
                gridIndex.set(key, [n.id]);
              }
            }
            gridIndexRef.current = gridIndex;
          }
        }
      }

      phaseRef.current += 0.02;

      // 计算鱼眼参数（世界坐标下的鼠标位置 + 放大因子）
      const fisheye = computeFisheye();

      // 计算当前视口的世界坐标范围（用于视口裁剪）
      const viewWorld = {
        x0: (-w / 2 - cam.x) / cam.zoom - 50,
        y0: (-h / 2 - cam.y) / cam.zoom - 50,
        x1: (w / 2 - cam.x) / cam.zoom + 50,
        y1: (h / 2 - cam.y) / cam.zoom + 50,
      };

      // 绘制社区聚类区域（在边之前绘制，作为背景层）
      if (clusterModeRef.current && communities) {
        drawClusterRegions(ctx, nodes);
      }

      // 绘制（传入视口范围用于裁剪）
      drawEdgesOptimized(ctx, nodes, fisheye, viewWorld);
      drawParticlesOptimized(ctx, nodes, fisheye, viewWorld);
      drawNodesOptimized(ctx, nodes, fisheye, viewWorld);

      ctx.restore();

      // 同步 tooltip DOM 位置（每帧更新，不走 React）
      if (tooltipRef.current) {
        if (tooltipVisibleRef.current && tooltipNodeIdRef.current) {
          tooltipRef.current.style.left = `${tooltipPosRef.current.x}px`;
          tooltipRef.current.style.top = `${tooltipPosRef.current.y}px`;
          tooltipRef.current.style.display = "block";
        } else {
          tooltipRef.current.style.display = "none";
        }
      }

      if (showMinimap && minimapOpen && minimapRef.current && frameCounterRef.current % 3 === 0) {
        const mmCanvas = minimapRef.current;
        const mmCtx = mmCanvas.getContext("2d");
        if (mmCtx) {
          drawMinimap(mmCtx, nodes);
        }
      }

      rafRef.current = requestAnimationFrame(render);
    };

    rafRef.current = requestAnimationFrame(render);
    return () => {
      running = false;
      cancelAnimationFrame(rafRef.current);
    };
  }, [dimensions, token]);

  function getScreenToWorld(sx: number, sy: number): { x: number; y: number } {
    const cam = cameraRef.current;
    const w = dimensions.width;
    const h = dimensions.height;
    return {
      x: (sx - w / 2 - cam.x) / cam.zoom,
      y: (sy - h / 2 - cam.y) / cam.zoom,
    };
  }

  // ── 鱼眼放大镜 ──
  // 鼠标位置附近的节点会被放大，形成类似 Obsidian 的局部放大效果
  const FISHEYE_RADIUS = 180; // 世界坐标下的影响半径
  const FISHEYE_STRENGTH = 0.45; // 放大强度 (0~1)

  interface FisheyeState {
    active: boolean;
    worldX: number;
    worldY: number;
    radius: number;
    strength: number;
  }

  function computeFisheye(): FisheyeState {
    const m = mouseScreenRef.current;
    if (!fisheyeEnabledRef.current || !m.active) {
      return { active: false, worldX: 0, worldY: 0, radius: FISHEYE_RADIUS, strength: 0 };
    }
    const world = getScreenToWorld(m.x, m.y);
    return {
      active: true,
      worldX: world.x,
      worldY: world.y,
      radius: FISHEYE_RADIUS / cameraRef.current.zoom,
      strength: FISHEYE_STRENGTH,
    };
  }

  // 根据鱼眼计算节点的缩放倍率
  function fisheyeScale(nodeX: number, nodeY: number, fisheye: FisheyeState): number {
    if (!fisheye.active) { return 1; }
    const dx = nodeX - fisheye.worldX;
    const dy = nodeY - fisheye.worldY;
    const dist = Math.sqrt(dx * dx + dy * dy);
    if (dist > fisheye.radius) { return 1; }
    // 平滑衰减：距离越近放大越多
    const t = 1 - dist / fisheye.radius;
    return 1 + fisheye.strength * t * t * (3 - 2 * t); // smoothstep
  }

  // ── 社区聚类区域渲染 ──
  function drawClusterRegions(ctx: CanvasRenderingContext2D, nodes: PhysicsNode[]) {
    if (!communities) { return; }
    const centroids = communityCentroidsRef.current;
    if (centroids.size === 0) { return; }

    // 按社区分组收集节点位置
    const communityNodes = new Map<number, { sx: number; sy: number }[]>();
    for (const node of nodes) {
      const cid = communities.get(node.id);
      if (cid === undefined) { continue; }
      const list = communityNodes.get(cid) ?? [];
      list.push({ sx: node.x, sy: node.y });
      communityNodes.set(cid, list);
    }

    // 为每个社区绘制一个半透明的"气泡"
    for (const [cid, points] of communityNodes) {
      if (points.length < 2) { continue; }
      const color = communityPalette[cid % communityPalette.length];

      // 计算该社区节点的包围盒
      let minX = Infinity, minY = Infinity, maxX = -Infinity, maxY = -Infinity;
      for (const p of points) {
        if (p.sx < minX) { minX = p.sx; }
        if (p.sy < minY) { minY = p.sy; }
        if (p.sx > maxX) { maxX = p.sx; }
        if (p.sy > maxY) { maxY = p.sy; }
      }
      const cx = (minX + maxX) / 2;
      const cy = (minY + maxY) / 2;
      const rx = (maxX - minX) / 2 + 40;
      const ry = (maxY - minY) / 2 + 40;

      // 绘制柔化的椭圆背景
      ctx.save();
      const grad = ctx.createRadialGradient(cx, cy, 0, cx, cy, Math.max(rx, ry));
      grad.addColorStop(0, hexToRgba(color, 0.12));
      grad.addColorStop(0.6, hexToRgba(color, 0.06));
      grad.addColorStop(1, hexToRgba(color, 0));
      ctx.fillStyle = grad;
      ctx.beginPath();
      ctx.ellipse(cx, cy, rx, ry, 0, 0, Math.PI * 2);
      ctx.fill();

      // 社区标签
      const centroid = centroids.get(cid);
      if (centroid && centroid.count >= 2) {
        ctx.globalAlpha = 0.5;
        ctx.font = "bold 11px Inter, system-ui, sans-serif";
        ctx.textAlign = "center";
        ctx.textBaseline = "middle";
        ctx.fillStyle = color;
        ctx.fillText(t("wiki.graph.clusterLabel", { id: cid }) + ` · ${centroid.count}`, cx, cy - ry + 14);
      }
      ctx.restore();
    }
  }

  // ── 优化绘制函数：带视口裁剪，跳过屏幕外元素 ──

  function isInView(
    x: number,
    y: number,
    view: { x0: number; y0: number; x1: number; y1: number },
    margin = 80,
  ): boolean {
    return x >= view.x0 - margin && x <= view.x1 + margin && y >= view.y0 - margin && y <= view.y1 + margin;
  }

  function drawEdgesOptimized(
    ctx: CanvasRenderingContext2D,
    nodes: PhysicsNode[],
    fisheye: FisheyeState,
    viewWorld: { x0: number; y0: number; x1: number; y1: number },
  ) {
    const edgeMeta = edgeMetaRef.current;
    const hovered = hoverNodeRef.current;
    const selected = selectedNodeIdRef.current;
    const visibleTypes = visibleEdgeTypesRef.current;
    const visibleCommunitiesSet = visibleCommunitiesRef.current;
    const zoom = cameraRef.current.zoom;

    const hasCommunityFilter = hasCommunityFilterRef.current;

    for (let i = 0; i < edgeMeta.length; i++) {
      const em = edgeMeta[i];

      if (!visibleTypes.has(em.type)) { continue; }

      // 直接数组访问，避免 Map 查找
      const s = nodes[em.sourceIdx];
      const t = nodes[em.targetIdx];
      if (!s || !t) { continue; }

      // 视口裁剪：两端节点都不在视口内时跳过
      if (!isInView(s.x, s.y, viewWorld) && !isInView(t.x, t.y, viewWorld)) { continue; }

      if (hasCommunityFilter) {
        const sCid = communities?.get(em.source);
        const tCid = communities?.get(em.target);
        const sVisible = sCid === undefined || visibleCommunitiesSet.has(sCid);
        const tVisible = tCid === undefined || visibleCommunitiesSet.has(tCid);
        if (!sVisible || !tVisible) { continue; }
      }

      const isRelevant = hovered && (em.source === hovered || em.target === hovered)
        || selected && (em.source === selected || em.target === selected);

      // 低缩放下简化渲染
      if (zoom < 0.3 && !isRelevant) { continue; }

      const sScale = fisheyeScale(s.x, s.y, fisheye);
      const tScale = fisheyeScale(t.x, t.y, fisheye);
      const avgScale = (sScale + tScale) / 2;

      ctx.beginPath();
      ctx.moveTo(s.x, s.y);
      ctx.lineTo(t.x, t.y);

      if (isRelevant) {
        ctx.strokeStyle = em.color;
        ctx.lineWidth = em.width * 1.5 * avgScale;
        ctx.globalAlpha = 0.9;
      } else {
        ctx.strokeStyle = em.color;
        ctx.lineWidth = em.width * avgScale;
        ctx.globalAlpha = 0.25;
      }
      ctx.stroke();
      ctx.globalAlpha = 1;
    }
  }

  function drawParticlesOptimized(
    ctx: CanvasRenderingContext2D,
    nodes: PhysicsNode[],
    fisheye: FisheyeState,
    viewWorld: { x0: number; y0: number; x1: number; y1: number },
  ) {
    const zoom = cameraRef.current.zoom;
    if (zoom < 0.5) { return; }

    const particles = particlesRef.current;
    const edgeMeta = edgeMetaRef.current;
    const visibleTypes = visibleEdgeTypesRef.current;

    const isStable = idleCounterRef.current > 0;

    // 稳定时粒子每 3 帧才更新一次位置
    if (!isStable || idleCounterRef.current % 3 === 0) {
      for (const p of particles) {
        p.progress += p.speed;
        if (p.progress > 1) { p.progress -= 1; }
      }
    }

    for (const p of particles) {
      const em = edgeMeta[p.edgeIndex];
      if (!em) { continue; }
      if (!visibleTypes.has(em.type)) { continue; }

      // 直接数组访问，避免 Map 查找
      const s = nodes[em.sourceIdx];
      const t = nodes[em.targetIdx];
      if (!s || !t) { continue; }

      const x = s.x + (t.x - s.x) * p.progress;
      const y = s.y + (t.y - s.y) * p.progress;

      // 视口裁剪：粒子不在视口内时跳过
      if (!isInView(x, y, viewWorld, 30)) { continue; }

      const scale = fisheyeScale(x, y, fisheye);
      const alpha = 0.6 + 0.4 * Math.sin(p.progress * Math.PI * 2);
      ctx.save();
      // 稳定时跳过 shadowBlur（开销大）
      if (!isStable) {
        ctx.shadowColor = p.color;
        ctx.shadowBlur = 6 * scale;
      }
      ctx.fillStyle = p.color;
      ctx.globalAlpha = alpha;
      ctx.beginPath();
      ctx.arc(x, y, p.size * scale, 0, Math.PI * 2);
      ctx.fill();
      ctx.restore();
    }
    ctx.globalAlpha = 1;
  }

  function drawNodesOptimized(
    ctx: CanvasRenderingContext2D,
    nodes: PhysicsNode[],
    fisheye: FisheyeState,
    viewWorld: { x0: number; y0: number; x1: number; y1: number },
  ) {
    const phase = phaseRef.current;
    const hovered = hoverNodeRef.current;
    const selected = selectedNodeIdRef.current;
    const highlight = highlightSetRef.current;
    const hasHighlight = highlight && highlight.size > 0;

    const neighbors = neighborsRef.current;
    const neighborsOfHovered = hovered ? (neighbors.get(hovered) || EMPTY_SET) : EMPTY_SET;
    const neighborsOfSelected = selected ? (neighbors.get(selected) || EMPTY_SET) : EMPTY_SET;

    const visibleCommunitiesSet = visibleCommunitiesRef.current;
    const hasCommunityFilter = hasCommunityFilterRef.current;

    const zoom = cameraRef.current.zoom;
    const showAllLabels = zoom >= 0.6 && !hasHighlight && !hovered && !selected && nodes.length < 300;

    for (const node of nodes) {
      // 视口裁剪：节点不在视口内时跳过（大幅减少绘制开销）
      if (!isInView(node.x, node.y, viewWorld)) { continue; }

      if (hasCommunityFilter) {
        const cid = communities?.get(node.id);
        if (cid !== undefined && !visibleCommunitiesSet.has(cid)) { continue; }
      }

      const color = nodeColorRef.current.get(node.id) || token.colorPrimary;
      const baseSize = nodeSizeRef.current.get(node.id) || 6;

      const feScale = fisheyeScale(node.x, node.y, fisheye);

      let size = baseSize * feScale;
      let alpha = 1;
      let glowAlpha = 0.4;
      let glowRadius = baseSize * 2.5 * feScale;
      let showLabel = false;

      const isSelected = selected === node.id;
      const isHovered = hovered === node.id;

      if (isSelected) {
        size = baseSize * 1.8 * feScale;
        glowAlpha = 0.8;
        glowRadius = baseSize * 3 * feScale;
        showLabel = true;
      } else if (isHovered) {
        size = baseSize * 1.5 * feScale;
        glowAlpha = 0.6;
        glowRadius = baseSize * 2.5 * feScale;
        showLabel = true;
      } else if (selected && neighborsOfSelected.has(node.id)) {
        size = baseSize * 1.2 * feScale;
        glowAlpha = 0.3;
        showLabel = true;
      } else if (hovered && neighborsOfHovered.has(node.id)) {
        size = baseSize * 1.1 * feScale;
        glowAlpha = 0.25;
      } else if (hasHighlight && !highlight!.has(node.id)) {
        alpha = 0.15;
        glowAlpha = 0;
        size = baseSize * 0.8 * feScale;
      } else if (hovered || selected) {
        alpha = 0.3;
        glowAlpha = 0;
        size = baseSize * 0.85 * feScale;
      }

      const pulse = 1 + Math.sin(phase + node.x * 0.01) * 0.08;
      const finalSize = size * pulse;

      // 仅对 hovered/selected/邻居节点绘制 glow，跳过远处节点
      // 视口裁剪增强：0.6x 缩放以下完全跳过 glow
      if (glowAlpha > 0 && zoom >= 0.6) {
        ctx.save();
        // 稳定时跳过 shadowBlur（开销大）
        if (idleCounterRef.current === 0) {
          ctx.shadowColor = color;
          ctx.shadowBlur = glowRadius;
        }
        ctx.globalAlpha = glowAlpha * alpha;
        ctx.beginPath();
        ctx.arc(node.x, node.y, finalSize, 0, Math.PI * 2);
        ctx.fillStyle = color;
        ctx.fill();
        ctx.restore();
      }

      // 节点核心（预渲染 sprite 缓存）
      const sprite = nodeSpriteCacheRef.current.get(color);
      ctx.save();
      ctx.globalAlpha = alpha;
      if (sprite) {
        const dstSize = finalSize * 2;
        ctx.drawImage(sprite, 0, 0, SPRITE_SIZE, SPRITE_SIZE, node.x - finalSize, node.y - finalSize, dstSize, dstSize);
      } else {
        const grad = ctx.createRadialGradient(
          node.x - finalSize * 0.3,
          node.y - finalSize * 0.3,
          0,
          node.x,
          node.y,
          finalSize,
        );
        grad.addColorStop(0, lightenColor(color, 40));
        grad.addColorStop(0.7, color);
        grad.addColorStop(1, darkenColor(color, 20));
        ctx.beginPath();
        ctx.arc(node.x, node.y, finalSize, 0, Math.PI * 2);
        ctx.fillStyle = grad;
        ctx.fill();
      }
      ctx.restore();

      // 标签：仅在需要时绘制
      if (showLabel || showAllLabels) {
        const meta = nodeMetaRef.current.get(node.id);
        if (meta) {
          ctx.save();
          ctx.globalAlpha = alpha * 0.9;
          ctx.font = `${Math.round(12 * feScale)}px Inter, system-ui, sans-serif`;
          ctx.textAlign = "center";
          ctx.textBaseline = "top";
          const label = meta.title.length > 15 ? meta.title.slice(0, 13) + "…" : meta.title;
          ctx.fillStyle = token.colorText;
          ctx.fillText(label, node.x, node.y + finalSize + 4);
          ctx.restore();
        }
      }
    }

    // 绘制鱼眼透镜边框
    if (fisheye.active) {
      ctx.save();
      ctx.strokeStyle = token.colorPrimary;
      ctx.globalAlpha = 0.15;
      ctx.lineWidth = 1 / cameraRef.current.zoom;
      ctx.setLineDash([4 / cameraRef.current.zoom, 4 / cameraRef.current.zoom]);
      ctx.beginPath();
      ctx.arc(fisheye.worldX, fisheye.worldY, fisheye.radius, 0, Math.PI * 2);
      ctx.stroke();
      ctx.setLineDash([]);
      ctx.restore();
    }
  }

  // ── 颜色工具 ──
  function lightenColor(hex: string, percent: number): string {
    const num = parseInt(hex.slice(1), 16);
    const r = Math.min(255, (num >> 16) + percent);
    const g = Math.min(255, ((num >> 8) & 0xff) + percent);
    const b = Math.min(255, (num & 0xff) + percent);
    return `rgb(${r},${g},${b})`;
  }

  function darkenColor(hex: string, percent: number): string {
    const num = parseInt(hex.slice(1), 16);
    const r = Math.max(0, (num >> 16) - percent);
    const g = Math.max(0, ((num >> 8) & 0xff) - percent);
    const b = Math.max(0, (num & 0xff) - percent);
    return `rgb(${r},${g},${b})`;
  }

  function hexToRgba(hex: string, alpha: number): string {
    const num = parseInt(hex.slice(1), 16);
    const r = (num >> 16) & 0xff;
    const g = (num >> 8) & 0xff;
    const b = num & 0xff;
    return `rgba(${r},${g},${b},${alpha})`;
  }

  const SPRITE_SIZE = 128;

  function preRenderNodeSprite(color: string): HTMLCanvasElement {
    const canvas = document.createElement("canvas");
    canvas.width = SPRITE_SIZE;
    canvas.height = SPRITE_SIZE;
    const ctx = canvas.getContext("2d")!;
    const cx = SPRITE_SIZE / 2;
    const cy = SPRITE_SIZE / 2;
    const radius = SPRITE_SIZE * 0.47;

    const grad = ctx.createRadialGradient(
      cx - radius * 0.3,
      cy - radius * 0.3,
      0,
      cx,
      cy,
      radius,
    );
    grad.addColorStop(0, lightenColor(color, 40));
    grad.addColorStop(0.7, color);
    grad.addColorStop(1, darkenColor(color, 20));

    ctx.fillStyle = grad;
    ctx.beginPath();
    ctx.arc(cx, cy, radius, 0, Math.PI * 2);
    ctx.fill();
    return canvas;
  }

  function buildNodeSpriteCache() {
    const colors = new Set<string>();
    for (const color of nodeColorRef.current.values()) {
      colors.add(color);
    }
    const cache = new Map<string, HTMLCanvasElement>();
    for (const color of colors) {
      cache.set(color, preRenderNodeSprite(color));
    }
    nodeSpriteCacheRef.current = cache;
  }

  // ── 交互事件 ──

  const findNodeAt = useCallback((sx: number, sy: number): string | null => {
    const world = getScreenToWorld(sx, sy);
    const grid = gridIndexRef.current;
    const gx = Math.floor(world.x / GRID_CELL_SIZE);
    const gy = Math.floor(world.y / GRID_CELL_SIZE);
    const posMap = posMapRef.current;

    for (let dy = -1; dy <= 1; dy++) {
      for (let dx = -1; dx <= 1; dx++) {
        const key = `${gx + dx},${gy + dy}`;
        const ids = grid.get(key);
        if (!ids || ids.length === 0) { continue; }
        for (let i = ids.length - 1; i >= 0; i--) {
          const id = ids[i];
          const n = posMap.get(id);
          if (!n) { continue; }
          const size = nodeSizeRef.current.get(id) || 6;
          const wx = n.x - world.x;
          const wy = n.y - world.y;
          if (wx * wx + wy * wy < size * size) {
            return id;
          }
        }
      }
    }
    return null;
  }, [dimensions]);

  const MINIMAP_W = 200;
  const MINIMAP_H = 150;

  const drawMinimap = useCallback((mmCtx: CanvasRenderingContext2D, nodes: PhysicsNode[]) => {
    if (nodes.length === 0) { return; }

    let minX = Infinity, minY = Infinity, maxX = -Infinity, maxY = -Infinity;
    for (const n of nodes) {
      if (n.x < minX) { minX = n.x; }
      if (n.y < minY) { minY = n.y; }
      if (n.x > maxX) { maxX = n.x; }
      if (n.y > maxY) { maxY = n.y; }
    }
    const bboxW = Math.max(maxX - minX, 1);
    const bboxH = Math.max(maxY - minY, 1);
    const padX = bboxW * 0.1;
    const padY = bboxH * 0.1;
    minX -= padX;
    maxX += padX;
    minY -= padY;
    maxY += padY;

    const scale = Math.min(MINIMAP_W / (maxX - minX), MINIMAP_H / (maxY - minY));
    const offsetX = (MINIMAP_W - (maxX - minX) * scale) / 2;
    const offsetY = (MINIMAP_H - (maxY - minY) * scale) / 2;

    mmCtx.clearRect(0, 0, MINIMAP_W, MINIMAP_H);
    mmCtx.fillStyle = token.colorBgContainer;
    mmCtx.fillRect(0, 0, MINIMAP_W, MINIMAP_H);

    const gridSize = 20;
    mmCtx.fillStyle = hexToRgba(token.colorText, 0.05);
    for (let x = gridSize; x < MINIMAP_W; x += gridSize) {
      for (let y = gridSize; y < MINIMAP_H; y += gridSize) {
        mmCtx.beginPath();
        mmCtx.arc(x, y, 0.5, 0, Math.PI * 2);
        mmCtx.fill();
      }
    }

    for (const n of nodes) {
      const color = nodeColorRef.current.get(n.id) || token.colorPrimary;
      const mx = (n.x - minX) * scale + offsetX;
      const my = (n.y - minY) * scale + offsetY;
      mmCtx.fillStyle = color;
      mmCtx.beginPath();
      mmCtx.arc(mx, my, 1.8, 0, Math.PI * 2);
      mmCtx.fill();
    }

    const cam = cameraRef.current;
    const vx = ((-cam.x / cam.zoom) - minX) * scale + offsetX;
    const vy = ((-cam.y / cam.zoom) - minY) * scale + offsetY;
    const vw = (dimensions.width / cam.zoom) * scale;
    const vh = (dimensions.height / cam.zoom) * scale;

    mmCtx.save();
    mmCtx.strokeStyle = token.colorPrimary;
    mmCtx.lineWidth = 1.5;
    mmCtx.globalAlpha = 0.8;
    mmCtx.strokeRect(vx - vw / 2, vy - vh / 2, vw, vh);
    mmCtx.fillStyle = hexToRgba(token.colorPrimary, 0.08);
    mmCtx.fillRect(vx - vw / 2, vy - vh / 2, vw, vh);
    mmCtx.restore();
  }, [token, dimensions]);

  const getMinimapWorldBounds = useCallback(() => {
    const nodes = physNodesRef.current;
    if (nodes.length === 0) { return null; }
    let minX = Infinity, minY = Infinity, maxX = -Infinity, maxY = -Infinity;
    for (const n of nodes) {
      if (n.x < minX) { minX = n.x; }
      if (n.y < minY) { minY = n.y; }
      if (n.x > maxX) { maxX = n.x; }
      if (n.y > maxY) { maxY = n.y; }
    }
    const bboxW = Math.max(maxX - minX, 1);
    const bboxH = Math.max(maxY - minY, 1);
    const padX = bboxW * 0.1;
    const padY = bboxH * 0.1;
    minX -= padX;
    maxX += padX;
    minY -= padY;
    maxY += padY;
    const scale = Math.min(MINIMAP_W / (maxX - minX), MINIMAP_H / (maxY - minY));
    const offsetX = (MINIMAP_W - (maxX - minX) * scale) / 2;
    const offsetY = (MINIMAP_H - (maxY - minY) * scale) / 2;
    return { minX, minY, scale, offsetX, offsetY };
  }, []);

  const handleMinimapNavigate = useCallback((mmX: number, mmY: number) => {
    const bounds = getMinimapWorldBounds();
    if (!bounds) { return; }
    const wx = (mmX - bounds.offsetX) / bounds.scale + bounds.minX;
    const wy = (mmY - bounds.offsetY) / bounds.scale + bounds.minY;
    const cam = cameraRef.current;
    const targetZoom = Math.max(cam.zoom, 1);
    cam.x = -wx * targetZoom;
    cam.y = -wy * targetZoom;
    cam.zoom = targetZoom;
  }, [getMinimapWorldBounds]);

  const handleMinimapMouseDown = useCallback((e: ReactMouseEvent<HTMLCanvasElement>) => {
    e.preventDefault();
    const rect = minimapRef.current!.getBoundingClientRect();
    const mmX = e.clientX - rect.left;
    const mmY = e.clientY - rect.top;
    minimapDragRef.current = true;
    handleMinimapNavigate(mmX, mmY);
  }, [handleMinimapNavigate]);

  const handleMinimapMouseMove = useCallback((e: ReactMouseEvent<HTMLCanvasElement>) => {
    if (!minimapDragRef.current) { return; }
    const rect = minimapRef.current!.getBoundingClientRect();
    const mmX = e.clientX - rect.left;
    const mmY = e.clientY - rect.top;
    handleMinimapNavigate(mmX, mmY);
  }, [handleMinimapNavigate]);

  const handleMinimapMouseUp = useCallback(() => {
    minimapDragRef.current = false;
  }, []);

  const handleMouseDown = useCallback((e: ReactMouseEvent<HTMLCanvasElement>) => {
    const rect = canvasRef.current!.getBoundingClientRect();
    const sx = e.clientX - rect.left;
    const sy = e.clientY - rect.top;
    const nodeId = findNodeAt(sx, sy);

    if (nodeId) {
      suppressAutoFocusRef.current = true;
      const node = physNodesRef.current.find((n) => n.id === nodeId);
      if (node) {
        node.fixed = true;
        dragRef.current = { nodeId };
        onNodeClick?.(nodeId);
      }
    } else {
      // 开始平移
      panRef.current = { startX: e.clientX, startY: e.clientY, camX: cameraRef.current.x, camY: cameraRef.current.y };
      onDeselect?.();
    }
  }, [findNodeAt, onNodeClick, onDeselect]);

  const handleMouseMove = useCallback((e: ReactMouseEvent<HTMLCanvasElement>) => {
    const rect = canvasRef.current!.getBoundingClientRect();
    const sx = e.clientX - rect.left;
    const sy = e.clientY - rect.top;

    // 记录鼠标屏幕位置（供鱼眼使用）
    mouseScreenRef.current = { x: sx, y: sy, active: true };

    if (dragRef.current) {
      const world = getScreenToWorld(sx, sy);
      const node = physNodesRef.current.find((n) => n.id === dragRef.current!.nodeId);
      if (node) {
        node.x = world.x;
        node.y = world.y;
        node.vx = 0;
        node.vy = 0;
      }
    } else if (panRef.current) {
      const dx = e.clientX - panRef.current.startX;
      const dy = e.clientY - panRef.current.startY;
      cameraRef.current.x = panRef.current.camX + dx;
      cameraRef.current.y = panRef.current.camY + dy;
    } else {
      // hover 检测
      const nodeId = findNodeAt(sx, sy);
      if (nodeId !== hoverNodeRef.current) {
        hoverNodeRef.current = nodeId;
        onNodeHover?.(nodeId);
        canvasRef.current!.style.cursor = nodeId ? "pointer" : "grab";

        // 节点变化：更新内容（低频 React 渲染）+ 位置（ref）
        if (nodeId) {
          const tooltipX = Math.min(sx + 16, dimensions.width - 260);
          const tooltipY = Math.min(sy + 16, dimensions.height - 160);
          tooltipPosRef.current = { x: tooltipX, y: tooltipY };
          tooltipVisibleRef.current = true;
          setTooltipNodeIdState(nodeId);
        } else {
          tooltipVisibleRef.current = false;
          setTooltipNodeIdState(null);
        }
      } else if (nodeId) {
        // 同一节点移动：只更新位置（ref，无 React 渲染）
        const tooltipX = Math.min(sx + 16, dimensions.width - 260);
        const tooltipY = Math.min(sy + 16, dimensions.height - 160);
        tooltipPosRef.current = { x: tooltipX, y: tooltipY };
      }
    }
  }, [findNodeAt, onNodeHover, dimensions]);

  const handleMouseUp = useCallback(() => {
    if (dragRef.current) {
      const node = physNodesRef.current.find((n) => n.id === dragRef.current!.nodeId);
      if (node) {
        node.fixed = false;
        node.fx = 0;
        node.fy = 0;
      }
      dragRef.current = null;

      // 拖拽结束后保存布局到 localStorage
      if (wikiIdRef.current) {
        saveLayout(wikiIdRef.current, physNodesRef.current);
      }
    }
    panRef.current = null;
  }, []);

  const handleMouseLeave = useCallback(() => {
    hoverNodeRef.current = null;
    mouseScreenRef.current = { x: 0, y: 0, active: false };
    tooltipVisibleRef.current = false;
    setTooltipNodeIdState(null);
    onNodeHover?.(null);
    if (dragRef.current) {
      const node = physNodesRef.current.find((n) => n.id === dragRef.current!.nodeId);
      if (node) { node.fixed = false; }
      dragRef.current = null;
    }
    panRef.current = null;
  }, [onNodeHover]);

  const handleWheel = useCallback((e: React.WheelEvent<HTMLCanvasElement>) => {
    e.preventDefault();
    const rect = canvasRef.current!.getBoundingClientRect();
    const sx = e.clientX - rect.left;
    const sy = e.clientY - rect.top;

    const zoomFactor = e.deltaY < 0 ? 1.1 : 0.9;
    const oldZoom = cameraRef.current.zoom;
    const newZoom = Math.max(0.05, Math.min(5, oldZoom * zoomFactor));

    // 缩放以鼠标位置为中心
    const worldBefore = getScreenToWorld(sx, sy);
    cameraRef.current.zoom = newZoom;
    const worldAfter = getScreenToWorld(sx, sy);
    cameraRef.current.x += (worldAfter.x - worldBefore.x) * newZoom;
    cameraRef.current.y += (worldAfter.y - worldBefore.y) * newZoom;
  }, [dimensions]);

  const handleDoubleClick = useCallback((e: ReactMouseEvent<HTMLCanvasElement>) => {
    const rect = canvasRef.current!.getBoundingClientRect();
    const sx = e.clientX - rect.left;
    const sy = e.clientY - rect.top;
    const nodeId = findNodeAt(sx, sy);
    if (nodeId) {
      suppressAutoFocusRef.current = true;
      onNodeDoubleClick?.(nodeId);
    }
  }, [findNodeAt, onNodeDoubleClick]);

  const handleContextMenu = useCallback((e: ReactMouseEvent<HTMLCanvasElement>) => {
    e.preventDefault();
    const rect = canvasRef.current!.getBoundingClientRect();
    const sx = e.clientX - rect.left;
    const sy = e.clientY - rect.top;
    const nodeId = findNodeAt(sx, sy);
    if (nodeId) {
      suppressAutoFocusRef.current = true;
      onContextMenu?.(nodeId, { x: e.clientX, y: e.clientY });
    }
  }, [findNodeAt, onContextMenu]);

  // 原生非被动 wheel 监听：React 的 onWheel 为被动模式，preventDefault 无效
  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) { return; }
    const preventWheel = (e: WheelEvent) => {
      e.preventDefault();
    };
    canvas.addEventListener("wheel", preventWheel, { passive: false });
    return () => canvas.removeEventListener("wheel", preventWheel);
  }, []);

  // ── 触摸事件处理 ──
  const touchStateRef = useRef<{
    lastDist?: number;
    startX?: number;
    startY?: number;
    camX?: number;
    camY?: number;
  }>({});

  const handleTouchStart = useCallback((e: React.TouchEvent<HTMLCanvasElement>) => {
    if (e.touches.length === 1) {
      const touch = e.touches[0];
      const rect = canvasRef.current!.getBoundingClientRect();
      const sx = touch.clientX - rect.left;
      const sy = touch.clientY - rect.top;
      const nodeId = findNodeAt(sx, sy);

      if (nodeId) {
        suppressAutoFocusRef.current = true;
        const node = physNodesRef.current.find((n) => n.id === nodeId);
        if (node) {
          node.fixed = true;
          dragRef.current = { nodeId };
          onNodeClick?.(nodeId);
        }
      } else {
        panRef.current = {
          startX: touch.clientX,
          startY: touch.clientY,
          camX: cameraRef.current.x,
          camY: cameraRef.current.y,
        };
        onDeselect?.();
      }

      // 记录触摸起始位置，用于长按检测
      touchStateRef.current.startX = touch.clientX;
      touchStateRef.current.startY = touch.clientY;
      touchStateRef.current.camX = cameraRef.current.x;
      touchStateRef.current.camY = cameraRef.current.y;
    } else if (e.touches.length === 2) {
      // 双指缩放
      const t1 = e.touches[0];
      const t2 = e.touches[1];
      const dx = t1.clientX - t2.clientX;
      const dy = t1.clientY - t2.clientY;
      touchStateRef.current.lastDist = Math.sqrt(dx * dx + dy * dy);
      dragRef.current = null;
      panRef.current = null;
    }
  }, [findNodeAt, onNodeClick, onDeselect]);

  const handleTouchMove = useCallback((e: React.TouchEvent<HTMLCanvasElement>) => {
    e.preventDefault();

    if (e.touches.length === 1) {
      const touch = e.touches[0];
      const rect = canvasRef.current!.getBoundingClientRect();
      const sx = touch.clientX - rect.left;
      const sy = touch.clientY - rect.top;

      mouseScreenRef.current = { x: sx, y: sy, active: true };

      if (dragRef.current) {
        const world = getScreenToWorld(sx, sy);
        const node = physNodesRef.current.find((n) => n.id === dragRef.current!.nodeId);
        if (node) {
          node.x = world.x;
          node.y = world.y;
          node.vx = 0;
          node.vy = 0;
        }
      } else if (panRef.current) {
        const dx = touch.clientX - panRef.current.startX;
        const dy = touch.clientY - panRef.current.startY;
        cameraRef.current.x = panRef.current.camX + dx;
        cameraRef.current.y = panRef.current.camY + dy;
      }
    } else if (e.touches.length === 2) {
      // 双指缩放
      const t1 = e.touches[0];
      const t2 = e.touches[1];
      const dx = t1.clientX - t2.clientX;
      const dy = t1.clientY - t2.clientY;
      const dist = Math.sqrt(dx * dx + dy * dy);

      if (touchStateRef.current.lastDist) {
        const scale = dist / touchStateRef.current.lastDist;
        const oldZoom = cameraRef.current.zoom;
        const newZoom = Math.max(0.05, Math.min(5, oldZoom * scale));

        const rect = canvasRef.current!.getBoundingClientRect();
        const centerX = (t1.clientX + t2.clientX) / 2 - rect.left;
        const centerY = (t1.clientY + t2.clientY) / 2 - rect.top;

        const worldBefore = getScreenToWorld(centerX, centerY);
        cameraRef.current.zoom = newZoom;
        const worldAfter = getScreenToWorld(centerX, centerY);
        cameraRef.current.x += (worldAfter.x - worldBefore.x) * newZoom;
        cameraRef.current.y += (worldAfter.y - worldBefore.y) * newZoom;
      }

      touchStateRef.current.lastDist = dist;
    }
  }, []);

  const handleTouchEnd = useCallback((e: React.TouchEvent<HTMLCanvasElement>) => {
    if (dragRef.current) {
      const node = physNodesRef.current.find((n) => n.id === dragRef.current!.nodeId);
      if (node) {
        node.fixed = false;
        node.fx = 0;
        node.fy = 0;
      }
      dragRef.current = null;

      // 拖拽结束后保存布局
      if (wikiIdRef.current) {
        saveLayout(wikiIdRef.current, physNodesRef.current);
      }
    }
    panRef.current = null;
    touchStateRef.current.lastDist = undefined;

    // 触摸结束后检测是否为点击（移动距离小于阈值）
    if (e.changedTouches.length === 1 && touchStateRef.current.startX !== undefined) {
      const touch = e.changedTouches[0];
      const movedX = Math.abs(touch.clientX - touchStateRef.current.startX);
      const movedY = Math.abs(touch.clientY - (touchStateRef.current.startY ?? 0));
      if (movedX < 5 && movedY < 5) {
        // 这是一次点击，已在 touchstart 中处理
      }
    }
  }, []);

  // 键盘导航 + 删除（带确认）
  const pendingDeleteRef = useRef<string | null>(null);
  useEffect(() => {
    const handleKey = (e: KeyboardEvent) => {
      const target = e.target as HTMLElement;
      const isInputFocused = target.tagName === "INPUT" || target.tagName === "TEXTAREA" || target.isContentEditable;

      // 空格键：聚焦画布
      if (e.key === " " && !isInputFocused) {
        e.preventDefault();
        containerRef.current?.focus();
      }

      if (e.key === "Escape") {
        pendingDeleteRef.current = null;
        onDeselect?.();
      }

      // 方向键平移视图
      if (!isInputFocused) {
        const panStep = 50 / cameraRef.current.zoom;
        const panSpeed = e.shiftKey ? panStep * 2 : panStep;

        switch (e.key) {
          case "ArrowUp":
            e.preventDefault();
            cameraRef.current.y += panSpeed;
            break;
          case "ArrowDown":
            e.preventDefault();
            cameraRef.current.y -= panSpeed;
            break;
          case "ArrowLeft":
            e.preventDefault();
            cameraRef.current.x += panSpeed;
            break;
          case "ArrowRight":
            e.preventDefault();
            cameraRef.current.x -= panSpeed;
            break;
          case "+":
          case "=":
            e.preventDefault();
            cameraRef.current.zoom = Math.min(5, cameraRef.current.zoom * 1.2);
            break;
          case "-":
          case "_":
            e.preventDefault();
            cameraRef.current.zoom = Math.max(0.05, cameraRef.current.zoom / 1.2);
            break;
          case "0":
            e.preventDefault();
            cameraRef.current.zoom = 1;
            cameraRef.current.x = 0;
            cameraRef.current.y = 0;
            break;
          case "f":
          case "F":
            // 聚焦选中节点
            if (selectedNodeIdRef.current) {
              const node = physNodesRef.current.find((n) => n.id === selectedNodeIdRef.current);
              if (node) {
                const targetZoom = Math.max(cameraRef.current.zoom, 1.5);
                cameraRef.current.x = -node.x * targetZoom;
                cameraRef.current.y = -node.y * targetZoom;
                cameraRef.current.zoom = targetZoom;
              }
            }
            break;
          case "h":
          case "H":
            // 切换鱼眼模式
            fisheyeEnabledRef.current = !fisheyeEnabledRef.current;
            break;
          case "l":
          case "L":
            // 切换聚类模式
            clusterModeRef.current = !clusterModeRef.current;
            break;
        }
      }

      // Delete/Backspace 删除（需二次确认）
      if ((e.key === "Delete" || e.key === "Backspace") && selectedNodeIdRef.current && !isInputFocused) {
        const nodeId = selectedNodeIdRef.current;
        if (pendingDeleteRef.current === nodeId) {
          pendingDeleteRef.current = null;
          onDeleteNode?.(nodeId);
        } else {
          pendingDeleteRef.current = nodeId;
          setTimeout(() => {
            if (pendingDeleteRef.current === nodeId) {
              pendingDeleteRef.current = null;
            }
          }, 1500);
        }
      }
    };
    const el = containerRef.current;
    el?.addEventListener("keydown", handleKey);
    return () => el?.removeEventListener("keydown", handleKey);
  }, [onDeleteNode, onDeselect]);

  useEffect(() => {
    const handle = () => {
      minimapDragRef.current = false;
    };
    window.addEventListener("mouseup", handle);
    return () => window.removeEventListener("mouseup", handle);
  }, []);

  // ── 工具栏操作 ──

  const handleZoomIn = useCallback(() => {
    cameraRef.current.zoom = Math.min(5, cameraRef.current.zoom * 1.2);
  }, []);
  const handleZoomOut = useCallback(() => {
    cameraRef.current.zoom = Math.max(0.05, cameraRef.current.zoom / 1.2);
  }, []);
  const handleFitAll = useCallback(() => {
    const nodes = physNodesRef.current;
    if (nodes.length === 0) { return; }
    let minX = Infinity, minY = Infinity, maxX = -Infinity, maxY = -Infinity;
    for (const n of nodes) {
      if (n.x < minX) { minX = n.x; }
      if (n.y < minY) { minY = n.y; }
      if (n.x > maxX) { maxX = n.x; }
      if (n.y > maxY) { maxY = n.y; }
    }
    const bboxW = maxX - minX;
    const bboxH = maxY - minY;
    const targetZoom = Math.min(
      (dimensions.width * 0.8) / Math.max(bboxW, 1),
      (dimensions.height * 0.8) / Math.max(bboxH, 1),
      2,
    );
    cameraRef.current.x = -(minX + maxX) / 2 * targetZoom;
    cameraRef.current.y = -(minY + maxY) / 2 * targetZoom;
    cameraRef.current.zoom = targetZoom;
  }, [dimensions]);
  const handleFullscreenToggle = useCallback(() => {
    if (isFullscreen) {
      document.exitFullscreen();
    } else {
      containerRef.current?.requestFullscreen();
    }
  }, [isFullscreen]);
  const handleExportPNG = useCallback(() => {
    const canvas = canvasRef.current;
    if (!canvas) { return; }
    const link = document.createElement("a");
    link.download = `wiki-graph-${Date.now()}.png`;
    link.href = canvas.toDataURL("image/png");
    link.click();
  }, []);

  const handleExportHD = useCallback(() => {
    const canvas = canvasRef.current;
    if (!canvas) { return; }
    // 高清导出：2x 分辨率
    const scale = 2;
    const hdCanvas = document.createElement("canvas");
    hdCanvas.width = canvas.width * scale;
    hdCanvas.height = canvas.height * scale;
    const ctx = hdCanvas.getContext("2d");
    if (!ctx) { return; }
    ctx.scale(scale, scale);
    ctx.drawImage(canvas, 0, 0);
    const link = document.createElement("a");
    link.download = `wiki-graph-hd-${Date.now()}.png`;
    link.href = hdCanvas.toDataURL("image/png");
    link.click();
  }, []);

  const handleExportSVG = useCallback(() => {
    const nodes = physNodesRef.current;
    const edges = physEdgesRef.current;
    const nodeMeta = nodeMetaRef.current;
    const colorCache = nodeColorRef.current;
    const sizeCache = nodeSizeRef.current;
    const visibleTypes = visibleEdgeTypesRef.current;

    if (nodes.length === 0) { return; }

    // 计算边界框
    let minX = Infinity, minY = Infinity, maxX = -Infinity, maxY = -Infinity;
    for (const n of nodes) {
      if (n.x < minX) { minX = n.x; }
      if (n.y < minY) { minY = n.y; }
      if (n.x > maxX) { maxX = n.x; }
      if (n.y > maxY) { maxY = n.y; }
    }

    const padding = 50;
    const viewBoxW = maxX - minX + padding * 2;
    const viewBoxH = maxY - minY + padding * 2;
    const offsetX = -minX + padding;
    const offsetY = -minY + padding;

    const svgParts: string[] = [];
    svgParts.push(`<?xml version="1.0" encoding="UTF-8"?>`);
    svgParts.push(
      `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${viewBoxW} ${viewBoxH}" width="${viewBoxW}" height="${viewBoxH}">`,
    );
    svgParts.push(`<rect width="100%" height="100%" fill="${token.colorBgContainer}"/>`);

    // 绘制边
    for (let i = 0; i < edges.length; i++) {
      const em = edgeMetaRef.current[i];
      if (!em || !visibleTypes.has(em.type)) { continue; }
      const s = nodes[em.sourceIdx];
      const t = nodes[em.targetIdx];
      if (!s || !t) { continue; }
      const x1 = s.x + offsetX;
      const y1 = s.y + offsetY;
      const x2 = t.x + offsetX;
      const y2 = t.y + offsetY;
      svgParts.push(
        `<line x1="${x1}" y1="${y1}" x2="${x2}" y2="${y2}" stroke="${em.color}" stroke-width="${em.width}" opacity="0.7"/>`,
      );
    }

    // 绘制节点
    for (const node of nodes) {
      const meta = nodeMeta.get(node.id);
      if (!meta) { continue; }
      const color = colorCache.get(node.id) || token.colorPrimary;
      const size = sizeCache.get(node.id) || 6;
      const cx = node.x + offsetX;
      const cy = node.y + offsetY;
      svgParts.push(`<circle cx="${cx}" cy="${cy}" r="${size}" fill="${color}" opacity="0.9"/>`);
      // 标签
      const label = meta.title.length > 20 ? meta.title.slice(0, 18) + "…" : meta.title;
      svgParts.push(
        `<text x="${cx}" y="${
          cy + size + 12
        }" text-anchor="middle" font-size="10" fill="${token.colorText}" font-family="Inter, system-ui, sans-serif">${label}</text>`,
      );
    }

    svgParts.push(`</svg>`);

    const svgBlob = new Blob([svgParts.join("\n")], { type: "image/svg+xml" });
    const url = URL.createObjectURL(svgBlob);
    const link = document.createElement("a");
    link.download = `wiki-graph-${Date.now()}.svg`;
    link.href = url;
    link.click();
    URL.revokeObjectURL(url);
  }, [token]);
  const handleRelaunchLayout = useCallback(() => {
    const nodes = physNodesRef.current;

    // 清除已保存的布局缓存
    if (wikiIdRef.current) {
      localStorage.removeItem(LAYOUT_STORAGE_PREFIX + wikiIdRef.current);
    }

    initializePositions(nodes, dimensions.width, dimensions.height);
    const enableClusters = clusterModeRef.current && communities;
    const config: PhysicsConfig = {
      theta: 0.5,
      repulsion: 6000,
      gravity: 0.01,
      damping: 0.92,
      dt: 0.25,
      springForce: 0.04,
      springDamping: 0.85,
      maxVelocity: 4,
      clusterForce: enableClusters ? 0.15 : undefined,
    };
    const centroids = enableClusters ? computeCommunityCentroids(nodes, communities!) : undefined;
    for (let i = 0; i < 60; i++) {
      stepPhysics(
        nodes,
        physEdgesRef.current,
        config,
        undefined,
        enableClusters ? communities : undefined,
        centroids,
      );
    }

    // 保存新布局
    if (wikiIdRef.current) {
      saveLayout(wikiIdRef.current, nodes);
    }

    const gridIndex = new Map<string, string[]>();
    for (const n of nodes) {
      const gx = Math.floor(n.x / GRID_CELL_SIZE);
      const gy = Math.floor(n.y / GRID_CELL_SIZE);
      const key = `${gx},${gy}`;
      const bucket = gridIndex.get(key);
      if (bucket) {
        bucket.push(n.id);
      } else {
        gridIndex.set(key, [n.id]);
      }
    }
    gridIndexRef.current = gridIndex;
  }, [dimensions, communities]);

  const focusOnNode = useCallback((nodeId: string) => {
    const node = physNodesRef.current.find((n) => n.id === nodeId);
    if (!node) { return; }

    const cam = cameraRef.current;
    const targetZoom = Math.max(cam.zoom, 1.5);
    const targetX = -node.x * targetZoom;
    const targetY = -node.y * targetZoom;

    const startX = cam.x;
    const startY = cam.y;
    const startZoom = cam.zoom;
    const duration = 400;
    const startTime = performance.now();

    const animate = (now: number) => {
      const elapsed = now - startTime;
      const t = Math.min(elapsed / duration, 1);
      const ease = t < 0.5 ? 2 * t * t : 1 - Math.pow(-2 * t + 2, 2) / 2;

      cam.x = startX + (targetX - startX) * ease;
      cam.y = startY + (targetY - startY) * ease;
      cam.zoom = startZoom + (targetZoom - startZoom) * ease;

      if (t < 1) {
        requestAnimationFrame(animate);
      }
    };
    requestAnimationFrame(animate);
  }, []);

  useImperativeHandle(ref, () => ({
    focusOnNode,
  }), [focusOnNode]);

  // ── 渲染 UI ──

  if (data.nodes.length === 0) {
    return (
      <Card
        style={{
          height: "100%",
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
          borderRadius: 12,
          background: `linear-gradient(135deg, ${token.colorBgContainer}08, ${token.colorBgContainer}15)`,
          border: `1px solid ${token.colorBorderSecondary}30`,
        }}
      >
        <Empty description={t("wiki.graph.empty")} />
      </Card>
    );
  }

  const ctrlBtnStyle: CSSProperties = {
    width: 26,
    height: 26,
    minWidth: 26,
    padding: 0,
    borderRadius: 7,
    background: `${token.colorBgContainer}e6`,
    backdropFilter: "blur(8px)",
    border: `1px solid ${token.colorBorderSecondary}30`,
    display: "flex",
    alignItems: "center",
    justifyContent: "center",
    transition: "all 0.15s ease",
  };

  const hoverBtnStyle = (e: ReactMouseEvent) => {
    const el = e.currentTarget as HTMLElement;
    el.style.background = token.colorBgTextHover;
    el.style.transform = "scale(1.05)";
  };
  const leaveBtnStyle = (e: ReactMouseEvent) => {
    const el = e.currentTarget as HTMLElement;
    el.style.background = `${token.colorBgContainer}e6`;
    el.style.transform = "scale(1)";
  };

  const nodeCount = data.nodes.length;
  const edgeCount = data.edges.length;

  return (
    <div
      ref={containerRef}
      tabIndex={0}
      className="outline-none focus-visible:outline-2 focus-visible:outline-offset-2"
      style={{ width: "100%", height: "100%", position: "relative" }}
    >
      <canvas
        ref={canvasRef}
        style={{
          display: "block",
          width: "100%",
          height: "100%",
          cursor: dragRef.current ? "grabbing" : "grab",
          touchAction: "none",
        }}
        onMouseDown={handleMouseDown}
        onMouseMove={handleMouseMove}
        onMouseUp={handleMouseUp}
        onMouseLeave={handleMouseLeave}
        onDoubleClick={handleDoubleClick}
        onContextMenu={handleContextMenu}
        onWheel={handleWheel}
        onTouchStart={handleTouchStart}
        onTouchMove={handleTouchMove}
        onTouchEnd={handleTouchEnd}
      />

      {/* 左上角：筛选 */}
      <div style={{ position: "absolute", top: 10, left: 10, zIndex: 10 }}>
        <Popover
          open={legendOpen}
          onOpenChange={setLegendOpen}
          trigger="click"
          placement="bottomLeft"
          arrow={false}
          styles={{ root: { width: 280 }, container: { padding: "12px 14px" } }}
          content={
            <div style={{ display: "flex", flexDirection: "column", gap: 8, fontSize: 11 }}>
              <div style={{ color: token.colorTextSecondary, fontSize: 11, marginBottom: 4 }}>
                {t("wiki.graph.edgeTypes")}
              </div>
              <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
                {(Object.keys(edgeTypeLabels) as GraphEdgeType[]).map((et) => {
                  const isVisible = visibleEdgeTypes.has(et);
                  const style = getEdgeTypeStylesMap(token)[et];
                  return (
                    <button
                      key={et}
                      onClick={() => toggleEdgeType(et)}
                      style={{
                        display: "flex",
                        alignItems: "center",
                        gap: 4,
                        padding: "2px 6px",
                        borderRadius: 4,
                        border: `1px solid ${isVisible ? style.color : token.colorBorderSecondary}`,
                        background: isVisible ? `${style.color}15` : "transparent",
                        cursor: "pointer",
                        opacity: isVisible ? 1 : 0.5,
                        transition: "opacity 0.15s",
                        fontSize: 11,
                        color: isVisible ? style.color : token.colorTextSecondary,
                      }}
                    >
                      <svg width="20" height="8">
                        <line
                          x1="0"
                          y1="4"
                          x2="20"
                          y2="4"
                          stroke={isVisible ? style.color : token.colorBorderSecondary}
                          strokeWidth={style.width}
                        />
                      </svg>
                      <span>{t(edgeTypeLabels[et])}</span>
                    </button>
                  );
                })}
              </div>
              {/* 社区筛选 */}
              {communities && communities.size > 0 && (
                <>
                  <div style={{ color: token.colorTextSecondary, fontSize: 11, marginBottom: 4, marginTop: 8 }}>
                    {t("wiki.graph.communities")}
                  </div>
                  <div style={{ display: "flex", gap: 6, flexWrap: "wrap" }}>
                    {(() => {
                      const uniqueCids = new Set<number>();
                      for (const cid of communities.values()) {
                        uniqueCids.add(cid);
                      }
                      return Array.from(uniqueCids).map((cid) => {
                        const isVisible = visibleCommunities.has(cid);
                        const color = communityPalette[cid % communityPalette.length];
                        return (
                          <button
                            key={cid}
                            onClick={() => toggleCommunity(cid)}
                            style={{
                              display: "flex",
                              alignItems: "center",
                              gap: 4,
                              padding: "2px 6px",
                              borderRadius: 4,
                              border: `1px solid ${isVisible ? color : token.colorBorderSecondary}`,
                              background: isVisible ? `${color}15` : "transparent",
                              cursor: "pointer",
                              opacity: isVisible ? 1 : 0.5,
                              transition: "opacity 0.15s",
                              fontSize: 11,
                              color: isVisible ? color : token.colorTextSecondary,
                            }}
                          >
                            <span
                              style={{
                                width: 8,
                                height: 8,
                                borderRadius: "50%",
                                background: isVisible ? color : token.colorBorderSecondary,
                              }}
                            />
                            <span>{t("wiki.graph.clusterLabel", { id: cid })}</span>
                          </button>
                        );
                      });
                    })()}
                  </div>
                </>
              )}
            </div>
          }
        >
          <Button
            size="small"
            type="text"
            icon={<SlidersHorizontal size={13} />}
            style={ctrlBtnStyle}
            onMouseEnter={hoverBtnStyle}
            onMouseLeave={leaveBtnStyle}
            title={t("wiki.graph.legend")}
          />
        </Popover>
      </div>

      {/* 右上角：统计 */}
      <div style={{ position: "absolute", top: 10, right: 10, zIndex: 10 }}>
        <Popover
          open={statsOpen}
          onOpenChange={setStatsOpen}
          trigger="click"
          placement="bottomRight"
          arrow={false}
          styles={{ root: { width: 180 }, container: { padding: "10px 14px" } }}
          content={
            <div style={{ display: "flex", flexDirection: "column", gap: 4, fontSize: 12 }}>
              <Typography.Text type="secondary" style={{ fontSize: 11 }}>{t("wiki.graph.stats")}</Typography.Text>
              <span>{t("wiki.graph.nodes")}: {nodeCount}</span>
              <span>{t("wiki.graph.edges")}: {edgeCount}</span>
              <span>Zoom: {cameraRef.current.zoom.toFixed(2)}×</span>
            </div>
          }
        >
          <Button
            size="small"
            type="text"
            style={{ ...ctrlBtnStyle, fontSize: 10, fontWeight: 600, color: token.colorTextSecondary }}
            onMouseEnter={hoverBtnStyle}
            onMouseLeave={leaveBtnStyle}
            title={t("wiki.graph.stats")}
          >
            {nodeCount}
          </Button>
        </Popover>
      </div>

      {/* 底部中央：工具栏 */}
      <div
        style={{
          position: "absolute",
          bottom: 10,
          left: "50%",
          transform: "translateX(-50%)",
          zIndex: 10,
          display: "flex",
          alignItems: "center",
          gap: 2,
          padding: "3px 8px",
          borderRadius: 16,
          background: `${token.colorBgContainer}f0`,
          backdropFilter: "blur(16px)",
          border: `1px solid ${token.colorBorderSecondary}30`,
          boxShadow: `0 2px 8px ${token.colorBgMask}20`,
        }}
      >
        <Tooltip title={t("wiki.graph.zoomIn")}>
          <button
            onClick={handleZoomIn}
            style={{ ...ctrlBtnStyle, width: 24, height: 24, minWidth: 24, background: "transparent", border: "none" }}
            onMouseEnter={hoverBtnStyle}
            onMouseLeave={leaveBtnStyle}
          >
            <ZoomIn size={14} />
          </button>
        </Tooltip>
        <Tooltip title={t("wiki.graph.zoomOut")}>
          <button
            onClick={handleZoomOut}
            style={{ ...ctrlBtnStyle, width: 24, height: 24, minWidth: 24, background: "transparent", border: "none" }}
            onMouseEnter={hoverBtnStyle}
            onMouseLeave={leaveBtnStyle}
          >
            <ZoomOut size={14} />
          </button>
        </Tooltip>
        <Tooltip title={t("wiki.graph.fitView")}>
          <button
            onClick={handleFitAll}
            style={{ ...ctrlBtnStyle, width: 24, height: 24, minWidth: 24, background: "transparent", border: "none" }}
            onMouseEnter={hoverBtnStyle}
            onMouseLeave={leaveBtnStyle}
          >
            <Maximize2 size={14} />
          </button>
        </Tooltip>
        <div style={{ width: 1, height: 14, background: token.colorBorderSecondary, margin: "0 2px" }} />
        {/* 鱼眼放大镜 toggle */}
        <Tooltip title={fisheyeEnabled ? t("wiki.graph.fisheyeOn") : t("wiki.graph.fisheyeOff")}>
          <button
            onClick={() => setFisheyeEnabled((v) => !v)}
            style={{
              ...ctrlBtnStyle,
              width: 24,
              height: 24,
              minWidth: 24,
              background: fisheyeEnabled ? `${token.colorPrimary}20` : "transparent",
              border: "none",
              color: fisheyeEnabled ? token.colorPrimary : token.colorTextSecondary,
            }}
            onMouseEnter={hoverBtnStyle}
            onMouseLeave={leaveBtnStyle}
          >
            <Eye size={14} />
          </button>
        </Tooltip>
        {/* 聚类模式 toggle */}
        <Tooltip title={clusterMode ? t("wiki.graph.clusterOff") : t("wiki.graph.clusterOn")}>
          <button
            onClick={() => setClusterMode((v) => !v)}
            style={{
              ...ctrlBtnStyle,
              width: 24,
              height: 24,
              minWidth: 24,
              background: clusterMode ? `${token.colorPrimary}20` : "transparent",
              border: "none",
              color: clusterMode ? token.colorPrimary : token.colorTextSecondary,
              fontSize: 11,
              fontWeight: 700,
            }}
            onMouseEnter={hoverBtnStyle}
            onMouseLeave={leaveBtnStyle}
          >
            ◈
          </button>
        </Tooltip>
        <div style={{ width: 1, height: 14, background: token.colorBorderSecondary, margin: "0 2px" }} />
        <Tooltip title={t("wiki.graph.fullscreen")}>
          <button
            onClick={handleFullscreenToggle}
            style={{ ...ctrlBtnStyle, width: 24, height: 24, minWidth: 24, background: "transparent", border: "none" }}
            onMouseEnter={hoverBtnStyle}
            onMouseLeave={leaveBtnStyle}
          >
            <Fullscreen size={14} />
          </button>
        </Tooltip>
        <div style={{ width: 1, height: 14, background: token.colorBorderSecondary, margin: "0 2px" }} />
        {/* 导出下拉菜单 */}
        <Popover
          trigger="click"
          placement="top"
          arrow={false}
          styles={{ root: { width: 140 }, container: { padding: "4px" } }}
          content={
            <div style={{ display: "flex", flexDirection: "column", gap: 2 }}>
              <button
                onClick={handleExportPNG}
                style={{
                  padding: "6px 12px",
                  background: "transparent",
                  border: "none",
                  borderRadius: 4,
                  cursor: "pointer",
                  fontSize: 12,
                  color: token.colorText,
                  textAlign: "left",
                }}
                onMouseEnter={(e) => {
                  e.currentTarget.style.background = token.colorPrimaryBg;
                }}
                onMouseLeave={(e) => {
                  e.currentTarget.style.background = "transparent";
                }}
              >
                {t("wiki.graph.exportPNG")}
              </button>
              <button
                onClick={handleExportHD}
                style={{
                  padding: "6px 12px",
                  background: "transparent",
                  border: "none",
                  borderRadius: 4,
                  cursor: "pointer",
                  fontSize: 12,
                  color: token.colorText,
                  textAlign: "left",
                }}
                onMouseEnter={(e) => {
                  e.currentTarget.style.background = token.colorPrimaryBg;
                }}
                onMouseLeave={(e) => {
                  e.currentTarget.style.background = "transparent";
                }}
              >
                {t("wiki.graph.exportHD")}
              </button>
              <button
                onClick={handleExportSVG}
                style={{
                  padding: "6px 12px",
                  background: "transparent",
                  border: "none",
                  borderRadius: 4,
                  cursor: "pointer",
                  fontSize: 12,
                  color: token.colorText,
                  textAlign: "left",
                }}
                onMouseEnter={(e) => {
                  e.currentTarget.style.background = token.colorPrimaryBg;
                }}
                onMouseLeave={(e) => {
                  e.currentTarget.style.background = "transparent";
                }}
              >
                {t("wiki.graph.exportSVG")}
              </button>
            </div>
          }
        >
          <Tooltip title={t("wiki.graph.exportPNG")}>
            <button
              style={{
                ...ctrlBtnStyle,
                width: 24,
                height: 24,
                minWidth: 24,
                background: "transparent",
                border: "none",
              }}
              onMouseEnter={hoverBtnStyle}
              onMouseLeave={leaveBtnStyle}
            >
              <Download size={14} />
            </button>
          </Tooltip>
        </Popover>
        <Tooltip title={t("wiki.graph.relayout")}>
          <button
            onClick={handleRelaunchLayout}
            style={{ ...ctrlBtnStyle, width: 24, height: 24, minWidth: 24, background: "transparent", border: "none" }}
            onMouseEnter={hoverBtnStyle}
            onMouseLeave={leaveBtnStyle}
          >
            <RefreshCw size={14} />
          </button>
        </Tooltip>
      </div>

      {/* Hover Tooltip — DOM ref 定位，内容用 React 渲染（仅节点变化时） */}
      <div
        ref={tooltipRef}
        style={{
          position: "absolute",
          zIndex: 20,
          pointerEvents: "none",
          maxWidth: 250,
          background: `${token.colorBgContainer}f5`,
          backdropFilter: "blur(12px)",
          border: `1px solid ${token.colorBorderSecondary}`,
          borderRadius: 10,
          padding: "10px 14px",
          boxShadow: `0 4px 16px ${token.colorBgMask}30`,
          transition: "left 0.06s ease-out, top 0.06s ease-out",
          display: "none",
        }}
      >
        {tooltipNodeIdState && (() => {
          const meta = nodeMetaRef.current.get(tooltipNodeIdState);
          if (!meta) { return null; }
          const nodeColor = nodeColorRef.current.get(tooltipNodeIdState) || token.colorPrimary;
          const communityId = communities?.get(tooltipNodeIdState);
          return (
            <>
              {/* 标题 */}
              <div style={{ display: "flex", alignItems: "center", gap: 6, marginBottom: 6 }}>
                <span
                  style={{
                    width: 8,
                    height: 8,
                    borderRadius: "50%",
                    background: nodeColor,
                    boxShadow: `0 0 6px ${nodeColor}80`,
                    flexShrink: 0,
                  }}
                />
                <span
                  style={{
                    fontWeight: 600,
                    fontSize: 13,
                    color: token.colorText,
                    overflow: "hidden",
                    textOverflow: "ellipsis",
                    whiteSpace: "nowrap",
                  }}
                >
                  {meta.title}
                </span>
              </div>

              {/* 类型 + 社区 */}
              <div style={{ display: "flex", gap: 6, flexWrap: "wrap", marginBottom: 6 }}>
                <span
                  style={{
                    fontSize: 10,
                    padding: "1px 6px",
                    borderRadius: 4,
                    background: `${nodeColor}20`,
                    color: nodeColor,
                    fontWeight: 500,
                  }}
                >
                  {t(`wiki.graph.nodeType.${meta.type}`)}
                </span>
                {communityId !== undefined && (
                  <span
                    style={{
                      fontSize: 10,
                      padding: "1px 6px",
                      borderRadius: 4,
                      background: `${communityPalette[communityId % communityPalette.length]}20`,
                      color: communityPalette[communityId % communityPalette.length],
                      fontWeight: 500,
                    }}
                  >
                    {t("wiki.graph.clusterLabel", { id: communityId })}
                  </span>
                )}
              </div>

              {/* 统计 */}
              <div style={{ display: "flex", gap: 10, fontSize: 11, color: token.colorTextSecondary, marginBottom: 6 }}>
                <span>→ {meta.linkCount}</span>
                <span>← {meta.backlinkCount}</span>
                <span>Σ {meta.linkCount + meta.backlinkCount}</span>
              </div>

              {/* 路径 */}
              <div
                style={{
                  fontSize: 10,
                  color: token.colorTextTertiary,
                  wordBreak: "break-all",
                  maxHeight: 32,
                  overflow: "hidden",
                }}
              >
                {meta.path}
              </div>

              {/* 标签 */}
              {meta.tags.length > 0 && (
                <div style={{ display: "flex", gap: 3, flexWrap: "wrap", marginTop: 6 }}>
                  {meta.tags.slice(0, 5).map((tag) => (
                    <span
                      key={tag}
                      style={{
                        fontSize: 9,
                        padding: "0 4px",
                        borderRadius: 3,
                        background: token.colorFillSecondary,
                        color: token.colorTextSecondary,
                      }}
                    >
                      #{tag}
                    </span>
                  ))}
                  {meta.tags.length > 5 && (
                    <span style={{ fontSize: 9, color: token.colorTextTertiary }}>+{meta.tags.length - 5}</span>
                  )}
                </div>
              )}
            </>
          );
        })()}
      </div>

      {showMinimap && (
        <div
          style={{
            position: "absolute",
            bottom: 50,
            right: 10,
            zIndex: 10,
            display: "flex",
            flexDirection: "column",
            alignItems: "flex-end",
            gap: 4,
          }}
        >
          <button
            onClick={() => setMinimapOpen((v) => !v)}
            style={{
              ...ctrlBtnStyle,
              width: 22,
              height: 22,
              minWidth: 22,
              fontSize: 10,
              fontWeight: 700,
              color: token.colorTextSecondary,
              cursor: "pointer",
            }}
            title={minimapOpen ? "Collapse minimap" : "Expand minimap"}
          >
            {minimapOpen ? "▾" : "▴"}
          </button>
          {minimapOpen && (
            <canvas
              ref={minimapRef}
              width={MINIMAP_W}
              height={MINIMAP_H}
              onMouseDown={handleMinimapMouseDown}
              onMouseMove={handleMinimapMouseMove}
              onMouseUp={handleMinimapMouseUp}
              style={{
                borderRadius: 8,
                background: `${token.colorBgContainer}f0`,
                backdropFilter: "blur(12px)",
                border: `1px solid ${token.colorBorderSecondary}`,
                boxShadow: `0 4px 16px ${token.colorBgMask}30`,
                cursor: minimapDragRef.current ? "grabbing" : "crosshair",
              }}
            />
          )}
        </div>
      )}
    </div>
  );
});

export const GraphView = memo(GraphViewInner);
export { GraphView as default };

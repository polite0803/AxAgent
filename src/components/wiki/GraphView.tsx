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
import {
  Download,
  Eye,
  Fullscreen,
  Maximize2,
  RefreshCw,
  SlidersHorizontal,
  Sparkles,
  ZoomIn,
  ZoomOut,
} from "lucide-react";
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
import type { WorkerMessage, WorkerResponse } from "./graphPhysics.worker";
import {
  applySavedLayout,
  buildNodeColorCache,
  clamp,
  clearLayout,
  communityPalette,
  edgeTypeLabels,
  escapeXml,
  getEdgeTypeStylesMap,
  getNodeSize,
  loadLayout,
  parseColor,
  saveLayout,
} from "./graphViewUtils";

// ── P7: 生产诊断日志收敛到单一 DEBUG_GRAPH 开关 ──
// 仅开发模式 + localStorage 显式开启时输出高频诊断日志，避免生产环境 DevTools 卡顿。
const DEBUG_GRAPH = (() => {
  if (!import.meta.env.DEV) { return false; }
  try {
    return localStorage.getItem("DEBUG_GRAPH") === "true";
  } catch {
    return false;
  }
})();

function debugLog(...args: unknown[]): void {
  if (DEBUG_GRAPH) {
    console.log(...args);
  }
}

// ── 预热物理配置：随 init 消息传给 Worker，在 Worker 内完成初始布局收敛 ──
// 之前在主线程同步执行 warmupPhysics，几万节点时冻结 UI 数秒；现移入 Worker。
const WARMUP_PHYSICS_CONFIG: PhysicsConfig = {
  theta: 0.6,
  repulsion: 30000,
  gravity: 0.002,
  damping: 0.85,
  dt: 0.4,
  springForce: 0.06,
  springDamping: 0.9,
  maxVelocity: 10,
};

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

const EMPTY_SET: ReadonlySet<string> = new Set<string>();

/**
 * requestIdleCallback 安全封装：macOS WKWebView（Safari 引擎）不支持该 API，
 * 裸调用会抛 ReferenceError 导致 LOD 更新 / 聚合几何刷新 / 位图缓存重建全部中断。
 * 不支持时降级为 setTimeout(0)（立即在下一事件循环执行）。
 */
function scheduleIdle(callback: () => void, timeout: number): void {
  if (typeof requestIdleCallback === "function") {
    requestIdleCallback(callback, { timeout });
  } else {
    setTimeout(callback, 0);
  }
}

// 布局持久化 / 配色 / 节点尺寸 / 颜色工具已抽至 ./graphViewUtils（F8 拆分）。

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

  // token 的实时引用：渲染循环/数据 effect 通过 ref 读取最新 token，
  // 主题切换无需重建物理世界，只需重算颜色缓存（见 token 主题 effect）。
  const tokenRef = useRef(token);
  tokenRef.current = token;

  // 原始图数据/社区引用：供主题 effect 重算颜色缓存（数据 effect 不再依赖 token）
  const dataRef = useRef<GraphData | null>(null);
  const rawCommunitiesRef = useRef<Map<string, number> | null>(null);

  const canvasRef = useRef<HTMLCanvasElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const rafRef = useRef<number>(0);

  // Worker 相关
  const workerRef = useRef<Worker | null>(null);
  const workerInitializedRef = useRef(false);
  const workerErrorCountRef = useRef(0); // Worker 连续错误计数，超阈值降级到主线程
  const workerResultRef = useRef<
    {
      positions: Float64Array;
      velocities: Float64Array;
      stable: boolean;
      tick: number;
    } | null
  >(null);
  const pendingStepRef = useRef(false);
  // 追踪已处理的 Worker tick：只有 Worker 返回新结果时才更新节点/重建网格，
  // 避免每帧都用旧结果重算 O(N) 网格索引（大图下每秒 60 次 × 20k 节点 = 灾难性）
  const lastProcessedTickRef = useRef(-1);
  // L2/L3 修复：收敛期重型重建（gridIndex / 聚类几何 / 位图缓存）限流计数器，
  // 按收到的 Worker result 步数计数
  const workerStepCounterRef = useRef(0);

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

  // 大图位图缓存：将所有节点/边预渲染到离屏 Canvas，每帧仅 drawImage 拷贝
  // 彻底消除每帧 5 万+ 矢量 Canvas 操作导致的主线程阻塞
  const spriteCacheRef = useRef<HTMLCanvasElement | null>(null);
  const spriteWorldBBoxRef = useRef({ minX: -5000, minY: -5000, maxX: 5000, maxY: 5000 });
  const FORCE_BITMAP_THRESHOLD = 3000; // 超过此节点数时强制使用位图模式

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
  const fisheyeEnabledRef = useRef(false);
  const clusterModeRef = useRef(false);
  // 自动 force cluster 标记：区分自动触发和用户手动开启的聚类模式
  // 自动触发时默认展开社区让用户看到真实节点；用户手动开启时保持全折叠
  const isAutoForceClusterRef = useRef(false);
  // 粒子流动默认开启（对齐 Obsidian 的动态美感；大规模节点自动降级）
  const particlesEnabledRef = useRef(true);
  // ── 社区聚合折叠 ──
  // 折叠的社区集合（聚类模式下默认全折叠；点击聚合节点展开/收起）
  const collapsedRef = useRef<Set<number>>(new Set());
  const hoverClusterRef = useRef<number | null>(null);
  // LOD 缩放阈值：渐进式展开，类似地图缩放细节
  const LOD_THRESHOLDS = {
    COLLAPSED: 0.5, // zoom < 0.5: 全折叠
    VIEWPORT: 1.0, // 0.5 <= zoom < 1: 视口内展开
    EXPANDED: 2.0, // 1 <= zoom < 2: 视口+邻近展开
    ALL: 4.0, // zoom >= 2: 全部展开
  };
  // 上次 LOD 级别，防抖用
  const lastLodLevelRef = useRef(0);
  // 手动展开的社区（用户点击展开的，不会因缩放折叠回去）
  const manualExpandedRef = useRef<Set<number>>(new Set());
  // 每帧最多新增展开的社区数（防止一次性展开过多导致卡顿）
  const MAX_EXPAND_PER_FRAME = 5;
  // 聚合节点几何缓存：cid → { 质心, 半径, 计数, 代表名 }（低频刷新）
  const clusterGeomRef = useRef<
    Map<number, { cx: number; cy: number; r: number; count: number; label: string }>
  >(new Map());
  // ── 聚合物理（聚类折叠模式下物理只模拟聚合节点 + 未折叠节点，而非全部底层节点）──
  // 折叠社区的成员节点不参与物理（数量级骤降），聚合节点坐标驱动 clusterGeom。
  const aggPhysRef = useRef<
    {
      nodes: PhysicsNode[];
      edges: PhysicsEdge[];
      cidToNodeIdx: Map<number, number>;
      neighborMap: NeighborMap;
    } | null
  >(null);
  // 展开/收起状态变化时触发重渲染
  const [, setClusterCollapseVersion] = useState(0);
  const mouseScreenRef = useRef({ x: 0, y: 0, active: false });
  const communityCentroidsRef = useRef<Map<number, { cx: number; cy: number; count: number }>>(new Map());
  // 聚类气泡（drawClusterRegions）稳定态缓存（P9）：节点位置/折叠集合变化或每 30 帧才重建分组与渐变，
  // 图完全静止时直接复用缓存，避免每 5 帧全量 O(N) 分组 + 为每个社区新建 radialGradient。
  const clusterRegionCacheRef = useRef<{
    lastFrame: number;
    dirty: boolean;
    lastCollapsed: Set<number> | null;
    regions: Map<number, { cx: number; cy: number; rx: number; ry: number; grad: CanvasGradient }>;
  }>({ lastFrame: -9999, dirty: true, lastCollapsed: null, regions: new Map() });
  // communities prop 的 ref 镜像，供 useCallback / 事件回调读取最新值而无需将其加入依赖
  const communitiesRef = useRef<Map<string, number> | undefined>(undefined);
  useEffect(() => {
    // 优先使用哈希合并后的虚拟聚类映射
    communitiesRef.current = effectiveCommunitiesRef.current ?? communities;
  }, [communities]);

  const gridIndexRef = useRef<Map<string, string[]>>(new Map());
  const GRID_CELL_SIZE = 80;
  // minimap 包围盒缓存：系统稳定时复用，避免每 15 帧全量遍历计算
  const minimapBBoxRef = useRef<{ minX: number; minY: number; maxX: number; maxY: number } | null>(null);

  // ── 性能 LOD 阈值（万级节点保障） ──
  const GLOW_NODE_LIMIT = 2000; // 超过此节点数：普通节点不绘制 glow，仅交互节点保留
  const MINIMAP_REDRAW_INTERVAL = 15; // minimap 重绘间隔（帧），大图避免每帧全量遍历
  // 节点数超过此值且 communities 可用时，打开自动进入聚类折叠聚合视图，
  // 物理只模拟聚合节点（几十个），从根本上避免万级节点全量力导向收敛导致的卡死。
  const AUTO_CLUSTER_THRESHOLD = 3000;
  // 聚合物理规模上限：聚合节点 + 未折叠节点数超过此值时，放弃力导向（仅静态显示），
  // 防止社区粒度极细（甚至每节点一社区）时聚合物理规模仍达万级，主线程每帧 O(n log n) 卡死不响应。
  const MAX_AGG_PHYS_NODES = 800;
  // 强制聚类数量上限：当社区数超过此值时，通过哈希合并到虚拟聚类
  const FORCE_CLUSTER_COUNT = 200;
  // 主线程物理规模上限：超过此节点数时，fallback 主线程物理一律禁用（静态显示）。
  // fallback 是 Worker 未就绪时的兜底；若在大图上每帧跑全量 O(n log n) 力导向，
  // 主线程会被完全阻塞、鼠标键盘全部无响应。大图等待 Worker 就绪即可，绝不走主线程物理。
  const MAX_MAIN_THREAD_PHYSICS = 1500;

  // 有效社区映射（考虑哈希合并后的虚拟聚类）
  const effectiveCommunitiesRef = useRef<Map<string, number> | undefined>(undefined);

  // 统一的社区查找函数：所有代码路径必须使用这个，不能直接用 communities prop
  // 因为哈希合并后的虚拟聚类映射存在 effectiveCommunitiesRef 中
  const getCommunityId = useCallback((nodeId: string): number | undefined => {
    return effectiveCommunitiesRef.current?.get(nodeId);
  }, []);

  // 哈希字符串转整数（用于节点到虚拟聚类的稳定分桶）
  function hashStringToInt(str: string): number {
    let hash = 0;
    for (let i = 0; i < str.length; i++) {
      hash = ((hash << 5) - hash + str.charCodeAt(i)) | 0;
    }
    return hash;
  }

  const minimapRef = useRef<HTMLCanvasElement>(null);
  const [minimapOpen, setMinimapOpen] = useState(true);
  const minimapDragRef = useRef(false);

  // wikiId ref，用于布局持久化
  const wikiIdRef = useRef<string | undefined>(wikiId);
  wikiIdRef.current = wikiId;

  // 渲染缓存：posMap 和预计算的邻居集合，避免每帧重建 O(N)/O(E)
  const posMapRef = useRef<Map<string, PhysicsNode>>(new Map());
  // N6 修复：统计弹窗 Zoom 值的 DOM 引用，渲染循环中直接写 textContent 实时刷新
  const statsZoomTextRef = useRef<HTMLSpanElement | null>(null);
  const neighborsRef = useRef<Map<string, Set<string>>>(new Map());

  const [fisheyeEnabled, setFisheyeEnabled] = useState(false);
  const [clusterMode, setClusterMode] = useState(false);
  const [particlesEnabled, setParticlesEnabled] = useState(false);

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
  useEffect(() => {
    particlesEnabledRef.current = particlesEnabled;
  }, [particlesEnabled]);

  // 聚类模式切换：开启时默认全折叠（聚合视图），关闭时清空
  useEffect(() => {
    if (clusterMode) {
      // 自动 force cluster 模式下跳过全折叠初始化，保持展开状态让用户看到真实节点
      if (isAutoForceClusterRef.current) {
        return;
      }
      // 使用 effectiveCommunitiesRef（可能是哈希合并后的虚拟聚类）
      const ec = effectiveCommunitiesRef.current;
      if (ec) {
        const all = new Set<number>();
        for (const cid of ec.values()) {
          all.add(cid);
        }
        // 排除当前选中节点所在社区
        if (selectedNodeIdRef.current) {
          const selCid = getCommunityId(selectedNodeIdRef.current);
          if (selCid !== undefined) {
            all.delete(selCid);
          }
        }
        collapsedRef.current = all;
        refreshClusterGeom();
        buildAggregatePhysics();
        setClusterCollapseVersion((v) => v + 1);
      }
    } else {
      // 用户关闭聚类模式时清除自动 force cluster 标志
      isAutoForceClusterRef.current = false;
      collapsedRef.current = new Set();
      hoverClusterRef.current = null;
      aggPhysRef.current = null;
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [clusterMode, communities]);

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

  // 选中/导航到折叠社区内的节点时：自动展开该社区，确保目标可见
  useEffect(() => {
    if (!selectedNodeId || !clusterModeRef.current || !effectiveCommunitiesRef.current) {
      return;
    }
    const cid = getCommunityId(selectedNodeId);
    if (cid !== undefined && collapsedRef.current.has(cid)) {
      const next = new Set(collapsedRef.current);
      next.delete(cid);
      collapsedRef.current = next;
      // 标记为手动展开，防止 LOD 自动折叠
      const manualNext = new Set(manualExpandedRef.current);
      manualNext.add(cid);
      manualExpandedRef.current = manualNext;
      refreshClusterGeom();
      buildAggregatePhysics();
      setClusterCollapseVersion((v) => v + 1);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [selectedNodeId, communities]);

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
      // 平滑移动相机到节点位置（400ms 缓动，避免相机突变割裂感）
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
    });
  }, [selectedNodeId]);

  // 容器尺寸监听（rAF 去抖，避免拖动窗口时高频触发渲染重建）
  useEffect(() => {
    const el = containerRef.current;
    if (!el) { return; }
    let rafId = 0;
    const update = () => setDimensions({ width: el.clientWidth, height: el.clientHeight });
    update();
    const scheduleUpdate = () => {
      if (rafId) { return; }
      rafId = requestAnimationFrame(() => {
        rafId = 0;
        setDimensions({ width: el.clientWidth, height: el.clientHeight });
      });
    };
    const ro = new ResizeObserver(scheduleUpdate);
    ro.observe(el);
    return () => {
      if (rafId) { cancelAnimationFrame(rafId); }
      ro.disconnect();
    };
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

    // 供主题 effect 重算颜色缓存（数据 effect 不依赖 token，主题切换不重建图）
    dataRef.current = data;
    rawCommunitiesRef.current = communities ?? null;

    // 清空 minimap 包围盒缓存（节点集已变化，旧缓存失效）
    minimapBBoxRef.current = null;

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
    let layoutApplied = false;
    if (wikiId) {
      const saved = loadLayout(wikiId);
      if (saved) {
        layoutApplied = applySavedLayout(pNodes, saved);
        // D7: 仅当布局成功恢复时才恢复相机视角（布局不匹配时视角会偏移）
        if (layoutApplied && saved.camera) {
          cameraRef.current.x = saved.camera.x;
          cameraRef.current.y = saved.camera.y;
          cameraRef.current.zoom = saved.camera.zoom;
        }
      }
    }

    // 若无已保存布局或匹配率太低（applySavedLayout 返回 false），则使用圆形布局
    if (!layoutApplied) {
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

    // ── 预热迭代：移交给 Worker 在 init 时后台执行 ──
    // 此前在主线程同步跑 40~80 次 Barnes-Hut，几万节点首开会冻结 UI 数秒（
    // 见下方"不在主线程同步跑 stepPhysics"的说明）。现在只计算预热参数，
    // 随 Worker init 消息传入，由 Worker 完成初始布局收敛，主线程保持响应。
    // 预热迭代数降为 20~30 次（Worker 单步更快），保证 ready 快速返回，
    // 剩余收敛由渲染循环的持续 STEP 完成。
    let warmupIters = 0;
    if (!layoutApplied) {
      warmupIters = pNodes.length > 5000 ? 20 : 30;
    }

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
    const edgeStyles = getEdgeTypeStylesMap(tokenRef.current);
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

    // 粒子系统（动态上限：大图场景自动减少粒子数）
    const particleNodeCount = pNodes.length;
    const maxParticles = particleNodeCount > 10000 ? 300 : particleNodeCount > 5000 ? 1000 : 4000;
    const particles: Particle[] = [];
    for (let i = 0; i < data.edges.length; i++) {
      if (particles.length >= maxParticles) { break; }
      const em = edgeMetaRef.current[i];
      if (em.animated) {
        // 每条动画边 1-2 个粒子
        const count = em.type === "reference" ? 2 : 1;
        for (let j = 0; j < count; j++) {
          if (particles.length >= maxParticles) { break; }
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

    // 初始布局收敛交由 Worker 完成（见下文 Worker init + 渲染循环持续 STEP）。
    // 不在主线程同步跑 stepPhysics：几万节点时 Barnes-Hut 单步即数百 ms，
    // 主线程同步迭代会冻结 UI 数秒。Worker 就绪前节点保持 initial/保存布局即可。

    // ── 预计算有效社区映射（在 Worker 初始化之前执行） ──
    // 大图（>3000节点）必须进入聚类模式，无论社区粒度如何。
    // 社区数过多时（>MAX_AGG_PHYS_NODES），通过哈希合并到 FORCE_CLUSTER_COUNT 个虚拟聚类。
    let effectiveCommunities: Map<string, number> | undefined = communities;
    const shouldForceCluster = pNodes.length > AUTO_CLUSTER_THRESHOLD;
    if (shouldForceCluster) {
      if (!effectiveCommunities || effectiveCommunities.size > MAX_AGG_PHYS_NODES) {
        // 哈希合并：将所有节点均匀分配到 FORCE_CLUSTER_COUNT 个虚拟聚类
        const hashMap = new Map<string, number>();
        if (effectiveCommunities) {
          for (const [nodeId] of effectiveCommunities) {
            const hash = Math.abs(hashStringToInt(nodeId)) % FORCE_CLUSTER_COUNT;
            hashMap.set(nodeId, hash);
          }
        } else {
          // 没有社区数据时，直接对所有节点哈希分桶
          for (const n of pNodes) {
            const hash = Math.abs(hashStringToInt(n.id)) % FORCE_CLUSTER_COUNT;
            hashMap.set(n.id, hash);
          }
        }
        effectiveCommunities = hashMap;
      }
      // 关键修复：在 forceCluster 模式下，确保所有节点都被映射到社区。
      // 即使原始 communities 数据已经存在，也可能只覆盖了部分节点。
      // 补全缺失节点的社区分配，确保 buildAggregatePhysics 能正确处理所有边。
      if (effectiveCommunities) {
        let hasMissingNodes = false;
        for (const n of pNodes) {
          if (!effectiveCommunities.has(n.id)) {
            hasMissingNodes = true;
            break;
          }
        }
        if (hasMissingNodes) {
          // 补全缺失节点：使用哈希分配到现有或新的社区
          const updatedMap = new Map<string, number>(effectiveCommunities);
          for (const n of pNodes) {
            if (!updatedMap.has(n.id)) {
              const hash = Math.abs(hashStringToInt(n.id)) % FORCE_CLUSTER_COUNT;
              updatedMap.set(n.id, hash);
            }
          }
          effectiveCommunities = updatedMap;
        }
      }
      // 关键：更新 effectiveCommunitiesRef，供 Worker 初始化和后续代码使用
      effectiveCommunitiesRef.current = effectiveCommunities;
    } else {
      // 小图直接使用原始 communities
      effectiveCommunitiesRef.current = effectiveCommunities;
    }

    // ── 节点颜色缓存 ──
    // N5 修复：颜色缓存构建必须放在 effectiveCommunities 计算之后——
    // force-cluster 哈希合并模式下按"虚拟聚类 ID"染色，与聚合彩球/气泡
    // （communityPalette[cid]）取色一致，否则展开社区后内部节点颜色与彩球不对应。
    // 普通模式下 effectiveCommunitiesRef.current 即原始 communities，行为不变。
    const colorCommunities = effectiveCommunitiesRef.current ?? communities;
    nodeColorRef.current = buildNodeColorCache(data.nodes, colorCommunities, tokenRef.current);
    buildNodeSpriteCache();

    // ── 初始化物理 Worker ──
    // 销毁旧 Worker
    if (workerRef.current) {
      workerRef.current.postMessage({ type: "destroy" } as WorkerMessage);
      workerRef.current.terminate();
      workerRef.current = null;
      workerInitializedRef.current = false;
      lastProcessedTickRef.current = -1;
    }

    const worker = new Worker(
      new URL("./graphPhysics.worker.ts", import.meta.url),
      { type: "module" },
    );
    workerRef.current = worker;

    // ── 零拷贝初始化：使用 Float64Array + Transfer List ──
    const workerConfig: PhysicsConfig = {
      theta: 0.5,
      repulsion: 18000,
      gravity: 0.003,
      damping: 0.82,
      dt: 0.35,
      springForce: 0.08,
      springDamping: 0.85,
      maxVelocity: 8,
    };

    // ── 零拷贝初始化：使用 Float64Array + Int32Array + Transfer List ──
    // 彻底消除字符串数组的 structured clone 开销（2万+ 字符串序列化阻塞主线程数秒）
    // 节点布局：[x, y, vx, vy, fx, fy, mass, fixed(0/1), kind(enum), idx] = 10 floats
    // 边布局：[sIdx, tIdx, restLength, stiffness, damping] = 5 floats
    const nodeCount = pNodes.length;
    const edgeCount = pEdges.length;
    const NODE_STRIDE = 10;
    const EDGE_STRIDE = 5;
    const nodeBuffer = new Float64Array(nodeCount * NODE_STRIDE);
    const edgeBuffer = new Float64Array(edgeCount * EDGE_STRIDE);
    // 节点类型枚举映射（用 Uint8Array 传输，避免字符串序列化）
    const kindToEnum = new Map<string, number>();
    const nodeKindEnum = new Uint8Array(nodeCount);
    // 直接构建 节点索引 → 社区ID 映射（Int32Array，零拷贝传输）
    // 避免 Worker 中用 nodeIds 反查 communities 的二次构建开销
    const nodeIdxToCommunity = new Int32Array(nodeCount).fill(-1);
    const ecLookup = effectiveCommunities;
    for (let i = 0; i < nodeCount; i++) {
      const n = pNodes[i];
      const base = i * NODE_STRIDE;
      nodeBuffer[base] = n.x;
      nodeBuffer[base + 1] = n.y;
      nodeBuffer[base + 2] = n.vx;
      nodeBuffer[base + 3] = n.vy;
      nodeBuffer[base + 4] = n.fx;
      nodeBuffer[base + 5] = n.fy;
      nodeBuffer[base + 6] = n.mass;
      nodeBuffer[base + 7] = n.fixed ? 1 : 0;
      // kind 枚举化
      let kindVal = kindToEnum.get(n.kind);
      if (kindVal === undefined) {
        kindVal = kindToEnum.size;
        kindToEnum.set(n.kind, kindVal);
      }
      nodeKindEnum[i] = kindVal;
      nodeBuffer[base + 8] = kindVal;
      nodeBuffer[base + 9] = n.idx;
      // 社区映射（直接用节点 ID 查找）
      if (ecLookup) {
        const cid = ecLookup.get(n.id);
        if (cid !== undefined) {
          nodeIdxToCommunity[i] = cid;
        }
      }
    }

    for (let e = 0; e < edgeCount; e++) {
      const edge = pEdges[e];
      const eBase = e * EDGE_STRIDE;
      edgeBuffer[eBase] = edge.sourceIdx;
      edgeBuffer[eBase + 1] = edge.targetIdx;
      edgeBuffer[eBase + 2] = edge.restLength;
      edgeBuffer[eBase + 3] = edge.stiffness;
      edgeBuffer[eBase + 4] = edge.damping;
    }

    const initMsg: WorkerMessage = {
      type: "init",
      payload: {
        nodes: [],
        edges: [],
        config: workerConfig,
        communities: undefined, // 已通过 nodeIdxToCommunity 传递，不再需要
        // 预热参数：Worker init 时在后台完成初始布局收敛（避免主线程同步冻结）
        warmupIterations: warmupIters,
        warmupConfig: warmupIters > 0 ? WARMUP_PHYSICS_CONFIG : undefined,
        compact: {
          nodeBuffer,
          edgeBuffer,
          nodeIdxToCommunity,
          nodeKindEnum,
          nodeCount,
          edgeCount,
        },
      },
    };

    // 使用 Transfer List 实现零拷贝：所有 ArrayBuffer 所有权直接转移到 Worker
    // 彻底消除 structured clone 开销（之前 2万+ 字符串序列化阻塞主线程数秒）
    worker.postMessage(initMsg, [
      nodeBuffer.buffer,
      edgeBuffer.buffer,
      nodeIdxToCommunity.buffer,
      nodeKindEnum.buffer,
    ]);

    worker.onmessage = (e: MessageEvent<WorkerResponse>) => {
      const msg = e.data;
      if (msg.type === "ready") {
        workerInitializedRef.current = true;
        workerErrorCountRef.current = 0;
      } else if (msg.type === "result") {
        workerResultRef.current = {
          positions: msg.payload.positions,
          velocities: msg.payload.velocities,
          stable: msg.payload.stable,
          tick: msg.payload.tick,
        };
        pendingStepRef.current = false;
        workerErrorCountRef.current = 0;
      } else if (msg.type === "error") {
        console.error("[GraphWorker]", msg.message);
        pendingStepRef.current = false;
        workerErrorCountRef.current++;
        // 连续 3 次错误：terminate 并降级到主线程物理
        if (workerErrorCountRef.current >= 3 && workerRef.current === worker) {
          console.warn("[GraphWorker] persistent errors, falling back to main-thread physics");
          worker.terminate();
          workerRef.current = null;
          workerInitializedRef.current = false;
        }
      }
    };

    // ── 大图自动聚合：设置折叠状态（延迟计算放到 requestIdleCallback 或下一帧） ──
    if (shouldForceCluster) {
      const comm = effectiveCommunitiesRef.current;

      // 关键：同步更新 communitiesRef（buildAggregatePhysics 依赖它）
      communitiesRef.current = comm;

      // 自动 force cluster 模式：默认不折叠社区，让用户看到真实节点
      // 用户可通过 UI 手动切换聚类模式来折叠/展开
      isAutoForceClusterRef.current = true;
      collapsedRef.current = new Set();
      clusterModeRef.current = true;
      // 注意：不在此处同步调用 refreshClusterGeom/buildAggregatePhysics
      // 这两个函数在 2万+ 节点下是 O(N) + O(E)，会阻塞主线程数秒
      // 改为延迟到 Worker ready 后再计算（Worker ready 回调中处理）
      setClusterMode(true);

      // Worker ready 回调中处理聚合几何和物理构建
      const originalOnMessage = worker.onmessage.bind(worker);
      worker.onmessage = (e: MessageEvent<WorkerResponse>) => {
        originalOnMessage(e);
        if (e.data.type === "ready") {
          // 用 setTimeout(0) 而非 requestIdleCallback：
          // requestIdleCallback 在高负载下可能长时间不触发，导致 clusterGeom 始终为空
          // setTimeout(0) 会立即在下一个事件循环中执行，确保聚类数据尽快就绪
          setTimeout(() => {
            if (!clusterModeRef.current) { return; }
            refreshClusterGeom();
            buildAggregatePhysics();
            setClusterCollapseVersion((v) => v + 1);
          }, 0);
        }
      };
    }

    // 组件卸载时销毁 Worker，避免线程泄漏和内存堆积
    return () => {
      if (workerRef.current === worker) {
        worker.postMessage({ type: "destroy" } as WorkerMessage);
        worker.terminate();
        workerRef.current = null;
        workerInitializedRef.current = false;
      }
    };
    // 依赖不含 token：主题切换不再触发物理世界重建（布局保留），
    // 颜色更新由下方"主题 effect"单独处理。
  }, [data, communities]);

  // ── 主题 effect：token 变化时只重算颜色缓存，不重建物理世界 ──
  // 数据 effect 已不依赖 token；此处保证明暗主题切换后节点/边/粒子/背景颜色即时更新，
  // 同时保留当前布局与相机状态，避免此前"切换主题导致布局重置"的问题。
  useEffect(() => {
    const d = dataRef.current;
    if (!d || d.nodes.length === 0) { return; }

    // 节点颜色（社区色 palette 为常量，类型色随主题更新）
    // N5 修复：优先使用 effectiveCommunities（force-cluster 哈希合并后的虚拟聚类），
    // 与聚合彩球颜色保持一致；未启用聚类时即原始 communities
    nodeColorRef.current = buildNodeColorCache(
      d.nodes,
      effectiveCommunitiesRef.current ?? rawCommunitiesRef.current ?? undefined,
      token,
    );

    // 边样式颜色/宽度
    const edgeStyles = getEdgeTypeStylesMap(token);
    const meta = edgeMetaRef.current;
    if (meta) {
      for (const m of meta) {
        const style = edgeStyles[m.type] || edgeStyles.link;
        m.color = style.color;
        m.width = style.width;
        m.animated = style.animated;
      }
    }

    // 粒子颜色跟随边颜色
    const particles = particlesRef.current;
    if (particles) {
      for (const p of particles) {
        const em = meta[p.edgeIndex];
        if (em) { p.color = em.color; }
      }
    }

    // 重建节点精灵缓存 + 清空背景渐变缓存（颜色来自 token）
    buildNodeSpriteCache();
    bgCacheRef.current = null;
    minimapBBoxRef.current = null;
  }, [token]);

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

      // 绘制背景（纯色渐变，无网格点阵——对齐 Obsidian 的干净感）
      const grad = offCtx.createRadialGradient(w / 2, h / 2, 0, w / 2, h / 2, Math.max(w, h) * 0.7);
      grad.addColorStop(0, token.colorBgContainer);
      grad.addColorStop(1, token.colorBgElevated);
      offCtx.fillStyle = grad;
      offCtx.fillRect(0, 0, w, h);

      bgCacheRef.current = offscreen;
      bgCacheSizeRef.current = { w, h };
      return offscreen;
    };

    const render = () => {
      if (!running) { return; }

      const dpr = window.devicePixelRatio || 1;
      const w = dimensions.width;
      const h = dimensions.height;

      // ── 空闲跳帧：系统闲置超过 1 秒且无交互时，完全跳过 Canvas 绘制 ──
      // 节点位置由 Worker/物理模拟驱动，稳定后画面不变；跳帧避免每帧 O(N+E) 遍历
      // 大图（万级节点）下这是关键优化：将 60fps 全量渲染降为按需渲染
      if (idleCounterRef.current > 60) {
        const hasInteraction = mouseScreenRef.current.active || !!dragRef.current || !!panRef.current;
        if (!hasInteraction) {
          rafRef.current = requestAnimationFrame(render);
          return;
        }
      }

      // ── 绘制降频：空闲超过 0.5 秒时，每 2 帧才绘制一次 ──
      // 物理仍以 60fps 运行，但 Canvas 渲染降为 30fps
      const isIdleSlow = idleCounterRef.current > 30;
      const shouldRender = !isIdleSlow || frameCounterRef.current % 2 === 0;

      // ── Worker 未就绪时的大图保护：节点数 > 3000 且 Worker 未就绪时，
      // 跳过完整渲染（只保留上一帧画面），避免在主线程用 fallback 渲染 20k 节点。
      // Worker 初始化通常 < 500ms，此期间显示加载指示器即可
      const workerNotReadyLargeGraph = !workerInitializedRef.current && physNodesRef.current.length > 3000;

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

      // ── 计算有效社区映射（优先使用哈希合并后的虚拟聚类） ──
      const effCommunities = effectiveCommunitiesRef.current ?? communities;

      // ── Worker 物理步进 + 帧间插值 ──
      const worker = workerRef.current;
      const workerReady = workerInitializedRef.current;
      const hasDrag = !!dragRef.current;
      const hasInteraction = mouseScreenRef.current.active || !!panRef.current;

      // P9: 交互（拖拽/平移）时节点位置持续变化 → 气泡缓存置脏，下一帧重建
      if (hasInteraction) {
        clusterRegionCacheRef.current.dirty = true;
      }

      // 预先获取聚合物理状态供 LOD 逻辑使用
      const aggPhys = aggPhysRef.current;
      const aggActive = aggPhys !== null && aggPhys.nodes.length > 0;

      // ── LOD 渐进式聚类展开：根据缩放级别自动展开/折叠社区 ──
      // 类似地图缩放：缩得越近，看到的细节越多
      // 关键修复：自动 force cluster 模式下跳过 LOD 折叠，确保首次打开就能看到节点
      if (clusterModeRef.current && aggActive) {
        const zoom = cam.zoom;
        const geom = clusterGeomRef.current;

        // 自动 force cluster 模式：默认全展开，用户缩放后才启用 LOD
        // 避免首次打开时因 zoom < 0.5 导致 LOD 0 全折叠，节点完全不可见
        if (isAutoForceClusterRef.current && !hasInteraction) {
          // 保持 collapsed 为空集（全展开），不执行 LOD 折叠
          collapsedRef.current = new Set();
          lastLodLevelRef.current = -1; // 重置 LOD 状态，下次交互时重新计算
        } else {
          // 计算当前 LOD 级别
          let lodLevel = 0;
          if (zoom >= LOD_THRESHOLDS.ALL) { lodLevel = 3; }
          else if (zoom >= LOD_THRESHOLDS.EXPANDED) { lodLevel = 2; }
          else if (zoom >= LOD_THRESHOLDS.VIEWPORT) { lodLevel = 1; }

          // LOD 变化时重新计算折叠状态（防抖：至少保持 5 帧）
          if (lodLevel !== lastLodLevelRef.current && frameCounterRef.current % 5 === 0) {
            lastLodLevelRef.current = lodLevel;

            const newCollapsed = new Set<number>();
            const expandedInThisFrame: number[] = [];
            const prevCollapsedSize = collapsedRef.current.size;

            // 视口范围（世界坐标）
            const viewW = cam.zoom > 0 ? w / cam.zoom : 0;
            const viewH = cam.zoom > 0 ? h / cam.zoom : 0;
            const vx0 = -cam.x / cam.zoom - viewW / 2;
            const vy0 = -cam.y / cam.zoom - viewH / 2;
            const vx1 = -cam.x / cam.zoom + viewW / 2;
            const vy1 = -cam.y / cam.zoom + viewH / 2;

            for (const [cid, g] of geom) {
              // 手动展开的永远保持展开
              if (manualExpandedRef.current.has(cid)) { continue; }

              if (lodLevel === 0) {
                // LOD 0: 全折叠
                newCollapsed.add(cid);
              } else if (lodLevel === 1) {
                // LOD 1: 仅视口内展开
                const inViewport = g.cx >= vx0 && g.cx <= vx1 && g.cy >= vy0 && g.cy <= vy1;
                if (!inViewport) { newCollapsed.add(cid); }
              } else if (lodLevel === 2) {
                // LOD 2: 视口 + 邻近区域展开（2x 视口范围）
                const marginX = viewW;
                const marginY = viewH;
                const inExpanded = g.cx >= vx0 - marginX && g.cx <= vx1 + marginX
                  && g.cy >= vy0 - marginY && g.cy <= vy1 + marginY;
                if (!inExpanded) { newCollapsed.add(cid); }
              }
              // lodLevel === 3: 全展开（newCollapsed 保持空）
            }

            // 渐进式展开：限制每帧新增展开的社区数
            if (newCollapsed.size < collapsedRef.current.size) {
              // 有新的展开，限制数量
              const toExpand = [];
              for (const cid of collapsedRef.current) {
                if (!newCollapsed.has(cid) && !manualExpandedRef.current.has(cid)) {
                  toExpand.push(cid);
                }
              }
              // 按距离视口中心排序，优先展开近处的
              const cx = (vx0 + vx1) / 2;
              const cy = (vy0 + vy1) / 2;
              toExpand.sort((a, b) => {
                const ga = geom.get(a);
                const gb = geom.get(b);
                if (!ga || !gb) { return 0; }
                const da = Math.hypot(ga.cx - cx, ga.cy - cy);
                const db = Math.hypot(gb.cx - cx, gb.cy - cy);
                return da - db;
              });

              // 计算展开后预计的物理节点数
              const expandedCount = toExpand.length;
              const newAggNodeCount = (aggPhys?.nodes.length ?? 0) + expandedCount * 10; // 粗略估算

              // 如果展开后会超出物理节点限制，只展开部分
              const maxExpand = newAggNodeCount > MAX_AGG_PHYS_NODES
                ? Math.max(1, Math.floor((MAX_AGG_PHYS_NODES - (aggPhys?.nodes.length ?? 0)) / 10))
                : MAX_EXPAND_PER_FRAME;

              for (let i = 0; i < Math.min(maxExpand, toExpand.length); i++) {
                newCollapsed.delete(toExpand[i]);
                expandedInThisFrame.push(toExpand[i]);
              }
            }

            // 物理节点数保护：如果当前聚合物理已超限，强制折叠最远的非手动社区
            if (aggPhys && aggPhys.nodes.length > MAX_AGG_PHYS_NODES) {
              const cx = (vx0 + vx1) / 2;
              const cy = (vy0 + vy1) / 2;
              const collapsible = [];
              for (const cid of newCollapsed) {
                if (manualExpandedRef.current.has(cid)) { continue; }
                const g = geom.get(cid);
                if (!g) { continue; }
                collapsible.push({ cid, dist: Math.hypot(g.cx - cx, g.cy - cy), count: g.count });
              }
              // 按距离从远到近排序，折叠最远的
              collapsible.sort((a, b) => b.dist - a.dist);
              let currentOver = aggPhys.nodes.length - MAX_AGG_PHYS_NODES;
              for (const { cid, count } of collapsible) {
                if (currentOver <= 0) { break; }
                newCollapsed.add(cid);
                currentOver -= count;
              }
            }

            collapsedRef.current = newCollapsed;

            debugLog("[GraphView] LOD update", {
              lodLevel,
              newCollapsedSize: newCollapsed.size,
              prevCollapsedSize,
            });

            // LOD 变化导致折叠集合改变 → 重建聚合物理集
            if (newCollapsed.size !== prevCollapsedSize) {
              // 放到 requestIdleCallback 中：避免阻塞下一帧渲染
              scheduleIdle(() => {
                refreshClusterGeom();
                buildAggregatePhysics();
                setClusterCollapseVersion((v) => v + 1);
              }, 500);
            }
          }
        }
      }

      // ── 聚合物理分支（聚类折叠模式）：物理只模拟聚合节点 + 未折叠节点 ──
      // 折叠社区成员不参与力导向模拟（数量级骤降），聚合节点坐标回写 clusterGeom，
      // 驱动折叠社区几何/聚合边/聚合节点渲染。万级节点打开不卡死的核心。
      if (aggActive) {
        const config: PhysicsConfig = {
          theta: 0.5,
          repulsion: 18000,
          gravity: 0.003,
          damping: 0.82,
          dt: 0.35,
          springForce: 0.08,
          springDamping: 0.85,
          maxVelocity: 8,
        };
        // 规模保护：聚合物理节点过多（社区粒度极细）时放弃力导向，仅静态显示聚合节点，
        // 聚合节点坐标保持质心，避免主线程每帧 O(n log n) 力导向导致完全不响应。
        // 拖拽仍有效（mouse 事件直接写 node.x/y），不受此限制。
        const aggOver = aggPhys.nodes.length > MAX_AGG_PHYS_NODES;
        const stable = aggOver ? false : isSystemStable(aggPhys.nodes, 0.15);
        if (hasInteraction) {
          idleCounterRef.current = 0;
        } else if (stable) {
          idleCounterRef.current++;
        } else {
          idleCounterRef.current = 0;
        }
        // 稳定降频：非交互时每 6 帧才跑一次聚合物理（规模小，成本极低）
        const shouldRun = !aggOver && (hasInteraction || !stable || frameCounterRef.current % 6 === 0);
        if (shouldRun) {
          stepPhysics(
            aggPhys.nodes,
            aggPhys.edges,
            config,
            undefined,
            undefined,
            undefined,
            aggPhys.neighborMap,
          );
          // 聚合节点坐标 → 回写 clusterGeom，驱动折叠社区几何/聚合边/聚合节点渲染
          const geom = clusterGeomRef.current;
          for (const [cid, idx] of aggPhys.cidToNodeIdx) {
            const gn = aggPhys.nodes[idx];
            const g = geom.get(cid);
            if (g) {
              g.cx = gn.x;
              g.cy = gn.y;
            }
          }
          // 不再用聚合节点覆盖 gridIndex —— 会导致 drawExpandedCommunity 找不到原始节点
          // 原始节点的 gridIndex 已在数据初始化时构建，保持不变
          // 聚合节点的位置变化通过 clusterGeom 的 cx/cy 回写驱动渲染
        }

        // 关键修复：aggActive 模式下仍然需要应用 Worker 结果更新原始节点位置。
        // 之前设置 workerResultRef.current = null 导致原始节点位置从未被更新，
        // 节点停留在初始化时的随机位置，而边使用 clusterGeom 中已更新的聚合位置，
        // 造成"只显示边、不显示节点"的问题。
        if (worker && workerReady && !hasDrag) {
          // 请求下一个 Worker 步进
          if (!pendingStepRef.current && !hasDrag && (hasInteraction || frameCounterRef.current % 12 === 0)) {
            const workerConfig: PhysicsConfig = {
              theta: 0.5,
              repulsion: 18000,
              gravity: 0.003,
              damping: 0.82,
              dt: 0.35,
              springForce: 0.08,
              springDamping: 0.85,
              maxVelocity: 8,
            };

            // P8: 社区质心由 Worker 内部维护，主线程不再每 12 帧 O(N) 重算 + 序列化传输
            worker.postMessage({
              type: "step",
              payload: {
                config: workerConfig,
              },
            } as WorkerMessage);
            pendingStepRef.current = true;
          }

          // 应用 Worker 返回的结果到原始节点位置
          const result = workerResultRef.current;
          if (result && result.positions) {
            const hasNewResult = result.tick !== lastProcessedTickRef.current;
            if (hasNewResult) {
              lastProcessedTickRef.current = result.tick;
              // P9: Worker 返回新位置 → 气泡缓存置脏，下一帧重建
              clusterRegionCacheRef.current.dirty = true;
              const n = nodes.length;
              // P6: posMap 存节点对象引用，位置更新经同一对象自动同步，无需再逐条回写。
              // 折叠社区内节点不渲染，跳过其位置回写，避免每 12 帧对全量节点做无意义更新。
              const collapsedSet = collapsedRef.current;
              for (let i = 0; i < n; i++) {
                const node = nodes[i];
                if (node.fixed) { continue; }
                if (collapsedSet.size > 0) {
                  const cid = effCommunities?.get(node.id);
                  if (cid !== undefined && collapsedSet.has(cid)) { continue; }
                }
                node.x = result.positions[i * 2];
                node.y = result.positions[i * 2 + 1];
                node.vx = result.velocities[i * 2];
                node.vy = result.velocities[i * 2 + 1];
              }

              // 重建 gridIndex（仅在有新结果时重建）
              const gridIndex = new Map<string, string[]>();
              for (const n2 of nodes) {
                const gx = Math.floor(n2.x / GRID_CELL_SIZE);
                const gy = Math.floor(n2.y / GRID_CELL_SIZE);
                const key = `${gx},${gy}`;
                const bucket = gridIndex.get(key);
                if (bucket) {
                  bucket.push(n2.id);
                } else {
                  gridIndex.set(key, [n2.id]);
                }
              }
              gridIndexRef.current = gridIndex;

              // 稳定检测
              if (result.stable && !hasInteraction) {
                idleCounterRef.current++;
              } else {
                idleCounterRef.current = 0;
              }
            }
          }
        } else if (worker && !workerReady && !hasDrag) {
          // Worker 未就绪时清空结果标记，避免使用旧结果
          workerResultRef.current = null;
        }
      } else if (worker && workerReady && nodes.length > 0) {
        const enableClusters = clusterModeRef.current && effCommunities;

        // 拖拽时同步位置到 Worker
        if (hasDrag) {
          const dragNode = nodes.find((n) => n.id === dragRef.current!.nodeId);
          if (dragNode) {
            worker.postMessage({
              type: "update",
              payload: {
                nodeIdx: dragNode.idx,
                x: dragNode.x,
                y: dragNode.y,
                fixed: dragNode.fixed,
                vx: dragNode.vx,
                vy: dragNode.vy,
              },
            } as WorkerMessage);
          }
        }

        // 请求下一个物理步进（无 pending 时；稳定后降频到每 12 帧一次，减少 worker 空转）
        if (!pendingStepRef.current && !hasDrag && (hasInteraction || frameCounterRef.current % 12 === 0)) {
          const config: PhysicsConfig = {
            theta: 0.5,
            repulsion: 18000,
            gravity: 0.003,
            damping: 0.82,
            dt: 0.35,
            springForce: 0.08,
            springDamping: 0.85,
            maxVelocity: 8,
            clusterForce: enableClusters ? 0.15 : undefined,
          };

          // P8: communities 与社区质心均由 Worker 内部维护（init 时 nodeIdxToCommunity 已零拷贝传输），
          // 主线程不再每 12 帧 O(N) 重算质心 + Object.fromEntries 序列化
          worker.postMessage({
            type: "step",
            payload: {
              config,
            },
          } as WorkerMessage);
          pendingStepRef.current = true;
        }

        // 应用 Worker 返回的结果到物理节点
        const result = workerResultRef.current;
        if (result && result.positions) {
          // 关键优化：只有 Worker 返回新结果（tick 变化）时才更新节点和重建网格
          // 否则每帧都会用旧结果重算 O(N) 操作，大图下是性能灾难
          const hasNewResult = result.tick !== lastProcessedTickRef.current;
          if (hasNewResult) {
            lastProcessedTickRef.current = result.tick;
            // P9: Worker 返回新位置 → 气泡缓存置脏，下一帧重建
            clusterRegionCacheRef.current.dirty = true;
            const n = nodes.length;
            for (let i = 0; i < n; i++) {
              const node = nodes[i];
              if (!node.fixed) {
                node.x = result.positions[i * 2];
                node.y = result.positions[i * 2 + 1];
                node.vx = result.velocities[i * 2];
                node.vy = result.velocities[i * 2 + 1];
              }
            }

            // 同步重建 gridIndex：Worker 返回新位置后更新网格索引。
            // L2 修复：收敛期不再每步全量重建（2 万节点下单次 10-30ms，收敛期
            // 每步都做会持续占死主线程），改为每 5 步一次，stable 后重建最终版。
            // 间隔内命中检测使用上一次索引，位置滞后 ≤5 步，物理模拟下可接受。
            workerStepCounterRef.current++;
            if (result.stable || workerStepCounterRef.current % 5 === 0) {
              const gridIndex = new Map<string, string[]>();
              for (const n2 of nodes) {
                const gx = Math.floor(n2.x / GRID_CELL_SIZE);
                const gy = Math.floor(n2.y / GRID_CELL_SIZE);
                const key = `${gx},${gy}`;
                const bucket = gridIndex.get(key);
                if (bucket) {
                  bucket.push(n2.id);
                } else {
                  gridIndex.set(key, [n2.id]);
                }
              }
              gridIndexRef.current = gridIndex;
            }

            // 如果处于聚类模式且 Worker ready，更新聚类几何
            // 仅在非聚合物理模式下（aggActive 下由聚合物理节点回写）
            // L2 修复：与 gridIndex 同门限流，收敛期避免每步都排 O(N) 的 idle 任务
            if (
              clusterModeRef.current && !aggActive
              && (result.stable || workerStepCounterRef.current % 5 === 0)
            ) {
              requestIdleCallback(() => {
                refreshClusterGeom();
              }, { timeout: 200 });
            }

            // 大图位图缓存：异步重建
            // L3 修复：不再每次 result 都重建（O(N+E) 绘制 + 巨型 canvas 分配，
            // 收敛期被 idle 回调频繁触发反而抵消位图模式收益），改为收敛期每 30 步
            // 一次、stable 后重建最终版
            if (
              nodes.length > FORCE_BITMAP_THRESHOLD && !hasInteraction
              && (result.stable || workerStepCounterRef.current % 30 === 0)
            ) {
              requestIdleCallback(() => {
                spriteCacheRef.current = buildBigGraphSpriteCache(nodes);
              }, { timeout: 1000 });
            }
          }

          // 稳定检测：即使没有新结果，也基于上一次的 stable 状态更新 idle 计数
          if (result.stable && !hasInteraction) {
            idleCounterRef.current++;
          } else {
            idleCounterRef.current = 0;
          }
        }
      } else if (nodes.length > 0 && !hasDrag) {
        // 回退：没有 Worker 时用原来的主线程物理（兼容 fallback）
        // 大图保护：主线程物理只对中小图可用；超过 MAX_MAIN_THREAD_PHYSICS 时放弃力导向（静态显示）。
        // 否则每帧全量 O(n log n) 会让主线程完全阻塞、主应用无响应。大图等待 Worker 就绪即可。
        const mainThreadSafe = nodes.length <= MAX_MAIN_THREAD_PHYSICS;
        const stable = mainThreadSafe ? isSystemStable(nodes, 0.15) : true;
        if (stable && !hasInteraction) {
          idleCounterRef.current++;
        } else {
          idleCounterRef.current = 0;
        }
        const shouldRunPhysics = mainThreadSafe && (hasInteraction || !stable || idleCounterRef.current % 12 === 0);
        if (shouldRunPhysics) {
          const enableClusters = clusterModeRef.current && effCommunities;
          let centroids = communityCentroidsRef.current;
          if (enableClusters && frameCounterRef.current % 3 === 0) {
            centroids = computeCommunityCentroids(nodes, effCommunities!);
            communityCentroidsRef.current = centroids;
          }
          const config: PhysicsConfig = {
            theta: 0.5,
            repulsion: 18000,
            gravity: 0.003,
            damping: 0.82,
            dt: 0.35,
            springForce: 0.08,
            springDamping: 0.85,
            maxVelocity: 8,
            clusterForce: enableClusters ? 0.15 : undefined,
          };
          stepPhysics(
            nodes,
            edges,
            config,
            undefined,
            enableClusters ? effCommunities : undefined,
            enableClusters ? centroids : undefined,
            neighborMapCacheRef.current,
          );
          // gridIndex 重建改为异步（主线程 fallback 模式，节点数 <= 1500，影响较小）
          if (frameCounterRef.current % 3 === 0) {
            scheduleIdle(() => {
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
            }, 100);
          }
        }
      }

      phaseRef.current += 0.02;

      // 获取当前交互状态（绘制阶段需要）
      const hovered = hoverNodeRef.current;
      const selected = selectedNodeIdRef.current;

      // 计算鱼眼参数（世界坐标下的鼠标位置 + 放大因子）
      const fisheye = computeFisheye();

      // 计算当前视口的世界坐标范围（用于视口裁剪）
      const viewWorld = {
        x0: (-w / 2 - cam.x) / cam.zoom - 50,
        y0: (-h / 2 - cam.y) / cam.zoom - 50,
        x1: (w / 2 - cam.x) / cam.zoom + 50,
        y1: (h / 2 - cam.y) / cam.zoom + 50,
      };

      // 绘制社区聚类区域（背景层；5 帧一次降频）。
      // 聚合折叠视图下由聚合节点表达社区，跳过气泡避免视觉重叠
      // 规模保护：社区数失控（粒度极细至万级）时跳过气泡，避免为每个"社区"绘制
      // 上万 radial-gradient 气泡 + 标签 → 主线程每 5 帧一次全量绘制仍会卡死。
      // D5: 不全折叠时才画气泡——折叠社区由聚类标记表达，不重复绘制气泡。
      // 因此条件从 collapsedRef.current.size === 0（全局）改为 < communities.size（逐个社区判断）。
      if (
        clusterModeRef.current && communities
        && communities.size <= MAX_AGG_PHYS_NODES
        && frameCounterRef.current % 5 === 0
        && collapsedRef.current.size < communities.size
      ) {
        drawClusterRegions(ctx, nodes);
      }

      // 聚合几何已在 Worker ready 回调和 LOD 切换时异步计算
      // 渲染循环中不再同步调用 refreshClusterGeom()，避免 O(N) 阻塞主线程
      // 聚合物理激活时由聚合物理节点回写驱动，非激活时使用上次计算结果
      const forceCluster = nodes.length > AUTO_CLUSTER_THRESHOLD;

      // 绘制（传入视口范围用于裁剪）
      // Worker 未就绪的大图：跳过完整渲染，避免主线程 fallback 卡死
      if (shouldRender && !workerNotReadyLargeGraph) {
        // 强制聚类：节点数 > 3000 时自动进入聚类渲染模式
        // 但如果 clusterGeom 还没准备好，强制走原始渲染路径
        const geomReady = clusterGeomRef.current.size > 0;
        const shouldUseClusterRender = (aggActive || clusterModeRef.current || forceCluster) && geomReady;

        // 关键诊断日志：每 60 帧输出一次渲染路径状态
        if (frameCounterRef.current % 60 === 0) {
          const camInfo = cameraRef.current;
          const vpW = camInfo.zoom > 0 ? w / camInfo.zoom : 0;
          const vpH = camInfo.zoom > 0 ? h / camInfo.zoom : 0;
          debugLog("[GraphView] render path", {
            forceCluster,
            aggActive,
            clusterMode: clusterModeRef.current,
            autoForce: isAutoForceClusterRef.current,
            geomReady,
            shouldUseClusterRender,
            nodes: nodes.length,
            posMapSize: posMapRef.current.size,
            gridIndexCells: gridIndexRef.current?.size ?? 0,
            collapsedSize: collapsedRef.current.size,
            zoom: camInfo.zoom.toFixed(2),
            viewport: { w: vpW.toFixed(0), h: vpH.toFixed(0) },
            spriteCache: spriteCacheRef.current
              ? `${spriteCacheRef.current.width}x${spriteCacheRef.current.height}`
              : null,
          });
        }

        // 主渲染路径本帧绘制的节点数：
        // -1 = 已通过其他方式绘制（聚类标记 / 位图 / 矢量 fallback），安全阀无需介入
        // >=0 = drawExpandedCommunity 实际绘制的节点数，为 0 时安全阀兜底
        let expandedNodesDrawn = -1;

        if (shouldUseClusterRender) {
          // ── 聚类模式：极简渲染策略 ──
          // 全折叠时只画小型聚类标记 + 聚合边
          // 展开社区时才画内部节点
          const activeCommunities = effectiveCommunitiesRef.current ?? communities;
          const totalCommunities = activeCommunities ? new Set(activeCommunities.values()).size : 0;
          // forceCluster 模式下：只有用户手动触发时才强制全折叠
          // 自动 force cluster（isAutoForceClusterRef=true）时保持部分展开，让用户看到真实节点
          const allCollapsed = (forceCluster && !isAutoForceClusterRef.current)
            ? totalCommunities > 0
            : collapsedRef.current.size >= totalCommunities && totalCommunities > 0;
          const isLargeGraph = nodes.length > 5000;

          if (allCollapsed) {
            // ── 全折叠：只画聚类标记（最大15px）+ 聚合边 ──
            const geom = clusterGeomRef.current;
            const aggPhysLocal = aggPhysRef.current;
            // 视口信息（用于调试日志）
            const camLocal = cameraRef.current;
            const zoomLocal = camLocal.zoom;
            const viewW = zoomLocal > 0 ? w / zoomLocal : 0;
            const viewH = zoomLocal > 0 ? h / zoomLocal : 0;
            const vx0Local = -camLocal.x / zoomLocal - viewW / 2;
            const vy0Local = -camLocal.y / zoomLocal - viewH / 2;
            const vx1Local = -camLocal.x / zoomLocal + viewW / 2;
            const vy1Local = -camLocal.y / zoomLocal + viewH / 2;
            if (frameCounterRef.current % 60 === 0) {
              debugLog("[GraphView] forceCluster render state", {
                forceCluster,
                aggActive,
                clusterMode: clusterModeRef.current,
                totalCommunities,
                allCollapsed,
                geomSize: geom.size,
                aggPhysNull: aggPhysLocal === null,
                aggPhysNodes: aggPhysLocal?.nodes.length ?? 0,
                aggPhysEdges: aggPhysLocal?.edges.length ?? 0,
                collapsedSize: collapsedRef.current.size,
                camera: { x: camLocal.x.toFixed(0), y: camLocal.y.toFixed(0), zoom: zoomLocal.toFixed(2) },
                viewport: {
                  x0: vx0Local.toFixed(0),
                  y0: vy0Local.toFixed(0),
                  x1: vx1Local.toFixed(0),
                  y1: vy1Local.toFixed(0),
                },
              });
            }
            if (geom.size > 0) {
              // Obsidian 风格聚合边：细线条、柔和透明度、动态宽度
              const zoom = cameraRef.current.zoom;
              const aggBaseWidth = 0.3;
              const aggZoomScale = zoom < 0.5 ? zoom * 1.5 : Math.min(1, zoom);
              const aggDynamicWidth = aggBaseWidth * aggZoomScale;
              const aggAlpha = zoom < 0.3 ? 0.12 : zoom < 0.5 ? 0.2 : 0.3;

              if (aggPhysLocal && aggPhysLocal.edges.length > 0) {
                ctx.save();
                ctx.strokeStyle = token.colorBorder;
                ctx.lineWidth = aggDynamicWidth;
                ctx.globalAlpha = aggAlpha;
                const aggBatchPaths = new Map<string, Path2D>();
                const aggSampleRate = zoom < 0.3 ? 0.3 : zoom < 0.5 ? 0.6 : 1.0;

                for (let i = 0; i < aggPhysLocal.edges.length; i++) {
                  const e = aggPhysLocal.edges[i];
                  const sNode = aggPhysLocal.nodes[e.sourceIdx];
                  const tNode = aggPhysLocal.nodes[e.targetIdx];
                  if (!sNode || !tNode) { continue; }
                  if (
                    !isInView(sNode.x, sNode.y, viewWorld, 30) || !isInView(tNode.x, tNode.y, viewWorld, 30)
                  ) { continue; }
                  // 确定性降采样：N4 修复——用 source+target 稳定散列替代索引等差
                  // （(i * 77777) % 1000 与边序号线性相关，低采样率时保留边呈周期条纹）
                  if (aggSampleRate < 1.0) {
                    const hash = (Math.abs(hashStringToInt(e.source + e.target)) % 1000) / 1000;
                    if (hash > aggSampleRate) { continue; }
                  }
                  // 聚合边统一用一种颜色和宽度
                  let path = aggBatchPaths.get("default");
                  if (!path) {
                    path = new Path2D();
                    aggBatchPaths.set("default", path);
                  }
                  path.moveTo(sNode.x, sNode.y);
                  path.lineTo(tNode.x, tNode.y);
                }
                for (const path of aggBatchPaths.values()) {
                  ctx.stroke(path);
                }
                ctx.globalAlpha = 1;
                ctx.restore();
              }

              // 聚类标记（小圆形，最大15px）
              ctx.save();
              let drawnClusters = 0;
              let skippedClusters = 0;
              for (const [cid, g] of geom) {
                if (!collapsedRef.current.has(cid)) {
                  skippedClusters++;
                  if (skippedClusters <= 3) {
                    debugLog("[GraphView] cluster skipped (not collapsed)", { cid });
                  }
                  continue;
                }
                if (!isInView(g.cx, g.cy, viewWorld, 30)) { continue; }
                const color = communityPalette[cid % communityPalette.length];
                const maxR = Math.min(15, g.r);
                // 主体
                ctx.globalAlpha = 0.85;
                ctx.beginPath();
                ctx.arc(g.cx, g.cy, maxR, 0, Math.PI * 2);
                ctx.fillStyle = color;
                ctx.fill();
                drawnClusters++;
                // 标签：D4 修复——阈值从 0.8 降至 0.3，总览低 zoom 下聚合彩球也有标注。
                // 字号随缩放动态调整（世界坐标保持约 12px，通过 zoom 换算），配合 measureText 截断。
                if (cam.zoom >= 0.3) {
                  const fontSize = 12 / cam.zoom;
                  ctx.globalAlpha = 0.9;
                  ctx.font = `${fontSize.toFixed(1)}px Inter, system-ui, sans-serif`;
                  ctx.textAlign = "center";
                  ctx.textBaseline = "top";
                  ctx.fillStyle = token.colorText;
                  const label = `${g.label} (${g.count})`;
                  // 限制标签最大宽度，超过则截断
                  const maxLabelWidth = 80 / cam.zoom;
                  let displayLabel = label;
                  const metrics = ctx.measureText(label);
                  if (metrics.width > maxLabelWidth) {
                    const ellipsis = "…";
                    let w = ctx.measureText(ellipsis).width;
                    let i = 0;
                    while (w < maxLabelWidth && i < label.length) {
                      i++;
                      w = ctx.measureText(label.slice(0, i) + ellipsis).width;
                    }
                    displayLabel = label.slice(0, i) + ellipsis;
                  }
                  ctx.fillText(displayLabel, g.cx, g.cy + maxR + fontSize);
                }
              }
              if (frameCounterRef.current % 60 === 0) {
                debugLog("[GraphView] cluster draw stats", {
                  totalGeom: geom.size,
                  drawn: drawnClusters,
                  skipped: skippedClusters,
                  collapsedSize: collapsedRef.current.size,
                });
              }
              ctx.restore();
            } else {
              // ── Fallback：聚合几何尚未就绪（Worker ready 回调延迟计算），
              // 退回非聚类渲染路径，确保节点和边始终可见。
              // 临时清除 collapsed 集，避免 drawNodesOptimized/drawEdgesOptimized
              // 因 clusterModeRef.current 为 true 而跳过折叠社区的节点/边。 ──
              const prevCollapsed = collapsedRef.current;
              collapsedRef.current = new Set();
              const isLargeGraphFallback = nodes.length > FORCE_BITMAP_THRESHOLD;
              if (isLargeGraphFallback && spriteCacheRef.current) {
                const bbox = spriteWorldBBoxRef.current;
                const worldW = bbox.maxX - bbox.minX;
                const worldH = bbox.maxY - bbox.minY;
                const camZ = cam.zoom;
                const sx = bbox.minX * camZ;
                const sy = bbox.minY * camZ;
                const sw = worldW * camZ;
                const sh = worldH * camZ;
                ctx.drawImage(spriteCacheRef.current, sx, sy, sw, sh);
              } else {
                drawEdgesOptimized(ctx, nodes, fisheye, viewWorld);
                drawParticlesOptimized(ctx, nodes, fisheye, viewWorld);
                drawNodesOptimized(ctx, nodes, fisheye, viewWorld);
              }
              collapsedRef.current = prevCollapsed;
            }
          } else {
            // ── 部分展开：绘制展开社区的节点和边 ──
            if (activeCommunities) {
              // N1 修复：接收实际绘制节点数，供下方安全阀判断（此前返回值被丢弃，
              // expandedNodesDrawn 恒为 -1，兜底条件永不触发）
              expandedNodesDrawn = drawExpandedCommunity(ctx, nodes, viewWorld, activeCommunities, isLargeGraph);
            } else {
              // ── Fallback：无社区数据时退回非聚类渲染路径。
              // 临时清除 collapsed 集，避免因 clusterModeRef.current 为 true
              // 而跳过折叠社区的节点/边。 ──
              const prevCollapsed2 = collapsedRef.current;
              collapsedRef.current = new Set();
              const isLargeFallback = nodes.length > FORCE_BITMAP_THRESHOLD;
              if (isLargeFallback && spriteCacheRef.current) {
                const bbox = spriteWorldBBoxRef.current;
                const worldW = bbox.maxX - bbox.minX;
                const worldH = bbox.maxY - bbox.minY;
                const camZ = cam.zoom;
                const sx = bbox.minX * camZ;
                const sy = bbox.minY * camZ;
                const sw = worldW * camZ;
                const sh = worldH * camZ;
                ctx.drawImage(spriteCacheRef.current, sx, sy, sw, sh);
              } else {
                drawEdgesOptimized(ctx, nodes, fisheye, viewWorld);
                drawParticlesOptimized(ctx, nodes, fisheye, viewWorld);
                drawNodesOptimized(ctx, nodes, fisheye, viewWorld);
              }
              collapsedRef.current = prevCollapsed2;
            }
          }
        } else {
          // ── 非聚类模式：使用原始渲染路径 ──
          const isLargeGraph = nodes.length > FORCE_BITMAP_THRESHOLD;
          const hasActiveInteraction = hovered || !!selected || !!dragRef.current;

          if (isLargeGraph && !hasActiveInteraction && spriteCacheRef.current) {
            // 位图模式
            const bbox = spriteWorldBBoxRef.current;
            const worldW = bbox.maxX - bbox.minX;
            const worldH = bbox.maxY - bbox.minY;
            const camZ = cam.zoom;
            const sx = (bbox.minX) * camZ;
            const sy = (bbox.minY) * camZ;
            const sw = worldW * camZ;
            const sh = worldH * camZ;
            ctx.drawImage(spriteCacheRef.current, sx, sy, sw, sh);
          } else {
            // 矢量模式
            drawEdgesOptimized(ctx, nodes, fisheye, viewWorld);
            drawParticlesOptimized(ctx, nodes, fisheye, viewWorld);
            drawNodesOptimized(ctx, nodes, fisheye, viewWorld);
          }
        }

        // ── 终极安全阀：auto force cluster 模式下直接绘制节点 ──
        // 仅在主渲染路径（drawExpandedCommunity）本帧实际绘制 0 个节点时兜底，
        // 防止社区过滤/聚类逻辑导致节点不可见；正常帧不再叠加绘制，
        // 避免双重绘制导致的亮度失真与 hover/selected 高亮被覆盖。
        if (isAutoForceClusterRef.current && nodes.length > 0 && expandedNodesDrawn === 0) {
          const maxDraw = 3000;
          let drawn = 0;
          ctx.save();
          ctx.globalAlpha = 0.85;
          for (const node of nodes) {
            if (drawn >= maxDraw) { break; }
            if (!isInView(node.x, node.y, viewWorld, 30)) { continue; }
            const color = nodeColorRef.current.get(node.id) || token.colorPrimary;
            const size = (nodeSizeRef.current.get(node.id) || 5) * 1.0;
            ctx.fillStyle = color;
            ctx.beginPath();
            ctx.arc(node.x, node.y, size, 0, Math.PI * 2);
            ctx.fill();
            drawn++;
          }
          ctx.restore();
          if (frameCounterRef.current % 60 === 0) {
            debugLog("[GraphView] safety net nodes drawn", { drawn });
          }
        }
      }

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

      // N6 修复：统计弹窗 Zoom 值实时刷新（每 15 帧直接写 DOM，不走 React 重渲染）
      if (statsZoomTextRef.current && frameCounterRef.current % 15 === 0) {
        statsZoomTextRef.current.textContent = `${cameraRef.current.zoom.toFixed(2)}×`;
      }

      if (showMinimap && minimapOpen && minimapRef.current && frameCounterRef.current % MINIMAP_REDRAW_INTERVAL === 0) {
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
    // communities 异步加载后会变化：加入依赖使渲染循环闭包拿到最新值，
    // 否则聚类气泡/社区筛选/聚合折叠全部读不到社区数据（stale closure）
  }, [dimensions, token, communities]);

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
  // D1: 质心数据在 refreshClusterGeom 中与 clusterGeom 同步回填，Worker 主路径下不再为空。
  // D5: 部分折叠时仅折叠社区绘制气泡（展开社区显示真实节点、不画），全折叠时由聚类标记表达、不画。
  // P9: 稳定态缓存——节点位置/折叠集合变化或每 30 帧才重建分组与渐变，静止时直接复用。
  function drawClusterRegions(ctx: CanvasRenderingContext2D, nodes: PhysicsNode[]) {
    const activeCommunities = effectiveCommunitiesRef.current ?? communities;
    if (!activeCommunities) { return; }
    const centroids = communityCentroidsRef.current;
    if (centroids.size === 0) { return; }
    const collapsed = collapsedRef.current;
    const cache = clusterRegionCacheRef.current;

    // 折叠集合变化（LOD / 手动切换）→ 强制重建
    if (collapsed !== cache.lastCollapsed) {
      cache.lastCollapsed = collapsed;
      cache.dirty = true;
    }

    // 全折叠：由聚类标记（彩球）+ 聚合边表达社区，跳过气泡避免视觉重叠
    if (collapsed.size >= centroids.size) { return; }

    // 稳定态判定：非脏且距上次重建不足 30 帧 → 直接复用缓存绘制
    const needsRebuild = cache.dirty || frameCounterRef.current - cache.lastFrame >= 30;
    if (needsRebuild) {
      cache.dirty = false;
      cache.lastFrame = frameCounterRef.current;
      cache.regions.clear();

      // 按社区分组收集节点位置（D5: 部分折叠时展开社区不画气泡，直接跳过）
      const communityNodes = new Map<number, { sx: number; sy: number }[]>();
      for (const node of nodes) {
        const cid = activeCommunities.get(node.id);
        if (cid === undefined) { continue; }
        if (collapsed.size > 0 && !collapsed.has(cid)) { continue; }
        const list = communityNodes.get(cid) ?? [];
        list.push({ sx: node.x, sy: node.y });
        communityNodes.set(cid, list);
      }

      // 为每个社区计算包围盒 + radialGradient，写入缓存
      for (const [cid, points] of communityNodes) {
        if (points.length < 2) { continue; }
        const color = communityPalette[cid % communityPalette.length];

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

        const grad = ctx.createRadialGradient(cx, cy, 0, cx, cy, Math.max(rx, ry));
        grad.addColorStop(0, hexToRgba(color, 0.12));
        grad.addColorStop(0.6, hexToRgba(color, 0.06));
        grad.addColorStop(1, hexToRgba(color, 0));
        cache.regions.set(cid, { cx, cy, rx, ry, grad });
      }
    }

    // 用缓存绘制气泡 + 标签
    for (const [cid, region] of cache.regions) {
      ctx.save();
      ctx.fillStyle = region.grad;
      ctx.beginPath();
      ctx.ellipse(region.cx, region.cy, region.rx, region.ry, 0, 0, Math.PI * 2);
      ctx.fill();

      // 社区标签
      const centroid = centroids.get(cid);
      if (centroid && centroid.count >= 2) {
        ctx.globalAlpha = 0.5;
        // N3 修复：字号处于世界坐标系，除以 zoom 保证任何缩放级别下屏幕字号恒定（11px）
        const labelFontSize = 11 / (cameraRef.current.zoom || 1);
        ctx.font = `bold ${labelFontSize.toFixed(1)}px Inter, system-ui, sans-serif`;
        ctx.textAlign = "center";
        ctx.textBaseline = "middle";
        ctx.fillStyle = communityPalette[cid % communityPalette.length];
        ctx.fillText(
          t("wiki.graph.clusterLabel", { id: cid }) + ` · ${centroid.count}`,
          region.cx,
          region.cy - region.ry + 14,
        );
      }
      ctx.restore();
    }
  }

  // ── 优化绘制函数：带视口裁剪，跳过屏幕外元素 ──

  // 构建大图位图缓存：将所有节点/边预渲染到离屏 Canvas
  // 万级节点下每帧 5 万+ 矢量操作是卡死根因，位图模式将其降为 1 次 drawImage
  function buildBigGraphSpriteCache(nodes: PhysicsNode[]): HTMLCanvasElement | null {
    if (nodes.length === 0) { return null; }

    // 计算节点分布 bounding box —— 聚类模式下只计算可见节点
    // 但 auto force cluster 模式下必须包含所有节点
    const clusterActive = clusterModeRef.current && !isAutoForceClusterRef.current;
    let minX = Infinity, minY = Infinity, maxX = -Infinity, maxY = -Infinity;
    let hasVisible = false;
    for (const n of nodes) {
      if (clusterActive) {
        const cid = getCommunityId(n.id);
        if (cid !== undefined && collapsedRef.current.has(cid)) { continue; }
      }
      if (n.x < minX) { minX = n.x; }
      if (n.y < minY) { minY = n.y; }
      if (n.x > maxX) { maxX = n.x; }
      if (n.y > maxY) { maxY = n.y; }
      hasVisible = true;
    }
    // 如果所有节点都被折叠，使用全量范围
    if (!hasVisible) {
      minX = -500;
      minY = -500;
      maxX = 500;
      maxY = 500;
    }

    // Padding 覆盖整个可视范围
    const padding = 800;
    minX -= padding;
    minY -= padding;
    maxX += padding;
    maxY += padding;
    spriteWorldBBoxRef.current = { minX, minY, maxX, maxY };

    const worldW = maxX - minX;
    const worldH = maxY - minY;

    // 限制离屏 Canvas 最大尺寸，防止内存溢出
    // L1 修复：原逻辑只限单边 16384，不限面积——大世界下 16384²×4 ≈ 1GB RGBA，
    // 浏览器可能分配失败导致位图模式黑屏。增加面积上限 4096²（≈64MB RGBA），
    // 取三个约束的最小缩放比。
    const MAX_CANVAS = 16384;
    const MAX_SPRITE_AREA = 4096 * 4096;
    let scale = Math.min(
      1,
      MAX_CANVAS / Math.max(worldW, worldH),
      Math.sqrt(MAX_SPRITE_AREA / Math.max(1, worldW * worldH)),
    );
    // 保底分辨率：极端分散布局下 sprite 最大边不低于 512px（保底后面积 ≤512²，仍在面积上限内）
    scale = Math.max(scale, 512 / Math.max(worldW, worldH));
    const cw = Math.max(1, Math.ceil(worldW * scale));
    const ch = Math.max(1, Math.ceil(worldH * scale));

    const oc = document.createElement("canvas");
    oc.width = cw;
    oc.height = ch;
    const octx = oc.getContext("2d")!;

    // 世界坐标 → 离屏坐标变换
    octx.save();
    octx.scale(scale, scale);
    octx.translate(-minX, -minY);

    // 批量绘制边（Path2D 合并）—— 聚类模式下跳过折叠社区的边
    const edgeMeta = edgeMetaRef.current;
    const nodeColors = nodeColorRef.current;
    const edgeBatches = new Map<string, Path2D>();
    for (let i = 0; i < edgeMeta.length; i++) {
      const em = edgeMeta[i];
      const sIdx = em.sourceIdx;
      const tIdx = em.targetIdx;
      if (sIdx < 0 || tIdx < 0) { continue; }
      const s = nodes[sIdx];
      const t = nodes[tIdx];
      if (!s || !t) { continue; }
      // 聚类模式：跳过两端都在折叠社区内的边
      if (clusterActive) {
        const sCid = getCommunityId(s.id);
        const tCid = getCommunityId(t.id);
        if (
          sCid !== undefined && tCid !== undefined
          && collapsedRef.current.has(sCid) && collapsedRef.current.has(tCid)
        ) { continue; }
      }
      if (!edgeBatches.has(em.color)) { edgeBatches.set(em.color, new Path2D()); }
      const p = edgeBatches.get(em.color)!;
      p.moveTo(s.x, s.y);
      p.lineTo(t.x, t.y);
    }
    octx.lineWidth = 0.8;
    for (const [color, path] of edgeBatches) {
      octx.strokeStyle = color;
      octx.stroke(path);
    }

    // 批量绘制节点（按颜色合并）
    const nodeBatches = new Map<string, Path2D>();
    const nodeSizes = nodeSizeRef.current;
    for (const n of nodes) {
      if (clusterActive) {
        const ncid = getCommunityId(n.id);
        if (ncid !== undefined && collapsedRef.current.has(ncid)) { continue; }
      }
      const color = nodeColors.get(n.id) || token.colorPrimary;
      const size = (nodeSizes.get(n.id) || 6) * 1.2;
      const key = `${color}|${size.toFixed(1)}`;
      if (!nodeBatches.has(key)) { nodeBatches.set(key, new Path2D()); }
      const p = nodeBatches.get(key)!;
      // 用 arc 添加到 Path2D
      const r = size;
      p.moveTo(n.x + r, n.y);
      p.arc(n.x, n.y, r, 0, Math.PI * 2);
    }
    for (const [key, path] of nodeBatches) {
      const [color] = key.split("|");
      octx.fillStyle = color;
      octx.fill(path);
    }

    octx.restore();
    return oc;
  }

  function isInView(
    x: number,
    y: number,
    view: { x0: number; y0: number; x1: number; y1: number },
    margin = 80,
  ): boolean {
    return x >= view.x0 - margin && x <= view.x1 + margin && y >= view.y0 - margin && y <= view.y1 + margin;
  }

  function drawExpandedCommunity(
    ctx: CanvasRenderingContext2D,
    _nodes: PhysicsNode[],
    viewWorld: { x0: number; y0: number; x1: number; y1: number },
    activeCommunities: Map<string, number>,
    isLargeGraph: boolean,
  ) {
    const zoom = cameraRef.current.zoom;
    const collapsedSet = collapsedRef.current;
    const edgeMeta = edgeMetaRef.current;
    const posMap = posMapRef.current;
    const gridIndex = gridIndexRef.current;

    // 收集展开社区的节点（不在 collapsed 中的社区）
    // 关键修复：节点无社区 ID 时也应绘制，不能被跳过。
    // 只有当节点有社区 ID 且该社区被折叠时才跳过。
    const expandedNodeIds = new Set<string>();

    // 判断节点是否可见的辅助函数
    const isNodeVisible = (id: string): boolean => {
      const cid = activeCommunities.get(id);
      // 无社区 ID 的节点始终可见；有社区 ID 且社区未折叠时可见
      return cid === undefined || !collapsedSet.has(cid);
    };

    // 第一优先级：使用网格索引（O(可见区域) 效率高）
    if (gridIndex) {
      const gx0 = Math.floor(viewWorld.x0 / GRID_CELL_SIZE);
      const gy0 = Math.floor(viewWorld.y0 / GRID_CELL_SIZE);
      const gx1 = Math.floor(viewWorld.x1 / GRID_CELL_SIZE);
      const gy1 = Math.floor(viewWorld.y1 / GRID_CELL_SIZE);

      for (let gx = gx0; gx <= gx1; gx++) {
        for (let gy = gy0; gy <= gy1; gy++) {
          const bucket = gridIndex.get(`${gx},${gy}`);
          if (!bucket) { continue; }
          for (const id of bucket) {
            if (isNodeVisible(id)) {
              expandedNodeIds.add(id);
            }
          }
        }
      }
    }

    // Fallback 1：posMap 遍历（覆盖网格索引未命中的节点）
    if (expandedNodeIds.size === 0 && posMap.size > 0) {
      for (const [id, node] of posMap) {
        if (!isInView(node.x, node.y, viewWorld, 20)) { continue; }
        if (isNodeVisible(id)) {
          expandedNodeIds.add(id);
        }
      }
    }

    // Fallback 2：直接遍历 _nodes 数组（最终兜底，确保节点不会因任何过滤逻辑丢失）
    if (expandedNodeIds.size === 0 && _nodes.length > 0) {
      for (const node of _nodes) {
        if (!isInView(node.x, node.y, viewWorld, 20)) { continue; }
        if (isNodeVisible(node.id)) {
          expandedNodeIds.add(node.id);
        }
      }
    }

    // Fallback 3：终极兜底，跳过所有社区过滤，直接绘制所有视口内节点
    if (expandedNodeIds.size === 0 && _nodes.length > 0) {
      for (const node of _nodes) {
        if (!isInView(node.x, node.y, viewWorld, 20)) { continue; }
        expandedNodeIds.add(node.id);
      }
    }

    if (expandedNodeIds.size === 0) { return 0; }

    // 降采样：大图只画部分节点（使用确定性采样避免闪烁）
    const nodeSampleRate = isLargeGraph ? 0.5 : 1.0;
    const visibleNodes: { id: string; x: number; y: number; size: number; color: string }[] = [];

    for (const id of expandedNodeIds) {
      // 确定性采样：使用节点 ID 的哈希，确保每帧绘制相同的节点
      if (nodeSampleRate < 1) {
        const hash = Math.abs(hashStringToInt(id));
        if (hash % 100 >= nodeSampleRate * 100) { continue; }
      }
      const node = posMap.get(id);
      if (!node) { continue; }
      if (!isInView(node.x, node.y, viewWorld, 20)) { continue; }
      const color = nodeColorRef.current.get(id) || token.colorPrimary;
      const size = nodeSizeRef.current.get(id) || 5;
      visibleNodes.push({ id, x: node.x, y: node.y, size, color });
    }

    // 绘制节点
    ctx.save();
    for (const node of visibleNodes) {
      ctx.globalAlpha = 0.85;
      ctx.beginPath();
      ctx.arc(node.x, node.y, node.size, 0, Math.PI * 2);
      ctx.fillStyle = node.color;
      ctx.fill();
    }
    ctx.restore();

    // 绘制标签（zoom 足够时）
    if (zoom >= 0.4 && visibleNodes.length > 0) {
      ctx.save();
      ctx.textAlign = "center";
      ctx.textBaseline = "top";
      // 字号处于世界坐标系，必须除以 zoom 换算，保证任何缩放级别下屏幕字号恒定（10~12px）。
      // 修复前直接用屏幕像素值：zoom=0.4 时屏幕仅 3.6px 不可读，zoom=5 时 60px 巨大。
      const screenFontSize = zoom >= 1 ? 12 : zoom >= 0.6 ? 11 : 10;
      const fontSize = screenFontSize / zoom;
      // 标签与节点的间距同样换算为世界坐标
      const labelOffset = 3 / zoom;
      ctx.font = `${fontSize.toFixed(1)}px Inter, system-ui, sans-serif`;
      // 标签数量上限：全量画白字标签在万级节点下重叠成白色浓雾。
      // 只画 size（度数代理）最大的 top-N，其余不标。
      const cap = visibleNodes.length > 4000 ? 120 : visibleNodes.length > 1500 ? 250 : 500;
      let labelNodes = visibleNodes;
      if (visibleNodes.length > cap) {
        labelNodes = [...visibleNodes].sort((a, b) => b.size - a.size).slice(0, cap);
      }
      ctx.fillStyle = token.colorText;
      ctx.globalAlpha = 0.85;
      for (const node of labelNodes) {
        const meta = nodeMetaRef.current.get(node.id);
        if (!meta) { continue; }
        const title = meta.title.length > 18 ? meta.title.slice(0, 16) + "…" : meta.title;
        ctx.fillText(title, node.x, node.y + node.size + labelOffset);
      }
      ctx.globalAlpha = 1;
      ctx.restore();
    }

    // 绘制边（只连接展开社区的节点）
    // Obsidian 风格：更细的线宽、更柔和的透明度、动态降采样
    // N2 修复：与 drawEdgesOptimized 对齐——补充边类型筛选、社区筛选、
    // hover/selected 相关边高亮，以及交互时普通边减淡
    if (edgeMeta.length > 0 && visibleNodes.length > 1) {
      const idSet = new Set(visibleNodes.map(n => n.id));
      const zoom = cameraRef.current.zoom;
      const hovered = hoverNodeRef.current;
      const selected = selectedNodeIdRef.current;
      const visibleTypes = visibleEdgeTypesRef.current;
      const hasCommunityFilter = hasCommunityFilterRef.current;
      const visibleCommunitiesSet = visibleCommunitiesRef.current;
      const hasActiveInteraction = !!hovered || !!selected;

      // 动态采样率
      let edgeSampleRate = 1.0;
      if (isLargeGraph) {
        edgeSampleRate = zoom < 0.3 ? 0.15 : zoom < 0.5 ? 0.3 : 0.5;
      } else {
        edgeSampleRate = zoom < 0.3 ? 0.3 : zoom < 0.5 ? 0.6 : 1.0;
      }
      // 交互状态下不降采样，保证相关边完整呈现（与 drawEdgesOptimized 一致）
      if (hasActiveInteraction) { edgeSampleRate = 1.0; }

      // 动态线宽
      const baseWidth = 0.3;
      const zoomScale = zoom < 0.5 ? zoom * 1.5 : Math.min(1, zoom);
      const dynamicWidth = baseWidth * zoomScale;

      ctx.save();
      const batchPaths = new Map<string, { path: Path2D; color: string; width: number }>();
      const relevantPaths = new Map<string, { path: Path2D; color: string; width: number }>();

      for (let i = 0; i < edgeMeta.length; i++) {
        const em = edgeMeta[i];
        if (!idSet.has(em.source) || !idSet.has(em.target)) { continue; }

        // 边类型筛选：用户关闭的类型不绘制
        if (!visibleTypes.has(em.type)) { continue; }

        // 社区筛选：开启过滤后，只画两端社区都可见的边
        if (hasCommunityFilter) {
          const sCid = getCommunityId(em.source);
          const tCid = getCommunityId(em.target);
          const sVisible = sCid === undefined || visibleCommunitiesSet.has(sCid);
          const tVisible = tCid === undefined || visibleCommunitiesSet.has(tCid);
          if (!sVisible || !tVisible) { continue; }
        }

        const sNode = posMap.get(em.source);
        const tNode = posMap.get(em.target);
        if (!sNode || !tNode) { continue; }
        if (!isInView(sNode.x, sNode.y, viewWorld, 10) && !isInView(tNode.x, tNode.y, viewWorld, 10)) { continue; }

        // hover/selected 相关边：单独收集，高亮绘制
        const isRelevant = (hovered && (em.source === hovered || em.target === hovered))
          || (selected && (em.source === selected || em.target === selected));

        // 确定性降采样：P11 用 source+target 稳定散列替代索引等差，避免保留边呈周期条纹
        if (!isRelevant && edgeSampleRate < 1.0) {
          const hash = (Math.abs(hashStringToInt(em.source + em.target)) % 1000) / 1000;
          if (hash > edgeSampleRate) { continue; }
        }

        const width = isRelevant
          ? Math.max(0.5, dynamicWidth * 2) * (em.width / 0.4)
          : dynamicWidth * (em.width / 0.4);
        const key = `${em.color}|${width.toFixed(2)}`;
        const store = isRelevant ? relevantPaths : batchPaths;
        let entry = store.get(key);
        if (!entry) {
          entry = { path: new Path2D(), color: em.color, width };
          store.set(key, entry);
        }
        entry.path.moveTo(sNode.x, sNode.y);
        entry.path.lineTo(tNode.x, tNode.y);
      }

      // Obsidian 风格透明度；交互时普通边减淡，突出相关边
      const normalAlpha = zoom < 0.3 ? 0.12 : zoom < 0.5 ? 0.2 : 0.3;
      ctx.globalAlpha = hasActiveInteraction ? 0.08 : normalAlpha;
      for (const entry of batchPaths.values()) {
        ctx.strokeStyle = entry.color;
        ctx.lineWidth = entry.width;
        ctx.stroke(entry.path);
      }
      // 相关边高亮：更宽、更不透明
      if (relevantPaths.size > 0) {
        ctx.globalAlpha = 0.85;
        for (const entry of relevantPaths.values()) {
          ctx.strokeStyle = entry.color;
          ctx.lineWidth = entry.width;
          ctx.stroke(entry.path);
        }
      }
      ctx.globalAlpha = 1;
      ctx.restore();
    }

    return visibleNodes.length;
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

    // Obsidian 风格连线：根据缩放级别动态调整
    // 低缩放时降采样 + 更透明，高缩放时全量 + 更清晰
    const totalEdges = edgeMeta.length;
    const hasActiveInteraction = hovered || !!selected;

    // 降采样率：缩放越低，采样率越低
    let sampleRate = 1.0;
    if (zoom < 0.2) {
      sampleRate = 0.2; // 极低缩放：只画 20% 的边
    } else if (zoom < 0.4) {
      sampleRate = 0.4; // 低缩放：只画 40% 的边
    } else if (zoom < 0.6) {
      sampleRate = 0.7; // 中低缩放：画 70% 的边
    }

    // 大图边数量保护
    let edgeLimit = totalEdges;
    if (totalEdges > 50000 && !hasActiveInteraction) {
      edgeLimit = Math.floor(totalEdges * sampleRate);
    }

    // 动态线宽：根据缩放调整
    const baseWidth = 0.3; // 基础线宽（Obsidian 风格）
    const zoomScale = zoom < 0.5 ? zoom * 1.5 : Math.min(1, zoom);
    const dynamicWidth = baseWidth * zoomScale;

    const hasCommunityFilter = hasCommunityFilterRef.current;
    const batchPaths = new Map<string, { path: Path2D; color: string; width: number }>();

    for (let i = 0; i < edgeLimit; i++) {
      const em = edgeMeta[i];

      if (!visibleTypes.has(em.type)) { continue; }

      // 降采样：用 source+target 稳定散列确保每帧画相同的边（P11 替代索引等差避免条纹）
      if (sampleRate < 1.0 && !hasActiveInteraction) {
        const hash = (Math.abs(hashStringToInt(em.source + em.target)) % 1000) / 1000;
        if (hash > sampleRate) { continue; }
      }

      const sNode = nodes[em.sourceIdx];
      const tNode = nodes[em.targetIdx];
      if (!sNode || !tNode) { continue; }

      const sCid = getCommunityId(em.source);
      const tCid = getCommunityId(em.target);
      const skipClusterCollapse = isAutoForceClusterRef.current;
      const sCollapsed = !skipClusterCollapse && clusterModeRef.current && sCid !== undefined
        && collapsedRef.current.has(sCid);
      const tCollapsed = !skipClusterCollapse && clusterModeRef.current && tCid !== undefined
        && collapsedRef.current.has(tCid);
      const sGeom = sCollapsed ? clusterGeomRef.current.get(sCid!) : undefined;
      const tGeom = tCollapsed ? clusterGeomRef.current.get(tCid!) : undefined;
      const s: { x: number; y: number } = sGeom ? { x: sGeom.cx, y: sGeom.cy } : sNode;
      const t: { x: number; y: number } = tGeom ? { x: tGeom.cx, y: tGeom.cy } : tNode;

      if (!isInView(s.x, s.y, viewWorld) && !isInView(t.x, t.y, viewWorld)) { continue; }

      if (hasCommunityFilter) {
        const sVisible = sCid === undefined || visibleCommunitiesSet.has(sCid);
        const tVisible = tCid === undefined || visibleCommunitiesSet.has(tCid);
        if (!sVisible || !tVisible) { continue; }
      }

      const isRelevant = hovered && (em.source === hovered || em.target === hovered)
        || selected && (em.source === selected || em.target === selected);

      if (zoom < 0.15 && !isRelevant) { continue; }

      if (isRelevant) {
        const sScale = fisheyeScale(s.x, s.y, fisheye);
        const tScale = fisheyeScale(t.x, t.y, fisheye);
        const avgScale = (sScale + tScale) / 2;
        ctx.beginPath();
        ctx.moveTo(s.x, s.y);
        const dx = t.x - s.x;
        const dy = t.y - s.y;
        const mx = (s.x + t.x) / 2;
        const my = (s.y + t.y) / 2;
        const curveAmount = Math.min(30, Math.sqrt(dx * dx + dy * dy) * 0.15);
        const nx = -dy / (Math.sqrt(dx * dx + dy * dy) || 1);
        const ny = dx / (Math.sqrt(dx * dx + dy * dy) || 1);
        const cpX = mx + nx * curveAmount;
        const cpY = my + ny * curveAmount;
        ctx.quadraticCurveTo(cpX, cpY, t.x, t.y);
        ctx.strokeStyle = em.color;
        ctx.lineWidth = Math.max(0.5, dynamicWidth * 2) * avgScale;
        ctx.globalAlpha = 0.85;
        ctx.stroke();
      } else {
        const width = dynamicWidth * (em.width / 0.4);
        const key = `${em.color}|${width.toFixed(2)}`;
        let entry = batchPaths.get(key);
        if (!entry) {
          entry = { path: new Path2D(), color: em.color, width };
          batchPaths.set(key, entry);
        }
        if (nodes.length < 5000 && zoom >= 0.3) {
          const dx = t.x - s.x;
          const dy = t.y - s.y;
          const mx = (s.x + t.x) / 2;
          const my = (s.y + t.y) / 2;
          const curveAmount = Math.min(20, Math.sqrt(dx * dx + dy * dy) * 0.08);
          const len = Math.sqrt(dx * dx + dy * dy) || 1;
          const cpX = mx + (-dy / len) * curveAmount;
          const cpY = my + (dx / len) * curveAmount;
          entry.path.moveTo(s.x, s.y);
          entry.path.quadraticCurveTo(cpX, cpY, t.x, t.y);
        } else {
          entry.path.moveTo(s.x, s.y);
          entry.path.lineTo(t.x, t.y);
        }
      }
    }

    if (batchPaths.size > 0) {
      // Obsidian 风格透明度：正常 0.35，hover/选中时更淡 0.1
      const normalAlpha = zoom < 0.3 ? 0.15 : zoom < 0.5 ? 0.25 : 0.35;
      const hoverAlpha = 0.08;
      ctx.globalAlpha = (hovered || selected) ? hoverAlpha : normalAlpha;
      const batchFeScale = fisheye.active ? fisheyeScale(fisheye.worldX, fisheye.worldY, fisheye) : 1;
      for (const entry of batchPaths.values()) {
        ctx.strokeStyle = entry.color;
        ctx.lineWidth = entry.width * batchFeScale;
        ctx.stroke(entry.path);
      }
      ctx.globalAlpha = 1;
    }
  }

  function drawParticlesOptimized(
    ctx: CanvasRenderingContext2D,
    nodes: PhysicsNode[],
    fisheye: FisheyeState,
    viewWorld: { x0: number; y0: number; x1: number; y1: number },
  ) {
    // 粒子默认关闭（对齐 Obsidian 静态细边），开关在工具栏/快捷键 p
    if (!particlesEnabledRef.current) { return; }
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

      // 聚类折叠模式：折叠社区内的边不画粒子（由聚合节点/聚合边表达）
      if (clusterModeRef.current) {
        const sCid = getCommunityId(em.source);
        const tCid = getCommunityId(em.target);
        if (
          (sCid !== undefined && collapsedRef.current.has(sCid))
          || (tCid !== undefined && collapsedRef.current.has(tCid))
        ) {
          continue;
        }
      }

      const x = s.x + (t.x - s.x) * p.progress;
      const y = s.y + (t.y - s.y) * p.progress;

      // 视口裁剪：粒子不在视口内时跳过
      if (!isInView(x, y, viewWorld, 30)) { continue; }

      const scale = fisheyeScale(x, y, fisheye);
      const alpha = 0.6 + 0.4 * Math.sin(p.progress * Math.PI * 2);
      // 稳定时跳过 shadowBlur（开销大）；用直接属性设置替代 save/restore
      if (!isStable) {
        ctx.shadowColor = p.color;
        ctx.shadowBlur = 6 * scale;
      }
      ctx.fillStyle = p.color;
      ctx.globalAlpha = alpha;
      ctx.beginPath();
      ctx.arc(x, y, p.size * scale, 0, Math.PI * 2);
      ctx.fill();
    }
    ctx.globalAlpha = 1;
    ctx.shadowBlur = 0;
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
    const showAllLabels = zoom >= 0.35 && !hasHighlight;
    const isLargeGraph = nodes.length > GLOW_NODE_LIMIT;

    // ── 关键性能优化：使用网格索引获取视口内的节点，避免遍历所有节点 ──
    const gridIndex = gridIndexRef.current;
    const nodeMap = posMapRef.current; // id -> PhysicsNode 映射
    const visibleNodeIds = new Set<string>();

    if (gridIndex && nodes.length > 1000) {
      // 大图模式：使用网格索引
      const gx0 = Math.floor(viewWorld.x0 / GRID_CELL_SIZE);
      const gy0 = Math.floor(viewWorld.y0 / GRID_CELL_SIZE);
      const gx1 = Math.floor(viewWorld.x1 / GRID_CELL_SIZE);
      const gy1 = Math.floor(viewWorld.y1 / GRID_CELL_SIZE);

      for (let gx = gx0; gx <= gx1; gx++) {
        for (let gy = gy0; gy <= gy1; gy++) {
          const bucket = gridIndex.get(`${gx},${gy}`);
          if (bucket) {
            for (const id of bucket) {
              visibleNodeIds.add(id);
            }
          }
        }
      }
    } else {
      // 小图模式：直接遍历所有节点（小图性能影响不大）
      for (const node of nodes) {
        if (isInView(node.x, node.y, viewWorld)) {
          visibleNodeIds.add(node.id);
        }
      }
    }

    // 只绘制视口内的节点
    // 交互外标签（showAllLabels）延后收集：万级节点全量画白字标签会重叠成
    // 一团白色浓雾（截图实锤），且每帧上万次 fillText 是性能黑洞。
    const deferredLabels: { id: string; x: number; y: number; size: number; alpha: number }[] = [];
    for (const nodeId of visibleNodeIds) {
      const node = nodeMap.get(nodeId);
      if (!node) { continue; }

      // 聚类折叠模式：折叠社区的节点由聚合节点替代，不单独绘制
      // 但在 auto force cluster 模式下，必须绘制所有节点
      if (clusterModeRef.current && !isAutoForceClusterRef.current) {
        const ncid = getCommunityId(node.id);
        if (ncid !== undefined && collapsedRef.current.has(ncid)) { continue; }
      }

      if (hasCommunityFilter) {
        const cid = getCommunityId(node.id);
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
        alpha = 0.15;
        glowAlpha = 0;
        size = baseSize * 0.85 * feScale;
      }

      const pulse = 1 + Math.sin(phase + node.x * 0.01) * 0.08;
      const finalSize = size * pulse;

      const isInteractNode = isSelected || isHovered
        || (selected && neighborsOfSelected.has(node.id))
        || (hovered && neighborsOfHovered.has(node.id));

      if (glowAlpha > 0 && zoom >= 0.6 && (isInteractNode || !isLargeGraph)) {
        if (idleCounterRef.current === 0) {
          ctx.shadowColor = color;
          ctx.shadowBlur = glowRadius;
        }
        ctx.globalAlpha = glowAlpha * alpha;
        ctx.beginPath();
        ctx.arc(node.x, node.y, finalSize, 0, Math.PI * 2);
        ctx.fillStyle = color;
        ctx.fill();
        ctx.shadowBlur = 0;
      }

      ctx.globalAlpha = alpha;
      const screenR = finalSize * cameraRef.current.zoom;
      const sprite = nodeSpriteCacheRef.current.get(color);
      if (sprite && screenR >= 4) {
        const dstSize = finalSize * 2;
        ctx.drawImage(sprite, 0, 0, SPRITE_SIZE, SPRITE_SIZE, node.x - finalSize, node.y - finalSize, dstSize, dstSize);
      } else {
        ctx.fillStyle = color;
        ctx.fillRect(node.x - finalSize, node.y - finalSize, finalSize * 2, finalSize * 2);
      }

      if (isHovered) {
        const ripplePhase = phase * 0.5;
        const rippleBase = finalSize * 2.5;
        for (let ri = 0; ri < 2; ri++) {
          const rp = (ripplePhase + ri * 0.5) % 1;
          ctx.globalAlpha = (ri === 0 ? 0.35 : 0.18) * (1 - rp);
          ctx.strokeStyle = color;
          ctx.lineWidth = 1.2;
          ctx.beginPath();
          ctx.arc(node.x, node.y, rippleBase + rp * 26, 0, Math.PI * 2);
          ctx.stroke();
        }
        ctx.globalAlpha = 1;
      }

      if (showLabel) {
        const meta = nodeMetaRef.current.get(node.id);
        if (meta) {
          ctx.save();
          ctx.globalAlpha = alpha * 0.9;
          // N3 修复：字号处于世界坐标系，必须除以 zoom 换算（feScale 为鱼眼放大系数，保留）。
          // 修复前 zoom=0.35 时屏幕字号仅 ~4px 不可读，zoom=5 时 60px 巨大
          ctx.font = `${Math.round((12 * feScale) / zoom)}px Inter, system-ui, sans-serif`;
          ctx.textAlign = "center";
          ctx.textBaseline = "top";
          const label = meta.title.length > 15 ? meta.title.slice(0, 13) + "…" : meta.title;
          ctx.fillStyle = token.colorText;
          ctx.fillText(label, node.x, node.y + finalSize + 4);
          ctx.restore();
        }
      } else if (showAllLabels) {
        // 交互外标签延后绘制（见 deferredLabels 声明处的说明）
        deferredLabels.push({ id: node.id, x: node.x, y: node.y, size: finalSize, alpha });
      }
    }

    // 非交互标签：按节点大小（度数代理）排序后只画 top-N，避免标签浓雾
    if (deferredLabels.length > 0) {
      const cap = deferredLabels.length > 4000 ? 120 : deferredLabels.length > 1500 ? 250 : 500;
      if (deferredLabels.length > cap) {
        deferredLabels.sort((a, b) => b.size - a.size);
        deferredLabels.length = cap;
      }
      ctx.save();
      ctx.font = `${Math.round(12 / zoom)}px Inter, system-ui, sans-serif`;
      ctx.textAlign = "center";
      ctx.textBaseline = "top";
      ctx.fillStyle = token.colorText;
      ctx.globalAlpha = 0.9;
      for (const d of deferredLabels) {
        const meta = nodeMetaRef.current.get(d.id);
        if (!meta) { continue; }
        const label = meta.title.length > 15 ? meta.title.slice(0, 13) + "…" : meta.title;
        ctx.fillText(label, d.x, d.y + d.size + 4);
      }
      ctx.globalAlpha = 1;
      ctx.restore();
    }
    ctx.globalAlpha = 1;

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
  function lightenColor(color: string, percent: number): string {
    const c = parseColor(color);
    if (!c) { return color; }
    const r = clamp(c.r + percent, 0, 255);
    const g = clamp(c.g + percent, 0, 255);
    const b = clamp(c.b + percent, 0, 255);
    return `rgb(${r},${g},${b})`;
  }

  function darkenColor(color: string, percent: number): string {
    const c = parseColor(color);
    if (!c) { return color; }
    const r = clamp(c.r - percent, 0, 255);
    const g = clamp(c.g - percent, 0, 255);
    const b = clamp(c.b - percent, 0, 255);
    return `rgb(${r},${g},${b})`;
  }

  function hexToRgba(color: string, alpha: number): string {
    const c = parseColor(color);
    if (!c) { return color; }
    return `rgba(${c.r},${c.g},${c.b},${alpha})`;
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

  // 构建聚合物理集：聚类折叠模式下，只对「聚合节点 + 未折叠节点」做物理。
  // 折叠社区的成员节点不进入物理（数量级骤降，物理规模 = 社区数 + 未折叠成员数），
  // 从根本上避免万级节点全量力导向收敛导致的卡死。
  const buildAggregatePhysics = useCallback(() => {
    const collapsed = collapsedRef.current;
    const communitiesMap = communitiesRef.current;
    const allNodes = physNodesRef.current;
    const edgeMeta = edgeMetaRef.current;
    if (!communitiesMap || collapsed.size === 0 || allNodes.length === 0) {
      debugLog("[GraphView] buildAggregatePhysics early return", {
        communitiesMapNull: communitiesMap === null,
        collapsedSize: collapsed.size,
        allNodesLength: allNodes.length,
      });
      aggPhysRef.current = null;
      return;
    }
    const aggNodes: PhysicsNode[] = [];
    const idToIdx = new Map<string, number>();
    const cidToNodeIdx = new Map<number, number>();

    // 预计算每个社区成员数（O(N) 一次遍历，替代 naive 的 O(C×N) 双重循环。
    // 大图打开时若社区粒度细，O(C×N) 可达数千万次 Map 查找，主线程会卡死数秒）
    const memberCount = new Map<number, number>();
    for (const node of allNodes) {
      const cid = communitiesMap.get(node.id);
      if (cid !== undefined) {
        memberCount.set(cid, (memberCount.get(cid) ?? 0) + 1);
      }
    }

    // 每个折叠社区 → 1 个聚合物理节点（坐标取当前聚合几何质心）
    for (const cid of collapsed) {
      const geom = clusterGeomRef.current.get(cid);
      const count = memberCount.get(cid) ?? 0;
      const idx = aggNodes.length;
      const id = `__agg__${cid}`;
      aggNodes.push({
        id,
        x: geom?.cx ?? 0,
        y: geom?.cy ?? 0,
        vx: 0,
        vy: 0,
        fx: 0,
        fy: 0,
        mass: Math.max(1, count * 0.6), // 聚合质量 = 成员数加权
        fixed: false,
        kind: "source",
        idx,
      });
      idToIdx.set(id, idx);
      cidToNodeIdx.set(cid, idx);
    }

    // 未折叠社区成员 + 零散节点 → 真实物理节点（共享 physNodesRef 对象引用，就地更新）
    for (const node of allNodes) {
      const cid = communitiesMap.get(node.id);
      if (cid !== undefined && collapsed.has(cid)) { continue; }
      idToIdx.set(node.id, aggNodes.length);
      aggNodes.push(node);
    }

    // 聚合边：遍历全部边，把端点映射到聚合/真实节点索引，去重合并
    const aggEdges: PhysicsEdge[] = [];
    const seen = new Map<number, number>();
    const edgeKey = (a: number, b: number) => (a < b ? a * 100000 + b : b * 100000 + a);
    for (const em of edgeMeta) {
      const sCid = communitiesMap.get(em.source);
      const tCid = communitiesMap.get(em.target);
      const sIsCollapsed = sCid !== undefined && collapsed.has(sCid);
      const tIsCollapsed = tCid !== undefined && collapsed.has(tCid);
      const sKey = sIsCollapsed ? `__agg__${sCid}` : em.source;
      const tKey = tIsCollapsed ? `__agg__${tCid}` : em.target;
      const sIdx = idToIdx.get(sKey);
      const tIdx = idToIdx.get(tKey);
      if (sIdx === undefined || tIdx === undefined || sIdx === tIdx) { continue; }
      const key = edgeKey(sIdx, tIdx);
      const existing = seen.get(key);
      if (existing !== undefined) {
        // 合并重复边：保留更紧凑的 restLength（多边归并为单一拓扑张力）
        const e = aggEdges[existing];
        if (e.restLength > 140) { e.restLength = 140; }
        continue;
      }
      seen.set(key, aggEdges.length);
      aggEdges.push({
        source: sKey,
        target: tKey,
        restLength: 140,
        stiffness: 0.8,
        damping: 0.6,
        sourceIdx: sIdx,
        targetIdx: tIdx,
      });
    }

    aggPhysRef.current = {
      nodes: aggNodes,
      edges: aggEdges,
      cidToNodeIdx,
      neighborMap: buildNeighborMap(aggEdges),
    };
    debugLog("[GraphView] buildAggregatePhysics success", {
      aggNodes: aggNodes.length,
      aggEdges: aggEdges.length,
      cidToNodeIdx: cidToNodeIdx.size,
    });
  }, []);

  // 刷新聚合节点几何（质心/半径/计数/代表名）。O(N) 遍历，低频调用（每 6 帧 / 切换时）
  const refreshClusterGeom = useCallback(() => {
    const activeCommunities = effectiveCommunitiesRef.current ?? communities;
    const nodeCount = physNodesRef.current.length;
    // 强制聚类模式：节点数超过阈值时也需要计算聚类几何
    const isForceCluster = nodeCount > AUTO_CLUSTER_THRESHOLD;
    if (!activeCommunities || (!clusterModeRef.current && !isForceCluster)) {
      if (frameCounterRef.current % 60 === 0) {
        debugLog("[GraphView] refreshClusterGeom early return", {
          activeCommunitiesNull: activeCommunities === null,
          clusterMode: clusterModeRef.current,
          isForceCluster,
        });
      }
      clusterGeomRef.current = new Map();
      // D1: 几何不可用时空置质心缓存，保持 drawClusterRegions 的 early return 语义一致
      communityCentroidsRef.current = new Map();
      return;
    }
    const buckets = new Map<
      number,
      { sx: number; sy: number; count: number; bestId: string | null; bestDegree: number }
    >();
    const nodes = physNodesRef.current;
    for (let i = 0; i < nodes.length; i++) {
      const node = nodes[i];
      const cid = activeCommunities.get(node.id);
      if (cid === undefined) { continue; }
      const b = buckets.get(cid) ?? { sx: 0, sy: 0, count: 0, bestId: null, bestDegree: -1 };
      b.sx += node.x;
      b.sy += node.y;
      b.count += 1;
      const meta = nodeMetaRef.current.get(node.id);
      const deg = (meta?.linkCount ?? 0) + (meta?.backlinkCount ?? 0);
      if (deg > b.bestDegree) {
        b.bestDegree = deg;
        b.bestId = node.id;
      }
      buckets.set(cid, b);
    }
    const next = new Map<number, { cx: number; cy: number; r: number; count: number; label: string }>();
    for (const [cid, b] of buckets) {
      const cx = b.sx / b.count;
      const cy = b.sy / b.count;
      const r = Math.max(10, Math.min(44, 8 + Math.sqrt(b.count) * 2.2));
      const title = b.bestId ? (nodeMetaRef.current.get(b.bestId)?.title ?? "") : "";
      const label = title.length > 14 ? title.slice(0, 12) + "…" : title || `#${cid}`;
      next.set(cid, { cx, cy, r, count: b.count, label });
    }
    clusterGeomRef.current = next;
    // D1: 同步回填社区质心缓存，供 drawClusterRegions（背景气泡）使用。
    // refreshClusterGeom 在 Worker ready 回调 / LOD 切换 / 折叠切换时都会被调用，
    // 使 Worker 主路径下 communityCentroidsRef 不再为空，恢复气泡渲染。
    const centroidMap = new Map<number, { cx: number; cy: number; count: number }>();
    for (const [cid, g] of next) {
      centroidMap.set(cid, { cx: g.cx, cy: g.cy, count: g.count });
    }
    communityCentroidsRef.current = centroidMap;
    // 质心变化 → 气泡缓存置脏，下一帧重建
    clusterRegionCacheRef.current.dirty = true;
    if (frameCounterRef.current % 60 === 0) {
      const positions = [];
      let minX = Infinity, maxX = -Infinity, minY = Infinity, maxY = -Infinity;
      for (const [cid, g] of next) {
        positions.push({ cid, cx: g.cx.toFixed(0), cy: g.cy.toFixed(0), r: g.r.toFixed(0), count: g.count });
        minX = Math.min(minX, g.cx);
        maxX = Math.max(maxX, g.cx);
        minY = Math.min(minY, g.cy);
        maxY = Math.max(maxY, g.cy);
      }
      debugLog("[GraphView] refreshClusterGeom success", {
        bucketCount: buckets.size,
        nextSize: next.size,
        bbox: { minX: minX.toFixed(0), maxX: maxX.toFixed(0), minY: minY.toFixed(0), maxY: maxY.toFixed(0) },
        sample: positions.slice(0, 5),
      });
    }
  }, [communities]);

  // 切换社区折叠状态（点击聚合节点）
  const toggleCluster = useCallback((cid: number) => {
    const next = new Set(collapsedRef.current);
    const manualNext = new Set(manualExpandedRef.current);
    if (next.has(cid)) {
      next.delete(cid);
      // 手动展开的社区标记，防止 LOD 自动折叠
      manualNext.add(cid);
    } else {
      next.add(cid);
      // 手动折叠的社区，从手动展开列表移除
      manualNext.delete(cid);
    }
    collapsedRef.current = next;
    manualExpandedRef.current = manualNext;
    // 立即刷新聚合几何（展开/收起后质心渲染立即生效）
    refreshClusterGeom();
    // 折叠集合变化 → 重建聚合物理集（聚合节点/未折叠成员集合都变了）
    buildAggregatePhysics();
    setClusterCollapseVersion((v) => v + 1);
  }, [refreshClusterGeom, buildAggregatePhysics]);

  // 聚合节点命中检测（聚类模式 + 折叠社区）
  const findClusterAt = useCallback((sx: number, sy: number): number | null => {
    if (!clusterModeRef.current) { return null; }
    const world = getScreenToWorld(sx, sy);
    for (const [cid, geom] of clusterGeomRef.current) {
      if (!collapsedRef.current.has(cid)) { continue; }
      const dx = world.x - geom.cx;
      const dy = world.y - geom.cy;
      const hitR = geom.r * 1.6; // 含外圈光晕
      if (dx * dx + dy * dy < hitR * hitR) {
        return cid;
      }
    }
    return null;
  }, [dimensions]);

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
          // 聚类折叠模式：折叠社区的节点被聚合节点覆盖，不参与命中
          if (clusterModeRef.current) {
            const cid = getCommunityId(id);
            if (cid !== undefined && collapsedRef.current.has(cid)) { continue; }
          }
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

    // 系统稳定时复用缓存包围盒；运动中或无缓存时重算
    const stable = idleCounterRef.current > 30;
    let bbox = stable ? minimapBBoxRef.current : null;
    if (!bbox) {
      let minX = Infinity, minY = Infinity, maxX = -Infinity, maxY = -Infinity;
      for (const n of nodes) {
        if (n.x < minX) { minX = n.x; }
        if (n.y < minY) { minY = n.y; }
        if (n.x > maxX) { maxX = n.x; }
        if (n.y > maxY) { maxY = n.y; }
      }
      bbox = { minX, minY, maxX, maxY };
      minimapBBoxRef.current = bbox;
    }
    let { minX, minY, maxX, maxY } = bbox;
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

    // 聚合折叠模式：minimap 与主视图一致——折叠社区画聚合点，展开社区画真实节点
    const clusterActive = clusterModeRef.current && collapsedRef.current.size > 0;
    if (clusterActive) {
      const geom = clusterGeomRef.current;
      // 折叠社区 → 聚合点（社区色，更大）
      for (const [cid, g] of geom) {
        if (!collapsedRef.current.has(cid)) { continue; }
        const mx = (g.cx - minX) * scale + offsetX;
        const my = (g.cy - minY) * scale + offsetY;
        mmCtx.fillStyle = communityPalette[cid % communityPalette.length];
        mmCtx.beginPath();
        mmCtx.arc(mx, my, 2.6, 0, Math.PI * 2);
        mmCtx.fill();
      }
      // 展开社区 → 真实节点（小点，降采样）
      const nodeStep = nodes.length > 20000 ? 8 : nodes.length > 8000 ? 4 : nodes.length > 3000 ? 2 : 1;
      for (let i = 0; i < nodes.length; i += nodeStep) {
        const n = nodes[i];
        const cid = getCommunityId(n.id);
        if (cid !== undefined && collapsedRef.current.has(cid)) { continue; }
        const color = nodeColorRef.current.get(n.id) || token.colorPrimary;
        const mx = (n.x - minX) * scale + offsetX;
        const my = (n.y - minY) * scale + offsetY;
        mmCtx.fillStyle = color;
        mmCtx.beginPath();
        mmCtx.arc(mx, my, 1.8, 0, Math.PI * 2);
        mmCtx.fill();
      }
    } else {
      // 普通模式：节点绘制降采样（大图概览无需逐点绘制）
      const nodeStep = nodes.length > 20000 ? 8 : nodes.length > 8000 ? 4 : nodes.length > 3000 ? 2 : 1;
      for (let i = 0; i < nodes.length; i += nodeStep) {
        const n = nodes[i];
        const color = nodeColorRef.current.get(n.id) || token.colorPrimary;
        const mx = (n.x - minX) * scale + offsetX;
        const my = (n.y - minY) * scale + offsetY;
        mmCtx.fillStyle = color;
        mmCtx.beginPath();
        mmCtx.arc(mx, my, 1.8, 0, Math.PI * 2);
        mmCtx.fill();
      }
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
  }, [token, dimensions, communities]);

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

    // 聚合节点点击：展开/收起社区（优先于普通节点/平移）
    const clusterId = findClusterAt(sx, sy);
    if (clusterId !== null) {
      suppressAutoFocusRef.current = true;
      toggleCluster(clusterId);
      return;
    }

    const nodeId = findNodeAt(sx, sy);

    if (nodeId) {
      suppressAutoFocusRef.current = true;
      const node = posMapRef.current.get(nodeId);
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
  }, [findNodeAt, findClusterAt, toggleCluster, onNodeClick, onDeselect]);

  const handleMouseMove = useCallback((e: ReactMouseEvent<HTMLCanvasElement>) => {
    const rect = canvasRef.current!.getBoundingClientRect();
    const sx = e.clientX - rect.left;
    const sy = e.clientY - rect.top;

    // 记录鼠标屏幕位置（供鱼眼使用）
    mouseScreenRef.current = { x: sx, y: sy, active: true };

    if (dragRef.current) {
      const world = getScreenToWorld(sx, sy);
      const node = posMapRef.current.get(dragRef.current!.nodeId);
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
      // 聚合节点 hover 优先（聚类折叠模式）
      const clusterId = findClusterAt(sx, sy);
      if (clusterId !== null) {
        if (hoverClusterRef.current !== clusterId) {
          hoverClusterRef.current = clusterId;
          canvasRef.current!.style.cursor = "pointer";
        }
        if (hoverNodeRef.current) {
          hoverNodeRef.current = null;
          onNodeHover?.(null);
          tooltipVisibleRef.current = false;
          setTooltipNodeIdState(null);
        }
        return;
      }
      hoverClusterRef.current = null;

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
  }, [findNodeAt, findClusterAt, onNodeHover, dimensions]);

  const handleMouseUp = useCallback(() => {
    if (dragRef.current) {
      const node = posMapRef.current.get(dragRef.current!.nodeId);
      if (node) {
        node.fixed = false;
        node.fx = 0;
        node.fy = 0;
        // 同步到 Worker：释放节点
        const worker = workerRef.current;
        if (worker) {
          worker.postMessage({
            type: "update",
            payload: {
              nodeIdx: node.idx,
              x: node.x,
              y: node.y,
              fixed: false,
              vx: 0,
              vy: 0,
            },
          } as WorkerMessage);
        }
      }
      dragRef.current = null;

      // 拖拽结束后保存布局到 localStorage
      if (wikiIdRef.current) {
        saveLayout(wikiIdRef.current, physNodesRef.current, cameraRef.current);
      }
    }
    panRef.current = null;
  }, []);

  const handleMouseLeave = useCallback(() => {
    hoverNodeRef.current = null;
    hoverClusterRef.current = null;
    mouseScreenRef.current = { x: 0, y: 0, active: false };
    tooltipVisibleRef.current = false;
    setTooltipNodeIdState(null);
    onNodeHover?.(null);
    if (dragRef.current) {
      const node = posMapRef.current.get(dragRef.current!.nodeId);
      if (node) {
        node.fixed = false;
        const worker = workerRef.current;
        if (worker) {
          worker.postMessage({
            type: "update",
            payload: {
              nodeIdx: node.idx,
              x: node.x,
              y: node.y,
              fixed: false,
              vx: 0,
              vy: 0,
            },
          } as WorkerMessage);
        }
      }
      dragRef.current = null;
    }
    panRef.current = null;
  }, [onNodeHover]);

  const handleWheel = useCallback((e: React.WheelEvent<HTMLCanvasElement>) => {
    // 注意：React 的 onWheel 是 passive 事件，不能调用 preventDefault
    // 阻止默认滚动已通过原生非被动监听实现（见 useEffect 中的 wheel 监听）
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

  // 原生非被动 touchmove 监听：React 的 onTouchMove 为被动模式，preventDefault 无效
  // 阻止触摸滚动，让画布可以处理拖拽和缩放手势
  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) { return; }
    const preventTouchMove = (e: TouchEvent) => {
      e.preventDefault();
    };
    canvas.addEventListener("touchmove", preventTouchMove, { passive: false });
    return () => canvas.removeEventListener("touchmove", preventTouchMove);
  }, []);

  // ── 触摸事件处理 ──
  const touchStateRef = useRef<{
    lastDist?: number;
    startX?: number;
    startY?: number;
    camX?: number;
    camY?: number;
  }>({});

  // N7 修复：移动端长按 500ms 触发上下文菜单（等价桌面端右键 onContextMenu）
  const longPressTimerRef = useRef<number | null>(null);
  const cancelLongPress = useCallback(() => {
    if (longPressTimerRef.current !== null) {
      window.clearTimeout(longPressTimerRef.current);
      longPressTimerRef.current = null;
    }
  }, []);
  // 组件卸载时清理未触发的长按定时器
  useEffect(() => cancelLongPress, [cancelLongPress]);

  const handleTouchStart = useCallback((e: React.TouchEvent<HTMLCanvasElement>) => {
    if (e.touches.length === 1) {
      const touch = e.touches[0];
      const rect = canvasRef.current!.getBoundingClientRect();
      const sx = touch.clientX - rect.left;
      const sy = touch.clientY - rect.top;
      const nodeId = findNodeAt(sx, sy);

      if (nodeId) {
        suppressAutoFocusRef.current = true;
        const node = posMapRef.current.get(nodeId);
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

      // 长按 500ms 后在起始位置检测节点并呼出上下文菜单；
      // 期间移动超过阈值或抬起/第二根手指按下都会取消（见 move/end 处理）
      cancelLongPress();
      longPressTimerRef.current = window.setTimeout(() => {
        longPressTimerRef.current = null;
        const st = touchStateRef.current;
        if (st.startX === undefined || st.startY === undefined) { return; }
        const lpRect = canvasRef.current?.getBoundingClientRect();
        if (!lpRect) { return; }
        const lpNodeId = findNodeAt(st.startX - lpRect.left, st.startY - lpRect.top);
        if (!lpNodeId) { return; }
        suppressAutoFocusRef.current = true;
        // 长按呼出菜单后结束按住拖拽状态，避免节点悬挂在 fixed 状态
        if (dragRef.current) {
          const dragNode = posMapRef.current.get(dragRef.current.nodeId);
          if (dragNode) {
            dragNode.fixed = false;
            dragNode.fx = 0;
            dragNode.fy = 0;
          }
          dragRef.current = null;
        }
        onContextMenu?.(lpNodeId, { x: st.startX, y: st.startY });
      }, 500);
    } else if (e.touches.length === 2) {
      // 双指缩放
      cancelLongPress();
      const t1 = e.touches[0];
      const t2 = e.touches[1];
      const dx = t1.clientX - t2.clientX;
      const dy = t1.clientY - t2.clientY;
      touchStateRef.current.lastDist = Math.sqrt(dx * dx + dy * dy);
      dragRef.current = null;
      panRef.current = null;
    }
  }, [findNodeAt, onNodeClick, onDeselect, onContextMenu, cancelLongPress]);

  const handleTouchMove = useCallback((e: React.TouchEvent<HTMLCanvasElement>) => {
    // 注意：React 的 onTouchMove 是 passive 事件，不能调用 preventDefault
    // 阻止默认滚动已通过原生非被动监听实现（见 useEffect 中的 touchmove 监听）

    if (e.touches.length === 1) {
      const touch = e.touches[0];
      const rect = canvasRef.current!.getBoundingClientRect();
      const sx = touch.clientX - rect.left;
      const sy = touch.clientY - rect.top;

      mouseScreenRef.current = { x: sx, y: sy, active: true };

      // 移动超过阈值取消长按（视为拖拽/平移手势）
      if (touchStateRef.current.startX !== undefined) {
        const movedX = Math.abs(touch.clientX - touchStateRef.current.startX);
        const movedY = Math.abs(touch.clientY - (touchStateRef.current.startY ?? 0));
        if (movedX > 10 || movedY > 10) { cancelLongPress(); }
      }
      if (dragRef.current) {
        const world = getScreenToWorld(sx, sy);
        const node = posMapRef.current.get(dragRef.current!.nodeId);
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
      cancelLongPress();
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
  }, [cancelLongPress]);

  const handleTouchEnd = useCallback((e: React.TouchEvent<HTMLCanvasElement>) => {
    cancelLongPress();
    if (dragRef.current) {
      const node = posMapRef.current.get(dragRef.current!.nodeId);
      if (node) {
        node.fixed = false;
        node.fx = 0;
        node.fy = 0;
      }
      dragRef.current = null;

      // 拖拽结束后保存布局
      if (wikiIdRef.current) {
        saveLayout(wikiIdRef.current, physNodesRef.current, cameraRef.current);
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
  }, [cancelLongPress]);

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
              const node = posMapRef.current.get(selectedNodeIdRef.current);
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
            // 切换鱼眼模式（同步 state 使工具栏按钮状态一致）
            fisheyeEnabledRef.current = !fisheyeEnabledRef.current;
            setFisheyeEnabled(fisheyeEnabledRef.current);
            break;
          case "l":
          case "L":
            // 切换聚类模式（同步 state 使工具栏按钮状态一致）
            clusterModeRef.current = !clusterModeRef.current;
            setClusterMode(clusterModeRef.current);
            break;
          case "p":
          case "P":
            // 切换粒子流动（默认关闭；同步 state 使工具栏按钮状态一致）
            particlesEnabledRef.current = !particlesEnabledRef.current;
            setParticlesEnabled(particlesEnabledRef.current);
            break;
        }
      }

      // Delete/Backspace 删除（需二次确认）
      if ((e.key === "Delete" || e.key === "Backspace") && selectedNodeIdRef.current && !isInputFocused) {
        // 阻止 Backspace 在浏览器中触发"返回上一页"，避免误操作离开图谱页
        e.preventDefault();
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
    // cluster mode 下折叠节点的位置仍是原始坐标（远离聚合质心），
    // 若参与包围盒会导致 fitAll 后聚合节点挤在角落；
    // 此模式下用聚合几何 + 未折叠节点计算包围盒
    const clusterGeoms = clusterGeomRef.current;
    const collapsed = collapsedRef.current;
    const communitiesMap = communitiesRef.current;
    const isClusterActive = clusterModeRef.current && communitiesMap && collapsed.size > 0;
    for (const n of nodes) {
      if (isClusterActive && communitiesMap) {
        const cid = communitiesMap.get(n.id);
        if (cid !== undefined && collapsed.has(cid)) {
          continue; // 折叠节点不参与包围盒
        }
      }
      if (n.x < minX) { minX = n.x; }
      if (n.y < minY) { minY = n.y; }
      if (n.x > maxX) { maxX = n.x; }
      if (n.y > maxY) { maxY = n.y; }
    }
    // 加入聚合节点的包围盒
    if (isClusterActive) {
      for (const [, geom] of clusterGeoms) {
        if (geom.cx < minX) { minX = geom.cx; }
        if (geom.cy < minY) { minY = geom.cy; }
        if (geom.cx > maxX) { maxX = geom.cx; }
        if (geom.cy > maxY) { maxY = geom.cy; }
      }
    }
    if (!isFinite(minX)) { return; }
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
    svgParts.push(`<rect width="100%" height="100%" fill="${escapeXml(token.colorBgContainer)}"/>`);

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
        `<line x1="${x1}" y1="${y1}" x2="${x2}" y2="${y2}" stroke="${
          escapeXml(em.color)
        }" stroke-width="${em.width}" opacity="0.7"/>`,
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
      svgParts.push(`<circle cx="${cx}" cy="${cy}" r="${size}" fill="${escapeXml(color)}" opacity="0.9"/>`);
      // 标签
      const label = meta.title.length > 20 ? meta.title.slice(0, 18) + "…" : meta.title;
      svgParts.push(
        `<text x="${cx}" y="${cy + size + 12}" text-anchor="middle" font-size="10" fill="${
          escapeXml(token.colorText)
        }" font-family="Inter, system-ui, sans-serif">${escapeXml(label)}</text>`,
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
      clearLayout(wikiIdRef.current);
    }

    initializePositions(nodes, dimensions.width, dimensions.height);

    // 集群力模式下，重置时同步社区质心，Worker step 会据此收敛
    const activeCommunities = effectiveCommunitiesRef.current ?? communities;
    const enableClusters = clusterModeRef.current && activeCommunities;
    const centroids = enableClusters
      ? computeCommunityCentroids(nodes, activeCommunities!)
      : undefined;
    if (enableClusters) {
      communityCentroidsRef.current = centroids!;
    }

    // 同步新布局到 Worker（避免主线程同步跑 Barnes-Hut 冻结 UI）
    const worker = workerRef.current;
    if (worker && workerInitializedRef.current) {
      const positions = new Float64Array(nodes.length * 2);
      for (let i = 0; i < nodes.length; i++) {
        positions[i * 2] = nodes[i].x;
        positions[i * 2 + 1] = nodes[i].y;
      }
      worker.postMessage({ type: "reset", payload: { positions } } as WorkerMessage);
      pendingStepRef.current = false;
    } // Worker 未就绪时：主线程短暂收敛（仅小图，避免大图卡顿——大图 Worker 几乎总是就绪）
    else if (nodes.length <= 8000) {
      const config: PhysicsConfig = {
        theta: 0.5,
        repulsion: 18000,
        gravity: 0.003,
        damping: 0.82,
        dt: 0.35,
        springForce: 0.08,
        springDamping: 0.85,
        maxVelocity: 8,
        clusterForce: enableClusters ? 0.15 : undefined,
      };
      for (let i = 0; i < 30; i++) {
        stepPhysics(
          nodes,
          physEdgesRef.current,
          config,
          undefined,
          enableClusters ? communities : undefined,
          centroids,
        );
      }
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
    const node = posMapRef.current.get(nodeId);
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
        role="application"
        aria-label={t("wiki.graph.canvasAriaLabel")}
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
              <span>
                Zoom: <span ref={statsZoomTextRef}>{cameraRef.current.zoom.toFixed(2)}×</span>
              </span>
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
            onClick={() => {
              // 用户手动切换聚类模式时，清除自动 force cluster 标志
              // 让用户的操作优先于自动行为
              isAutoForceClusterRef.current = false;
              setClusterMode((v) => !v);
            }}
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
        {/* 粒子流动 toggle（默认关闭，对齐 Obsidian 静态细边） */}
        <Tooltip title={particlesEnabled ? t("wiki.graph.particlesOn") : t("wiki.graph.particlesOff")}>
          <button
            onClick={() => setParticlesEnabled((v) => !v)}
            style={{
              ...ctrlBtnStyle,
              width: 24,
              height: 24,
              minWidth: 24,
              background: particlesEnabled ? `${token.colorPrimary}20` : "transparent",
              border: "none",
              color: particlesEnabled ? token.colorPrimary : token.colorTextSecondary,
            }}
            onMouseEnter={hoverBtnStyle}
            onMouseLeave={leaveBtnStyle}
          >
            <Sparkles size={14} />
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
          const communityId = getCommunityId(tooltipNodeIdState);
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
                <span>{t("wiki.graph.linksCount", { count: meta.linkCount })}</span>
                <span>{t("wiki.graph.backlinksCount", { count: meta.backlinkCount })}</span>
                <span>{t("wiki.graph.totalDegree", { count: meta.linkCount + meta.backlinkCount })}</span>
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
            title={minimapOpen ? t("wiki.graph.collapseMinimap") : t("wiki.graph.expandMinimap")}
            aria-label={minimapOpen ? t("wiki.graph.collapseMinimap") : t("wiki.graph.expandMinimap")}
          >
            {minimapOpen ? "▾" : "▴"}
          </button>
          {minimapOpen && (
            <canvas
              ref={minimapRef}
              width={MINIMAP_W}
              height={MINIMAP_H}
              role="application"
              aria-label={t("wiki.graph.minimapAriaLabel")}
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

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
// LRU 上限：最多保留 10 个 wiki 的布局，超出按 savedAt 时间淘汰最旧的
const LAYOUT_MAX_ENTRIES = 10;
// 单 wiki 布局超过此节点数则不持久化（避免万级节点序列化 500KB+ 逼近配额）
const LAYOUT_MAX_NODES = 2000;

interface SavedLayout {
  positions: Record<string, { x: number; y: number }>;
  savedAt: number;
}

function pruneLayoutStorage(currentWikiId: string): void {
  // 收集所有布局条目，按 savedAt 升序，超出上限时删除最旧
  const entries: Array<{ wikiId: string; savedAt: number }> = [];
  for (let i = 0; i < localStorage.length; i++) {
    const key = localStorage.key(i);
    if (!key || !key.startsWith(LAYOUT_STORAGE_PREFIX)) { continue; }
    const wid = key.slice(LAYOUT_STORAGE_PREFIX.length);
    if (wid === currentWikiId) { continue; }
    try {
      const raw = localStorage.getItem(key);
      if (!raw) { continue; }
      const layout = JSON.parse(raw) as SavedLayout;
      entries.push({ wikiId: wid, savedAt: layout.savedAt || 0 });
    } catch {
      // 损坏的条目直接删除
      localStorage.removeItem(key);
    }
  }
  entries.sort((a, b) => a.savedAt - b.savedAt);
  // 已有条目数（不含当前）+ 当前 1 个 > 上限 → 删除最旧的
  const excess = entries.length + 1 - LAYOUT_MAX_ENTRIES;
  for (let i = 0; i < excess; i++) {
    localStorage.removeItem(LAYOUT_STORAGE_PREFIX + entries[i].wikiId);
  }
}

function saveLayout(wikiId: string, nodes: PhysicsNode[]): void {
  // 节点数超阈值时不持久化（避免逼近 localStorage 配额）
  if (nodes.length > LAYOUT_MAX_NODES) { return; }
  try {
    const positions: Record<string, { x: number; y: number }> = {};
    for (const node of nodes) {
      positions[node.id] = { x: node.x, y: node.y };
    }
    const layout: SavedLayout = {
      positions,
      savedAt: Date.now(),
    };
    // 写入前做 LRU 清理，确保不超过 LAYOUT_MAX_ENTRIES
    pruneLayoutStorage(wikiId);
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

function applySavedLayout(nodes: PhysicsNode[], saved: SavedLayout): boolean {
  let matched = 0;
  for (const node of nodes) {
    const savedPos = saved.positions[node.id];
    if (savedPos) {
      node.x = savedPos.x;
      node.y = savedPos.y;
      matched++;
    }
  }
  // 匹配率低于 30% 时整体放弃：清空位置，返回 false 让 initializePositions 重新圆形布局
  if (matched < nodes.length * 0.3) {
    return false;
  }
  // 匹配率 ≥ 30% 但部分未匹配：给未匹配节点做圆形分布，避免堆叠在原点
  const unmatched = nodes.filter((n) => !saved.positions[n.id]);
  if (unmatched.length > 0) {
    const radius = Math.max(200, Math.sqrt(unmatched.length) * 30);
    unmatched.forEach((n, i) => {
      const angle = (i / unmatched.length) * Math.PI * 2;
      n.x = Math.cos(angle) * radius;
      n.y = Math.sin(angle) * radius;
    });
  }
  return true;
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

// ── XML 转义（SVG 导出防注入） ──
function escapeXml(s: string): string {
  return s
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&apos;");
}

// ── 颜色工具：支持 #RRGGBB / #RRGGBBAA / #RGB / rgb()/rgba() ──
interface RGBA {
  r: number;
  g: number;
  b: number;
  a: number;
}

function parseColor(color: string): RGBA | null {
  if (!color) { return null; }
  const hexMatch = color.match(/^#([0-9a-fA-F]{3}|[0-9a-fA-F]{6}|[0-9a-fA-F]{8})$/);
  if (hexMatch) {
    const hex = hexMatch[1];
    if (hex.length === 3) {
      return {
        r: parseInt(hex[0] + hex[0], 16),
        g: parseInt(hex[1] + hex[1], 16),
        b: parseInt(hex[2] + hex[2], 16),
        a: 255,
      };
    }
    return {
      r: parseInt(hex.slice(0, 2), 16),
      g: parseInt(hex.slice(2, 4), 16),
      b: parseInt(hex.slice(4, 6), 16),
      a: hex.length === 8 ? parseInt(hex.slice(6, 8), 16) : 255,
    };
  }
  const rgbMatch = color.match(/^rgba?\(\s*(\d+)\s*,\s*(\d+)\s*,\s*(\d+)(?:\s*,\s*([\d.]+))?\s*\)$/);
  if (rgbMatch) {
    return {
      r: parseInt(rgbMatch[1], 10),
      g: parseInt(rgbMatch[2], 10),
      b: parseInt(rgbMatch[3], 10),
      a: rgbMatch[4] !== undefined ? Math.round(parseFloat(rgbMatch[4]) * 255) : 255,
    };
  }
  return null;
}

function clamp(v: number, min: number, max: number): number {
  return Math.max(min, Math.min(max, v));
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

    // 清空 minimap 包围盒缓存（节点集已变化，旧缓存失效）
    minimapBBoxRef.current = null;

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
    let layoutApplied = false;
    if (wikiId) {
      const saved = loadLayout(wikiId);
      if (saved) {
        layoutApplied = applySavedLayout(pNodes, saved);
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
      repulsion: 6000,
      gravity: 0.01,
      damping: 0.92,
      dt: 0.25,
      springForce: 0.04,
      springDamping: 0.85,
      maxVelocity: 4,
    };

    // ── 零拷贝初始化：使用 Float64Array + Transfer List ──
    // 避免 JSON 序列化（structured clone）阻塞主线程
    // 节点布局：[x, y, vx, vy, fx, fy, mass, fixed(0/1), kind(0), idx] = 10 floats
    // 边布局：[sIdx, tIdx, restLength, stiffness, damping] = 5 floats
    const nodeCount = pNodes.length;
    const edgeCount = pEdges.length;
    const NODE_STRIDE = 10;
    const EDGE_STRIDE = 5;
    const nodeBuffer = new Float64Array(nodeCount * NODE_STRIDE);
    const edgeBuffer = new Float64Array(edgeCount * EDGE_STRIDE);
    const nodeIds: string[] = Array.from({ length: nodeCount });
    const nodeKinds: string[] = Array.from({ length: nodeCount });

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
      nodeBuffer[base + 8] = 0; // kind 存储在 nodeKinds 数组中
      nodeBuffer[base + 9] = n.idx;
      nodeIds[i] = n.id;
      nodeKinds[i] = n.kind;
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
        communities: effectiveCommunities
          ? Object.fromEntries(effectiveCommunities)
          : undefined,
        compact: {
          nodeBuffer,
          edgeBuffer,
          nodeIds,
          nodeKinds,
          nodeCount,
          edgeCount,
        },
      },
    };

    // 使用 Transfer List 实现零拷贝：ArrayBuffer 所有权直接转移到 Worker
    // 主线程零阻塞（之前用 postMessage 传递 JSON 对象时，structured clone 会阻塞数秒）
    worker.postMessage(initMsg, [nodeBuffer.buffer, edgeBuffer.buffer]);

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

    // ── 大图自动聚合：设置折叠状态、刷新几何、构建聚合物理 ──
    // （effectiveCommunities 已在 Worker 初始化前预计算好）
    // 关键修复：即使 effectiveCommunities 为 null，也要基于 effectiveCommunitiesRef.current 初始化
    if (shouldForceCluster) {
      const commForInit = effectiveCommunities ?? effectiveCommunitiesRef.current;
      if (!commForInit) {
        // 终极 fallback：基于所有节点创建哈希映射
        const fallbackMap = new Map<string, number>();
        for (const n of pNodes) {
          const hash = Math.abs(hashStringToInt(n.id)) % FORCE_CLUSTER_COUNT;
          fallbackMap.set(n.id, hash);
        }
        effectiveCommunitiesRef.current = fallbackMap;
      }

      const comm = effectiveCommunitiesRef.current;
      console.log("[GraphView] forceCluster init", {
        nodeCount: pNodes.length,
        communityCount: comm?.size ?? 0,
      });

      // 关键：同步更新 communitiesRef（buildAggregatePhysics 依赖它）
      communitiesRef.current = comm;

      const all = new Set<number>();
      if (comm) {
        for (const cid of comm.values()) {
          all.add(cid);
        }
      }
      collapsedRef.current = all;
      clusterModeRef.current = true;
      setClusterMode(true);
      refreshClusterGeom();
      buildAggregatePhysics();
      console.log("[GraphView] forceCluster init done", {
        collapsedSize: collapsedRef.current.size,
        clusterGeomSize: clusterGeomRef.current.size,
        aggPhysNotNull: aggPhysRef.current !== null,
      });
      setClusterCollapseVersion((v) => v + 1);
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

      // 预先获取聚合物理状态供 LOD 逻辑使用
      const aggPhys = aggPhysRef.current;
      const aggActive = aggPhys !== null && aggPhys.nodes.length > 0;

      // ── LOD 渐进式聚类展开：根据缩放级别自动展开/折叠社区 ──
      // 类似地图缩放：缩得越近，看到的细节越多
      if (clusterModeRef.current && aggActive) {
        const zoom = cam.zoom;
        const geom = clusterGeomRef.current;

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

          // LOD 变化导致折叠集合改变 → 重建聚合物理集
          if (newCollapsed.size !== prevCollapsedSize) {
            // 延迟一帧重建，避免在渲染循环中立即触发重计算
            setTimeout(() => {
              refreshClusterGeom();
              buildAggregatePhysics();
              setClusterCollapseVersion((v) => v + 1);
            }, 0);
          }
        }
      }

      // ── 聚合物理分支（聚类折叠模式）：物理只模拟聚合节点 + 未折叠节点 ──
      // 折叠社区成员不参与力导向模拟（数量级骤降），聚合节点坐标回写 clusterGeom，
      // 驱动折叠社区几何/聚合边/聚合节点渲染。万级节点打开不卡死的核心。
      if (aggActive) {
        const config: PhysicsConfig = {
          theta: 0.5,
          repulsion: 6000,
          gravity: 0.01,
          damping: 0.92,
          dt: 0.25,
          springForce: 0.04,
          springDamping: 0.85,
          maxVelocity: 4,
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
          // 更新网格空间索引（未折叠节点位置可能变化）
          const gridIndex = new Map<string, string[]>();
          for (const n of aggPhys.nodes) {
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
        // 聚合物理激活时不使用 Worker 结果
        workerResultRef.current = null;
      } else if (worker && workerReady && nodes.length > 0) {
        const enableClusters = clusterModeRef.current && effCommunities;

        // 拖拽时同步位置到 Worker
        if (hasDrag) {
          const dragNode = nodes.find((n) => n.id === dragRef.current!.nodeId);
          if (dragNode) {
            worker.postMessage({
              type: "update",
              payload: {
                nodeId: dragNode.id,
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
            repulsion: 6000,
            gravity: 0.01,
            damping: 0.92,
            dt: 0.25,
            springForce: 0.04,
            springDamping: 0.85,
            maxVelocity: 4,
            clusterForce: enableClusters ? 0.15 : undefined,
          };
          const centroids = enableClusters
            ? computeCommunityCentroids(nodes, effCommunities!)
            : undefined;

          worker.postMessage({
            type: "step",
            payload: {
              config,
              // communities 在 init/reset 时已同步到 Worker（nodeIdxToCommunity），
              // step 无需重复传递，避免每 12 帧一次 O(N) 序列化
              centroids: centroids ? Object.fromEntries(centroids) : undefined,
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

            // 仅在节点位置更新后才重建网格空间索引
            {
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

            // 关键修复：在 forceCluster/clusterMode 下，Worker 返回新位置后必须重建
            // clusterGeomRef.current（聚类质心/半径）和 aggPhysRef.current（聚合物理节点）。
            // 否则渲染时使用的是初始位置计算的旧几何数据，所有聚类看起来重叠在一起。
            if (clusterModeRef.current || nodes.length > AUTO_CLUSTER_THRESHOLD) {
              refreshClusterGeom();
              buildAggregatePhysics();
            }

            // 大图位图缓存：Worker 返回新结果时重建（节点位置已更新）
            // 仅在非交互状态下重建，避免与拖拽冲突
            if (nodes.length > FORCE_BITMAP_THRESHOLD && !hasInteraction) {
              spriteCacheRef.current = buildBigGraphSpriteCache(nodes);
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
            enableClusters ? effCommunities : undefined,
            enableClusters ? centroids : undefined,
            neighborMapCacheRef.current,
          );
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
      if (
        clusterModeRef.current && communities && collapsedRef.current.size === 0
        && communities.size <= MAX_AGG_PHYS_NODES
        && frameCounterRef.current % 5 === 0
      ) {
        drawClusterRegions(ctx, nodes);
      }

      // 聚合几何低频刷新（6 帧一次；切换展开/收起时立即刷新）
      // 系统稳定时跳过（节点位置不变，cluster geom 也无需重算，节省 O(N) 遍历）
      // 聚合物理激活时跳过：折叠社区的 cx/cy 由聚合物理节点回写驱动，避免被质心覆盖
      const forceCluster = nodes.length > AUTO_CLUSTER_THRESHOLD;
      if (
        (clusterModeRef.current || forceCluster) && !aggActive && frameCounterRef.current % 6 === 0
        && idleCounterRef.current < 30
      ) {
        refreshClusterGeom();
      }

      // 绘制（传入视口范围用于裁剪）
      // Worker 未就绪的大图：跳过完整渲染，避免主线程 fallback 卡死
      if (shouldRender && !workerNotReadyLargeGraph) {
        // 强制聚类：节点数 > 3000 时自动进入聚类渲染模式
        if (aggActive || clusterModeRef.current || forceCluster) {
          // ── 聚类模式：极简渲染策略 ──
          // 全折叠时只画小型聚类标记 + 聚合边
          // 展开社区时才画内部节点
          const activeCommunities = effectiveCommunitiesRef.current ?? communities;
          const totalCommunities = activeCommunities ? new Set(activeCommunities.values()).size : 0;
          // forceCluster 模式下如果没有折叠状态，强制全折叠
          const allCollapsed = forceCluster
            ? totalCommunities > 0
            : collapsedRef.current.size >= totalCommunities && totalCommunities > 0;
          const isLargeGraph = nodes.length > 5000;

          if (allCollapsed) {
            // ── 全折叠：只画聚类标记（最大15px）+ 聚合边 ──
            const geom = clusterGeomRef.current;
            const aggPhysLocal = aggPhysRef.current;
            if (frameCounterRef.current % 60 === 0) {
              console.log("[GraphView] forceCluster render state", {
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
              });
            }
            if (geom.size > 0) {
              // 聚合边
              if (aggPhysLocal && aggPhysLocal.edges.length > 0) {
                ctx.save();
                ctx.strokeStyle = token.colorBorder;
                ctx.lineWidth = 0.5;
                ctx.globalAlpha = 0.5;
                ctx.beginPath();
                for (const e of aggPhysLocal.edges) {
                  const sNode = aggPhysLocal.nodes[e.sourceIdx];
                  const tNode = aggPhysLocal.nodes[e.targetIdx];
                  if (!sNode || !tNode) { continue; }
                  if (
                    !isInView(sNode.x, sNode.y, viewWorld, 30) || !isInView(tNode.x, tNode.y, viewWorld, 30)
                  ) { continue; }
                  ctx.moveTo(sNode.x, sNode.y);
                  ctx.lineTo(tNode.x, tNode.y);
                }
                ctx.stroke();
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
                    console.log("[GraphView] cluster skipped (not collapsed)", { cid });
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
                // 标签
                if (cam.zoom >= 0.3) {
                  ctx.globalAlpha = 0.9;
                  ctx.font = `${Math.max(10, Math.round(11 * cam.zoom + 2))}px Inter, system-ui, sans-serif`;
                  ctx.textAlign = "center";
                  ctx.textBaseline = "top";
                  ctx.fillStyle = token.colorText;
                  ctx.fillText(`${g.label} (${g.count})`, g.cx, g.cy + maxR + 2);
                }
              }
              if (frameCounterRef.current % 60 === 0) {
                console.log("[GraphView] cluster draw stats", {
                  totalGeom: geom.size,
                  drawn: drawnClusters,
                  skipped: skippedClusters,
                  collapsedSize: collapsedRef.current.size,
                });
              }
              ctx.restore();
            }
          } else {
            // ── 部分展开：绘制展开社区的节点和边 ──
            if (activeCommunities) {
              drawExpandedCommunity(ctx, nodes, viewWorld, activeCommunities, isLargeGraph);
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
  function drawClusterRegions(ctx: CanvasRenderingContext2D, nodes: PhysicsNode[]) {
    const activeCommunities = effectiveCommunitiesRef.current ?? communities;
    if (!activeCommunities) { return; }
    const centroids = communityCentroidsRef.current;
    if (centroids.size === 0) { return; }

    // 按社区分组收集节点位置
    const communityNodes = new Map<number, { sx: number; sy: number }[]>();
    for (const node of nodes) {
      const cid = activeCommunities.get(node.id);
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

  // 构建大图位图缓存：将所有节点/边预渲染到离屏 Canvas
  // 万级节点下每帧 5 万+ 矢量操作是卡死根因，位图模式将其降为 1 次 drawImage
  function buildBigGraphSpriteCache(nodes: PhysicsNode[]): HTMLCanvasElement | null {
    if (nodes.length === 0) { return null; }

    // 计算节点分布 bounding box —— 聚类模式下只计算可见节点
    const clusterActive = clusterModeRef.current;
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
    const MAX_CANVAS = 16384;
    const scale = Math.min(1, MAX_CANVAS / Math.max(worldW, worldH));
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
      if (clusterModeRef.current) {
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
    // 关键修复：在 forceCluster/aggActive 模式下，gridIndex 只包含聚合节点 ID（__agg__*），
    // 不包含原始节点 ID，导致无法通过网格索引找到展开社区的原始节点。
    // 解决方案：当 gridIndex 查找失败时，使用 posMap 作为 fallback 数据源。
    const expandedNodeIds = new Set<string>();

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
            const cid = activeCommunities.get(id);
            if (cid !== undefined && !collapsedSet.has(cid)) {
              expandedNodeIds.add(id);
            }
          }
        }
      }
    }

    // Fallback：在 forceCluster/aggActive 模式下，gridIndex 可能只包含聚合节点 ID。
    // 如果网格索引查找结果为空，改用 posMap 遍历所有节点（O(N) 但保证正确性）。
    // 这是必要的性能-正确性权衡：大图模式下节点可见性比性能更重要。
    if (expandedNodeIds.size === 0 && posMap.size > 0) {
      for (const [id, node] of posMap) {
        if (!isInView(node.x, node.y, viewWorld, 20)) { continue; }
        const cid = activeCommunities.get(id);
        if (cid !== undefined && !collapsedSet.has(cid)) {
          expandedNodeIds.add(id);
        }
      }
    }

    if (expandedNodeIds.size === 0) { return; }

    // 降采样：大图只画部分节点
    const nodeSampleRate = isLargeGraph ? 0.5 : 1.0;
    const visibleNodes: { id: string; x: number; y: number; size: number; color: string }[] = [];

    for (const id of expandedNodeIds) {
      if (nodeSampleRate < 1 && Math.random() > nodeSampleRate) { continue; }
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
      for (const node of visibleNodes) {
        const meta = nodeMetaRef.current.get(node.id);
        if (!meta) { continue; }
        const title = meta.title.length > 18 ? meta.title.slice(0, 16) + "…" : meta.title;
        const fontSize = Math.max(9, Math.min(12, Math.round(10 * zoom + 2)));
        ctx.font = `${fontSize}px Inter, system-ui, sans-serif`;
        ctx.globalAlpha = 0.85;
        ctx.fillStyle = token.colorText;
        ctx.fillText(title, node.x, node.y + node.size + 3);
      }
      ctx.globalAlpha = 1;
      ctx.restore();
    }

    // 绘制边（只连接展开社区的节点）
    if (edgeMeta.length > 0 && visibleNodes.length > 1) {
      const idSet = new Set(visibleNodes.map(n => n.id));
      const edgeSampleRate = isLargeGraph ? 0.3 : 0.7;

      ctx.save();
      ctx.strokeStyle = token.colorBorder;
      ctx.lineWidth = 0.4;
      ctx.globalAlpha = 0.4;

      // 批量路径
      const batchPaths = new Map<string, Path2D>();

      for (let i = 0; i < edgeMeta.length; i++) {
        const em = edgeMeta[i];
        if (!idSet.has(em.source) || !idSet.has(em.target)) { continue; }
        if (edgeSampleRate < 1 && (i % Math.round(1 / edgeSampleRate)) !== 0) { continue; }

        const sNode = posMap.get(em.source);
        const tNode = posMap.get(em.target);
        if (!sNode || !tNode) { continue; }
        if (!isInView(sNode.x, sNode.y, viewWorld, 10) && !isInView(tNode.x, tNode.y, viewWorld, 10)) { continue; }

        const key = `${em.color}|${em.width}`;
        let path = batchPaths.get(key);
        if (!path) {
          path = new Path2D();
          batchPaths.set(key, path);
        }
        path.moveTo(sNode.x, sNode.y);
        path.lineTo(tNode.x, tNode.y);
      }

      // 绘制批量路径
      for (const [key, path] of batchPaths) {
        const [color, width] = key.split("|");
        ctx.strokeStyle = color;
        ctx.lineWidth = parseFloat(width);
        ctx.stroke(path);
      }
      ctx.globalAlpha = 1;
      ctx.restore();
    }
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

    // 大图边数量保护：超过 50000 条边时，限制处理数量避免主线程阻塞
    // 交互时（hover/选中）恢复全量，确保用户能看到所有相关边
    const totalEdges = edgeMeta.length;
    const hasActiveInteraction = hovered || !!selected;
    const edgeLimit = totalEdges > 50000 && !hasActiveInteraction ? 30000 : totalEdges;

    const hasCommunityFilter = hasCommunityFilterRef.current;

    // 批量描边：普通边按 (颜色, 线宽) 合并到 Path2D，最后统一 stroke。
    // 万级边场景下从「每边一次 stroke」降为「每样式一次 stroke」，是最大的性能收益。
    const batchPaths = new Map<string, { path: Path2D; color: string; width: number }>();

    for (let i = 0; i < edgeLimit; i++) {
      const em = edgeMeta[i];

      if (!visibleTypes.has(em.type)) { continue; }

      // 直接数组访问，避免 Map 查找
      const sNode = nodes[em.sourceIdx];
      const tNode = nodes[em.targetIdx];
      if (!sNode || !tNode) { continue; }

      // 聚类折叠模式：折叠社区的成员端点接到聚合节点质心
      const sCid = getCommunityId(em.source);
      const tCid = getCommunityId(em.target);
      const sCollapsed = clusterModeRef.current && sCid !== undefined && collapsedRef.current.has(sCid);
      const tCollapsed = clusterModeRef.current && tCid !== undefined && collapsedRef.current.has(tCid);
      const sGeom = sCollapsed ? clusterGeomRef.current.get(sCid!) : undefined;
      const tGeom = tCollapsed ? clusterGeomRef.current.get(tCid!) : undefined;
      const s: { x: number; y: number } = sGeom ? { x: sGeom.cx, y: sGeom.cy } : sNode;
      const t: { x: number; y: number } = tGeom ? { x: tGeom.cx, y: tGeom.cy } : tNode;

      // 视口裁剪：两端都不在视口内时跳过
      if (!isInView(s.x, s.y, viewWorld) && !isInView(t.x, t.y, viewWorld)) { continue; }

      if (hasCommunityFilter) {
        const sVisible = sCid === undefined || visibleCommunitiesSet.has(sCid);
        const tVisible = tCid === undefined || visibleCommunitiesSet.has(tCid);
        if (!sVisible || !tVisible) { continue; }
      }

      const isRelevant = hovered && (em.source === hovered || em.target === hovered)
        || selected && (em.source === selected || em.target === selected);

      // 低缩放下简化渲染
      if (zoom < 0.3 && !isRelevant) { continue; }

      if (isRelevant) {
        // 相关边（hover/选中邻居）逐条绘制：使用贝塞尔曲线 + 鱼眼线宽
        const sScale = fisheyeScale(s.x, s.y, fisheye);
        const tScale = fisheyeScale(t.x, t.y, fisheye);
        const avgScale = (sScale + tScale) / 2;
        ctx.beginPath();
        ctx.moveTo(s.x, s.y);
        // 贝塞尔曲线：控制点为中点 + 垂直偏移
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
        ctx.lineWidth = em.width * 1.5 * avgScale;
        ctx.globalAlpha = 0.9;
        ctx.stroke();
      } else {
        // 普通边收集到批量路径：小图用贝塞尔曲线，大图用直线
        const key = `${em.color}|${em.width}`;
        let entry = batchPaths.get(key);
        if (!entry) {
          entry = { path: new Path2D(), color: em.color, width: em.width };
          batchPaths.set(key, entry);
        }
        if (nodes.length < 5000) {
          // 小图：贝塞尔曲线
          const dx = t.x - s.x;
          const dy = t.y - s.y;
          const mx = (s.x + t.x) / 2;
          const my = (s.y + t.y) / 2;
          const curveAmount = Math.min(20, Math.sqrt(dx * dx + dy * dy) * 0.1);
          const len = Math.sqrt(dx * dx + dy * dy) || 1;
          const cpX = mx + (-dy / len) * curveAmount;
          const cpY = my + (dx / len) * curveAmount;
          entry.path.moveTo(s.x, s.y);
          entry.path.quadraticCurveTo(cpX, cpY, t.x, t.y);
        } else {
          // 大图：直线（性能优先）
          entry.path.moveTo(s.x, s.y);
          entry.path.lineTo(t.x, t.y);
        }
      }
    }

    // 批量 stroke：普通边统一低透明度；hover/选中时非邻居边近乎消失（对齐 Obsidian 的彻底淡出）
    // 鱼眼激活时：对批量边宽度应用 fisheye 中心处的全局 scale，
    // 避免与逐条绘制的相关边形成明显线宽断层（性能与一致性的折中）
    if (batchPaths.size > 0) {
      ctx.globalAlpha = (hovered || selected) ? 0.08 : 0.25;
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
    for (const nodeId of visibleNodeIds) {
      const node = nodeMap.get(nodeId);
      if (!node) { continue; }

      // 聚类折叠模式：折叠社区的节点由聚合节点替代，不单独绘制
      if (clusterModeRef.current) {
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
      console.log("[GraphView] buildAggregatePhysics early return", {
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
    console.log("[GraphView] buildAggregatePhysics success", {
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
        console.log("[GraphView] refreshClusterGeom early return", {
          activeCommunitiesNull: activeCommunities === null,
          clusterMode: clusterModeRef.current,
          isForceCluster,
        });
      }
      clusterGeomRef.current = new Map();
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
    if (frameCounterRef.current % 60 === 0) {
      console.log("[GraphView] refreshClusterGeom success", {
        bucketCount: buckets.size,
        nextSize: next.size,
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
  }, [findNodeAt, findClusterAt, toggleCluster, onNodeClick, onDeselect]);

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
      const node = physNodesRef.current.find((n) => n.id === dragRef.current!.nodeId);
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
              nodeId: node.id,
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
        saveLayout(wikiIdRef.current, physNodesRef.current);
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
      const node = physNodesRef.current.find((n) => n.id === dragRef.current!.nodeId);
      if (node) {
        node.fixed = false;
        const worker = workerRef.current;
        if (worker) {
          worker.postMessage({
            type: "update",
            payload: {
              nodeId: node.id,
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
      localStorage.removeItem(LAYOUT_STORAGE_PREFIX + wikiIdRef.current);
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
        repulsion: 6000,
        gravity: 0.01,
        damping: 0.92,
        dt: 0.25,
        springForce: 0.04,
        springDamping: 0.85,
        maxVelocity: 4,
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

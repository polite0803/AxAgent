// SPDX-License-Identifier: AGPL-3.0-only
/**
 * Wiki 图谱视图：基于 sigma.js (WebGL) + graphology 渲染。
 *
 * 设计目标：支持 10 万节点流畅交互。
 * - 节点：WebGL 圆点 + 标签（LOD 自动控制密度）
 * - 边：WebGL 线段
 * - 布局：ForceAtlas2 在 Web Worker 中计算，不阻塞主线程
 * - 交互：单击/双击/右键/hover/缩放/平移/聚焦
 * - 大图降级：> 5000 节点自动关闭标签渲染，> 10000 节点启用 Barnes-Hut
 */

import { Tooltip } from "@/components/layout/Tooltip";
import type { LayoutRequest, WorkerOutbound } from "@/components/wiki/graphLayout.worker";
import {
  Button,
  Card,
  Empty,
  Input,
  Popover,
  Segmented,
  Select,
  Space,
  Tag,
  theme,
  Tooltip as AntTooltip,
  Typography,
} from "antd";
import Graph from "graphology";
import {
  Download,
  Fullscreen,
  Info,
  Maximize2,
  Minimize2,
  RefreshCw,
  Search,
  SlidersHorizontal,
  ZoomIn,
  ZoomOut,
} from "lucide-react";
import {
  type CSSProperties,
  memo,
  type MouseEvent as ReactMouseEvent,
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import { useTranslation } from "react-i18next";
import Sigma from "sigma";

// ─────────────────────────────────────────────────────────────────────────────
// 公共类型（保持向后兼容，外部文件 WikiGraphPage / WikiDetailPanel 依赖）
// ─────────────────────────────────────────────────────────────────────────────

export type GraphNodeType = "note" | "concept" | "entity" | "source";

export type GraphEdgeType =
  | "link"
  | "backlink"
  | "reference"
  | "derived_from"
  | "contradicts";

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
  onNodeClick?: (nodeId: string) => void;
  onNodeDoubleClick?: (nodeId: string) => void;
  onNodeHover?: (nodeId: string | null) => void;
  onContextMenu?: (nodeId: string, position: { x: number; y: number }) => void;
  onDeleteNode?: (nodeId: string) => void;
  onDeselect?: () => void;
  highlightedNodeIds?: Set<string>;
  selectedNodeId?: string | null;
  filters?: {
    tags?: string[];
    pathPrefix?: string;
    types?: GraphNodeType[];
  };
  onFiltersChange?: (filters: {
    tags?: string[];
    types?: GraphNodeType[];
  }) => void;
  showMinimap?: boolean;
  communities?: Map<string, number>;
}

// ─────────────────────────────────────────────────────────────────────────────
// 常量与工具
// ─────────────────────────────────────────────────────────────────────────────

type TokenType = ReturnType<typeof theme.useToken>["token"];

// 性能阈值
const LAYOUT_FORCE_THRESHOLD = 200; // 小图直接同步 ForceAtlas2，不丢 worker 开销
const LABEL_RENDER_THRESHOLD = 5000; // 超过此数量关闭默认标签渲染
const BARNES_HUT_THRESHOLD = 10000; // 超过此数量启用 Barnes-Hut 优化
const FORCE_ATLAS_ITERATIONS = 150; // 默认迭代次数（增加到 150 让布局更稳定）

// Obsidian 风格的鲜明节点配色
const getNodeColorMap = (token: TokenType): Record<GraphNodeType, string> => ({
  note: token.colorPrimary,
  concept: token.colorSuccess,
  entity: "#FA8C16",
  source: "#EB2F96",
});

// 更有辨识度的社区调色板
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

// Obsidian 风格的边样式：link 更细更淡，reference 更鲜明
const getEdgeTypeStylesMap = (token: TokenType): Record<
  GraphEdgeType,
  { color: string; width: number }
> => ({
  link: { color: token.colorBorderSecondary, width: 0.8 },
  backlink: { color: token.colorPrimary, width: 1.2 },
  reference: { color: "#52C41A", width: 1.8 },
  derived_from: { color: "#FA8C16", width: 1.5 },
  contradicts: { color: token.colorError, width: 2 },
});

const edgeTypeLabels: Record<GraphEdgeType, string> = {
  link: "wiki.graph.edgeType.link",
  backlink: "wiki.graph.edgeType.backlink",
  reference: "wiki.graph.edgeType.reference",
  derived_from: "wiki.graph.edgeType.derived",
  contradicts: "wiki.graph.edgeType.contradicts",
};

function resolveColor(c: string): string {
  // 简单 var() 解析：sigma 不支持 CSS 变量，这里回退到固定值
  if (c.startsWith("var(--orange")) { return "#fa8c16"; }
  if (c.startsWith("var(--magenta")) { return "#eb2f96"; }
  return c;
}

function getNodeColor(
  node: GraphNode,
  communities: Map<string, number> | undefined,
  token: TokenType | undefined,
): string {
  if (communities && communities.has(node.id)) {
    const cid = communities.get(node.id)!;
    return communityPalette[cid % communityPalette.length];
  }
  const map = token ? getNodeColorMap(token) : {
    note: "#1890ff",
    concept: "#52c41a",
    entity: "#fa8c16",
    source: "#eb2f96",
  };
  return resolveColor(map[node.type] || map.note);
}

function getNodeSize(node: GraphNode): number {
  const degree = node.linkCount + node.backlinkCount;
  // Obsidian 风格：节点大小范围更大，entity 节点明显更大
  if (node.type === "entity") {
    return Math.max(6, Math.min(20, 6 + degree * 0.8));
  }
  if (node.type === "concept") {
    return Math.max(5, Math.min(16, 5 + degree * 0.6));
  }
  return Math.max(4, Math.min(14, 4 + degree * 0.4));
}

/** 网格布局：O(n) 同步，用于 hierarchy 模式或 worker 启动前的初始布局 */
function gridLayout(
  nodes: GraphNode[],
  width: number,
  height: number,
): Map<string, { x: number; y: number }> {
  const positions = new Map<string, { x: number; y: number }>();
  if (nodes.length === 0) { return positions; }
  const aspect = width / Math.max(height, 1);
  const cols = Math.max(1, Math.ceil(Math.sqrt(nodes.length * aspect)));
  const rows = Math.ceil(nodes.length / cols);
  const cellW = width / cols;
  const cellH = height / rows;
  nodes.forEach((node, i) => {
    const col = i % cols;
    const row = Math.floor(i / cols);
    positions.set(node.id, {
      x: col * cellW + cellW / 2 - width / 2,
      y: row * cellH + cellH / 2 - height / 2,
    });
  });
  return positions;
}

/**
 * 圆形布局：节点按同心环排列，环数由 sqrt(n) 决定。
 * - 中心 1 个节点（枢纽感）
 * - 每环节点数随环号递增（环 r 上放 6*r 个节点，r 从 1 开始）
 * - 半径基准取容器短边 * 0.4，保证不超出视口
 * 与 ForceAtlas2 的 linLogMode 不同，这里产出的是规整的同心圆。
 */
function circularLayout(
  nodes: GraphNode[],
  width: number,
  height: number,
): Map<string, { x: number; y: number }> {
  const positions = new Map<string, { x: number; y: number }>();
  if (nodes.length === 0) { return positions; }
  if (nodes.length === 1) {
    positions.set(nodes[0].id, { x: 0, y: 0 });
    return positions;
  }
  // 容器短边作为基准半径，留 20% 边距
  const baseRadius = Math.min(width, height) * 0.4;
  // 中心放第一个节点
  positions.set(nodes[0].id, { x: 0, y: 0 });
  // 其余节点分环排列：环 r 放 6*r 个节点
  let placed = 1;
  let ring = 1;
  while (placed < nodes.length) {
    const slots = 6 * ring;
    const remaining = nodes.length - placed;
    const onThisRing = Math.min(slots, remaining);
    const radius = baseRadius * (ring / Math.max(1, Math.ceil(Math.sqrt(nodes.length / 6))));
    // 角度从 -π/2（正上方）开始，顺时针分布
    for (let i = 0; i < onThisRing; i++) {
      const angle = -Math.PI / 2 + (i / onThisRing) * Math.PI * 2;
      const node = nodes[placed];
      if (!node) { break; }
      positions.set(node.id, {
        x: Math.cos(angle) * radius,
        y: Math.sin(angle) * radius,
      });
      placed++;
    }
    ring++;
  }
  return positions;
}

/** 计算节点群包围盒，用于相机居中 */
function computeBoundingBox(
  positions: Array<{ x: number; y: number }>,
): { centerX: number; centerY: number; width: number; height: number } | null {
  if (positions.length === 0) { return null; }
  let minX = Infinity;
  let minY = Infinity;
  let maxX = -Infinity;
  let maxY = -Infinity;
  for (const p of positions) {
    if (p.x < minX) { minX = p.x; }
    if (p.y < minY) { minY = p.y; }
    if (p.x > maxX) { maxX = p.x; }
    if (p.y > maxY) { maxY = p.y; }
  }
  return {
    centerX: (minX + maxX) / 2,
    centerY: (minY + maxY) / 2,
    width: maxX - minX,
    height: maxY - minY,
  };
}

/** 计算连通分量统计 */
function computeComponentStats(
  nodeIds: string[],
  edges: Array<{ source: string; target: string }>,
): { components: number; largestSize: number; avgDegree: string } {
  if (nodeIds.length === 0) {
    return { components: 0, largestSize: 0, avgDegree: "0" };
  }
  const adj = new Map<string, Set<string>>();
  for (const id of nodeIds) { adj.set(id, new Set()); }
  for (const e of edges) {
    adj.get(e.source)?.add(e.target);
    adj.get(e.target)?.add(e.source);
  }
  let components = 0;
  let largestSize = 0;
  const visited = new Set<string>();
  for (const id of nodeIds) {
    if (visited.has(id)) { continue; }
    components++;
    let size = 0;
    const queue = [id];
    visited.add(id);
    for (let i = 0; i < queue.length; i++) {
      size++;
      for (const nb of adj.get(queue[i]) || []) {
        if (!visited.has(nb)) {
          visited.add(nb);
          queue.push(nb);
        }
      }
    }
    if (size > largestSize) { largestSize = size; }
  }
  const avgDegree = edges.length > 0
    ? ((edges.length * 2) / nodeIds.length).toFixed(1)
    : "0";
  return { components, largestSize, avgDegree };
}

// ─────────────────────────────────────────────────────────────────────────────
// 主组件
// ─────────────────────────────────────────────────────────────────────────────

function GraphViewInner({
  data,
  onNodeClick,
  onNodeDoubleClick,
  onNodeHover,
  onContextMenu,
  onDeleteNode,
  onDeselect,
  highlightedNodeIds,
  selectedNodeId,
  filters,
  onFiltersChange,
  showMinimap: _showMinimap = true,
  communities,
}: GraphViewProps) {
  const { token } = theme.useToken();
  const { t } = useTranslation();

  const containerRef = useRef<HTMLDivElement>(null);
  const sigmaRef = useRef<Sigma | null>(null);
  const graphRef = useRef<Graph | null>(null);
  const workerRef = useRef<Worker | null>(null);
  const workerBusyRef = useRef(false);
  const nodeIndexRef = useRef<Map<string, GraphNode>>(new Map());
  // hover 节点：用于 nodeReducer 视觉反馈（不触发 React 重渲染，性能更好）
  const hoveredNodeRef = useRef<string | null>(null);
  // 位置动画 RAF 句柄：用于取消未完成的动画
  const positionAnimRef = useRef<number | null>(null);
  // 入场动画 RAF 句柄
  const enterAnimRef = useRef<number | null>(null);
  // 选中节点 ref：nodeReducer 通过它读取最新选中态，避免闭包失效
  const selectedNodeIdRef = useRef<string | null>(null);
  // 高亮集合 ref：搜索/外部高亮时用于淡化非命中节点
  const highlightSetRef = useRef<Set<string> | undefined>(undefined);

  const [dimensions, setDimensions] = useState({ width: 800, height: 600 });
  const [layoutMode, setLayoutMode] = useState<LayoutMode>("radial");
  const [layoutRunning, setLayoutRunning] = useState(false);
  const [stats, setStats] = useState({ visible: 0, total: 0, edges: 0 });
  const [searchTerm, setSearchTerm] = useState("");
  const [hiddenEdgeTypes, setHiddenEdgeTypes] = useState<Set<GraphEdgeType>>(new Set());
  const [isFullscreen, setIsFullscreen] = useState(false);
  const [filtersOpen, setFiltersOpen] = useState(false);
  const [statsOpen, setStatsOpen] = useState(false);
  const [legendOpen, setLegendOpen] = useState(false);

  const nodeColors = useMemo(() => getNodeColorMap(token), [token]);

  const componentStats = useMemo(
    () => computeComponentStats(data.nodes.map((n) => n.id), data.edges),
    [data],
  );
  const edgeTypeStyles = useMemo(() => getEdgeTypeStylesMap(token), [token]);

  const isLargeGraph = data.nodes.length > LABEL_RENDER_THRESHOLD;
  const useBarnesHut = data.nodes.length > BARNES_HUT_THRESHOLD;

  // ── 过滤 ──
  const filteredNodes = useMemo(() => {
    return data.nodes.filter((node) => {
      if (filters?.tags?.length && !node.tags.some((ft) => filters.tags!.includes(ft))) {
        return false;
      }
      if (filters?.pathPrefix && !node.path.startsWith(filters.pathPrefix)) {
        return false;
      }
      if (filters?.types?.length && !filters.types.includes(node.type)) {
        return false;
      }
      return true;
    });
  }, [data.nodes, filters]);

  const visibleNodeIds = useMemo(
    () => new Set(filteredNodes.map((n) => n.id)),
    [filteredNodes],
  );

  const filteredEdges = useMemo(
    () =>
      data.edges.filter(
        (e) =>
          visibleNodeIds.has(e.source)
          && visibleNodeIds.has(e.target)
          && !hiddenEdgeTypes.has(e.type),
      ),
    [data.edges, visibleNodeIds, hiddenEdgeTypes],
  );

  const allTags = useMemo(() => {
    const tags = new Set<string>();
    data.nodes.forEach((n) => n.tags.forEach((ft) => tags.add(ft)));
    return Array.from(tags).sort();
  }, [data.nodes]);

  // ── 容器尺寸 ──
  useEffect(() => {
    const el = containerRef.current;
    if (!el) { return; }
    const update = () => setDimensions({ width: el.clientWidth, height: el.clientHeight });
    update();
    const ro = new ResizeObserver(update);
    ro.observe(el);
    return () => ro.disconnect();
  }, []);

  // ── 全屏 ──
  useEffect(() => {
    const handle = () => setIsFullscreen(!!document.fullscreenElement);
    document.addEventListener("fullscreenchange", handle);
    return () => document.removeEventListener("fullscreenchange", handle);
  }, []);

  // ── 初始化 sigma + graphology ──
  useEffect(() => {
    const el = containerRef.current;
    if (!el) { return; }

    const graph = new Graph({ multi: false, type: "undirected" });
    graphRef.current = graph;

    const sigmaInstance = new Sigma(graph, el, {
      renderLabels: !isLargeGraph,
      renderEdgeLabels: false,
      defaultNodeColor: token.colorPrimary,
      defaultEdgeColor: token.colorBorderSecondary,
      // Obsidian 风格：更智能的标签 LOD
      labelDensity: 0.05,
      labelGridCellSize: 80,
      labelRenderedSizeThreshold: 4,
      labelFont: "Inter, system-ui, sans-serif",
      labelSize: 11,
      labelColor: { color: token.colorText },
      edgeLabelSize: 10,
      minCameraRatio: 0.02,
      maxCameraRatio: 10,
    });
    sigmaRef.current = sigmaInstance;

    // ── nodeReducer：Obsidian 风格的 hover/选中效果 ──
    // 选中节点：大幅放大 + 脉冲发光
    // hover 节点：适度放大 + 轻微高亮
    // 非相关节点：淡化 + 隐藏标签
    sigmaInstance.setSetting("nodeReducer", (node, data) => {
      const res = { ...data };
      const hovered = hoveredNodeRef.current;
      const selected = selectedNodeIdRef.current;
      const highlight = highlightSetRef.current;
      const hasHighlight = highlight && highlight.size > 0;
      const g = sigmaInstance.getGraph();

      // 收集与 hover/selected 节点相邻的节点
      const connectedToHovered = new Set<string>();
      const connectedToSelected = new Set<string>();
      if (hovered && g.hasNode(hovered)) {
        g.forEachNeighbor(hovered, (id) => connectedToHovered.add(id));
      }
      if (selected && g.hasNode(selected)) {
        g.forEachNeighbor(selected, (id) => connectedToSelected.add(id));
      }

      // 选中节点效果
      if (selected === node) {
        res.size = data.size * 1.8;
        res.color = `${resolveColor(data.color)}FF`;
        res.highlighted = true;
        res.label = nodeIndexRef.current.get(node)?.title ?? data.label;
      } // 选中节点的邻居：轻微高亮
      else if (selected && connectedToSelected.has(node)) {
        res.size = data.size * 1.3;
        res.color = resolveColor(data.color);
      } // hover 节点效果
      else if (hovered === node) {
        res.size = data.size * 1.5;
        res.color = `${resolveColor(data.color)}FF`;
        res.highlighted = true;
        res.label = nodeIndexRef.current.get(node)?.title ?? data.label;
      } // hover 节点的邻居：轻度高亮
      else if (hovered && connectedToHovered.has(node)) {
        res.size = data.size * 1.15;
        res.color = resolveColor(data.color);
      } // 搜索高亮模式下：淡化非命中节点
      else if (hasHighlight && !highlight!.has(node)) {
        res.color = `${resolveColor(data.color)}25`;
        res.label = undefined;
        res.size = data.size * 0.8;
      } // 有 hover/selected 时，淡化其他节点
      else if (hovered || selected) {
        res.color = `${resolveColor(data.color)}60`;
        res.size = data.size * 0.85;
      }

      return res;
    });

    // ── edgeReducer：Obsidian 风格的边高亮效果 ──
    sigmaInstance.setSetting("edgeReducer", (edge, data) => {
      const res = { ...data };
      const hovered = hoveredNodeRef.current;
      const selected = selectedNodeIdRef.current;

      // 无 hover/selected 时保持原样
      if (!hovered && !selected) { return res; }

      const g = sigmaInstance.getGraph();
      const [src, tgt] = g.extremities(edge);

      const isConnectedToHovered = hovered && (src === hovered || tgt === hovered);
      const isConnectedToSelected = selected && (src === selected || tgt === selected);

      if (isConnectedToHovered || isConnectedToSelected) {
        // 相关边：高亮 + 加粗
        res.color = resolveColor(data.color);
        res.size = data.size * 2.5;
      } else {
        // 非相关边：大幅淡化
        res.color = `${resolveColor(data.color)}15`;
        res.size = data.size * 0.5;
      }
      return res;
    });

    // 事件
    sigmaInstance.on("clickNode", ({ node }) => onNodeClick?.(node));
    sigmaInstance.on("doubleClickNode", ({ node }) => onNodeDoubleClick?.(node));
    sigmaInstance.on("rightClickNode", ({ node, event }) => {
      // sigma 的 MouseCoords 没有 preventDefault，需要通过原生 DOM event 阻止
      const nativeEvent = (event as unknown as { original?: MouseEvent }).original
        ?? (event as unknown as { event?: MouseEvent }).event;
      nativeEvent?.preventDefault();
      onContextMenu?.(node, { x: event.x, y: event.y });
    });
    sigmaInstance.on("enterNode", ({ node }) => {
      hoveredNodeRef.current = node;
      onNodeHover?.(node);
      // skipIndexation：节点索引不变，仅重绘，性能更好
      sigmaInstance.refresh({ skipIndexation: true });
    });
    sigmaInstance.on("leaveNode", () => {
      hoveredNodeRef.current = null;
      onNodeHover?.(null);
      sigmaInstance.refresh({ skipIndexation: true });
    });
    sigmaInstance.on("clickStage", () => onDeselect?.());

    // 滚轮缩放交给 sigma 默认行为处理：sigma v3 默认 zoomOnMouse=true，
    // 会对准鼠标位置缩放，且做了正确的频率控制。
    // 之前自定义 wheel + camera.animate 在高频事件下会互相打断导致视图乱跑，
    // 这里不覆盖默认行为。

    // 键盘
    const handleKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        onDeselect?.();
      }
      if ((e.key === "Delete" || e.key === "Backspace") && selectedNodeId) {
        const target = e.target as HTMLElement;
        if (
          target.tagName === "INPUT" || target.tagName === "TEXTAREA"
          || target.isContentEditable
        ) {
          return;
        }
        onDeleteNode?.(selectedNodeId);
      }
    };
    el.addEventListener("keydown", handleKey);

    return () => {
      el.removeEventListener("keydown", handleKey);
      // 清理进行中的动画，避免 sigma kill 后继续 refresh 导致报错
      if (positionAnimRef.current !== null) {
        cancelAnimationFrame(positionAnimRef.current);
        positionAnimRef.current = null;
      }
      if (enterAnimRef.current !== null) {
        cancelAnimationFrame(enterAnimRef.current);
        enterAnimRef.current = null;
      }
      sigmaInstance.kill();
      sigmaRef.current = null;
      graphRef.current = null;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [token, isLargeGraph]);

  // ── 数据变化时重建图 ──
  useEffect(() => {
    const graph = graphRef.current;
    const sigmaInstance = sigmaRef.current;
    if (!graph || !sigmaInstance) { return; }

    // 取消可能进行中的入场/位置动画
    if (enterAnimRef.current !== null) {
      cancelAnimationFrame(enterAnimRef.current);
      enterAnimRef.current = null;
    }

    // 清空
    graph.clear();
    nodeIndexRef.current = new Map();

    // 初始位置：按当前布局模式选择，避免"先矩形再跳圆形"的视觉跳变
    const initialPositions = layoutMode === "radial"
      ? circularLayout(filteredNodes, dimensions.width, dimensions.height)
      : gridLayout(filteredNodes, dimensions.width, dimensions.height);

    // 节点直接放到目标位置，但 size 从 0 开始增长（避免从中心爆开的眩晕感）
    for (const node of filteredNodes) {
      const pos = initialPositions.get(node.id) ?? { x: 0, y: 0 };
      const color = getNodeColor(node, communities, token);
      const targetSize = getNodeSize(node);
      graph.addNode(node.id, {
        x: pos.x,
        y: pos.y,
        size: 0, // 入场动画从 0 增长到 targetSize
        targetSize, // 自定义属性，动画读取
        color,
        label: node.title,
        nodeType: node.type,
      });
      nodeIndexRef.current.set(node.id, node);
    }

    let edgeIdx = 0;
    for (const edge of filteredEdges) {
      const style = edgeTypeStyles[edge.type] || edgeTypeStyles.link;
      const edgeId = `e${edgeIdx++}`;
      if (graph.hasNode(edge.source) && graph.hasNode(edge.target)) {
        // graphology undirected 不支持平行边，但 hasEdge 检查会拒绝重复
        if (!graph.hasEdge(edge.source, edge.target)) {
          const isReference = edge.type === "reference";
          graph.addEdgeWithKey(edgeId, edge.source, edge.target, {
            color: resolveColor(style.color),
            size: style.width,
            edgeType: edge.type,
            ...(isReference ? { dash: [6, 4] } : {}),
          });
        }
      }
    }

    setStats({
      visible: filteredNodes.length,
      total: data.nodes.length,
      edges: filteredEdges.length,
    });
    sigmaInstance.refresh();

    // ── 入场动画：节点 size 从 0 弹性增长到目标值 ──
    // 用 ease-out back（带轻微过冲），有"弹出"的活力感
    const startTime = performance.now();
    const enterDuration = 500;
    // 收集入场后的目标位置，用于动画结束后首次居中
    const initialPositionsArr = Array.from(initialPositions.values());
    const enterTick = (now: number) => {
      const t = Math.min(1, (now - startTime) / enterDuration);
      // ease-out back：c1=1.7018，过冲量适中
      const c1 = 1.7018;
      const c3 = c1 + 1;
      const eased = 1 + c3 * Math.pow(t - 1, 3) + c1 * Math.pow(t - 1, 2);
      graph.forEachNode((id, attrs) => {
        const target = attrs.targetSize;
        if (typeof target === "number") {
          graph.setNodeAttribute(id, "size", Math.max(0, target * eased));
        }
      });
      sigmaInstance.refresh({ skipIndexation: true });
      if (t < 1) {
        enterAnimRef.current = requestAnimationFrame(enterTick);
      } else {
        enterAnimRef.current = null;
        // 首次入场完成后居中视图（后续 worker 完成不再自动居中，避免打断用户交互）
        centerCameraOnGraph(initialPositionsArr, 600);
      }
    };
    enterAnimRef.current = requestAnimationFrame(enterTick);

    // 触发布局计算（worker 算完后会通过 applyPositions 做二次位置动画）
    triggerLayout();
    // 注意：dimensions 不在依赖里——尺寸变化由独立的 effect 处理位置重算，
    // 不触发图重建（避免清空图 + 重跑入场动画导致的视觉跳变）
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [filteredNodes, filteredEdges, communities, token, layoutMode]);

  // ── 尺寸变化时只重算位置，不重建图 ──
  // 容器 resize（如侧栏开合）时，radial/hierarchy 需要按新尺寸重新分布节点。
  // 用 applyPositions 做位置插值动画，视觉上是节点平滑移动到新位置，不会闪烁。
  useEffect(() => {
    if (filteredNodes.length === 0) { return; }
    // 只对纯几何布局生效（force 模式由 worker 接管，尺寸变化不需要重算）
    if (layoutMode === "force") { return; }
    const positions = layoutMode === "radial"
      ? circularLayout(filteredNodes, dimensions.width, dimensions.height)
      : gridLayout(filteredNodes, dimensions.width, dimensions.height);
    applyPositions(
      Array.from(positions.entries()).map(([id, p]) => ({ id, x: p.x, y: p.y })),
    );
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [dimensions, layoutMode]);

  // ── Worker 生命周期 ──
  useEffect(() => {
    const worker = new Worker(
      new URL("./graphLayout.worker.ts", import.meta.url),
      { type: "module" },
    );
    worker.onmessage = (ev: MessageEvent<WorkerOutbound>) => {
      workerBusyRef.current = false;
      setLayoutRunning(false);
      const msg = ev.data;
      if (msg.type === "done") {
        applyPositions(msg.positions);
      } else if (msg.type === "error") {
        console.warn("[GraphView] layout worker error:", msg.message);
      }
    };
    worker.onerror = (e) => {
      workerBusyRef.current = false;
      setLayoutRunning(false);
      console.error("[GraphView] worker crash:", e);
    };
    workerRef.current = worker;
    return () => {
      worker.terminate();
      workerRef.current = null;
    };
  }, []);

  // ── 触发布局计算 ──
  const triggerLayout = useCallback(() => {
    const worker = workerRef.current;
    const graph = graphRef.current;
    if (!worker || !graph) { return; }
    if (filteredNodes.length === 0) { return; }

    // hierarchy 模式：网格布局
    if (layoutMode === "hierarchy") {
      const positions = gridLayout(filteredNodes, dimensions.width, dimensions.height);
      applyPositions(Array.from(positions.entries()).map(([id, p]) => ({ id, x: p.x, y: p.y })));
      return;
    }

    // radial 模式：真正的同心圆布局（不走 ForceAtlas2，避免收敛成不规则形状）
    if (layoutMode === "radial") {
      const positions = circularLayout(filteredNodes, dimensions.width, dimensions.height);
      applyPositions(Array.from(positions.entries()).map(([id, p]) => ({ id, x: p.x, y: p.y })));
      return;
    }

    // force 模式：小图直接用网格（worker 开销大于计算），大图走 worker
    if (filteredNodes.length < LAYOUT_FORCE_THRESHOLD) {
      const positions = gridLayout(filteredNodes, dimensions.width, dimensions.height);
      applyPositions(Array.from(positions.entries()).map(([id, p]) => ({ id, x: p.x, y: p.y })));
      return;
    }

    if (workerBusyRef.current) { return; }
    workerBusyRef.current = true;
    setLayoutRunning(true);

    const nodesPayload = filteredNodes.map((n) => {
      const p = graph.getNodeAttributes(n.id);
      return { id: n.id, x: p.x, y: p.y };
    });
    const edgesPayload = filteredEdges.map((e) => ({ source: e.source, target: e.target }));

    const settings: LayoutRequest["settings"] = {
      barnesHutOptimize: useBarnesHut,
      barnesHutTheta: 0.5,
      // 适中的重力让节点自然聚集但不过分
      gravity: 0.8,
      // 更高的 slowDown 让布局更平滑收敛
      slowDown: 3,
      // 更小的 scaling 让节点更紧凑
      scaling: 0.8,
      linLogMode: false,
    };

    const req: LayoutRequest = {
      type: "layout",
      nodes: nodesPayload,
      edges: edgesPayload,
      iterations: FORCE_ATLAS_ITERATIONS,
      settings,
    };
    worker.postMessage(req);
  }, [filteredNodes, filteredEdges, dimensions, layoutMode, useBarnesHut]);

  // ── 位置插值动画（RAF + ease-out cubic）──
  // 通用方法：从 startPositions 插值到 endPositions，duration 毫秒内完成
  const animatePositions = useCallback(
    (
      startPositions: Map<string, { x: number; y: number }>,
      endPositions: Map<string, { x: number; y: number }>,
      duration: number,
      onDone?: () => void,
    ) => {
      const graph = graphRef.current;
      const sigmaInstance = sigmaRef.current;
      if (!graph || !sigmaInstance) { return; }
      // 取消未完成的位置动画，避免叠加抖动
      if (positionAnimRef.current !== null) {
        cancelAnimationFrame(positionAnimRef.current);
        positionAnimRef.current = null;
      }
      const ids = Array.from(endPositions.keys());
      const startTime = performance.now();
      const tick = (now: number) => {
        const t = Math.min(1, (now - startTime) / duration);
        // ease-out cubic：起步快、收尾稳，适合节点落地
        const eased = 1 - Math.pow(1 - t, 3);
        for (const id of ids) {
          const start = startPositions.get(id);
          const end = endPositions.get(id);
          if (!start || !end || !graph.hasNode(id)) { continue; }
          graph.setNodeAttribute(id, "x", start.x + (end.x - start.x) * eased);
          graph.setNodeAttribute(id, "y", start.y + (end.y - start.y) * eased);
        }
        sigmaInstance.refresh({ skipIndexation: true });
        if (t < 1) {
          positionAnimRef.current = requestAnimationFrame(tick);
        } else {
          positionAnimRef.current = null;
          onDone?.();
        }
      };
      positionAnimRef.current = requestAnimationFrame(tick);
    },
    [],
  );

  // ── 居中相机到节点群包围盒 ──
  const centerCameraOnGraph = useCallback(
    (positions: Array<{ x: number; y: number }>, duration = 450) => {
      const sigma = sigmaRef.current;
      if (!sigma) { return; }
      const bbox = computeBoundingBox(positions);
      if (!bbox) { return; }
      const viewportWidth = sigma.getDimensions().width;
      const viewportHeight = sigma.getDimensions().height;
      const bboxMaxDim = Math.max(bbox.width, bbox.height, 1);
      const viewportMinDim = Math.min(viewportWidth, viewportHeight, 1);
      // ratio = 包围盒最大边 / 视口短边 * 1.2（留 20% 边距）
      const targetRatio = Math.max(
        (sigma.getSetting("minCameraRatio") as number | undefined) ?? 0.02,
        Math.min(
          (sigma.getSetting("maxCameraRatio") as number | undefined) ?? 10,
          (bboxMaxDim / viewportMinDim) * 1.2,
        ),
      );
      sigma.getCamera().animate(
        { x: bbox.centerX, y: bbox.centerY, ratio: targetRatio },
        { duration },
      );
    },
    [],
  );

  // ── 应用位置：从当前位置插值到目标位置 ──
  // 注意：不在这里自动居中相机，避免打断用户的缩放/平移交互。
  // 居中只发生在首次入场和用户点击"适应视图"按钮时。
  const applyPositions = useCallback(
    (positions: Array<{ id: string; x: number; y: number }>) => {
      const graph = graphRef.current;
      if (!graph) { return; }
      const startPositions = new Map<string, { x: number; y: number }>();
      const endPositions = new Map<string, { x: number; y: number }>();
      for (const p of positions) {
        if (graph.hasNode(p.id)) {
          const attrs = graph.getNodeAttributes(p.id);
          startPositions.set(p.id, { x: attrs.x, y: attrs.y });
          endPositions.set(p.id, { x: p.x, y: p.y });
        }
      }
      animatePositions(startPositions, endPositions, 550);
    },
    [animatePositions],
  );

  // ── 内部搜索高亮：合并外部 + 内部 ──
  const effectiveHighlights = useMemo(() => {
    if (!searchTerm) {
      return highlightedNodeIds && highlightedNodeIds.size > 0 ? highlightedNodeIds : undefined;
    }
    const q = searchTerm.toLowerCase();
    const ids = new Set<string>();
    data.nodes.forEach((n) => {
      if (
        n.title.toLowerCase().includes(q)
        || n.tags.some((t) => t.toLowerCase().includes(q))
        || n.path.toLowerCase().includes(q)
      ) {
        ids.add(n.id);
      }
    });
    if (highlightedNodeIds && highlightedNodeIds.size > 0) {
      for (const id of highlightedNodeIds) { ids.add(id); }
    }
    return ids.size > 0 ? ids : undefined;
  }, [searchTerm, data.nodes, highlightedNodeIds]);

  // ── 高亮/选中：同步 ref，视觉反馈由 nodeReducer 即时计算 ──
  useEffect(() => {
    selectedNodeIdRef.current = selectedNodeId ?? null;
    highlightSetRef.current = effectiveHighlights;
    // skipIndexation：节点索引不变，仅重绘以应用 reducer
    sigmaRef.current?.refresh({ skipIndexation: true });
  }, [effectiveHighlights, selectedNodeId]);

  // ── 工具栏 ──
  const handleZoomIn = useCallback(() => {
    sigmaRef.current?.getCamera().animatedZoom({ duration: 300 });
  }, []);
  const handleZoomOut = useCallback(() => {
    sigmaRef.current?.getCamera().animatedUnzoom({ duration: 300 });
  }, []);
  const handleFitAll = useCallback(() => {
    const graph = graphRef.current;
    if (!graph) { return; }
    // 收集所有节点当前位置，居中到包围盒
    const positions: Array<{ x: number; y: number }> = [];
    graph.forEachNode((_, attrs) => {
      positions.push({ x: attrs.x, y: attrs.y });
    });
    centerCameraOnGraph(positions, 600);
  }, [centerCameraOnGraph]);
  const handleFocusSelected = useCallback(() => {
    if (!selectedNodeId || !sigmaRef.current || !graphRef.current) { return; }
    const graph = graphRef.current;
    if (!graph.hasNode(selectedNodeId)) { return; }
    const attrs = graph.getNodeAttributes(selectedNodeId);
    sigmaRef.current.getCamera().animate(
      { x: attrs.x, y: attrs.y, ratio: 0.5 },
      { duration: 500 },
    );
  }, [selectedNodeId]);
  const handleFullscreenToggle = useCallback(() => {
    if (isFullscreen) {
      document.exitFullscreen();
    } else {
      containerRef.current?.requestFullscreen();
    }
  }, [isFullscreen]);
  const handleExportPNG = useCallback(() => {
    const canvas = containerRef.current?.querySelector("canvas");
    if (!canvas) { return; }
    const link = document.createElement("a");
    link.download = `wiki-graph-${Date.now()}.png`;
    link.href = canvas.toDataURL("image/png");
    link.click();
  }, []);
  const handleRelaunchLayout = useCallback(() => {
    triggerLayout();
  }, [triggerLayout]);

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

  // 通用按钮样式：Obsidian 风格
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

  return (
    <div
      ref={containerRef}
      tabIndex={0}
      className="outline-none focus-visible:outline-2 focus-visible:outline-offset-2"
      style={{ width: "100%", height: "100%", position: "relative" }}
    >
      {/* sigma 渲染容器：100% 填充父级 */}
      <div style={{ width: "100%", height: "100%" }} />

      {/* 左上角：筛选按钮 */}
      <div
        style={{
          position: "absolute",
          top: 10,
          left: 10,
          zIndex: 10,
        }}
      >
        <Popover
          open={filtersOpen}
          onOpenChange={setFiltersOpen}
          trigger="click"
          placement="bottomLeft"
          arrow={false}
          styles={{
            root: { width: 280 },
            container: { padding: "12px 14px" },
          }}
          content={
            <Space orientation="vertical" size="middle" style={{ width: "100%" }}>
              <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
                <Typography.Text strong style={{ fontSize: 12 }}>{t("wiki.graph.filters")}</Typography.Text>
                <Segmented
                  size="small"
                  value={layoutMode}
                  onChange={(v) => setLayoutMode(v as LayoutMode)}
                  options={[
                    { label: t("wiki.graph.layoutForce"), value: "force" },
                    { label: t("wiki.graph.layoutRadial"), value: "radial" },
                    { label: t("wiki.graph.layoutHierarchy"), value: "hierarchy" },
                  ]}
                />
              </div>
              <Input
                size="small"
                prefix={<Search size={12} />}
                placeholder={t("wiki.searchGraph")}
                value={searchTerm}
                onChange={(e) => setSearchTerm(e.target.value)}
                allowClear
              />
              <Select
                mode="multiple"
                placeholder={t("wiki.graph.filterByTags")}
                style={{ width: "100%" }}
                allowClear
                size="small"
                value={filters?.tags}
                onChange={(values) => onFiltersChange?.({ tags: values, types: filters?.types })}
                options={allTags.map((tag) => ({ label: tag, value: tag }))}
                maxTagCount={2}
              />
              <div style={{ display: "flex", gap: 4, flexWrap: "wrap" }}>
                {(["note", "concept", "entity", "source"] as GraphNodeType[]).map((type) => {
                  const activeTypes = filters?.types || [];
                  const isActive = activeTypes.length === 0 || activeTypes.includes(type);
                  return (
                    <Tag
                      key={type}
                      color={isActive ? nodeColors[type] : undefined}
                      style={{
                        fontSize: 10,
                        margin: 0,
                        cursor: "pointer",
                        opacity: isActive ? 1 : 0.4,
                        padding: "0 4px",
                        lineHeight: "18px",
                        borderRadius: 4,
                      }}
                      onClick={() => {
                        const current = new Set(activeTypes);
                        if (current.has(type)) { current.delete(type); }
                        else { current.add(type); }
                        const arr = Array.from(current);
                        onFiltersChange?.({ types: arr.length > 0 ? arr : undefined, tags: filters?.tags });
                      }}
                    >
                      {type[0].toUpperCase()}
                      {data.nodes.filter((n) => n.type === type).length}
                    </Tag>
                  );
                })}
              </div>
            </Space>
          }
        >
          <AntTooltip title={t("wiki.graph.filters")}>
            <Button
              size="small"
              type="text"
              icon={<SlidersHorizontal size={13} />}
              style={ctrlBtnStyle}
              onMouseEnter={hoverBtnStyle}
              onMouseLeave={leaveBtnStyle}
            />
          </AntTooltip>
        </Popover>
      </div>

      {/* 右上角：统计按钮 */}
      <div
        style={{
          position: "absolute",
          top: 10,
          right: 10,
          zIndex: 10,
        }}
      >
        <Popover
          open={statsOpen}
          onOpenChange={setStatsOpen}
          trigger="click"
          placement="bottomRight"
          arrow={false}
          styles={{
            root: { width: 180 },
            container: { padding: "10px 14px" },
          }}
          content={
            <Space orientation="vertical" size={4} style={{ width: "100%" }}>
              <Typography.Text type="secondary" style={{ fontSize: 11 }}>{t("wiki.graph.stats")}</Typography.Text>
              <Typography.Text style={{ fontSize: 12 }}>
                {t("wiki.graph.nodes")}: {stats.visible}/{stats.total}
              </Typography.Text>
              <Typography.Text style={{ fontSize: 12 }}>{t("wiki.graph.edges")}: {stats.edges}</Typography.Text>
              <Typography.Text style={{ fontSize: 12 }}>
                {t("wiki.graph.components")}: {componentStats.components}
              </Typography.Text>
              <Typography.Text style={{ fontSize: 12 }}>
                {t("wiki.graph.largestComponent")}: {componentStats.largestSize}
              </Typography.Text>
              {layoutRunning && (
                <Typography.Text type="warning" style={{ fontSize: 11 }}>
                  ⏳ {t("wiki.graph.layoutRunning")}
                </Typography.Text>
              )}
            </Space>
          }
        >
          <AntTooltip title={t("wiki.graph.stats")}>
            <Button
              size="small"
              type="text"
              style={{
                ...ctrlBtnStyle,
                fontSize: 10,
                fontWeight: 600,
                color: token.colorTextSecondary,
              }}
              onMouseEnter={hoverBtnStyle}
              onMouseLeave={leaveBtnStyle}
            >
              {stats.total}
            </Button>
          </AntTooltip>
        </Popover>
      </div>

      {/* 右下角：图例按钮 */}
      <div
        style={{
          position: "absolute",
          bottom: 50,
          right: 10,
          zIndex: 10,
        }}
      >
        <Popover
          open={legendOpen}
          onOpenChange={setLegendOpen}
          trigger="click"
          placement="topRight"
          arrow={false}
          styles={{ container: { padding: "10px 14px" } }}
          content={
            <div style={{ display: "flex", gap: 8, flexWrap: "wrap", fontSize: 11 }}>
              {(Object.keys(edgeTypeStyles) as GraphEdgeType[]).map((et) => {
                const s = edgeTypeStyles[et];
                const isHidden = hiddenEdgeTypes.has(et);
                const isReference = et === "reference";
                return (
                  <span
                    key={et}
                    style={{
                      display: "flex",
                      alignItems: "center",
                      gap: 4,
                      cursor: "pointer",
                      opacity: isHidden ? 0.3 : 1,
                      textDecoration: isHidden ? "line-through" : "none",
                      padding: "2px 6px",
                      borderRadius: 4,
                      transition: "background 0.15s",
                    }}
                    onMouseEnter={(e) => {
                      e.currentTarget.style.background = token.colorBgTextHover;
                    }}
                    onMouseLeave={(e) => {
                      e.currentTarget.style.background = "transparent";
                    }}
                    onClick={() => {
                      setHiddenEdgeTypes((prev) => {
                        const next = new Set(prev);
                        if (next.has(et)) { next.delete(et); }
                        else { next.add(et); }
                        return next;
                      });
                    }}
                  >
                    <svg width="20" height="8">
                      {isReference
                        ? (
                          <line
                            x1="0"
                            y1="4"
                            x2="20"
                            y2="4"
                            stroke={resolveColor(s.color)}
                            strokeWidth={s.width}
                            strokeDasharray="4 3"
                          />
                        )
                        : <line x1="0" y1="4" x2="20" y2="4" stroke={resolveColor(s.color)} strokeWidth={s.width} />}
                    </svg>
                    <span style={{ color: resolveColor(s.color) }}>{t(edgeTypeLabels[et])}</span>
                  </span>
                );
              })}
            </div>
          }
        >
          <AntTooltip title={t("wiki.graph.legend") || "Legend"}>
            <Button
              size="small"
              type="text"
              icon={<Info size={13} />}
              style={ctrlBtnStyle}
              onMouseEnter={hoverBtnStyle}
              onMouseLeave={leaveBtnStyle}
            />
          </AntTooltip>
        </Popover>
      </div>

      {/* 底部中央：工具栏（Obsidian 风格） */}
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
            <ZoomIn size={14} />
          </button>
        </Tooltip>
        <Tooltip title={t("wiki.graph.zoomOut")}>
          <button
            onClick={handleZoomOut}
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
            <ZoomOut size={14} />
          </button>
        </Tooltip>
        <Tooltip title={t("wiki.graph.fitView")}>
          <button
            onClick={handleFitAll}
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
            <Maximize2 size={14} />
          </button>
        </Tooltip>
        {selectedNodeId && (
          <Tooltip title={t("wiki.graph.focusSelected")}>
            <button
              onClick={handleFocusSelected}
              style={{
                ...ctrlBtnStyle,
                width: 24,
                height: 24,
                minWidth: 24,
                background: "transparent",
                border: "none",
                color: token.colorPrimary,
              }}
              onMouseEnter={hoverBtnStyle}
              onMouseLeave={leaveBtnStyle}
            >
              <Minimize2 size={14} />
            </button>
          </Tooltip>
        )}
        <div
          style={{
            width: 1,
            height: 14,
            background: token.colorBorderSecondary,
            margin: "0 2px",
          }}
        />
        <Tooltip title={t("wiki.graph.fullscreen") || t("wiki.graph.fitView")}>
          <button
            onClick={handleFullscreenToggle}
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
            <Fullscreen size={14} />
          </button>
        </Tooltip>
        <Tooltip title={t("wiki.graph.exportPNG")}>
          <button
            onClick={handleExportPNG}
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
        <Tooltip title={t("wiki.graph.relayout")}>
          <button
            onClick={handleRelaunchLayout}
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
            <RefreshCw size={14} />
          </button>
        </Tooltip>
      </div>
    </div>
  );
}

export const GraphView = memo(GraphViewInner);

// 保留向后兼容：旧代码可能 import { GraphView }
export { GraphView as default };

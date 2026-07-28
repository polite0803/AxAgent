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
import { memo, useCallback, useEffect, useMemo, useRef, useState } from "react";
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
const FORCE_ATLAS_ITERATIONS = 100; // 默认迭代次数

const getNodeColorMap = (token: TokenType): Record<GraphNodeType, string> => ({
  note: token.colorPrimary,
  concept: token.colorSuccess,
  entity: "var(--orange, #fa8c16)",
  source: "var(--magenta, #eb2f96)",
});

const communityPalette = [
  "#4C72B0",
  "#DD8452",
  "#55A868",
  "#C44E52",
  "#8172B3",
  "#937860",
  "#DA8BC3",
  "#8C8C8C",
  "#CCB974",
  "#64B5CD",
  "#E18B6C",
  "#7AA153",
];

const getEdgeTypeStylesMap = (token: TokenType): Record<
  GraphEdgeType,
  { color: string; width: number }
> => ({
  link: { color: token.colorBorderSecondary, width: 1 },
  backlink: { color: token.colorPrimary, width: 1.5 },
  reference: { color: token.colorSuccess, width: 1.5 },
  derived_from: { color: "var(--orange, #fa8c16)", width: 1.5 },
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
  const linkSum = node.linkCount + node.backlinkCount;
  // sigma 节点 size 范围建议 1-15
  return Math.max(3, Math.min(15, 3 + linkSum * 0.5));
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
      labelDensity: 0.07,
      labelGridCellSize: 60,
      labelRenderedSizeThreshold: 6,
      labelFont: "Inter, system-ui, sans-serif",
      labelSize: 12,
      labelColor: { color: token.colorText },
      edgeLabelSize: 10,
      minCameraRatio: 0.02,
      maxCameraRatio: 10,
    });
    sigmaRef.current = sigmaInstance;

    // ── nodeReducer：hover 放大 + 选中高亮 + 淡化非命中节点 ──
    // 通过 ref 读取最新状态，避免闭包失效和频繁重设 reducer
    sigmaInstance.setSetting("nodeReducer", (node, data) => {
      const res = { ...data };
      const hovered = hoveredNodeRef.current;
      const selected = selectedNodeIdRef.current;
      const highlight = highlightSetRef.current;
      const hasHighlight = highlight && highlight.size > 0;

      if (selected === node) {
        res.size = data.size * 1.6;
        res.highlighted = true;
      } else if (hovered === node) {
        res.size = data.size * 1.4;
        res.highlighted = true;
      } else if (hasHighlight && !highlight!.has(node)) {
        // 搜索/外部高亮时，非命中节点淡化
        res.color = `${resolveColor(data.color)}30`;
        res.label = undefined;
      }
      return res;
    });

    // ── edgeReducer：hover 时高亮相关边、淡化无关边 ──
    sigmaInstance.setSetting("edgeReducer", (edge, data) => {
      const res = { ...data };
      const hovered = hoveredNodeRef.current;
      if (!hovered) { return res; }
      const g = sigmaInstance.getGraph();
      const [src, tgt] = g.extremities(edge);
      if (src !== hovered && tgt !== hovered) {
        // 淡化非相关边
        res.color = `${resolveColor(data.color)}20`;
        res.hidden = false;
      } else {
        // 高亮相关边
        res.size = data.size * 2.2;
        res.color = resolveColor(data.color);
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

    // ── 自定义滚轮缩放：跟随鼠标位置（sigma 默认缩放视口中心，不跟手）──
    const minRatio = (sigmaInstance.getSetting("minCameraRatio") as number | undefined) ?? 0.02;
    const maxRatio = (sigmaInstance.getSetting("maxCameraRatio") as number | undefined) ?? 10;
    const handleWheel = (e: WheelEvent) => {
      e.preventDefault();
      e.stopPropagation();
      const sigma = sigmaRef.current;
      if (!sigma) { return; }
      const camera = sigma.getCamera();
      const state = camera.getState();
      // 滚轮向上 → 放大（ratio 减小）；向下 → 缩小（ratio 增大）
      const factor = e.deltaY < 0 ? 0.82 : 1.22;
      const newRatio = Math.max(minRatio, Math.min(maxRatio, state.ratio * factor));
      if (newRatio === state.ratio) { return; }
      // 鼠标在视口中的位置
      const rect = el.getBoundingClientRect();
      const mouseX = e.clientX - rect.left;
      const mouseY = e.clientY - rect.top;
      // 鼠标对应的世界坐标
      const world = sigma.viewportToGraph({ x: mouseX, y: mouseY });
      // 缩放后让鼠标位置仍对应同一世界坐标：
      // newCamera = world + (oldCamera - world) * (newRatio / oldRatio)
      const newX = world.x + (state.x - world.x) * (newRatio / state.ratio);
      const newY = world.y + (state.y - world.y) * (newRatio / state.ratio);
      camera.animate(
        { x: newX, y: newY, ratio: newRatio },
        { duration: 180, easing: (k) => k },
      );
    };
    // capture: true → 在 sigma 默认 wheel 处理之前拦截
    el.addEventListener("wheel", handleWheel, { passive: false, capture: true });

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
      el.removeEventListener("wheel", handleWheel, { capture: true });
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

    // 目标位置：网格布局
    const initialPositions = gridLayout(filteredNodes, dimensions.width, dimensions.height);

    // 节点入场：先全部放到视口中心 (0,0)，再展开到网格位置
    for (const node of filteredNodes) {
      const color = getNodeColor(node, communities, token);
      const size = getNodeSize(node);
      graph.addNode(node.id, {
        x: 0,
        y: 0,
        size,
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
          graph.addEdgeWithKey(edgeId, edge.source, edge.target, {
            color: resolveColor(style.color),
            size: style.width,
            edgeType: edge.type,
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

    // ── 入场动画：从中心 (0,0) 展开到网格位置 ──
    const endPositions = new Map<string, { x: number; y: number }>();
    for (const node of filteredNodes) {
      const target = initialPositions.get(node.id) ?? { x: 0, y: 0 };
      endPositions.set(node.id, target);
    }
    // 用 enterAnimRef 单独管理入场动画，避免被 applyPositions 的取消逻辑打断
    const startTime = performance.now();
    const enterDuration = 600;
    const enterTick = (now: number) => {
      const t = Math.min(1, (now - startTime) / enterDuration);
      // ease-out quint：更明显的"落地"感
      const eased = 1 - Math.pow(1 - t, 5);
      for (const [id, end] of endPositions) {
        if (!graph.hasNode(id)) { continue; }
        graph.setNodeAttribute(id, "x", end.x * eased);
        graph.setNodeAttribute(id, "y", end.y * eased);
      }
      sigmaInstance.refresh({ skipIndexation: true });
      if (t < 1) {
        enterAnimRef.current = requestAnimationFrame(enterTick);
      } else {
        enterAnimRef.current = null;
      }
    };
    enterAnimRef.current = requestAnimationFrame(enterTick);

    // 触发布局计算（worker 算完后会通过 applyPositions 做二次位置动画）
    triggerLayout();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [filteredNodes, filteredEdges, communities, token, dimensions, layoutMode]);

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
      barnesHutTheta: 0.6,
      gravity: 1.0,
      slowDown: 4,
      scaling: 1.0,
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

  // ── 应用位置：从当前位置插值到目标位置，完成后强制视觉居中 ──
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
      animatePositions(startPositions, endPositions, 550, () => {
        // 布局完成后：计算节点群包围盒，把相机对准包围盒中心（强制视觉居中）
        const sigma = sigmaRef.current;
        if (!sigma) { return; }
        const bbox = computeBoundingBox(positions);
        if (!bbox) { return; }
        // ratio 根据包围盒大小自适应：保证整个图能放进视口，留 20% 边距
        // sigma 的 camera ratio 是"视口短边 / 世界短边"的反比，越大越缩小
        const viewportWidth = sigma.getDimensions().width;
        const viewportHeight = sigma.getDimensions().height;
        const bboxMaxDim = Math.max(bbox.width, bbox.height, 1);
        const viewportMinDim = Math.min(viewportWidth, viewportHeight, 1);
        // ratio = bboxMaxDim / viewportMinDim * 1.2（1.2 是边距系数）
        const targetRatio = Math.max(
          (sigma.getSetting("minCameraRatio") as number | undefined) ?? 0.02,
          Math.min(
            (sigma.getSetting("maxCameraRatio") as number | undefined) ?? 10,
            (bboxMaxDim / viewportMinDim) * 1.2,
          ),
        );
        sigma.getCamera().animate(
          { x: bbox.centerX, y: bbox.centerY, ratio: targetRatio },
          { duration: 450 },
        );
      });
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
    const sigma = sigmaRef.current;
    const graph = graphRef.current;
    if (!sigma || !graph) { return; }
    // 收集所有节点的当前位置，计算包围盒后居中
    const positions: Array<{ x: number; y: number }> = [];
    graph.forEachNode((_, attrs) => {
      positions.push({ x: attrs.x, y: attrs.y });
    });
    const bbox = computeBoundingBox(positions);
    if (!bbox) {
      sigma.getCamera().animatedReset({ duration: 600 });
      return;
    }
    const viewportWidth = sigma.getDimensions().width;
    const viewportHeight = sigma.getDimensions().height;
    const bboxMaxDim = Math.max(bbox.width, bbox.height, 1);
    const viewportMinDim = Math.min(viewportWidth, viewportHeight, 1);
    const targetRatio = Math.max(
      (sigma.getSetting("minCameraRatio") as number | undefined) ?? 0.02,
      Math.min(
        (sigma.getSetting("maxCameraRatio") as number | undefined) ?? 10,
        (bboxMaxDim / viewportMinDim) * 1.2,
      ),
    );
    sigma.getCamera().animate(
      { x: bbox.centerX, y: bbox.centerY, ratio: targetRatio },
      { duration: 600 },
    );
  }, []);
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
        }}
      >
        <Empty description={t("wiki.graph.empty")} />
      </Card>
    );
  }

  return (
    <div
      ref={containerRef}
      tabIndex={0}
      className="outline-none focus-visible:outline-2 focus-visible:outline-offset-2"
      style={{ width: "100%", height: "100%", position: "relative" }}
    >
      {/* sigma 渲染容器：100% 填充父级 */}
      <div style={{ width: "100%", height: "100%" }} />

      {/* 左上角小按钮组 */}
      <div
        style={{
          position: "absolute",
          top: 8,
          left: 8,
          zIndex: 10,
          display: "flex",
          flexDirection: "column",
          gap: 4,
        }}
      >
        {/* 筛选按钮 */}
        <Popover
          open={filtersOpen}
          onOpenChange={setFiltersOpen}
          trigger="click"
          placement="bottomLeft"
          arrow={false}
          overlayInnerStyle={{ padding: "10px 12px" }}
          overlayStyle={{ width: 260 }}
          content={
            <Space direction="vertical" size="small" style={{ width: "100%" }}>
              <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
                <Typography.Text strong style={{ fontSize: 12 }}>{t("wiki.graph.filters")}</Typography.Text>
                <Segmented
                  size="small"
                  value={layoutMode}
                  onChange={(v) => setLayoutMode(v as LayoutMode)}
                  options={[
                    { label: "F", value: "force" },
                    { label: "R", value: "radial" },
                    { label: "D", value: "hierarchy" },
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
              icon={<SlidersHorizontal size={14} />}
              style={{
                width: 28,
                height: 28,
                minWidth: 28,
                padding: 0,
                borderRadius: 8,
                background: `${token.colorBgContainer}dd`,
                backdropFilter: "blur(8px)",
                border: `1px solid ${token.colorBorderSecondary}40`,
              }}
            />
          </AntTooltip>
        </Popover>
      </div>

      {/* 右上角小按钮组 */}
      <div
        style={{
          position: "absolute",
          top: 8,
          right: 8,
          zIndex: 10,
          display: "flex",
          flexDirection: "column",
          gap: 4,
        }}
      >
        {/* 统计按钮 */}
        <Popover
          open={statsOpen}
          onOpenChange={setStatsOpen}
          trigger="click"
          placement="bottomRight"
          arrow={false}
          overlayInnerStyle={{ padding: "10px 12px" }}
          overlayStyle={{ width: 180 }}
          content={
            <Space direction="vertical" size={2} style={{ width: "100%" }}>
              <Typography.Text type="secondary" style={{ fontSize: 11 }}>{t("wiki.graph.stats")}</Typography.Text>
              <Typography.Text style={{ fontSize: 11 }}>
                {t("wiki.graph.nodes")}: {stats.visible}/{stats.total}
              </Typography.Text>
              <Typography.Text style={{ fontSize: 11 }}>{t("wiki.graph.edges")}: {stats.edges}</Typography.Text>
              <Typography.Text style={{ fontSize: 11 }}>
                {t("wiki.graph.components")}: {componentStats.components}
              </Typography.Text>
              <Typography.Text style={{ fontSize: 11 }}>
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
                width: 28,
                height: 28,
                minWidth: 28,
                padding: 0,
                borderRadius: 8,
                background: `${token.colorBgContainer}dd`,
                backdropFilter: "blur(8px)",
                border: `1px solid ${token.colorBorderSecondary}40`,
                fontSize: 10,
                fontWeight: 600,
              }}
            >
              {stats.total}
            </Button>
          </AntTooltip>
        </Popover>
      </div>

      {/* 右下角小按钮 */}
      <div
        style={{
          position: "absolute",
          bottom: 52,
          right: 8,
          zIndex: 10,
          display: "flex",
          flexDirection: "column",
          gap: 4,
        }}
      >
        <Popover
          open={legendOpen}
          onOpenChange={setLegendOpen}
          trigger="click"
          placement="topRight"
          arrow={false}
          overlayInnerStyle={{ padding: "8px 10px" }}
          content={
            <div style={{ display: "flex", gap: 8, flexWrap: "wrap", fontSize: 10 }}>
              {(Object.keys(edgeTypeStyles) as GraphEdgeType[]).map((et) => {
                const s = edgeTypeStyles[et];
                const isHidden = hiddenEdgeTypes.has(et);
                return (
                  <span
                    key={et}
                    style={{
                      display: "flex",
                      alignItems: "center",
                      gap: 3,
                      cursor: "pointer",
                      opacity: isHidden ? 0.3 : 1,
                      textDecoration: isHidden ? "line-through" : "none",
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
                    <svg width="18" height="6">
                      <line x1="0" y1="3" x2="18" y2="3" stroke={resolveColor(s.color)} strokeWidth={s.width} />
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
              icon={<Info size={14} />}
              style={{
                width: 28,
                height: 28,
                minWidth: 28,
                padding: 0,
                borderRadius: 8,
                background: `${token.colorBgContainer}dd`,
                backdropFilter: "blur(8px)",
                border: `1px solid ${token.colorBorderSecondary}40`,
              }}
            />
          </AntTooltip>
        </Popover>
      </div>

      {/* 底部中：工具栏 */}
      <div
        style={{
          position: "absolute",
          bottom: 8,
          left: "50%",
          transform: "translateX(-50%)",
          zIndex: 10,
          display: "flex",
          alignItems: "center",
          gap: 2,
          padding: "2px 6px",
          borderRadius: 20,
          background: `${token.colorBgContainer}ee`,
          backdropFilter: "blur(12px)",
          border: `1px solid ${token.colorBorderSecondary}40`,
        }}
      >
        <Tooltip title={t("wiki.graph.zoomIn")}>
          <button
            onClick={handleZoomIn}
            style={{
              background: "none",
              border: "none",
              cursor: "pointer",
              padding: 4,
              borderRadius: 6,
              display: "flex",
              alignItems: "center",
              color: token.colorTextSecondary,
            }}
            onMouseEnter={(e) => {
              e.currentTarget.style.background = token.colorBgTextHover;
            }}
            onMouseLeave={(e) => {
              e.currentTarget.style.background = "none";
            }}
          >
            <ZoomIn size={16} />
          </button>
        </Tooltip>
        <Tooltip title={t("wiki.graph.zoomOut")}>
          <button
            onClick={handleZoomOut}
            style={{
              background: "none",
              border: "none",
              cursor: "pointer",
              padding: 4,
              borderRadius: 6,
              display: "flex",
              alignItems: "center",
              color: token.colorTextSecondary,
            }}
            onMouseEnter={(e) => {
              e.currentTarget.style.background = token.colorBgTextHover;
            }}
            onMouseLeave={(e) => {
              e.currentTarget.style.background = "none";
            }}
          >
            <ZoomOut size={16} />
          </button>
        </Tooltip>
        <Tooltip title={t("wiki.graph.fitView")}>
          <button
            onClick={handleFitAll}
            style={{
              background: "none",
              border: "none",
              cursor: "pointer",
              padding: 4,
              borderRadius: 6,
              display: "flex",
              alignItems: "center",
              color: token.colorTextSecondary,
            }}
            onMouseEnter={(e) => {
              e.currentTarget.style.background = token.colorBgTextHover;
            }}
            onMouseLeave={(e) => {
              e.currentTarget.style.background = "none";
            }}
          >
            <Maximize2 size={16} />
          </button>
        </Tooltip>
        {selectedNodeId && (
          <Tooltip title={t("wiki.graph.focusSelected")}>
            <button
              onClick={handleFocusSelected}
              style={{
                background: "none",
                border: "none",
                cursor: "pointer",
                padding: 4,
                borderRadius: 6,
                display: "flex",
                alignItems: "center",
                color: token.colorPrimary,
              }}
              onMouseEnter={(e) => {
                e.currentTarget.style.background = token.colorBgTextHover;
              }}
              onMouseLeave={(e) => {
                e.currentTarget.style.background = "none";
              }}
            >
              <Minimize2 size={16} />
            </button>
          </Tooltip>
        )}
        <div
          style={{
            width: 1,
            height: 16,
            background: token.colorBorderSecondary,
            margin: "0 2px",
          }}
        />
        <Tooltip title={t("wiki.graph.fullscreen") || t("wiki.graph.fitView")}>
          <button
            onClick={handleFullscreenToggle}
            style={{
              background: "none",
              border: "none",
              cursor: "pointer",
              padding: 4,
              borderRadius: 6,
              display: "flex",
              alignItems: "center",
              color: token.colorTextSecondary,
            }}
            onMouseEnter={(e) => {
              e.currentTarget.style.background = token.colorBgTextHover;
            }}
            onMouseLeave={(e) => {
              e.currentTarget.style.background = "none";
            }}
          >
            <Fullscreen size={16} />
          </button>
        </Tooltip>
        <Tooltip title={t("wiki.graph.exportPNG")}>
          <button
            onClick={handleExportPNG}
            style={{
              background: "none",
              border: "none",
              cursor: "pointer",
              padding: 4,
              borderRadius: 6,
              display: "flex",
              alignItems: "center",
              color: token.colorTextSecondary,
            }}
            onMouseEnter={(e) => {
              e.currentTarget.style.background = token.colorBgTextHover;
            }}
            onMouseLeave={(e) => {
              e.currentTarget.style.background = "none";
            }}
          >
            <Download size={16} />
          </button>
        </Tooltip>
        <Tooltip title={t("wiki.graph.relayout")}>
          <button
            onClick={handleRelaunchLayout}
            style={{
              background: "none",
              border: "none",
              cursor: "pointer",
              padding: 4,
              borderRadius: 6,
              display: "flex",
              alignItems: "center",
              color: token.colorTextSecondary,
            }}
            onMouseEnter={(e) => {
              e.currentTarget.style.background = token.colorBgTextHover;
            }}
            onMouseLeave={(e) => {
              e.currentTarget.style.background = "none";
            }}
          >
            <RefreshCw size={16} />
          </button>
        </Tooltip>
      </div>
    </div>
  );
}

export const GraphView = memo(GraphViewInner);

// 保留向后兼容：旧代码可能 import { GraphView }
export { GraphView as default };

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
import { Card, Empty, Segmented, Select, Space, Tag, theme, Typography } from "antd";
import Graph from "graphology";
import { Maximize2, Minimize2, ZoomIn, ZoomOut } from "lucide-react";
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

  const [dimensions, setDimensions] = useState({ width: 800, height: 600 });
  const [layoutMode, setLayoutMode] = useState<LayoutMode>("force");
  const [layoutRunning, setLayoutRunning] = useState(false);
  const [stats, setStats] = useState({ visible: 0, total: 0, edges: 0 });

  const nodeColors = useMemo(() => getNodeColorMap(token), [token]);
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
        (e) => visibleNodeIds.has(e.source) && visibleNodeIds.has(e.target),
      ),
    [data.edges, visibleNodeIds],
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
    sigmaInstance.on("enterNode", ({ node }) => onNodeHover?.(node));
    sigmaInstance.on("leaveNode", () => onNodeHover?.(null));
    sigmaInstance.on("clickStage", () => onDeselect?.());

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

    // 清空
    graph.clear();
    nodeIndexRef.current = new Map();

    // 初始位置：网格布局，作为 worker 跑完前的占位
    const initialPositions = gridLayout(filteredNodes, dimensions.width, dimensions.height);

    for (const node of filteredNodes) {
      const pos = initialPositions.get(node.id) ?? { x: 0, y: 0 };
      const color = getNodeColor(node, communities, token);
      const size = getNodeSize(node);
      graph.addNode(node.id, {
        x: pos.x,
        y: pos.y,
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

    // 触发布局计算
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
        console.warn("[GraphView] 布局 worker 错误:", msg.message);
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

    // hierarchy 模式：直接用网格，不跑 worker
    if (layoutMode === "hierarchy") {
      const positions = gridLayout(filteredNodes, dimensions.width, dimensions.height);
      applyPositions(Array.from(positions.entries()).map(([id, p]) => ({ id, x: p.x, y: p.y })));
      return;
    }

    // 小图同步跑：worker 启动开销大于计算本身
    if (filteredNodes.length < LAYOUT_FORCE_THRESHOLD) {
      applyPositions(
        filteredNodes.map((n) => {
          const p = graph.getNodeAttributes(n.id);
          return { id: n.id, x: p.x, y: p.y };
        }),
      );
      // 简单抖动一次，避免节点重叠
      // 不引入 d3-force，直接用网格结果即可
      return;
    }

    // 大图走 worker
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
      gravity: layoutMode === "radial" ? 0.5 : 1.0,
      slowDown: 4,
      scaling: 1.0,
      linLogMode: layoutMode === "radial",
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

  // ── 应用位置 ──
  const applyPositions = useCallback(
    (positions: Array<{ id: string; x: number; y: number }>) => {
      const graph = graphRef.current;
      const sigmaInstance = sigmaRef.current;
      if (!graph || !sigmaInstance) { return; }
      for (const p of positions) {
        if (graph.hasNode(p.id)) {
          graph.setNodeAttribute(p.id, "x", p.x);
          graph.setNodeAttribute(p.id, "y", p.y);
        }
      }
      sigmaInstance.refresh();
      // 自适应视图
      setTimeout(() => {
        sigmaInstance.getCamera().animate(
          { ...sigmaInstance.getCamera().getState(), ratio: 1.2 },
          { duration: 300 },
        );
      }, 50);
    },
    [],
  );

  // ── 高亮/选中 ──
  useEffect(() => {
    const graph = graphRef.current;
    const sigmaInstance = sigmaRef.current;
    if (!graph || !sigmaInstance) { return; }

    const hasHighlights = highlightedNodeIds && highlightedNodeIds.size > 0;
    graph.forEachNode((node, attrs) => {
      const original = nodeIndexRef.current.get(node);
      if (!original) { return; }
      const baseColor = getNodeColor(original, communities, token);
      const isSelected = selectedNodeId === node;
      const isHighlighted = !hasHighlights || highlightedNodeIds?.has(node);
      const color = isHighlighted ? baseColor : `${baseColor}40`;
      const size = isSelected ? getNodeSize(original) * 1.5 : getNodeSize(original);
      if (attrs.color !== color) { graph.setNodeAttribute(node, "color", color); }
      if (attrs.size !== size) { graph.setNodeAttribute(node, "size", size); }
    });
    sigmaInstance.refresh();
  }, [highlightedNodeIds, selectedNodeId, communities, token]);

  // ── 工具栏 ──
  const handleZoomIn = useCallback(() => {
    sigmaRef.current?.getCamera().animatedZoom({ duration: 300 });
  }, []);
  const handleZoomOut = useCallback(() => {
    sigmaRef.current?.getCamera().animatedUnzoom({ duration: 300 });
  }, []);
  const handleFitAll = useCallback(() => {
    sigmaRef.current?.getCamera().animatedReset({ duration: 600 });
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

      {/* 左上：过滤面板 */}
      <div
        style={{
          position: "absolute",
          top: 12,
          left: 12,
          zIndex: 10,
          minWidth: 220,
        }}
      >
        <Card
          size="small"
          style={{
            borderRadius: 10,
            backdropFilter: "blur(12px)",
            background: `${token.colorBgContainer}ee`,
            border: `1px solid ${token.colorBorderSecondary}40`,
          }}
        >
          <Space orientation="vertical" size="small" style={{ width: "100%" }}>
            <div
              style={{
                display: "flex",
                alignItems: "center",
                justifyContent: "space-between",
              }}
            >
              <Typography.Text strong style={{ fontSize: 12 }}>
                {t("wiki.graph.filters")}
              </Typography.Text>
              <Segmented
                size="small"
                value={layoutMode}
                onChange={(v) => setLayoutMode(v as LayoutMode)}
                options={[
                  { label: "Force", value: "force" },
                  { label: "Radial", value: "radial" },
                  { label: "Dense", value: "hierarchy" },
                ]}
              />
            </div>
            <Select
              mode="multiple"
              placeholder={t("wiki.graph.filterByTags")}
              style={{ width: "100%" }}
              allowClear
              value={filters?.tags}
              onChange={(values) => onFiltersChange?.({ tags: values, types: filters?.types })}
              options={allTags.map((tag) => ({ label: tag, value: tag }))}
              maxTagCount={3}
            />
            <div style={{ display: "flex", gap: 6, flexWrap: "wrap" }}>
              {(["note", "concept", "entity", "source"] as GraphNodeType[]).map((type) => (
                <Tag
                  key={type}
                  color={nodeColors[type]}
                  style={{ fontSize: 12, margin: 0 }}
                >
                  {type}: {data.nodes.filter((n) => n.type === type).length}
                </Tag>
              ))}
            </div>
            {communities && communities.size > 0 && (
              <div
                style={{
                  display: "flex",
                  gap: 4,
                  flexWrap: "wrap",
                  marginTop: 4,
                }}
              >
                <Typography.Text
                  type="secondary"
                  style={{ fontSize: 10, width: "100%" }}
                >
                  {t("wiki.graph.communities")}
                </Typography.Text>
                {Array.from(new Set(communities.values()))
                  .slice(0, 8)
                  .map((cid) => (
                    <Tag
                      key={cid}
                      color={communityPalette[cid % communityPalette.length]}
                      style={{ fontSize: 10 }}
                    >
                      C{cid}
                    </Tag>
                  ))}
              </div>
            )}
            {layoutRunning && (
              <Typography.Text type="secondary" style={{ fontSize: 10 }}>
                {t("wiki.graph.layoutRunning") ?? "正在计算布局…"}
              </Typography.Text>
            )}
          </Space>
        </Card>
      </div>

      {/* 右上：统计面板 */}
      <div
        style={{
          position: "absolute",
          top: 12,
          right: 12,
          zIndex: 10,
        }}
      >
        <Card
          size="small"
          style={{
            borderRadius: 10,
            backdropFilter: "blur(12px)",
            background: `${token.colorBgContainer}ee`,
            border: `1px solid ${token.colorBorderSecondary}40`,
          }}
        >
          <Space orientation="vertical" size="small">
            <Typography.Text type="secondary" style={{ fontSize: 12 }}>
              {t("wiki.graph.stats")}
            </Typography.Text>
            <Typography.Text style={{ fontSize: 12 }}>
              {t("wiki.graph.nodes")}: {stats.visible} / {stats.total}
            </Typography.Text>
            <Typography.Text style={{ fontSize: 12 }}>
              {t("wiki.graph.edges")}: {stats.edges}
            </Typography.Text>
            {highlightedNodeIds && highlightedNodeIds.size > 0 && (
              <Typography.Text type="secondary" style={{ fontSize: 12 }}>
                Highlighted: {highlightedNodeIds.size}
              </Typography.Text>
            )}
          </Space>
        </Card>
      </div>

      {/* 右下：图例 */}
      <div
        style={{
          position: "absolute",
          bottom: 12,
          right: 12,
          zIndex: 10,
        }}
      >
        <Card
          size="small"
          style={{
            borderRadius: 10,
            backdropFilter: "blur(12px)",
            background: `${token.colorBgContainer}ee`,
            border: `1px solid ${token.colorBorderSecondary}40`,
            padding: "4px 8px",
          }}
        >
          <div
            style={{
              display: "flex",
              gap: 8,
              flexWrap: "wrap",
              fontSize: 10,
            }}
          >
            {(Object.keys(edgeTypeStyles) as GraphEdgeType[]).map((et) => {
              const s = edgeTypeStyles[et];
              return (
                <span
                  key={et}
                  style={{ display: "flex", alignItems: "center", gap: 3 }}
                >
                  <svg width="20" height="6">
                    <line
                      x1="0"
                      y1="3"
                      x2="20"
                      y2="3"
                      stroke={resolveColor(s.color)}
                      strokeWidth={s.width}
                    />
                  </svg>
                  <span style={{ color: resolveColor(s.color) }}>
                    {t(edgeTypeLabels[et])}
                  </span>
                </span>
              );
            })}
          </div>
        </Card>
      </div>

      {/* 底部中：工具栏 */}
      <div
        style={{
          position: "absolute",
          bottom: 12,
          left: "50%",
          transform: "translateX(-50%)",
          zIndex: 10,
          display: "flex",
          alignItems: "center",
          gap: 4,
          padding: "4px 8px",
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
      </div>
    </div>
  );
}

export const GraphView = memo(GraphViewInner);

// 保留向后兼容：旧代码可能 import { GraphView }
export { GraphView as default };

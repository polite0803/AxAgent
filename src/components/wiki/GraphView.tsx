import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import ReactFlow, {
  Background,
  ConnectionMode,
  Edge,
  EdgeLabelRenderer,
  EdgeProps,
  getBezierPath,
  MiniMap,
  Node,
  NodeTypes,
  Panel,
  ReactFlowProvider,
  useEdgesState,
  useNodesState,
  useReactFlow,
} from "reactflow";
import "reactflow/dist/style.css";
import { Card, Empty, Segmented, Select, Space, Tag, theme, Tooltip, Typography } from "antd";
import {
  forceCenter,
  forceCollide,
  forceLink,
  forceManyBody,
  forceRadial,
  forceSimulation,
  type SimulationLinkDatum,
  type SimulationNodeDatum,
} from "d3-force";
import { Maximize2, Minimize2, ZoomIn, ZoomOut } from "lucide-react";
import { useTranslation } from "react-i18next";

const { Text } = Typography;

export type GraphNodeType = "note" | "concept" | "entity" | "source";

export type GraphEdgeType = "link" | "backlink" | "reference" | "derived_from" | "contradicts";

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
  onFiltersChange?: (filters: { tags?: string[]; types?: GraphNodeType[] }) => void;
  showMinimap?: boolean;
  communities?: Map<string, number>;
}

const nodeColors: Record<GraphNodeType, string> = {
  note: "#1890ff",
  concept: "#52c41a",
  entity: "#fa8c16",
  source: "#eb2f96",
};

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

const edgeTypeStyles: Record<
  GraphEdgeType,
  { stroke: string; strokeWidth: number; dashArray: string | undefined; animated: boolean }
> = {
  link: { stroke: "#d9d9d9", strokeWidth: 1, dashArray: undefined, animated: false },
  backlink: { stroke: "#1890ff", strokeWidth: 2, dashArray: undefined, animated: true },
  reference: { stroke: "#52c41a", strokeWidth: 1.5, dashArray: "8,4", animated: false },
  derived_from: { stroke: "#fa8c16", strokeWidth: 1.5, dashArray: "2,4", animated: false },
  contradicts: { stroke: "#ff4d4f", strokeWidth: 2, dashArray: "4,4", animated: false },
};

const edgeTypeLabels: Record<GraphEdgeType, string> = {
  link: "wiki.graph.edgeType.link",
  backlink: "wiki.graph.edgeType.backlink",
  reference: "wiki.graph.edgeType.reference",
  derived_from: "wiki.graph.edgeType.derived",
  contradicts: "wiki.graph.edgeType.contradicts",
};

function getNodeColor(node: GraphNode, communities?: Map<string, number>): string {
  if (communities && communities.has(node.id)) {
    const communityId = communities.get(node.id)!;
    return communityPalette[communityId % communityPalette.length];
  }
  return nodeColors[node.type] || nodeColors.note;
}

interface SimNode {
  id: string;
  x: number;
  y: number;
}

interface SimLink {
  source: string | SimNode;
  target: string | SimNode;
}

function computeLayout(
  nodes: GraphNode[],
  edges: GraphEdge[],
  mode: LayoutMode,
  width: number,
  height: number,
): Map<string, { x: number; y: number }> {
  const cx = width / 2;
  const cy = height / 2;
  const simNodes: SimNode[] = nodes.map((node) => ({
    id: node.id,
    x: node.x ?? Math.random() * width,
    y: node.y ?? Math.random() * height,
  }));

  const simLinks: SimLink[] = edges.map((edge) => ({
    source: edge.source,
    target: edge.target,
  }));

  const simulation = forceSimulation<SimulationNodeDatum>(simNodes as SimulationNodeDatum[])
    .force("collide", forceCollide(70));

  if (mode === "force") {
    simulation
      .force(
        "link",
        forceLink<SimulationNodeDatum, SimulationLinkDatum<SimulationNodeDatum>>(
          simLinks as SimulationLinkDatum<SimulationNodeDatum>[],
        ).id((d: SimulationNodeDatum) => (d as SimNode).id).distance(120),
      )
      .force("charge", forceManyBody().strength(-250))
      .force("center", forceCenter(cx, cy));
  } else if (mode === "radial") {
    simulation
      .force(
        "link",
        forceLink<SimulationNodeDatum, SimulationLinkDatum<SimulationNodeDatum>>(
          simLinks as SimulationLinkDatum<SimulationNodeDatum>[],
        ).id((d: SimulationNodeDatum) => (d as SimNode).id).distance(80),
      )
      .force("radial", forceRadial(Math.min(width, height) * 0.3, cx, cy).strength(0.8))
      .force("center", forceCenter(cx, cy));
  } else {
    simulation
      .force(
        "link",
        forceLink<SimulationNodeDatum, SimulationLinkDatum<SimulationNodeDatum>>(
          simLinks as SimulationLinkDatum<SimulationNodeDatum>[],
        ).id((d: SimulationNodeDatum) => (d as SimNode).id).distance(100).strength(0.5),
      )
      .force("charge", forceManyBody().strength(-400))
      .force("center", forceCenter(cx, cy));
  }

  simulation.tick(300);

  const positions = new Map<string, { x: number; y: number }>();
  for (const node of simNodes) {
    positions.set(node.id, { x: node.x, y: node.y });
  }
  return positions;
}

function WikiEdgeComponent({
  id,
  sourceX,
  sourceY,
  targetX,
  targetY,
  sourcePosition,
  targetPosition,
  data,
  selected,
}: EdgeProps<{ edgeType: GraphEdgeType }>) {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const [edgePath, labelX, labelY] = getBezierPath({
    sourceX,
    sourceY,
    sourcePosition,
    targetX,
    targetY,
    targetPosition,
  });

  const edgeType = data?.edgeType || "link";
  const style = edgeTypeStyles[edgeType];
  const isSelected = !!selected;

  return (
    <>
      {edgeType === "contradicts" && (
        <path
          d={edgePath}
          stroke={style.stroke}
          strokeWidth={style.strokeWidth + 2}
          fill="none"
          opacity={0.3}
        />
      )}
      <path
        id={id}
        className="react-flow__edge-path"
        d={edgePath}
        stroke={isSelected ? "#1890ff" : style.stroke}
        strokeWidth={isSelected ? style.strokeWidth + 0.5 : style.strokeWidth}
        fill="none"
        strokeDasharray={style.dashArray}
        opacity={isSelected ? 1 : 0.6}
        style={{ transition: "stroke 0.3s ease, opacity 0.3s ease" }}
      />
      {style.animated && (
        <path
          d={edgePath}
          stroke={style.stroke}
          strokeWidth={style.strokeWidth}
          fill="none"
          strokeDasharray="5,5"
          opacity={0.6}
        >
          <animate attributeName="stroke-dashoffset" from="0" to="10" dur="0.5s" repeatCount="indefinite" />
        </path>
      )}
      {edgeType !== "link" && (
        <EdgeLabelRenderer>
          <div
            style={{
              position: "absolute",
              transform: `translate(-50%, -50%) translate(${labelX}px,${labelY}px)`,
              fontSize: 9,
              color: style.stroke,
              background: `${token.colorBgContainer}dd`,
              padding: "1px 4px",
              borderRadius: 3,
              pointerEvents: "none",
              fontWeight: 500,
            }}
          >
            {t(edgeTypeLabels[edgeType])}
          </div>
        </EdgeLabelRenderer>
      )}
    </>
  );
}

const CustomNode = ({
  data,
  selected,
}: {
  data: GraphNode & {
    onHover?: (id: string | null) => void;
    isHighlighted?: boolean;
    isSelected?: boolean;
    color?: string;
    isExpanded?: boolean;
    entranceVisible?: boolean;
  };
  selected: boolean;
}) => {
  const { token } = theme.useToken();
  const nodeColor = data.color || nodeColors[data.type] || nodeColors.note;
  const isHighlighted = data.isHighlighted !== false;
  const isSelected = data.isSelected || selected;
  const entranceVisible = data.entranceVisible !== false;

  const linkSum = data.linkCount + data.backlinkCount;
  const size = Math.max(120, Math.min(200, 100 + linkSum * 4));

  return (
    <Tooltip
      title={
        <div>
          <div style={{ fontWeight: 600 }}>{data.title}</div>
          <div style={{ fontSize: 12, opacity: 0.8 }}>
            →{data.linkCount} outgoing / ←{data.backlinkCount} incoming
          </div>
          <div style={{ fontSize: 11, opacity: 0.6 }}>{data.path}</div>
        </div>
      }
    >
      <div
        className="wiki-graph-node"
        style={{
          padding: "8px 14px",
          borderRadius: 12,
          background: isSelected
            ? `linear-gradient(135deg, ${token.colorBgContainer}f5, ${token.colorBgContainer}ee)`
            : `${token.colorBgContainer}ee`,
          backdropFilter: "blur(12px)",
          border: `1.5px solid ${isSelected ? nodeColor : `${token.colorBorderSecondary}40`}`,
          boxShadow: isSelected
            ? `0 0 0 2px ${nodeColor}25, 0 0 20px ${nodeColor}15, 0 8px 32px rgba(0,0,0,0.1)`
            : `0 2px 8px rgba(0,0,0,0.04)`,
          opacity: entranceVisible ? (isHighlighted ? 1 : 0.15) : 0,
          minWidth: size * 0.6,
          maxWidth: size,
          cursor: "pointer",
          transition: "box-shadow 0.5s cubic-bezier(0.16, 1, 0.3, 1), transform 0.5s cubic-bezier(0.16, 1, 0.3, 1)",
          transform: entranceVisible
            ? (isSelected ? "scale(1.05)" : "scale(1)")
            : "scale(0.3)",
          position: "relative",
          overflow: "hidden",
        }}
        onMouseEnter={(e) => {
          e.currentTarget.style.transform = "scale(1.06)";
          e.currentTarget.style.boxShadow =
            `0 0 0 2px ${nodeColor}30, 0 4px 24px ${nodeColor}20, 0 8px 24px rgba(0,0,0,0.08)`;
          e.currentTarget.style.borderColor = nodeColor;
        }}
        onMouseLeave={(e) => {
          e.currentTarget.style.transform = isSelected ? "scale(1.05)" : "scale(1)";
          e.currentTarget.style.boxShadow = isSelected
            ? `0 0 0 2px ${nodeColor}25, 0 0 20px ${nodeColor}15, 0 8px 32px rgba(0,0,0,0.1)`
            : `0 2px 8px rgba(0,0,0,0.04)`;
          e.currentTarget.style.borderColor = isSelected ? nodeColor : `${token.colorBorderSecondary}40`;
        }}
      >
        {isSelected && (
          <div
            style={{
              position: "absolute",
              inset: 0,
              background: `radial-gradient(circle at center, ${nodeColor}08, transparent 70%)`,
              pointerEvents: "none",
            }}
          />
        )}
        <div style={{ display: "flex", alignItems: "center", gap: 6, marginBottom: 4 }}>
          <div
            style={{
              width: 8,
              height: 8,
              borderRadius: "50%",
              backgroundColor: nodeColor,
              boxShadow: `0 0 6px ${nodeColor}60`,
              flexShrink: 0,
            }}
          />
          <Text strong ellipsis style={{ fontSize: 13, flex: 1, minWidth: 0 }}>{data.title}</Text>
        </div>
        {data.tags.length > 0 && (
          <div style={{ display: "flex", gap: 3, flexWrap: "wrap" }}>
            {data.tags.slice(0, 3).map((tag) => (
              <span
                key={tag}
                style={{
                  fontSize: 9,
                  padding: "1px 6px",
                  borderRadius: 999,
                  background: `${nodeColor}12`,
                  color: nodeColor,
                  fontWeight: 500,
                }}
              >
                {tag}
              </span>
            ))}
            {data.tags.length > 3 && (
              <span
                style={{
                  fontSize: 9,
                  padding: "1px 5px",
                  borderRadius: 999,
                  background: `${token.colorBorderSecondary}25`,
                  color: token.colorTextSecondary,
                }}
              >
                +{data.tags.length - 3}
              </span>
            )}
          </div>
        )}
        <div
          style={{
            display: "flex",
            justifyContent: "space-between",
            marginTop: 4,
            fontSize: 10,
            color: token.colorTextTertiary,
          }}
        >
          <span>→{data.linkCount}</span>
          <span>←{data.backlinkCount}</span>
        </div>
      </div>
    </Tooltip>
  );
};

const nodeTypes: NodeTypes = {
  customNode: CustomNode,
};

const edgeTypes = {
  wikiEdge: WikiEdgeComponent,
};

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
  showMinimap = true,
  communities,
}: GraphViewProps) {
  const { token } = theme.useToken();
  const { t } = useTranslation();
  const containerRef = useRef<HTMLDivElement>(null);
  const [dimensions, setDimensions] = useState({ width: 800, height: 600 });
  const [expandedNodeIds, setExpandedNodeIds] = useState<Set<string>>(new Set());
  const [layoutMode, setLayoutMode] = useState<LayoutMode>("force");
  const [entranceComplete, setEntranceComplete] = useState(false);
  const reactFlowInstance = useReactFlow();

  useEffect(() => {
    const updateDimensions = () => {
      if (containerRef.current) {
        setDimensions({
          width: containerRef.current.clientWidth,
          height: containerRef.current.clientHeight,
        });
      }
    };
    updateDimensions();
    const observer = new ResizeObserver(updateDimensions);
    if (containerRef.current) {
      observer.observe(containerRef.current);
    }
    return () => observer.disconnect();
  }, []);

  useEffect(() => {
    if (data.nodes.length > 0 && !entranceComplete) {
      const timer = setTimeout(() => setEntranceComplete(true), 150);
      return () => clearTimeout(timer);
    }
  }, [data.nodes.length, entranceComplete]);

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        onDeselect?.();
      }
      if ((e.key === "Delete" || e.key === "Backspace") && selectedNodeId) {
        const target = e.target as HTMLElement;
        if (target.tagName === "INPUT" || target.tagName === "TEXTAREA" || target.isContentEditable) {
          return;
        }
        onDeleteNode?.(selectedNodeId);
      }
    };
    containerRef.current?.addEventListener("keydown", handleKeyDown);
    return () => containerRef.current?.removeEventListener("keydown", handleKeyDown);
  }, [selectedNodeId, onDeleteNode, onDeselect]);

  const hasHighlights = highlightedNodeIds && highlightedNodeIds.size > 0;

  const neighborMap = useMemo(() => {
    const map = new Map<string, Set<string>>();
    for (const edge of data.edges) {
      if (!map.has(edge.source)) { map.set(edge.source, new Set()); }
      if (!map.has(edge.target)) { map.set(edge.target, new Set()); }
      map.get(edge.source)!.add(edge.target);
      map.get(edge.target)!.add(edge.source);
    }
    return map;
  }, [data.edges]);

  const allNodeIds = useMemo(() => new Set(data.nodes.map((n) => n.id)), [data.nodes]);

  const expandedNeighborIds = useMemo(() => {
    const ids = new Set<string>();
    for (const expandedId of expandedNodeIds) {
      const neighbors = neighborMap.get(expandedId);
      if (neighbors) {
        for (const nid of neighbors) {
          if (!allNodeIds.has(nid)) { ids.add(nid); }
        }
      }
    }
    return ids;
  }, [expandedNodeIds, neighborMap, allNodeIds]);

  const filteredNodes = useMemo(() => {
    return data.nodes.filter((node) => {
      if (filters?.tags?.length && !node.tags.some((ft) => filters.tags!.includes(ft))) { return false; }
      if (filters?.pathPrefix && !node.path.startsWith(filters.pathPrefix)) { return false; }
      if (filters?.types?.length && !filters.types.includes(node.type)) { return false; }
      return true;
    });
  }, [data.nodes, filters]);

  const visibleNodeIds = useMemo(() => {
    const ids = new Set(filteredNodes.map((n) => n.id));
    for (const nid of expandedNeighborIds) { ids.add(nid); }
    return ids;
  }, [filteredNodes, expandedNeighborIds]);

  const filteredEdges = useMemo(() => {
    return data.edges.filter((e) => visibleNodeIds.has(e.source) && visibleNodeIds.has(e.target));
  }, [data.edges, visibleNodeIds]);

  const layoutPositions = useMemo(() => {
    return computeLayout(filteredNodes, filteredEdges, layoutMode, dimensions.width, dimensions.height);
  }, [filteredNodes, filteredEdges, layoutMode, dimensions]);

  const initialNodes: Node[] = useMemo(
    () =>
      filteredNodes.map((node) => {
        const pos = layoutPositions.get(node.id) ?? { x: node.x ?? 0, y: node.y ?? 0 };
        return {
          id: node.id,
          type: "customNode",
          position: pos,
          data: {
            ...node,
            onHover: onNodeHover,
            isHighlighted: !hasHighlights || (highlightedNodeIds?.has(node.id) ?? true),
            isSelected: selectedNodeId === node.id,
            color: getNodeColor(node, communities),
            isExpanded: expandedNodeIds.has(node.id),
            entranceVisible: entranceComplete,
          },
        };
      }),
    [
      filteredNodes,
      layoutPositions,
      onNodeHover,
      hasHighlights,
      highlightedNodeIds,
      selectedNodeId,
      communities,
      expandedNodeIds,
      entranceComplete,
    ],
  );

  const initialEdges: Edge[] = useMemo(
    () =>
      filteredEdges.map((edge) => {
        const style = edgeTypeStyles[edge.type] || edgeTypeStyles.link;
        return {
          id: `${edge.source}-${edge.target}`,
          source: edge.source,
          target: edge.target,
          type: "wikiEdge",
          data: { edgeType: edge.type },
          style: {
            stroke: style.stroke,
            strokeWidth: style.strokeWidth,
            opacity: hasHighlights
              ? (highlightedNodeIds?.has(edge.source) && highlightedNodeIds?.has(edge.target) ? 0.8 : 0.08)
              : 0.6,
            transition: "opacity 0.4s ease",
          },
          animated: edge.type === "backlink",
        };
      }),
    [filteredEdges, hasHighlights, highlightedNodeIds],
  );

  const [nodes, setNodes, onNodesChange] = useNodesState(initialNodes);
  const [edges, setEdges, onEdgesChange] = useEdgesState(initialEdges);

  useEffect(() => {
    setNodes(initialNodes);
    setEdges(initialEdges);
  }, [initialNodes, initialEdges, setNodes, setEdges]);

  useEffect(() => {
    if (reactFlowInstance && initialNodes.length > 0) {
      const timer = setTimeout(() => {
        reactFlowInstance.fitView({ padding: 0.15, duration: 600 });
      }, 100);
      return () => clearTimeout(timer);
    }
  }, [layoutMode, reactFlowInstance, initialNodes.length]);

  const onNodeClickHandler = useCallback(
    (_: React.MouseEvent, node: Node) => {
      onNodeClick?.(node.id);
    },
    [onNodeClick],
  );

  const onNodeDoubleClickHandler = useCallback(
    (_: React.MouseEvent, node: Node) => {
      setExpandedNodeIds((prev) => {
        const next = new Set(prev);
        if (next.has(node.id)) { next.delete(node.id); }
        else { next.add(node.id); }
        return next;
      });
      onNodeDoubleClick?.(node.id);
    },
    [onNodeDoubleClick],
  );

  const onNodeContextMenuHandler = useCallback(
    (event: React.MouseEvent, node: Node) => {
      event.preventDefault();
      onContextMenu?.(node.id, { x: event.clientX, y: event.clientY });
    },
    [onContextMenu],
  );

  const onNodeMouseEnter = useCallback(
    (_: React.MouseEvent, node: Node) => {
      onNodeHover?.(node.id);
    },
    [onNodeHover],
  );

  const onNodeMouseLeave = useCallback(() => {
    onNodeHover?.(null);
  }, [onNodeHover]);

  const handleFocusSelected = useCallback(() => {
    if (selectedNodeId && reactFlowInstance) {
      const node = nodes.find((n) => n.id === selectedNodeId);
      if (node) {
        reactFlowInstance.fitView({
          nodes: [node],
          padding: 0.4,
          duration: 500,
        });
      }
    }
  }, [selectedNodeId, reactFlowInstance, nodes]);

  const handleFitAll = useCallback(() => {
    reactFlowInstance?.fitView({ padding: 0.15, duration: 600 });
  }, [reactFlowInstance]);

  const handleZoomIn = useCallback(() => {
    reactFlowInstance?.zoomIn({ duration: 300 });
  }, [reactFlowInstance]);

  const handleZoomOut = useCallback(() => {
    reactFlowInstance?.zoomOut({ duration: 300 });
  }, [reactFlowInstance]);

  const allTags = useMemo(() => {
    const tags = new Set<string>();
    data.nodes.forEach((n) => n.tags.forEach((ft) => tags.add(ft)));
    return Array.from(tags).sort();
  }, [data.nodes]);

  if (data.nodes.length === 0) {
    return (
      <Card style={{ height: "100%", display: "flex", alignItems: "center", justifyContent: "center" }}>
        <Empty description={t("wiki.graph.empty")} />
      </Card>
    );
  }

  return (
    <div
      ref={containerRef}
      tabIndex={0}
      style={{ width: "100%", height: "100%", position: "relative", outline: "none" }}
    >
      <ReactFlow
        nodes={nodes}
        edges={edges}
        onNodesChange={onNodesChange}
        onEdgesChange={onEdgesChange}
        onNodeClick={onNodeClickHandler}
        onNodeDoubleClick={onNodeDoubleClickHandler}
        onNodeContextMenu={onNodeContextMenuHandler}
        onNodeMouseEnter={onNodeMouseEnter}
        onNodeMouseLeave={onNodeMouseLeave}
        nodeTypes={nodeTypes}
        edgeTypes={edgeTypes}
        connectionMode={ConnectionMode.Loose}
        fitView
        fitViewOptions={{ padding: 0.15, duration: 800 }}
        minZoom={0.05}
        maxZoom={4}
        defaultViewport={{ zoom: 1, x: 0, y: 0 }}
        attributionPosition="bottom-left"
        style={{ background: token.colorBgLayout }}
        proOptions={{ hideAttribution: true }}
      >
        <Background gap={20} size={1} color={`${token.colorBorderSecondary}40`} />

        {showMinimap && (
          <MiniMap
            nodeColor={(n) => {
              const graphNode = data.nodes.find((gn) => gn.id === n.id);
              return graphNode ? getNodeColor(graphNode, communities) : nodeColors.note;
            }}
            maskColor={`${token.colorBgContainer}aa`}
            style={{ borderRadius: 8, overflow: "hidden" }}
            pannable
            zoomable
          />
        )}

        <Panel position="top-left">
          <Card
            size="small"
            style={{
              minWidth: 220,
              borderRadius: 10,
              backdropFilter: "blur(12px)",
              background: `${token.colorBgContainer}ee`,
              border: `1px solid ${token.colorBorderSecondary}40`,
            }}
          >
            <Space direction="vertical" size="small" style={{ width: "100%" }}>
              <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
                <Text strong style={{ fontSize: 12 }}>{t("wiki.graph.filters")}</Text>
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
                  <Tag key={type} color={nodeColors[type]} style={{ fontSize: 11, margin: 0 }}>
                    {type}: {data.nodes.filter((n) =>
                      n.type === type
                    ).length}
                  </Tag>
                ))}
              </div>
              {communities && communities.size > 0 && (
                <div style={{ display: "flex", gap: 4, flexWrap: "wrap", marginTop: 4 }}>
                  <Text type="secondary" style={{ fontSize: 10, width: "100%" }}>{t("wiki.graph.communities")}</Text>
                  {Array.from(new Set(communities.values())).slice(0, 8).map((cid) => (
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
            </Space>
          </Card>
        </Panel>

        <Panel position="top-right">
          <Card
            size="small"
            style={{
              borderRadius: 10,
              backdropFilter: "blur(12px)",
              background: `${token.colorBgContainer}ee`,
              border: `1px solid ${token.colorBorderSecondary}40`,
            }}
          >
            <Space direction="vertical" size="small">
              <Text type="secondary" style={{ fontSize: 11 }}>{t("wiki.graph.stats")}</Text>
              <Text style={{ fontSize: 12 }}>
                {t("wiki.graph.nodes")}: {filteredNodes.length} / {data.nodes.length}
              </Text>
              <Text style={{ fontSize: 12 }}>
                {t("wiki.graph.edges")}: {filteredEdges.length} / {data.edges.length}
              </Text>
              {expandedNodeIds.size > 0 && (
                <Text type="secondary" style={{ fontSize: 11 }}>Expanded: {expandedNodeIds.size}</Text>
              )}
              {hasHighlights && (
                <Text type="secondary" style={{ fontSize: 11 }}>Highlighted: {highlightedNodeIds!.size}</Text>
              )}
            </Space>
          </Card>
        </Panel>

        <Panel position="bottom-right">
          <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
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
              <div style={{ display: "flex", gap: 8, flexWrap: "wrap", fontSize: 10 }}>
                {(Object.keys(edgeTypeStyles) as GraphEdgeType[]).map((et) => {
                  const s = edgeTypeStyles[et];
                  return (
                    <span key={et} style={{ display: "flex", alignItems: "center", gap: 3 }}>
                      <svg width="20" height="6">
                        <line
                          x1="0"
                          y1="3"
                          x2="20"
                          y2="3"
                          stroke={s.stroke}
                          strokeWidth={s.strokeWidth}
                          strokeDasharray={s.dashArray}
                        />
                      </svg>
                      <span style={{ color: s.stroke }}>{t(edgeTypeLabels[et])}</span>
                    </span>
                  );
                })}
              </div>
            </Card>
          </div>
        </Panel>

        <Panel position="bottom-center">
          <div
            style={{
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
                  transition: "box-shadow 0.2s, transform 0.2s",
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
                  transition: "box-shadow 0.2s, transform 0.2s",
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
                  transition: "box-shadow 0.2s, transform 0.2s",
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
                    transition: "box-shadow 0.2s, transform 0.2s",
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
        </Panel>
      </ReactFlow>
    </div>
  );
}

export function GraphView(props: GraphViewProps) {
  return (
    <ReactFlowProvider>
      <GraphViewInner {...props} />
    </ReactFlowProvider>
  );
}

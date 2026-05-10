import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import ReactFlow, {
  Background,
  ConnectionMode,
  Controls,
  Edge,
  EdgeLabelRenderer,
  EdgeProps,
  getBezierPath,
  MiniMap,
  Node,
  NodeTypes,
  Panel,
  useEdgesState,
  useNodesState,
} from "reactflow";
import "reactflow/dist/style.css";
import { Card, Empty, Select, Space, Tag, theme, Tooltip, Typography } from "antd";
import { Book, FileText, Hash, Link2 } from "lucide-react";
import { useTranslation } from "react-i18next";
import {
  forceCenter,
  forceCollide,
  forceLink,
  forceManyBody,
  forceSimulation,
  type SimulationLinkDatum,
  type SimulationNodeDatum,
} from "d3-force";

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

export interface GraphViewProps {
  data: GraphData;
  onNodeClick?: (nodeId: string) => void;
  onNodeDoubleClick?: (nodeId: string) => void;
  onNodeHover?: (nodeId: string | null) => void;
  onContextMenu?: (nodeId: string, position: { x: number; y: number }) => void;
  highlightedNodeIds?: Set<string>;
  selectedNodeId?: string | null;
  filters?: {
    tags?: string[];
    pathPrefix?: string;
    types?: GraphNodeType[];
  };
  onFiltersChange?: (filters: { tags?: string[]; types?: GraphNodeType[] }) => void;
  showMinimap?: boolean;
  showControls?: boolean;
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

const edgeTypeStyles: Record<GraphEdgeType, { stroke: string; strokeWidth: number; dashArray: string | undefined; animated: boolean }> = {
  link: { stroke: "#d9d9d9", strokeWidth: 1, dashArray: undefined, animated: false },
  backlink: { stroke: "#1890ff", strokeWidth: 2, dashArray: undefined, animated: true },
  reference: { stroke: "#52c41a", strokeWidth: 1.5, dashArray: "8,4", animated: false },
  derived_from: { stroke: "#fa8c16", strokeWidth: 1.5, dashArray: "2,4", animated: false },
  contradicts: { stroke: "#ff4d4f", strokeWidth: 2, dashArray: "4,4", animated: false },
};

const edgeTypeLabels: Record<GraphEdgeType, string> = {
  link: "link",
  backlink: "backlink",
  reference: "ref",
  derived_from: "derived",
  contradicts: "contra",
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
          style={{ filter: "url(#wavy-filter)" }}
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
        opacity={isSelected ? 1 : 0.7}
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
              background: "rgba(255,255,255,0.85)",
              padding: "1px 4px",
              borderRadius: 3,
              pointerEvents: "none",
              fontWeight: 500,
            }}
          >
            {edgeTypeLabels[edgeType]}
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
  };
  selected: boolean;
}) => {
  const { token } = theme.useToken();
  const nodeColor = data.color || nodeColors[data.type] || nodeColors.note;
  const isHighlighted = data.isHighlighted !== false;
  const isSelected = data.isSelected || selected;

  return (
    <Tooltip
      title={
        <div>
          <div style={{ fontWeight: 600 }}>{data.title}</div>
          <div style={{ fontSize: 12, opacity: 0.8 }}>
            {data.linkCount} outgoing / {data.backlinkCount} incoming
          </div>
          <div style={{ fontSize: 11, opacity: 0.6 }}>{data.path}</div>
          {data.isExpanded && (
            <div style={{ fontSize: 10, opacity: 0.7, marginTop: 2 }}>expanded</div>
          )}
        </div>
      }
    >
      <div
        style={{
          padding: "8px 12px",
          borderRadius: 10,
          background: `${token.colorBgContainer}ee`,
          backdropFilter: "blur(8px)",
          border: `1.5px solid ${isSelected ? nodeColor : `${token.colorBorderSecondary}30`}`,
          boxShadow: isSelected
            ? `0 0 0 1px ${nodeColor}30, 0 4px 24px ${nodeColor}20, 0 8px 16px rgba(0,0,0,0.08)`
            : "0 2px 12px rgba(0,0,0,0.06), 0 1px 3px rgba(0,0,0,0.04)",
          opacity: isHighlighted ? 1 : 0.2,
          minWidth: 120,
          maxWidth: 200,
          cursor: "pointer",
          transition: "all 0.25s cubic-bezier(0.16, 1, 0.3, 1)",
          transform: isSelected ? "scale(1.03)" : "scale(1)",
        }}
        onMouseEnter={(e) => {
          e.currentTarget.style.transform = "scale(1.04)";
          e.currentTarget.style.boxShadow = `0 4px 20px rgba(0,0,0,0.1), 0 2px 6px rgba(0,0,0,0.06)`;
        }}
        onMouseLeave={(e) => {
          e.currentTarget.style.transform = isSelected ? "scale(1.03)" : "scale(1)";
          e.currentTarget.style.boxShadow = isSelected
            ? `0 0 0 1px ${nodeColor}30, 0 4px 24px ${nodeColor}20, 0 8px 16px rgba(0,0,0,0.08)`
            : "0 2px 12px rgba(0,0,0,0.06), 0 1px 3px rgba(0,0,0,0.04)";
        }}
      >
        <div style={{ display: "flex", alignItems: "center", gap: 6, marginBottom: 4 }}>
          {data.type === "note" && <FileText size={14} style={{ color: nodeColor }} />}
          {data.type === "concept" && <Hash size={14} style={{ color: nodeColor }} />}
          {data.type === "entity" && <Book size={14} style={{ color: nodeColor }} />}
          {data.type === "source" && <Link2 size={14} style={{ color: nodeColor }} />}
          <Text strong style={{ fontSize: 13, overflow: "hidden", textOverflow: "ellipsis" }}>
            {data.title}
          </Text>
        </div>
        <div style={{ display: "flex", gap: 3, flexWrap: "wrap" }}>
          {data.tags.slice(0, 3).map((tag) => (
            <span
              key={tag}
              style={{
                fontSize: 9,
                padding: "1px 5px",
                borderRadius: 999,
                background: `${nodeColor}14`,
                color: nodeColor,
                fontWeight: 500,
                letterSpacing: "0.02em",
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
                background: `${token.colorBorderSecondary}30`,
                color: token.colorTextSecondary,
              }}
            >
              +{data.tags.length - 3}
            </span>
          )}
        </div>
        <div
          style={{
            display: "flex",
            justifyContent: "space-between",
            marginTop: 6,
            fontSize: 11,
            color: token.colorTextSecondary,
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

export function GraphView({
  data,
  onNodeClick,
  onNodeDoubleClick,
  onNodeHover,
  onContextMenu,
  highlightedNodeIds,
  selectedNodeId,
  filters,
  onFiltersChange,
  showMinimap = true,
  showControls = true,
  communities,
}: GraphViewProps) {
  const { token } = theme.useToken();
  const { t } = useTranslation();
  const containerRef = useRef<HTMLDivElement>(null);
  const [, setDimensions] = useState({ width: 800, height: 600 });
  const [expandedNodeIds, setExpandedNodeIds] = useState<Set<string>>(new Set());

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

  const hasHighlights = highlightedNodeIds && highlightedNodeIds.size > 0;

  const neighborMap = useMemo(() => {
    const map = new Map<string, Set<string>>();
    for (const edge of data.edges) {
      if (!map.has(edge.source)) map.set(edge.source, new Set());
      if (!map.has(edge.target)) map.set(edge.target, new Set());
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
          if (!allNodeIds.has(nid)) {
            ids.add(nid);
          }
        }
      }
    }
    return ids;
  }, [expandedNodeIds, neighborMap, allNodeIds]);

  const filteredNodes = useMemo(() => {
    return data.nodes.filter((node) => {
      if (filters?.tags?.length && !node.tags.some((t) => filters.tags!.includes(t))) {
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

  const visibleNodeIds = useMemo(() => {
    const ids = new Set(filteredNodes.map((n) => n.id));
    for (const nid of expandedNeighborIds) {
      ids.add(nid);
    }
    return ids;
  }, [filteredNodes, expandedNeighborIds]);

  const filteredEdges = useMemo(() => {
    return data.edges.filter((e) => visibleNodeIds.has(e.source) && visibleNodeIds.has(e.target));
  }, [data.edges, visibleNodeIds]);

  const layoutPositions = useMemo(() => {
    const simNodes: SimNode[] = filteredNodes.map((node) => ({
      id: node.id,
      x: node.x ?? Math.random() * 400,
      y: node.y ?? Math.random() * 400,
    }));

    const simLinks: SimLink[] = filteredEdges.map((edge) => ({
      source: edge.source,
      target: edge.target,
    }));

    const simulation = forceSimulation<SimulationNodeDatum>(simNodes as SimulationNodeDatum[])
      .force(
        "link",
        forceLink<SimulationNodeDatum, SimulationLinkDatum<SimulationNodeDatum>>(simLinks as SimulationLinkDatum<SimulationNodeDatum>[])
          .id((d: SimulationNodeDatum) => (d as SimNode).id)
          .distance(100),
      )
      .force("charge", forceManyBody().strength(-200))
      .force("center", forceCenter(250, 250))
      .force("collide", forceCollide(60))
      .stop();

    simulation.tick(300);

    const positions = new Map<string, { x: number; y: number }>();
    for (const node of simNodes) {
      positions.set(node.id, { x: node.x, y: node.y });
    }
    return positions;
  }, [filteredNodes, filteredEdges]);

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
          },
        };
      }),
    [filteredNodes, layoutPositions, onNodeHover, hasHighlights, highlightedNodeIds, selectedNodeId, communities, expandedNodeIds],
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
              ? (highlightedNodeIds?.has(edge.source) && highlightedNodeIds?.has(edge.target) ? 0.8 : 0.1)
              : 1,
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
        if (next.has(node.id)) {
          next.delete(node.id);
        } else {
          next.add(node.id);
        }
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

  const allTags = useMemo(() => {
    const tags = new Set<string>();
    data.nodes.forEach((n) => n.tags.forEach((t) => tags.add(t)));
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
    <div ref={containerRef} style={{ width: "100%", height: "100%", position: "relative" }}>
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
        attributionPosition="bottom-left"
        style={{ background: token.colorBgLayout }}
      >
        {showControls && <Controls />}
        {showMinimap && (
          <MiniMap
            nodeColor={(n) => {
              const graphNode = data.nodes.find((gn) => gn.id === n.id);
              return graphNode ? getNodeColor(graphNode, communities) : nodeColors.note;
            }}
            maskColor={`${token.colorBgContainer}cc`}
          />
        )}
        <Background gap={16} color={`${token.colorBorderSecondary}`} />

        <Panel position="top-left">
          <Card size="small" style={{ minWidth: 200 }}>
            <Space direction="vertical" size="small" style={{ width: "100%" }}>
              <Text strong style={{ fontSize: 12 }}>
                {t("wiki.graph.filters")}
              </Text>
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
              <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
                {(["note", "concept", "entity", "source"] as GraphNodeType[]).map((type) => (
                  <Tag key={type} color={nodeColors[type]} style={{ fontSize: 11 }}>
                    {type}: {data.nodes.filter((n) =>
                      n.type === type
                    ).length}
                  </Tag>
                ))}
              </div>
              {communities && communities.size > 0 && (
                <div style={{ display: "flex", gap: 4, flexWrap: "wrap", marginTop: 4 }}>
                  <Text type="secondary" style={{ fontSize: 10, width: "100%" }}>Communities</Text>
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
          <Card size="small">
            <Space direction="vertical" size="small">
              <Text type="secondary" style={{ fontSize: 11 }}>
                {t("wiki.graph.stats")}
              </Text>
              <Text>
                {t("wiki.graph.nodes")}: {filteredNodes.length} / {data.nodes.length}
              </Text>
              <Text>
                {t("wiki.graph.edges")}: {filteredEdges.length} / {data.edges.length}
              </Text>
              {expandedNodeIds.size > 0 && (
                <Text type="secondary" style={{ fontSize: 11 }}>
                  Expanded: {expandedNodeIds.size}
                </Text>
              )}
              {hasHighlights && (
                <Text type="secondary" style={{ fontSize: 11 }}>
                  Highlighted: {highlightedNodeIds!.size}
                </Text>
              )}
            </Space>
          </Card>
        </Panel>

        <Panel position="bottom-right">
          <Card size="small" style={{ padding: "4px 8px" }}>
            <div style={{ display: "flex", gap: 8, flexWrap: "wrap", fontSize: 10 }}>
              {(Object.keys(edgeTypeStyles) as GraphEdgeType[]).map((et) => {
                const s = edgeTypeStyles[et];
                return (
                  <span key={et} style={{ display: "flex", alignItems: "center", gap: 3 }}>
                    <svg width="20" height="6">
                      <line
                        x1="0" y1="3" x2="20" y2="3"
                        stroke={s.stroke}
                        strokeWidth={s.strokeWidth}
                        strokeDasharray={s.dashArray}
                      />
                    </svg>
                    <span style={{ color: s.stroke }}>{edgeTypeLabels[et]}</span>
                  </span>
                );
              })}
            </div>
          </Card>
        </Panel>
      </ReactFlow>
    </div>
  );
}

export default GraphView;

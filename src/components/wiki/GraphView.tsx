import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import ReactFlow, {
  Background,
  ConnectionMode,
  Controls,
  Edge,
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

const { Text } = Typography;

export type GraphNodeType = "note" | "concept" | "entity" | "source";

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
  type: "link" | "backlink";
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
}

const nodeColors: Record<GraphNodeType, string> = {
  note: "#1890ff",
  concept: "#52c41a",
  entity: "#fa8c16",
  source: "#eb2f96",
};

const CustomNode = ({
  data,
  selected,
}: {
  data: GraphNode & {
    onHover?: (id: string | null) => void;
    isHighlighted?: boolean;
    isSelected?: boolean;
  };
  selected: boolean;
}) => {
  const { token } = theme.useToken();
  const nodeColor = nodeColors[data.type] || nodeColors.note;
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
}: GraphViewProps) {
  const { token } = theme.useToken();
  const { t } = useTranslation();
  const containerRef = useRef<HTMLDivElement>(null);
  const [, setDimensions] = useState({ width: 800, height: 600 });

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

  const nodeIds = useMemo(() => new Set(filteredNodes.map((n) => n.id)), [filteredNodes]);

  const filteredEdges = useMemo(() => {
    return data.edges.filter((e) => nodeIds.has(e.source) && nodeIds.has(e.target));
  }, [data.edges, nodeIds]);

  const initialNodes: Node[] = useMemo(
    () =>
      filteredNodes.map((node) => ({
        id: node.id,
        type: "customNode",
        position: { x: node.x ?? Math.random() * 500, y: node.y ?? Math.random() * 500 },
        data: {
          ...node,
          onHover: onNodeHover,
          isHighlighted: !hasHighlights || (highlightedNodeIds?.has(node.id) ?? true),
          isSelected: selectedNodeId === node.id,
        },
      })),
    [filteredNodes, onNodeHover, hasHighlights, highlightedNodeIds, selectedNodeId],
  );

  const initialEdges: Edge[] = useMemo(
    () =>
      filteredEdges.map((edge) => ({
        id: `${edge.source}-${edge.target}`,
        source: edge.source,
        target: edge.target,
        type: "smoothstep",
        style: {
          stroke: edge.type === "backlink" ? "#1890ff" : "#d9d9d9",
          strokeWidth: edge.type === "backlink" ? 2 : 1,
          opacity: hasHighlights
            ? (highlightedNodeIds?.has(edge.source) && highlightedNodeIds?.has(edge.target) ? 0.8 : 0.1)
            : 1,
        },
        animated: edge.type === "backlink",
      })),
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
              return graphNode ? nodeColors[graphNode.type] : nodeColors.note;
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
              {hasHighlights && (
                <Text type="secondary" style={{ fontSize: 11 }}>
                  Highlighted: {highlightedNodeIds!.size}
                </Text>
              )}
            </Space>
          </Card>
        </Panel>
      </ReactFlow>
    </div>
  );
}

export default GraphView;

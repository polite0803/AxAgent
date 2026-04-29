import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import ReactFlow, {
  Background,
  Controls,
  MiniMap,
  Node,
  Edge,
  useNodesState,
  useEdgesState,
  ConnectionMode,
  NodeTypes,
  Panel,
} from 'reactflow';
import 'reactflow/dist/style.css';
import { theme, Select, Tag, Tooltip, Card, Typography, Space, Empty } from 'antd';
import { useTranslation } from 'react-i18next';
import { FileText, Book, Hash, Link2 } from 'lucide-react';

const { Text } = Typography;

export type GraphNodeType = 'note' | 'concept' | 'entity' | 'source';

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
  type: 'link' | 'backlink';
}

export interface GraphData {
  nodes: GraphNode[];
  edges: GraphEdge[];
}

export interface GraphViewProps {
  data: GraphData;
  onNodeClick?: (nodeId: string) => void;
  onNodeHover?: (nodeId: string | null) => void;
  filters?: {
    tags?: string[];
    pathPrefix?: string;
    types?: GraphNodeType[];
  };
  showMinimap?: boolean;
  showControls?: boolean;
}

const nodeColors: Record<GraphNodeType, string> = {
  note: '#1890ff',
  concept: '#52c41a',
  entity: '#fa8c16',
  source: '#eb2f96',
};

const CustomNode = ({
  data,
  selected,
}: {
  data: GraphNode & { onHover?: (id: string | null) => void };
  selected: boolean;
}) => {
  const { token } = theme.useToken();
  const nodeColor = nodeColors[data.type] || nodeColors.note;

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
          padding: '8px 12px',
          borderRadius: 8,
          background: token.colorBgContainer,
          border: `2px solid ${selected ? nodeColor : 'transparent'}`,
          boxShadow: selected
            ? `0 0 0 2px ${nodeColor}33, 0 4px 12px rgba(0,0,0,0.15)`
            : '0 2px 8px rgba(0,0,0,0.1)',
          minWidth: 120,
          maxWidth: 200,
          cursor: 'pointer',
          transition: 'all 0.2s ease',
        }}
      >
        <div style={{ display: 'flex', alignItems: 'center', gap: 6, marginBottom: 4 }}>
          {data.type === 'note' && <FileText size={14} style={{ color: nodeColor }} />}
          {data.type === 'concept' && <Hash size={14} style={{ color: nodeColor }} />}
          {data.type === 'entity' && <Book size={14} style={{ color: nodeColor }} />}
          {data.type === 'source' && <Link2 size={14} style={{ color: nodeColor }} />}
          <Text strong style={{ fontSize: 13, overflow: 'hidden', textOverflow: 'ellipsis' }}>
            {data.title}
          </Text>
        </div>
        <div style={{ display: 'flex', gap: 4, flexWrap: 'wrap' }}>
          {data.tags.slice(0, 3).map((tag) => (
            <Tag key={tag} style={{ fontSize: 10, margin: 0 }}>
              {tag}
            </Tag>
          ))}
          {data.tags.length > 3 && (
            <Tag style={{ fontSize: 10, margin: 0 }}>+{data.tags.length - 3}</Tag>
          )}
        </div>
        <div
          style={{
            display: 'flex',
            justifyContent: 'space-between',
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
  onNodeHover,
  filters,
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
        type: 'customNode',
        position: { x: node.x ?? Math.random() * 500, y: node.y ?? Math.random() * 500 },
        data: { ...node, onHover: onNodeHover },
      })),
    [filteredNodes, onNodeHover]
  );

  const initialEdges: Edge[] = useMemo(
    () =>
      filteredEdges.map((edge) => ({
        id: `${edge.source}-${edge.target}`,
        source: edge.source,
        target: edge.target,
        type: 'smoothstep',
        style: {
          stroke: edge.type === 'backlink' ? '#1890ff' : '#d9d9d9',
          strokeWidth: edge.type === 'backlink' ? 2 : 1,
        },
        animated: edge.type === 'backlink',
      })),
    [filteredEdges]
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
    [onNodeClick]
  );

  const onNodeMouseEnter = useCallback(
    (_: React.MouseEvent, node: Node) => {
      onNodeHover?.(node.id);
    },
    [onNodeHover]
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
      <Card style={{ height: '100%', display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
        <Empty description={t('wiki.graph.empty')} />
      </Card>
    );
  }

  return (
    <div ref={containerRef} style={{ width: '100%', height: '100%', position: 'relative' }}>
      <ReactFlow
        nodes={nodes}
        edges={edges}
        onNodesChange={onNodesChange}
        onEdgesChange={onEdgesChange}
        onNodeClick={onNodeClickHandler}
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
            <Space direction="vertical" size="small" style={{ width: '100%' }}>
              <Text strong style={{ fontSize: 12 }}>
                {t('wiki.graph.filters')}
              </Text>
              <Select
                mode="multiple"
                placeholder={t('wiki.graph.filterByTags')}
                style={{ width: '100%' }}
                allowClear
                options={allTags.map((tag) => ({ label: tag, value: tag }))}
                maxTagCount={3}
              />
              <div style={{ display: 'flex', gap: 8, flexWrap: 'wrap' }}>
                {(['note', 'concept', 'entity', 'source'] as GraphNodeType[]).map((type) => (
                  <Tag key={type} color={nodeColors[type]} style={{ fontSize: 11 }}>
                    {type}: {data.nodes.filter((n) => n.type === type).length}
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
                {t('wiki.graph.stats')}
              </Text>
              <Text>
                {t('wiki.graph.nodes')}: {filteredNodes.length} / {data.nodes.length}
              </Text>
              <Text>
                {t('wiki.graph.edges')}: {filteredEdges.length} / {data.edges.length}
              </Text>
            </Space>
          </Card>
        </Panel>
      </ReactFlow>
    </div>
  );
}

export default GraphView;
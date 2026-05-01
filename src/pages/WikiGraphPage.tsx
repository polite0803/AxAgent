import { useEffect, useState, useCallback } from 'react';
import { Card, Typography, Space, Button, message, Spin, Select, Tag, Empty } from 'antd';
import { ArrowLeftOutlined, ReloadOutlined } from '@ant-design/icons';
import { useNavigate, useParams } from 'react-router-dom';
import { GraphView, GraphData, GraphNodeType } from '@/components/wiki/GraphView';
import { invoke } from '@/lib/invoke';
import { useTranslation } from 'react-i18next';

const { Title, Text } = Typography;

export function WikiGraphPage() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const { wikiId } = useParams<{ wikiId: string }>();

  const [graphData, setGraphData] = useState<GraphData | null>(null);
  const [loading, setLoading] = useState(true);
  const [filters, setFilters] = useState<{
    tags?: string[];
    types?: GraphNodeType[];
  }>({});
  const [hoveredNodeId, setHoveredNodeId] = useState<string | null>(null);

  useEffect(() => {
    loadGraphData();
  }, [wikiId]);

  const loadGraphData = async () => {
    if (!wikiId) {
      message.error(t('wiki.graph.noWikiId'));
      return;
    }

    setLoading(true);
    try {
      const data = await invoke<GraphData>('get_wiki_graph', { wikiId });
      setGraphData(data);
    } catch (e) {
      message.error(t('wiki.graph.loadError', { error: String(e) }));
    } finally {
      setLoading(false);
    }
  };

  const handleNodeClick = useCallback(
    (nodeId: string) => {
      navigate(`/wiki/${wikiId}/note/${nodeId}`);
    },
    [navigate, wikiId]
  );

  const handleNodeHover = useCallback((nodeId: string | null) => {
    setHoveredNodeId(nodeId);
  }, []);

  const allTags = graphData
    ? Array.from(new Set(graphData.nodes.flatMap((n) => n.tags))).sort()
    : [];

  const nodeTypeCounts = graphData
    ? {
        note: graphData.nodes.filter((n) => n.type === 'note').length,
        concept: graphData.nodes.filter((n) => n.type === 'concept').length,
        entity: graphData.nodes.filter((n) => n.type === 'entity').length,
        source: graphData.nodes.filter((n) => n.type === 'source').length,
      }
    : { note: 0, concept: 0, entity: 0, source: 0 };

  if (loading) {
    return (
      <div style={{ height: '100vh', display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
        <Spin size="large" tip={t('wiki.graph.loading')} />
      </div>
    );
  }

  if (!graphData || graphData.nodes.length === 0) {
    return (
      <div style={{ padding: 24 }}>
        <Card style={{ marginBottom: 16 }}>
          <Space>
            <Button icon={<ArrowLeftOutlined />} onClick={() => navigate(-1)}>
              {t('wiki.common.back')}
            </Button>
            <Button icon={<ReloadOutlined />} onClick={loadGraphData}>
              {t('wiki.common.refresh')}
            </Button>
          </Space>
        </Card>
        <Empty description={t('wiki.graph.empty')} style={{ marginTop: 100 }} />
      </div>
    );
  }

  return (
    <div style={{ height: '100vh', display: 'flex', flexDirection: 'column' }}>
      <Card
        style={{ flexShrink: 0 }}
        bodyStyle={{ padding: '12px 16px' }}
      >
        <Space style={{ width: '100%', justifyContent: 'space-between' }}>
          <Space>
            <Button icon={<ArrowLeftOutlined />} onClick={() => navigate(-1)} />
            <Title level={4} style={{ margin: 0 }}>
              {t('wiki.graph.title')}
            </Title>
            <Tag color="blue">{graphData.nodes.length} nodes</Tag>
            <Tag>{graphData.edges.length} edges</Tag>
          </Space>

          <Space>
            <Select
              mode="multiple"
              placeholder={t('wiki.graph.filterByTags')}
              style={{ minWidth: 200 }}
              allowClear
              maxTagCount={2}
              options={allTags.map((tag) => ({ label: tag, value: tag }))}
              onChange={(values) => setFilters((f) => ({ ...f, tags: values }))}
            />
            <Select
              mode="multiple"
              placeholder={t('wiki.graph.filterByType')}
              style={{ minWidth: 150 }}
              allowClear
              maxTagCount={4}
              options={[
                { label: `${t('wiki.graph.type.note')} (${nodeTypeCounts.note})`, value: 'note' },
                { label: `${t('wiki.graph.type.concept')} (${nodeTypeCounts.concept})`, value: 'concept' },
                { label: `${t('wiki.graph.type.entity')} (${nodeTypeCounts.entity})`, value: 'entity' },
                { label: `${t('wiki.graph.type.source')} (${nodeTypeCounts.source})`, value: 'source' },
              ]}
              onChange={(values) => setFilters((f) => ({ ...f, types: values as GraphNodeType[] }))}
            />
            <Button icon={<ReloadOutlined />} onClick={loadGraphData}>
              {t('wiki.common.refresh')}
            </Button>
          </Space>
        </Space>
      </Card>

      <div style={{ flex: 1, position: 'relative' }}>
        <GraphView
          data={graphData}
          onNodeClick={handleNodeClick}
          onNodeHover={handleNodeHover}
          filters={filters}
          onFiltersChange={(f) => setFilters((prev) => ({ ...prev, ...f }))}
          showMinimap={true}
          showControls={true}
        />
      </div>

      {hoveredNodeId && (
        <Card
          size="small"
          style={{
            position: 'absolute',
            bottom: 24,
            left: 24,
            maxWidth: 300,
            opacity: 0.95,
          }}
        >
          {(() => {
            const node = graphData.nodes.find((n) => n.id === hoveredNodeId);
            if (!node) return null;
            return (
              <Space direction="vertical" size="small">
                <Text strong>{node.title}</Text>
                <Text type="secondary" style={{ fontSize: 12 }}>
                  {node.path}
                </Text>
                <Space>
                  <Tag>{node.type}</Tag>
                  <Text type="secondary" style={{ fontSize: 11 }}>
                    → {node.linkCount} | ← {node.backlinkCount}
                  </Text>
                </Space>
                {node.tags.length > 0 && (
                  <Space wrap>
                    {node.tags.map((tag) => (
                      <Tag key={tag} style={{ fontSize: 10 }}>
                        {tag}
                      </Tag>
                    ))}
                  </Space>
                )}
              </Space>
            );
          })()}
        </Card>
      )}
    </div>
  );
}
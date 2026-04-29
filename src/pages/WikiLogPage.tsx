import { useEffect, useState } from 'react';
import {
  Card,
  Typography,
  Select,
  Button,
  Space,
  Table,
  Tag,
  Spin,
  Drawer,
  Descriptions,
  Empty,
  Statistic,
  Row,
  Col,
} from 'antd';
import {
  LeftOutlined,
  ReloadOutlined,
  CheckCircleOutlined,
  CloseCircleOutlined,
  SyncOutlined,
} from '@ant-design/icons';
import { useTranslation } from 'react-i18next';
import { useNavigate, useSearchParams } from 'react-router-dom';
import { useLlmWikiStore, WikiOperation } from '@/stores/feature/llmWikiStore';

const { Title, Text } = Typography;

export function WikiLogPage() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const [searchParams] = useSearchParams();
  const wikiIdFromUrl = searchParams.get('wikiId');

  const {
    wikis,
    selectedWikiId,
    operations,
    loading,
    loadWikis,
    selectWiki,
    loadOperations,
  } = useLlmWikiStore();

  const [selectedWikiIdState, setSelectedWikiIdState] = useState<string | null>(null);
  const [statusFilter, setStatusFilter] = useState<string | null>(null);
  const [typeFilter, setTypeFilter] = useState<string | null>(null);
  const [selectedOperation, setSelectedOperation] = useState<WikiOperation | null>(null);
  const [detailDrawerOpen, setDetailDrawerOpen] = useState(false);

  useEffect(() => {
    loadWikis();
  }, [loadWikis]);

  useEffect(() => {
    if (wikiIdFromUrl) {
      setSelectedWikiIdState(wikiIdFromUrl);
      selectWiki(wikiIdFromUrl);
    } else if (wikis.length > 0 && !selectedWikiIdState) {
      setSelectedWikiIdState(wikis[0].id);
      selectWiki(wikis[0].id);
    }
  }, [wikiIdFromUrl, wikis, selectedWikiIdState, selectWiki]);

  useEffect(() => {
    if (selectedWikiIdState) {
      loadOperations(selectedWikiIdState);
    }
  }, [selectedWikiIdState, loadOperations]);

  const handleBack = () => {
    navigate(-1);
  };

  const handleRefresh = () => {
    if (selectedWikiIdState) {
      loadOperations(selectedWikiIdState);
    }
  };

  const getStatusColor = (status: string) => {
    switch (status) {
      case 'completed':
        return 'success';
      case 'failed':
        return 'error';
      case 'running':
        return 'processing';
      case 'pending':
        return 'default';
      default:
        return 'default';
    }
  };

  const getOperationTypeColor = (type: string) => {
    switch (type) {
      case 'ingest':
        return 'blue';
      case 'compile':
        return 'green';
      case 'lint':
        return 'orange';
      case 'sync':
        return 'purple';
      default:
        return 'default';
    }
  };

  const formatDuration = (start?: number, end?: number) => {
    if (!start || !end) return '-';
    const diff = end - start;
    if (diff < 1000) return `${diff}ms`;
    if (diff < 60000) return `${(diff / 1000).toFixed(1)}s`;
    return `${(diff / 60000).toFixed(1)}m`;
  };

  const handleViewDetail = (operation: WikiOperation) => {
    setSelectedOperation(operation);
    setDetailDrawerOpen(true);
  };

  const columns = [
    {
      title: t('wiki.operation.status', 'Status'),
      dataIndex: 'status',
      key: 'status',
      width: 100,
      render: (status: string) => (
        <Tag color={getStatusColor(status)}>
          {status.toUpperCase()}
        </Tag>
      ),
    },
    {
      title: t('wiki.operation.type', 'Type'),
      dataIndex: 'operationType',
      key: 'operationType',
      width: 120,
      render: (type: string) => (
        <Tag color={getOperationTypeColor(type)}>
          {type.toUpperCase()}
        </Tag>
      ),
    },
    {
      title: t('wiki.operation.createdAt', 'Created'),
      dataIndex: 'createdAt',
      key: 'createdAt',
      width: 180,
      render: (timestamp: number) =>
        new Date(timestamp * 1000).toLocaleString(),
    },
    {
      title: t('wiki.operation.duration', 'Duration'),
      key: 'duration',
      width: 100,
      render: (_: unknown, record: WikiOperation) =>
        formatDuration(record.createdAt, record.completedAt),
    },
    {
      title: t('wiki.operation.result', 'Result'),
      key: 'result',
      render: (_: unknown, record: WikiOperation) => {
        if (record.status === 'failed') {
          return (
            <Text type="danger" ellipsis>
              {record.errorMessage || 'Failed'}
            </Text>
          );
        }
        if (record.detailsJson) {
          const resultStr = JSON.stringify(record.detailsJson);
          return (
            <Text ellipsis style={{ maxWidth: 200 }}>
              {resultStr.length > 50 ? resultStr.substring(0, 50) + '...' : resultStr}
            </Text>
          );
        }
        return '-';
      },
    },
    {
      title: t('common.actions', 'Actions'),
      key: 'actions',
      width: 100,
      render: (_: unknown, record: WikiOperation) => (
        <Button type="link" onClick={() => handleViewDetail(record)}>
          {t('common.view', 'View')}
        </Button>
      ),
    },
  ];

  const filteredOperations = operations.filter((op) => {
    if (statusFilter && op.status !== statusFilter) return false;
    if (typeFilter && op.operationType !== typeFilter) return false;
    return true;
  });

  const stats = {
    total: filteredOperations.length,
    completed: filteredOperations.filter((op) => op.status === 'completed').length,
    failed: filteredOperations.filter((op) => op.status === 'failed').length,
    running: filteredOperations.filter((op) => op.status === 'running').length,
  };

  if (loading && wikis.length === 0) {
    return (
      <div className="h-full flex items-center justify-center">
        <Spin size="large" />
      </div>
    );
  }

  const displayWikiId = selectedWikiIdState || selectedWikiId;

  return (
    <div className="h-full flex flex-col" style={{ overflow: 'hidden' }}>
      <div className="flex items-center gap-4 p-4 border-b">
        <Button icon={<LeftOutlined />} onClick={handleBack} type="text" />
        <Title level={3} className="m-0 flex-1">
          {t('wiki.log.title', 'Wiki Operation Logs')}
        </Title>
        <Select
          value={displayWikiId}
          onChange={(value) => {
            setSelectedWikiIdState(value);
            selectWiki(value);
          }}
          style={{ width: 200 }}
          placeholder={t('wiki.selectWiki', 'Select Wiki')}
        >
          {wikis.map((wiki) => (
            <Select.Option key={wiki.id} value={wiki.id}>
              {wiki.name}
            </Select.Option>
          ))}
        </Select>
        <Button icon={<ReloadOutlined />} onClick={handleRefresh}>
          {t('common.refresh', 'Refresh')}
        </Button>
      </div>

      <div className="px-4 py-2 border-b">
        <Space wrap>
          <Select
            allowClear
            placeholder={t('wiki.log.filterStatus', 'Filter by Status')}
            value={statusFilter}
            onChange={setStatusFilter}
            style={{ width: 140 }}
            options={[
              { label: 'Completed', value: 'completed' },
              { label: 'Failed', value: 'failed' },
              { label: 'Running', value: 'running' },
              { label: 'Pending', value: 'pending' },
            ]}
          />
          <Select
            allowClear
            placeholder={t('wiki.log.filterType', 'Filter by Type')}
            value={typeFilter}
            onChange={setTypeFilter}
            style={{ width: 140 }}
            options={[
              { label: 'Ingest', value: 'ingest' },
              { label: 'Compile', value: 'compile' },
              { label: 'Lint', value: 'lint' },
              { label: 'Sync', value: 'sync' },
            ]}
          />
        </Space>
      </div>

      {displayWikiId ? (
        <>
          <Row gutter={16} className="px-4 py-3">
            <Col span={6}>
              <Card size="small">
                <Statistic
                  title={t('wiki.log.total', 'Total')}
                  value={stats.total}
                  prefix={<SyncOutlined />}
                />
              </Card>
            </Col>
            <Col span={6}>
              <Card size="small">
                <Statistic
                  title={t('wiki.log.completed', 'Completed')}
                  value={stats.completed}
                  prefix={<CheckCircleOutlined />}
                  valueStyle={{ color: '#52c41a' }}
                />
              </Card>
            </Col>
            <Col span={6}>
              <Card size="small">
                <Statistic
                  title={t('wiki.log.failed', 'Failed')}
                  value={stats.failed}
                  prefix={<CloseCircleOutlined />}
                  valueStyle={{ color: '#ff4d4f' }}
                />
              </Card>
            </Col>
            <Col span={6}>
              <Card size="small">
                <Statistic
                  title={t('wiki.log.running', 'Running')}
                  value={stats.running}
                  prefix={<SyncOutlined spin />}
                  valueStyle={{ color: '#1890ff' }}
                />
              </Card>
            </Col>
          </Row>

          <div className="flex-1 overflow-auto px-4 pb-4">
            <Table
              dataSource={filteredOperations}
              columns={columns}
              rowKey="id"
              pagination={{ pageSize: 20, showSizeChanger: true }}
              loading={loading}
              locale={{ emptyText: <Empty description={t('wiki.log.noOperations', 'No operations found')} /> }}
            />
          </div>
        </>
      ) : (
        <div className="flex-1 flex items-center justify-center">
          <Text type="secondary">{t('wiki.selectWikiPrompt', 'Please select a wiki first')}</Text>
        </div>
      )}

      <Drawer
        title={t('wiki.log.operationDetail', 'Operation Detail')}
        open={detailDrawerOpen}
        onClose={() => setDetailDrawerOpen(false)}
        width={600}
      >
        {selectedOperation && (
          <Descriptions column={1} bordered>
            <Descriptions.Item label={t('wiki.operation.id', 'ID')}>
              {selectedOperation.id}
            </Descriptions.Item>
            <Descriptions.Item label={t('wiki.operation.type', 'Type')}>
              <Tag color={getOperationTypeColor(selectedOperation.operationType)}>
                {selectedOperation.operationType.toUpperCase()}
              </Tag>
            </Descriptions.Item>
            <Descriptions.Item label={t('wiki.operation.status', 'Status')}>
              <Tag color={getStatusColor(selectedOperation.status)}>
                {selectedOperation.status.toUpperCase()}
              </Tag>
            </Descriptions.Item>
            <Descriptions.Item label={t('wiki.operation.createdAt', 'Created At')}>
              {selectedOperation.createdAt
                ? new Date(selectedOperation.createdAt * 1000).toLocaleString()
                : '-'}
            </Descriptions.Item>
            <Descriptions.Item label={t('wiki.operation.completedAt', 'Completed At')}>
              {selectedOperation.completedAt
                ? new Date(selectedOperation.completedAt * 1000).toLocaleString()
                : '-'}
            </Descriptions.Item>
            <Descriptions.Item label={t('wiki.operation.duration', 'Duration')}>
              {formatDuration(selectedOperation.createdAt, selectedOperation.completedAt)}
            </Descriptions.Item>
            {selectedOperation.errorMessage && (
              <Descriptions.Item label={t('wiki.operation.error', 'Error')}>
                <Text type="danger">{selectedOperation.errorMessage}</Text>
              </Descriptions.Item>
            )}
            {selectedOperation.detailsJson && (
              <Descriptions.Item label={t('wiki.operation.details', 'Details')}>
                <pre className="whitespace-pre-wrap text-xs bg-gray-50 p-2 rounded">
                  {JSON.stringify(selectedOperation.detailsJson, null, 2)}
                </pre>
              </Descriptions.Item>
            )}
          </Descriptions>
        )}
      </Drawer>
    </div>
  );
}
import { Button, List, Space, Tag, Typography } from 'antd';
import {
  CheckCircleOutlined,
  DeleteOutlined,
  PlusOutlined,
} from '@ant-design/icons';
import { CredibilityBadge } from './CredibilityBadge';

const { Text, Title } = Typography;

interface Citation {
  id: string;
  sourceUrl: string;
  sourceTitle: string;
  sourceType: string;
  credibility: number;
  inReport: boolean;
  accessedAt?: string;
  usedInSection?: string;
}

interface CitationManagerProps {
  citations: Citation[];
  onCitationSelect?: (citation: Citation) => void;
  onCitationRemove?: (citationId: string) => void;
  onToggleInReport?: (citationId: string) => void;
  onAddNew?: () => void;
  selectedCitationId?: string | null;
}

function getSourceTypeName(sourceType: string): string {
  const nameMap: Record<string, string> = {
    web: '网页',
    academic: '学术',
    wikipedia: '维基百科',
    github: 'GitHub',
    documentation: '文档',
    news: '新闻',
    blog: '博客',
    forum: '论坛',
    unknown: '未知',
  };
  return nameMap[sourceType.toLowerCase()] || sourceType;
}

export function CitationManager({
  citations,
  onCitationSelect,
  onCitationRemove,
  onToggleInReport,
  onAddNew,
  selectedCitationId,
}: CitationManagerProps) {
  const citationsInReport = citations.filter((c) => c.inReport);
  const citationsNotInReport = citations.filter((c) => !c.inReport);

  return (
    <div className="citation-manager">
      <div className="flex items-center justify-between mb-3">
        <Title level={5} className="mb-0">
          引用管理 ({citations.length})
        </Title>
        {onAddNew && (
          <Button type="primary" size="small" icon={<PlusOutlined />} onClick={onAddNew}>
            添加引用
          </Button>
        )}
      </div>

      {citationsInReport.length > 0 && (
        <div className="mb-4">
          <Text type="secondary" className="text-sm">
            报告中使用的引用 ({citationsInReport.length})
          </Text>
          <List
            size="small"
            dataSource={citationsInReport}
            renderItem={(item) => (
              <List.Item
                className={`cursor-pointer hover:bg-gray-50 ${
                  selectedCitationId === item.id ? 'bg-blue-50' : ''
                }`}
                onClick={() => onCitationSelect?.(item)}
                actions={[
                  <Button
                    type="text"
                    size="small"
                    danger
                    icon={<DeleteOutlined />}
                    onClick={(e) => {
                      e.stopPropagation();
                      onCitationRemove?.(item.id);
                    }}
                  />,
                ]}
              >
                <List.Item.Meta
                  avatar={
                    <CheckCircleOutlined
                      style={{ color: item.inReport ? '#52c41a' : '#d9d9d9' }}
                      onClick={(e) => {
                        e.stopPropagation();
                        onToggleInReport?.(item.id);
                      }}
                      className="cursor-pointer"
                    />
                  }
                  title={<Text ellipsis>{item.sourceTitle}</Text>}
                  description={
                    <Space size="small">
                      <Tag>{getSourceTypeName(item.sourceType)}</Tag>
                      <CredibilityBadge score={item.credibility} />
                    </Space>
                  }
                />
              </List.Item>
            )}
          />
        </div>
      )}

      {citationsNotInReport.length > 0 && (
        <div>
          <Text type="secondary" className="text-sm">
            未使用的引用 ({citationsNotInReport.length})
          </Text>
          <List
            size="small"
            dataSource={citationsNotInReport}
            renderItem={(item) => (
              <List.Item
                className={`cursor-pointer hover:bg-gray-50 ${
                  selectedCitationId === item.id ? 'bg-blue-50' : ''
                }`}
                onClick={() => onCitationSelect?.(item)}
                actions={[
                  <Button
                    type="text"
                    size="small"
                    icon={<CheckCircleOutlined />}
                    onClick={(e) => {
                      e.stopPropagation();
                      onToggleInReport?.(item.id);
                    }}
                    title="添加到报告"
                  />,
                  <Button
                    type="text"
                    size="small"
                    danger
                    icon={<DeleteOutlined />}
                    onClick={(e) => {
                      e.stopPropagation();
                      onCitationRemove?.(item.id);
                    }}
                  />,
                ]}
              >
                <List.Item.Meta
                  avatar={
                    <CheckCircleOutlined
                      style={{ color: item.inReport ? '#52c41a' : '#d9d9d9' }}
                      onClick={(e) => {
                        e.stopPropagation();
                        onToggleInReport?.(item.id);
                      }}
                      className="cursor-pointer"
                    />
                  }
                  title={<Text ellipsis>{item.sourceTitle}</Text>}
                  description={
                    <Space size="small">
                      <Tag>{getSourceTypeName(item.sourceType)}</Tag>
                      <CredibilityBadge score={item.credibility} />
                    </Space>
                  }
                />
              </List.Item>
            )}
          />
        </div>
      )}

      {citations.length === 0 && (
        <div className="text-center text-gray-400 py-8">
          暂无引用，请从搜索结果中添加
        </div>
      )}
    </div>
  );
}

interface CitationStatsProps {
  citations: Citation[];
}

export function CitationStats({ citations }: CitationStatsProps) {
  const stats = {
    total: citations.length,
    inReport: citations.filter((c) => c.inReport).length,
    byType: citations.reduce((acc, c) => {
      acc[c.sourceType] = (acc[c.sourceType] || 0) + 1;
      return acc;
    }, {} as Record<string, number>),
    avgCredibility:
      citations.length > 0
        ? citations.reduce((sum, c) => sum + c.credibility, 0) / citations.length
        : 0,
  };

  return (
    <div className="citation-stats">
      <Space direction="vertical" size="small" style={{ width: '100%' }}>
        <div className="flex justify-between">
          <Text type="secondary">总引用数:</Text>
          <Text strong>{stats.total}</Text>
        </div>
        <div className="flex justify-between">
          <Text type="secondary">报告中使用:</Text>
          <Text strong>{stats.inReport}</Text>
        </div>
        <div className="flex justify-between">
          <Text type="secondary">平均可信度:</Text>
          <CredibilityBadge score={stats.avgCredibility} />
        </div>
        <div>
          <Text type="secondary" className="block mb-1">
            来源类型分布:
          </Text>
          <Space size="small" wrap>
            {Object.entries(stats.byType).map(([type, count]) => (
              <Tag key={type}>
                {getSourceTypeName(type)}: {count}
              </Tag>
            ))}
          </Space>
        </div>
      </Space>
    </div>
  );
}

export default CitationManager;

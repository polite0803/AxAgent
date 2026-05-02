import { PlusOutlined } from "@ant-design/icons";
import { Button, Card, List, Space, Tag, Typography } from "antd";
import { CredibilityBadge } from "./CredibilityBadge";

const { Text, Title, Paragraph } = Typography;

interface SearchResult {
  id: string;
  sourceType: string;
  url: string;
  title: string;
  snippet: string;
  credibilityScore: number | null;
  relevanceScore: number;
}

interface ResearchSourcesProps {
  sources: SearchResult[];
  onSourceSelect?: (source: SearchResult) => void;
  onAddToCitation?: (source: SearchResult) => void;
  selectedSourceId?: string | null;
  maxDisplay?: number;
}

function getSourceTypeColor(sourceType: string): string {
  const colorMap: Record<string, string> = {
    web: "blue",
    academic: "green",
    wikipedia: "cyan",
    github: "purple",
    documentation: "orange",
    news: "magenta",
    blog: "gold",
    forum: "default",
    unknown: "default",
  };
  return colorMap[sourceType.toLowerCase()] || "default";
}

function getSourceTypeName(sourceType: string): string {
  const nameMap: Record<string, string> = {
    web: "网页",
    academic: "学术",
    wikipedia: "维基百科",
    github: "GitHub",
    documentation: "文档",
    news: "新闻",
    blog: "博客",
    forum: "论坛",
    unknown: "未知",
  };
  return nameMap[sourceType.toLowerCase()] || sourceType;
}

export function ResearchSources({
  sources,
  onSourceSelect,
  onAddToCitation,
  selectedSourceId,
  maxDisplay,
}: ResearchSourcesProps) {
  const displaySources = maxDisplay ? sources.slice(0, maxDisplay) : sources;

  return (
    <div className="research-sources">
      <List
        size="small"
        dataSource={displaySources}
        locale={{ emptyText: "暂无搜索结果" }}
        renderItem={(item) => (
          <List.Item
            className={`cursor-pointer hover:bg-gray-50 ${
              selectedSourceId === item.id ? "bg-blue-50 border-l-4 border-blue-500" : ""
            }`}
            onClick={() => onSourceSelect?.(item)}
          >
            <List.Item.Meta
              title={
                <Space>
                  <a href={item.url} target="_blank" rel="noopener noreferrer" onClick={(e) => e.stopPropagation()}>
                    {item.title}
                  </a>
                  <Tag color={getSourceTypeColor(item.sourceType)}>
                    {getSourceTypeName(item.sourceType)}
                  </Tag>
                  {item.relevanceScore > 0 && (
                    <Tag color={item.relevanceScore > 0.7 ? "green" : item.relevanceScore > 0.4 ? "orange" : "red"}>
                      相关度: {Math.round(item.relevanceScore * 100)}%
                    </Tag>
                  )}
                </Space>
              }
              description={
                <div>
                  <Paragraph ellipsis={{ rows: 2 }} className="mb-1 text-sm">
                    {item.snippet}
                  </Paragraph>
                  <Space size="small">
                    {item.credibilityScore !== null && <CredibilityBadge score={item.credibilityScore} />}
                    {onAddToCitation && (
                      <Button
                        type="link"
                        size="small"
                        icon={<PlusOutlined />}
                        onClick={(e) => {
                          e.stopPropagation();
                          onAddToCitation(item);
                        }}
                      >
                        添加到引用
                      </Button>
                    )}
                  </Space>
                </div>
              }
            />
          </List.Item>
        )}
      />
      {maxDisplay && sources.length > maxDisplay && (
        <Text type="secondary" className="text-sm">
          还有 {sources.length - maxDisplay} 个来源...
        </Text>
      )}
    </div>
  );
}

interface SourceDetailPanelProps {
  source: SearchResult | null;
  onAddToCitation?: (source: SearchResult) => void;
}

export function SourceDetailPanel({ source, onAddToCitation }: SourceDetailPanelProps) {
  if (!source) {
    return (
      <Card size="small" className="h-full">
        <div className="flex items-center justify-center h-full text-gray-400">
          选择一个来源查看详情
        </div>
      </Card>
    );
  }

  return (
    <Card size="small" className="h-full">
      <Title level={5} className="mb-2">
        来源详情
      </Title>

      <div className="space-y-3">
        <div>
          <Text type="secondary" className="text-sm">
            标题
          </Text>
          <div>
            <a href={source.url} target="_blank" rel="noopener noreferrer">
              {source.title}
            </a>
          </div>
        </div>

        <div>
          <Text type="secondary" className="text-sm">
            来源类型
          </Text>
          <div>
            <Tag color={getSourceTypeColor(source.sourceType)}>
              {getSourceTypeName(source.sourceType)}
            </Tag>
          </div>
        </div>

        <div>
          <Text type="secondary" className="text-sm">
            URL
          </Text>
          <div className="truncate">
            <a href={source.url} target="_blank" rel="noopener noreferrer">
              {source.url}
            </a>
          </div>
        </div>

        <div>
          <Text type="secondary" className="text-sm">
            摘要
          </Text>
          <div>
            <Text>{source.snippet}</Text>
          </div>
        </div>

        <div>
          <Text type="secondary" className="text-sm">
            可信度评分
          </Text>
          <div>
            {source.credibilityScore !== null
              ? <CredibilityBadge score={source.credibilityScore} />
              : <Text type="secondary">未评估</Text>}
          </div>
        </div>

        <div>
          <Text type="secondary" className="text-sm">
            相关度评分
          </Text>
          <div>
            <Text>{source.relevanceScore > 0 ? `${Math.round(source.relevanceScore * 100)}%` : "未评估"}</Text>
          </div>
        </div>

        {onAddToCitation && (
          <Button
            type="primary"
            icon={<PlusOutlined />}
            onClick={() => onAddToCitation(source)}
            block
          >
            添加到报告引用
          </Button>
        )}
      </div>
    </Card>
  );
}

export default ResearchSources;

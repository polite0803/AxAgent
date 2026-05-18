import { PlusOutlined } from "@ant-design/icons";
import { Button, Card, List, Space, Tag, Typography } from "antd";
import { useTranslation } from "react-i18next";
import { CredibilityBadge } from "./CredibilityBadge";
import { getSourceTypeColor, getSourceTypeName, type SearchResult } from "./researchUtils";

const { Text, Title, Paragraph } = Typography;

interface ResearchSourcesProps {
  sources: SearchResult[];
  onSourceSelect?: (source: SearchResult) => void;
  onAddToCitation?: (source: SearchResult) => void;
  selectedSourceId?: string | null;
  maxDisplay?: number;
}

export function ResearchSources({
  sources,
  onSourceSelect,
  onAddToCitation,
  selectedSourceId,
  maxDisplay,
}: ResearchSourcesProps) {
  const { t } = useTranslation();
  const displaySources = maxDisplay ? sources.slice(0, maxDisplay) : sources;

  return (
    <div className="research-sources">
      <List
        size="small"
        dataSource={displaySources}
        locale={{ emptyText: t("research.noSearchResults") }}
        renderItem={(item) => (
          <List.Item
            className={`cursor-pointer hover:bg-zinc-50 ${
              selectedSourceId === item.id ? "bg-blue-50 border-l-4 border-blue-500" : ""
            }`}
            onClick={() => onSourceSelect?.(item)}
            aria-label={`${item.title} - ${getSourceTypeName(item.sourceType, t)}`}
          >
            <List.Item.Meta
              title={
                <Space>
                  <a
                    href={item.url}
                    target="_blank"
                    rel="noopener noreferrer"
                    onClick={(e) => e.stopPropagation()}
                    aria-label={item.title}
                  >
                    {item.title}
                  </a>
                  <Tag color={getSourceTypeColor(item.sourceType)}>
                    {getSourceTypeName(item.sourceType, t)}
                  </Tag>
                  <Tag color={item.relevanceScore > 0.7 ? "green" : item.relevanceScore > 0.4 ? "orange" : "red"}>
                    {item.relevanceScore > 0
                      ? `${t("research.relevance")}: ${Math.round(item.relevanceScore * 100)}%`
                      : t("research.notEvaluated")}
                  </Tag>
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
                        {t("research.addToCitation")}
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
          {t("research.moreSources", { count: sources.length - maxDisplay })}
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
  const { t } = useTranslation();
  if (!source) {
    return (
      <Card size="small" className="h-full">
        <div className="flex items-center justify-center h-full text-zinc-400">
          {t("research.selectSourceToView")}
        </div>
      </Card>
    );
  }

  return (
    <Card size="small" className="h-full">
      <Title level={5} className="mb-2">
        {t("research.sourceDetail")}
      </Title>

      <div className="space-y-3">
        <div>
          <Text type="secondary" className="text-sm">
            {t("research.sourceTitle")}
          </Text>
          <div>
            <a href={source.url} target="_blank" rel="noopener noreferrer" aria-label={source.title}>
              {source.title}
            </a>
          </div>
        </div>

        <div>
          <Text type="secondary" className="text-sm">
            {t("research.sourceType")}
          </Text>
          <div>
            <Tag color={getSourceTypeColor(source.sourceType)}>
              {getSourceTypeName(source.sourceType, t)}
            </Tag>
          </div>
        </div>

        <div>
          <Text type="secondary" className="text-sm">
            URL
          </Text>
          <div className="truncate">
            <a href={source.url} target="_blank" rel="noopener noreferrer" aria-label={`URL: ${source.url}`}>
              {source.url}
            </a>
          </div>
        </div>

        <div>
          <Text type="secondary" className="text-sm">
            {t("research.snippet")}
          </Text>
          <div>
            <Text>{source.snippet}</Text>
          </div>
        </div>

        <div>
          <Text type="secondary" className="text-sm">
            {t("research.credibilityScore")}
          </Text>
          <div>
            {source.credibilityScore !== null
              ? <CredibilityBadge score={source.credibilityScore} />
              : <Text type="secondary">{t("research.notEvaluated")}</Text>}
          </div>
        </div>

        <div>
          <Text type="secondary" className="text-sm">
            {t("research.relevanceScore")}
          </Text>
          <div>
            <Text>
              {source.relevanceScore > 0 ? `${Math.round(source.relevanceScore * 100)}%` : t("research.notEvaluated")}
            </Text>
          </div>
        </div>

        {onAddToCitation && (
          <Button
            type="primary"
            icon={<PlusOutlined />}
            onClick={() => onAddToCitation(source)}
            block
          >
            {t("research.addToReportCitation")}
          </Button>
        )}
      </div>
    </Card>
  );
}

export type { SearchResult };

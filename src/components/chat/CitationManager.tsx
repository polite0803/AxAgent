import { CheckCircleOutlined, DeleteOutlined, PlusOutlined } from "@ant-design/icons";
import { Button, List, Space, Tag, Typography } from "antd";
import { useTranslation } from "react-i18next";
import { CredibilityBadge } from "./CredibilityBadge";

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

function getSourceTypeName(sourceType: string, t: (key: string) => string): string {
  const nameMap: Record<string, string> = {
    web: t("citationManager.sourceType.web"),
    academic: t("citationManager.sourceType.academic"),
    wikipedia: t("citationManager.sourceType.wikipedia"),
    github: t("citationManager.sourceType.github"),
    documentation: t("citationManager.sourceType.documentation"),
    news: t("citationManager.sourceType.news"),
    blog: t("citationManager.sourceType.blog"),
    forum: t("citationManager.sourceType.forum"),
    unknown: t("citationManager.sourceType.unknown"),
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
  const { t } = useTranslation();
  const citationsInReport = citations.filter((c) => c.inReport);
  const citationsNotInReport = citations.filter((c) => !c.inReport);

  return (
    <div className="citation-manager">
      <div className="flex items-center justify-between mb-3">
        <Title level={5} className="mb-0">
          {t("citationManager.title", { count: citations.length })}
        </Title>
        {onAddNew && (
          <Button type="primary" size="small" icon={<PlusOutlined />} onClick={onAddNew}>
            {t("citationManager.addCitation")}
          </Button>
        )}
      </div>

      {citationsInReport.length > 0 && (
        <div className="mb-4">
          <Text type="secondary" className="text-sm">
            {t("citationManager.inReport", { count: citationsInReport.length })}
          </Text>
          <List
            size="small"
            dataSource={citationsInReport}
            renderItem={(item) => (
              <List.Item
                className={`cursor-pointer hover:bg-gray-50 ${selectedCitationId === item.id ? "bg-blue-50" : ""}`}
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
                      style={{ color: item.inReport ? "#52c41a" : "#d9d9d9" }}
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
                      <Tag>{getSourceTypeName(item.sourceType, t)}</Tag>
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
            {t("citationManager.notInReport", { count: citationsNotInReport.length })}
          </Text>
          <List
            size="small"
            dataSource={citationsNotInReport}
            renderItem={(item) => (
              <List.Item
                className={`cursor-pointer hover:bg-gray-50 ${selectedCitationId === item.id ? "bg-blue-50" : ""}`}
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
                    title={t("citationManager.addToReport")}
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
                      style={{ color: item.inReport ? "#52c41a" : "#d9d9d9" }}
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
                      <Tag>{getSourceTypeName(item.sourceType, t)}</Tag>
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
          {t("citationManager.empty")}
        </div>
      )}
    </div>
  );
}

interface CitationStatsProps {
  citations: Citation[];
}

export function CitationStats({ citations }: CitationStatsProps) {
  const { t } = useTranslation();
  const stats = {
    total: citations.length,
    inReport: citations.filter((c) => c.inReport).length,
    byType: citations.reduce((acc, c) => {
      acc[c.sourceType] = (acc[c.sourceType] || 0) + 1;
      return acc;
    }, {} as Record<string, number>),
    avgCredibility: citations.length > 0
      ? citations.reduce((sum, c) => sum + c.credibility, 0) / citations.length
      : 0,
  };

  return (
    <div className="citation-stats">
      <Space direction="vertical" size="small" style={{ width: "100%" }}>
        <div className="flex justify-between">
          <Text type="secondary">{t("citationManager.totalCitations")}</Text>
          <Text strong>{stats.total}</Text>
        </div>
        <div className="flex justify-between">
          <Text type="secondary">{t("citationManager.usedInReport")}</Text>
          <Text strong>{stats.inReport}</Text>
        </div>
        <div className="flex justify-between">
          <Text type="secondary">{t("citationManager.avgCredibility")}</Text>
          <CredibilityBadge score={stats.avgCredibility} />
        </div>
        <div>
          <Text type="secondary" className="block mb-1">
            {t("citationManager.sourceDistribution")}
          </Text>
          <Space size="small" wrap>
            {Object.entries(stats.byType).map(([type, count]) => (
              <Tag key={type}>
                {getSourceTypeName(type, t)}: {count}
              </Tag>
            ))}
          </Space>
        </div>
      </Space>
    </div>
  );
}

export default CitationManager;

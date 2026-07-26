// SPDX-License-Identifier: AGPL-3.0-only

import { useStreamStore } from "@/stores";
import type { Citation, CitationStatsData } from "@/types";
import { CheckCircleOutlined, DeleteOutlined, PlusOutlined } from "@ant-design/icons";
import { Button, Space, Tag, Typography } from "antd";
import { useMemo } from "react";
import { useTranslation } from "react-i18next";
import { CredibilityBadge } from "./CredibilityBadge";

const { Text, Title } = Typography;

function getSourceTypeName(
  sourceType: string,
  t: (key: string) => string,
): string {
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

interface CitationManagerProps {
  citations?: Citation[];
  onCitationSelect?: (citation: Citation) => void;
  onCitationRemove?: (citationId: string) => void;
  onToggleInReport?: (citationId: string) => void;
  onAddNew?: () => void;
  selectedCitationId?: string | null;
}

export function CitationManager({
  citations: externalCitations,
  onCitationSelect,
  onCitationRemove,
  onToggleInReport,
  onAddNew,
  selectedCitationId: externalSelectedId,
}: CitationManagerProps) {
  const { t } = useTranslation();
  // 用选择器精确订阅，避免订阅整个 stream store 导致的无谓重渲染
  const storeCitations = useStreamStore((s) => s.citations);
  const storeSelectedCitationId = useStreamStore((s) => s.selectedCitationId);
  const selectCitation = useStreamStore((s) => s.selectCitation);
  const removeCitation = useStreamStore((s) => s.removeCitation);
  const toggleInReport = useStreamStore((s) => s.toggleInReport);
  const citations = externalCitations ?? storeCitations;
  const selectedCitationId = externalSelectedId ?? storeSelectedCitationId;
  const citationsInReport = citations.filter((c) => c.inReport);
  const citationsNotInReport = citations.filter((c) => !c.inReport);

  const handleSelect = (citation: Citation) => {
    if (onCitationSelect) {
      onCitationSelect(citation);
    } else {
      selectCitation(citation.id);
    }
  };

  const handleRemove = (citationId: string) => {
    if (onCitationRemove) {
      onCitationRemove(citationId);
    } else {
      removeCitation(citationId);
    }
  };

  const handleToggle = (citationId: string) => {
    if (onToggleInReport) {
      onToggleInReport(citationId);
    } else {
      toggleInReport(citationId);
    }
  };

  return (
    <div className="citation-manager">
      <div className="flex items-center justify-between mb-3">
        <Title level={5} className="mb-0">
          {t("citationManager.title", { count: citations.length })}
        </Title>
        {onAddNew && (
          <Button
            type="primary"
            size="small"
            icon={<PlusOutlined />}
            onClick={onAddNew}
          >
            {t("citationManager.addCitation")}
          </Button>
        )}
      </div>

      {citationsInReport.length > 0 && (
        <div className="mb-4">
          <Text type="secondary" className="text-sm">
            {t("citationManager.inReport", { count: citationsInReport.length })}
          </Text>
          <div className="divide-y divide-gray-100">
            {citationsInReport.map((item) => (
              <div
                key={item.id}
                className={`cursor-pointer hover:bg-zinc-50 ${selectedCitationId === item.id ? "bg-blue-50" : ""}`}
                style={{
                  padding: "8px 0",
                  display: "flex",
                  alignItems: "center",
                  justifyContent: "space-between",
                }}
                onClick={() => handleSelect(item)}
              >
                <div style={{ display: "flex", alignItems: "center", gap: 12 }}>
                  <CheckCircleOutlined
                    style={{ color: item.inReport ? "#52c41a" : "#d9d9d9" }}
                    onClick={(e) => {
                      e.stopPropagation();
                      handleToggle(item.id);
                    }}
                    className="cursor-pointer"
                  />
                  <div>
                    <div style={{ fontWeight: 500 }}>
                      <Text ellipsis>{item.sourceTitle}</Text>
                    </div>
                    <div
                      style={{
                        color: "var(--text-secondary, rgba(0,0,0,0.45))",
                        fontSize: 13,
                        marginTop: 2,
                      }}
                    >
                      <Space size="small">
                        <Tag>{getSourceTypeName(item.sourceType, t)}</Tag>
                        <CredibilityBadge score={item.credibility} />
                      </Space>
                    </div>
                  </div>
                </div>
                <Button
                  type="text"
                  size="small"
                  danger
                  icon={<DeleteOutlined />}
                  onClick={(e) => {
                    e.stopPropagation();
                    handleRemove(item.id);
                  }}
                />
              </div>
            ))}
          </div>
        </div>
      )}

      {citationsNotInReport.length > 0 && (
        <div>
          <Text type="secondary" className="text-sm">
            {t("citationManager.notInReport", {
              count: citationsNotInReport.length,
            })}
          </Text>
          <div className="divide-y divide-gray-100">
            {citationsNotInReport.map((item) => (
              <div
                key={item.id}
                className={`cursor-pointer hover:bg-zinc-50 ${selectedCitationId === item.id ? "bg-blue-50" : ""}`}
                style={{
                  padding: "8px 0",
                  display: "flex",
                  alignItems: "center",
                  justifyContent: "space-between",
                }}
                onClick={() => handleSelect(item)}
              >
                <div style={{ display: "flex", alignItems: "center", gap: 12 }}>
                  <CheckCircleOutlined
                    style={{ color: item.inReport ? "#52c41a" : "#d9d9d9" }}
                    onClick={(e) => {
                      e.stopPropagation();
                      handleToggle(item.id);
                    }}
                    className="cursor-pointer"
                  />
                  <div>
                    <div style={{ fontWeight: 500 }}>
                      <Text ellipsis>{item.sourceTitle}</Text>
                    </div>
                    <div
                      style={{
                        color: "var(--text-secondary, rgba(0,0,0,0.45))",
                        fontSize: 13,
                        marginTop: 2,
                      }}
                    >
                      <Space size="small">
                        <Tag>{getSourceTypeName(item.sourceType, t)}</Tag>
                        <CredibilityBadge score={item.credibility} />
                      </Space>
                    </div>
                  </div>
                </div>
                <div style={{ display: "flex", alignItems: "center", gap: 4 }}>
                  <Button
                    type="text"
                    size="small"
                    icon={<CheckCircleOutlined />}
                    onClick={(e) => {
                      e.stopPropagation();
                      handleToggle(item.id);
                    }}
                    title={t("citationManager.addToReport")}
                  />
                  <Button
                    type="text"
                    size="small"
                    danger
                    icon={<DeleteOutlined />}
                    onClick={(e) => {
                      e.stopPropagation();
                      handleRemove(item.id);
                    }}
                  />
                </div>
              </div>
            ))}
          </div>
        </div>
      )}

      {citations.length === 0 && (
        <div className="text-center text-zinc-400 py-8">
          {t("citationManager.empty")}
        </div>
      )}
    </div>
  );
}

interface CitationStatsProps {
  citations?: Citation[];
}

export function CitationStats({
  citations: externalCitations,
}: CitationStatsProps) {
  const { t } = useTranslation();
  const storeCitations = useStreamStore((s) => s.citations);
  const stats: CitationStatsData = useMemo(() => {
    const src = externalCitations ?? storeCitations;
    const total = src.length;
    const inReport = src.filter((c) => c.inReport).length;
    const byType = src.reduce<Partial<Record<string, number>>>((acc, c) => {
      acc[c.sourceType] = (acc[c.sourceType] || 0) + 1;
      return acc;
    }, {});
    const avgCredibility = total > 0
      ? src.reduce((sum, c) => sum + c.credibility, 0) / total
      : 0;
    return { total, inReport, byType, avgCredibility };
  }, [externalCitations, storeCitations]);

  return (
    <div className="citation-stats">
      <Space orientation="vertical" size="small" style={{ width: "100%" }}>
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

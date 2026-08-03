// SPDX-License-Identifier: AGPL-3.0-only

import { Tooltip } from "@/components/layout/Tooltip";
import { invoke, logIpcError } from "@/lib/invoke";
import type { LintIssue, LintResult } from "@/types";
import { CheckCircleOutlined, CloseCircleOutlined, ReloadOutlined, WarningOutlined } from "@ant-design/icons";
import { Badge, Button, Card, Empty, Popover, Progress, Space, Spin, Tag, Typography } from "antd";
import React, { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
const { Text } = Typography;

interface QualityScoreProps {
  wikiId: string;
  pageId?: string;
  autoRefresh?: boolean;
  refreshInterval?: number;
  /** 紧凑模式：工具栏里只显示分数 Badge 按钮，点击 Popover 展示详情 */
  compact?: boolean;
}

interface QualityDetails {
  score: number;
  issues: LintIssue[];
  factors: {
    name: string;
    impact: number;
    description: string;
  }[];
}

export function QualityScore({
  wikiId,
  pageId,
  autoRefresh = false,
  refreshInterval = 60000,
  compact = false,
}: QualityScoreProps) {
  const { t } = useTranslation();
  const [loading, setLoading] = useState(true);
  const [details, setDetails] = useState<QualityDetails | null>(null);
  const [refreshing, setRefreshing] = useState(false);
  const [popoverOpen, setPopoverOpen] = useState(false);

  useEffect(() => {
    void loadQualityScore();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [wikiId, pageId]);

  useEffect(() => {
    if (!autoRefresh) {
      return;
    }
    const interval = setInterval(
      () => {
        void loadQualityScore();
      },
      refreshInterval,
    );
    return () => clearInterval(interval);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [autoRefresh, refreshInterval, wikiId, pageId]);

  async function loadQualityScore() {
    setRefreshing(true);
    try {
      if (pageId) {
        const result = await invoke<LintResult>("llm_wiki_lint", {
          noteId: pageId,
        });
        const score = result?.score ?? 1.0;
        const issues = result?.issues || [];
        setDetails({
          score,
          issues,
          factors: analyzeFactors(issues),
        });
      } else {
        const result = await invoke<LintResult[]>("llm_wiki_lint_vault", {
          wikiId,
        });
        const allIssues = result?.flatMap((r) => r.issues) || [];
        const score = calculateScore(allIssues);
        setDetails({
          score,
          issues: allIssues,
          factors: analyzeFactors(allIssues),
        });
      }
    } catch (e) {
      logIpcError("Failed to load quality score")(e);
    }
    setLoading(false);
    setRefreshing(false);
  }

  const calculateScore = (issues: LintIssue[]): number => {
    if (issues.length === 0) {
      return 1.0;
    }
    let score = 1.0;
    for (const issue of issues) {
      switch (issue.severity) {
        case "Error":
          score -= 0.3;
          break;
        case "Warning":
          score -= 0.1;
          break;
        case "Info":
          score -= 0.02;
          break;
      }
    }
    return Math.max(0, Math.min(1, score));
  };

  const analyzeFactors = (issues: LintIssue[]): QualityDetails["factors"] => {
    const factors: QualityDetails["factors"] = [];

    const errorCount = issues.filter((i) => i.severity === "Error").length;
    if (errorCount > 0) {
      factors.push({
        name: t("wiki.quality.errors"),
        impact: -errorCount * 0.3,
        description: t("wiki.quality.errorsDesc", { count: errorCount }),
      });
    }

    const warningCount = issues.filter((i) => i.severity === "Warning").length;
    if (warningCount > 0) {
      factors.push({
        name: t("wiki.quality.warnings"),
        impact: -warningCount * 0.1,
        description: t("wiki.quality.warningsDesc", { count: warningCount }),
      });
    }

    const infoCount = issues.filter((i) => i.severity === "Info").length;
    if (infoCount > 0) {
      factors.push({
        name: t("wiki.quality.suggestions"),
        impact: -infoCount * 0.02,
        description: t("wiki.quality.suggestionsDesc", { count: infoCount }),
      });
    }

    return factors;
  };

  const getScoreColor = (score: number) => {
    if (score >= 0.8) {
      return "#52c41a";
    }
    if (score >= 0.5) {
      return "#faad14";
    }
    return "#ff4d4f";
  };

  const getScoreIcon = (score: number) => {
    if (score >= 0.8) {
      return <CheckCircleOutlined style={{ color: "#52c41a" }} />;
    }
    if (score >= 0.5) {
      return <WarningOutlined style={{ color: "#faad14" }} />;
    }
    return <CloseCircleOutlined style={{ color: "#ff4d4f" }} />;
  };

  const getScoreLabel = (score: number) => {
    if (score >= 0.8) {
      return t("wiki.quality.excellent");
    }
    if (score >= 0.6) {
      return t("wiki.quality.good");
    }
    if (score >= 0.4) {
      return t("wiki.quality.fair");
    }
    return t("wiki.quality.poor");
  };

  const getIssueSeverityColor = (
    severity: string,
  ): "success" | "error" | "processing" | "default" | "warning" => {
    switch (severity) {
      case "error":
        return "error";
      case "warning":
        return "warning";
      case "info":
        return "default";
      default:
        return "default";
    }
  };

  // ========== 紧凑模式：工具栏分数按钮 + 极致紧凑 Popover ==========
  if (compact) {
    const percent = details ? Math.round(details.score * 100) : 100;
    return (
      <Popover
        open={popoverOpen}
        onOpenChange={setPopoverOpen}
        trigger="click"
        placement="bottomRight"
        arrow={false}
        styles={{
          root: { width: 260, maxWidth: "90vw" },
          container: { padding: "8px 10px" },
        }}
        content={
          <div style={{ width: "100%" }}>
            {/* 顶部：标题 + 刷新 */}
            <div className="flex items-center justify-between mb-2">
              <Space size={4}>
                {details
                  ? getScoreIcon(details.score)
                  : <CheckCircleOutlined style={{ color: "#52c41a", fontSize: 12 }} />}
                <Text strong style={{ fontSize: 12 }}>{t("wiki.quality.title")}</Text>
              </Space>
              <Tooltip title={t("wiki.quality.refresh")}>
                <Button
                  type="text"
                  size="small"
                  icon={<ReloadOutlined spin={refreshing} style={{ fontSize: 11 }} />}
                  style={{ width: 20, height: 20, minWidth: 20, padding: 0 }}
                  onClick={() => {
                    void loadQualityScore();
                  }}
                />
              </Tooltip>
            </div>

            {details
              ? (
                <>
                  {/* 分数 + 评级 */}
                  <div className="flex items-center gap-2 mb-2">
                    <Progress
                      type="circle"
                      percent={percent}
                      width={44}
                      strokeWidth={8}
                      strokeColor={getScoreColor(details.score)}
                      format={() => <span style={{ fontSize: 11, fontWeight: 700 }}>{percent}%</span>}
                    />
                    <div>
                      <Text strong style={{ fontSize: 12, color: getScoreColor(details.score) }}>
                        {getScoreLabel(details.score)}
                      </Text>
                      <div>
                        <Text type="secondary" style={{ fontSize: 10 }}>
                          {details.issues.length} {t("wiki.quality.issues")}
                        </Text>
                      </div>
                    </div>
                  </div>

                  {/* 影响因素 */}
                  {details.factors.length > 0 && (
                    <div style={{ marginBottom: 6 }}>
                      {details.factors.map((factor) => (
                        <div key={factor.name} className="flex items-center gap-1" style={{ padding: "1px 0" }}>
                          <Tag
                            color={factor.impact < -0.2
                              ? "error"
                              : factor.impact < -0.05
                              ? "warning"
                              : "default"}
                            style={{ fontSize: 9, margin: 0, lineHeight: "14px", padding: "0 3px" }}
                          >
                            {factor.name}
                          </Tag>
                          <Text type="secondary" style={{ fontSize: 9 }}>{factor.description}</Text>
                        </div>
                      ))}
                    </div>
                  )}

                  {/* 问题列表 */}
                  {details.issues.length > 0 && (
                    <div style={{ maxHeight: 100, overflowY: "auto" }}>
                      {details.issues.slice(0, 5).map((issue) => (
                        <div key={`${issue.line}-${issue.message}`} style={{ padding: "1px 0" }}>
                          <Space size={3}>
                            <Badge status={getIssueSeverityColor(issue.severity)} />
                            <Text style={{ fontSize: 9 }}>{issue.message}</Text>
                            {issue.line && (
                              <Tag style={{ fontSize: 8, margin: 0, lineHeight: "12px", padding: "0 2px" }}>
                                L{issue.line}
                              </Tag>
                            )}
                          </Space>
                        </div>
                      ))}
                      {details.issues.length > 5 && (
                        <Text type="secondary" style={{ fontSize: 9 }}>
                          +{details.issues.length - 5}
                        </Text>
                      )}
                    </div>
                  )}
                </>
              )
              : (
                <Empty
                  description={false}
                  image={Empty.PRESENTED_IMAGE_SIMPLE}
                  style={{ padding: "4px 0", margin: 0 }}
                  imageStyle={{ height: 20 }}
                >
                  <Text type="secondary" style={{ fontSize: 10 }}>{t("wiki.quality.noData")}</Text>
                </Empty>
              )}
          </div>
        }
      >
        <Tooltip title={t("wiki.quality.title")}>
          <Badge
            count={details && details.issues.length > 0 ? details.issues.length : 0}
            size="small"
            style={{
              backgroundColor: details
                ? getScoreColor(details.score)
                : "#52c41a",
            }}
            showZero={false}
          >
            <Button
              size="small"
              type="text"
              style={{
                color: details ? getScoreColor(details.score) : undefined,
                fontSize: 11,
                fontWeight: 600,
                padding: "0 4px",
                minWidth: 32,
                height: 26,
              }}
            >
              {percent}%
            </Button>
          </Badge>
        </Tooltip>
      </Popover>
    );
  }

  // ========== 完整面板模式 ==========
  if (loading) {
    return (
      <Card size="small">
        <div className="flex items-center justify-center py-4">
          <Spin size="small" />
        </div>
      </Card>
    );
  }

  if (!details) {
    return (
      <Card size="small">
        <Empty description={t("wiki.quality.noData")} />
      </Card>
    );
  }

  return (
    <QualityScorePanel
      loading={loading}
      refreshing={refreshing}
      details={details}
      onRefresh={() => {
        void loadQualityScore();
      }}
      getScoreColor={getScoreColor}
      getScoreIcon={getScoreIcon}
      getScoreLabel={getScoreLabel}
      getIssueSeverityColor={getIssueSeverityColor}
      t={t}
      standalone
    />
  );
}

// ========== 内部：质量分面板内容（Popover 和 standalone 共用） ==========
interface QualityScorePanelProps {
  loading: boolean;
  refreshing: boolean;
  details: QualityDetails | null;
  onRefresh: () => void;
  getScoreColor: (s: number) => string;
  getScoreIcon: (s: number) => React.ReactNode;
  getScoreLabel: (s: number) => string;
  getIssueSeverityColor: (s: string) => "success" | "error" | "processing" | "default" | "warning";
  t: (key: string, params?: Record<string, unknown>) => string;
  standalone?: boolean;
}

function QualityScorePanel({
  refreshing,
  details,
  onRefresh,
  getScoreColor,
  getScoreIcon,
  getScoreLabel,
  getIssueSeverityColor,
  t,
  standalone,
}: QualityScorePanelProps) {
  if (!details) {
    return <Empty description={t("wiki.quality.noData")} image={Empty.PRESENTED_IMAGE_SIMPLE} />;
  }

  const percent = Math.round(details.score * 100);

  const content = (
    <Space orientation="vertical" size="middle" style={{ width: "100%" }}>
      <div className="flex items-center gap-3">
        <Progress
          type="circle"
          percent={percent}
          size={standalone ? 80 : 60}
          strokeColor={getScoreColor(details.score)}
          format={() => <span style={{ fontSize: standalone ? 16 : 12, fontWeight: 700 }}>{percent}%</span>}
        />
        <div>
          <Space size={4}>
            {getScoreIcon(details.score)}
            <Text strong style={{ fontSize: standalone ? 14 : 13 }}>{getScoreLabel(details.score)}</Text>
          </Space>
          <div>
            <Text type="secondary" style={{ fontSize: 11 }}>
              {details.issues.length} {t("wiki.quality.issues")}
            </Text>
          </div>
        </div>
      </div>

      {details.factors.length > 0 && (
        <div>
          {details.factors.map((factor) => (
            <div key={factor.name} className="flex items-center gap-2" style={{ padding: "2px 0" }}>
              <Tag
                color={factor.impact < -0.2
                  ? "error"
                  : factor.impact < -0.05
                  ? "warning"
                  : "default"}
                style={{ fontSize: 10, margin: 0 }}
              >
                {factor.name}
              </Tag>
              <Text type="secondary" style={{ fontSize: 11 }}>{factor.description}</Text>
            </div>
          ))}
        </div>
      )}

      {details.issues.length > 0 && (
        <div style={{ maxHeight: standalone ? 160 : 120, overflowY: "auto" }}>
          {details.issues.slice(0, standalone ? 10 : 5).map((issue) => (
            <div key={`${issue.line}-${issue.message}`} style={{ padding: "2px 0" }}>
              <Space size={4}>
                <Badge status={getIssueSeverityColor(issue.severity)} />
                <Text style={{ fontSize: 10 }}>{issue.message}</Text>
                {issue.line && <Tag style={{ fontSize: 9, margin: 0 }}>L{issue.line}</Tag>}
              </Space>
            </div>
          ))}
          {details.issues.length > (standalone ? 10 : 5) && (
            <Text type="secondary" style={{ fontSize: 10 }}>
              +{details.issues.length - (standalone ? 10 : 5)} {t("wiki.quality.more")}
            </Text>
          )}
        </div>
      )}
    </Space>
  );

  if (standalone) {
    return (
      <Card
        size="small"
        title={
          <Space>
            <span>{t("wiki.quality.title")}</span>
            {refreshing && <Spin size="small" />}
          </Space>
        }
        extra={
          <Tooltip title={t("wiki.quality.refresh")}>
            <Button
              type="text"
              size="small"
              icon={<ReloadOutlined spin={refreshing} />}
              onClick={onRefresh}
            />
          </Tooltip>
        }
      >
        {content}
      </Card>
    );
  }
  return content;
}

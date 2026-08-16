// SPDX-License-Identifier: AGPL-3.0-only

import { useEvolutionStore } from "@/stores";
import type { EvolutionDecision } from "@/types";
import { Button, Card, Empty, Progress, Skeleton, Space, Tag, theme, Typography } from "antd";
import { BarChart3, Brain, CircleDot, RefreshCw, Route, TrendingUp } from "lucide-react";
import { useCallback, useEffect } from "react";
import { useTranslation } from "react-i18next";

const { Text } = Typography;

/** 进化决策 → 颜色（evolve 警示红 / observe 观察金 / stable 稳定绿） */
const DECISION_COLORS: Record<EvolutionDecision, string> = {
  evolve: "red",
  observe: "gold",
  stable: "green",
};

/**
 * 进化证据面板（T5A.4）：展示「决策标签流 → 贝叶斯后验」的进化决策，
 * 以及进化产物真实执行反馈（execution_feedback，真实成败证据）。
 */
export function EvolutionEvidencePanel({ conversationId }: { conversationId: string }) {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const evidence = useEvolutionStore((s) => s.evolutionEvidence);
  const loading = useEvolutionStore((s) => s.evidenceLoading);
  const fetchEvolutionEvidence = useEvolutionStore((s) => s.fetchEvolutionEvidence);

  const load = useCallback(() => {
    void fetchEvolutionEvidence(conversationId);
  }, [conversationId, fetchEvolutionEvidence]);

  // 会话切换时自动拉取
  useEffect(() => {
    load();
  }, [load]);

  return (
    <div style={{ padding: "12px 16px", display: "flex", flexDirection: "column", gap: 12 }}>
      {/* 决策概览 */}
      <Card
        size="small"
        styles={{ body: { padding: "12px 14px" } }}
        title={
          <Space size={6}>
            <Brain size={13} />
            <Text style={{ fontSize: 13 }}>{t("evolutionEvidence.title")}</Text>
          </Space>
        }
        extra={
          <Button
            type="text"
            size="small"
            icon={<RefreshCw size={13} />}
            loading={loading}
            onClick={load}
          />
        }
      >
        {loading && !evidence
          ? <Skeleton active paragraph={{ rows: 3 }} title={false} />
          : !evidence
          ? (
            <Empty
              image={Empty.PRESENTED_IMAGE_SIMPLE}
              description={
                <Text type="secondary" style={{ fontSize: 12 }}>
                  {t("evolutionEvidence.empty")}
                </Text>
              }
            />
          )
          : (
            <Space orientation="vertical" size={10} style={{ width: "100%" }}>
              {/* 决策标签 + 后验概率 */}
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", gap: 8 }}>
                <Space size={6}>
                  <Tag color={DECISION_COLORS[evidence.decision]} style={{ marginInlineEnd: 0 }}>
                    {t(`evolutionEvidence.decision.${evidence.decision}`)}
                  </Tag>
                  <Text type="secondary" style={{ fontSize: 11 }}>
                    {t("evolutionEvidence.pSuccess", {
                      value: Math.round(evidence.pSuccess * 100),
                    })}
                  </Text>
                </Space>
                <Progress
                  percent={Math.round(evidence.pSuccess * 100)}
                  size="small"
                  style={{ flex: 1, maxWidth: 120, marginInlineEnd: 0 }}
                />
              </div>

              {/* 决策原因 */}
              <Text type="secondary" style={{ fontSize: 12, lineHeight: 1.5 }}>
                {evidence.reason}
              </Text>

              {/* 证据量 / 标签消费 */}
              <div style={{ display: "flex", flexWrap: "wrap", gap: 6 }}>
                <Tag color="blue">
                  <BarChart3 size={11} style={{ verticalAlign: -1, marginRight: 2 }} />
                  {t("evolutionEvidence.evidenceVolume", { value: evidence.evidenceVolume.toFixed(1) })}
                </Tag>
                <Tag>
                  {t("evolutionEvidence.consumedLabels", {
                    consumed: evidence.consumedLabels,
                    total: evidence.totalLabels,
                  })}
                </Tag>
              </div>

              {/* 证据来源路由路径 */}
              {evidence.routePaths.length > 0 && (
                <div>
                  <Text strong style={{ fontSize: 12.5, display: "block", marginBottom: 6 }}>
                    {t("evolutionEvidence.routePaths")} ({evidence.routePaths.length})
                  </Text>
                  <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
                    {evidence.routePaths.map((path) => (
                      <div
                        key={path}
                        style={{
                          display: "flex",
                          alignItems: "center",
                          gap: 6,
                          fontSize: 12,
                          padding: "4px 8px",
                          borderRadius: 6,
                          backgroundColor: token.colorBgLayout,
                          wordBreak: "break-all",
                        }}
                      >
                        <Route size={11} style={{ color: token.colorTextTertiary, flexShrink: 0 }} />
                        {path}
                      </div>
                    ))}
                  </div>
                </div>
              )}
            </Space>
          )}
      </Card>

      {/* 真实执行反馈 */}
      <Card
        size="small"
        styles={{ body: { padding: "12px 14px" } }}
        title={
          <Space size={6}>
            <TrendingUp size={13} />
            <Text style={{ fontSize: 13 }}>{t("evolutionEvidence.executionTitle")}</Text>
          </Space>
        }
      >
        {loading && !evidence
          ? <Skeleton active paragraph={{ rows: 3 }} title={false} />
          : !evidence
          ? (
            <Empty
              image={Empty.PRESENTED_IMAGE_SIMPLE}
              description={
                <Text type="secondary" style={{ fontSize: 12 }}>
                  {t("evolutionEvidence.empty")}
                </Text>
              }
            />
          )
          : evidence.executionFeedback.totalRuns === 0
          ? (
            <Empty
              image={Empty.PRESENTED_IMAGE_SIMPLE}
              description={
                <Text type="secondary" style={{ fontSize: 12 }}>
                  {t("evolutionEvidence.noExecution")}
                </Text>
              }
            />
          )
          : (
            <Space orientation="vertical" size={10} style={{ width: "100%" }}>
              {/* 汇总统计 */}
              <div style={{ display: "flex", justifyContent: "space-between", gap: 8 }}>
                <Text type="secondary" style={{ fontSize: 12 }}>
                  {t("evolutionEvidence.totalRuns", {
                    value: evidence.executionFeedback.totalRuns,
                  })}
                </Text>
                <Text strong style={{ fontSize: 12 }}>
                  {t("evolutionEvidence.successRate", {
                    value: Math.round(evidence.executionFeedback.successRate * 100),
                  })}
                </Text>
              </div>
              <Progress
                percent={Math.round(evidence.executionFeedback.successRate * 100)}
                size="small"
                status={evidence.executionFeedback.successRate >= 0.6 ? "success" : "exception"}
              />
              <div style={{ display: "flex", flexWrap: "wrap", gap: 6 }}>
                <Tag color="success">
                  {t("evolutionEvidence.totalSuccesses", {
                    value: evidence.executionFeedback.totalSuccesses,
                  })}
                </Tag>
                <Tag color="error">
                  {t("evolutionEvidence.totalFailures", {
                    value: evidence.executionFeedback.totalFailures,
                  })}
                </Tag>
                <Tag>
                  {t("evolutionEvidence.toolCount", {
                    value: evidence.executionFeedback.toolCount,
                  })}
                </Tag>
              </div>

              {/* 按产物明细 */}
              {evidence.executionFeedback.details.length > 0 && (
                <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
                  {evidence.executionFeedback.details.map((d) => {
                    const successRate = d.usageCount > 0 ? d.successes / d.usageCount : 0;
                    return (
                      <div
                        key={d.toolId}
                        style={{
                          display: "flex",
                          alignItems: "center",
                          gap: 8,
                          padding: "6px 8px",
                          borderRadius: 6,
                          backgroundColor: token.colorBgLayout,
                        }}
                      >
                        <CircleDot
                          size={12}
                          style={{
                            flexShrink: 0,
                            color: successRate >= 0.6
                              ? token.colorSuccess
                              : successRate > 0
                              ? token.colorWarning
                              : token.colorError,
                          }}
                        />
                        <Text style={{ fontSize: 12, flex: 1, wordBreak: "break-all" }}>
                          {d.toolId}
                        </Text>
                        <Tag color={successRate >= 0.6 ? "success" : "error"} style={{ marginInlineEnd: 0 }}>
                          {t("evolutionEvidence.perSuccessRate", {
                            value: Math.round(successRate * 100),
                          })}
                        </Tag>
                      </div>
                    );
                  })}
                </div>
              )}
            </Space>
          )}
      </Card>
    </div>
  );
}

// SPDX-License-Identifier: AGPL-3.0-only

import { useIndustryLearningStore } from "@/stores";
import type { ExperiencePoolStats, RLPolicyUpdate } from "@/types";
import {
  BarChartOutlined,
  BulbOutlined,
  FallOutlined,
  MinusOutlined,
  ReloadOutlined,
  RiseOutlined,
  ThunderboltOutlined,
  TrophyOutlined,
} from "@ant-design/icons";
import { Alert, Button, Card, Col, Empty, Progress, Row, Space, Statistic, Tag, Typography } from "antd";
import { useCallback, useEffect } from "react";
import { useTranslation } from "react-i18next";

const { Text, Title } = Typography;

interface RLLearningPanelProps {
  /** 行业 ID */
  industryId?: string;
  /** 紧凑模式（嵌入其他页面） */
  compact?: boolean;
}

/**
 * RL 学习面板 — 展示经验池统计、策略优化状态和自动学习闭环历史
 */
export function RLLearningPanel({ industryId, compact = false }: RLLearningPanelProps) {
  const { t } = useTranslation();
  const {
    loadRLStats,
    loadConfig,
    triggerOptimization,
    rlStats,
    rlGlobalStats,
    rlPolicyUpdates,
    autoLearningHistory,
    rlLoading,
    getConfig,
  } = useIndustryLearningStore();

  const stats = industryId ? rlStats.get(industryId) ?? emptyStats() : rlGlobalStats ?? emptyStats();
  const config = industryId ? getConfig(industryId) : undefined;
  const policyUpdate = industryId ? rlPolicyUpdates.get(industryId) : undefined;

  const loadData = useCallback(async () => {
    await loadRLStats(industryId);
    if (industryId) {
      await loadConfig(industryId);
    }
  }, [industryId, loadRLStats, loadConfig]);

  useEffect(() => {
    loadData();
  }, [loadData]);

  const handleOptimize = async () => {
    if (industryId) {
      await triggerOptimization(industryId);
    }
  };

  return (
    <div className={compact ? "p-2" : "p-4"}>
      <Space direction="vertical" size="middle" style={{ width: "100%" }}>
        {/* 顶部操作栏 */}
        <Row justify="space-between" align="middle">
          <Col>
            <Space>
              <ThunderboltOutlined style={{ fontSize: 20, color: "#722ed1" }} />
              <Title level={compact ? 5 : 4} style={{ margin: 0 }}>
                {t("opc.rl.panelTitle")}
              </Title>
              {config?.reinforcementLearningEnabled
                ? <Tag color="green">{t("opc.rl.enabled")}</Tag>
                : <Tag color="default">{t("opc.rl.disabled")}</Tag>}
            </Space>
          </Col>
          <Col>
            <Space>
              <Button
                icon={<ReloadOutlined />}
                onClick={loadData}
                loading={rlLoading}
                size={compact ? "small" : "middle"}
              >
                {t("opc.rl.refresh")}
              </Button>
              {industryId && (
                <Button
                  icon={<BulbOutlined />}
                  onClick={handleOptimize}
                  loading={rlLoading}
                  disabled={!config?.reinforcementLearningEnabled}
                  size={compact ? "small" : "middle"}
                >
                  {t("opc.rl.optimize")}
                </Button>
              )}
            </Space>
          </Col>
        </Row>

        {/* 经验池统计卡片 */}
        <Row gutter={compact ? 8 : 16}>
          <Col xs={12} md={6}>
            <Card size={compact ? "small" : "default"}>
              <Statistic
                title={t("opc.rl.totalExperiences")}
                value={stats.totalExperiences}
                prefix={<BarChartOutlined />}
              />
            </Card>
          </Col>
          <Col xs={12} md={6}>
            <Card size={compact ? "small" : "default"}>
              <Statistic
                title={t("opc.rl.industryCount")}
                value={stats.industryCount}
              />
            </Card>
          </Col>
          <Col xs={12} md={6}>
            <Card size={compact ? "small" : "default"}>
              <Statistic
                title={t("opc.rl.avgReward")}
                value={stats.avgReward.toFixed(3)}
                valueStyle={{
                  color: stats.avgReward >= 0.6 ? "#3f8600" : stats.avgReward >= 0.3 ? "#d48806" : "#cf1322",
                }}
                prefix={<TrophyOutlined />}
              />
            </Card>
          </Col>
          <Col xs={12} md={6}>
            <Card size={compact ? "small" : "default"}>
              <Statistic
                title={t("opc.rl.successRate")}
                value={stats.successRate.toFixed(1)}
                suffix="%"
                prefix={stats.successRate >= 70
                  ? <RiseOutlined style={{ color: "#3f8600" }} />
                  : stats.successRate >= 40
                  ? <MinusOutlined style={{ color: "#d48806" }} />
                  : <FallOutlined style={{ color: "#cf1322" }} />}
              />
            </Card>
          </Col>
        </Row>

        {/* 策略优化结果 */}
        {policyUpdate && (
          <Card
            title={
              <Space>
                <BulbOutlined style={{ color: "#faad14" }} />
                <span>{t("opc.rl.policyUpdate")}</span>
              </Space>
            }
            size={compact ? "small" : "default"}
          >
            <PolicyUpdateContent update={policyUpdate} compact={compact} />
          </Card>
        )}

        {/* 自动学习历史 */}
        <Card
          title={
            <Space>
              <ThunderboltOutlined />
              <span>{t("opc.rl.autoLearningHistory")}</span>
              <Tag>{autoLearningHistory.length}</Tag>
            </Space>
          }
          size={compact ? "small" : "default"}
        >
          {autoLearningHistory.length === 0
            ? (
              <Empty
                description={t("opc.rl.noHistory")}
                image={Empty.PRESENTED_IMAGE_SIMPLE}
              />
            )
            : (
              <Space direction="vertical" size="small" style={{ width: "100%" }}>
                {autoLearningHistory.slice(0, compact ? 3 : 5).map((result, idx) => (
                  <AutoLearningResultItem key={idx} result={result} compact={compact} />
                ))}
              </Space>
            )}
        </Card>
      </Space>
    </div>
  );
}

function PolicyUpdateContent({
  update,
  compact,
}: {
  update: RLPolicyUpdate;
  compact: boolean;
}) {
  const { t } = useTranslation();
  const trendIcon = update.rewardTrend === "improving"
    ? <RiseOutlined style={{ color: "#3f8600" }} />
    : update.rewardTrend === "declining"
    ? <FallOutlined style={{ color: "#cf1322" }} />
    : <MinusOutlined style={{ color: "#8c8c8c" }} />;

  const trendLabel = update.rewardTrend === "improving"
    ? t("opc.rl.trendImproving")
    : update.rewardTrend === "declining"
    ? t("opc.rl.trendDeclining")
    : t("opc.rl.trendStable");

  return (
    <Space direction="vertical" size={compact ? "small" : "middle"} style={{ width: "100%" }}>
      <Row gutter={16}>
        <Col span={8}>
          <Text type="secondary">{t("opc.rl.experiencesUsed")}</Text>
          <div>
            <Text strong>{update.experiencesUsed}</Text>
          </div>
        </Col>
        <Col span={8}>
          <Text type="secondary">{t("opc.rl.rewardTrend")}</Text>
          <div>
            {trendIcon} <Text strong>{trendLabel}</Text>
          </div>
        </Col>
        <Col span={8}>
          <Text type="secondary">{t("opc.rl.avgReward")}</Text>
          <div>
            <Text strong>{update.avgReward.toFixed(3)}</Text>
          </div>
        </Col>
      </Row>

      {update.suggestedAdjustments.length > 0 && (
        <div>
          <Text type="secondary">{t("opc.rl.suggestions")}:</Text>
          <div style={{ marginTop: 8 }}>
            <Space wrap>
              {update.suggestedAdjustments.map((s, i) => (
                <Tag key={i} color="blue">
                  {s}
                </Tag>
              ))}
            </Space>
          </div>
        </div>
      )}

      {update.reflectionThreshold !== undefined && (
        <Alert
          message={t("opc.rl.thresholdAdjusted")}
          description={t("opc.rl.newThreshold") + `: ${update.reflectionThreshold.toFixed(2)}`}
          type="info"
          showIcon
        />
      )}

      {update.evolutionTriggerAdjusted !== undefined && (
        <Alert
          message={t("opc.rl.evolutionTriggerAdjusted")}
          description={update.evolutionTriggerAdjusted
            ? t("opc.rl.moreAggressive")
            : t("opc.rl.moreConservative")}
          type="warning"
          showIcon
        />
      )}
    </Space>
  );
}

function AutoLearningResultItem({
  result,
  compact,
}: {
  result: import("@/types").AutoLearningResult;
  compact: boolean;
}) {
  const { t } = useTranslation();
  const getStatusColor = (status: string) => {
    switch (status) {
      case "success":
        return "green";
      case "failed":
        return "red";
      case "skipped":
        return "default";
      default:
        return "default";
    }
  };

  const getStatusLabel = (status: string) => {
    switch (status) {
      case "success":
        return t("opc.rl.statusSuccess");
      case "failed":
        return t("opc.rl.statusFailed");
      case "skipped":
        return t("opc.rl.statusSkipped");
      default:
        return status;
    }
  };

  return (
    <div
      style={{
        padding: compact ? 8 : 12,
        border: "1px solid #f0f0f0",
        borderRadius: 8,
        background: "#fafafa",
      }}
    >
      <Space direction="vertical" size={4} style={{ width: "100%" }}>
        <Row justify="space-between">
          <Text type="secondary" style={{ fontSize: 12 }}>
            {new Date(result.triggeredAt).toLocaleString()}
          </Text>
        </Row>
        <Row gutter={8}>
          <Col>
            <Tag color={getStatusColor(result.reflection.status)}>
              {t("opc.rl.reflection")}: {getStatusLabel(result.reflection.status)}
            </Tag>
            {result.reflection.qualityScore !== undefined && (
              <Progress
                percent={Math.round((result.reflection.qualityScore as number) * 100)}
                size="small"
                style={{ display: "inline-block", width: 60 }}
              />
            )}
          </Col>
          {result.evolution && (
            <Col>
              <Tag color={getStatusColor(result.evolution.status)}>
                {t("opc.rl.evolution")}: {getStatusLabel(result.evolution.status)}
              </Tag>
            </Col>
          )}
          {result.selfImprovement && (
            <Col>
              <Tag color={getStatusColor(result.selfImprovement.status)}>
                {t("opc.rl.selfImprovement")}: {getStatusLabel(result.selfImprovement.status)}
              </Tag>
            </Col>
          )}
          {result.reinforcementLearning && (
            <Col>
              <Tag color={getStatusColor(result.reinforcementLearning.status)}>
                {t("opc.rl.rl")}: {getStatusLabel(result.reinforcementLearning.status)}
              </Tag>
            </Col>
          )}
        </Row>
      </Space>
    </div>
  );
}

function emptyStats(): ExperiencePoolStats {
  return {
    totalExperiences: 0,
    industryCount: 0,
    oldestTimestampMs: undefined,
    newestTimestampMs: undefined,
    avgReward: 0,
    successRate: 0,
  };
}

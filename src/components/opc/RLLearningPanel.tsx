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
                {t("opc.rl.panelTitle", "强化学习面板")}
              </Title>
              {config?.reinforcement_learning_enabled
                ? <Tag color="green">{t("opc.rl.enabled", "已启用")}</Tag>
                : <Tag color="default">{t("opc.rl.disabled", "未启用")}</Tag>}
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
                {t("opc.rl.refresh", "刷新")}
              </Button>
              {industryId && (
                <Button
                  icon={<BulbOutlined />}
                  onClick={handleOptimize}
                  loading={rlLoading}
                  disabled={!config?.reinforcement_learning_enabled}
                  size={compact ? "small" : "middle"}
                >
                  {t("opc.rl.optimize", "策略优化")}
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
                title={t("opc.rl.totalExperiences", "总经验数")}
                value={stats.total_experiences}
                prefix={<BarChartOutlined />}
              />
            </Card>
          </Col>
          <Col xs={12} md={6}>
            <Card size={compact ? "small" : "default"}>
              <Statistic
                title={t("opc.rl.industryCount", "行业数")}
                value={stats.industry_count}
              />
            </Card>
          </Col>
          <Col xs={12} md={6}>
            <Card size={compact ? "small" : "default"}>
              <Statistic
                title={t("opc.rl.avgReward", "平均奖励")}
                value={stats.avg_reward.toFixed(3)}
                valueStyle={{
                  color: stats.avg_reward >= 0.6 ? "#3f8600" : stats.avg_reward >= 0.3 ? "#d48806" : "#cf1322",
                }}
                prefix={<TrophyOutlined />}
              />
            </Card>
          </Col>
          <Col xs={12} md={6}>
            <Card size={compact ? "small" : "default"}>
              <Statistic
                title={t("opc.rl.successRate", "成功率")}
                value={stats.success_rate.toFixed(1)}
                suffix="%"
                prefix={stats.success_rate >= 70
                  ? <RiseOutlined style={{ color: "#3f8600" }} />
                  : stats.success_rate >= 40
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
                <span>{t("opc.rl.policyUpdate", "策略优化结果")}</span>
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
              <span>{t("opc.rl.autoLearningHistory", "自动学习闭环历史")}</span>
              <Tag>{autoLearningHistory.length}</Tag>
            </Space>
          }
          size={compact ? "small" : "default"}
        >
          {autoLearningHistory.length === 0
            ? (
              <Empty
                description={t("opc.rl.noHistory", "尚无学习历史")}
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
  const trendIcon = update.reward_trend === "improving"
    ? <RiseOutlined style={{ color: "#3f8600" }} />
    : update.reward_trend === "declining"
    ? <FallOutlined style={{ color: "#cf1322" }} />
    : <MinusOutlined style={{ color: "#8c8c8c" }} />;

  const trendLabel = update.reward_trend === "improving"
    ? t("opc.rl.trendImproving", "上升")
    : update.reward_trend === "declining"
    ? t("opc.rl.trendDeclining", "下降")
    : t("opc.rl.trendStable", "稳定");

  return (
    <Space direction="vertical" size={compact ? "small" : "middle"} style={{ width: "100%" }}>
      <Row gutter={16}>
        <Col span={8}>
          <Text type="secondary">{t("opc.rl.experiencesUsed", "使用经验")}</Text>
          <div>
            <Text strong>{update.experiences_used}</Text>
          </div>
        </Col>
        <Col span={8}>
          <Text type="secondary">{t("opc.rl.rewardTrend", "奖励趋势")}</Text>
          <div>
            {trendIcon} <Text strong>{trendLabel}</Text>
          </div>
        </Col>
        <Col span={8}>
          <Text type="secondary">{t("opc.rl.avgReward", "平均奖励")}</Text>
          <div>
            <Text strong>{update.avg_reward.toFixed(3)}</Text>
          </div>
        </Col>
      </Row>

      {update.suggested_adjustments.length > 0 && (
        <div>
          <Text type="secondary">{t("opc.rl.suggestions", "优化建议")}:</Text>
          <div style={{ marginTop: 8 }}>
            <Space wrap>
              {update.suggested_adjustments.map((s, i) => (
                <Tag key={i} color="blue">
                  {s}
                </Tag>
              ))}
            </Space>
          </div>
        </div>
      )}

      {update.reflection_threshold !== undefined && (
        <Alert
          message={t("opc.rl.thresholdAdjusted", "反思阈值调整")}
          description={t("opc.rl.newThreshold", "新阈值") + `: ${update.reflection_threshold.toFixed(2)}`}
          type="info"
          showIcon
        />
      )}

      {update.evolution_trigger_adjusted !== undefined && (
        <Alert
          message={t("opc.rl.evolutionTriggerAdjusted", "进化触发条件调整")}
          description={update.evolution_trigger_adjusted
            ? t("opc.rl.moreAggressive", "更积极触发进化")
            : t("opc.rl.moreConservative", "更保守触发进化")}
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
        return t("opc.rl.statusSuccess", "成功");
      case "failed":
        return t("opc.rl.statusFailed", "失败");
      case "skipped":
        return t("opc.rl.statusSkipped", "跳过");
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
            {new Date(result.triggered_at).toLocaleString()}
          </Text>
        </Row>
        <Row gutter={8}>
          <Col>
            <Tag color={getStatusColor(result.reflection.status)}>
              {t("opc.rl.reflection", "反思")}: {getStatusLabel(result.reflection.status)}
            </Tag>
            {result.reflection.quality_score !== undefined && (
              <Progress
                percent={Math.round((result.reflection.quality_score as number) * 100)}
                size="small"
                style={{ display: "inline-block", width: 60 }}
              />
            )}
          </Col>
          {result.evolution && (
            <Col>
              <Tag color={getStatusColor(result.evolution.status)}>
                {t("opc.rl.evolution", "进化")}: {getStatusLabel(result.evolution.status)}
              </Tag>
            </Col>
          )}
          {result.self_improvement && (
            <Col>
              <Tag color={getStatusColor(result.self_improvement.status)}>
                {t("opc.rl.selfImprovement", "自改进")}: {getStatusLabel(result.self_improvement.status)}
              </Tag>
            </Col>
          )}
          {result.reinforcement_learning && (
            <Col>
              <Tag color={getStatusColor(result.reinforcement_learning.status)}>
                {t("opc.rl.rl", "RL")}: {getStatusLabel(result.reinforcement_learning.status)}
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
    total_experiences: 0,
    industry_count: 0,
    oldest_timestamp_ms: undefined,
    newest_timestamp_ms: undefined,
    avg_reward: 0,
    success_rate: 0,
  };
}

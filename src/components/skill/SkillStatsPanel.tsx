// SPDX-License-Identifier: AGPL-3.0-only

/**
 * SkillStatsPanel — Skill 执行统计仪表盘
 *
 * 展示 Skill 的执行成功率、平均耗时、使用次数等关键指标。
 *
 * @module components/skill/SkillStatsPanel
 */

import { invoke, logIpcError } from "@/lib/invoke";
import { Card, Col, Progress, Row, Statistic, Typography } from "antd";
import { Clock, Target, TrendingUp, Zap } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

// SK-P1-2: 后端暂未实现 get_skill_execution_stats,定义前端类型
// TODO: 后端实现后将此类型迁移到 @/types
interface SkillExecutionStats {
  name: string;
  successRate: number;
  avgExecutionTimeMs: number;
  totalUsages: number;
  successfulUsages: number;
  qualityScore: number;
  lastUsedAt?: string;
}

export function SkillStatsPanel() {
  const { t } = useTranslation();
  const [stats, setStats] = useState<SkillExecutionStats[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    let cancelled = false;
    /* eslint-disable react-hooks/set-state-in-effect */
    setLoading(true);
    /* eslint-enable react-hooks/set-state-in-effect */

    async function load() {
      try {
        const allStats = await invoke<SkillExecutionStats[]>(
          "get_skill_execution_stats",
        );

        if (!cancelled) {
          const merged = allStats.map((s) => {
            return {
              ...s,
              successRate: s.successRate * 100,
              qualityScore: (s.qualityScore ?? 0.5) * 100,
            };
          });
          setStats(merged.sort((a, b) => b.totalUsages - a.totalUsages));
        }
      } catch (e) {
        // SK-P0-4: 后端未实现该命令,优雅降级显示空状态而非错误
        logIpcError("get_skill_execution_stats")(e);
        if (!cancelled) {
          setStats([]);
        }
      } finally {
        if (!cancelled) {
          setLoading(false);
        }
      }
    }

    load();
    return () => {
      cancelled = true;
    };
  }, []);

  if (loading) {
    return (
      <Typography.Text type="secondary">
        {t("skill.stats.loading")}
      </Typography.Text>
    );
  }

  if (stats.length === 0) {
    return (
      <Typography.Text type="secondary">
        {t("skill.stats.empty")}
      </Typography.Text>
    );
  }

  const totalUsages = stats.reduce((sum, s) => sum + s.totalUsages, 0);
  const avgSuccess = stats.length > 0
    ? stats.reduce((sum, s) => sum + s.successRate, 0) / stats.length
    : 0;
  const timedStats = stats.filter((s) => s.avgExecutionTimeMs > 0);
  const avgTime = timedStats.length > 0
    ? timedStats.reduce((sum, s) => sum + s.avgExecutionTimeMs, 0)
      / timedStats.length
    : 0;

  return (
    <div>
      <Typography.Title level={5} style={{ marginBottom: 16 }}>
        {t("skill.stats.title")}
      </Typography.Title>

      <Row gutter={[16, 16]} style={{ marginBottom: 24 }}>
        <Col span={6}>
          <Card size="small">
            <Statistic
              title={t("skill.stats.totalUsages")}
              value={totalUsages}
              prefix={<Zap size={16} />}
            />
          </Card>
        </Col>
        <Col span={6}>
          <Card size="small">
            <Statistic
              title={t("skill.stats.avgSuccess")}
              value={avgSuccess}
              precision={1}
              suffix="%"
              prefix={<Target size={16} />}
            />
          </Card>
        </Col>
        <Col span={6}>
          <Card size="small">
            <Statistic
              title={t("skill.stats.avgTime")}
              value={avgTime}
              precision={0}
              suffix="ms"
              prefix={<Clock size={16} />}
            />
          </Card>
        </Col>
        <Col span={6}>
          <Card size="small">
            <Statistic
              title={t("skill.stats.totalSkills")}
              value={stats.length}
              prefix={<TrendingUp size={16} />}
            />
          </Card>
        </Col>
      </Row>

      <Typography.Text strong style={{ display: "block", marginBottom: 8 }}>
        {t("skill.stats.perSkill")}
      </Typography.Text>

      {stats.map((s) => (
        <Card key={s.name} size="small" style={{ marginBottom: 8 }}>
          <Row align="middle" gutter={16}>
            <Col span={6}>
              <Typography.Text strong>{s.name}</Typography.Text>
              <Typography.Text
                type="secondary"
                style={{ display: "block", fontSize: 12 }}
              >
                {s.totalUsages > 0
                  ? t("skill.stats.usedCount", { count: s.totalUsages })
                  : t("skill.stats.neverUsed")}
              </Typography.Text>
            </Col>
            <Col span={6}>
              <Typography.Text type="secondary" style={{ fontSize: 12 }}>
                {t("skill.stats.successRate")}
              </Typography.Text>
              <Progress
                percent={s.successRate}
                size="small"
                status={s.successRate >= 80
                  ? "success"
                  : s.successRate >= 50
                  ? "normal"
                  : "exception"}
                format={(p) => `${p?.toFixed(0)}%`}
              />
            </Col>
            <Col span={6}>
              <Typography.Text type="secondary" style={{ fontSize: 12 }}>
                {t("skill.stats.quality")}
              </Typography.Text>
              <Progress
                percent={s.qualityScore}
                size="small"
                status={s.qualityScore >= 70 ? "success" : "normal"}
                format={(p) => `${p?.toFixed(0)}%`}
              />
            </Col>
            <Col span={6}>
              {s.avgExecutionTimeMs > 0 && (
                <Typography.Text type="secondary" style={{ fontSize: 12 }}>
                  {s.avgExecutionTimeMs}ms
                </Typography.Text>
              )}
            </Col>
          </Row>
        </Card>
      ))}
    </div>
  );
}

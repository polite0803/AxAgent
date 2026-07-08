/**
 * R1 复盘→进化：EvolutionDriftPanel
 *
 * 三段 UI：
 * 1. 权重表（每个 (strategy, period) 的现权重 / 旧权重 / 变化 / 胜率 / 样本 / 解释）
 * 2. 权重时间线 sparkline（点击某行后展开）
 * 3. 调整原因 Top 5
 *
 * 数据来源：stockAnalysisStore.evolutionDashboard
 * 重算按钮：调用 stockAnalysisStore.recalcEvolutionNow
 */

import {
  ArrowDownOutlined,
  ArrowUpOutlined,
  MinusOutlined,
  ReloadOutlined,
  ThunderboltOutlined,
} from "@ant-design/icons";
import { App, Button, Empty, Spin, Table, Tag, Tooltip } from "antd";
import type { ColumnsType } from "antd/es/table";
import type { ReactNode } from "react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import {
  type EvolutionRecentChangeRow,
  type EvolutionStrategyStatRow,
  type EvolutionStrategySummaryRow,
  useStockAnalysisStore,
} from "@/stores/feature/stockAnalysisStore";
import { useTimeAnchorStore } from "@/stores/feature/timeAnchorStore";

const trendIcon: Record<string, ReactNode> = {
  up: <ArrowUpOutlined style={{ color: "#cf1322" }} />,
  down: <ArrowDownOutlined style={{ color: "#389e0d" }} />,
  stable: <MinusOutlined style={{ color: "#8c8c8c" }} />,
};

function formatTime(ms: number): string {
  if (!ms) { return "—"; }
  const d = new Date(ms);
  return d.toLocaleString();
}

export function EvolutionDriftPanel() {
  const { message } = App.useApp();
  const { t } = useTranslation();
  const dashboard = useStockAnalysisStore((s) => s.evolutionDashboard);
  const recalculating = useStockAnalysisStore((s) => s.evolutionRecalculating);
  const lastError = useStockAnalysisStore((s) => s.evolutionLastError);
  const fetchDashboard = useStockAnalysisStore((s) => s.fetchEvolutionDashboard);
  const recalc = useStockAnalysisStore((s) => s.recalcEvolutionNow);
  const fetchAgreementHistory = useStockAnalysisStore((s) => s.fetchAgreementScoreHistory);
  const agreementHistory = useStockAnalysisStore((s) => s.agreementScoreHistory);
  const agreementLoading = useStockAnalysisStore((s) => s.agreementScoreHistoryLoading);
  const asOfDate = useTimeAnchorStore((s) => s.asOfDate);
  const [selectedKey, setSelectedKey] = useState<string | null>(null);
  const [timelineData, setTimelineData] = useState<
    Array<{ appliedAt: number; newWeight: number; deltaPct: number; trigger: string }>
  >(
    [],
  );
  const [timelineLoading, setTimelineLoading] = useState(false);

  useEffect(() => {
    fetchDashboard(asOfDate);
    fetchAgreementHistory(50);
  }, [asOfDate, fetchDashboard, fetchAgreementHistory]);

  const handleRecalc = async () => {
    await recalc(asOfDate);
    if (!lastError) {
      message.success(t("stockAnalysis.evolutionDrift.recalcSuccess"));
    }
  };

  const handleSelect = async (record: EvolutionStrategyStatRow) => {
    const key = `${record.strategyId}_${record.period}`;
    setSelectedKey(key);
    setTimelineLoading(true);
    try {
      const points = await (await import("@/lib/invoke")).invoke<
        Array<{
          appliedAt: number;
          newWeight: number;
          oldWeight: number;
          deltaPct: number;
          trigger: string;
        }>
      >("get_evolution_drift_timeline", {
        strategyId: record.strategyId,
        period: record.period,
        limit: 30,
      });
      setTimelineData(
        points.map((p) => ({
          appliedAt: p.appliedAt,
          newWeight: p.newWeight,
          deltaPct: p.deltaPct,
          trigger: p.trigger,
        })),
      );
    } catch (e) {
      console.error("[EvolutionDriftPanel] timeline fetch failed:", e);
      setTimelineData([]);
    } finally {
      setTimelineLoading(false);
    }
  };

  if (!dashboard) {
    return (
      <div style={{ padding: 16, textAlign: "center" }}>
        <Spin tip={t("stockAnalysis.evolutionDrift.loading")} />
        {lastError && (
          <div style={{ marginTop: 12, color: "#cf1322" }}>
            {t("stockAnalysis.evolutionDrift.error", { msg: lastError })}
          </div>
        )}
      </div>
    );
  }

  const columns: ColumnsType<EvolutionStrategyStatRow> = [
    {
      title: t("stockAnalysis.evolutionDrift.table.strategy"),
      dataIndex: "strategyId",
      key: "strategyId",
      width: 100,
      render: (v: string) => <Tag color="blue">{v}</Tag>,
    },
    {
      title: t("stockAnalysis.evolutionDrift.table.period"),
      dataIndex: "period",
      key: "period",
      width: 80,
      render: (v: string) => <Tag>{v}</Tag>,
    },
    {
      title: t("stockAnalysis.evolutionDrift.table.oldWeight"),
      dataIndex: "oldWeight",
      key: "oldWeight",
      width: 90,
      align: "right",
      render: (v: number) => v.toFixed(2),
    },
    {
      title: t("stockAnalysis.evolutionDrift.table.newWeight"),
      dataIndex: "newWeight",
      key: "newWeight",
      width: 90,
      align: "right",
      render: (v: number) => (
        <strong style={{ color: v > 1.0 ? "#cf1322" : v < 1.0 ? "#389e0d" : undefined }}>
          {v.toFixed(2)}
        </strong>
      ),
    },
    {
      title: t("stockAnalysis.evolutionDrift.table.delta"),
      dataIndex: "deltaPct",
      key: "deltaPct",
      width: 90,
      align: "right",
      render: (v: number) => {
        if (Math.abs(v) < 0.5) { return <span style={{ color: "#8c8c8c" }}>—</span>; }
        const color = v > 0 ? "#cf1322" : "#389e0d";
        return <span style={{ color, fontWeight: 500 }}>{(v > 0 ? "+" : "") + v.toFixed(1) + "%"}</span>;
      },
    },
    {
      title: t("stockAnalysis.evolutionDrift.table.winRate"),
      dataIndex: "winRate",
      key: "winRate",
      width: 80,
      align: "right",
      render: (v: number) => `${(v * 100).toFixed(0)}%`,
    },
    {
      title: t("stockAnalysis.evolutionDrift.table.sampleSize"),
      dataIndex: "sampleSize",
      key: "sampleSize",
      width: 80,
      align: "right",
      render: (v: number) => (
        <Tooltip title={t("stockAnalysis.evolutionDrift.table.confidenceLabel", { c: 0 })}>
          <Tag color={v >= 20 ? "green" : v >= 5 ? "orange" : "default"}>{v}</Tag>
        </Tooltip>
      ),
    },
    {
      title: t("stockAnalysis.evolutionDrift.table.rationale"),
      dataIndex: "rationale",
      key: "rationale",
      ellipsis: true,
    },
  ];

  return (
    <div style={{ padding: 16 }}>
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 16 }}>
        <h3 style={{ margin: 0 }}>
          <ThunderboltOutlined /> {t("stockAnalysis.evolutionDrift.title")}
        </h3>
        <Button
          type="primary"
          icon={<ReloadOutlined />}
          loading={recalculating}
          onClick={handleRecalc}
        >
          {t("stockAnalysis.evolutionDrift.recalcNow")}
        </Button>
      </div>

      <div style={{ marginBottom: 12, fontSize: 12, color: "#8c8c8c" }}>
        {t("stockAnalysis.evolutionDrift.lastRecalcAt", {
          time: formatTime(dashboard.lastRecalcAt),
        })}
        {asOfDate && (
          <Tag color="purple" style={{ marginLeft: 8 }}>
            {t("stockAnalysis.evolutionDrift.replayMode", { date: asOfDate })}
          </Tag>
        )}
      </div>

      <div style={{ marginBottom: 24 }}>
        <h4 style={{ marginTop: 0 }}>
          {t("stockAnalysis.evolutionDrift.summaryTitle")}
        </h4>
        <div style={{ display: "flex", gap: 12, flexWrap: "wrap" }}>
          {dashboard.strategySummary.map((s: EvolutionStrategySummaryRow) => (
            <div
              key={s.strategyId}
              style={{
                border: "1px solid #d9d9d9",
                borderRadius: 4,
                padding: "8px 12px",
                minWidth: 140,
              }}
            >
              <div style={{ fontSize: 12, color: "#8c8c8c" }}>{s.strategyId}</div>
              <div style={{ display: "flex", alignItems: "center", gap: 4, marginTop: 4 }}>
                {trendIcon[s.trend] ?? trendIcon.stable}
                <span style={{ fontSize: 18, fontWeight: 600 }}>{s.avgWeight.toFixed(2)}</span>
              </div>
              <div style={{ fontSize: 11, color: "#8c8c8c", marginTop: 2 }}>
                {t("stockAnalysis.evolutionDrift.summarySamples", { n: s.totalSamples })} ·{" "}
                {t("stockAnalysis.evolutionDrift.summaryWinRate", { p: (s.avgWinRate * 100).toFixed(0) })}
              </div>
            </div>
          ))}
        </div>
      </div>

      <h4>{t("stockAnalysis.evolutionDrift.tableTitle")}</h4>
      <Table
        size="small"
        rowKey={(r) => `${r.strategyId}_${r.period}`}
        columns={columns}
        dataSource={dashboard.stats}
        pagination={false}
        rowClassName={(r) => `${r.strategyId}_${r.period}` === selectedKey ? "ant-table-row-selected" : ""}
        onRow={(r) => ({
          onClick: () => handleSelect(r),
          style: { cursor: "pointer" },
        })}
        locale={{
          emptyText: <Empty description={t("stockAnalysis.evolutionDrift.noData")} />,
        }}
      />

      {selectedKey && (
        <div style={{ marginTop: 16, padding: 12, background: "#fafafa", borderRadius: 4 }}>
          <h4 style={{ marginTop: 0 }}>
            {t("stockAnalysis.evolutionDrift.timelineTitle", { key: selectedKey })}
          </h4>
          {timelineLoading
            ? <Spin size="small" />
            : timelineData.length === 0
            ? <Empty description={t("stockAnalysis.evolutionDrift.noTimeline")} />
            : <TimelineSparkline points={timelineData} />}
        </div>
      )}

      <div style={{ marginTop: 24 }}>
        <h4>{t("stockAnalysis.evolutionDrift.reasonsTitle")}</h4>
        {dashboard.recentChanges.length === 0
          ? <Empty description={t("stockAnalysis.evolutionDrift.noReasons")} />
          : (
            <ul style={{ paddingLeft: 16, margin: 0 }}>
              {dashboard.recentChanges.map((r: EvolutionRecentChangeRow) => (
                <li key={r.id} style={{ marginBottom: 8, fontSize: 13 }}>
                  <Tag color="blue">{r.strategyId}</Tag>
                  <Tag>{r.period}</Tag>
                  <Tag color={r.deltaPct > 0 ? "red" : r.deltaPct < 0 ? "green" : "default"}>
                    {(r.deltaPct > 0 ? "+" : "") + r.deltaPct.toFixed(1) + "%"}
                  </Tag>
                  <span style={{ color: "#8c8c8c", marginLeft: 8 }}>{formatTime(r.appliedAt)}</span>
                  {r.rationale && (
                    <div style={{ color: "#595959", marginTop: 2, marginLeft: 8 }}>
                      {r.rationale}
                    </div>
                  )}
                </li>
              ))}
            </ul>
          )}
      </div>

      {/* Phase 3: 双视角一致性趋势 */}
      <div style={{ marginTop: 24 }}>
        <h4>{t("stockAnalysis.evolutionDrift.agreementTrendTitle")}</h4>
        {agreementLoading
          ? <Spin size="small" />
          : !agreementHistory || agreementHistory.length === 0
          ? <Empty description={t("stockAnalysis.evolutionDrift.noAgreementData")} />
          : (
            <div>
              <div className="flex items-center gap-2 mb-2">
                <span className="text-xs" style={{ color: "var(--muted)" }}>
                  {t("stockAnalysis.evolutionDrift.agreementRecentAvg")}: {Math.round(
                    agreementHistory.slice(0, 10).reduce((s, r) => s + r.agreementScore, 0)
                      / Math.min(10, agreementHistory.length),
                  )}/100
                </span>
              </div>
              <AgreementSparkline points={agreementHistory} />
            </div>
          )}
      </div>
    </div>
  );
}

/** 极简 sparkline：避免引入 recharts/chart 包 */
function TimelineSparkline({
  points,
}: {
  points: Array<{ appliedAt: number; newWeight: number; deltaPct: number; trigger: string }>;
}) {
  if (points.length === 0) { return null; }
  const minW = Math.min(...points.map((p) => p.newWeight));
  const maxW = Math.max(...points.map((p) => p.newWeight));
  const range = maxW - minW || 0.01;
  const width = 600;
  const height = 60;
  const xStep = points.length > 1 ? width / (points.length - 1) : width;
  const path = points
    .map((p, i) => {
      const x = i * xStep;
      const y = height - ((p.newWeight - minW) / range) * (height - 10) - 5;
      return `${i === 0 ? "M" : "L"}${x.toFixed(1)},${y.toFixed(1)}`;
    })
    .join(" ");
  return (
    <svg width={width} height={height} style={{ display: "block" }}>
      <line x1={0} y1={height - 5} x2={width} y2={height - 5} stroke="#d9d9d9" strokeWidth={1} />
      <path d={path} fill="none" stroke="#1677ff" strokeWidth={2} />
      {points.map((p, i) => {
        const x = i * xStep;
        const y = height - ((p.newWeight - minW) / range) * (height - 10) - 5;
        return <circle key={i} cx={x} cy={y} r={3} fill="#1677ff" />;
      })}
    </svg>
  );
}

/** Phase 3: 一致性分数趋势 sparkline（紫色调） */
function AgreementSparkline({
  points,
}: {
  points: Array<
    {
      exitAt: number;
      agreementScore: number;
      stockCode: string;
      stockName: string;
      returnPct: number;
      wasCorrect: number;
    }
  >;
}) {
  const { t } = useTranslation();
  if (points.length === 0) { return null; }
  const minS = Math.min(...points.map((p) => p.agreementScore));
  const maxS = Math.max(...points.map((p) => p.agreementScore));
  const range = maxS - minS || 0.01;
  const width = 600;
  const height = 60;
  const xStep = points.length > 1 ? width / (points.length - 1) : width;
  const path = points
    .map((p, i) => {
      const x = i * xStep;
      const y = height - ((p.agreementScore - minS) / range) * (height - 10) - 5;
      return `${i === 0 ? "M" : "L"}${x.toFixed(1)},${y.toFixed(1)}`;
    })
    .join(" ");
  return (
    <div>
      <svg width={width} height={height} style={{ display: "block" }}>
        <line x1={0} y1={height - 5} x2={width} y2={height - 5} stroke="#d9d9d9" strokeWidth={1} />
        <path d={path} fill="none" stroke="#7c3aed" strokeWidth={2} />
        {points.map((p, i) => {
          const x = i * xStep;
          const y = height - ((p.agreementScore - minS) / range) * (height - 10) - 5;
          const color = p.agreementScore >= 60 ? "#10b981" : p.agreementScore >= 40 ? "#f59e0b" : "#ef4444";
          return <circle key={i} cx={x} cy={y} r={3} fill={color} />;
        })}
      </svg>
      <div className="flex items-center gap-2 mt-1">
        <span className="text-[10px]" style={{ color: "#7c3aed" }}>
          {t("stockAnalysis.evolutionDrift.agreementTrendLabel")}
        </span>
        <span className="text-[10px]" style={{ color: "var(--muted)" }}>
          {t("stockAnalysis.evolutionDrift.agreementRange")}: {minS}-{maxS}
        </span>
      </div>
    </div>
  );
}

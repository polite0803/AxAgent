// CompareTab — 多 run 指标对比

import { Alert, Button, Card, Space, Table, Tag, Typography } from "antd";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import { QuantMetricsCard } from "@/components/quant/charts/QuantMetricsCard";
import { useBacktestStore } from "@/stores/feature/quant";
import type { RunWithMetrics } from "@/types/quant";

const { Title, Text, Paragraph } = Typography;

export function CompareTab() {
  const { t } = useTranslation();
  const compare = useBacktestStore((s) => s.compare);
  const isComparing = useBacktestStore((s) => s.isComparing);
  const compareRuns = useBacktestStore((s) => s.compareRuns);
  const recentRuns = useBacktestStore((s) => s.recentRuns);
  const error = useBacktestStore((s) => s.error);

  const [selectedIds, setSelectedIds] = useState<string[]>([]);

  useEffect(() => {
    if (recentRuns.length >= 2 && selectedIds.length === 0) {
      Promise.resolve().then(() =>
        setSelectedIds(recentRuns.slice(0, Math.min(3, recentRuns.length)).map((r) => r.id))
      );
    }
  }, [recentRuns, selectedIds.length]);

  const onCompare = async () => {
    if (selectedIds.length < 2) { return; }
    try {
      await compareRuns(selectedIds);
    } catch {
      /* error captured in store */
    }
  };

  const columns = [
    { title: "Run", dataIndex: "runId", key: "runId", render: (v: string) => <Tag>{v.slice(0, 8)}</Tag> },
    { title: "策略", dataIndex: "strategyName", key: "strategyName" },
    {
      title: t("quant.metrics.totalReturn"),
      key: "totalReturn",
      render: (_: unknown, r: RunWithMetrics) => r.metrics ? `${(r.metrics.totalReturn * 100).toFixed(2)}%` : "—",
    },
    {
      title: t("quant.metrics.sharpe"),
      key: "sharpe",
      render: (_: unknown, r: RunWithMetrics) => (r.metrics ? r.metrics.sharpe.toFixed(2) : "—"),
    },
    {
      title: t("quant.metrics.maxDrawdownPct"),
      key: "maxDD",
      render: (_: unknown, r: RunWithMetrics) => r.metrics ? `${(r.metrics.maxDrawdownPct * 100).toFixed(2)}%` : "—",
    },
    {
      title: t("quant.metrics.winRate"),
      key: "winRate",
      render: (_: unknown, r: RunWithMetrics) => (r.metrics ? `${(r.metrics.winRate * 100).toFixed(1)}%` : "—"),
    },
    {
      title: t("quant.metrics.totalTrades"),
      key: "trades",
      render: (_: unknown, r: RunWithMetrics) => (r.metrics?.totalTrades ?? "—"),
    },
  ];

  return (
    <Space direction="vertical" size="large" style={{ width: "100%" }}>
      <Card title={t("quant.compare.title")} size="small">
        <Paragraph type="secondary">{t("quant.compare.selectRuns")}</Paragraph>
        <Space wrap size={6} style={{ marginBottom: 12 }}>
          {recentRuns.length === 0 && <Text type="secondary">无历史 run</Text>}
          {recentRuns.map((r) => (
            <label
              key={r.id}
              style={{
                display: "inline-flex",
                alignItems: "center",
                gap: 4,
                padding: "4px 10px",
                border: "1px solid var(--border-color, #d9d9d9)",
                borderRadius: 16,
                cursor: "pointer",
                background: selectedIds.includes(r.id) ? "#1677ff22" : "transparent",
              }}
            >
              <input
                type="checkbox"
                checked={selectedIds.includes(r.id)}
                onChange={(e) => {
                  setSelectedIds((cur) => (e.target.checked ? [...cur, r.id] : cur.filter((x) => x !== r.id)));
                }}
              />
              <span style={{ fontSize: 12 }}>{r.id.slice(0, 8)} · {r.startDate} → {r.endDate}</span>
            </label>
          ))}
        </Space>
        <Button type="primary" loading={isComparing} onClick={onCompare} disabled={selectedIds.length < 2}>
          {isComparing ? t("quant.compare.comparing") : t("quant.compare.compare")}
        </Button>
      </Card>

      {error && <Alert type="error" showIcon title={error} closable />}

      {compare && (
        <Card title={`${t("quant.compare.title")} (${compare.runs.length} runs)`} size="small">
          {Object.keys(compare.bestBy).length > 0 && (
            <Space wrap style={{ marginBottom: 12 }}>
              {Object.entries(compare.bestBy).map(([metric, runId]) => (
                <Tag color="green" key={metric}>
                  {t("quant.metrics.bestBy", { metric, runId: runId.slice(0, 8) })}
                </Tag>
              ))}
            </Space>
          )}
          <Table<RunWithMetrics>
            rowKey={(r) => r.run.id}
            dataSource={compare.runs}
            columns={columns}
            pagination={false}
            size="small"
          />
          {/* Top metrics of first run for quick glance */}
          {compare.runs[0]?.metrics && (
            <div style={{ marginTop: 16 }}>
              <Title level={5}>首条 run 指标</Title>
              <div style={{ display: "grid", gap: 12, gridTemplateColumns: "repeat(auto-fit, minmax(160px, 1fr))" }}>
                <QuantMetricsCard
                  title={t("quant.metrics.totalReturn")}
                  value={compare.runs[0].metrics!.totalReturn * 100}
                  suffix="%"
                />
                <QuantMetricsCard title={t("quant.metrics.sharpe")} value={compare.runs[0].metrics!.sharpe} />
                <QuantMetricsCard
                  title={t("quant.metrics.maxDrawdownPct")}
                  value={compare.runs[0].metrics!.maxDrawdownPct * 100}
                  suffix="%"
                />
              </div>
            </div>
          )}
        </Card>
      )}
    </Space>
  );
}

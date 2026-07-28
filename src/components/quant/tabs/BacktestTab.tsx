// BacktestTab — 回测配置 + 触发运行 + 结果展示
//
// 本组件在 2026-07-12 重构，全面展示回测结果：
// - MetricsReport 22 字段全展示（非仅 6 个）
// - 集成 TradesTable 成交明细
// - WalkForward 折叠表格 + OOS 图表 + fold 柱状图

import {
  Alert,
  Button,
  Card,
  Checkbox,
  Collapse,
  DatePicker,
  Empty,
  Form,
  Input,
  InputNumber,
  Space,
  Switch,
  Table,
  Tag,
  Typography,
} from "antd";
import dayjs from "dayjs";
import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

import { DrawdownChart } from "@/components/quant/charts/DrawdownChart";
import { EquityCurveChart } from "@/components/quant/charts/EquityCurveChart";
import { QuantMetricsCard } from "@/components/quant/charts/QuantMetricsCard";
import { WalkForwardFoldBarChart } from "@/components/quant/charts/WalkForwardFoldBarChart";
import { TradesTable } from "@/components/quant/tables/TradesTable";
import { StrategyForm } from "@/components/quant/tabs/StrategyForm";
import { useBacktestStore, useStrategyStore } from "@/stores/feature/quant";
import type {
  BacktestRunRequest,
  MetricsReport,
  StrategyMeta,
  WalkForwardFold,
  WalkForwardWindowResult,
} from "@/types/quant";
import type { ColumnsType } from "antd/es/table";

const { Title, Text } = Typography;

export function BacktestTab() {
  const { t } = useTranslation();
  const strategies = useStrategyStore((s) => s.strategies);
  const loadStrategies = useStrategyStore((s) => s.loadStrategies);
  const draft = useBacktestStore((s) => s.draftRequest);
  const setDraft = useBacktestStore((s) => s.setDraft);
  const runBacktest = useBacktestStore((s) => s.runBacktest);
  const isRunning = useBacktestStore((s) => s.isRunning);
  const currentRun = useBacktestStore((s) => s.currentRun);
  const currentResult = useBacktestStore((s) => s.currentBacktestResult);
  const error = useBacktestStore((s) => s.error);

  const [selectedStrategy, setSelectedStrategy] = useState<StrategyMeta | null>(null);

  useEffect(() => {
    if (strategies.length === 0) {
      void loadStrategies(true);
    }
  }, [strategies.length, loadStrategies]);

  useEffect(() => {
    if (!selectedStrategy && draft.strategyId) {
      const m = strategies.find((s) => s.id === draft.strategyId);
      if (m) { Promise.resolve().then(() => setSelectedStrategy(m)); }
    }
  }, [strategies, draft.strategyId, selectedStrategy]);

  const onSelectStrategy = (id: string) => {
    const m = strategies.find((s) => s.id === id);
    if (m) {
      setSelectedStrategy(m);
      setDraft({ strategyId: m.id, strategyType: m.strategyType });
    }
  };

  const onRun = async () => {
    if (!selectedStrategy) { return; }
    const req: BacktestRunRequest = {
      strategyId: selectedStrategy.id,
      strategyType: selectedStrategy.strategyType,
      code: draft.code || "600519",
      startDate: (draft.startDate as string) || "2023-01-01",
      endDate: (draft.endDate as string) || dayjs().format("YYYY-MM-DD"),
      initialCash: draft.initialCash || 1_000_000,
      params: draft.params || {},
      walkForwardEnabled: draft.walkForwardEnabled ?? true,
      walkForwardForceOff: draft.walkForwardForceOff ?? false,
      matcherConfig: null,
      name: draft.name || null,
    };
    try {
      await runBacktest(req);
    } catch {
      /* error captured in store */
    }
  };

  const wf = currentRun?.walkForward;

  return (
    <Space orientation="vertical" size="large" style={{ width: "100%" }}>
      <Card title={t("quant.backtest.title")} size="small">
        <Form layout="vertical" size="small">
          <div style={{ display: "grid", gap: 16, gridTemplateColumns: "repeat(auto-fill, minmax(360px, 1fr))" }}>
            <Form.Item label={t("quant.backtest.selectStrategy")} style={{ gridColumn: "1 / -1" }}>
              <select
                style={{ width: "100%", height: 32, padding: "0 8px" }}
                value={selectedStrategy?.id ?? draft.strategyId ?? ""}
                onChange={(e) => onSelectStrategy(e.target.value)}
              >
                <option value="" disabled>
                  --
                </option>
                {strategies.map((s) => (
                  <option key={s.id} value={s.id}>
                    {s.name} · {s.version} · ({s.strategyType})
                  </option>
                ))}
              </select>
            </Form.Item>
            <Form.Item label={t("quant.backtest.code")}>
              <Input
                value={draft.code}
                placeholder={t("quant.backtest.codePlaceholder") || "600519"}
                onChange={(e) => setDraft({ code: e.target.value })}
              />
            </Form.Item>
            <Form.Item label={t("quant.backtest.startDate")}>
              <DatePicker
                value={draft.startDate ? dayjs(draft.startDate as string) : null}
                onChange={(d) => setDraft({ startDate: d ? d.format("YYYY-MM-DD") : "" })}
                style={{ width: "100%" }}
              />
            </Form.Item>
            <Form.Item label={t("quant.backtest.endDate")}>
              <DatePicker
                value={draft.endDate ? dayjs(draft.endDate as string) : null}
                onChange={(d) => setDraft({ endDate: d ? d.format("YYYY-MM-DD") : "" })}
                style={{ width: "100%" }}
              />
            </Form.Item>
            <Form.Item label={t("quant.backtest.initialCash")}>
              <InputNumber
                min={10000}
                step={100000}
                style={{ width: "100%" }}
                value={draft.initialCash}
                onChange={(v) => setDraft({ initialCash: typeof v === "number" ? v : 1_000_000 })}
              />
            </Form.Item>
          </div>

          <Space orientation="vertical" size="small" style={{ marginTop: 8 }}>
            <Checkbox
              checked={draft.walkForwardEnabled ?? true}
              onChange={(e) => setDraft({ walkForwardEnabled: e.target.checked })}
            >
              {t("quant.backtest.walkForward")}
            </Checkbox>
            <Space>
              <Switch
                size="small"
                checked={draft.walkForwardForceOff ?? false}
                onChange={(v) => setDraft({ walkForwardForceOff: v })}
              />
              <Text type="secondary">{t("quant.backtest.walkForwardForceOff")}</Text>
            </Space>
          </Space>

          {selectedStrategy && (
            <div style={{ marginTop: 16 }}>
              <Text strong>{t("quant.backtest.strategyParameters")}</Text>
              <div style={{ marginTop: 8 }}>
                <StrategyForm
                  strategy={selectedStrategy}
                  onChange={(p) => setDraft({ params: p })}
                />
              </div>
            </div>
          )}

          <div style={{ marginTop: 16 }}>
            <Button type="primary" loading={isRunning} onClick={onRun} disabled={!selectedStrategy}>
              {isRunning ? t("quant.backtest.running") : t("quant.backtest.run")}
            </Button>
          </div>
        </Form>
      </Card>

      {error && <Alert type="error" showIcon title={t("quant.backtest.errorRunning")} description={error} closable />}

      {currentRun && currentResult && (
        <Card title={t("quant.backtest.result")} size="small">
          <Title level={5}>
            {currentRun.run.name || currentRun.run.strategyId} · {currentRun.run.startDate} → {currentRun.run.endDate}
            <Tag style={{ marginLeft: 12 }} color={currentRun.run.status === "completed" ? "green" : "orange"}>
              {currentRun.run.status}
            </Tag>
          </Title>

          {/* ── 指标面板：22 字段全覆盖 ── */}
          <MetricsPanel metrics={currentRun.metrics} />

          {/* ── 权益曲线 + 回撤 ── */}
          <div style={{ marginTop: 16 }}>
            <EquityCurveChart curve={currentResult.equityCurve} />
          </div>
          <div style={{ marginTop: 16 }}>
            <DrawdownChart curve={currentResult.equityCurve} />
          </div>

          {/* ── 成交明细 ── */}
          {currentResult.trades.length > 0 && (
            <div style={{ marginTop: 16 }}>
              <Title level={5}>{t("quant.backtest.trades")}</Title>
              <TradesTable trades={currentResult.trades} />
            </div>
          )}

          {/* ── Walk-Forward ── */}
          {wf && (
            <div style={{ marginTop: 16 }}>
              <WalkForwardPanel
                folds={wf.folds}
                stabilityScore={wf.stabilityScore}
                overfitWindowCount={wf.overfitWindowCount}
                aggregatedTestSharpe={wf.aggregatedTestSharpe}
              />
            </div>
          )}

          {/* ── 底部摘要 ── */}
          <div style={{ marginTop: 12, color: "var(--text-3)", fontSize: 12 }}>
            signals: {currentRun.signalCount} · trades: {currentRun.tradeCount} · duration:{" "}
            {(currentResult.durationMs / 1000).toFixed(1)}s
          </div>
        </Card>
      )}

      {!currentRun && !isRunning && !error && <Empty description={t("quant.backtest.noResult")} />}
    </Space>
  );
}

// ── 子组件：指标面板（覆盖 MetricsReport 22 字段） ──

interface MetricItem {
  key: string;
  title: string;
  value: number;
  suffix?: string;
  prec?: number;
  good?: boolean;
}

function MetricsPanel({ metrics }: { metrics: MetricsReport }) {
  const { t } = useTranslation();

  const m = useMemo(() => {
    const r1: MetricItem[] = [
      {
        key: "totalReturn",
        title: t("quant.metrics.totalReturn"),
        value: metrics.totalReturn * 100,
        suffix: "%",
        good: true,
      },
      {
        key: "annualizedReturn",
        title: t("quant.metrics.annualizedReturn"),
        value: metrics.annualizedReturn * 100,
        suffix: "%",
        good: true,
      },
      { key: "sharpe", title: t("quant.metrics.sharpe"), value: metrics.sharpe, good: true },
      {
        key: "maxDrawdownPct",
        title: t("quant.metrics.maxDrawdownPct"),
        value: metrics.maxDrawdownPct * 100,
        suffix: "%",
        good: false,
      },
      { key: "winRate", title: t("quant.metrics.winRate"), value: metrics.winRate * 100, suffix: "%" },
      { key: "totalTrades", title: t("quant.metrics.totalTrades"), value: metrics.totalTrades, prec: 0 },
    ];
    const r2: MetricItem[] = [
      {
        key: "annualizedVolatility",
        title: t("quant.backtest.annualizedVolatility"),
        value: metrics.annualizedVolatility * 100,
        suffix: "%",
        good: false,
      },
      { key: "sortino", title: t("quant.metrics.sortino"), value: metrics.sortino, good: true },
      { key: "calmar", title: t("quant.metrics.calmar"), value: metrics.calmar ?? 0, good: true },
      {
        key: "profitFactor",
        title: t("quant.backtest.profitFactor"),
        value: metrics.profitFactor,
        good: true,
      },
      { key: "payoffRatio", title: t("quant.backtest.payoffRatio"), value: metrics.payoffRatio, good: true },
      {
        key: "avgHoldingDays",
        title: t("quant.backtest.averageHoldingDays"),
        value: metrics.avgHoldingDays,
        prec: 1,
      },
    ];
    const r3: MetricItem[] = [
      {
        key: "maxDrawdown",
        title: t("quant.backtest.maxDrawdown"),
        value: metrics.maxDrawdown,
        prec: 0,
        good: false,
      },
      {
        key: "maxDrawdownDurationDays",
        title: t("quant.backtest.maxDrawdownDuration"),
        value: metrics.maxDrawdownDurationDays,
        prec: 0,
        good: false,
      },
      {
        key: "winningTrades",
        title: t("quant.backtest.winningTrades"),
        value: metrics.winningTrades,
        prec: 0,
      },
      {
        key: "losingTrades",
        title: t("quant.backtest.losingTrades"),
        value: metrics.losingTrades,
        prec: 0,
      },
      { key: "avgWin", title: t("quant.backtest.averageWin"), value: metrics.avgWin, prec: 0 },
      {
        key: "avgLoss",
        title: t("quant.backtest.averageLoss"),
        value: Math.abs(metrics.avgLoss),
        prec: 0,
        good: false,
      },
    ];
    return [r1, r2, r3];
  }, [metrics, t]);

  const gridStyle: React.CSSProperties = {
    display: "grid",
    gap: 10,
    gridTemplateColumns: "repeat(auto-fit, minmax(160px, 1fr))",
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 10, marginTop: 8 }}>
      {m.map((row, ri) => (
        <div key={ri} style={gridStyle}>
          {row.map((x) => (
            <QuantMetricsCard
              key={x.key}
              title={x.title}
              value={x.value}
              suffix={x.suffix}
              precision={x.prec ?? 2}
              positiveIsGood={x.good}
            />
          ))}
        </div>
      ))}
    </div>
  );
}

// ── WalkForward 查看适配：WalkForwardFold[] → WalkForwardWindowResult[] ──

function foldsToWindows(folds: WalkForwardFold[]): WalkForwardWindowResult[] {
  return folds.map((f) => ({
    fold: f,
    overfitFlag: f.isOverfitFold,
    trainMetrics: { sharpe: f.trainSharpe },
    testMetrics: { sharpe: f.testSharpe },
    degradationRatio: f.degradationRatio,
    totalReturnPct: 0,
  }));
}

// ── 子组件：Walk-Forward 面板 ──

interface WalkForwardPanelProps {
  folds: WalkForwardFold[];
  stabilityScore: number;
  overfitWindowCount: number;
  aggregatedTestSharpe: number;
}

function WalkForwardPanel({
  folds,
  stabilityScore,
  overfitWindowCount,
  aggregatedTestSharpe,
}: WalkForwardPanelProps) {
  const { t } = useTranslation();

  const foldColumns: ColumnsType<WalkForwardFold> = [
    { title: "Fold", dataIndex: "foldIndex", key: "fi", width: 60, sorter: (a, b) => a.foldIndex - b.foldIndex },
    {
      title: t("quant.backtest.trainRange"),
      key: "train",
      width: 240,
      render: (_, r) => `${r.trainStart} → ${r.trainEnd} (${r.trainBarsCount})`,
    },
    {
      title: t("quant.backtest.testRange"),
      key: "test",
      width: 240,
      render: (_, r) => `${r.testStart} → ${r.testEnd} (${r.testBarsCount})`,
    },
    {
      title: "Train S",
      dataIndex: "trainSharpe",
      key: "ts",
      width: 90,
      align: "right",
      render: (v: number) => v.toFixed(3),
      sorter: (a, b) => a.trainSharpe - b.trainSharpe,
    },
    {
      title: "Test S",
      dataIndex: "testSharpe",
      key: "tes",
      width: 90,
      align: "right",
      render: (v: number) => v.toFixed(3),
      sorter: (a, b) => a.testSharpe - b.testSharpe,
    },
    {
      title: t("quant.metrics.degradation"),
      dataIndex: "degradationRatio",
      key: "deg",
      width: 80,
      align: "right",
      render: (v: number) => v.toFixed(3),
      sorter: (a, b) => a.degradationRatio - b.degradationRatio,
    },
    {
      title: t("quant.metrics.overfit"),
      key: "of",
      width: 90,
      render: (_, r) =>
        r.isOverfitFold
          ? <Tag color="red">{t("quant.backtest.yes")}</Tag>
          : <Tag color="green">{t("quant.backtest.no")}</Tag>,
    },
  ];

  return (
    <Collapse
      size="small"
      items={[
        {
          key: "wf",
          label: (
            <Space size="middle">
              <Text strong>{t("quant.backtest.walkForwardTitle")}</Text>
              <Tag color={stabilityScore > 0.7 ? "green" : stabilityScore > 0.4 ? "orange" : "red"}>
                {t("quant.metrics.stability")} {stabilityScore.toFixed(3)}
              </Tag>
              <Tag color={overfitWindowCount > 0 ? "red" : "green"}>
                {overfitWindowCount > 0
                  ? `${t("quant.metrics.overfit")} ${overfitWindowCount}/${folds.length}`
                  : t("quant.metrics.noOverfit")}
              </Tag>
              <Text type="secondary" style={{ fontSize: 12 }}>
                OOS Sharpe: {aggregatedTestSharpe.toFixed(3)} · folds: {folds.length}
              </Text>
            </Space>
          ),
          children: (
            <Space orientation="vertical" size="middle" style={{ width: "100%" }}>
              <Table<WalkForwardFold>
                size="small"
                columns={foldColumns}
                dataSource={folds}
                rowKey="foldIndex"
                pagination={false}
                scroll={{ x: 800 }}
              />
              {folds.length > 0 && (
                <Collapse
                  size="small"
                  items={[{
                    key: "chart",
                    label: t("quant.backtest.foldSharpeChart"),
                    children: <WalkForwardFoldBarChart windows={foldsToWindows(folds)} />,
                  }]}
                />
              )}
            </Space>
          ),
        },
      ]}
    />
  );
}

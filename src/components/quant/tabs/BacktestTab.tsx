// BacktestTab — 回测配置 + 触发运行 + 结果展示

import {
  Alert,
  Button,
  Card,
  Checkbox,
  DatePicker,
  Empty,
  Form,
  Input,
  InputNumber,
  Space,
  Switch,
  Tag,
  Typography,
} from "antd";
import dayjs from "dayjs";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import { DrawdownChart } from "@/components/quant/charts/DrawdownChart";
import { EquityCurveChart } from "@/components/quant/charts/EquityCurveChart";
import { QuantMetricsCard } from "@/components/quant/charts/QuantMetricsCard";
import { StrategyForm } from "@/components/quant/tabs/StrategyForm";
import { useBacktestStore, useStrategyStore } from "@/stores/feature/quant";
import type { BacktestRunRequest, StrategyMeta } from "@/types";

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
  const overfitCount = wf?.overfitWindowCount ?? 0;

  return (
    <Space direction="vertical" size="large" style={{ width: "100%" }}>
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

          <Space direction="vertical" size="small" style={{ marginTop: 8 }}>
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
              <Text strong>策略参数</Text>
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
          <div style={{ display: "grid", gap: 12, gridTemplateColumns: "repeat(auto-fit, minmax(240px, 1fr))" }}>
            <QuantMetricsCard
              title={t("quant.metrics.totalReturn")}
              value={currentRun.metrics.totalReturn * 100}
              suffix="%"
              positiveIsGood
            />
            <QuantMetricsCard
              title={t("quant.metrics.annualizedReturn")}
              value={currentRun.metrics.annualizedReturn * 100}
              suffix="%"
              positiveIsGood
            />
            <QuantMetricsCard
              title={t("quant.metrics.sharpe")}
              value={currentRun.metrics.sharpe}
              positiveIsGood
            />
            <QuantMetricsCard
              title={t("quant.metrics.maxDrawdownPct")}
              value={currentRun.metrics.maxDrawdownPct * 100}
              suffix="%"
              positiveIsGood={false}
            />
            <QuantMetricsCard title={t("quant.metrics.winRate")} value={currentRun.metrics.winRate * 100} suffix="%" />
            <QuantMetricsCard
              title={t("quant.metrics.totalTrades")}
              value={currentRun.metrics.totalTrades}
              precision={0}
            />
          </div>

          <div style={{ marginTop: 16 }}>
            <EquityCurveChart curve={currentResult.equityCurve} />
          </div>
          <div style={{ marginTop: 16 }}>
            <DrawdownChart curve={currentResult.equityCurve} />
          </div>

          {wf && (
            <Alert
              style={{ marginTop: 16 }}
              type={overfitCount > 0 ? "warning" : "info"}
              showIcon
              title={t("quant.backtest.walkForwardTitle")}
              description={
                <Space direction="vertical" size={4}>
                  <Text>
                    {t("quant.backtest.stabilityScore")}: <b>{wf.stabilityScore.toFixed(3)}</b>
                  </Text>
                  <Text>
                    folds: <b>{wf.folds.length}</b>
                  </Text>
                  {overfitCount > 0 && (
                    <Text type="warning">
                      {t("quant.backtest.overfitWarning", { count: overfitCount })}
                    </Text>
                  )}
                </Space>
              }
            />
          )}

          <div style={{ marginTop: 12, color: "var(--text-3)", fontSize: 12 }}>
            signals: {currentRun.signalCount} · trades: {currentRun.tradeCount}
          </div>
        </Card>
      )}

      {!currentRun && !isRunning && !error && <Empty description={t("quant.backtest.noResult")} />}
    </Space>
  );
}

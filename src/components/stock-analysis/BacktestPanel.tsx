import { ReplayBadge, ReplayWatermark } from "@/components/time-travel/ReplayBadge";
import { invoke } from "@/lib/invoke";
import { useTimeAnchorStore } from "@/stores/feature/timeAnchorStore";
import type { BacktestComparisonResponse, BacktestStats, GroupBacktestResult } from "@/types/stock-analysis";
import { Alert, Button, Card, Empty, InputNumber, Segmented, Spin, Statistic, Table, Tag, Tooltip } from "antd";
import { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";

type BacktestScope = "all" | "live" | "replay";

export function BacktestPanel() {
  const { t } = useTranslation();
  const asOfDate = useTimeAnchorStore((s) => s.asOfDate);
  const anchorMode = useTimeAnchorStore((s) => s.mode);
  const [scope, setScope] = useState<BacktestScope>("all");
  const [stats, setStats] = useState<BacktestStats | null>(null);
  const [loading, setLoading] = useState(false);
  const [fetchError, setFetchError] = useState(false);
  // Bug 5 修复: 持有天数改为可配置(原硬编码 5)
  const [holdingDays, setHoldingDays] = useState<number>(5);
  const isReplay = anchorMode === "replay" && asOfDate !== null;

  // ── 策略回测状态 ──
  const [strategyResult, setStrategyResult] = useState<BacktestComparisonResponse | null>(null);
  const [strategyLoading, setStrategyLoading] = useState(false);
  const [strategyError, setStrategyError] = useState<string | null>(null);

  // Bug 4 修复: 统一请求级取消令牌
  const reqTokenRef = useRef(0);
  // R2-Bug-I 修复: 策略回测独立 token,避免和 load 互相取消
  const strategyTokenRef = useRef(0);

  /**
   * 统一加载入口(useEffect 与"重试"按钮共用)。
   * 旧的 `load` 没有任何取消机制,与 useEffect 的 `cancelled` 各管各的,
   * 快速连点"重试"会拿到乱序 stats。
   */
  const load = useCallback(async () => {
    const myToken = ++reqTokenRef.current;
    setLoading(true);
    setFetchError(false);
    try {
      const result = await invoke<BacktestStats>("backtest_all_history", {
        holdingDays,
        scope,
      });
      if (myToken !== reqTokenRef.current) { return; }
      setStats(result);
    } catch {
      if (myToken !== reqTokenRef.current) { return; }
      setFetchError(true);
    } finally {
      if (myToken === reqTokenRef.current) { setLoading(false); }
    }
  }, [holdingDays, scope]);

  useEffect(() => {
    // Bug 4 修复: 走统一 load 入口
    Promise.resolve().then(() => load());
  }, [load]);

  const runStrategyBacktest = useCallback(async () => {
    // R2-Bug-I 修复: 快速双击"策略回测"按钮会产生并发请求,
    // 此处用独立 token 让后返回的慢请求无法覆盖较新的快请求结果。
    const myToken = ++strategyTokenRef.current;
    setStrategyLoading(true);
    setStrategyResult(null);
    setStrategyError(null);
    try {
      const result = await invoke<BacktestComparisonResponse>("backtest_reco_strategies");
      if (myToken !== strategyTokenRef.current) { return; }
      setStrategyResult(result);
    } catch (e: unknown) {
      if (myToken !== strategyTokenRef.current) { return; }
      setStrategyError(
        typeof e === "string" ? e : e instanceof Error ? e.message : t("stockAnalysis.backtest.strategyFailed"),
      );
    }
    if (myToken === strategyTokenRef.current) { setStrategyLoading(false); }
  }, [t]);

  const accuracyColor = stats ? stats.accuracyPct >= 60 ? "var(--sa-red)" : "var(--sa-green)" : undefined;
  const returnColor = stats && stats.avgReturnPct >= 0 ? "var(--sa-red)" : "var(--sa-green)";

  return (
    <Card
      size="small"
      title={
        <div className="flex items-center gap-2">
          <span>📊 {t("stockAnalysis.backtest.sectionTitle")}</span>
          {isReplay && <ReplayBadge />}
        </div>
      }
      styles={{ body: { padding: "8px 10px" } }}
      extra={
        <Button size="small" loading={loading} onClick={load}>
          {t("stockAnalysis.retry")}
        </Button>
      }
    >
      {isReplay && asOfDate && (
        <Alert
          type="info"
          showIcon
          className="!text-xs !mb-2"
          message={
            <span className="text-xs">
              {t("timeTravel.backtestHint", { date: asOfDate })}
            </span>
          }
        />
      )}
      <div className="mb-2 flex items-center gap-2 flex-wrap">
        <Segmented
          size="small"
          value={scope}
          onChange={(v) => setScope(v as BacktestScope)}
          options={[
            { label: t("stockAnalysis.backtest.scopeAll"), value: "all" },
            { label: t("stockAnalysis.backtest.scopeLive"), value: "live" },
            { label: t("stockAnalysis.backtest.scopeReplay"), value: "replay" },
          ]}
        />
        {/* Bug 5 修复: 持有天数可配置 —— 触发 useEffect 重拉 */}
        <span className="text-xs text-gray-500">
          {t("stockAnalysis.backtest.holdingDays") ?? "持有天数"}
        </span>
        <InputNumber
          size="small"
          min={1}
          max={120}
          step={1}
          value={holdingDays}
          onChange={(v) => v != null && setHoldingDays(v)}
          style={{ width: 80 }}
        />
      </div>
      <div style={{ position: "relative" }}>
        {loading
          ? <Spin size="small" style={{ display: "block", margin: "16px auto" }} />
          : fetchError
          ? <Empty description={t("stockAnalysis.error")} image={Empty.PRESENTED_IMAGE_SIMPLE} />
          : !stats || stats.totalAnalyses === 0
          ? <Empty description={t("stockAnalysis.noRecords")} image={Empty.PRESENTED_IMAGE_SIMPLE} />
          : (
            <div>
              <div className="grid grid-cols-2 gap-2 mb-3">
                <Statistic
                  title={t("stockAnalysis.backtest.total")}
                  value={stats.totalAnalyses}
                  valueStyle={{ fontSize: 18 }}
                />
                <Statistic
                  title={t("stockAnalysis.backtest.accuracy")}
                  value={stats.accuracyPct.toFixed(1)}
                  suffix="%"
                  valueStyle={{ fontSize: 18, color: accuracyColor, fontWeight: "bold" }}
                />
                <Statistic
                  title={t("stockAnalysis.backtest.avgReturn")}
                  value={stats.avgReturnPct.toFixed(2)}
                  suffix="%"
                  valueStyle={{ fontSize: 18, color: returnColor, fontWeight: "bold" }}
                />
                <Statistic
                  title={t("stockAnalysis.backtest.maxDrawdown")}
                  value={stats.avgMaxDrawdownPct.toFixed(2)}
                  suffix="%"
                  valueStyle={{ fontSize: 18, color: "var(--sa-green)" }}
                />
              </div>
              <div className="grid grid-cols-2 gap-2 text-xs text-gray-500">
                <span>{t("stockAnalysis.backtest.avgConfidence")}: {(stats.avgConfidence * 100).toFixed(0)}%</span>
                {stats.alphaPct != null && (
                  <span>{t("stockAnalysis.backtest.alpha")}: {stats.alphaPct.toFixed(2)}%</span>
                )}
              </div>
            </div>
          )}
        {isReplay && <ReplayWatermark />}
      </div>

      {/* ── 荐股策略回测（两组对比） ── */}
      <div className="mt-4 pt-3 border-t border-gray-700/30">
        <div className="flex items-center justify-between mb-2">
          <span className="text-sm font-medium">📈 {t("stockAnalysis.backtest.strategyTitle")}</span>
          <Button
            size="small"
            type="primary"
            ghost
            loading={strategyLoading}
            onClick={runStrategyBacktest}
          >
            {strategyLoading ? t("stockAnalysis.backtest.strategyRunning") : t("stockAnalysis.backtest.strategyRun")}
          </Button>
        </div>

        {strategyError && (
          <Alert
            type="warning"
            showIcon
            className="!text-xs !mb-2"
            message={strategyError}
          />
        )}

        {strategyResult && (
          <>
            <div className="text-xs text-gray-500 mb-2">
              {t("stockAnalysis.backtest.strategyPositive", { count: strategyResult.positive.stockCount })} |{" "}
              {t("stockAnalysis.backtest.strategyNegative", { count: strategyResult.negative.stockCount })}
              {strategyResult.skipped.length > 0 && (
                <Tooltip title={strategyResult.skipped.join("\n")}>
                  <Tag className="ml-1 cursor-help" color="warning" style={{ fontSize: 10 }}>
                    {t("stockAnalysis.backtest.strategySkipped", { count: strategyResult.skipped.length })}
                  </Tag>
                </Tooltip>
              )}
            </div>

            {/* 正向组 */}
            <div className="mb-3">
              <div className="text-xs font-medium text-green-500 mb-1">
                ✅ {strategyResult.positive.label}
              </div>
              <StrategyTable data={strategyResult.positive} />
            </div>

            {/* 负向组 */}
            <div>
              <div className="text-xs font-medium text-orange-500 mb-1">
                ❌ {strategyResult.negative.label}
              </div>
              <StrategyTable data={strategyResult.negative} />
            </div>
          </>
        )}
      </div>
    </Card>
  );
}

/** 单组策略回测表格 */
function StrategyTable({ data }: { data: GroupBacktestResult }) {
  const { t } = useTranslation();
  const entries = Object.entries(data.strategies);

  const columns = [
    {
      title: t("stockAnalysis.backtest.colStrategy"),
      dataIndex: "strategyId",
      key: "strategyId",
      width: 100,
      render: (v: string) => {
        const nameKey = `stockAnalysis.strategyNames.${v}` as const;
        return <span className="text-xs font-medium">{t(nameKey)}</span>;
      },
    },
    {
      title: t("stockAnalysis.backtest.colSignalCount"),
      dataIndex: "totalSignals",
      key: "totalSignals",
      width: 60,
      render: (v: number) => <span className="text-xs">{v}</span>,
    },
    {
      title: t("stockAnalysis.backtest.colWinRate"),
      dataIndex: "winRatePct",
      key: "winRatePct",
      width: 70,
      render: (v: number) => {
        const color = v >= 50 ? "var(--sa-red)" : "var(--sa-green)";
        return <span className="text-xs font-bold" style={{ color }}>{v.toFixed(1)}%</span>;
      },
    },
    {
      title: t("stockAnalysis.backtest.colAvgReturn"),
      dataIndex: "avgReturnPct",
      key: "avgReturnPct",
      width: 80,
      render: (v: number) => {
        const color = v >= 0 ? "var(--sa-red)" : "var(--sa-green)";
        return <span className="text-xs" style={{ color }}>{v.toFixed(2)}%</span>;
      },
    },
    {
      title: t("stockAnalysis.backtest.colTotalReturn"),
      dataIndex: "totalReturnPct",
      key: "totalReturnPct",
      width: 80,
      render: (v: number) => {
        const color = v >= 0 ? "var(--sa-red)" : "var(--sa-green)";
        return <span className="text-xs" style={{ color }}>{v.toFixed(1)}%</span>;
      },
    },
    {
      title: t("stockAnalysis.backtest.colMaxDrawdown"),
      dataIndex: "avgMaxDrawdownPct",
      key: "avgMaxDrawdownPct",
      width: 80,
      render: (v: number) => <span className="text-xs" style={{ color: "var(--sa-green)" }}>{v.toFixed(1)}%</span>,
    },
    {
      title: t("stockAnalysis.backtest.colMaxLossStreak"),
      dataIndex: "maxConsecutiveLosses",
      key: "maxConsecutiveLosses",
      width: 50,
      render: (v: number) => <span className={`text-xs ${v >= 5 ? "text-red-500" : ""}`}>{v}</span>,
    },
    {
      title: "Sharpe",
      dataIndex: "sharpeRatio",
      key: "sharpeRatio",
      width: 65,
      render: (v: number | null) => {
        if (v == null) { return <span className="text-xs text-gray-500">—</span>; }
        const color = v >= 1 ? "#52c41a" : v >= 0 ? "#faad14" : "#ff4d4f";
        return <span className="text-xs font-medium" style={{ color }}>{v.toFixed(2)}</span>;
      },
    },
    {
      title: t("stockAnalysis.backtest.colProfitFactor"),
      dataIndex: "profitFactor",
      key: "profitFactor",
      width: 65,
      render: (v: number | null) => {
        if (v == null) { return <span className="text-xs text-gray-500">—</span>; }
        const color = v >= 2 ? "#52c41a" : v >= 1 ? "#faad14" : "#ff4d4f";
        return <span className="text-xs font-medium" style={{ color }}>{v.toFixed(2)}</span>;
      },
    },
  ];

  const dataSource = entries.map(([key, val]) => ({ key, ...val }));

  return (
    <Table
      dataSource={dataSource}
      columns={columns}
      pagination={false}
      size="small"
      bordered={false}
      className="strategy-backtest-table"
    />
  );
}

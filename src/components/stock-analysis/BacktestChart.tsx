/**
 * 回测结果可视化图表组件
 *
 * 展示：
 * - 权益曲线（Equity Curve）
 * - 交易标记（买入/卖出）
 * - 回撤曲线
 * - 绩效指标摘要
 *
 * 接收数据来自 BacktestRunResponse 或 BacktestPage 的回测结果
 */

import * as echarts from "echarts";
import { useEffect, useMemo, useRef } from "react";
import { useTranslation } from "react-i18next";

/** 权益曲线数据点（对齐后端 EquityPoint 的 camelCase） */
export interface EquityPointData {
  date: string;
  equity: number;
  cash: number;
  positionValue: number;
}

/** 交易标记 */
export interface TradeMarker {
  date: string;
  type: "buy" | "sell";
  price: number;
  reason?: string;
}

/** 回测指标摘要 */
export interface MetricsSummary {
  strategyName: string;
  totalReturn: number;
  annualizedReturn: number;
  sharpe: number;
  maxDrawdown: number;
  winRate: number;
  totalTrades: number;
}

/** 组件 Props */
export interface BacktestChartProps {
  /** 权益曲线数据 */
  equityCurve: EquityPointData[];
  /** 交易标记（可选） */
  trades?: TradeMarker[];
  /** 基准对比线（可选，如沪深 300） */
  benchmarkLine?: { date: string; value: number }[];
  /** 绩效指标摘要（可选，显示在顶部） */
  metrics?: MetricsSummary;
  /** 高度（px，默认 400） */
  height?: number;
  /** 宽度百分比（默认 "100%"） */
  width?: string | number;
}

/** 通用 echarts hook（回测专用，dark 主题） */
function useBacktestECharts(
  optionBuilder: () => echarts.EChartsOption,
  deps: unknown[],
): React.RefObject<HTMLDivElement | null> {
  const ref = useRef<HTMLDivElement>(null);
  const instance = useRef<echarts.ECharts | null>(null);

  useEffect(() => {
    if (!ref.current) { return; }
    if (!instance.current) {
      instance.current = echarts.init(ref.current, "dark");
    }
    instance.current.setOption(optionBuilder());
    const handleResize = () => instance.current?.resize();
    window.addEventListener("resize", handleResize);
    return () => window.removeEventListener("resize", handleResize);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, deps);

  useEffect(() => {
    return () => {
      instance.current?.dispose();
      instance.current = null;
    };
  }, []);

  return ref;
}

/**
 * 回测权益曲线 + 交易标记组合图
 */
export function BacktestChart({
  equityCurve,
  trades,
  benchmarkLine,
  metrics,
  height = 400,
  width = "100%",
}: BacktestChartProps) {
  const { t } = useTranslation();

  // 回撤数据（独立图表）
  const drawdownData = useMemo(() => {
    if (equityCurve.length < 2) { return []; }
    let peak = equityCurve[0].equity;
    return equityCurve.map((ep) => {
      if (ep.equity > peak) { peak = ep.equity; }
      const dd = peak > 0 ? ((peak - ep.equity) / peak) * 100 : 0;
      return { date: ep.date, drawdown: -Math.abs(dd) };
    });
  }, [equityCurve]);

  // 格式化金额
  const fmtMoney = (v: number) => {
    if (Math.abs(v) >= 1_0000_0000) {
      return `¥${(v / 1_0000_0000).toFixed(2)}${t("stockAnalysis.backtest.chart.moneyYi")}`;
    }
    if (Math.abs(v) >= 1_0000) {
      return `¥${(v / 1_0000).toFixed(2)}${t("stockAnalysis.backtest.chart.moneyWan")}`;
    }
    return `¥${v.toFixed(2)}`;
  };

  // 权益曲线 echarts option
  const equityRef = useBacktestECharts(
    () => {
      const dates = equityCurve.map((d) => d.date);
      const hasBenchmark = benchmarkLine && benchmarkLine.length > 0;

      const series: echarts.SeriesOption[] = [
        {
          name: "equity",
          type: "line",
          data: equityCurve.map((d) => d.equity),
          smooth: true,
          showSymbol: false,
          lineStyle: { color: "#22c55e", width: 2 },
          areaStyle: {
            color: new echarts.graphic.LinearGradient(0, 0, 0, 1, [
              { offset: 0, color: "rgba(34, 197, 94, 0.25)" },
              { offset: 1, color: "rgba(34, 197, 94, 0.02)" },
            ]),
          },
        },
        {
          name: "cash",
          type: "line",
          data: equityCurve.map((d) => d.cash),
          smooth: true,
          showSymbol: false,
          lineStyle: { color: "#6B7280", width: 1, type: "dashed" },
        },
        {
          name: "positionValue",
          type: "line",
          data: equityCurve.map((d) => d.positionValue),
          smooth: true,
          showSymbol: false,
          lineStyle: { color: "#F59E0B", width: 1, type: "dashed" },
        },
      ];

      if (hasBenchmark) {
        series.push({
          name: "benchmark",
          type: "line",
          data: benchmarkLine!.map((d) => d.value),
          smooth: true,
          showSymbol: false,
          lineStyle: { color: "#3B82F6", width: 1 },
        });
      }

      // 交易标记
      const buyTrades: number[][] = [];
      const sellTrades: number[][] = [];
      for (const tr of trades ?? []) {
        const idx = dates.indexOf(tr.date);
        if (idx < 0) { continue; }
        const point: number[] = [idx, equityCurve[idx].equity];
        if (tr.type === "buy") { buyTrades.push(point); }
        else { sellTrades.push(point); }
      }

      if (buyTrades.length > 0) {
        series.push({
          name: "buy_marker",
          type: "scatter",
          data: buyTrades,
          symbol: "triangle",
          symbolSize: 10,
          itemStyle: { color: "#ef4444" },
          tooltip: { show: true, formatter: () => t("stockAnalysis.backtest.chart.buy") },
        });
      }
      if (sellTrades.length > 0) {
        series.push({
          name: "sell_marker",
          type: "scatter",
          data: sellTrades,
          symbol: "triangle",
          symbolRotate: 180,
          symbolSize: 10,
          itemStyle: { color: "#22c55e" },
          tooltip: { show: true, formatter: () => t("stockAnalysis.backtest.chart.sell") },
        });
      }

      const labels: Record<string, string> = {
        equity: t("stockAnalysis.backtest.chart.equity"),
        cash: t("stockAnalysis.backtest.chart.cash"),
        positionValue: t("stockAnalysis.backtest.chart.positionValue"),
        benchmark: t("stockAnalysis.backtest.chart.benchmark"),
      };

      return {
        tooltip: {
          trigger: "axis",
          backgroundColor: "#1F2937",
          borderColor: "#374151",
          textStyle: { color: "#F3F4F6", fontSize: 12 },
          formatter: (params: unknown) => {
            const ps = params as { seriesName: string; value: number; axisValue: string }[];
            let result = ps[0]?.axisValue ?? "";
            for (const p of ps) {
              const label = labels[p.seriesName] ?? p.seriesName;
              if (label === "buy_marker" || label === "sell_marker") { continue; }
              result += `<br/>${label}: ${fmtMoney(p.value)}`;
            }
            return result;
          },
        },
        legend: {
          textStyle: { color: "#D1D5DB", fontSize: 12 },
          data: ["equity", "cash", "positionValue", ...(hasBenchmark ? ["benchmark"] : [])],
        },
        grid: { top: 36, right: 16, bottom: 24, left: 72 },
        xAxis: {
          type: "category",
          data: dates,
          axisLine: { lineStyle: { color: "#4B5563" } },
          axisLabel: { color: "#9CA3AF", fontSize: 10 },
          axisTick: { show: false },
          boundaryGap: false,
        },
        yAxis: {
          type: "value",
          axisLine: { lineStyle: { color: "#4B5563" } },
          axisLabel: {
            color: "#9CA3AF",
            fontSize: 10,
            formatter: (v: number) => fmtMoney(v),
          },
          splitLine: { lineStyle: { color: "#374151", type: "dashed" } },
          axisTick: { show: false },
        },
        series,
      };
    },
    [equityCurve, benchmarkLine, trades, t],
  );

  // 回撤曲线 echarts option
  const drawdownRef = useBacktestECharts(
    () => ({
      tooltip: {
        trigger: "axis",
        backgroundColor: "#1F2937",
        borderColor: "#374151",
        textStyle: { color: "#F3F4F6", fontSize: 12 },
        formatter: (params: unknown) => {
          const ps = params as { value: number }[];
          return `${ps[0]?.value.toFixed(2)}%`;
        },
      },
      grid: { top: 16, right: 16, bottom: 24, left: 56 },
      xAxis: {
        type: "category",
        data: drawdownData.map((d) => d.date),
        axisLine: { lineStyle: { color: "#4B5563" } },
        axisLabel: { color: "#9CA3AF", fontSize: 10 },
        axisTick: { show: false },
        boundaryGap: false,
      },
      yAxis: {
        type: "value",
        min: -100,
        max: 5,
        axisLine: { lineStyle: { color: "#4B5563" } },
        axisLabel: {
          color: "#9CA3AF",
          fontSize: 10,
          formatter: (v: number) => `${v.toFixed(1)}%`,
        },
        splitLine: { lineStyle: { color: "#374151", type: "dashed" } },
        axisTick: { show: false },
      },
      series: [
        {
          type: "line",
          data: drawdownData.map((d) => d.drawdown),
          smooth: true,
          showSymbol: false,
          lineStyle: { color: "#EF4444", width: 1 },
          areaStyle: {
            color: new echarts.graphic.LinearGradient(0, 0, 0, 1, [
              { offset: 0, color: "rgba(239, 68, 68, 0.25)" },
              { offset: 1, color: "rgba(239, 68, 68, 0.02)" },
            ]),
          },
        },
      ],
    }),
    [drawdownData],
  );

  return (
    <div className="space-y-4">
      {/* 绩效指标摘要 */}
      {metrics && (
        <div className="grid grid-cols-2 sm:grid-cols-4 gap-3 p-3 rounded-lg bg-gray-800/60">
          <MetricItem
            label={t("stockAnalysis.backtest.metrics.totalReturn")}
            value={`${(metrics.totalReturn * 100).toFixed(2)}%`}
            color={metrics.totalReturn >= 0 ? "#22c55e" : "#ef4444"}
          />
          <MetricItem
            label={t("stockAnalysis.backtest.metrics.sharpe")}
            value={metrics.sharpe.toFixed(3)}
            color={metrics.sharpe >= 1 ? "#22c55e" : metrics.sharpe >= 0 ? "#eab308" : "#ef4444"}
          />
          <MetricItem
            label={t("stockAnalysis.backtest.metrics.maxDrawdown")}
            value={`${(metrics.maxDrawdown * 100).toFixed(2)}%`}
            color="#ef4444"
          />
          <MetricItem
            label={t("stockAnalysis.backtest.metrics.winRate")}
            value={`${(metrics.winRate * 100).toFixed(1)}%`}
            color={metrics.winRate >= 0.5 ? "#22c55e" : "#eab308"}
          />
        </div>
      )}

      {/* 权益曲线 + 交易标记合成图 */}
      <div className="bg-gray-900/50 rounded-lg p-3">
        <h4 className="text-sm font-medium text-gray-300 mb-2">
          {t("stockAnalysis.backtest.chart.equityCurve")}
          {metrics && (
            <span className="text-xs text-gray-500 ml-2">
              {metrics.strategyName}
            </span>
          )}
        </h4>
        <div ref={equityRef} style={{ width, height: height * 0.65 }} />
      </div>

      {/* 回撤曲线 */}
      <div className="bg-gray-900/50 rounded-lg p-3">
        <h4 className="text-sm font-medium text-gray-300 mb-2">
          {t("stockAnalysis.backtest.chart.drawdown")}
        </h4>
        <div ref={drawdownRef} style={{ width, height: height * 0.35 }} />
      </div>

      {/* 交易标记表（如有） */}
      {trades && trades.length > 0 && (
        <div className="bg-gray-900/50 rounded-lg p-3">
          <h4 className="text-sm font-medium text-gray-300 mb-2">
            {t("stockAnalysis.backtest.chart.trades")}
            <span className="text-xs text-gray-500 ml-2">
              ({t("stockAnalysis.backtest.chart.tradeCount", { count: trades.length })})
            </span>
          </h4>
          <div className="max-h-48 overflow-y-auto text-xs">
            <table className="w-full">
              <thead>
                <tr className="text-gray-400 border-b border-gray-700">
                  <th className="text-left py-1 px-2">
                    {t("stockAnalysis.backtest.chart.tradeDate")}
                  </th>
                  <th className="text-left py-1 px-2">
                    {t("stockAnalysis.backtest.chart.tradeType")}
                  </th>
                  <th className="text-right py-1 px-2">
                    {t("stockAnalysis.backtest.chart.tradePrice")}
                  </th>
                  <th className="text-left py-1 px-2">
                    {t("stockAnalysis.backtest.chart.tradeReason")}
                  </th>
                </tr>
              </thead>
              <tbody>
                {trades.map((trade, i) => (
                  <tr key={i} className="border-b border-gray-800 hover:bg-gray-800/40">
                    <td className="py-1 px-2 text-gray-300">{trade.date}</td>
                    <td className="py-1 px-2">
                      <span
                        className={`px-1.5 py-0.5 rounded text-xs font-medium ${
                          trade.type === "buy"
                            ? "bg-red-900/40 text-red-400"
                            : "bg-green-900/40 text-green-400"
                        }`}
                      >
                        {trade.type === "buy"
                          ? t("stockAnalysis.backtest.chart.buy")
                          : t("stockAnalysis.backtest.chart.sell")}
                      </span>
                    </td>
                    <td className="py-1 px-2 text-right text-gray-300">
                      {fmtMoney(trade.price)}
                    </td>
                    <td className="py-1 px-2 text-gray-400 truncate max-w-[200px]">
                      {trade.reason ?? "-"}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      )}
    </div>
  );
}

/** 单个指标展示小卡片 */
function MetricItem({
  label,
  value,
  color,
}: {
  label: string;
  value: string;
  color: string;
}) {
  return (
    <div className="text-center">
      <div className="text-xs text-gray-400 mb-0.5">{label}</div>
      <div className="text-lg font-semibold tabular-nums" style={{ color }}>
        {value}
      </div>
    </div>
  );
}

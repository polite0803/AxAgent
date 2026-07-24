/**
 * 回测结果可视化图表组件
 *
 * 展示：
 * - 权益曲线（Equity Curve）
 * - 交易标记（买入/卖出）
 * - 回撤曲线
 * - 绩效指标摘要
 * - 图表拖动缩放
 *
 * 接收数据来自 BacktestRunResponse 或 BacktestPage 的回测结果
 */

import { useMemo } from "react";
import { useTranslation } from "react-i18next";

import {
  Area,
  AreaChart,
  CartesianGrid,
  ComposedChart,
  Legend,
  Line,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from "recharts";

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
    const peakValues: number[] = [];
    let peak = equityCurve[0].equity;
    return equityCurve.map((ep) => {
      if (ep.equity > peak) { peak = ep.equity; }
      const dd = peak > 0 ? ((peak - ep.equity) / peak) * 100 : 0;
      peakValues.push(peak);
      return { date: ep.date, drawdown: -Math.abs(dd) };
    });
  }, [equityCurve]);

  // 格式化金额
  const fmtMoney = (v: number) => {
    if (Math.abs(v) >= 1_0000_0000) { return `¥${(v / 1_0000_0000).toFixed(2)}亿`; }
    if (Math.abs(v) >= 1_0000) { return `¥${(v / 1_0000).toFixed(2)}万`; }
    return `¥${v.toFixed(2)}`;
  };

  return (
    <div className="space-y-4">
      {/* 绩效指标摘要 */}
      {metrics && (
        <div className="grid grid-cols-2 sm:grid-cols-4 gap-3 p-3 rounded-lg bg-gray-800/60">
          <MetricItem
            label={t("stockAnalysis.backtest.metrics.totalReturn", "总收益")}
            value={`${(metrics.totalReturn * 100).toFixed(2)}%`}
            color={metrics.totalReturn >= 0 ? "#22c55e" : "#ef4444"}
          />
          <MetricItem
            label={t("stockAnalysis.backtest.metrics.sharpe", "夏普比率")}
            value={metrics.sharpe.toFixed(3)}
            color={metrics.sharpe >= 1 ? "#22c55e" : metrics.sharpe >= 0 ? "#eab308" : "#ef4444"}
          />
          <MetricItem
            label={t("stockAnalysis.backtest.metrics.maxDrawdown", "最大回撤")}
            value={`${(metrics.maxDrawdown * 100).toFixed(2)}%`}
            color="#ef4444"
          />
          <MetricItem
            label={t("stockAnalysis.backtest.metrics.winRate", "胜率")}
            value={`${(metrics.winRate * 100).toFixed(1)}%`}
            color={metrics.winRate >= 0.5 ? "#22c55e" : "#eab308"}
          />
        </div>
      )}

      {/* 权益曲线 + 交易标记合成图 */}
      <div className="bg-gray-900/50 rounded-lg p-3">
        <h4 className="text-sm font-medium text-gray-300 mb-2">
          {t("stockAnalysis.backtest.chart.equityCurve", "权益曲线")}
          {metrics && (
            <span className="text-xs text-gray-500 ml-2">
              {metrics.strategyName}
            </span>
          )}
        </h4>
        <ResponsiveContainer width={width} height={height * 0.65}>
          <ComposedChart data={equityCurve}>
            <CartesianGrid strokeDasharray="3 3" stroke="#374151" />
            <XAxis
              dataKey="date"
              tick={{ fill: "#9CA3AF", fontSize: 10 }}
              tickLine={false}
              axisLine={{ stroke: "#4B5563" }}
              interval="preserveStartEnd"
            />
            <YAxis
              domain={["auto", "auto"]}
              tick={{ fill: "#9CA3AF", fontSize: 10 }}
              tickLine={false}
              axisLine={{ stroke: "#4B5563" }}
              tickFormatter={(v: number) => fmtMoney(v)}
            />
            <Tooltip
              contentStyle={{
                backgroundColor: "#1F2937",
                border: "1px solid #374151",
                borderRadius: "8px",
                color: "#F3F4F6",
              }}
              // eslint-disable-next-line @typescript-eslint/no-explicit-any
              formatter={(value: any, name: string) => {
                const labels: Record<string, string> = {
                  equity: t("stockAnalysis.backtest.chart.equity", "净资产"),
                  cash: t("stockAnalysis.backtest.chart.cash", "现金"),
                  positionValue: t("stockAnalysis.backtest.chart.positionValue", "持仓市值"),
                  benchmark: t("stockAnalysis.backtest.chart.benchmark", "基准"),
                };
                return [fmtMoney(value), labels[name] ?? name];
              }}
            />
            <Legend
              wrapperStyle={{ color: "#D1D5DB", fontSize: 12 }}
            />
            {/* 权益曲线面积 */}
            <Area
              type="monotone"
              dataKey="equity"
              stroke="#22c55e"
              fill="#22c55e"
              fillOpacity={0.1}
              strokeWidth={2}
              dot={false}
              name="equity"
            />
            {/* 现金线 */}
            <Line
              type="monotone"
              dataKey="cash"
              stroke="#6B7280"
              strokeWidth={1}
              strokeDasharray="4 4"
              dot={false}
              name="cash"
            />
            {/* 持仓市值线 */}
            <Line
              type="monotone"
              dataKey="positionValue"
              stroke="#F59E0B"
              strokeWidth={1}
              strokeDasharray="2 2"
              dot={false}
              name="positionValue"
            />
            {/* 基准线 */}
            {benchmarkLine && (
              <Line
                type="monotone"
                dataKey="value"
                stroke="#3B82F6"
                strokeWidth={1}
                dot={false}
                name="benchmark"
              />
            )}
          </ComposedChart>
        </ResponsiveContainer>
      </div>

      {/* 回撤曲线 */}
      <div className="bg-gray-900/50 rounded-lg p-3">
        <h4 className="text-sm font-medium text-gray-300 mb-2">
          {t("stockAnalysis.backtest.chart.drawdown", "回撤曲线")}
        </h4>
        <ResponsiveContainer width={width} height={height * 0.35}>
          <AreaChart data={drawdownData}>
            <CartesianGrid strokeDasharray="3 3" stroke="#374151" />
            <XAxis
              dataKey="date"
              tick={{ fill: "#9CA3AF", fontSize: 10 }}
              tickLine={false}
              axisLine={{ stroke: "#4B5563" }}
              interval="preserveStartEnd"
            />
            <YAxis
              domain={[-100, 5]}
              tick={{ fill: "#9CA3AF", fontSize: 10 }}
              tickLine={false}
              axisLine={{ stroke: "#4B5563" }}
              tickFormatter={(v: number) => `${v.toFixed(1)}%`}
            />
            <Tooltip
              contentStyle={{
                backgroundColor: "#1F2937",
                border: "1px solid #374151",
                borderRadius: "8px",
                color: "#F3F4F6",
              }}
              // eslint-disable-next-line @typescript-eslint/no-explicit-any
              formatter={(value: any) => [`${value.toFixed(2)}%`, t("stockAnalysis.backtest.chart.drawdown", "回撤")]}
            />
            <Area
              type="monotone"
              dataKey="drawdown"
              stroke="#EF4444"
              fill="#EF4444"
              fillOpacity={0.2}
              strokeWidth={1}
              dot={false}
            />
          </AreaChart>
        </ResponsiveContainer>
      </div>

      {/* 交易标记表（如有） */}
      {trades && trades.length > 0 && (
        <div className="bg-gray-900/50 rounded-lg p-3">
          <h4 className="text-sm font-medium text-gray-300 mb-2">
            {t("stockAnalysis.backtest.chart.trades", "交易记录")}
            <span className="text-xs text-gray-500 ml-2">({trades.length} 笔)</span>
          </h4>
          <div className="max-h-48 overflow-y-auto text-xs">
            <table className="w-full">
              <thead>
                <tr className="text-gray-400 border-b border-gray-700">
                  <th className="text-left py-1 px-2">
                    {t("stockAnalysis.backtest.chart.tradeDate", "日期")}
                  </th>
                  <th className="text-left py-1 px-2">
                    {t("stockAnalysis.backtest.chart.tradeType", "方向")}
                  </th>
                  <th className="text-right py-1 px-2">
                    {t("stockAnalysis.backtest.chart.tradePrice", "价格")}
                  </th>
                  <th className="text-left py-1 px-2">
                    {t("stockAnalysis.backtest.chart.tradeReason", "原因")}
                  </th>
                </tr>
              </thead>
              <tbody>
                {trades.map((t, i) => (
                  <tr key={i} className="border-b border-gray-800 hover:bg-gray-800/40">
                    <td className="py-1 px-2 text-gray-300">{t.date}</td>
                    <td className="py-1 px-2">
                      <span
                        className={`px-1.5 py-0.5 rounded text-xs font-medium ${
                          t.type === "buy"
                            ? "bg-red-900/40 text-red-400"
                            : "bg-green-900/40 text-green-400"
                        }`}
                      >
                        {t.type === "buy" ? "买入" : "卖出"}
                      </span>
                    </td>
                    <td className="py-1 px-2 text-right text-gray-300">
                      {fmtMoney(t.price)}
                    </td>
                    <td className="py-1 px-2 text-gray-400 truncate max-w-[200px]">
                      {t.reason ?? "-"}
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

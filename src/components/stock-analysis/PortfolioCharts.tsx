/**
 * 组合可视化图表集
 *
 * 包含：
 * - SectorAllocationDonut: 行业配置圆环图
 * - CorrelationHeatmap: 持仓相关性热力图
 * - PnLHistogram: 交易盈亏分布直方图
 * - PortfolioPerformanceLine: 组合净值 vs 基准对比曲线
 */

import { useMemo } from "react";
import { useTranslation } from "react-i18next";
import {
  Bar,
  BarChart,
  CartesianGrid,
  Cell,
  Legend,
  Line,
  LineChart,
  Pie,
  PieChart,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from "recharts";

/** 颜色方案（固定色盘，避免随机） */
const COLORS = [
  "#3B82F6",
  "#22C55E",
  "#EAB308",
  "#EF4444",
  "#A855F7",
  "#EC4899",
  "#14B8A6",
  "#F97316",
  "#6366F1",
  "#84CC16",
  "#06B6D4",
  "#D946EF",
  "#F43F5E",
  "#0EA5E9",
  "#8B5CF6",
];

/** 行业配置数据 */
interface SectorAllocation {
  sector: string;
  pct: number;
  value: number;
}

interface SectorAllocationDonutProps {
  data: SectorAllocation[];
  height?: number;
}

/**
 * 行业配置圆环图
 */
export function SectorAllocationDonut({ data, height = 250 }: SectorAllocationDonutProps) {
  const { t } = useTranslation();
  const sorted = useMemo(() => [...data].sort((a, b) => b.pct - a.pct), [data]);

  if (sorted.length === 0) {
    return (
      <div className="text-gray-400 text-xs text-center py-8">
        {t("stockAnalysis.charts.noSectorData", "无行业配置数据")}
      </div>
    );
  }

  return (
    <div className="bg-gray-900/50 rounded-lg p-3">
      <h4 className="text-sm font-medium text-gray-300 mb-2">
        {t("stockAnalysis.charts.sectorAllocation", "行业配置")}
      </h4>
      <ResponsiveContainer width="100%" height={height}>
        <PieChart>
          <Pie
            data={sorted}
            dataKey="pct"
            nameKey="sector"
            cx="50%"
            cy="50%"
            innerRadius={55}
            outerRadius={85}
            paddingAngle={2}
          >
            {sorted.map((_, i) => <Cell key={i} fill={COLORS[i % COLORS.length]} stroke="none" />)}
          </Pie>
          <Tooltip
            contentStyle={{
              backgroundColor: "#1F2937",
              border: "1px solid #374151",
              borderRadius: "8px",
              color: "#F3F4F6",
              fontSize: 12,
            }}
            // eslint-disable-next-line @typescript-eslint/no-explicit-any
            formatter={(value: any, name: string) => [`${value.toFixed(1)}%`, name]}
          />
        </PieChart>
      </ResponsiveContainer>
      {/* 图例 */}
      <div className="grid grid-cols-2 sm:grid-cols-3 gap-1 mt-2">
        {sorted.slice(0, 9).map((item, i) => (
          <div key={item.sector} className="flex items-center gap-1.5 text-xs">
            <span
              className="w-2 h-2 rounded-full shrink-0"
              style={{ backgroundColor: COLORS[i % COLORS.length] }}
            />
            <span className="text-gray-400 truncate">{item.sector}</span>
            <span className="text-gray-300 font-mono">{item.pct.toFixed(1)}%</span>
          </div>
        ))}
        {sorted.length > 9 && <div className="text-xs text-gray-500">+{sorted.length - 9} 更多</div>}
      </div>
    </div>
  );
}

// ── 相关性热力图 ──

interface CorrelationCell {
  stock1: string;
  stock2: string;
  correlation: number;
}

interface CorrelationHeatmapProps {
  stocks: string[];
  correlations: CorrelationCell[];
  height?: number;
}

/**
 * 持仓相关性热力图（简化版 — 矩阵表格）
 */
export function CorrelationHeatmap({ stocks, correlations, height = 200 }: CorrelationHeatmapProps) {
  const { t } = useTranslation();

  if (stocks.length < 2) {
    return (
      <div className="text-gray-400 text-xs text-center py-4">
        {t("stockAnalysis.charts.needMoreStocks", "至少需要 2 只持仓股票")}
      </div>
    );
  }

  const getCorr = (s1: string, s2: string): number => {
    if (s1 === s2) { return 1.0; }
    const cell = correlations.find(
      (c) => (c.stock1 === s1 && c.stock2 === s2) || (c.stock1 === s2 && c.stock2 === s1),
    );
    return cell?.correlation ?? 0;
  };

  const corrColor = (v: number): string => {
    if (v > 0.7) { return "bg-red-900/60 text-red-300"; }
    if (v > 0.5) { return "bg-orange-900/50 text-orange-300"; }
    if (v > 0.3) { return "bg-yellow-900/40 text-yellow-300"; }
    if (v < -0.3) { return "bg-green-900/40 text-green-300"; }
    return "bg-gray-800/60 text-gray-400";
  };

  return (
    <div className="bg-gray-900/50 rounded-lg p-3">
      <h4 className="text-sm font-medium text-gray-300 mb-2">
        {t("stockAnalysis.charts.correlationMatrix", "相关性矩阵")}
      </h4>
      <div className="overflow-x-auto" style={{ height }}>
        <table className="text-xs">
          <thead>
            <tr>
              <th className="p-1 text-gray-500 font-normal" />
              {stocks.map((s) => (
                <th key={s} className="p-1 text-gray-500 font-normal text-right">
                  {s}
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {stocks.map((s1) => (
              <tr key={s1}>
                <td className="p-1 text-gray-400 font-medium pr-2">{s1}</td>
                {stocks.map((s2) => {
                  const v = getCorr(s1, s2);
                  return (
                    <td
                      key={s2}
                      className={`p-1 text-center text-[10px] font-mono rounded ${corrColor(v)}`}
                    >
                      {v.toFixed(2)}
                    </td>
                  );
                })}
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}

// ── 交易盈亏分布 ──

interface PnLBucket {
  range: string;
  count: number;
  total: number;
}

interface PnLHistogramProps {
  data: PnLBucket[];
  height?: number;
}

/**
 * 交易盈亏分布柱状图
 */
export function PnLHistogram({ data, height = 200 }: PnLHistogramProps) {
  const { t } = useTranslation();

  if (data.length === 0) {
    return (
      <div className="text-gray-400 text-xs text-center py-8">
        {t("stockAnalysis.charts.noTradeData", "无交易数据")}
      </div>
    );
  }

  return (
    <div className="bg-gray-900/50 rounded-lg p-3">
      <h4 className="text-sm font-medium text-gray-300 mb-2">
        {t("stockAnalysis.charts.pnlDistribution", "盈亏分布")}
      </h4>
      <ResponsiveContainer width="100%" height={height}>
        <BarChart data={data}>
          <CartesianGrid strokeDasharray="3 3" stroke="#374151" />
          <XAxis
            dataKey="range"
            tick={{ fill: "#9CA3AF", fontSize: 10 }}
            tickLine={false}
            axisLine={{ stroke: "#4B5563" }}
          />
          <YAxis
            tick={{ fill: "#9CA3AF", fontSize: 10 }}
            tickLine={false}
            axisLine={{ stroke: "#4B5563" }}
          />
          <Tooltip
            contentStyle={{
              backgroundColor: "#1F2937",
              border: "1px solid #374151",
              borderRadius: "8px",
              color: "#F3F4F6",
              fontSize: 12,
            }}
          />
          <Bar dataKey="count" radius={[3, 3, 0, 0]}>
            {data.map((entry, i) => (
              <Cell
                key={i}
                fill={entry.total >= 0 ? "#22C55E" : "#EF4444"}
                fillOpacity={0.7}
              />
            ))}
          </Bar>
        </BarChart>
      </ResponsiveContainer>
    </div>
  );
}

// ── 组合净值 vs 基准 ──

interface EquityDataPoint {
  date: string;
  portfolio: number;
  benchmark?: number;
}

interface PortfolioPerformanceLineProps {
  data: EquityDataPoint[];
  height?: number;
}

/**
 * 组合净值 vs 基准对比曲线
 */
export function PortfolioPerformanceLine({ data, height = 250 }: PortfolioPerformanceLineProps) {
  const { t } = useTranslation();

  if (data.length < 2) {
    return (
      <div className="text-gray-400 text-xs text-center py-8">
        {t("stockAnalysis.charts.needMoreData", "数据不足")}
      </div>
    );
  }

  return (
    <div className="bg-gray-900/50 rounded-lg p-3">
      <h4 className="text-sm font-medium text-gray-300 mb-2">
        {t("stockAnalysis.charts.portfolioVsBenchmark", "组合 vs 基准")}
      </h4>
      <ResponsiveContainer width="100%" height={height}>
        <LineChart data={data}>
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
            tickFormatter={(v: number) => `${v.toFixed(0)}%`}
          />
          <Tooltip
            contentStyle={{
              backgroundColor: "#1F2937",
              border: "1px solid #374151",
              borderRadius: "8px",
              color: "#F3F4F6",
              fontSize: 12,
            }}
            // eslint-disable-next-line @typescript-eslint/no-explicit-any
            formatter={(value: any, name: string) => {
              const labels: Record<string, string> = {
                portfolio: t("stockAnalysis.charts.portfolio", "组合"),
                benchmark: t("stockAnalysis.charts.benchmark", "基准"),
              };
              return [`${value.toFixed(2)}%`, labels[name] ?? name];
            }}
          />
          <Legend wrapperStyle={{ color: "#D1D5DB", fontSize: 12 }} />
          <Line
            type="monotone"
            dataKey="portfolio"
            stroke="#22C55E"
            strokeWidth={2}
            dot={false}
            name="portfolio"
          />
          {data[0]?.benchmark !== undefined && (
            <Line
              type="monotone"
              dataKey="benchmark"
              stroke="#6B7280"
              strokeWidth={1}
              strokeDasharray="4 4"
              dot={false}
              name="benchmark"
            />
          )}
        </LineChart>
      </ResponsiveContainer>
    </div>
  );
}

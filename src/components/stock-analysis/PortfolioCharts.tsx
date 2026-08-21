/**
 * 组合可视化图表集
 *
 * 包含：
 * - SectorAllocationDonut: 行业配置圆环图
 * - CorrelationHeatmap: 持仓相关性热力图
 * - PnLHistogram: 交易盈亏分布直方图
 * - PortfolioPerformanceLine: 组合净值 vs 基准对比曲线
 */

import * as echarts from "echarts";
import { useEffect, useMemo, useRef } from "react";
import { useTranslation } from "react-i18next";

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

/** 通用 echarts hook */
function useECharts(
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
 * 行业配置圆环图
 */
export function SectorAllocationDonut({ data, height = 250 }: SectorAllocationDonutProps) {
  const { t } = useTranslation();
  const sorted = useMemo(() => [...data].sort((a, b) => b.pct - a.pct), [data]);

  const ref = useECharts(
    () => ({
      tooltip: {
        trigger: "item",
        backgroundColor: "#1F2937",
        borderColor: "#374151",
        textStyle: { color: "#F3F4F6", fontSize: 12 },
        formatter: (params: unknown) => {
          const p = params as { value: number; name: string };
          return `${p.name}: ${p.value.toFixed(1)}%`;
        },
      },
      series: [
        {
          type: "pie",
          radius: ["45%", "75%"],
          center: ["50%", "50%"],
          padAngle: 2,
          itemStyle: { borderRadius: 4, borderColor: "#111827", borderWidth: 2 },
          label: { show: false },
          data: sorted.map((d, i) => ({
            value: d.pct,
            name: d.sector,
            itemStyle: { color: COLORS[i % COLORS.length] },
          })),
        },
      ],
    }),
    [sorted],
  );

  if (sorted.length === 0) {
    return (
      <div className="text-gray-400 text-xs text-center py-8">
        {t("stockAnalysis.charts.noSectorData")}
      </div>
    );
  }

  return (
    <div className="bg-gray-900/50 rounded-lg p-3">
      <h4 className="text-sm font-medium text-gray-300 mb-2">
        {t("stockAnalysis.charts.sectorAllocation")}
      </h4>
      <div ref={ref} style={{ width: "100%", height }} />
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
        {sorted.length > 9 && (
          <div className="text-xs text-gray-500">+{t("stockAnalysis.charts.more", { count: sorted.length - 9 })}</div>
        )}
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
        {t("stockAnalysis.charts.needMoreStocks")}
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
        {t("stockAnalysis.charts.correlationMatrix")}
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

  const ref = useECharts(
    () => ({
      tooltip: {
        trigger: "axis",
        backgroundColor: "#1F2937",
        borderColor: "#374151",
        textStyle: { color: "#F3F4F6", fontSize: 12 },
      },
      grid: { top: 12, right: 12, bottom: 24, left: 48 },
      xAxis: {
        type: "category",
        data: data.map((d) => d.range),
        axisLine: { lineStyle: { color: "#4B5563" } },
        axisLabel: { color: "#9CA3AF", fontSize: 10 },
        axisTick: { show: false },
      },
      yAxis: {
        type: "value",
        axisLine: { lineStyle: { color: "#4B5563" } },
        axisLabel: { color: "#9CA3AF", fontSize: 10 },
        splitLine: { lineStyle: { color: "#374151", type: "dashed" } },
        axisTick: { show: false },
      },
      series: [
        {
          type: "bar",
          data: data.map((d) => ({
            value: d.count,
            itemStyle: { color: d.total >= 0 ? "#22C55E" : "#EF4444", opacity: 0.7 },
          })),
          barMaxWidth: 32,
          itemStyle: { borderRadius: [3, 3, 0, 0] },
        },
      ],
    }),
    [data],
  );

  if (data.length === 0) {
    return (
      <div className="text-gray-400 text-xs text-center py-8">
        {t("stockAnalysis.charts.noTradeData")}
      </div>
    );
  }

  return (
    <div className="bg-gray-900/50 rounded-lg p-3">
      <h4 className="text-sm font-medium text-gray-300 mb-2">
        {t("stockAnalysis.charts.pnlDistribution")}
      </h4>
      <div ref={ref} style={{ width: "100%", height }} />
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

  const ref = useECharts(
    () => {
      const hasBenchmark = data.length > 0 && data[0]?.benchmark !== undefined;
      return {
        tooltip: {
          trigger: "axis",
          backgroundColor: "#1F2937",
          borderColor: "#374151",
          textStyle: { color: "#F3F4F6", fontSize: 12 },
          formatter: (params: unknown) => {
            const ps = params as { seriesName: string; value: number; axisValue: string }[];
            const labels: Record<string, string> = {
              portfolio: t("stockAnalysis.charts.portfolio"),
              benchmark: t("stockAnalysis.charts.benchmark"),
            };
            let result = ps[0]?.axisValue ?? "";
            for (const p of ps) {
              result += `<br/>${labels[p.seriesName] ?? p.seriesName}: ${p.value.toFixed(2)}%`;
            }
            return result;
          },
        },
        legend: { textStyle: { color: "#D1D5DB", fontSize: 12 } },
        grid: { top: 32, right: 16, bottom: 24, left: 48 },
        xAxis: {
          type: "category",
          data: data.map((d) => d.date),
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
            formatter: (v: number) => `${v.toFixed(0)}%`,
          },
          splitLine: { lineStyle: { color: "#374151", type: "dashed" } },
          axisTick: { show: false },
        },
        series: [
          {
            name: "portfolio",
            type: "line",
            data: data.map((d) => d.portfolio),
            smooth: true,
            lineStyle: { color: "#22C55E", width: 2 },
            itemStyle: { color: "#22C55E" },
            showSymbol: false,
          },
          ...(hasBenchmark
            ? [
              {
                name: "benchmark",
                type: "line" as const,
                data: data.map((d) => d.benchmark),
                smooth: true,
                lineStyle: { color: "#6B7280", width: 1, type: "dashed" as const },
                itemStyle: { color: "#6B7280" },
                showSymbol: false,
              },
            ]
            : []),
        ],
      };
    },
    [data, t],
  );

  if (data.length < 2) {
    return (
      <div className="text-gray-400 text-xs text-center py-8">
        {t("stockAnalysis.charts.needMoreData")}
      </div>
    );
  }

  return (
    <div className="bg-gray-900/50 rounded-lg p-3">
      <h4 className="text-sm font-medium text-gray-300 mb-2">
        {t("stockAnalysis.charts.portfolioVsBenchmark")}
      </h4>
      <div ref={ref} style={{ width: "100%", height }} />
    </div>
  );
}

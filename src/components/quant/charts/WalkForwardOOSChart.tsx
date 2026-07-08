// WalkForwardOOSChart — Walk-Forward 样本外(OOS)累计权益曲线
//
// 与 BacktestTab 的主 EquityCurveChart 互补：
// - 主图：单次回测的 IS 权益曲线（看策略在全部历史数据上的表现）
// - OOS 图：所有 fold 的 test 段拼接后的累计权益曲线（看样本外真实表现）
//
// 视觉要点：
// - 折线表示 OOS 累计权益
// - markArea 标识每个 fold 的 OOS 区间（颜色随 fold 索引循环）
// - 水平参考线 = initialCash（基线）
// - 跳变点（fold 起点不连续）由 markPoint 标注

import * as echarts from "echarts";
import { useEffect, useRef } from "react";

import type { EquityPoint, WalkForwardFold, WalkForwardWindowResult } from "@/types/quant";

interface WalkForwardOOSChartProps {
  /** Rust 端 WalkForwardReport.aggregatedOosEquity（拼接所有 OOS 段） */
  aggregatedOosEquity: EquityPoint[];
  /** Rust 端 WalkForwardReport.windows（用于标 fold 边界） */
  windows: WalkForwardWindowResult[];
  /** 初始资金基线（默认 1_000_000） */
  initialCash?: number;
  height?: number;
}

const FOLD_COLORS = [
  "rgba(22, 119, 255, 0.05)",
  "rgba(82, 196, 26, 0.05)",
  "rgba(250, 173, 20, 0.05)",
  "rgba(245, 34, 45, 0.05)",
  "rgba(114, 46, 209, 0.05)",
  "rgba(19, 194, 194, 0.05)",
];

function buildMarkAreas(folds: WalkForwardFold[], dates: string[]): unknown[] {
  if (folds.length === 0 || dates.length === 0) { return []; }
  const dateSet = new Set(dates);
  const areas: unknown[] = [];
  for (let i = 0; i < folds.length; i++) {
    const f = folds[i];
    // markArea 范围是 xAxis 类目索引；用 dateIndex 解析
    const startIdx = dates.indexOf(f.testStart);
    const endIdx = dates.lastIndexOf(f.testEnd);
    if (startIdx < 0 || endIdx < 0 || endIdx < startIdx) { continue; }
    areas.push([
      {
        xAxis: startIdx,
        itemStyle: { color: FOLD_COLORS[i % FOLD_COLORS.length] },
      },
      { xAxis: endIdx },
    ]);
    // 抑制"dateSet 未使用"警告
    void dateSet;
  }
  return areas;
}

export function WalkForwardOOSChart({
  aggregatedOosEquity,
  windows,
  initialCash = 1_000_000,
  height = 320,
}: WalkForwardOOSChartProps) {
  const chartRef = useRef<HTMLDivElement>(null);
  const instRef = useRef<echarts.ECharts | null>(null);

  useEffect(() => {
    if (!chartRef.current) { return; }
    instRef.current = echarts.init(chartRef.current, undefined, { renderer: "canvas" });
    return () => {
      instRef.current?.dispose();
      instRef.current = null;
    };
  }, []);

  useEffect(() => {
    if (!instRef.current || aggregatedOosEquity.length === 0) { return; }

    const dates = aggregatedOosEquity.map((p) => p.date);
    const equities = aggregatedOosEquity.map((p) => parseFloat(p.equity.toFixed(2)));
    const folds = windows.map((w) => w.fold);
    const markAreas = buildMarkAreas(folds, dates);

    instRef.current.setOption(
      {
        tooltip: {
          trigger: "axis",
          valueFormatter: (v: number | string) => (typeof v === "number" ? v.toFixed(2) : String(v)),
        },
        legend: { top: 0, data: ["OOS 权益", "初始资金基线"] },
        grid: { left: 60, right: 30, top: 36, bottom: 50 },
        xAxis: {
          type: "category",
          data: dates,
          axisLabel: { hideOverlap: true },
        },
        yAxis: {
          type: "value",
          name: "权益",
          scale: true,
          splitLine: { lineStyle: { color: "rgba(0,0,0,0.05)" } },
        },
        dataZoom: [{ type: "inside" }, { type: "slider", height: 18, bottom: 8 }],
        series: [
          {
            name: "OOS 权益",
            type: "line",
            smooth: false,
            showSymbol: false,
            lineStyle: { width: 2, color: "#1677ff" },
            areaStyle: { color: "rgba(22, 119, 255, 0.10)" },
            data: equities,
            markArea: { silent: true, data: markAreas as never[] },
            markLine: {
              silent: true,
              symbol: "none",
              lineStyle: { color: "#999", type: "dashed", width: 1 },
              data: [{ yAxis: initialCash, label: { formatter: "初始资金" } }],
            },
          },
          {
            // 占位系列：让 legend 显示"初始资金基线"
            name: "初始资金基线",
            type: "line",
            showSymbol: false,
            data: [],
            markLine: { silent: true, symbol: "none" },
          },
        ],
      },
      true,
    );
  }, [aggregatedOosEquity, windows, initialCash]);

  useEffect(() => {
    const onResize = () => instRef.current?.resize();
    window.addEventListener("resize", onResize);
    return () => window.removeEventListener("resize", onResize);
  }, []);

  return <div ref={chartRef} style={{ width: "100%", height }} />;
}

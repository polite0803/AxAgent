// WalkForwardFoldBarChart — per-fold 训练 vs 测试 Sharpe 柱状图
//
// 目的：直观看到每个 fold 的 IS/OOS 性能退化，识别过拟合 fold
// 视觉：
// - X 轴：fold 索引（0..N-1）
// - 柱状：train Sharpe（蓝） vs test Sharpe（绿/红：过拟合标红）
// - 折线：degradationRatio（test/train），右侧 Y 轴
// - 0.3 阈值线：低于则视为过拟合
// - tooltip：fold 区间 + 关键指标

import * as echarts from "echarts";
import { useEffect, useRef } from "react";

import type { WalkForwardWindowResult } from "@/types/quant";

interface WalkForwardFoldBarChartProps {
  windows: WalkForwardWindowResult[];
  height?: number;
}

export function WalkForwardFoldBarChart({ windows, height = 280 }: WalkForwardFoldBarChartProps) {
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
    if (!instRef.current || windows.length === 0) { return; }

    const foldLabels = windows.map((w) => `F${w.fold.foldIndex}`);
    const trainSharpe = windows.map((w) => parseFloat(w.trainMetrics.sharpe.toFixed(3)));
    const testSharpe = windows.map((w) => parseFloat(w.testMetrics.sharpe.toFixed(3)));
    // test_bar 颜色：过拟合 fold 红色，否则绿色
    const testColors = windows.map((w) => (w.overfitFlag ? "#cf1322" : "#52c41a"));
    const degradation = windows.map((w) => parseFloat(w.degradationRatio.toFixed(3)));

    instRef.current.setOption(
      {
        tooltip: {
          trigger: "axis",
          axisPointer: { type: "shadow" },
          formatter: (params: unknown) => {
            const arr = params as Array<{ dataIndex: number }>;
            const w = windows[arr[0].dataIndex];
            const f = w.fold;
            return [
              `<b>Fold ${f.foldIndex}</b> ${w.overfitFlag ? "<span style='color:#cf1322'>(过拟合)</span>" : ""}`,
              `IS: ${f.trainStart} → ${f.trainEnd} (${f.trainBarsCount} 根)`,
              `OOS: ${f.testStart} → ${f.testEnd} (${f.testBarsCount} 根)`,
              `Train Sharpe: ${w.trainMetrics.sharpe.toFixed(3)}`,
              `Test  Sharpe: ${w.testMetrics.sharpe.toFixed(3)}`,
              `Degradation: ${w.degradationRatio.toFixed(3)} ${w.overfitFlag ? "⚠️" : ""}`,
            ].join("<br/>");
          },
        },
        legend: {
          top: 0,
          data: ["Train Sharpe", "Test Sharpe", "Degradation Ratio"],
        },
        grid: { left: 60, right: 60, top: 36, bottom: 40 },
        xAxis: {
          type: "category",
          data: foldLabels,
          name: "Fold",
          axisLabel: { interval: 0 },
        },
        yAxis: [
          {
            type: "value",
            name: "Sharpe",
            splitLine: { lineStyle: { color: "rgba(0,0,0,0.05)" } },
          },
          {
            type: "value",
            name: "Degradation",
            position: "right",
            min: 0,
            max: 1.2,
            splitLine: { show: false },
          },
        ],
        dataZoom: windows.length > 12 ? [{ type: "inside" }, { type: "slider", height: 14, bottom: 4 }] : [],
        series: [
          {
            name: "Train Sharpe",
            type: "bar",
            data: trainSharpe,
            itemStyle: { color: "#1677ff" },
            barGap: 0,
          },
          {
            name: "Test Sharpe",
            type: "bar",
            data: testSharpe.map((v, i) => ({ value: v, itemStyle: { color: testColors[i] } })),
          },
          {
            name: "Degradation Ratio",
            type: "line",
            yAxisIndex: 1,
            smooth: false,
            showSymbol: true,
            symbolSize: 6,
            lineStyle: { width: 1.5, color: "#fa8c16", type: "dashed" },
            itemStyle: { color: "#fa8c16" },
            data: degradation,
            markLine: {
              silent: true,
              symbol: "none",
              lineStyle: { color: "#cf1322", type: "dotted", width: 1 },
              data: [{ yAxis: 0.3, label: { formatter: "过拟合阈值 0.3" } }],
            },
          },
        ],
      },
      true,
    );
  }, [windows]);

  useEffect(() => {
    const onResize = () => instRef.current?.resize();
    window.addEventListener("resize", onResize);
    return () => window.removeEventListener("resize", onResize);
  }, []);

  return <div ref={chartRef} style={{ width: "100%", height }} />;
}

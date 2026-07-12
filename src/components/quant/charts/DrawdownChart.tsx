// DrawdownChart — 回撤面积图

import * as echarts from "echarts";
import { useEffect, useRef } from "react";
import { useTranslation } from "react-i18next";

import type { EquityPoint } from "@/types";

import { computeDrawdownPercent } from "./drawdown";

interface DrawdownChartProps {
  curve: EquityPoint[];
  height?: number;
}

export function DrawdownChart({ curve, height = 220 }: DrawdownChartProps) {
  const { t } = useTranslation();
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
    if (!instRef.current || curve.length === 0) { return; }
    const dates = curve.map((p) => p.date);
    // 回撤由权益曲线本地推导（Rust EquityPoint 不含 drawdown 字段）
    const data = computeDrawdownPercent(curve);

    instRef.current.setOption(
      {
        tooltip: { trigger: "axis", valueFormatter: (v: number) => `${v.toFixed(2)}%` },
        grid: { left: 50, right: 30, top: 16, bottom: 40 },
        xAxis: { type: "category", data: dates, axisLabel: { hideOverlap: true } },
        yAxis: {
          type: "value",
          name: t("quant.metrics.maxDrawdownPct"),
          max: 0,
          splitLine: { lineStyle: { color: "rgba(0,0,0,0.05)" } },
        },
        dataZoom: [{ type: "inside" }, { type: "slider", height: 14, bottom: 6 }],
        series: [
          {
            type: "line",
            showSymbol: false,
            smooth: true,
            lineStyle: { color: "#cf1322", width: 1.5 },
            areaStyle: { color: "rgba(207, 19, 34, 0.18)" },
            data,
          },
        ],
      },
      true,
    );
  }, [curve, t]);

  useEffect(() => {
    const onResize = () => instRef.current?.resize();
    window.addEventListener("resize", onResize);
    return () => window.removeEventListener("resize", onResize);
  }, []);

  return <div ref={chartRef} style={{ width: "100%", height }} />;
}

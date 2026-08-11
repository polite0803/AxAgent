// SPDX-License-Identifier: AGPL-3.0-only

import { useTranslation } from "react-i18next";
import { useEffect, useMemo, useRef } from "react";
import * as echarts from "echarts";

const COLORS = [
  "#1677ff",
  "#52c41a",
  "#faad14",
  "#ff4d4f",
  "#722ed1",
  "#13c2c2",
  "#eb2f96",
  "#fa8c16",
];

type ChartType = "line" | "bar" | "pie" | "scatter" | "area";

interface ChartRendererImplProps {
  chartType: ChartType;
  data: Record<string, unknown>[];
  xKey: string;
  yKey: string;
  seriesKey?: string;
}

function buildChartOption(
  chartType: ChartType,
  data: Record<string, unknown>[],
  xKey: string,
  yKey: string,
  seriesKey?: string,
): echarts.EChartsOption {
  const xData = data.map((d) => String(d[xKey] ?? ""));

  switch (chartType) {
    case "bar": {
      if (seriesKey) {
        return {
          tooltip: { trigger: "axis" },
          grid: { top: 16, right: 16, bottom: 32, left: 48 },
          xAxis: {
            type: "category",
            data: xData,
            axisLabel: { color: "#999" },
          },
          yAxis: { type: "value", axisLabel: { color: "#999" } },
          series: [
            {
              type: "bar",
              data: data.map((d, i) => ({
                value: Number(d[seriesKey] ?? 0),
                itemStyle: { color: COLORS[i % COLORS.length] },
              })),
            },
          ],
        };
      }
      return {
        tooltip: { trigger: "axis" },
        grid: { top: 16, right: 16, bottom: 32, left: 48 },
        xAxis: {
          type: "category",
          data: xData,
          axisLabel: { color: "#999" },
        },
        yAxis: { type: "value", axisLabel: { color: "#999" } },
        series: [
          {
            type: "bar",
            data: data.map((d) => Number(d[yKey] ?? 0)),
            itemStyle: { color: COLORS[0] },
          },
        ],
      };
    }

    case "line": {
      return {
        tooltip: { trigger: "axis" },
        grid: { top: 16, right: 16, bottom: 32, left: 48 },
        xAxis: {
          type: "category",
          data: xData,
          boundaryGap: false,
          axisLabel: { color: "#999" },
        },
        yAxis: { type: "value", axisLabel: { color: "#999" } },
        series: [
          {
            type: "line",
            data: data.map((d) => Number(d[yKey] ?? 0)),
            smooth: true,
            showSymbol: false,
            lineStyle: { color: COLORS[0], width: 2 },
            itemStyle: { color: COLORS[0] },
          },
        ],
      };
    }

    case "area": {
      return {
        tooltip: { trigger: "axis" },
        grid: { top: 16, right: 16, bottom: 32, left: 48 },
        xAxis: {
          type: "category",
          data: xData,
          boundaryGap: false,
          axisLabel: { color: "#999" },
        },
        yAxis: { type: "value", axisLabel: { color: "#999" } },
        series: [
          {
            type: "line",
            data: data.map((d) => Number(d[yKey] ?? 0)),
            smooth: true,
            showSymbol: false,
            lineStyle: { color: COLORS[0], width: 2 },
            itemStyle: { color: COLORS[0] },
            areaStyle: { color: COLORS[0], opacity: 0.3 },
          },
        ],
      };
    }

    case "pie": {
      return {
        tooltip: { trigger: "item" },
        legend: { bottom: 0, left: "center" },
        series: [
          {
            type: "pie",
            radius: "65%",
            center: ["50%", "45%"],
            data: data.map((d, i) => ({
              value: Number(d[yKey] ?? 0),
              name: String(d[xKey] ?? ""),
              itemStyle: { color: COLORS[i % COLORS.length] },
            })),
            label: { show: true },
          },
        ],
      };
    }

    case "scatter": {
      return {
        tooltip: { trigger: "item" },
        grid: { top: 16, right: 16, bottom: 32, left: 48 },
        xAxis: { type: "value", name: xKey, axisLabel: { color: "#999" } },
        yAxis: { type: "value", name: yKey, axisLabel: { color: "#999" } },
        series: [
          {
            type: "scatter",
            data: data.map((d) => [Number(d[xKey] ?? 0), Number(d[yKey] ?? 0)]),
            itemStyle: { color: COLORS[0] },
          },
        ],
      };
    }

    default:
      return {};
  }
}

export function ChartRendererImpl({
  chartType,
  data,
  xKey,
  yKey,
  seriesKey,
}: ChartRendererImplProps) {
  const { t } = useTranslation();
  const chartRef = useRef<HTMLDivElement>(null);
  const chartInstance = useRef<echarts.ECharts | null>(null);

  const option = useMemo(
    () => buildChartOption(chartType, data, xKey, yKey, seriesKey),
    [chartType, data, xKey, yKey, seriesKey],
  );

  useEffect(() => {
    if (!chartRef.current) { return; }
    if (!chartInstance.current) {
      chartInstance.current = echarts.init(chartRef.current);
    }
    chartInstance.current.setOption(option, true);
    const handleResize = () => chartInstance.current?.resize();
    window.addEventListener("resize", handleResize);
    return () => {
      window.removeEventListener("resize", handleResize);
      chartInstance.current?.dispose();
      chartInstance.current = null;
    };
  }, [option]);

  if (data.length === 0) {
    return (
      <div className="flex items-center justify-center h-64 text-gray-400">
        {t("dynamicUI.noData")}
      </div>
    );
  }

  return (
    <div
      ref={chartRef}
      style={{ width: "100%", height: 300 }}
    />
  );
}

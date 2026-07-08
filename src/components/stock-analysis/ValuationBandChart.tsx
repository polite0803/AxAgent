import { useSettingsStore } from "@/stores";
import * as echarts from "echarts";
import { useCallback, useEffect, useRef } from "react";
import { useTranslation } from "react-i18next";

/// 与后端 ValuationBand 一致的最小结构(只读前端需要的字段)
export interface ValuationBandData {
  stockCode: string;
  metricPe: MetricBandData;
  metricPb: MetricBandData;
  metricPs: MetricBandData;
  sampleStart?: string;
  sampleEnd?: string;
  verdict: string;
  note?: string;
}

export interface MetricBandData {
  /// [5, 10, 25, 50, 75, 90, 95] 分位
  percentiles: number[];
  current?: number;
  currentPercentile?: number;
  sampleSize: number;
}

interface Props {
  data: ValuationBandData | null;
  /// "pe" | "pb" | "ps" - 默认 "pe"
  primary?: "pe" | "pb" | "ps";
  loading?: boolean;
  height?: number;
}

const PERCENTILE_LABELS = ["P5", "P10", "P25", "P50", "P75", "P90", "P95"];

const verdictColor = (v: string): string => {
  if (v === "deep_value") { return "#16a34a"; }
  if (v === "undervalued") { return "#22c55e"; }
  if (v === "overvalued") { return "#ef4444"; }
  if (v === "expensive") { return "#f97316"; }
  return "#94a3b8";
};

const verdictLabel = (v: string, t: (k: string) => string): string => {
  if (v === "deep_value") { return t("stockAnalysis.valuationBand.verdictDeepValue"); }
  if (v === "undervalued") { return t("stockAnalysis.valuationBand.verdictUndervalued"); }
  if (v === "overvalued") { return t("stockAnalysis.valuationBand.verdictOvervalued"); }
  if (v === "expensive") { return t("stockAnalysis.valuationBand.verdictExpensive"); }
  if (v === "fair") { return t("stockAnalysis.valuationBand.verdictFair"); }
  if (v === "insufficient") { return t("stockAnalysis.valuationBand.verdictInsufficient"); }
  return v;
};

/**
 * 估值带图表(R3-C):
 * - 柱状图:7 个分位 (P5/10/25/50/75/90/95) 横向对比 PE / PB
 * - 红点:当前值;虚线:当前分位 (0-100)
 * - 顶部 verdict 文字 + 颜色
 */
export function ValuationBandChart({ data, primary = "pe", loading, height = 240 }: Props) {
  const { t } = useTranslation();
  const themeMode = useSettingsStore((s) => s.settings.theme_mode);
  const elRef = useRef<HTMLDivElement | null>(null);
  const instanceRef = useRef<echarts.ECharts | null>(null);

  const isDark = themeMode === "dark"
    || (themeMode === "system" && window.matchMedia("(prefers-color-scheme: dark)").matches);

  const buildOption = useCallback((d: ValuationBandData | null): echarts.EChartsOption => {
    if (!d) {
      return {
        backgroundColor: "transparent",
        title: {
          text: t("stockAnalysis.valuationBand.noData"),
          left: "center",
          top: 8,
          textStyle: { fontSize: 12, color: "#94a3b8" },
        },
      };
    }
    const pe = d.metricPe;
    const pb = d.metricPb;
    const primaryMetric = primary === "pb" ? pb : primary === "ps" ? d.metricPs : pe;
    const primaryLabel = primary === "pb" ? "PB" : primary === "ps" ? "PS" : "PE";
    const otherMetric = primary === "pb" ? pe : pb;
    const otherLabel = primary === "pb" ? "PE" : "PB";

    const hasPrimary = primaryMetric.percentiles.length >= 7 && primaryMetric.percentiles.some((v) => v > 0);

    return {
      backgroundColor: "transparent",
      grid: [
        { left: 60, right: "55%", top: 50, bottom: 30, containLabel: false },
        { left: "55%", right: 30, top: 50, bottom: 30, containLabel: false },
      ],
      title: [
        {
          text: `${primaryLabel} ${t("stockAnalysis.valuationBand.title")}`,
          left: 60,
          top: 10,
          textStyle: { fontSize: 11, color: isDark ? "#cbd5e1" : "#475569" },
        },
        {
          text: `${otherLabel} ${t("stockAnalysis.valuationBand.title")}`,
          left: "55%",
          top: 10,
          textStyle: { fontSize: 11, color: isDark ? "#cbd5e1" : "#475569" },
        },
      ],
      tooltip: { trigger: "axis", axisPointer: { type: "shadow" } },
      xAxis: [
        {
          gridIndex: 0,
          type: "category",
          data: PERCENTILE_LABELS,
          axisLabel: { fontSize: 9, color: isDark ? "#cbd5e1" : "#475569" },
        },
        {
          gridIndex: 1,
          type: "category",
          data: PERCENTILE_LABELS,
          axisLabel: { fontSize: 9, color: isDark ? "#cbd5e1" : "#475569" },
        },
      ],
      yAxis: [
        {
          gridIndex: 0,
          type: "value",
          name: primaryLabel,
          nameTextStyle: { fontSize: 9, color: isDark ? "#94a3b8" : "#64748b" },
          axisLabel: { fontSize: 9, color: isDark ? "#94a3b8" : "#64748b" },
        },
        {
          gridIndex: 1,
          type: "value",
          name: otherLabel,
          nameTextStyle: { fontSize: 9, color: isDark ? "#94a3b8" : "#64748b" },
          axisLabel: { fontSize: 9, color: isDark ? "#94a3b8" : "#64748b" },
        },
      ],
      series: [
        {
          name: primaryLabel,
          type: "bar",
          xAxisIndex: 0,
          yAxisIndex: 0,
          data: hasPrimary ? primaryMetric.percentiles : [],
          itemStyle: { color: primary === "pb" ? "#3b82f6" : primary === "ps" ? "#a855f7" : "#f97316", opacity: 0.7 },
          barWidth: "60%",
          markPoint: primaryMetric.current != null
            ? {
              symbol: "pin",
              symbolSize: 36,
              data: [{
                name: t("stockAnalysis.valuationBand.current"),
                coord: ["P50", primaryMetric.current],
                value: primaryMetric.current,
                itemStyle: { color: "#dc2626" },
              }],
            }
            : undefined,
        },
        {
          name: otherLabel,
          type: "bar",
          xAxisIndex: 1,
          yAxisIndex: 1,
          data: otherMetric.percentiles,
          itemStyle: { color: primary === "pb" ? "#f97316" : "#3b82f6", opacity: 0.5 },
          barWidth: "60%",
          markPoint: otherMetric.current != null
            ? {
              symbol: "pin",
              symbolSize: 32,
              data: [{
                name: t("stockAnalysis.valuationBand.current"),
                coord: ["P50", otherMetric.current],
                value: otherMetric.current,
                itemStyle: { color: "#dc2626" },
              }],
            }
            : undefined,
        },
      ],
    } as echarts.EChartsOption;
  }, [t, isDark, primary]);

  useEffect(() => {
    if (!elRef.current) { return; }
    instanceRef.current = echarts.init(elRef.current, undefined, { renderer: "canvas" });
    return () => {
      instanceRef.current?.dispose();
      instanceRef.current = null;
    };
  }, []);

  useEffect(() => {
    if (!instanceRef.current) { return; }
    instanceRef.current.setOption(buildOption(data), true);
  }, [data, buildOption]);

  useEffect(() => {
    const handler = () => instanceRef.current?.resize();
    window.addEventListener("resize", handler);
    return () => window.removeEventListener("resize", handler);
  }, []);

  if (loading) {
    return (
      <div
        className="flex items-center justify-center text-xs text-gray-400"
        style={{ height }}
        data-testid="valuation-band-loading"
      >
        {t("common.loading")}
      </div>
    );
  }

  return (
    <div className="space-y-1">
      {data && (
        <div className="flex items-center gap-2 text-xs" data-testid="valuation-band-verdict">
          <Tag color={verdictColor(data.verdict)}>
            {verdictLabel(data.verdict, t)}
          </Tag>
          {data.metricPe.currentPercentile != null && (
            <span className="text-gray-500">
              {t("stockAnalysis.valuationBand.pePercentile")}: {data.metricPe.currentPercentile.toFixed(0)}%
            </span>
          )}
          {data.metricPb.currentPercentile != null && (
            <span className="text-gray-500">
              {t("stockAnalysis.valuationBand.pbPercentile")}: {data.metricPb.currentPercentile.toFixed(0)}%
            </span>
          )}
          {data.sampleStart && data.sampleEnd && (
            <span className="text-gray-400 ml-auto">
              {data.sampleStart} ~ {data.sampleEnd}
            </span>
          )}
        </div>
      )}
      <div ref={elRef} style={{ height }} data-testid="valuation-band-chart" />
      {data?.note && <div className="text-xs text-amber-500">{data.note}</div>}
    </div>
  );
}

// 局部 Tag 包装(避免 antd 强耦合,保持轻量)
function Tag({ color, children }: { color: string; children: React.ReactNode }) {
  return (
    <span
      className="px-1.5 py-0.5 rounded text-[10px] text-white"
      style={{ backgroundColor: color }}
    >
      {children}
    </span>
  );
}

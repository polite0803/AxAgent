import { useStockAnalysisStore } from "@/stores";
import { useTimeAnchorStore } from "@/stores/feature/timeAnchorStore";
import type { KLine } from "@/types/stock-analysis";
import * as echarts from "echarts";
import { useCallback, useEffect, useMemo, useRef } from "react";
import { useTranslation } from "react-i18next";

// ── 缓存 ──
interface CachedKline {
  data: KLine[];
  ts: number;
}
const klineCache = new Map<string, CachedKline>();
const KLINE_CACHE_TTL_MS = 5 * 60 * 1000;
const KLINE_CACHE_MAX = 20;
function cacheKey(sc: string, p: string, l: number, a: string | null) {
  return `${sc}|${p}|${l}|${a ?? "live"}`;
}

// ── 自适应粒度层级 ──
// 每个层级定义：可见 K 线数范围 → 对应 period + 要拉的 limit
// 目标：K 线柱子宽度大致恒定 (~8-12px/根)
const TIERS = [
  { min: 0, max: 12, period: "5", limit: 80, key: "5m" },
  { min: 12, max: 30, period: "15", limit: 120, key: "15m" },
  { min: 30, max: 55, period: "30", limit: 160, key: "30m" },
  { min: 55, max: 100, period: "60", limit: 200, key: "60m" },
  { min: 100, max: 220, period: "daily", limit: 320, key: "1d" },
  { min: 220, max: 500, period: "weekly", limit: 520, key: "1w" },
  { min: 500, max: Infinity, period: "monthly", limit: 600, key: "1M" },
] as const;
type Tier = typeof TIERS[number];

function findTier(visibleCount: number): Tier {
  for (const t of TIERS) { if (visibleCount >= t.min && visibleCount < t.max) { return t; } }
  return TIERS[TIERS.length - 1];
}

// ── MA 计算 ──
function calcMA(data: number[], w: number): (number | null)[] {
  return data.map((_, i) => {
    if (i < w - 1) { return null; }
    let s = 0;
    for (let j = i - w + 1; j <= i; j++) { s += data[j]; }
    return parseFloat((s / w).toFixed(2));
  });
}

// ── 颜色 ──
const SA_RED = "oklch(60% 0.20 30)";
const SA_GREEN = "oklch(62% 0.18 150)";
const MA_ORANGE = "oklch(68% 0.16 50)";
const MA_BLUE = "oklch(55% 0.14 250)";
const MA_PURPLE = "oklch(55% 0.14 310)";

const EARNINGS_COLORS: Record<string, string> = {
  preliminary: "#c026d3",
  express: "#7c3aed",
  formal: "#1e40af",
  shareholders_meeting: "#0891b2",
  other: "#6b7280",
};

export function KLineChart() {
  const { t } = useTranslation();
  const klineData = useStockAnalysisStore((s) => s.klineData);
  const stockCode = useStockAnalysisStore((s) => s.stockCode);
  const klinePeriod = useStockAnalysisStore((s) => s.klinePeriod);
  const setKlinePeriod = useStockAnalysisStore((s) => s.setKlinePeriod);
  const klineAdj = useStockAnalysisStore((s) => s.klineAdj);
  const setKlineAdj = useStockAnalysisStore((s) => s.setKlineAdj);
  const indicators = useStockAnalysisStore((s) => s.klineIndicators);
  const toggleIndicator = useStockAnalysisStore((s) => s.toggleIndicator);
  const getStockKline = useStockAnalysisStore((s) => s.getStockKline);
  const earningsEvents = useStockAnalysisStore((s) => s.earningsEvents);
  const showEarningsOnChart = useStockAnalysisStore((s) => s.showEarningsOnChart);
  const setShowEarningsOnChart = useStockAnalysisStore((s) => s.setShowEarningsOnChart);
  const asOfDate = useTimeAnchorStore((s) => s.asOfDate);
  const chartRef = useRef<HTMLDivElement>(null);
  const instanceRef = useRef<echarts.ECharts | null>(null);

  // 避免 zoomevent → setState → re-render → setOption → zoomevent 循环
  const zoomingRef = useRef(false);
  const lastTierRef = useRef<Tier | null>(null);
  // 持久化 zoom handler 引用，init 时绑定一次，避免 setOption 每次重绑
  const zoomHandlerRef = useRef<((params: unknown) => void) | null>(null);

  const getChart = useCallback((): echarts.ECharts | null => {
    const c = instanceRef.current;
    if (c && !c.isDisposed()) { return c; }
    instanceRef.current = null;
    return null;
  }, []);

  // ── 周期切换（手动或自适应） ──
  const switchPeriod = useCallback((period: string, limit: number) => {
    if (!stockCode) { return; }
    const asOfDate = useTimeAnchorStore.getState().asOfDate;
    const ck = cacheKey(stockCode, period, limit, asOfDate);
    const cached = klineCache.get(ck);
    if (cached && Date.now() - cached.ts < KLINE_CACHE_TTL_MS) {
      // 缓存命中：直接使用缓存数据，跳过网络请求
      klineCache.delete(ck);
      klineCache.set(ck, cached);
      useStockAnalysisStore.setState({ klineData: cached.data });
      setKlinePeriod(period);
      return;
    }
    setKlinePeriod(period);
    getStockKline(stockCode, period, limit);
  }, [stockCode, getStockKline, setKlinePeriod]);

  // ── 自适应 zoom 处理 ──
  // 使用 ref 持久化 handler，只在 init 时绑定一次，避免 setOption 每次重建
  const buildZoomHandler = useCallback((datesLength: number) => {
    return (params: unknown) => {
      const p = params as { start?: number; end?: number; batch?: unknown };
      if (zoomingRef.current) { return; }
      const batch = Array.isArray(p) ? p : [p];
      const b = batch[0];
      if (!b || b.batch != null) { return; }
      const start = b.start ?? 0;
      const end = b.end ?? 100;
      const visible = Math.round(datesLength * (end - start) / 100);
      const bestTier = findTier(visible);
      const curTier = lastTierRef.current;
      if (!curTier || curTier.key !== bestTier.key) {
        zoomingRef.current = true;
        lastTierRef.current = bestTier;
        switchPeriod(bestTier.period, bestTier.limit);
        requestAnimationFrame(() => {
          zoomingRef.current = false;
        });
      }
    };
  }, [switchPeriod]);

  // ── ECharts 初始化 + 一次性的 zoom handler 绑定 ──
  useEffect(() => {
    const el = chartRef.current;
    if (!el) { return; }
    const tryInit = () => {
      if (instanceRef.current && !instanceRef.current.isDisposed()) { return; }
      if (el.clientWidth === 0 || el.clientHeight === 0) { return; }
      const existing = instanceRef.current;
      if (existing && !existing.isDisposed()) { existing.dispose(); }
      instanceRef.current = echarts.init(el, undefined, { renderer: "canvas" });
      // init 时安装一次性的 zoom handler，注销旧 handler
      const chart = instanceRef.current;
      const oldHandler = zoomHandlerRef.current;
      if (oldHandler) { chart.off("dataZoom", oldHandler); }
      // 先用 placeholder，等 klineData 就绪后再建真实 handler
    };
    const onResize = () => {
      const c = instanceRef.current;
      if (c && !c.isDisposed()) { c.resize(); }
    };
    tryInit();
    const ro = new ResizeObserver(() => {
      onResize();
      tryInit();
    });
    ro.observe(el);
    const onVisibility = () => {
      if (document.visibilityState === "visible") {
        const c = instanceRef.current;
        if (c && !c.isDisposed()) { requestAnimationFrame(() => c.resize()); }
        else { tryInit(); }
      }
    };
    document.addEventListener("visibilitychange", onVisibility);
    window.addEventListener("resize", onResize);
    return () => {
      ro.disconnect();
      window.removeEventListener("resize", onResize);
      document.removeEventListener("visibilitychange", onVisibility);
      const c = instanceRef.current;
      if (c && !c.isDisposed()) { c.dispose(); }
      instanceRef.current = null;
      zoomHandlerRef.current = null;
    };
  }, []);

  // ── 缓存写入 ──
  useEffect(() => {
    if (klineData.length === 0 || !stockCode) { return; }
    const asOfDate = useTimeAnchorStore.getState().asOfDate;
    // 从当前数据推断最佳 tier（用 actual 数据长度）
    const tier = findTier(klineData.length);
    lastTierRef.current = tier;
    const ck = cacheKey(stockCode, klinePeriod, tier.limit, asOfDate);
    klineCache.set(ck, { data: klineData, ts: Date.now() });
    if (klineCache.size > KLINE_CACHE_MAX) {
      const firstKey = klineCache.keys().next().value;
      if (firstKey) { klineCache.delete(firstKey); }
    }
  }, [klineData, stockCode, klinePeriod]);

  // ── 时间旅行清缓存 ──
  useEffect(() => {
    return useTimeAnchorStore.subscribe((s, prev) => {
      if (s.asOfDate !== prev.asOfDate || s.mode !== prev.mode) {
        klineCache.clear();
        if (stockCode) {
          const tier = lastTierRef.current ?? TIERS[4]; // default daily
          switchPeriod(tier.period, tier.limit);
        }
      }
    });
  }, [stockCode, switchPeriod]);

  // ── 初始加载 ──
  useEffect(() => {
    if (!stockCode || klineData.length > 0) { return; }
    const initTier = TIERS[4]; // daily as initial
    lastTierRef.current = initTier;
    switchPeriod(initTier.period, initTier.limit);
  }, [stockCode, klineData.length, switchPeriod]);

  // ── setOption ──
  useEffect(() => {
    const chart = getChart();
    if (!chart || chart.isDisposed()) { return; }
    if (klineData.length === 0) {
      chart.clear();
      return;
    }

    const dates = klineData.map((k) => k.date);
    const ohlc = klineData.map((k) => [k.open, k.close, k.low, k.high] as [number, number, number, number]);
    const volumes = klineData.map((k) => k.volume);
    const closes = klineData.map((k) => k.close);

    const ma5 = indicators.ma5 ? calcMA(closes, 5) : [];
    const ma10 = indicators.ma10 ? calcMA(closes, 10) : [];
    const ma20 = indicators.ma20 ? calcMA(closes, 20) : [];

    // 财报事件 markPoint
    const earningsPoints: Array<
      { name: string; coord: [string, number]; value: string; itemStyle: { color: string } }
    > = [];
    if (showEarningsOnChart && earningsEvents.length > 0) {
      const dateIdx = new Map(dates.map((d, i) => [d, i]));
      for (const ev of earningsEvents) {
        const idx = dateIdx.get(ev.eventDate);
        if (idx == null) { continue; }
        earningsPoints.push({
          name: ev.eventType,
          coord: [ev.eventDate, klineData[idx].high],
          value: ev.period ?? ev.eventType,
          itemStyle: { color: EARNINGS_COLORS[ev.eventType] ?? EARNINGS_COLORS.other },
        });
      }
    }

    // 尝试保持 ~80 根 K 线可见（= 16px/根在 1280px 宽下）
    const targetVisible = 80;
    const total = dates.length;
    const zoomStart = Math.max(0, Math.round((1 - targetVisible / total) * 100));
    const zoomEnd = 100;

    const candleName = t("stockAnalysis.klineChart");
    const volumeName = t("stockAnalysis.volumeChart");

    chart.setOption({
      animation: true,
      animationDuration: 300,
      tooltip: {
        trigger: "axis",
        axisPointer: { type: "cross" },
        backgroundColor: "var(--surface)",
        borderColor: "var(--border)",
        borderWidth: 1,
        textStyle: { fontSize: 12, color: "var(--fg)" },
        formatter: (params: unknown) => {
          const arr = params as Array<{ seriesName: string; value: number; dataIndex: number }>;
          if (!arr || arr.length === 0) { return ""; }
          const candle = arr.find((p) => p.seriesName === candleName);
          const v = arr.find((p) => p.seriesName === volumeName);
          const idx = candle?.dataIndex ?? 0;
          const k = ohlc[idx];
          if (!k) { return ""; }
          const raw = klineData[idx];
          const prevClose = idx > 0 ? klineData[idx - 1]?.close : null;
          const changePct = raw && prevClose ? (raw.close - prevClose) / prevClose * 100 : null;
          const lines = [
            `<div style="font-weight:600;margin-bottom:4px">${dates[idx] || ""}</div>`,
            `${t("stockAnalysis.open")}: ${k[0].toFixed(2)} &nbsp; ${t("stockAnalysis.close")}: ${k[1].toFixed(2)}${
              changePct != null
                ? ` &nbsp; ${t("stockAnalysis.changePct")}: <span style="color:${
                  changePct >= 0 ? "#ef4444" : "#22c55e"
                }">${changePct >= 0 ? "+" : ""}${changePct.toFixed(2)}%</span>`
                : ""
            }`,
            `${t("stockAnalysis.high")}: ${k[3].toFixed(2)} &nbsp; ${t("stockAnalysis.low")}: ${k[2].toFixed(2)}`,
            v
              ? `${t("stockAnalysis.volumeChart")}: ${(v.value / 10000).toFixed(1)}${t("stockAnalysis.volumeUnit")}`
              : "",
          ];
          if (indicators.ma5 && ma5[idx] != null) {
            lines.push(`<span style="color:${MA_ORANGE}">MA5: ${ma5[idx]}</span>`);
          }
          if (indicators.ma10 && ma10[idx] != null) {
            lines.push(`<span style="color:${MA_BLUE}">MA10: ${ma10[idx]}</span>`);
          }
          if (indicators.ma20 && ma20[idx] != null) {
            lines.push(`<span style="color:${MA_PURPLE}">MA20: ${ma20[idx]}</span>`);
          }
          return lines.filter(Boolean).join("<br>");
        },
      },
      axisPointer: { link: [{ xAxisIndex: "all" }] },
      toolbox: {
        right: 10,
        feature: {
          dataZoom: {
            yAxisIndex: false,
            title: { zoom: t("stockAnalysis.chart.zoom"), back: t("stockAnalysis.chart.restore") },
          },
          restore: { title: t("stockAnalysis.chart.restore") },
        },
      },
      dataZoom: [
        { type: "inside", xAxisIndex: [0, 1], start: zoomStart, end: zoomEnd },
        {
          type: "slider",
          xAxisIndex: [0, 1],
          bottom: 0,
          height: 20,
          start: zoomStart,
          end: zoomEnd,
          borderColor: "var(--border)",
        },
      ],
      grid: [
        { left: "3%", right: "3%", top: 30, height: "58%" },
        { left: "3%", right: "3%", top: "72%", height: "18%" },
      ],
      xAxis: [
        { type: "category", data: dates, gridIndex: 0, axisLabel: { show: false }, boundaryGap: true },
        { type: "category", data: dates, gridIndex: 1, axisLabel: { rotate: 45, fontSize: 10 }, boundaryGap: true },
      ],
      yAxis: [
        {
          type: "value",
          gridIndex: 0,
          scale: true,
          splitArea: { show: true, areaStyle: { color: ["rgba(0,0,0,0.02)", "rgba(0,0,0,0.04)"] } },
          axisLabel: { fontSize: 10 },
        },
        { type: "value", gridIndex: 1, axisLabel: { show: false }, splitLine: { show: false } },
      ],
      series: [
        {
          name: candleName,
          type: "candlestick",
          data: ohlc,
          xAxisIndex: 0,
          yAxisIndex: 0,
          itemStyle: { color: SA_RED, color0: SA_GREEN, borderColor: SA_RED, borderColor0: SA_GREEN },
          markPoint: earningsPoints.length > 0
            ? {
              symbol: "pin",
              symbolSize: 22,
              symbolOffset: [0, -8],
              data: earningsPoints,
              label: { show: true, formatter: (p: { value?: string }) => p.value ?? "", fontSize: 9, color: "#fff" },
              z: 5,
            }
            : undefined,
        },
        ...(indicators.ma5
          ? [{
            name: "MA5",
            type: "line" as const,
            data: ma5,
            xAxisIndex: 0,
            yAxisIndex: 0,
            smooth: true,
            showSymbol: false,
            lineStyle: { width: 1.5, color: MA_ORANGE } as const,
            z: 1,
          }]
          : []),
        ...(indicators.ma10
          ? [{
            name: "MA10",
            type: "line" as const,
            data: ma10,
            xAxisIndex: 0,
            yAxisIndex: 0,
            smooth: true,
            showSymbol: false,
            lineStyle: { width: 1.5, color: MA_BLUE } as const,
            z: 1,
          }]
          : []),
        ...(indicators.ma20
          ? [{
            name: "MA20",
            type: "line" as const,
            data: ma20,
            xAxisIndex: 0,
            yAxisIndex: 0,
            smooth: true,
            showSymbol: false,
            lineStyle: { width: 1.5, color: MA_PURPLE } as const,
            z: 1,
          }]
          : []),
        {
          name: volumeName,
          type: "bar",
          data: volumes,
          xAxisIndex: 1,
          yAxisIndex: 1,
          itemStyle: {
            color: (p: { dataIndex: number }) => {
              const k = ohlc[p.dataIndex];
              return k && k[1] >= k[0] ? SA_RED : SA_GREEN;
            },
          },
        },
      ],
    });

    // 通过 ref 安装/替换 zoom handler（只绑定一次，避免 setOption 每轮重建）
    const oldHandler = zoomHandlerRef.current;
    if (oldHandler) { chart.off("dataZoom", oldHandler); }
    const newHandler = buildZoomHandler(dates.length);
    zoomHandlerRef.current = newHandler;
    chart.on("dataZoom", newHandler);
  }, [klineData, t, indicators, getChart, earningsEvents, showEarningsOnChart, buildZoomHandler]);

  const chartHeight = Math.max(300, Math.min(520, window.innerHeight * 0.38));
  const curTierInfo = useMemo(() => {
    if (klineData.length === 0) { return null; }
    return findTier(klineData.length);
  }, [klineData]);

  return (
    <div>
      {/* 顶栏：asof水印 + 层级指示器 + 复权 + MA开关 */}
      <div className="flex gap-1 mb-1 flex-wrap items-center">
        {asOfDate && (
          <span
            data-testid="kline-asof-badge"
            style={{
              display: "inline-flex",
              alignItems: "center",
              gap: 4,
              padding: "2px 8px",
              fontSize: 12,
              borderRadius: 4,
              background: "rgba(124,58,237,0.12)",
              color: "#7c3aed",
              border: "1px solid rgba(124,58,237,0.35)",
            }}
            title={t("timeTravel.replayBadge.tooltip", { date: asOfDate })}
          >
            ⏪ {t("timeTravel.pageAnchor.untilDate", { date: asOfDate })}
          </span>
        )}

        {/* 当前数据层级标签 */}
        <span
          style={{
            fontSize: 12,
            padding: "2px 8px",
            borderRadius: 4,
            background: "var(--accent-dim)",
            color: "var(--accent)",
            fontWeight: 500,
          }}
        >
          {curTierInfo ? t(`stockAnalysis.period.${curTierInfo.key}`) : "-"}
          <span style={{ marginLeft: 6, fontWeight: 400, opacity: 0.7 }}>({klineData.length})</span>
        </span>

        {/* 复权切换 */}
        <span className="ml-2" />
        {[
          { key: "auto" as const, label: t("stockAnalysis.adj.auto") },
          { key: "none" as const, label: t("stockAnalysis.adj.none") },
          { key: "forward" as const, label: t("stockAnalysis.adj.forward") },
          { key: "backward" as const, label: t("stockAnalysis.adj.backward") },
        ].map((opt) => (
          <button
            key={opt.key}
            onClick={() => {
              setKlineAdj(opt.key);
              if (stockCode) {
                for (const k of Array.from(klineCache.keys())) {
                  if (k.startsWith(`${stockCode}|`)) { klineCache.delete(k); }
                }
                const tier = lastTierRef.current ?? TIERS[4];
                getStockKline(stockCode, tier.period, tier.limit, opt.key);
              }
            }}
            style={{
              padding: "2px 8px",
              fontSize: 12,
              border: "1px solid",
              borderColor: klineAdj === opt.key ? "var(--accent)" : "var(--border)",
              borderRadius: 4,
              background: klineAdj === opt.key ? "var(--accent-bg)" : "transparent",
              color: klineAdj === opt.key ? "var(--accent)" : "var(--muted)",
              cursor: "pointer",
            }}
          >
            {opt.label}
          </button>
        ))}
      </div>

      {/* MA/财报图例开关 */}
      <div className="flex gap-3 mb-1 flex-wrap items-center text-xs" style={{ color: "var(--muted)" }}>
        {[{ k: "ma5" as const, c: MA_ORANGE, l: "MA5" }, { k: "ma10" as const, c: MA_BLUE, l: "MA10" }, {
          k: "ma20" as const,
          c: MA_PURPLE,
          l: "MA20",
        }].map((opt) => (
          <label
            key={opt.k}
            className="flex items-center gap-1 cursor-pointer select-none"
            onClick={() => toggleIndicator(opt.k)}
          >
            <input
              type="checkbox"
              checked={indicators[opt.k]}
              onChange={() => toggleIndicator(opt.k)}
              style={{ width: 12, height: 12 }}
            />
            <span style={{ color: opt.c }}>━</span> {opt.l}
          </label>
        ))}
        <label
          className="flex items-center gap-1 cursor-pointer select-none"
          onClick={() => setShowEarningsOnChart(!showEarningsOnChart)}
        >
          <input
            type="checkbox"
            checked={showEarningsOnChart}
            onChange={() => setShowEarningsOnChart(!showEarningsOnChart)}
            style={{ width: 12, height: 12 }}
          />
          <span style={{ color: EARNINGS_COLORS.formal }}>📅</span>
          {t("stockAnalysis.earningsOnChart")}
          {earningsEvents.length > 0 && <span className="text-gray-400">({earningsEvents.length})</span>}
        </label>
        {/* 粒度说明提示 */}
        <span style={{ fontSize: 11, color: "var(--muted)", opacity: 0.6 }}>
          {t("stockAnalysis.period.zoomHint")}
        </span>
      </div>

      <div ref={chartRef} style={{ width: "100%", height: chartHeight }} />
    </div>
  );
}

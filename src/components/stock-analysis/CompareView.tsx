import { invoke } from "@/lib/invoke";
import type { StockQuote } from "@/types/stock-analysis";
import { SwapOutlined } from "@ant-design/icons";
import { Button, Card, Empty, Input, Spin, Tag } from "antd";
import * as echarts from "echarts";
import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";

interface FinancialMetrics {
  pe: number | null;
  pb: number | null;
  roe: number | null;
  revenueYoy: number | null;
  profitYoy: number | null;
  grossMargin: number | null;
  debtRatio: number | null;
}

function CompareRow({ label, v1, v2, color1, color2, better }: {
  label: string;
  v1: string | number;
  v2: string | number;
  color1?: string;
  color2?: string;
  better?: "left" | "right";
}) {
  return (
    <div className="flex items-center text-xs" style={{ borderBottom: "1px solid var(--border)", padding: "4px 0" }}>
      <span className="shrink-0" style={{ width: 56, color: "var(--muted)" }}>{label}</span>
      <span className="flex-1 text-right font-mono" style={{ color: color1 }}>
        {better === "left" ? <Tag color="green" className="text-xs m-0 px-1">{v1}</Tag> : v1}
      </span>
      <span className="flex-1 text-right font-mono" style={{ color: color2 }}>
        {better === "right" ? <Tag color="green" className="text-xs m-0 px-1">{v2}</Tag> : v2}
      </span>
    </div>
  );
}

export function CompareView() {
  const { t } = useTranslation();
  const [code1, setCode1] = useState("");
  const [code2, setCode2] = useState("");
  const [quote1, setQuote1] = useState<StockQuote | null>(null);
  const [quote2, setQuote2] = useState<StockQuote | null>(null);
  const [fin1, setFin1] = useState<FinancialMetrics | null>(null);
  const [fin2, setFin2] = useState<FinancialMetrics | null>(null);
  const [loading, setLoading] = useState(false);
  const radarRef = useRef<HTMLDivElement>(null);
  const radarInstance = useRef<echarts.ECharts | null>(null);

  const compare = async () => {
    if (!code1 || !code2) { return; }
    setLoading(true);
    try {
      const [q1, q2, f1, f2] = await Promise.all([
        invoke<StockQuote>("get_stock_quote", { stockCode: code1 }),
        invoke<StockQuote>("get_stock_quote", { stockCode: code2 }),
        invoke<Record<string, unknown>[]>("get_stock_financials", { stockCode: code1 }).then(parseFin),
        invoke<Record<string, unknown>[]>("get_stock_financials", { stockCode: code2 }).then(parseFin),
      ]);
      setQuote1(q1);
      setQuote2(q2);
      setFin1(f1);
      setFin2(f2);
    } catch {
      /* 静默 */
    } finally {
      setLoading(false);
    }
  };

  const parseFin = (list: Record<string, unknown>[]): FinancialMetrics => {
    const last = list?.[list.length - 1];
    if (!last) {
      return { pe: null, pb: null, roe: null, revenueYoy: null, profitYoy: null, grossMargin: null, debtRatio: null };
    }
    return {
      pe: null,
      pb: null,
      roe: (last.roe ?? last.revenue_yoy) as number ?? null,
      revenueYoy: (last.revenueYoy ?? last.revenue_yoy) as number ?? null,
      profitYoy: (last.profitYoy ?? last.profit_yoy) as number ?? null,
      grossMargin: (last.grossMargin ?? last.gross_margin) as number ?? null,
      debtRatio: (last.debtRatio ?? last.debt_ratio) as number ?? null,
    };
  };

  // 雷达图
  useEffect(() => {
    if (!radarRef.current) { return; }
    if (radarRef.current.clientWidth === 0 || radarRef.current.clientHeight === 0) {
      return;
    }
    if (!radarInstance.current || radarInstance.current.isDisposed()) {
      radarInstance.current?.dispose();
      radarInstance.current = echarts.init(radarRef.current, undefined, { renderer: "canvas" });
    }
    const chart = radarInstance.current;
    if (!quote1 || !quote2) {
      chart.clear();
      return;
    }

    const labels = [t("stockAnalysis.price"), "PE", "PB", t("stockAnalysis.volume"), t("stockAnalysis.turnoverRate")];
    const raw1 = [quote1.price, quote1.pe ?? 0, quote1.pb ?? 0, quote1.volume / 10000, quote1.turnoverRate ?? 0];
    const raw2 = [quote2.price, quote2.pe ?? 0, quote2.pb ?? 0, quote2.volume / 10000, quote2.turnoverRate ?? 0];
    const maxes = raw1.map((v, i) => Math.max(v, raw2[i], 1));

    chart.setOption({
      animation: true,
      radar: {
        indicator: labels.map((name, i) => ({ name, max: maxes[i] })),
        center: ["50%", "55%"],
        radius: "52%",
        axisName: { color: "var(--muted)", fontSize: 10 },
        splitArea: { areaStyle: { color: ["rgba(22,119,255,0.02)", "rgba(22,119,255,0.04)"] } },
      },
      series: [{
        type: "radar",
        symbol: "circle",
        symbolSize: 4,
        data: [
          {
            value: raw1,
            name: quote1.code,
            areaStyle: { color: "oklch(60% 0.20 30 / 0.1)" },
            lineStyle: { color: "oklch(60% 0.20 30)" },
            itemStyle: { color: "oklch(60% 0.20 30)" },
          },
          {
            value: raw2,
            name: quote2.code,
            areaStyle: { color: "oklch(62% 0.18 150 / 0.1)" },
            lineStyle: { color: "oklch(62% 0.18 150)" },
            itemStyle: { color: "oklch(62% 0.18 150)" },
          },
        ],
      }],
    });

    const onResize = () => {
      if (!chart.isDisposed()) { chart.resize(); }
    };
    const ro = new ResizeObserver(onResize);
    ro.observe(radarRef.current!);
    window.addEventListener("resize", onResize);
    return () => {
      ro.disconnect();
      window.removeEventListener("resize", onResize);
      chart.dispose();
      radarInstance.current = null;
    };
  }, [quote1, quote2, t]);

  return (
    <Card
      size="small"
      title={
        <span>
          <SwapOutlined /> {t("stockAnalysis.compare")}
        </span>
      }
      styles={{ body: { padding: "8px 10px" } }}
    >
      <div className="flex gap-1 mb-2 flex-wrap">
        <Input
          placeholder={t("stockAnalysis.stockCode1")}
          value={code1}
          onChange={(e) => setCode1(e.target.value)}
          style={{ width: 80 }}
          size="small"
        />
        <Input
          placeholder={t("stockAnalysis.stockCode2")}
          value={code2}
          onChange={(e) => setCode2(e.target.value)}
          style={{ width: 80 }}
          size="small"
        />
        <Button size="small" onClick={compare} disabled={!code1 || !code2} loading={loading}>
          {t("stockAnalysis.compareBtn")}
        </Button>
      </div>

      {loading && <Spin size="small" />}

      {!loading && !quote1 && !quote2 && (
        <Empty
          image={Empty.PRESENTED_IMAGE_SIMPLE}
          description={t("stockAnalysis.settings.screener.discoverHint")}
          style={{ marginTop: 16 }}
        />
      )}

      {quote1 && quote2 && (
        <>
          {/* 行情对比 */}
          <div className="flex items-center text-xs font-semibold mb-1" style={{ padding: "2px 0" }}>
            <span style={{ width: 56 }} />
            <span className="flex-1 text-center">
              {quote1.name}
              <br />
              {quote1.code}
            </span>
            <span className="flex-1 text-center">
              {quote2.name}
              <br />
              {quote2.code}
            </span>
          </div>
          <CompareRow label={t("stockAnalysis.price")} v1={quote1.price.toFixed(2)} v2={quote2.price.toFixed(2)} />
          <CompareRow
            label={t("stockAnalysis.change")}
            v1={`${quote1.changePct.toFixed(2)}%`}
            v2={`${quote2.changePct.toFixed(2)}%`}
            color1={quote1.changePct >= 0 ? "var(--sa-red)" : "var(--sa-green)"}
            color2={quote2.changePct >= 0 ? "var(--sa-red)" : "var(--sa-green)"}
            better={quote1.changePct > quote2.changePct ? "left" : "right"}
          />
          <CompareRow
            label="PE"
            v1={quote1.pe ?? "—"}
            v2={quote2.pe ?? "—"}
            better={quote1.pe && quote2.pe ? (quote1.pe < quote2.pe ? "left" : "right") : undefined}
          />
          <CompareRow label="PB" v1={quote1.pb ?? "—"} v2={quote2.pb ?? "—"} />
          <CompareRow label={t("stockAnalysis.marketCap")} v1={fmtMv(quote1.totalMv)} v2={fmtMv(quote2.totalMv)} />
          <CompareRow
            label={t("stockAnalysis.turnoverRate")}
            v1={`${(quote1.turnoverRate ?? 0).toFixed(1)}%`}
            v2={`${(quote2.turnoverRate ?? 0).toFixed(1)}%`}
          />

          {/* 财务对比 */}
          {fin1 && fin2 && (
            <>
              <div className="text-xs font-semibold mt-2 mb-1" style={{ color: "var(--muted)" }}>财务指标</div>
              <CompareRow
                label="ROE"
                v1={fmtPct(fin1.roe)}
                v2={fmtPct(fin2.roe)}
                better={compareFin(fin1.roe, fin2.roe, true)}
              />
              <CompareRow
                label="营收增速"
                v1={fmtPct(fin1.revenueYoy)}
                v2={fmtPct(fin2.revenueYoy)}
                better={compareFin(fin1.revenueYoy, fin2.revenueYoy, true)}
              />
              <CompareRow
                label="利润增速"
                v1={fmtPct(fin1.profitYoy)}
                v2={fmtPct(fin2.profitYoy)}
                better={compareFin(fin1.profitYoy, fin2.profitYoy, true)}
              />
              <CompareRow
                label="毛利率"
                v1={fmtPct(fin1.grossMargin)}
                v2={fmtPct(fin2.grossMargin)}
                better={compareFin(fin1.grossMargin, fin2.grossMargin, true)}
              />
              <CompareRow
                label="负债率"
                v1={fmtPct(fin1.debtRatio)}
                v2={fmtPct(fin2.debtRatio)}
                better={compareFin(fin1.debtRatio, fin2.debtRatio, false)}
              />
            </>
          )}

          <div ref={radarRef} style={{ width: "100%", height: 180, marginTop: 8 }} />
        </>
      )}
    </Card>
  );
}

function fmtMv(v?: number | null): string {
  if (!v || v === 0) { return "—"; }
  if (v >= 1e8) { return `${(v / 1e8).toFixed(0)}亿`; }
  return `${(v / 1e4).toFixed(0)}万`;
}

function fmtPct(v?: number | null): string {
  if (v == null) { return "—"; }
  return `${v.toFixed(1)}%`;
}

function compareFin(a: number | null, b: number | null, higherIsBetter: boolean): "left" | "right" | undefined {
  if (a == null || b == null) { return undefined; }
  if (higherIsBetter) { return a > b ? "left" : "right"; }
  return a < b ? "left" : "right";
}

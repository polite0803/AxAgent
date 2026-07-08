import { invoke } from "@/lib/invoke";
import { ReloadOutlined } from "@ant-design/icons";
import { Button, Card, Table, Tag, Typography } from "antd";
import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router-dom";

interface PositionSummary {
  stockCode: string;
  stockName: string;
  totalShares: number;
  avgCost: number;
  currentPrice: number | null;
  marketValue: number | null;
  unrealizedPnl: number | null;
  unrealizedPnlPct: number | null;
}

interface RecentAnalysis {
  stockCode: string;
  stockName: string;
  decisionAction: string | null;
  analysisDate: string;
  status: string;
}

interface MarketRegimeInfo {
  regime: string;
  confidence: number;
  volatility: string;
  description: string;
}

/** 市场状态对应的颜色 */
function regimeColor(regime: string): string {
  if (regime === "bull") { return "var(--sa-red)"; }
  if (regime === "bear") { return "var(--sa-green)"; }
  return "var(--color-text-secondary)";
}

/** 市场状态对应的标签 */
function regimeLabel(regime: string, t: (key: string) => string): string {
  if (regime === "bull") { return t("stockAnalysis.dashboard.bull"); }
  if (regime === "bear") { return t("stockAnalysis.dashboard.bear"); }
  return t("stockAnalysis.dashboard.sideways");
}

export function InvestDashboard() {
  const { t } = useTranslation();
  const navigate = useNavigate();

  const [positions, setPositions] = useState<PositionSummary[]>([]);
  const [recentAnalyses, setRecentAnalyses] = useState<RecentAnalysis[]>([]);
  const [marketRegime, setMarketRegime] = useState<MarketRegimeInfo | null>(null);
  const [loading, setLoading] = useState(false);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const [pos, anl, regime] = await Promise.all([
        invoke<PositionSummary[]>("get_trade_positions").catch(() => []),
        invoke<RecentAnalysis[]>("get_recent_analyses", { limit: 5 }).catch(() => []),
        loadMarketRegime(),
      ]);
      if (Array.isArray(pos)) { setPositions(pos); }
      if (Array.isArray(anl)) { setRecentAnalyses(anl); }
      if (regime) { setMarketRegime(regime); }
    } catch { /* silent */ }
    setLoading(false);
  }, []);

  useEffect(() => {
    Promise.all([
      invoke<PositionSummary[]>("get_trade_positions").catch(() => []),
      invoke<RecentAnalysis[]>("get_recent_analyses", { limit: 5 }).catch(() => []),
      loadMarketRegime().catch(() => null),
    ]).then(([pos, anl, regime]) => {
      if (Array.isArray(pos)) { setPositions(pos); }
      if (Array.isArray(anl)) { setRecentAnalyses(anl); }
      if (regime) { setMarketRegime(regime); }
    }).finally(() => setLoading(false));
  }, []);

  const totalMv = positions.reduce((s, p) => s + (p.marketValue ?? 0), 0);
  const totalPnl = positions.reduce((s, p) => s + (p.unrealizedPnl ?? 0), 0);

  return (
    <div className="flex flex-col gap-2 p-3" style={{ maxWidth: 640, margin: "0 auto" }}>
      {/* Header */}
      <div className="flex items-center justify-between">
        <Typography.Title level={5} style={{ margin: 0, fontSize: 15, fontWeight: 500 }}>
          {t("stockAnalysis.dashboard.title")}
        </Typography.Title>
        <Button size="small" icon={<ReloadOutlined />} loading={loading} onClick={load} />
      </div>

      {/* Market Regime */}
      {marketRegime && (
        <Card size="small" styles={{ body: { padding: "8px 12px" } }}>
          <div className="flex items-center gap-2 text-xs">
            <span className="text-gray-500">{t("stockAnalysis.dashboard.marketState")}:</span>
            <Tag color={regimeColor(marketRegime.regime)} className="m-0 text-xs">
              {regimeLabel(marketRegime.regime, t)}
            </Tag>
            <span style={{ color: "var(--color-text-secondary)" }}>
              ({t("stockAnalysis.dashboard.confidence")}: {(marketRegime.confidence * 100).toFixed(0)}%)
            </span>
            <span className="text-gray-400">
              {marketRegime.volatility === "high" ? t("stockAnalysis.dashboard.highVol") : ""}
            </span>
          </div>
          <div className="text-xs mt-1" style={{ color: "var(--color-text-tertiary)" }}>
            {marketRegime.description}
          </div>
        </Card>
      )}

      {/* Positions */}
      {positions.length > 0 && (
        <Card
          size="small"
          title={<span className="text-xs font-medium">{t("stockAnalysis.holdings")} ({positions.length})</span>}
          styles={{ body: { padding: "4px 8px" } }}
          extra={
            <Button size="small" type="link" className="text-xs" onClick={() => navigate("/trade")}>
              {t("stockAnalysis.dashboard.viewAll")}
            </Button>
          }
        >
          <div className="text-xs flex gap-3 mb-1">
            <span>
              {t("stockAnalysis.totalMarketValue")}: <b>{(totalMv / 10000).toFixed(1)}{t("stockAnalysis.wanUnit")}</b>
            </span>
            <span style={{ color: totalPnl >= 0 ? "var(--sa-red)" : "var(--sa-green)" }}>
              {t("stockAnalysis.unrealizedPnl")}: {totalPnl >= 0 ? "+" : ""}
              {totalPnl.toFixed(0)}
            </span>
          </div>
          <Table
            size="small"
            dataSource={positions}
            rowKey="stockCode"
            pagination={false}
            showHeader={false}
            columns={[
              { dataIndex: "stockCode", width: 60, render: (v: string) => <Tag className="m-0 text-[10px]">{v}</Tag> },
              { dataIndex: "totalShares", width: 44, render: (v: number) => `${v.toFixed(0)}股` },
              { dataIndex: "avgCost", width: 52, render: (v: number) => v.toFixed(2) },
              {
                dataIndex: "unrealizedPnlPct",
                width: 56,
                render: (v: number | null) =>
                  v != null
                    ? (
                      <span style={{ color: v >= 0 ? "var(--sa-red)" : "var(--sa-green)" }}>
                        {v >= 0 ? "+" : ""}
                        {v.toFixed(1)}%
                      </span>
                    )
                    : <span className="text-gray-400">—</span>,
              },
            ]}
            onRow={(record) => ({
              style: { cursor: "pointer" },
              onClick: () => navigate(`/stock-analysis?code=${record.stockCode}`),
            })}
          />
        </Card>
      )}

      {/* Quick Actions */}
      <Card size="small" styles={{ body: { padding: "8px 12px" } }}>
        <div className="flex gap-2 flex-wrap">
          <Button size="small" type="primary" onClick={() => navigate("/screener")}>
            {t("stockAnalysis.dashboard.recommendations")}
          </Button>
          <Button size="small" onClick={() => navigate("/stock-analysis")}>
            {t("stockAnalysis.dashboard.analyzeStock")}
          </Button>
          <Button size="small" onClick={() => navigate("/trade")}>
            {t("stockAnalysis.dashboard.recordTrade")}
          </Button>
          <Button size="small" onClick={() => navigate("/watchlist")}>
            {t("stockAnalysis.dashboard.dailyReview")}
          </Button>
        </div>
      </Card>

      {/* Recent Analyses */}
      {recentAnalyses.length > 0 && (
        <Card
          size="small"
          title={<span className="text-xs font-medium">{t("stockAnalysis.dashboard.recentAnalyses")}</span>}
          styles={{ body: { padding: "4px 8px" } }}
        >
          <Table
            size="small"
            dataSource={recentAnalyses}
            rowKey={(r) => `${r.stockCode}-${r.analysisDate}`}
            pagination={false}
            showHeader={false}
            columns={[
              { dataIndex: "stockCode", width: 60, render: (v: string) => <Tag className="m-0 text-[10px]">{v}</Tag> },
              { dataIndex: "stockName", width: 64, render: (v: string) => <span className="text-xs">{v}</span> },
              {
                dataIndex: "decisionAction",
                width: 50,
                render: (v: string | null) =>
                  v
                    ? (
                      <Tag className="m-0 text-[10px]" color={v === "BUY" ? "red" : v === "SELL" ? "green" : "blue"}>
                        {v}
                      </Tag>
                    )
                    : <span className="text-xs text-gray-400">—</span>,
              },
              {
                dataIndex: "analysisDate",
                width: 72,
                render: (v: string) => <span className="text-xs text-gray-400">{v}</span>,
              },
            ]}
            onRow={(record) => ({
              style: { cursor: "pointer" },
              onClick: () => navigate(`/stock-analysis?code=${record.stockCode}`),
            })}
          />
        </Card>
      )}
    </div>
  );
}

/** 拉取 CSI300 近 60 日 K 线 → 判断市场状态 */
async function loadMarketRegime(): Promise<MarketRegimeInfo | null> {
  try {
    const klines = await invoke<Record<string, unknown>[]>("get_market_klines", {
      code: "000300",
      period: "daily",
      limit: 60,
    });
    if (!Array.isArray(klines) || klines.length < 20) { return null; }
    // 调用市场的 classify_regime — 简单在客户端用收盘价数组判断
    const closes = klines.map((k) => {
      const v = k.close;
      if (typeof v === "number") { return v; }
      if (v != null) { return parseFloat(String(v)); }
      return 0;
    });
    const ma20 = closes.slice(-20).reduce((a: number, b: number) => a + b, 0) / 20;
    const ma60 = closes.length >= 60
      ? closes.slice(-60).reduce((a: number, b: number) => a + b, 0) / 60
      : ma20;
    const last = closes[closes.length - 1];
    const pctAbove60 = ma60 > 0 ? (last - ma60) / ma60 : 0;
    const slope = closes.length >= 10
      ? (closes.slice(-5).reduce((a: number, b: number) => a + b, 0) / 5
        - closes.slice(-10, -5).reduce((a: number, b: number) => a + b, 0) / 5)
        / (closes.slice(-5).reduce((a: number, b: number) => a + b, 0) / 5)
      : 0;

    if (pctAbove60 > 0.05 && slope > 0.01) {
      return {
        regime: "bull",
        confidence: Math.min(pctAbove60 * 2, 0.95),
        volatility: "normal",
        description: `站上60日均线${(pctAbove60 * 100).toFixed(1)}%`,
      };
    }
    if (pctAbove60 < -0.03 && slope < -0.005) {
      return {
        regime: "bear",
        confidence: Math.min(Math.abs(pctAbove60) * 2, 0.95),
        volatility: "normal",
        description: `跌破60日均线${(Math.abs(pctAbove60) * 100).toFixed(1)}%`,
      };
    }
    return { regime: "sideways", confidence: 0.5, volatility: "normal", description: "均线交叉/粘合，方向不明确" };
  } catch {
    return null;
  }
}

import { invoke } from "@/lib/invoke";
import { ReloadOutlined } from "@ant-design/icons";
import { Button, Card, Tag } from "antd";
import { useCallback, useEffect, useState } from "react";

interface HoldingDaysBucket {
  label: string;
  count: number;
  winCount: number;
  totalPnl: number;
}

interface MonthlyPnl {
  yearMonth: string;
  realizedPnl: number;
  tradeCount: number;
  winCount: number;
}

interface StrategyBreakdown {
  strategy: string;
  tradeCount: number;
  totalPnl: number;
  winCount: number;
  winRate: number;
}

interface TradeStatsSummary {
  totalBuys: number;
  totalSells: number;
  totalFeesEst: number;
  totalStampTax: number;
  totalRealizedPnl: number;
  winCount: number;
  lossCount: number;
  winRate: number;
  avgWin: number;
  avgLoss: number;
  profitFactor: number;
  holdingDaysDist: HoldingDaysBucket[];
  avgHoldingDays: number;
  monthlyPnl: MonthlyPnl[];
  strategyBreakdown: StrategyBreakdown[];
}

export function TradeStatsPanel() {
  const [loading, setLoading] = useState(false);
  const [stats, setStats] = useState<TradeStatsSummary | null>(null);

  const loadStats = useCallback(async () => {
    setLoading(true);
    try {
      const result = await invoke<TradeStatsSummary>("get_trade_stats");
      setStats(result);
    } catch {
      // silent
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    let cancelled = false;
    invoke<TradeStatsSummary | null>("get_trade_stats")
      .then((data) => {
        if (!cancelled) { setStats(data); }
      })
      .catch(() => {})
      .finally(() => {
        if (!cancelled) { setLoading(false); }
      });
    return () => {
      cancelled = true;
    };
  }, []);

  if (!stats || stats.totalBuys === 0) { return null; }

  const profitFactorColor = (pf: number) => {
    if (pf >= 2) { return "var(--sa-red)"; }
    if (pf >= 1) { return "#fa8c16"; }
    return "var(--sa-green)";
  };

  return (
    <Card
      size="small"
      title={
        <div className="flex justify-between items-center">
          <span>统计分析</span>
          <Button size="small" icon={<ReloadOutlined />} loading={loading} onClick={loadStats} />
        </div>
      }
      styles={{ body: { padding: "8px 10px", maxHeight: 400, overflowY: "auto" } }}
    >
      {/* 概览行 */}
      <div className="grid grid-cols-4 gap-2 mb-2">
        <div className="text-xs p-1 rounded" style={{ background: "var(--surface)" }}>
          <span style={{ color: "var(--muted)" }}>总盈亏</span>
          <div style={{ color: stats.totalRealizedPnl >= 0 ? "var(--sa-red)" : "var(--sa-green)", fontWeight: "bold" }}>
            {stats.totalRealizedPnl >= 0 ? "+" : ""}
            {stats.totalRealizedPnl.toFixed(0)}
          </div>
        </div>
        <div className="text-xs p-1 rounded" style={{ background: "var(--surface)" }}>
          <span style={{ color: "var(--muted)" }}>胜率</span>
          <div style={{ fontWeight: "bold" }}>
            {stats.winRate.toFixed(1)}%<span className="text-xs" style={{ color: "var(--muted)" }}>
              ({stats.winCount}/{stats.totalSells})
            </span>
          </div>
        </div>
        <div className="text-xs p-1 rounded" style={{ background: "var(--surface)" }}>
          <span style={{ color: "var(--muted)" }}>盈亏比</span>
          <div style={{ color: profitFactorColor(stats.profitFactor), fontWeight: "bold" }}>
            {stats.profitFactor > 10 ? "∞" : stats.profitFactor.toFixed(2)}
          </div>
        </div>
        <div className="text-xs p-1 rounded" style={{ background: "var(--surface)" }}>
          <span style={{ color: "var(--muted)" }}>平均持有</span>
          <div style={{ fontWeight: "bold" }}>{stats.avgHoldingDays.toFixed(1)} 天</div>
        </div>
      </div>

      {/* 税费 */}
      <div className="text-xs mb-2 p-1 rounded" style={{ background: "var(--surface)" }}>
        <span style={{ color: "var(--muted)" }}>税费估算</span>
        <span style={{ color: "var(--sa-green)" }}>印花税 ¥{stats.totalStampTax.toFixed(0)}</span>
        <span className="mx-1" style={{ color: "var(--muted)" }}>|</span>
        <span style={{ color: "var(--muted)" }}>佣金 ¥{stats.totalFeesEst.toFixed(0)}</span>
        <span className="mx-1" style={{ color: "var(--muted)" }}>|</span>
        <span>交易 {stats.totalBuys + stats.totalSells} 笔 (买 {stats.totalBuys} / 卖 {stats.totalSells})</span>
      </div>

      {/* 持有期分布 */}
      {stats.holdingDaysDist.filter(h => h.count > 0).length > 0 && (
        <div className="mb-2">
          <div className="text-xs mb-1" style={{ color: "var(--muted)" }}>持有期分布</div>
          <div className="flex gap-1 flex-wrap">
            {stats.holdingDaysDist.filter(h => h.count > 0).map(h => (
              <div key={h.label} className="text-xs p-1 rounded" style={{ background: "var(--surface)", minWidth: 60 }}>
                <div>{h.label}</div>
                <b>{h.count} 笔</b>
                <div style={{ color: h.totalPnl >= 0 ? "var(--sa-red)" : "var(--sa-green)", fontSize: 9 }}>
                  {h.totalPnl >= 0 ? "+" : ""}
                  {h.totalPnl.toFixed(0)}
                </div>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* 策略分组 */}
      {stats.strategyBreakdown.length > 0 && (
        <div className="mb-2">
          <div className="text-xs mb-1" style={{ color: "var(--muted)" }}>按策略</div>
          <div className="flex gap-1 flex-wrap">
            {stats.strategyBreakdown.map(s => (
              <div
                key={s.strategy}
                className="text-xs p-1 rounded"
                style={{ background: "var(--surface)", minWidth: 80 }}
              >
                <Tag style={{ fontSize: 9, lineHeight: "14px", margin: 0 }}>{s.strategy}</Tag>
                <div>{s.tradeCount} 笔</div>
                <div style={{ color: s.totalPnl >= 0 ? "var(--sa-red)" : "var(--sa-green)", fontSize: 9 }}>
                  {s.totalPnl >= 0 ? "+" : ""}
                  {s.totalPnl.toFixed(0)} ({s.winRate.toFixed(0)}%)
                </div>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* 月度盈亏 */}
      {stats.monthlyPnl.length > 0 && (
        <div>
          <div className="text-xs mb-1" style={{ color: "var(--muted)" }}>月度盈亏</div>
          <div className="flex gap-1 flex-wrap">
            {stats.monthlyPnl.slice(-12).map(m => (
              <div
                key={m.yearMonth}
                className="text-xs p-1 rounded"
                style={{
                  background: m.realizedPnl >= 0 ? "rgba(255,77,79,0.08)" : "rgba(82,196,26,0.08)",
                  minWidth: 56,
                  textAlign: "center",
                }}
              >
                <div style={{ fontSize: 9, color: "var(--muted)" }}>{m.yearMonth.slice(5)}</div>
                <div style={{ color: m.realizedPnl >= 0 ? "var(--sa-red)" : "var(--sa-green)", fontWeight: "bold" }}>
                  {m.realizedPnl >= 0 ? "+" : ""}
                  {m.realizedPnl.toFixed(0)}
                </div>
                <div style={{ fontSize: 8, color: "var(--muted)" }}>{m.winCount}/{m.tradeCount}</div>
              </div>
            ))}
          </div>
        </div>
      )}
    </Card>
  );
}

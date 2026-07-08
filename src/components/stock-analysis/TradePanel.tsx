import { invoke } from "@/lib/invoke";
import { getActionColor, StockAction } from "@/lib/stock-analysis-utils";
import { useStockAnalysisStore } from "@/stores";
import { PlusOutlined, ReloadOutlined } from "@ant-design/icons";
import { App, Button, Card, Input, InputNumber, Select, Space, Statistic, Switch, Table, Tag } from "antd";
import dayjs from "dayjs";
import { useCallback, useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

interface TradeRecord {
  id: string;
  stockCode: string;
  stockName: string;
  direction: string;
  price: number;
  quantity: number;
  tradeDate: string;
  tradeTime: string;
  realizedPnl: number | null;
  notes: string | null;
  createdAt: number;
}

interface PositionSummary {
  stockCode: string;
  stockName: string;
  totalShares: number;
  avgCost: number;
  currentPrice: number | null;
  marketValue: number | null;
  unrealizedPnl: number | null;
  unrealizedPnlPct: number | null;
  totalRealizedPnl: number;
}

export function TradePanel() {
  const { message } = App.useApp();
  const { t } = useTranslation();
  const storeStockCode = useStockAnalysisStore((s) => s.stockCode);
  const storeStockName = useStockAnalysisStore((s) => s.stockName);
  const storeDecision = useStockAnalysisStore((s) => s.decision);
  // R2: 买入前 position_limits 校验
  const checkPositionLimits = useStockAnalysisStore((s) => s.checkPositionLimits);
  const [enabled, setEnabled] = useState(false);
  const [trades, setTrades] = useState<TradeRecord[]>([]);
  const [positions, setPositions] = useState<PositionSummary[]>([]);
  const [loading, setLoading] = useState(false);
  const [form, setForm] = useState({
    stockCode: "",
    stockName: "",
    direction: "buy" as string,
    price: 0,
    quantity: 100,
    notes: "",
  });

  const loadData = async () => {
    setLoading(true);
    try {
      const [t, p] = await Promise.all([
        invoke<TradeRecord[]>("list_trades", { stockCode: null, limit: 100 }),
        invoke<PositionSummary[]>("get_trade_positions"),
      ]);
      if (Array.isArray(t)) { setTrades(t); }
      if (Array.isArray(p)) { setPositions(p); }
    } catch {
      /* 静默 */
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    if (!enabled) { return; }
    let cancelled = false;
    Promise.resolve().then(() => {
      if (cancelled) { return; }
      setLoading(true);
      return Promise.all([
        invoke<TradeRecord[]>("list_trades", { stockCode: null, limit: 100 }),
        invoke<PositionSummary[]>("get_trade_positions"),
      ]);
    })
      .then((result) => {
        if (!result || cancelled) { return; }
        const [t, p] = result;
        if (Array.isArray(t)) { setTrades(t); }
        if (Array.isArray(p)) { setPositions(p); }
      })
      .catch(() => {})
      .finally(() => {
        if (!cancelled) { setLoading(false); }
      });
    return () => {
      cancelled = true;
    };
  }, [enabled]);

  // 同步分析页代码
  useEffect(() => {
    if (storeStockCode && !form.stockCode) {
      const dir = storeDecision?.action === StockAction.SELL ? "sell" : "buy";
      Promise.resolve().then(() => {
        setForm((f) => ({ ...f, stockCode: storeStockCode, stockName: storeStockName, direction: dir }));
      });
    }
  }, [storeStockCode, storeStockName, storeDecision, form.stockCode]);

  // 一键从分析结论录入
  const quickRecord = useCallback(() => {
    if (!storeDecision) { return; }
    const decisionData = storeDecision as unknown as Record<string, unknown>;
    const { action, positionPct, targetPrice, stopLoss } = decisionData;
    if (!action || (action as string) === StockAction.HOLD || (action as string) === StockAction.REDUCE) { return; }
    setForm((f) => ({
      ...f,
      stockCode: storeStockCode,
      stockName: storeStockName,
      direction: action === StockAction.SELL ? "sell" : "buy",
      price: (targetPrice as number) || 0,
      quantity: (positionPct as number) ? Math.round(((positionPct as number) / 100) * 1000) : 100,
      notes: stopLoss ? t("stockAnalysis.trade.stopLoss", { price: stopLoss }) : "",
    }));
  }, [storeDecision, storeStockCode, storeStockName, t]);

  const handleRecord = async () => {
    if (!form.stockCode || form.price <= 0) { return message.warning(t("trade.fillRequired")); }
    // R2: 买入前先做 position_limits 校验
    if (form.direction === "buy") {
      const check = await checkPositionLimits(
        form.stockCode,
        form.quantity,
        form.price,
      );
      if (check && !check.ok) {
        return message.warning(`${t("trade.limitsBlocked")}: ${check.reason ?? ""}`);
      }
    }
    const now = dayjs();
    const analysisId = useStockAnalysisStore.getState().analysisId;
    try {
      await invoke("record_trade", {
        stockCode: form.stockCode,
        stockName: form.stockName,
        direction: form.direction,
        price: form.price,
        quantity: form.quantity,
        tradeDate: now.format("YYYY-MM-DD"),
        tradeTime: now.format("HH:mm"),
        notes: form.notes || null,
        analysisId: analysisId ?? null,
      });
      message.success(t("trade.recorded"));
      loadData();
    } catch (e) {
      message.error(String(e));
    }
  };

  const handleToggle = async (val: boolean) => {
    try {
      await invoke("toggle_trading_enabled", { enabled: val });
      setEnabled(val);
    } catch {
      message.error(t("trade.toggleFailed"));
    }
  };

  // 绩效统计
  const stats = useMemo(() => {
    const sells = trades.filter((t) => t.direction === "sell" && t.realizedPnl != null);
    const winCount = sells.filter((t) => (t.realizedPnl ?? 0) > 0).length;
    const winRate = sells.length > 0 ? ((winCount / sells.length) * 100).toFixed(0) : "—";
    const totalPnl = trades.reduce((s, t) => s + (t.realizedPnl ?? 0), 0);
    const maxDd = computeMaxDrawdown(trades.filter((t) => t.direction === "sell"));
    return { totalTrades: trades.length, sells: sells.length, winRate, totalPnl, maxDd };
  }, [trades]);

  // 持仓汇总
  const totalMv = positions.reduce((s, p) => s + (p.marketValue ?? 0), 0);
  const totalPnl = positions.reduce((s, p) => s + (p.unrealizedPnl ?? 0), 0);

  const positionColumns = [
    { title: t("trade.stockCode"), dataIndex: "stockCode", width: 56 },
    { title: t("trade.shares"), dataIndex: "totalShares", width: 44, render: (v: number) => v.toFixed(0) },
    { title: t("trade.cost"), dataIndex: "avgCost", width: 50, render: (v: number) => v.toFixed(2) },
    {
      title: t("stockAnalysis.trade.pnlPercent"),
      dataIndex: "unrealizedPnlPct",
      width: 50,
      render: (v: number | null) =>
        v != null ? <span style={{ color: v >= 0 ? "var(--sa-red)" : "var(--sa-green)" }}>{v.toFixed(1)}%</span> : "-",
    },
    {
      title: t("trade.pnl"),
      dataIndex: "unrealizedPnl",
      width: 50,
      render: (v: number | null) =>
        v != null ? <span style={{ color: v >= 0 ? "var(--sa-red)" : "var(--sa-green)" }}>{v.toFixed(0)}</span> : "-",
    },
  ];

  const tradeColumns = [
    {
      title: "",
      dataIndex: "direction",
      width: 24,
      render: (v: string) => (
        <Tag color={v === "buy" ? "green" : "red"} style={{ fontSize: 10, lineHeight: "16px", padding: "0 4px" }}>
          {v === "buy" ? t("stockAnalysis.buyShort") : t("stockAnalysis.sellShort")}
        </Tag>
      ),
    },
    { title: t("trade.stockCode"), dataIndex: "stockCode", width: 54 },
    { title: t("trade.price"), dataIndex: "price", width: 50, render: (v: number) => v.toFixed(2) },
    { title: t("trade.quantity"), dataIndex: "quantity", width: 44 },
    {
      title: t("trade.pnl"),
      dataIndex: "realizedPnl",
      width: 50,
      render: (v: number | null) =>
        v != null ? <span style={{ color: v >= 0 ? "var(--sa-red)" : "var(--sa-green)" }}>{v.toFixed(0)}</span> : "-",
    },
  ];

  if (!enabled) {
    return (
      <Card size="small" title={t("trade.title")} styles={{ body: { padding: "8px 12px" } }}>
        <div className="flex items-center justify-between gap-2 text-xs" style={{ color: "var(--muted)" }}>
          <span>{t("trade.disabledHint")}</span>
          <Switch checked={enabled} onChange={handleToggle} />
        </div>
      </Card>
    );
  }

  return (
    <Card
      size="small"
      styles={{ body: { padding: "8px 10px" } }}
      title={
        <div className="flex justify-between items-center">
          <span>{t("trade.title")}</span>
          <Space size={4}>
            <Button size="small" icon={<ReloadOutlined />} loading={loading} onClick={loadData} />
            <Switch size="small" checked={enabled} onChange={handleToggle} />
          </Space>
        </div>
      }
    >
      {/* 绩效统计 */}
      {trades.length > 0 && (
        <div className="grid grid-cols-3 gap-1 mb-2 p-1 rounded" style={{ background: "var(--surface)" }}>
          <Statistic
            title={t("stockAnalysis.trade.totalTrades")}
            value={stats.totalTrades}
            valueStyle={{ fontSize: 14 }}
          />
          <Statistic
            title={t("stockAnalysis.trade.winRate")}
            value={stats.winRate}
            suffix="%"
            valueStyle={{ fontSize: 14 }}
          />
          <Statistic
            title={t("stockAnalysis.trade.totalPnl")}
            value={stats.totalPnl.toFixed(0)}
            valueStyle={{ fontSize: 14, color: stats.totalPnl >= 0 ? "var(--sa-red)" : "var(--sa-green)" }}
          />
        </div>
      )}

      {/* 快速录入 */}
      <div className="flex flex-col gap-1 mb-2">
        <div className="flex gap-1 flex-wrap">
          <Input
            size="small"
            placeholder={t("trade.stockCode")}
            value={form.stockCode}
            onChange={(e) => setForm({ ...form, stockCode: e.target.value })}
            style={{ width: 72 }}
          />
          <Input
            size="small"
            placeholder={t("trade.stockName")}
            value={form.stockName}
            onChange={(e) => setForm({ ...form, stockName: e.target.value })}
            style={{ width: 72 }}
          />
          <Select
            size="small"
            value={form.direction}
            onChange={(v) => setForm({ ...form, direction: v })}
            options={[{ value: "buy", label: t("stockAnalysis.buyShort") }, {
              value: "sell",
              label: t("stockAnalysis.sellShort"),
            }]}
            style={{ width: 50 }}
          />
        </div>
        <div className="flex gap-1">
          <InputNumber
            size="small"
            placeholder={t("trade.price")}
            value={form.price}
            onChange={(v) => setForm({ ...form, price: v || 0 })}
            style={{ width: 82 }}
            min={0}
            step={0.01}
          />
          <InputNumber
            size="small"
            placeholder={t("trade.quantity")}
            value={form.quantity}
            onChange={(v) => setForm({ ...form, quantity: v || 100 })}
            step={100}
            min={100}
            style={{ width: 82 }}
          />
          <Button size="small" type="primary" icon={<PlusOutlined />} loading={loading} onClick={handleRecord} />
        </div>
      </div>

      {/* 分析结论 → 一键录入 */}
      {storeDecision && (
        <div className="text-xs p-1 rounded mb-2" style={{ background: "var(--surface)" }}>
          <span style={{ color: "var(--muted)" }}>{t("stockAnalysis.recentAnalysis")}:</span>
          <Tag
            color={getActionColor((storeDecision as unknown as Record<string, unknown>).action as string)}
          >
            {(storeDecision as unknown as Record<string, unknown>).action as string}
          </Tag>
          <Button size="small" type="link" className="text-xs px-1" onClick={quickRecord}>
            {t("stockAnalysis.trade.quickRecord")}
          </Button>
        </div>
      )}

      {/* 持仓 */}
      {positions.length > 0 && (
        <>
          <Table size="small" dataSource={positions} rowKey="stockCode" pagination={false} columns={positionColumns} />
          <div className="text-xs grid grid-cols-2 gap-x-2 p-1 mt-1 rounded" style={{ background: "var(--surface)" }}>
            <span>
              {t("stockAnalysis.totalMarketValue")}: <b>{(totalMv / 10000).toFixed(1)}{t("stockAnalysis.wanUnit")}</b>
            </span>
            <span style={{ color: totalPnl >= 0 ? "var(--sa-red)" : "var(--sa-green)" }}>
              {t("stockAnalysis.unrealizedPnl")}: <b>{totalPnl >= 0 ? "+" : ""}{totalPnl.toFixed(0)}</b>
            </span>
            <span>
              {t("stockAnalysis.holdings")}: <b>{positions.length}{t("stockAnalysis.sharesUnit")}</b>
            </span>
          </div>
        </>
      )}

      {/* 最近交易 */}
      {trades.length > 0 && (
        <Table
          size="small"
          dataSource={trades.slice(0, 10)}
          rowKey="id"
          pagination={false}
          className="mt-1"
          columns={tradeColumns}
        />
      )}
    </Card>
  );
}

function computeMaxDrawdown(sells: TradeRecord[]): number {
  if (sells.length === 0) { return 0; }
  const pnls = sells.map((t) => t.realizedPnl ?? 0);
  let peak = 0, cum = 0, maxDd = 0;
  for (const p of pnls) {
    cum += p;
    if (cum > peak) { peak = cum; }
    const dd = peak - cum;
    if (dd > maxDd) { maxDd = dd; }
  }
  return maxDd;
}

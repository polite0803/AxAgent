import { invoke } from "@/lib/invoke";
import { ReloadOutlined } from "@ant-design/icons";
import { Button, Spin, Table, Tag } from "antd";
import { useCallback, useEffect, useMemo, useState } from "react";
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
  totalRealizedPnl: number;
  sectorName?: string | null;
}

export function PositionsMiniPanel() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const [positions, setPositions] = useState<PositionSummary[]>([]);
  const [loading, setLoading] = useState(false);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const p = await invoke<PositionSummary[]>("get_trade_positions");
      if (Array.isArray(p)) { setPositions(p); }
    } catch {
      // 静默 — 交易功能可能未启用
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    invoke<PositionSummary[]>("get_trade_positions")
      .then((p) => {
        if (Array.isArray(p)) { setPositions(p); }
      })
      .catch(() => {})
      .finally(() => setLoading(false));
  }, []);

  const totalMv = useMemo(
    () => positions.reduce((s, p) => s + (p.marketValue ?? 0), 0),
    [positions],
  );
  const totalPnl = useMemo(
    () => positions.reduce((s, p) => s + (p.unrealizedPnl ?? 0), 0),
    [positions],
  );

  const columns = [
    {
      title: t("trade.stockCode"),
      dataIndex: "stockCode",
      width: 64,
      render: (code: string) => <Tag className="m-0 text-[10px]">{code}</Tag>,
    },
    {
      title: t("trade.shares"),
      dataIndex: "totalShares",
      width: 50,
      render: (v: number) => v.toFixed(0),
    },
    {
      title: t("trade.cost"),
      dataIndex: "avgCost",
      width: 60,
      render: (v: number) => v.toFixed(2),
    },
    {
      title: t("stockAnalysis.trade.pnlPercent"),
      dataIndex: "unrealizedPnlPct",
      width: 60,
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
  ];

  if (loading) {
    return <Spin size="small" style={{ display: "block", margin: "16px auto" }} />;
  }

  if (positions.length === 0) {
    return (
      <div className="text-xs text-gray-400 text-center py-4">
        <div className="mb-1">{t("stockAnalysis.noHoldings")}</div>
        <Button size="small" type="link" onClick={() => navigate("/trade")}>
          {t("stockAnalysis.goToTrade")}
        </Button>
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-1">
      <div className="flex items-center justify-between px-1">
        <span className="text-xs text-gray-500">
          {t("stockAnalysis.holdings")}: <b>{positions.length}</b>
          {" | "}
          {t("stockAnalysis.totalMarketValue")}: <b>{(totalMv / 10000).toFixed(1)}{t("stockAnalysis.wanUnit")}</b>
          {" | "}
          <span style={{ color: totalPnl >= 0 ? "var(--sa-red)" : "var(--sa-green)" }}>
            {t("stockAnalysis.unrealizedPnl")}: {totalPnl >= 0 ? "+" : ""}
            {totalPnl.toFixed(0)}
          </span>
        </span>
        <Button size="small" icon={<ReloadOutlined />} loading={loading} onClick={load} />
      </div>
      <Table
        size="small"
        dataSource={positions}
        rowKey="stockCode"
        pagination={false}
        columns={columns}
        showHeader={false}
        onRow={(record) => ({
          style: { cursor: "pointer" },
          onClick: () => navigate(`/stock-analysis?code=${record.stockCode}`),
        })}
      />
    </div>
  );
}

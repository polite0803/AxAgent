import { invoke } from "@/lib/invoke";
import type { StrategySignalResult } from "@/types/stock-analysis";
import { Card, Empty, Select, Spin, Table, Tag, Tooltip } from "antd";
import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

interface RecoSignalTimelineProps {
  strategyId: string | null;
}

export function RecoSignalTimeline({ strategyId }: RecoSignalTimelineProps) {
  const { t } = useTranslation();
  const [signals, setSignals] = useState<StrategySignalResult[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [filterCode, setFilterCode] = useState<string>("");

  useEffect(() => {
    if (!strategyId) {
      Promise.resolve().then(() => {
        setSignals([]);
        setError(null);
      });
      return;
    }
    let cancelled = false;
    Promise.resolve().then(() => {
      if (cancelled) { return; }
      setLoading(true);
      return invoke<StrategySignalResult[]>("get_reco_signal_history", { strategyId });
    })
      .then((data) => {
        if (!cancelled) { setSignals(data ?? []); }
      })
      .catch((e) => {
        if (!cancelled) { setError(String(e)); }
      })
      .finally(() => {
        if (!cancelled) { setLoading(false); }
      });
    return () => {
      cancelled = true;
    };
  }, [strategyId]);

  // 按股票代码筛选
  const filtered = useMemo(() => {
    if (!filterCode) { return signals; }
    const q = filterCode.toUpperCase();
    return signals.filter(
      (s) => s.stockCode.toUpperCase().includes(q) || s.stockName.toUpperCase().includes(q),
    );
  }, [signals, filterCode]);

  // 去重股票列表（用于 Select 筛选）
  const stockOptions = useMemo(() => {
    const seen = new Set<string>();
    return signals
      .filter((s) => {
        if (seen.has(s.stockCode)) { return false; }
        seen.add(s.stockCode);
        return true;
      })
      .map((s) => ({ value: s.stockCode, label: `${s.stockCode} ${s.stockName}` }));
  }, [signals]);

  // 聚合统计
  const stats = useMemo(() => {
    if (filtered.length === 0) { return null; }
    const wins = filtered.filter((s) => s.wasProfitable).length;
    const total = filtered.length;
    const avgRet = filtered.reduce((a, s) => a + s.returnPct, 0) / total;
    return { wins, total, winRate: total > 0 ? (wins / total) * 100 : 0, avgRet };
  }, [filtered]);

  const columns = [
    {
      title: t("stockAnalysis.backtest.colDate") ?? "日期",
      dataIndex: "signalDate",
      key: "signalDate",
      width: 90,
      render: (v: string) => <span className="text-xs">{v}</span>,
    },
    {
      title: t("stockAnalysis.backtest.colCode") ?? "代码",
      dataIndex: "stockCode",
      key: "stockCode",
      width: 80,
      render: (v: string) => <span className="text-xs font-medium">{v}</span>,
    },
    {
      title: t("stockAnalysis.backtest.colName") ?? "名称",
      dataIndex: "stockName",
      key: "stockName",
      width: 80,
      render: (v: string) => <span className="text-xs">{v}</span>,
    },
    {
      title: t("stockAnalysis.backtest.colEntry") ?? "入场",
      dataIndex: "entryPrice",
      key: "entryPrice",
      width: 80,
      align: "right" as const,
      render: (v: number) => <span className="text-xs font-mono">{v.toFixed(2)}</span>,
    },
    {
      title: t("stockAnalysis.backtest.colExit") ?? "出场",
      dataIndex: "exitPrice",
      key: "exitPrice",
      width: 80,
      align: "right" as const,
      render: (v: number) => <span className="text-xs font-mono">{v.toFixed(2)}</span>,
    },
    {
      title: t("stockAnalysis.backtest.colReturn") ?? "收益",
      dataIndex: "returnPct",
      key: "returnPct",
      width: 70,
      align: "right" as const,
      render: (v: number) => {
        const color = v >= 0 ? "var(--sa-red)" : "var(--sa-green)";
        return <span className="text-xs font-bold" style={{ color }}>{v >= 0 ? "+" : ""}{v.toFixed(2)}%</span>;
      },
      sorter: (a: StrategySignalResult, b: StrategySignalResult) => a.returnPct - b.returnPct,
    },
    {
      title: t("stockAnalysis.backtest.colMaxDrawdown") ?? "最大回撤",
      dataIndex: "maxDrawdownPct",
      key: "maxDrawdownPct",
      width: 70,
      align: "right" as const,
      render: (v: number) => <span className="text-xs" style={{ color: "var(--sa-green)" }}>{v.toFixed(1)}%</span>,
    },
    {
      title: t("stockAnalysis.backtest.colHolding") ?? "持有",
      dataIndex: "holdingDays",
      key: "holdingDays",
      width: 50,
      align: "right" as const,
      render: (v: number) => <span className="text-xs">{v}d</span>,
    },
    {
      title: "",
      dataIndex: "wasProfitable",
      key: "wasProfitable",
      width: 40,
      render: (v: boolean) => (
        <Tag className="m-0 text-[10px]" color={v ? "green" : "red"}>
          {v ? "W" : "L"}
        </Tag>
      ),
    },
  ];

  return (
    <Card
      size="small"
      title={
        <div className="flex items-center gap-2">
          <span className="text-sm font-medium">
            {t("stockAnalysis.backtest.signalHistory") ?? "信号时间线"}
          </span>
          {strategyId && <Tag className="m-0 text-[10px]">{strategyId}</Tag>}
        </div>
      }
      extra={
        <div className="flex items-center gap-2">
          <Select
            size="small"
            allowClear
            placeholder={t("stockAnalysis.backtest.filterStock") ?? "筛选股票"}
            style={{ width: 160 }}
            options={stockOptions}
            onChange={(v) => setFilterCode(v ?? "")}
            value={filterCode || undefined}
          />
          {filtered.length > 0 && signals.length > 0 && filtered.length < signals.length && (
            <span className="text-[10px] text-gray-500">{filtered.length}/{signals.length}</span>
          )}
        </div>
      }
      styles={{ body: { padding: "8px 10px" } }}
    >
      {!strategyId
        ? (
          <Empty
            image={Empty.PRESENTED_IMAGE_SIMPLE}
            description={t("stockAnalysis.backtest.signalSelectHint") ?? "请在策略矩阵中点击一个策略查看信号明细"}
          />
        )
        : loading
        ? <Spin size="small" style={{ display: "block", margin: "24px auto" }} />
        : error
        ? <div className="text-xs text-red-500 text-center py-4">{error}</div>
        : filtered.length === 0
        ? (
          <Empty
            image={Empty.PRESENTED_IMAGE_SIMPLE}
            description={t("stockAnalysis.backtest.signalEmpty") ?? "该策略暂无历史信号"}
          />
        )
        : (
          <>
            {/* 聚合摘要 */}
            {stats && (
              <div className="flex items-center gap-3 mb-3 text-xs">
                <span>
                  {t("stockAnalysis.backtest.total") ?? "总计"}: <strong>{stats.total}</strong>
                </span>
                <span>
                  {t("stockAnalysis.backtest.colWinRate") ?? "胜率"}:{" "}
                  <strong style={{ color: stats.winRate >= 55 ? "var(--sa-red)" : "var(--sa-green)" }}>
                    {stats.winRate.toFixed(1)}%
                  </strong>
                </span>
                <span>
                  {t("stockAnalysis.backtest.colAvgReturn") ?? "平均收益"}:{" "}
                  <strong style={{ color: stats.avgRet >= 0 ? "var(--sa-red)" : "var(--sa-green)" }}>
                    {stats.avgRet >= 0 ? "+" : ""}
                    {stats.avgRet.toFixed(2)}%
                  </strong>
                </span>
                <Tooltip
                  title={t("stockAnalysis.backtest.winsAndLosses")
                    ?? `胜 ${stats.wins} / 负 ${stats.total - stats.wins}`}
                >
                  <span>
                    <Tag color="green" className="m-0 text-[10px]">W {stats.wins}</Tag>
                    <Tag color="red" className="m-0 text-[10px]">L {stats.total - stats.wins}</Tag>
                  </span>
                </Tooltip>
              </div>
            )}

            {/* 信号表格 */}
            <Table
              dataSource={filtered}
              columns={columns}
              rowKey={(r) => `${r.stockCode}-${r.signalDate}-${r.entryPrice}`}
              pagination={{
                size: "small",
                pageSize: 20,
                showSizeChanger: true,
                pageSizeOptions: ["10", "20", "50"],
              }}
              size="small"
              bordered={false}
              scroll={{ x: 640 }}
              className="signal-history-table"
            />
          </>
        )}
    </Card>
  );
}

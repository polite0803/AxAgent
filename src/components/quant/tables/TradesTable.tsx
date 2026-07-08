// TradesTable — 量化回测成交明细表格
//
// 数据源:BacktestResult.trades (QuantPaperTrade[])
// 顶部:6 张汇总卡(总笔数/买/卖/总成交额/总手续费/总已实现盈亏)
// 中部:过滤(方向/关键字) + 分页
// 下部:Antd Table,列=时间/代码/方向/价格/数量/成交额/总费用/已实现盈亏

import { Card, Input, Select, Space, Table, Tag } from "antd";
import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

import type { QuantPaperTrade, TradeSide } from "@/types";
import type { ColumnsType } from "antd/es/table";

interface TradesTableProps {
  trades: QuantPaperTrade[];
}

export function TradesTable({ trades }: TradesTableProps) {
  const { t } = useTranslation();
  const [sideFilter, setSideFilter] = useState<"all" | TradeSide>("all");
  const [keyword, setKeyword] = useState("");

  // ── 汇总指标 ──
  const summary = useMemo(() => {
    let longCount = 0;
    let flatCount = 0;
    let shortCount = 0;
    let totalAmount = 0;
    let totalFee = 0;
    let totalPnl = 0;
    for (const tr of trades) {
      if (tr.side === "long") { longCount++; }
      else if (tr.side === "flat") { flatCount++; }
      else { shortCount++; }
      totalAmount += tr.amount;
      totalFee += tr.commission + tr.stampTax + tr.slippage;
      totalPnl += tr.realizedPnl;
    }
    return { totalCount: trades.length, longCount, flatCount, shortCount, totalAmount, totalFee, totalPnl };
  }, [trades]);

  // ── 过滤后数据 ──
  const filtered = useMemo(() => {
    const kw = keyword.trim().toLowerCase();
    if (sideFilter === "all" && kw === "") { return trades; }
    return trades.filter((tr) => {
      if (sideFilter !== "all" && tr.side !== sideFilter) { return false; }
      if (kw && !tr.code.toLowerCase().includes(kw)) { return false; }
      return true;
    });
  }, [trades, sideFilter, keyword]);

  // ── 表格列 ──
  const columns: ColumnsType<QuantPaperTrade> = [
    {
      title: t("quant.trades.time"),
      dataIndex: "timestamp",
      key: "timestamp",
      width: 160,
      defaultSortOrder: "ascend",
      sorter: (a, b) => a.timestamp.localeCompare(b.timestamp),
    },
    {
      title: t("quant.trades.code"),
      dataIndex: "code",
      key: "code",
      width: 90,
    },
    {
      title: t("quant.trades.side"),
      dataIndex: "side",
      key: "side",
      width: 80,
      render: (side: TradeSide) => {
        if (side === "long") { return <Tag color="red">{t("quant.trades.long")}</Tag>; }
        if (side === "flat") { return <Tag color="green">{t("quant.trades.flat")}</Tag>; }
        return <Tag color="purple">{t("quant.trades.short")}</Tag>;
      },
    },
    {
      title: t("quant.trades.price"),
      dataIndex: "price",
      key: "price",
      align: "right",
      width: 90,
      render: (v: number) => v.toFixed(2),
    },
    {
      title: t("quant.trades.quantity"),
      dataIndex: "quantity",
      key: "quantity",
      align: "right",
      width: 100,
      render: (v: number) => v.toLocaleString("zh-CN"),
    },
    {
      title: t("quant.trades.amount"),
      dataIndex: "amount",
      key: "amount",
      align: "right",
      width: 130,
      render: (v: number) => v.toFixed(2),
    },
    {
      title: t("quant.trades.fee"),
      key: "fee",
      align: "right",
      width: 110,
      render: (_: unknown, tr) => (tr.commission + tr.stampTax + tr.slippage).toFixed(2),
    },
    {
      title: t("quant.trades.realizedPnl"),
      dataIndex: "realizedPnl",
      key: "realizedPnl",
      align: "right",
      width: 130,
      render: (v: number) => {
        if (v === 0) { return <span style={{ color: "#999" }}>0.00</span>; }
        const color = v > 0 ? "#cf1322" : "#389e0d";
        return <span style={{ color, fontWeight: 500 }}>{v.toFixed(2)}</span>;
      },
    },
    {
      title: t("quant.trades.reason"),
      dataIndex: "reason",
      key: "reason",
      ellipsis: true,
      render: (v: string | null) => v ?? "—",
    },
  ];

  if (trades.length === 0) {
    return (
      <Card>
        <span style={{ color: "#999" }}>{t("quant.trades.empty")}</span>
      </Card>
    );
  }

  return (
    <Space direction="vertical" size="middle" style={{ width: "100%" }}>
      <div
        style={{
          display: "grid",
          gap: 12,
          gridTemplateColumns: "repeat(auto-fit, minmax(140px, 1fr))",
        }}
      >
        <SummaryCard label={t("quant.trades.totalCount")} value={summary.totalCount.toString()} />
        <SummaryCard
          label={t("quant.trades.longCount")}
          value={summary.longCount.toString()}
          color="#cf1322"
        />
        <SummaryCard
          label={t("quant.trades.flatCount")}
          value={summary.flatCount.toString()}
          color="#389e0d"
        />
        {summary.shortCount > 0 && (
          <SummaryCard
            label={t("quant.trades.shortCount")}
            value={summary.shortCount.toString()}
            color="#722ed1"
          />
        )}
        <SummaryCard
          label={t("quant.trades.totalAmount")}
          value={summary.totalAmount.toFixed(2)}
        />
        <SummaryCard
          label={t("quant.trades.totalFee")}
          value={summary.totalFee.toFixed(2)}
        />
        <SummaryCard
          label={t("quant.trades.totalPnl")}
          value={summary.totalPnl.toFixed(2)}
          color={summary.totalPnl > 0 ? "#cf1322" : summary.totalPnl < 0 ? "#389e0d" : "#666"}
        />
      </div>

      <Space>
        <Select
          value={sideFilter}
          onChange={setSideFilter}
          style={{ width: 120 }}
          options={[
            { value: "all", label: t("quant.trades.filterAll") },
            { value: "long", label: t("quant.trades.long") },
            { value: "flat", label: t("quant.trades.flat") },
            { value: "short", label: t("quant.trades.short") },
          ]}
        />
        <Input.Search
          placeholder={t("quant.trades.searchPlaceholder")}
          value={keyword}
          onChange={(e) => setKeyword(e.target.value)}
          allowClear
          style={{ width: 200 }}
        />
      </Space>

      <Table<QuantPaperTrade>
        size="small"
        columns={columns}
        dataSource={filtered}
        rowKey="id"
        pagination={{
          pageSize: 20,
          showSizeChanger: true,
          pageSizeOptions: [10, 20, 50, 100],
          showTotal: (total) => t("quant.trades.totalRows", { total }),
        }}
        scroll={{ x: 900 }}
      />
    </Space>
  );
}

function SummaryCard({
  label,
  value,
  color,
}: {
  label: string;
  value: string;
  color?: string;
}) {
  return (
    <Card size="small" styles={{ body: { padding: "8px 12px" } }}>
      <div style={{ fontSize: 12, color: "#666" }}>{label}</div>
      <div style={{ fontSize: 18, fontWeight: 500, color: color ?? "#222" }}>
        {value}
      </div>
    </Card>
  );
}

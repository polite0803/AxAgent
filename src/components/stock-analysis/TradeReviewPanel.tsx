// i18n-exempt: 业务逻辑/API 描述/日志字符串，非 UI 展示文本
import { invoke } from "@/lib/invoke";
import { ReloadOutlined } from "@ant-design/icons";
import { Button, Card, Table, Tag } from "antd";
import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

interface TradeReviewItem {
  stockCode: string;
  stockName: string;
  entryDate: string;
  exitDate: string;
  holdingDays: number;
  entryPrice: number;
  exitPrice: number;
  pnlPct: number;
  pnlAmount: number;
  analysisTarget: number | null;
  analysisStop: number | null;
  targetDeviationPct: number | null;
  grade: string;
  comment: string;
}

interface TradeReviewSummary {
  totalClosed: number;
  items: TradeReviewItem[];
  winRate: number;
  totalPnl: number;
  avgGrade: string;
  suggestions: string[];
}

function gradeColor(g: string): string {
  switch (g) {
    case "优秀":
      return "green";
    case "良好":
      return "blue";
    case "及格":
      return "gold";
    default:
      return "red";
  }
}

export function TradeReviewPanel() {
  const [loading, setLoading] = useState(false);
  const [review, setReview] = useState<TradeReviewSummary | null>(null);
  const { t } = useTranslation();

  const loadReview = useCallback(async () => {
    setLoading(true);
    try {
      const r = await invoke<TradeReviewSummary>("get_trade_review");
      setReview(r);
    } catch {
      // silent
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    let cancelled = false;
    invoke<TradeReviewSummary | null>("get_trade_review")
      .then((data) => {
        if (!cancelled) { setReview(data); }
      })
      .catch(() => {})
      .finally(() => {
        if (!cancelled) { setLoading(false); }
      });
    return () => {
      cancelled = true;
    };
  }, []);

  if (!review || review.totalClosed === 0) { return null; }

  const columns = [
    { title: t("stockAnalysis.tradeReview.colCode"), dataIndex: "stockCode", width: 56 },
    { title: t("stockAnalysis.tradeReview.colEntry"), dataIndex: "entryDate", width: 72 },
    { title: t("stockAnalysis.tradeReview.colExit"), dataIndex: "exitDate", width: 72 },
    {
      title: t("stockAnalysis.tradeReview.colHolding"),
      dataIndex: "holdingDays",
      width: 36,
      render: (v: number) => `${v}d`,
    },
    {
      title: t("stockAnalysis.tradeReview.colEntryPrice"),
      dataIndex: "entryPrice",
      width: 56,
      render: (v: number) => v.toFixed(2),
    },
    {
      title: t("stockAnalysis.tradeReview.colExitPrice"),
      dataIndex: "exitPrice",
      width: 56,
      render: (v: number) => v.toFixed(2),
    },
    {
      title: t("stockAnalysis.tradeReview.colPnl"),
      dataIndex: "pnlPct",
      width: 50,
      render: (v: number) => (
        <span style={{ color: v >= 0 ? "var(--sa-red)" : "var(--sa-green)" }}>
          {v >= 0 ? "+" : ""}
          {v.toFixed(1)}%
        </span>
      ),
    },
    {
      title: t("stockAnalysis.tradeReview.colGrade"),
      dataIndex: "grade",
      width: 48,
      render: (v: string) => <Tag color={gradeColor(v)} style={{ fontSize: 10 }}>{v}</Tag>,
    },
  ];

  return (
    <Card
      size="small"
      title={
        <div className="flex justify-between items-center">
          <span>{t("stockAnalysis.tradeReview.title")}</span>
          <Button size="small" icon={<ReloadOutlined />} loading={loading} onClick={loadReview} />
        </div>
      }
      styles={{ body: { padding: "8px 10px", maxHeight: 320, overflowY: "auto" } }}
    >
      {/* 评分摘要 */}
      <div className="grid grid-cols-4 gap-1 mb-2 text-xs">
        <div className="p-1 rounded" style={{ background: "var(--surface)" }}>
          {t("stockAnalysis.tradeReview.closed")}
          <div style={{ fontWeight: "bold" }}>{review.totalClosed}</div>
        </div>
        <div className="p-1 rounded" style={{ background: "var(--surface)" }}>
          {t("stockAnalysis.tradeReview.winRate")}
          <div style={{ fontWeight: "bold" }}>{review.winRate.toFixed(0)}%</div>
        </div>
        <div className="p-1 rounded" style={{ background: "var(--surface)" }}>
          {t("stockAnalysis.tradeReview.totalPnl")}
          <div style={{ color: review.totalPnl >= 0 ? "var(--sa-red)" : "var(--sa-green)", fontWeight: "bold" }}>
            {review.totalPnl >= 0 ? "+" : ""}
            {review.totalPnl.toFixed(0)}
          </div>
        </div>
        <div className="p-1 rounded" style={{ background: "var(--surface)" }}>
          {t("stockAnalysis.tradeReview.avgGrade")}
          <div>
            <Tag color={gradeColor(review.avgGrade)}>{review.avgGrade}</Tag>
          </div>
        </div>
      </div>

      {/* 改进建议 */}
      <div className="text-xs mb-2 p-1 rounded" style={{ background: "var(--surface)" }}>
        {review.suggestions.map((s, i) => (
          <div
            key={i}
            style={{ color: s.includes("优秀") || s.includes("良好") ? "var(--sa-red)" : "var(--sa-green)" }}
          >
            • {s}
          </div>
        ))}
      </div>

      <Table
        size="small"
        dataSource={review.items.slice(0, 20)}
        rowKey={(r) => `${r.stockCode}-${r.exitDate}`}
        pagination={false}
        columns={columns}
      />
    </Card>
  );
}

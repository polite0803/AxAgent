import { invoke } from "@/lib/invoke";
import { ReloadOutlined } from "@ant-design/icons";
import { Button, Card, Space, Tag, Tooltip } from "antd";
import { useCallback, useState } from "react";

// ── 类型定义 ──

interface ExitSignal {
  signalType: string;
  severity: string;
  detail: string;
}

interface ExitRecommendation {
  stockCode: string;
  stockName: string;
  shares: number;
  avgCost: number;
  currentPrice: number;
  pnlPct: number;
  pnlAmount: number;
  positionPct: number;
  exitScore: number;
  action: "SELL_NOW" | "SELL_AT_LIMIT" | "SET_STOP_LOSS" | "HOLD" | "CONSIDER_ADD";
  suggestedPrice: number | null;
  timeframe: string;
  holdingDays: number;
  signals: ExitSignal[];
  reasoning: string;
}

interface ExitSummary {
  totalPositions: number;
  urgentExits: number;
  limitExits: number;
  stopLossNeeded: number;
  holds: number;
  recommendations: ExitRecommendation[];
}

function actionLabel(action: string): string {
  switch (action) {
    case "SELL_NOW":
      return "立即卖出";
    case "SELL_AT_LIMIT":
      return "挂限价卖出";
    case "SET_STOP_LOSS":
      return "设置止损";
    case "HOLD":
      return "继续持有";
    case "CONSIDER_ADD":
      return "考虑加仓";
    default:
      return action;
  }
}

function actionColor(action: string): string {
  switch (action) {
    case "SELL_NOW":
      return "var(--sa-green)";
    case "SELL_AT_LIMIT":
      return "#fa8c16";
    case "SET_STOP_LOSS":
      return "#faad14";
    case "HOLD":
      return "var(--sa-blue)";
    case "CONSIDER_ADD":
      return "var(--sa-red)";
    default:
      return "var(--muted)";
  }
}

function severityColor(severity: string): string {
  switch (severity) {
    case "critical":
      return "red";
    case "high":
      return "orange";
    case "medium":
      return "gold";
    case "low":
      return "blue";
    default:
      return "default";
  }
}

export function ExitRecommendationPanel() {
  const [loading, setLoading] = useState(false);
  const [summary, setSummary] = useState<ExitSummary | null>(null);

  const loadRecommendations = useCallback(async () => {
    setLoading(true);
    try {
      const result = await invoke<ExitSummary>("get_exit_recommendations");
      setSummary(result);
    } catch {
      // 无声失败
    } finally {
      setLoading(false);
    }
  }, []);

  if (!summary || summary.totalPositions === 0) {
    return null;
  }

  return (
    <Card
      size="small"
      title={
        <div className="flex justify-between items-center">
          <span>退出建议</span>
          <Space size={4}>
            <span className="text-xs" style={{ color: "var(--muted)" }}>
              {summary.urgentExits > 0 && <span style={{ color: "red" }}>⚠ {summary.urgentExits} 紧急</span>}
              {summary.limitExits > 0 && <span style={{ color: "#fa8c16" }}>⏰ {summary.limitExits} 限价</span>}
              {summary.stopLossNeeded > 0 && <span style={{ color: "#faad14" }}>🛡 {summary.stopLossNeeded} 止损</span>}
            </span>
            <Button size="small" icon={<ReloadOutlined />} loading={loading} onClick={loadRecommendations} />
          </Space>
        </div>
      }
      styles={{ body: { padding: "8px 10px", maxHeight: 360, overflowY: "auto" } }}
    >
      {summary.recommendations.slice(0, 10).map((rec) => <RecommendationCard key={rec.stockCode} rec={rec} />)}
    </Card>
  );
}

function RecommendationCard({ rec }: { rec: ExitRecommendation }) {
  const pnlColor = rec.pnlPct >= 0 ? "var(--sa-red)" : "var(--sa-green)";
  const scoreColor = rec.exitScore >= 40 ? "red" : rec.exitScore >= 20 ? "#fa8c16" : "#52c41a";
  const scoreBg = rec.exitScore >= 40 ? "#fff0f0" : rec.exitScore >= 20 ? "#fffbe6" : "#f6ffed";

  return (
    <div
      className="mb-1 p-2 rounded text-xs"
      style={{ background: scoreBg, borderLeft: `3px solid ${scoreColor}` }}
    >
      {/* 标题行 */}
      <div className="flex justify-between items-center mb-1">
        <Space size={6}>
          <b>{rec.stockCode}</b>
          <span style={{ color: "var(--muted)" }}>{rec.stockName}</span>
          <Tag color={actionColor(rec.action)} style={{ fontSize: 10, lineHeight: "16px", margin: 0 }}>
            {actionLabel(rec.action)}
          </Tag>
        </Space>
        <Space size={4}>
          <span style={{ fontWeight: "bold", color: scoreColor }}>{rec.exitScore.toFixed(0)}</span>
          <span style={{ color: "var(--muted)", fontSize: 10 }}>/100</span>
        </Space>
      </div>

      {/* 价格/盈亏行 */}
      <div className="flex gap-3 mb-1" style={{ color: "var(--muted)" }}>
        <span>成本 {rec.avgCost.toFixed(2)}</span>
        <span>
          现价 <b>{rec.currentPrice.toFixed(2)}</b>
        </span>
        <span style={{ color: pnlColor }}>
          盈亏 <b>{rec.pnlPct >= 0 ? "+" : ""}{rec.pnlPct.toFixed(1)}%</b>
        </span>
        <span>持有 {rec.holdingDays} 天</span>
        <span>仓位 {rec.positionPct.toFixed(0)}%</span>
        {rec.suggestedPrice && (
          <span style={{ color: "#fa8c16" }}>
            建议挂出 <b>{rec.suggestedPrice.toFixed(2)}</b>
          </span>
        )}
        <Tag style={{ fontSize: 10, lineHeight: "16px" }}>{rec.timeframe}</Tag>
      </div>

      {/* 信号标签 */}
      <div className="flex gap-1 flex-wrap">
        {rec.signals.slice(0, 4).map((sig, i) => (
          <Tooltip key={i} title={sig.detail}>
            <Tag color={severityColor(sig.severity)} style={{ fontSize: 9, lineHeight: "14px", cursor: "pointer" }}>
              {sig.signalType}
            </Tag>
          </Tooltip>
        ))}
        {rec.signals.length > 4 && <span style={{ color: "var(--muted)", fontSize: 9 }}>+{rec.signals.length - 4}
        </span>}
      </div>
    </div>
  );
}

import { getActionColor } from "@/lib/stock-analysis-utils";
import { useStockAnalysisStore } from "@/stores";
import { Button, Progress, Tag, theme } from "antd";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router-dom";

/** 后端返回的分析动作常量（用于比较，不做 UI 展示） */

/**
 * ChatView 中嵌入的股票分析状态指示器
 *
 * 当股票分析在后台运行时（通过 input 中的 /analyze 或 @code 触发），
 * 在对话消息区和输入框之间显示实时进度。
 * 完成后显示决策摘要和"查看详情"按钮，可关闭。
 */
export function StockAnalysisChatIndicator() {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const navigate = useNavigate();

  const status = useStockAnalysisStore((s) => s.status);
  const stockCode = useStockAnalysisStore((s) => s.stockCode);
  const stockName = useStockAnalysisStore((s) => s.stockName);
  const progressMessage = useStockAnalysisStore((s) => s.progressMessage);
  const progressPct = useStockAnalysisStore((s) => s.progressPct);
  const decision = useStockAnalysisStore((s) => s.decision);
  const error = useStockAnalysisStore((s) => s.error);
  const chatIndicatorDismissed = useStockAnalysisStore((s) => s.chatIndicatorDismissed);
  const dismissChatIndicator = useStockAnalysisStore((s) => s.dismissChatIndicator);

  // 空闲或已关闭 → 不显示
  if (status === "idle" || chatIndicatorDismissed) {
    return null;
  }

  const handleViewDetails = () => {
    navigate(`/stock-analysis?code=${stockCode}`);
  };

  const handleRetry = () => {
    useStockAnalysisStore.getState().startAnalysis(stockCode);
  };

  return (
    <div
      style={{
        display: "flex",
        alignItems: "center",
        gap: 8,
        padding: "6px 24px",
        fontSize: 13,
        color: token.colorTextSecondary,
        borderBottom: `1px solid ${token.colorBorderSecondary}`,
        backgroundColor: token.colorFillAlter,
      }}
    >
      {/* 运行中/加载中 */}
      {(status === "loading" || status === "running") && (
        <>
          <span className="inline-block w-2 h-2 rounded-full bg-blue-500 animate-pulse" />
          <span style={{ flex: 1, minWidth: 0, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
            {stockName && (
              <Tag variant="filled" color="blue" style={{ marginRight: 4, fontSize: 12 }}>
                {stockName}
              </Tag>
            )}
            {progressMessage || t("stockAnalysis.analyzing")}
          </span>
          {progressPct > 0 && (
            <div style={{ width: 120 }}>
              <Progress percent={progressPct} size="small" showInfo={false} strokeColor={token.colorPrimary} />
            </div>
          )}
        </>
      )}

      {/* 完成 */}
      {status === "completed" && decision && (
        <>
          <span style={{ color: token.colorSuccess }}>✅</span>
          <span style={{ flex: 1, minWidth: 0 }}>
            <Tag variant="filled" color="success" style={{ marginRight: 4, fontSize: 12 }}>
              {stockName || stockCode}
            </Tag>
            {t("stockAnalysis.completed")}
            {" · "}
            <Tag
              variant="filled"
              color={getActionColor(decision.action)}
              style={{ fontSize: 12 }}
            >
              {decision.action}
            </Tag>
            {decision.confidence > 0 && (
              <span style={{ marginLeft: 4, fontSize: 12 }}>
                {t("stockAnalysis.confidence")} {decision.confidence.toFixed(0)}%
              </span>
            )}
            {decision.targetPrice && (
              <span style={{ marginLeft: 4, fontSize: 12 }}>
                {t("stockAnalysis.targetPrice")} ¥{decision.targetPrice.toFixed(2)}
              </span>
            )}
            {decision.stopLoss && (
              <span style={{ marginLeft: 4, fontSize: 12 }}>
                {t("stockAnalysis.stopLoss")} ¥{decision.stopLoss.toFixed(2)}
              </span>
            )}
          </span>
          <Button type="link" size="small" onClick={handleViewDetails} style={{ padding: "0 4px", fontSize: 12 }}>
            {t("stockAnalysis.viewDetails")}
          </Button>
        </>
      )}

      {/* 完成但无决策数据 */}
      {status === "completed" && !decision && (
        <>
          <span style={{ color: token.colorSuccess }}>✅</span>
          <span style={{ flex: 1 }}>
            <Tag variant="filled" color="success" style={{ marginRight: 4, fontSize: 12 }}>
              {stockName || stockCode}
            </Tag>
            {t("stockAnalysis.completed")}
          </span>
          <Button type="link" size="small" onClick={handleViewDetails} style={{ padding: "0 4px", fontSize: 12 }}>
            {t("stockAnalysis.viewDetails")}
          </Button>
        </>
      )}

      {/* 错误 */}
      {status === "error" && (
        <>
          <span style={{ color: token.colorError }}>❌</span>
          <span style={{ flex: 1, minWidth: 0, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
            <Tag variant="filled" color="error" style={{ marginRight: 4, fontSize: 12 }}>
              {stockName || stockCode}
            </Tag>
            {error || t("stockAnalysis.error")}
          </span>
          <Button type="link" size="small" onClick={handleRetry} style={{ padding: "0 4px", fontSize: 12 }}>
            {t("stockAnalysis.retry")}
          </Button>
        </>
      )}

      {/* 关闭按钮（对所有非 idle 状态显示） */}
      <button
        type="button"
        onClick={dismissChatIndicator}
        style={{
          border: "none",
          background: "none",
          cursor: "pointer",
          color: token.colorTextQuaternary,
          fontSize: 14,
          lineHeight: 1,
          padding: "0 2px",
        }}
        title={t("common.close")}
      >
        ✕
      </button>
    </div>
  );
}

import { useExecutionStore } from "@/stores/feature/executionStore";
import { theme, Tooltip } from "antd";
import { Bot, GitBranch, History } from "lucide-react";
import { useTranslation } from "react-i18next";

interface AgentPoolSummary {
  total: number;
  completed: number;
  running: number;
  failed: number;
  pct: number;
}

interface AgentExecutionPanelCompactProps {
  conversationId: string;
  summary: AgentPoolSummary | null;
  onExpand?: () => void;
}

export function AgentExecutionPanelCompact({
  conversationId,
  summary,
  onExpand,
}: AgentExecutionPanelCompactProps) {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const currentToolCall = useExecutionStore((s) => s.currentToolCall);

  return (
    <div
      style={{
        height: "100%",
        display: "flex",
        flexDirection: "column",
        alignItems: "center",
        padding: "40px 4px 8px",
        gap: 8,
        overflow: "hidden",
      }}
    >
      {/* 池状态 — 点击展开 */}
      <Tooltip
        title={summary
          ? t("chat.agentPanel.poolTooltip", {
            total: summary.total,
            completed: summary.completed,
            running: summary.running,
            failed: summary.failed,
          })
          : t("chat.agentPanel.noActiveTasks")}
        placement="left"
      >
        <div
          onClick={onExpand}
          style={{
            width: 32,
            height: 32,
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            borderRadius: 8,
            backgroundColor: token.colorFillQuaternary,
            position: "relative",
            cursor: "pointer",
          }}
        >
          <Bot size={16} style={{ color: token.colorTextSecondary }} />
          {summary && summary.running > 0 && (
            <span
              style={{
                position: "absolute",
                top: -2,
                right: -2,
                width: 14,
                height: 14,
                borderRadius: "50%",
                backgroundColor: token.colorPrimary,
                fontSize: 9,
                color: "#fff",
                display: "flex",
                alignItems: "center",
                justifyContent: "center",
                fontWeight: 600,
              }}
            >
              {summary.running}
            </span>
          )}
        </div>
      </Tooltip>

      {/* 进度指示 */}
      {summary && (
        <div
          style={{
            width: 4,
            flex: 1,
            maxHeight: 80,
            borderRadius: 2,
            backgroundColor: token.colorFillSecondary,
            overflow: "hidden",
            position: "relative",
          }}
        >
          <div
            style={{
              position: "absolute",
              bottom: 0,
              width: "100%",
              height: `${Math.max(summary.pct, 5)}%`,
              backgroundColor: summary.failed > 0 ? token.colorWarning : token.colorSuccess,
              transition: "height 0.3s",
              borderRadius: 2,
            }}
          />
        </div>
      )}

      {/* 当前工具 */}
      {currentToolCall?.conversationId === conversationId && currentToolCall?.toolName && (
        <Tooltip title={t("chat.agentPanel.currentTool", { name: currentToolCall.toolName })} placement="left">
          <div
            style={{
              writingMode: "vertical-rl",
              fontSize: 9,
              color: token.colorPrimary,
              maxHeight: 100,
              overflow: "hidden",
              textOverflow: "ellipsis",
              whiteSpace: "nowrap",
            }}
          >
            {currentToolCall.toolName}
          </div>
        </Tooltip>
      )}

      {/* 时间线图标 */}
      <div
        style={{
          width: 28,
          height: 28,
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
          borderRadius: 6,
          backgroundColor: token.colorFillQuaternary,
          opacity: 0.5,
        }}
      >
        <GitBranch size={14} style={{ color: token.colorTextQuaternary }} />
      </div>

      {/* 回放图标 */}
      <div
        style={{
          width: 28,
          height: 28,
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
          borderRadius: 6,
          backgroundColor: token.colorFillQuaternary,
          opacity: 0.5,
        }}
      >
        <History size={14} style={{ color: token.colorTextQuaternary }} />
      </div>
    </div>
  );
}

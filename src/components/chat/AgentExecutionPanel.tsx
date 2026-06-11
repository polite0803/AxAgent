// SPDX-License-Identifier: AGPL-3.0-only

import { useExecutionStore } from "@/stores/feature/executionStore";
import { Tabs, theme } from "antd";
import { Bot, GitBranch, History } from "lucide-react";
import { useMemo } from "react";
import { useTranslation } from "react-i18next";
import { AgentPoolPanel } from "./AgentPoolPanel";
import { ExecutionTimeline } from "./ExecutionTimeline";
import { TrajectoryReplay } from "./TrajectoryReplay";
import "./AgentExecutionPanel.css";
import { Tooltip } from "@/components/layout/Tooltip";

interface AgentExecutionPanelProps {
  conversationId: string;
  compactMode?: boolean;
  onToggleCompact?: () => void;
}

const _EMPTY_POOL: never[] = [];

export function AgentExecutionPanel({
  conversationId,
  compactMode = false,
  onToggleCompact,
}: AgentExecutionPanelProps) {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const poolItems = useExecutionStore(
    (s) => s.agentPool[conversationId] || _EMPTY_POOL,
  );
  const currentToolCall = useExecutionStore((s) => s.currentToolCall);

  const poolSummary = useMemo(() => {
    if (poolItems.length === 0) {
      return null;
    }
    const completed = poolItems.filter((i) => i.status === "completed").length;
    const running = poolItems.filter((i) => i.status === "running").length;
    const failed = poolItems.filter((i) => i.status === "failed").length;
    const pct = poolItems.length > 0
      ? Math.round((completed / poolItems.length) * 100)
      : 0;
    return { total: poolItems.length, completed, running, failed, pct };
  }, [poolItems]);

  if (compactMode) {
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
        <Tooltip
          title={poolSummary
            ? t("chat.agentPanel.poolTooltip", {
              total: poolSummary.total,
              completed: poolSummary.completed,
              running: poolSummary.running,
              failed: poolSummary.failed,
            })
            : t("chat.agentPanel.noActiveTasks")}
          placement="left"
        >
          <div
            onClick={onToggleCompact}
            role="button"
            tabIndex={0}
            onKeyDown={(e) => {
              if (e.key === "Enter" || e.key === " ") {
                e.preventDefault();
                onToggleCompact?.();
              }
            }}
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
            {poolSummary && poolSummary.running > 0 && (
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
                {poolSummary.running}
              </span>
            )}
          </div>
        </Tooltip>

        {poolSummary && (
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
                height: `${Math.max(poolSummary.pct, 5)}%`,
                backgroundColor: poolSummary.failed > 0
                  ? token.colorWarning
                  : token.colorSuccess,
                transition: "height 0.3s",
                borderRadius: 2,
              }}
            />
          </div>
        )}

        {currentToolCall?.conversationId === conversationId
          && currentToolCall?.toolName && (
          <Tooltip
            title={t("chat.agentPanel.currentTool", {
              name: currentToolCall.toolName,
            })}
            placement="left"
          >
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

  return (
    <div
      className="agent-exec-panel"
      style={{ height: "100%", display: "flex", flexDirection: "column" }}
    >
      {/* 头部摘要栏 */}
      {poolSummary && (
        <div
          className="agent-exec-panel__summary"
          style={{
            display: "flex",
            alignItems: "center",
            gap: 10,
            padding: "8px 12px",
            borderBottom: `1px solid ${token.colorBorderSecondary}`,
            fontSize: 12,
            flexShrink: 0,
          }}
        >
          <span style={{ fontWeight: 600, color: token.colorText }}>
            {t("chat.agentPanel.poolSummary", {
              completed: poolSummary.completed,
              total: poolSummary.total,
            })}
          </span>
          {poolSummary.running > 0 && (
            <span style={{ color: token.colorPrimary }}>
              {t("chat.agentPanel.running", { count: poolSummary.running })}
            </span>
          )}
          {poolSummary.failed > 0 && (
            <span style={{ color: token.colorError }}>
              {poolSummary.failed} {t("chat.timeline.failed")}
            </span>
          )}
          <div style={{ flex: 1 }} />
          {/* 进度条 */}
          <div
            style={{
              width: 60,
              height: 4,
              borderRadius: 2,
              backgroundColor: token.colorFillSecondary,
              overflow: "hidden",
            }}
          >
            <div
              style={{
                width: `${poolSummary.pct}%`,
                height: "100%",
                backgroundColor: poolSummary.failed > 0
                  ? token.colorWarning
                  : token.colorSuccess,
                transition: "width 0.3s",
              }}
            />
          </div>
          <button
            type="button"
            className="agent-exec-panel__compact-btn"
            onClick={onToggleCompact}
            title={t("chat.agentPanel.collapse")}
            style={{
              border: "none",
              background: "none",
              cursor: "pointer",
              color: token.colorTextQuaternary,
              padding: "2px 4px",
              borderRadius: 4,
            }}
          >
            <svg
              width="14"
              height="14"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
            >
              <path d="M18 6L6 18M6 6h12v12" />
            </svg>
          </button>
        </div>
      )}

      {/* 标签页 */}
      <Tabs
        className="agent-exec-panel__tabs"
        defaultActiveKey="pool"
        destroyOnHidden
        size="small"
        style={{ flex: 1, display: "flex", flexDirection: "column" }}
        tabBarStyle={{ margin: "0 12px", flexShrink: 0 }}
        items={[
          {
            key: "pool",
            label: (
              <span
                style={{
                  display: "inline-flex",
                  alignItems: "center",
                  gap: 4,
                  fontSize: 12,
                }}
              >
                <Bot size={12} />
                {t("chat.agentPanel.pool")}
              </span>
            ),
            children: (
              <div className="agent-exec-panel__tab-content">
                <AgentPoolPanel conversationId={conversationId} />
              </div>
            ),
          },
          {
            key: "timeline",
            label: (
              <span
                style={{
                  display: "inline-flex",
                  alignItems: "center",
                  gap: 4,
                  fontSize: 12,
                }}
              >
                <GitBranch size={12} />
                {t("chat.agentPanel.timeline")}
              </span>
            ),
            children: (
              <div className="agent-exec-panel__tab-content">
                <ExecutionTimeline conversationId={conversationId} />
              </div>
            ),
          },
          {
            key: "replay",
            label: (
              <span
                style={{
                  display: "inline-flex",
                  alignItems: "center",
                  gap: 4,
                  fontSize: 12,
                }}
              >
                <History size={12} />
                {t("chat.agentPanel.replay")}
              </span>
            ),
            children: (
              <div className="agent-exec-panel__tab-content">
                <TrajectoryReplay conversationId={conversationId} />
              </div>
            ),
          },
        ]}
      />
    </div>
  );
}

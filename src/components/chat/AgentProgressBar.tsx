import { useExecutionStore } from "@/stores/feature/executionStore";
import { Progress, Spin, Tag, theme, Typography } from "antd";
import { Wrench } from "lucide-react";
import React, { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

const { Text } = Typography;

interface AgentProgressBarProps {
  conversationId: string;
}

function getToolDisplayName(toolName: string, t: (key: string, ...args: any[]) => string): string {
  const lower = toolName.toLowerCase();
  const map: Record<string, string> = {
    read: "FileRead",
    write: "FileWrite",
    edit: "FileEdit",
    bash: "Bash",
    file_read: t("progressBar.tool.fileRead"),
    file_write: t("progressBar.tool.fileWrite"),
    file_edit: t("progressBar.tool.fileEdit"),
    search: t("progressBar.tool.search"),
    grep: t("progressBar.tool.grep"),
    glob: t("progressBar.tool.glob"),
    web_fetch: t("progressBar.tool.webFetch"),
    web_search: t("progressBar.tool.webSearch"),
    task: t("progressBar.tool.task"),
    mcp: t("progressBar.tool.mcp"),
  };
  for (const [key, display] of Object.entries(map)) {
    if (lower.includes(key)) {
      return display;
    }
  }
  return toolName;
}

/**
 * Agent 执行进度指示器
 *
 * 在聊天界面中显示当前 agent 的工具执行状态：
 * - 根据 agentStore 中的 currentToolCall 和 isExecuting 展示进度
 * - 显示当前正在执行的工具名称
 */
export const AgentProgressBar: React.FC<AgentProgressBarProps> = ({
  conversationId,
}) => {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const currentToolCall = useExecutionStore((s) => s.currentToolCall);
  const isExecuting = useExecutionStore((s) => s.isActive(conversationId));

  const displayName = useMemo(() => {
    return currentToolCall ? getToolDisplayName(currentToolCall.toolName, t as any) : null;
  }, [currentToolCall?.toolName, currentToolCall?.toolUseId, t]);

  // 用于动画过渡：当工具切换时短暂闪烁
  const [lastToolName, setLastToolName] = useState<string | null>(null);
  const [transitioning, setTransitioning] = useState(false);

  useEffect(() => {
    if (currentToolCall?.toolName && currentToolCall.toolName !== lastToolName) {
      setTransitioning(true);
      const t = setTimeout(() => setTransitioning(false), 300);
      setLastToolName(currentToolCall.toolName);
      return () => clearTimeout(t);
    }
  }, [currentToolCall?.toolName, currentToolCall?.toolUseId, lastToolName]);

  const active = isExecuting || currentToolCall != null;

  if (!active) {
    return null;
  }

  const elapsed = currentToolCall
    ? Math.round((Date.now() - currentToolCall.startedAt) / 1000)
    : 0;

  return (
    <div
      className="agent-progress-bar"
      style={{
        display: "flex",
        alignItems: "center",
        gap: 8,
        padding: "4px 24px",
        fontSize: 12,
        color: token.colorTextSecondary,
        borderBottom: `1px solid ${token.colorBorderSecondary}`,
        backgroundColor: token.colorFillAlter,
        transition: "opacity 0.3s ease",
        opacity: transitioning ? 0.6 : 1,
      }}
    >
      {/* 左侧指示器 */}
      <Spin size="small" />

      {/* 工具名称 */}
      {displayName && (
        <Tag
          color="processing"
          style={{
            margin: 0,
            fontSize: 11,
            lineHeight: "18px",
            padding: "0 6px",
          }}
        >
          <Wrench size={10} style={{ marginRight: 4, verticalAlign: "middle" }} />
          {t("progressBar.executing", { name: displayName })}
        </Tag>
      )}

      {/* 进度条 */}
      <div style={{ flex: 1, maxWidth: 200 }}>
        <Progress
          percent={Math.min(elapsed * 10, 90)}
          showInfo={false}
          size="small"
          strokeColor={token.colorPrimary}
          trailColor={token.colorFillSecondary}
          style={{ margin: 0 }}
        />
      </div>

      {/* 耗时 */}
      {elapsed > 0 && (
        <Text
          type="secondary"
          style={{
            fontSize: 11,
            whiteSpace: "nowrap",
            fontVariantNumeric: "tabular-nums",
          }}
        >
          {elapsed < 60
            ? `${elapsed}s`
            : `${Math.floor(elapsed / 60)}m ${elapsed % 60}s`}
        </Text>
      )}
    </div>
  );
};

export default AgentProgressBar;

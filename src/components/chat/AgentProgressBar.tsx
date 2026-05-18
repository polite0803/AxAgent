import { useExecutionStore } from "@/stores/feature/executionStore";
import { Progress, Spin, Tag, theme, Typography } from "antd";
import { Wrench } from "lucide-react";
import React, { useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";

const { Text } = Typography;

interface AgentProgressBarProps {
  conversationId: string;
}

function getToolDisplayName(toolName: string, t: (key: string) => string): string {
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
  // js-set-map-lookups: 子串匹配无法用 Set.has 替代（需部分匹配 toolName）
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

  // 持久化最后一次看到的工具名称，解决 currentToolCall 被 handleToolResult 置 null
  // 但 phase 仍为 executing 时（工具间隙），UI 显示空转无名称的问题。
  const lastKnownToolNameRef = useRef<string | null>(null);
  const prevIsExecutingRef = useRef(false);

  // 清理持久名称：当执行结束（active → inactive）时重置
  useEffect(() => {
    if (!isExecuting && prevIsExecutingRef.current) {
      lastKnownToolNameRef.current = null;
    }
    prevIsExecutingRef.current = isExecuting;
  }, [isExecuting]);

  const currentDisplayName = useMemo(() => {
    return currentToolCall?.conversationId === conversationId
      ? getToolDisplayName(currentToolCall.toolName, t)
      : null;
  }, [currentToolCall?.toolName, currentToolCall?.toolUseId, currentToolCall?.conversationId, conversationId, t]);

  // 当有新的工具名称时，更新持久引用
  useEffect(() => {
    if (currentDisplayName) {
      lastKnownToolNameRef.current = currentDisplayName;
    }
  }, [currentDisplayName]);

  // displayName 优先使用当前 currentToolCall 的名称，fallback 到持久化的名称
  const displayName = currentDisplayName || lastKnownToolNameRef.current;

  // 用于动画过渡：当工具切换时短暂闪烁
  const lastToolNameRef = useRef<string | null>(null);
  const [transitioning, setTransitioning] = useState(false);

  useEffect(() => {
    if (
      currentToolCall?.conversationId === conversationId
      && currentToolCall?.toolName
      && currentToolCall.toolName !== lastToolNameRef.current
    ) {
      setTransitioning(true);
      const t = setTimeout(() => setTransitioning(false), 300);
      lastToolNameRef.current = currentToolCall.toolName;
      return () => clearTimeout(t);
    }
  }, [
    currentToolCall?.toolName,
    currentToolCall?.toolUseId,
    currentToolCall?.conversationId,
    conversationId,
  ]);

  // 仅依赖状态机判断当前对话是否活跃，不依赖全局 currentToolCall
  const active = isExecuting;
  // 工具名称只在本对话的 currentToolCall 有效时显示
  const ownToolActive = isExecuting && currentToolCall?.conversationId === conversationId;

  if (!active) {
    return null;
  }

  const elapsed = ownToolActive
    ? Math.round((Date.now() - currentToolCall!.startedAt) / 1000)
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

      {/* 工具名称 — 显示当前或最后已知的工具名称 */}
      {active && displayName && (
        <Tag
          color="processing"
          style={{
            margin: 0,
            fontSize: 12,
            lineHeight: "18px",
            padding: "0 6px",
          }}
        >
          <Wrench size={10} style={{ marginRight: 4, verticalAlign: "middle" }} />
          {t("progressBar.executing", { name: displayName })}
        </Tag>
      )}

      {/* 进度条 — 仅当有本对话的工具调用时显示 */}
      {ownToolActive && (
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
      )}

      {/* 耗时 — 仅当有本对话的工具调用时显示 */}
      {ownToolActive && elapsed > 0 && (
        <Text
          type="secondary"
          style={{
            fontSize: 12,
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

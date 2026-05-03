import { useAgentStore } from "@/stores";
import { Progress, Spin, Tag, theme, Typography } from "antd";
import { Wrench } from "lucide-react";
import React, { useEffect, useState } from "react";

const { Text } = Typography;

interface AgentProgressBarProps {
  conversationId: string;
}

/** 工具名称友好显示映射 */
const TOOL_DISPLAY_NAMES: Record<string, string> = {
  read: "FileRead",
  write: "FileWrite",
  edit: "FileEdit",
  bash: "Bash",
  file_read: "读取文件",
  file_write: "写入文件",
  file_edit: "编辑文件",
  search: "搜索",
  grep: "文本搜索",
  glob: "文件查找",
  web_fetch: "网页抓取",
  web_search: "网络搜索",
  task: "子任务",
  mcp: "MCP 工具",
};

function getToolDisplayName(toolName: string): string {
  const lower = toolName.toLowerCase();
  for (const [key, display] of Object.entries(TOOL_DISPLAY_NAMES)) {
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
  const { token } = theme.useToken();
  const currentToolCall = useAgentStore((s) => s.currentToolCall);
  const isExecuting = useAgentStore((s) => s.isExecuting[conversationId] ?? false);

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

  const displayName = currentToolCall
    ? getToolDisplayName(currentToolCall.toolName)
    : null;

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
          正在执行: {displayName}
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

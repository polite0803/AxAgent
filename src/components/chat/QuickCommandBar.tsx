import { useCompressStore, useConversationStore } from "@/stores";
import { Button, Tooltip, theme } from "antd";
import {
  Brain,
  Eraser,
  MessageSquare,
  Scissors,
  Zap,
} from "lucide-react";
import React, { useCallback } from "react";

interface QuickCommand {
  key: string;
  label: string;
  icon: React.ReactNode;
  tooltip: string;
  requiresAgent?: boolean;
  action: () => void;
}

export const QuickCommandBar: React.FC = () => {
  const { token } = theme.useToken();
  const clearAllMessages = useConversationStore((s) => s.clearAllMessages);
  const compressContext = useCompressStore((s) => s.compressContext);
  const compressing = useCompressStore((s) => s.compressing);
  const switchModel = useConversationStore((s) => s.switchModel);

  const handleClear = useCallback(() => {
    clearAllMessages();
  }, [clearAllMessages]);

  const handleCompact = useCallback(() => {
    compressContext();
  }, [compressContext]);

  const handleModelSwitch = useCallback(
    (keyword: string) => {
      switchModel(keyword);
    },
    [switchModel],
  );

  const commands: QuickCommand[] = [
    {
      key: "clear",
      label: "/clear",
      icon: <Eraser size={12} />,
      tooltip: "清空当前对话",
      action: handleClear,
    },
    {
      key: "compact",
      label: "/compact",
      icon: <Scissors size={12} />,
      tooltip: "压缩上下文",
      action: handleCompact,
    },
    {
      key: "model-opus",
      label: "/model opus",
      icon: <Brain size={12} />,
      tooltip: "切换到 Opus 模型",
      action: () => handleModelSwitch("opus"),
    },
    {
      key: "model-sonnet",
      label: "/model sonnet",
      icon: <MessageSquare size={12} />,
      tooltip: "切换到 Sonnet 模型",
      action: () => handleModelSwitch("sonnet"),
    },
    {
      key: "model-haiku",
      label: "/model haiku",
      icon: <Zap size={12} />,
      tooltip: "切换到 Haiku 模型",
      action: () => handleModelSwitch("haiku"),
    },
  ];

  return (
    <div
      className="quick-command-bar"
      style={{
        display: "flex",
        alignItems: "center",
        gap: 4,
        padding: "4px 12px",
        flexWrap: "wrap",
      }}
    >
      {commands.map((cmd) => (
        <Tooltip key={cmd.key} title={cmd.tooltip} placement="top">
          <Button
            size="small"
            type="text"
            loading={cmd.key === "compact" && compressing}
            onClick={cmd.action}
            style={{
              fontSize: 12,
              padding: "0 8px",
              height: 24,
              color: token.colorTextSecondary,
              borderRadius: token.borderRadiusSM,
            }}
            onMouseEnter={(e) => {
              (e.currentTarget as HTMLElement).style.color = token.colorPrimary;
            }}
            onMouseLeave={(e) => {
              (e.currentTarget as HTMLElement).style.color = token.colorTextSecondary;
            }}
          >
            {cmd.icon}
            <span style={{ marginLeft: 4 }}>{cmd.label}</span>
          </Button>
        </Tooltip>
      ))}
    </div>
  );
};

export default QuickCommandBar;

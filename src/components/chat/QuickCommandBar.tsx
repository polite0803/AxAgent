import { useCompressStore, useConversationStore, useProviderStore } from "@/stores";
import { Button, Dropdown, theme, Tooltip } from "antd";
import { Cpu, Eraser, Scissors } from "lucide-react";
import React, { useCallback, useMemo } from "react";

interface QuickCommand {
  key: string;
  label: string;
  icon: React.ReactNode;
  tooltip: string;
  action: () => void;
}

export const QuickCommandBar: React.FC = () => {
  const { token } = theme.useToken();
  const clearAllMessages = useConversationStore((s) => s.clearAllMessages);
  const compressContext = useCompressStore((s) => s.compressContext);
  const compressing = useCompressStore((s) => s.compressing);
  const switchModel = useConversationStore((s) => s.switchModel);
  const providers = useProviderStore((s) => s.providers);
  const activeConversationId = useConversationStore((s) => s.activeConversationId);
  const conversations = useConversationStore((s) => s.conversations);

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

  // 从所有启用的供应商中收集当前可用的模型
  const availableModels = useMemo(() => {
    return providers
      .filter((p) => p.enabled)
      .flatMap((p) =>
        p.models.filter((m) => m.enabled).map((m) => ({
          key: `${p.id}:${m.model_id}`,
          label: m.model_id,
          provider: p.name || p.id,
          onClick: () => handleModelSwitch(m.model_id),
        }))
      );
  }, [providers, handleModelSwitch]);

  // 当前会话使用的模型
  const currentModel = useMemo(() => {
    if (!activeConversationId) { return null; }
    const conv = conversations.find((c) => c.id === activeConversationId);
    if (!conv) { return null; }
    const provider = providers.find((p) => p.id === conv.provider_id);
    const modelId = conv.model_id || "";
    return { provider: provider?.name || conv.provider_id, model: modelId };
  }, [activeConversationId, conversations, providers]);

  const modelMenuItems = availableModels.map((m) => ({
    key: m.key,
    label: (
      <span>
        {m.label}
        <span style={{ fontSize: 11, color: token.colorTextQuaternary, marginLeft: 8 }}>
          {m.provider}
        </span>
      </span>
    ),
    onClick: m.onClick,
  }));

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

      {/* 动态模型切换器：替代硬编码的 /model opus|sonnet|haiku */}
      {availableModels.length > 0 && (
        <Dropdown menu={{ items: modelMenuItems }} trigger={["click"]} placement="bottomLeft">
          <Tooltip title="切换模型" placement="top">
            <Button
              size="small"
              type="text"
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
              <Cpu size={12} />
              <span style={{ marginLeft: 4 }}>
                /model{currentModel ? ` ${currentModel.model}` : ""}
              </span>
            </Button>
          </Tooltip>
        </Dropdown>
      )}
    </div>
  );
};

export default QuickCommandBar;

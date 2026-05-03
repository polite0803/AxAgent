import { Space, Tag, theme, Tooltip } from "antd";
import { BookOpen, Bot, Brain, Lightbulb, Search, Wrench } from "lucide-react";
import { useCallback, useMemo } from "react";
import { useTranslation } from "react-i18next";

interface ContextBarProps {
  modelName?: string;
  searchEnabled?: boolean;
  toolCount?: number;
  knowledgeCount?: number;
  memoryEnabled?: boolean;
  onChipClick?: (type: string) => void;
  /** 当前估算的 token 用量 */
  tokenUsed?: number;
  /** 模型的上下文窗口大小 */
  tokenMax?: number;
  onTokenClick?: () => void;
}

function getTokenUsageColor(ratio: number, token: ReturnType<typeof theme.useToken>["token"]) {
  if (ratio > 0.95) return token.colorError;
  if (ratio > 0.8) return token.colorWarning;
  if (ratio > 0.5) return token.colorWarningText;
  return token.colorSuccess;
}

export function ContextBar({
  modelName,
  searchEnabled = false,
  toolCount = 0,
  knowledgeCount = 0,
  memoryEnabled = false,
  onChipClick,
  tokenUsed,
  tokenMax,
  onTokenClick,
}: ContextBarProps) {
  const { t } = useTranslation();
  const { token } = theme.useToken();

  const handleClick = useCallback(
    (type: string) => {
      onChipClick?.(type);
    },
    [onChipClick],
  );

  const tokenRatio = (tokenUsed != null && tokenMax != null && tokenMax > 0)
    ? tokenUsed / tokenMax
    : null;
  const tokenColor = tokenRatio != null ? getTokenUsageColor(tokenRatio, token) : undefined;
  const tokenPercent = tokenRatio != null ? Math.min(100, Math.round(tokenRatio * 100)) : null;

  const chips = useMemo(
    () => [
      ...(modelName
        ? [
          {
            key: "model",
            icon: <Bot size={14} />,
            label: modelName,
            color: "purple" as const,
            tooltip: t("chat.context.model"),
          },
        ]
        : []),
      {
        key: "search",
        icon: <Search size={14} />,
        label: searchEnabled ? t("chat.context.enabled") : t("chat.context.disabled"),
        color: (searchEnabled ? "green" : "default") as string,
        tooltip: t("chat.context.search"),
      },
      {
        key: "tools",
        icon: <Wrench size={14} />,
        label: t("chat.context.count", { count: toolCount }),
        color: (toolCount > 0 ? "blue" : "default") as string,
        tooltip: t("chat.context.tools"),
      },
      {
        key: "knowledge",
        icon: <BookOpen size={14} />,
        label: t("chat.context.count", { count: knowledgeCount }),
        color: (knowledgeCount > 0 ? "blue" : "default") as string,
        tooltip: t("chat.context.knowledge"),
      },
      {
        key: "memory",
        icon: <Lightbulb size={14} />,
        label: memoryEnabled ? t("chat.context.enabled") : t("chat.context.disabled"),
        color: (memoryEnabled ? "green" : "default") as string,
        tooltip: t("chat.context.memory"),
      },
    ],
    [modelName, searchEnabled, toolCount, knowledgeCount, memoryEnabled, t],
  );

  return (
    <div
      style={{
        display: "flex",
        alignItems: "center",
        gap: 8,
        padding: "8px 16px",
        borderBottom: "1px solid var(--border-color)",
        backgroundColor: token.colorBgContainer,
        overflowX: "auto",
      }}
    >
      <Space size={[4, 4]} wrap>
        {chips.map((chip) => (
          <Tooltip key={chip.key} title={chip.tooltip}>
            <Tag
              icon={chip.icon}
              color={chip.color}
              style={{ cursor: onChipClick ? "pointer" : "default", margin: 0 }}
              onClick={() => handleClick(chip.key)}
            >
              {chip.label}
            </Tag>
          </Tooltip>
        ))}
        {/* Token 用量进度条 */}
        {tokenMax != null && tokenMax > 0 && (
          <Tooltip
            title={
              tokenUsed != null
                ? `${tokenUsed.toLocaleString()} / ${tokenMax.toLocaleString()} tokens (${tokenPercent}%)`
                : `${t("chat.context.tokenMax")}: ${tokenMax.toLocaleString()}`
            }
          >
            <div
              style={{
                display: "inline-flex",
                alignItems: "center",
                gap: 6,
                padding: "0 8px",
                height: 24,
                borderRadius: token.borderRadiusSM,
                backgroundColor: token.colorFillQuaternary,
                cursor: onTokenClick ? "pointer" : "default",
                fontSize: 12,
                whiteSpace: "nowrap",
              }}
              onClick={onTokenClick}
            >
              <Brain size={14} style={{ color: tokenColor ?? token.colorTextSecondary, flexShrink: 0 }} />
              {tokenUsed != null && (
                <span style={{ color: tokenColor, fontWeight: 500 }}>
                  {tokenPercent}%
                </span>
              )}
              <div
                style={{
                  width: 48,
                  height: 4,
                  borderRadius: 2,
                  backgroundColor: token.colorFillSecondary,
                  overflow: "hidden",
                }}
              >
                <div
                  style={{
                    width: `${tokenPercent ?? 0}%`,
                    height: "100%",
                    borderRadius: 2,
                    backgroundColor: tokenColor ?? token.colorSuccess,
                    transition: "width 0.3s ease, background-color 0.3s ease",
                  }}
                />
              </div>
            </div>
          </Tooltip>
        )}
      </Space>
    </div>
  );
}

export function estimateConversationTokens(messages: { role: string; content: string }[]): number {
  let total = 0;
  for (const msg of messages) {
    total += msg.content.length;
  }
  // 粗略估算：每 3.5 个字符约 1 token
  return Math.ceil(total / 3.5);
}

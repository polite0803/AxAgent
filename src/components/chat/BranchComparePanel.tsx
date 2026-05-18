import { Alert, Empty, Skeleton, Tag, theme, Typography } from "antd";
import { GitBranch } from "lucide-react";
import { useMemo } from "react";
import { useTranslation } from "react-i18next";

import { parseChatMarkdown } from "@/lib/chatMarkdown";
import type { Message } from "@/types";

import { AssistantMarkdown } from "./ChatMarkdownNodes";

export interface BranchComparePanelProps {
  leftMessage?: Message;
  rightMessage?: Message;
  loading?: boolean;
  error?: string;
  isDarkMode: boolean;
  codeBlockDarkTheme: string;
  codeBlockLightTheme: string;
  codeBlockThemes: string[];
  codeFontFamily?: string;
}

function formatTime(ts?: number): string {
  if (!ts) { return ""; }
  const d = new Date(ts);
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}`;
}

function CompareCard({
  message,
  side,
  isDarkMode,
  codeBlockDarkTheme,
  codeBlockLightTheme,
  codeBlockThemes,
  codeFontFamily,
}: {
  message: Message;
  side: "left" | "right";
  isDarkMode: boolean;
  codeBlockDarkTheme: string;
  codeBlockLightTheme: string;
  codeBlockThemes: string[];
  codeFontFamily?: string;
}) {
  const { t } = useTranslation();
  const { token } = theme.useToken();

  const nodes = useMemo(() => parseChatMarkdown(message.content), [message.content]);

  const modelName = message.model_id ?? "";

  return (
    <div
      style={{
        border: `1px solid ${token.colorBorderSecondary}`,
        borderRadius: token.borderRadiusLG,
        overflow: "hidden",
        height: "100%",
        display: "flex",
        flexDirection: "column",
      }}
    >
      <div
        style={{
          display: "flex",
          alignItems: "center",
          gap: 8,
          padding: "8px 12px",
          borderBottom: `1px solid ${token.colorBorderSecondary}`,
          backgroundColor: token.colorBgLayout,
        }}
      >
        <Tag color={side === "left" ? "blue" : "green"}>
          {side === "left" ? t("chat.branch.left") : t("chat.branch.right")}
        </Tag>
        {modelName && <Typography.Text style={{ fontSize: 13 }}>{modelName}</Typography.Text>}
        {message.created_at
          ? (
            <Typography.Text type="secondary" style={{ fontSize: 12, marginLeft: "auto" }}>
              {formatTime(message.created_at)}
            </Typography.Text>
          )
          : null}
      </div>
      <div style={{ padding: 12, overflow: "auto", flex: 1 }}>
        <AssistantMarkdown
          content={message.content}
          nodes={nodes}
          isDarkMode={isDarkMode}
          isStreaming={false}
          codeBlockDarkTheme={codeBlockDarkTheme}
          codeBlockLightTheme={codeBlockLightTheme}
          codeBlockThemes={codeBlockThemes}
          codeFontFamily={codeFontFamily}
        />
      </div>
    </div>
  );
}

export function BranchComparePanel({
  leftMessage,
  rightMessage,
  loading,
  error,
  isDarkMode,
  codeBlockDarkTheme,
  codeBlockLightTheme,
  codeBlockThemes,
  codeFontFamily,
}: BranchComparePanelProps) {
  const { t } = useTranslation();

  if (error) {
    return <Alert type="error" message={error} showIcon style={{ marginTop: 16 }} />;
  }

  if (loading) {
    return (
      <div style={{ display: "flex", gap: 12, marginTop: 16 }}>
        <div style={{ flex: 1 }}>
          <Skeleton active paragraph={{ rows: 8 }} />
        </div>
        <div style={{ flex: 1 }}>
          <Skeleton active paragraph={{ rows: 8 }} />
        </div>
      </div>
    );
  }

  if (!leftMessage && !rightMessage) {
    return <Empty description={t("chat.branch.compare")} style={{ marginTop: 32 }} />;
  }

  return (
    <div>
      <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 12 }}>
        <GitBranch size={16} />
        <Typography.Text strong>{t("chat.branch.compare")}</Typography.Text>
      </div>
      <div style={{ display: "flex", gap: 12, height: "calc(100% - 36px)" }}>
        <div style={{ flex: 1, minWidth: 0 }}>
          {leftMessage
            ? (
              <CompareCard
                message={leftMessage}
                side="left"
                isDarkMode={isDarkMode}
                codeBlockDarkTheme={codeBlockDarkTheme}
                codeBlockLightTheme={codeBlockLightTheme}
                codeBlockThemes={codeBlockThemes}
                codeFontFamily={codeFontFamily}
              />
            )
            : <Empty description={t("common.noData")} />}
        </div>
        <div style={{ flex: 1, minWidth: 0 }}>
          {rightMessage
            ? (
              <CompareCard
                message={rightMessage}
                side="right"
                isDarkMode={isDarkMode}
                codeBlockDarkTheme={codeBlockDarkTheme}
                codeBlockLightTheme={codeBlockLightTheme}
                codeBlockThemes={codeBlockThemes}
                codeFontFamily={codeFontFamily}
              />
            )
            : <Empty description={t("common.noData")} />}
        </div>
      </div>
    </div>
  );
}

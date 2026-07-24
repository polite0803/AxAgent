// SPDX-License-Identifier: AGPL-3.0-only

// 引用回复：消息气泡内显示被引用消息的预览块

import { Tooltip } from "@/components/layout/Tooltip";
import type { Message } from "@/types";
import { Avatar, Typography } from "antd";
import { MessageSquare } from "lucide-react";
import { useTranslation } from "react-i18next";

interface QuoteBlockProps {
  /** 被引用的消息 */
  quotedMessage: Message;
  /** 点击引用块时回调（通常滚动到被引用消息） */
  onJump: (messageId: string) => void;
}

function getRoleLabel(role: Message["role"], t: (k: string) => string): string {
  switch (role) {
    case "user":
      return t("chat.quote.roleUser");
    case "assistant":
      return t("chat.quote.roleAssistant");
    case "system":
      return t("chat.quote.roleSystem");
    case "tool":
      return t("chat.quote.roleTool");
    default:
      return role;
  }
}

function truncateContent(content: string, maxLen = 120): string {
  if (content.length <= maxLen) {
    return content;
  }
  return `${content.slice(0, maxLen)}…`;
}

/** 引用预览块：用于消息气泡顶部显示被引用消息内容 */
export function QuoteBlock({ quotedMessage, onJump }: QuoteBlockProps) {
  const { t } = useTranslation();
  const roleLabel = getRoleLabel(quotedMessage.role, t);
  const preview = truncateContent(quotedMessage.content);

  return (
    <Tooltip title={t("chat.quote.jumpHint")}>
      <div
        onClick={(e) => {
          e.stopPropagation();
          onJump(quotedMessage.id);
        }}
        style={{
          padding: "6px 10px",
          marginBottom: 8,
          backgroundColor: "rgba(0, 0, 0, 0.04)",
          borderLeft: "3px solid #1677ff",
          borderRadius: 4,
          cursor: "pointer",
          display: "flex",
          alignItems: "flex-start",
          gap: 8,
          transition: "background-color 0.15s",
        }}
        onMouseEnter={(e) => {
          (e.currentTarget as HTMLDivElement).style.backgroundColor = "rgba(22, 119, 255, 0.08)";
        }}
        onMouseLeave={(e) => {
          (e.currentTarget as HTMLDivElement).style.backgroundColor = "rgba(0, 0, 0, 0.04)";
        }}
      >
        <Avatar
          size={20}
          icon={<MessageSquare size={12} />}
          style={{ backgroundColor: "#1677ff", flexShrink: 0, marginTop: 2 }}
        />
        <div style={{ minWidth: 0, flex: 1 }}>
          <Typography.Text
            style={{ fontSize: 12, color: "rgba(0, 0, 0, 0.45)", display: "block" }}
          >
            {roleLabel}
          </Typography.Text>
          <Typography.Text
            style={{
              fontSize: 13,
              color: "rgba(0, 0, 0, 0.75)",
              wordBreak: "break-word",
              whiteSpace: "pre-wrap",
            }}
          >
            {preview}
          </Typography.Text>
        </div>
      </div>
    </Tooltip>
  );
}

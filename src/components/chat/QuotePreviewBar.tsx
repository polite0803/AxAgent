// SPDX-License-Identifier: AGPL-3.0-only

import { Button, theme, Typography } from "antd";
import { MessageSquare, X } from "lucide-react";
import { useTranslation } from "react-i18next";

import { Tooltip } from "@/components/layout/Tooltip";

interface QuotePreviewBarProps {
  /** 被引用消息内容 */
  content: string;
  /** 取消引用回调 */
  onCancel: () => void;
}

/**
 * 引用回复预览条：显示当前被引用的消息内容，可一键取消引用。
 */
export function QuotePreviewBar({ content, onCancel }: QuotePreviewBarProps) {
  const { t } = useTranslation();
  const { token } = theme.useToken();

  return (
    <div
      className="quote-preview-bar"
      style={{
        display: "flex",
        alignItems: "center",
        gap: 8,
        padding: "6px 10px",
        marginBottom: 6,
        backgroundColor: token.colorFillTertiary,
        borderLeft: `3px solid ${token.colorPrimary}`,
        borderRadius: token.borderRadiusSM,
      }}
    >
      <MessageSquare size={14} style={{ color: token.colorPrimary, flexShrink: 0 }} />
      <div style={{ minWidth: 0, flex: 1, overflow: "hidden" }}>
        <Typography.Text
          style={{ fontSize: 12, color: token.colorTextTertiary, display: "block" }}
        >
          {t("chat.quote.replyingTo")}
        </Typography.Text>
        <Typography.Text
          style={{
            fontSize: 13,
            color: token.colorTextSecondary,
            whiteSpace: "nowrap",
            overflow: "hidden",
            textOverflow: "ellipsis",
            display: "block",
          }}
        >
          {content.length > 100 ? `${content.slice(0, 100)}…` : content}
        </Typography.Text>
      </div>
      <Tooltip title={t("chat.quote.cancel")}>
        <Button
          type="text"
          size="small"
          icon={<X size={14} />}
          onClick={onCancel}
          style={{ color: token.colorTextTertiary, flexShrink: 0 }}
        />
      </Tooltip>
    </div>
  );
}

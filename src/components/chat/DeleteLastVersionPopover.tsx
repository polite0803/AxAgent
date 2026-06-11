// SPDX-License-Identifier: AGPL-3.0-only

import { App, Button, Popover, theme } from "antd";
import { AlertCircle, Trash2 } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";

import { Tooltip } from "@/components/layout/Tooltip";
import { useConversationStore } from "@/stores";
import type { Message } from "@/types";

export function DeleteLastVersionPopover({
  msg,
  conversationId,
  deleteMessage,
  deleteMessageGroup,
}: {
  msg: Message;
  conversationId: string;
  deleteMessage: (messageId: string) => Promise<void>;
  deleteMessageGroup: (convId: string, parentMsgId: string) => Promise<void>;
}) {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const { message: messageApi } = App.useApp();
  const [open, setOpen] = useState(false);

  const handleDeleteThisOnly = async () => {
    setOpen(false);
    try {
      await deleteMessage(msg.id);
    } catch (e) {
      messageApi.error(String(e));
    }
  };

  const handleDeleteAll = async () => {
    setOpen(false);
    try {
      if (msg.parent_message_id) {
        await deleteMessageGroup(conversationId, msg.parent_message_id);
      } else if (msg.id.startsWith("temp-")) {
        useConversationStore.setState((s) => ({
          messages: s.messages.filter((m) => m.id !== msg.id),
        }));
      }
    } catch (e) {
      messageApi.error(String(e));
    }
  };

  return (
    <Popover
      open={open}
      onOpenChange={setOpen}
      trigger="click"
      placement="top"
      content={
        <div style={{ maxWidth: 280 }}>
          <div
            style={{
              marginBottom: 12,
              display: "flex",
              alignItems: "flex-start",
              gap: 8,
            }}
          >
            <AlertCircle
              size={16}
              style={{ color: token.colorWarning, marginTop: 2, flexShrink: 0 }}
            />
            <span>{t("chat.deleteLastVersionHint")}</span>
          </div>
          <div style={{ display: "flex", justifyContent: "flex-end", gap: 8 }}>
            <Button size="small" onClick={() => setOpen(false)}>
              {t("common.cancel")}
            </Button>
            <Button size="small" onClick={handleDeleteThisOnly}>
              {t("chat.deleteThisOnly")}
            </Button>
            <Button
              size="small"
              danger
              type="primary"
              onClick={handleDeleteAll}
            >
              {t("chat.deleteAll")}
            </Button>
          </div>
        </div>
      }
    >
      <Tooltip title={t("chat.delete")}>
        <span
          className="axagent-action-item"
          style={{ color: token.colorError }}
        >
          <Trash2 size={14} />
        </span>
      </Tooltip>
    </Popover>
  );
}

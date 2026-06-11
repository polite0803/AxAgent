// SPDX-License-Identifier: AGPL-3.0-only

import { Button, theme, Typography } from "antd";
import { ChevronLeft, ChevronRight } from "lucide-react";

import { useConversationStore } from "@/stores";
import type { Message } from "@/types";

export function VersionPagination({
  msg,
  conversationId,
  allVersions,
}: {
  msg: Message;
  conversationId: string;
  allVersions: Message[];
}) {
  const { token } = theme.useToken();
  const switchMessageVersion = useConversationStore(
    (s) => s.switchMessageVersion,
  );

  const currentModelId = msg.model_id;
  const modelVersions = allVersions.filter(
    (v) => v.model_id === currentModelId,
  );

  if (modelVersions.length <= 1) {
    return null;
  }

  const sorted = modelVersions.toSorted(
    (a, b) => a.version_index - b.version_index,
  );
  const currentIdx = sorted.findIndex((v) => v.id === msg.id);
  const current = currentIdx >= 0 ? currentIdx : sorted.findIndex((v) => v.is_active);

  const handlePrev = () => {
    if (current > 0 && msg.parent_message_id) {
      switchMessageVersion(
        conversationId,
        msg.parent_message_id,
        sorted[current - 1].id,
      );
    }
  };
  const handleNext = () => {
    if (current < sorted.length - 1 && msg.parent_message_id) {
      switchMessageVersion(
        conversationId,
        msg.parent_message_id,
        sorted[current + 1].id,
      );
    }
  };

  return (
    <span
      style={{
        display: "inline-flex",
        alignItems: "center",
        gap: 2,
        marginRight: 8,
      }}
    >
      <Button
        type="text"
        size="small"
        icon={<ChevronLeft size={14} />}
        disabled={current <= 0}
        onClick={handlePrev}
        style={{ minWidth: 20, padding: "0 2px" }}
      />
      <Typography.Text
        style={{ fontSize: 12, color: token.colorTextSecondary }}
      >
        {current + 1}/{sorted.length}
      </Typography.Text>
      <Button
        type="text"
        size="small"
        icon={<ChevronRight size={14} />}
        disabled={current >= sorted.length - 1}
        onClick={handleNext}
        style={{ minWidth: 20, padding: "0 2px" }}
      />
    </span>
  );
}

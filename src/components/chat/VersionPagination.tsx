// SPDX-License-Identifier: AGPL-3.0-only

import { Button, theme, Typography } from "antd";
import { ChevronLeft, ChevronRight, Loader } from "lucide-react";
import { useCallback, useRef, useState } from "react";

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
  const [switching, setSwitching] = useState(false);
  const switchingRef = useRef(false);

  const currentModelId = msg.modelId;
  const modelVersions = allVersions.filter(
    (v) => v.modelId === currentModelId,
  );

  const sorted = modelVersions.toSorted(
    (a, b) => a.versionIndex - b.versionIndex,
  );
  const currentIdx = sorted.findIndex((v) => v.id === msg.id);
  const current = currentIdx >= 0 ? currentIdx : sorted.findIndex((v) => v.isActive);

  const doSwitch = useCallback(
    async (targetId: string) => {
      // Fix: concurrent guard — prevent rapid clicks from triggering
      // multiple switches before React state catches up. switchingRef
      // provides the lock; switching state disables buttons visually.
      if (switchingRef.current || !msg.parentMessageId) { return; }
      switchingRef.current = true;
      setSwitching(true);
      try {
        await switchMessageVersion(conversationId, msg.parentMessageId, targetId);
      } finally {
        switchingRef.current = false;
        setSwitching(false);
      }
    },
    [conversationId, msg.parentMessageId, switchMessageVersion],
  );

  if (modelVersions.length <= 1 && !switching) {
    return null;
  }

  const handlePrev = () => {
    if (current > 0) {
      doSwitch(sorted[current - 1].id);
    }
  };
  const handleNext = () => {
    if (current < sorted.length - 1) {
      doSwitch(sorted[current + 1].id);
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
      {switching
        ? (
          <Loader
            size={14}
            style={{
              animation: "axagent-think-spin 1s linear infinite",
              color: token.colorTextSecondary,
            }}
          />
        )
        : (
          <>
            <Button
              type="text"
              size="small"
              icon={<ChevronLeft size={14} />}
              disabled={switching || current <= 0}
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
              disabled={switching || current >= sorted.length - 1}
              onClick={handleNext}
              style={{ minWidth: 20, padding: "0 2px" }}
            />
          </>
        )}
    </span>
  );
}

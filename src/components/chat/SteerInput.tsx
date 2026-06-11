// SPDX-License-Identifier: AGPL-3.0-only

import { invoke } from "@/lib/invoke";
import { Button, Input, message, theme } from "antd";
import { useState } from "react";
import { useTranslation } from "react-i18next";

interface SteerInputProps {
  conversationId: string;
}

export function SteerInput({ conversationId }: SteerInputProps) {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const [instruction, setInstruction] = useState("");
  const [sending, setSending] = useState(false);

  const handleSteer = async () => {
    if (!instruction.trim()) {
      return;
    }
    setSending(true);
    try {
      await invoke("agent_steer", { conversationId, instruction });
      setInstruction("");
      message.success(t("chat.steerSent") || "Steer instruction sent");
    } catch (e) {
      message.error(t("chat.steerError") || String(e));
    } finally {
      setSending(false);
    }
  };

  return (
    <div
      className="flex gap-2 items-center p-2"
      style={{
        borderTop: `1px solid ${token.colorBorderSecondary}`,
        backgroundColor: token.colorFillAlter,
      }}
    >
      <Input
        id="steer-input-input-32"
        size="small"
        placeholder={t("chat.steerPlaceholder")}
        value={instruction}
        onChange={(e) => setInstruction(e.target.value)}
        onPressEnter={(e) => {
          if (!e.shiftKey) {
            e.preventDefault();
            handleSteer();
          }
        }}
        className="flex-1"
      />
      <Button
        size="small"
        type="primary"
        loading={sending}
        disabled={!instruction.trim()}
        onClick={handleSteer}
      >
        {t("chat.steer")}
      </Button>
    </div>
  );
}

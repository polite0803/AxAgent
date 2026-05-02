import { invoke } from "@/lib/invoke";
import { Button, Input } from "antd";
import { useState } from "react";
import { useTranslation } from "react-i18next";

export function SteerInput() {
  const { t } = useTranslation();
  const [instruction, setInstruction] = useState("");
  const [sending, setSending] = useState(false);

  const handleSteer = async () => {
    if (!instruction.trim()) { return; }
    setSending(true);
    try {
      await invoke("agent_steer", { instruction });
      setInstruction("");
    } finally {
      setSending(false);
    }
  };

  return (
    <div className="flex gap-2 items-center p-2 border-t border-amber-200 bg-amber-50">
      <Input
        size="small"
        placeholder={t("chat.steerPlaceholder", "Steer agent direction...")}
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
        style={{ backgroundColor: "#d97706", borderColor: "#d97706" }}
      >
        {t("chat.steer", "Steer")}
      </Button>
    </div>
  );
}

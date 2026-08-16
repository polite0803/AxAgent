// SPDX-License-Identifier: AGPL-3.0-only

import { Button, theme } from "antd";
import { Atom, CircleOff, Signal, SignalHigh, SignalLow, SignalMedium } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";

import { DropdownMenu } from "@/components/layout/DropdownMenu";
import type { DropdownItem } from "@/components/layout/DropdownMenu";
import { Tooltip } from "@/components/layout/Tooltip";
import { useConversationStore } from "@/stores";

interface ThinkingMenuProps {
  /** 当前模型是否支持推理 */
  hasReasoning: boolean;
}

const THINKING_OPTIONS = [
  { key: "default", labelKey: "chat.thinking.default", value: null },
  { key: "none", labelKey: "chat.thinking.none", value: 0 },
  { key: "low", labelKey: "chat.thinking.low", value: 1024 },
  { key: "medium", labelKey: "chat.thinking.medium", value: 4096 },
  { key: "high", labelKey: "chat.thinking.high", value: 8192 },
  { key: "xhigh", labelKey: "chat.thinking.xhigh", value: 16384 },
] as const;

function optionIcon(key: string, colorPrimary: string) {
  switch (key) {
    case "none":
      return <CircleOff size={14} />;
    case "low":
      return <SignalLow size={14} />;
    case "medium":
      return <SignalMedium size={14} />;
    case "high":
      return <SignalHigh size={14} />;
    case "xhigh":
      return <Signal size={14} />;
    default:
      return <Atom size={14} style={{ color: colorPrimary }} />;
  }
}

export function ThinkingMenu({ hasReasoning }: ThinkingMenuProps) {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const thinkingBudget = useConversationStore((s) => s.thinkingBudget);
  const setThinkingBudget = useConversationStore((s) => s.setThinkingBudget);
  const [dropdownOpen, setDropdownOpen] = useState(false);

  if (!hasReasoning) {
    return null;
  }

  const selectedKey = THINKING_OPTIONS.find((opt) => opt.value === thinkingBudget)?.key
    ?? "default";

  const items: DropdownItem[] = THINKING_OPTIONS.map((opt) => ({
    key: opt.key,
    label: t(opt.labelKey),
    icon: optionIcon(opt.key, token.colorPrimary),
    onClick: () => {
      // value 类型为 number | null，直接赋值
      setThinkingBudget(opt.value as number | null);
      setDropdownOpen(false);
    },
  }));

  return (
    <DropdownMenu
      items={items}
      open={dropdownOpen}
      onOpenChange={setDropdownOpen}
    >
      <Tooltip title={t("chat.thinkingIntensity")}>
        <Button
          type="text"
          size="small"
          icon={optionIcon(selectedKey, token.colorPrimary)}
          style={thinkingBudget === 0
            ? { color: token.colorError }
            : thinkingBudget !== null
            ? { color: token.colorPrimary }
            : undefined}
        />
      </Tooltip>
    </DropdownMenu>
  );
}

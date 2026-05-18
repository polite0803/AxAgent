// 可复用的上下文帮助按钮 — 悬浮 Tooltip + 点击打开 HelpPanel
import { useHelpStore } from "@/stores/feature/helpStore";
import { Tooltip } from "antd";
import { HelpCircle } from "lucide-react";
import { useTranslation } from "react-i18next";

interface ContextHelpProps {
  helpKey: string;
  placement?: "top" | "bottom" | "left" | "right";
  /** 点击时打开的 help section key */
  section?: string;
}

export function ContextHelp({
  helpKey,
  placement = "top",
  section,
}: ContextHelpProps) {
  const { t } = useTranslation();
  const openSection = useHelpStore((s) => s.openSection);

  const summary = t(`help.${helpKey}.summary`, "");
  if (!summary) {
    return null;
  }

  return (
    <Tooltip title={summary} placement={placement}>
      <button
        type="button"
        onClick={section ? () => openSection(section) : undefined}
        style={{
          border: "none",
          background: "transparent",
          cursor: section ? "pointer" : "default",
          padding: 0,
          display: "inline-flex",
          alignItems: "center",
          color: "var(--color-text-quaternary, #8c8c8c)",
          fontSize: 14,
          marginLeft: 4,
        }}
      >
        <HelpCircle size={14} />
      </button>
    </Tooltip>
  );
}

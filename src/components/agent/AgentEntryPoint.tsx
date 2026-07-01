// SPDX-License-Identifier: AGPL-3.0-only

import { useAgentPanelStore } from "@/stores";
import { Tooltip } from "antd";
import { Bot } from "lucide-react";
import { useTranslation } from "react-i18next";

/**
 * 页面浮动 Agent 入口按钮
 *
 * 在每个核心页面右下角添加浮动按钮，点击打开 Agent Panel。
 * 作为各页面与 Agent Panel 的桥接入口。
 */
export function AgentEntryPoint() {
  const { t } = useTranslation();
  const open = useAgentPanelStore((s) => s.open);
  const isOpen = useAgentPanelStore((s) => s.isOpen);

  if (isOpen) {
    return null;
  }

  return (
    <Tooltip title={t("agentPanel.openPanel")} placement="left">
      <button
        type="button"
        className="fixed z-40 w-10 h-10 rounded-full bg-[var(--color-primary)] text-white shadow-md flex items-center justify-center hover:scale-110 transition-transform"
        style={{
          right: 24,
          bottom: 140,
        }}
        onClick={open}
        aria-label={t("agentPanel.openPanel")}
      >
        <Bot size={19} />
      </button>
    </Tooltip>
  );
}

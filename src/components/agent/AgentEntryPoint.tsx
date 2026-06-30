// SPDX-License-Identifier: AGPL-3.0-only

import { useAgentPanelStore } from "@/stores/shared/agentPanelStore";
import { Tooltip } from "antd";
import { Bot } from "lucide-react";

/**
 * 页面浮动 Agent 入口按钮
 *
 * 在每个核心页面右下角添加浮动按钮，点击打开 Agent Panel。
 * 作为各页面与 Agent Panel 的桥接入口。
 */
export function AgentEntryPoint() {
  const open = useAgentPanelStore((s) => s.open);
  const isOpen = useAgentPanelStore((s) => s.isOpen);

  // 面板已打开时不显示入口按钮
  if (isOpen) {
    return null;
  }

  return (
    <Tooltip title="打开 AxAgent 面板" placement="left">
      <button
        type="button"
        className="fixed z-40 w-10 h-10 rounded-full bg-[var(--color-primary)] text-white shadow-md flex items-center justify-center hover:scale-110 transition-transform"
        style={{
          right: 24,
          bottom: 140,
        }}
        onClick={open}
        aria-label="打开 Agent 面板"
      >
        <Bot size={19} />
      </button>
    </Tooltip>
  );
}

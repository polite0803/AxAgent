// SPDX-License-Identifier: AGPL-3.0-only

import { useAgentPanelStore } from "@/stores";
import { Tooltip } from "antd";
import { Bot, X } from "lucide-react";
import { useCallback, useState } from "react";
import { useTranslation } from "react-i18next";

/** 隐藏状态持久化键（用户主动关闭后不再显示，避免每次启动打扰） */
const HIDE_KEY = "axagent:agentEntryPoint:hidden";

function loadHidden(): boolean {
  try {
    return localStorage.getItem(HIDE_KEY) === "true";
  } catch {
    return false;
  }
}

/**
 * 页面浮动 Agent 入口按钮
 *
 * 在每个核心页面右下角添加浮动按钮，点击打开 Agent Panel。
 * 作为各页面与 Agent Panel 的桥接入口。
 * - hover 时右上角出现隐藏角标，关闭后持久化（侧栏/顶栏仍有面板入口）
 */
export function AgentEntryPoint() {
  const { t } = useTranslation();
  const open = useAgentPanelStore((s) => s.open);
  const isOpen = useAgentPanelStore((s) => s.isOpen);
  const [hidden, setHidden] = useState(loadHidden);

  const hide = useCallback(() => {
    try {
      localStorage.setItem(HIDE_KEY, "true");
    } catch {
      // 忽略
    }
    setHidden(true);
  }, []);

  if (isOpen || hidden) {
    return null;
  }

  return (
    <div className="fixed z-40 group" style={{ right: 24, bottom: 140 }}>
      <Tooltip title={t("agentPanel.openPanel")} placement="left">
        <button
          type="button"
          className="w-10 h-10 rounded-full bg-[var(--color-primary)] text-white shadow-md flex items-center justify-center hover:scale-110 transition-transform"
          onClick={open}
          aria-label={t("agentPanel.openPanel")}
        >
          <Bot size={19} />
        </button>
      </Tooltip>
      <button
        type="button"
        className="absolute -top-1.5 -right-1.5 w-4 h-4 rounded-full bg-[var(--color-fill-alter)] text-[var(--color-text-secondary)] flex items-center justify-center opacity-0 group-hover:opacity-100 transition-opacity hover:bg-[var(--color-error)] hover:text-white cursor-pointer"
        onClick={hide}
        aria-label={t("common.hide")}
        title={t("common.hide")}
      >
        <X size={10} />
      </button>
    </div>
  );
}

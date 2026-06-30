// SPDX-License-Identifier: AGPL-3.0-only

import { useAgentPanelStore } from "@/stores/shared/agentPanelStore";
import { Tooltip } from "antd";
import { Bot, Maximize2, Minimize2, PanelRightClose } from "lucide-react";

export function AgentPanelHeader() {
  const agentContext = useAgentPanelStore((s) => s.agentContext);
  const isMiniMode = useAgentPanelStore((s) => s.isMiniMode);
  const toggleMiniMode = useAgentPanelStore((s) => s.toggleMiniMode);
  const close = useAgentPanelStore((s) => s.close);

  const contextLabel = agentContext
    ? agentContext.page
    : null;

  return (
    <div className="flex items-center justify-between px-3 py-2 border-b border-[var(--border-color)] shrink-0">
      {/* 左侧：标题 */}
      <div className="flex items-center gap-2 min-w-0">
        <Bot size={18} className="text-[var(--color-primary)] shrink-0" />
        <span className="font-semibold text-sm truncate">AxAgent</span>
      </div>

      {/* 中间：上下文指示 */}
      <div className="flex-1 mx-3 min-w-0">
        {contextLabel
          ? (
            <div className="text-xs px-2 py-0.5 rounded-full bg-[var(--color-primary)]/10 text-[var(--color-primary)] truncate text-center">
              {contextLabel}
              {agentContext?.selection && (
                <span className="ml-1 opacity-70">
                  · {agentContext.selection.label}
                </span>
              )}
            </div>
          )
          : (
            <div className="text-xs text-[var(--color-text-secondary)] text-center">
              全局模式
            </div>
          )}
      </div>

      {/* 右侧：操作按钮 */}
      <div className="flex items-center gap-1 shrink-0">
        <Tooltip title={isMiniMode ? "展开全尺寸面板" : "迷你模式"}>
          <button
            type="button"
            className="p-1 rounded hover:bg-[var(--color-fill-alter)] text-[var(--color-text-secondary)]"
            onClick={toggleMiniMode}
          >
            {isMiniMode ? <Maximize2 size={15} /> : <Minimize2 size={15} />}
          </button>
        </Tooltip>
        <Tooltip title="关闭面板">
          <button
            type="button"
            className="p-1 rounded hover:bg-[var(--color-fill-alter)] text-[var(--color-text-secondary)]"
            onClick={close}
          >
            <PanelRightClose size={15} />
          </button>
        </Tooltip>
      </div>
    </div>
  );
}

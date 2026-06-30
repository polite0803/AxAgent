// SPDX-License-Identifier: AGPL-3.0-only

import { useAgentPanelStore } from "@/stores/shared/agentPanelStore";
import { MapPin } from "lucide-react";

/**
 * Agent 上下文信息栏
 *
 * 紧凑的单行展示当前页面上下文：
 * - 有上下文时：页面名称 + 选中项信息
 * - 无上下文时：显示"全局模式"
 */
export function AgentContextBar() {
  const agentContext = useAgentPanelStore((s) => s.agentContext);

  if (!agentContext) {
    return (
      <span className="text-xs text-[var(--color-text-secondary)] inline-flex items-center gap-1">
        <MapPin size={12} />
        全局模式
      </span>
    );
  }

  return (
    <span className="text-xs inline-flex items-center gap-1 text-[var(--color-primary)]">
      <MapPin size={12} />
      <span className="truncate max-w-[180px]">
        {agentContext.page}
      </span>
      {agentContext.selection && (
        <span className="text-[var(--color-text-secondary)]">
          · {agentContext.selection.label}
        </span>
      )}
    </span>
  );
}

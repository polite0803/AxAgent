// SPDX-License-Identifier: AGPL-3.0-only

import { Empty } from "antd";

/**
 * 对话标签页 — Phase 1 占位
 *
 * 后续 Phase 将嵌入 ChatView（通过 ChatView embedded 模式）。
 * 当前仅展示占位提示。
 */
export function AgentChatTab() {
  return (
    <div className="flex items-center justify-center h-full p-6">
      <Empty
        image={Empty.PRESENTED_IMAGE_SIMPLE}
        description={
          <span className="text-[var(--color-text-secondary)]">
            Agent 对话功能即将在此处呈现
          </span>
        }
      />
    </div>
  );
}

// SPDX-License-Identifier: AGPL-3.0-only

import { Empty } from "antd";

/**
 * 执行追踪标签页 — Phase 1 占位
 *
 * 后续 Phase 将展示 Agent 执行轨迹、工具调用详情、进度状态机。
 */
export function AgentExecutionTab() {
  return (
    <div className="flex items-center justify-center h-full p-6">
      <Empty
        image={Empty.PRESENTED_IMAGE_SIMPLE}
        description={
          <span className="text-[var(--color-text-secondary)]">
            执行追踪面板将在后续版本中实现
          </span>
        }
      />
    </div>
  );
}

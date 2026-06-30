// SPDX-License-Identifier: AGPL-3.0-only

import { Empty } from "antd";

/**
 * 技能管理标签页 — Phase 1 占位
 *
 * 后续 Phase 将展示技能列表、启用/禁用、进化状态、版本历史。
 */
export function AgentSkillTab() {
  return (
    <div className="flex items-center justify-center h-full p-6">
      <Empty
        image={Empty.PRESENTED_IMAGE_SIMPLE}
        description={
          <span className="text-[var(--color-text-secondary)]">
            技能管理面板将在后续版本中实现
          </span>
        }
      />
    </div>
  );
}

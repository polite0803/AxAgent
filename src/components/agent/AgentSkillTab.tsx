// SPDX-License-Identifier: AGPL-3.0-only

import { Empty } from "antd";
import { useTranslation } from "react-i18next";

/**
 * 技能管理标签页 — Phase 1 占位
 *
 * 后续 Phase 将展示技能列表、启用/禁用、进化状态、版本历史。
 */
export function AgentSkillTab() {
  const { t } = useTranslation();
  return (
    <div className="flex items-center justify-center h-full p-6">
      <Empty
        image={Empty.PRESENTED_IMAGE_SIMPLE}
        description={
          <span className="text-[var(--color-text-secondary)]">
            {t("agentPanel.skillComingSoon")}
          </span>
        }
      />
    </div>
  );
}

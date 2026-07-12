// SPDX-License-Identifier: AGPL-3.0-only

import { AgentExecutionPanel } from "@/components/chat/AgentExecutionPanel";
import { useConversationStore } from "@/stores";
import { useExecutionStore } from "@/stores/feature/executionStore";
import { Empty } from "antd";
import { useEffect } from "react";
import { useTranslation } from "react-i18next";

/**
 * 执行追踪标签页
 *
 * 复用已有的 AgentExecutionPanel 组件，需要传入当前活跃会话 ID。
 * 没有活跃会话时展示空状态。
 */
export function AgentExecutionTab() {
  const { t } = useTranslation();
  const activeConversationId = useConversationStore((s) => s.activeConversationId);
  const fetchTrajectoryList = useExecutionStore((s) => s.fetchTrajectoryList);

  // 有活跃会话时自动加载轨迹列表
  useEffect(() => {
    if (activeConversationId) {
      fetchTrajectoryList(activeConversationId).catch(() => {});
    }
  }, [activeConversationId, fetchTrajectoryList]);

  if (!activeConversationId) {
    return (
      <div className="flex items-center justify-center h-full p-6">
        <Empty
          image={Empty.PRESENTED_IMAGE_SIMPLE}
          description={
            <span className="text-[var(--color-text-secondary)]">
              {t("agentPanel.executionComingSoon")}
            </span>
          }
        />
      </div>
    );
  }

  return (
    <div className="h-full overflow-auto">
      <AgentExecutionPanel conversationId={activeConversationId} />
    </div>
  );
}

// SPDX-License-Identifier: AGPL-3.0-only

import { useCognitiveRouteStore, useConversationStore } from "@/stores";
import type { CognitiveDecisionInfo } from "@/types";
import { Tag, theme, Typography } from "antd";
import { Bot, GitBranch, Route } from "lucide-react";
import { useTranslation } from "react-i18next";

const { Text } = Typography;

/** 决策标签的统一视图模型：兼容消息级 `decision` 与全局路由观测两种数据源 */
interface CognitiveDecisionView {
  executionMode: string;
  routePath: string;
  selectedWorkflowName: string | null;
  selectedAgentProfile: {
    id: string;
    name: string;
    role?: string | null;
    expert?: string | null;
  } | null;
}

/** 认知编排决策分支轻量卡片：展示某条（历史）消息或最近一次 cognitive_query 的决策结果。
 * - 传入 `decision`：展示对应消息自己记录的决策标签（每条历史消息独立）。
 * - 不传 `decision`：回退展示最近一次路由观测（新消息附近的实时决策）。
 * 让用户直观看到该轮选择了哪个工作流、哪个专家/角色、以何种模式执行。 */
export function CognitiveDecisionCard({ decision }: { decision?: CognitiveDecisionInfo | null }) {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const observation = useCognitiveRouteStore((s) => s.observation);
  const activeConversationId = useConversationStore((s) => s.activeConversationId);

  const data: CognitiveDecisionView | null = decision
    ? {
      executionMode: decision.executionMode,
      routePath: decision.routePath,
      selectedWorkflowName: decision.selectedWorkflowName ?? null,
      selectedAgentProfile: decision.selectedAgentProfile ?? null,
    }
    : observation && observation.conversationId === activeConversationId
    ? {
      executionMode: observation.executionMode,
      routePath: observation.routePath,
      selectedWorkflowName: observation.selectedWorkflowName,
      selectedAgentProfile: observation.selectedAgentProfile,
    }
    : null;

  // 无数据（既无消息级决策，也无属于当前会话的观测）→ 不渲染
  if (!data) {
    return null;
  }

  const modeLabel = t(`cognitiveRoute.executionModeMap.${data.executionMode}`);
  const hasDecision = !!(data.selectedWorkflowName || data.selectedAgentProfile);

  return (
    <div
      style={{
        display: "flex",
        alignItems: "center",
        flexWrap: "wrap",
        gap: 6,
        alignSelf: "flex-start",
        margin: "2px 16px",
        padding: "6px 10px",
        borderRadius: 8,
        border: `1px solid ${token.colorBorderSecondary}`,
        backgroundColor: token.colorBgContainer,
        fontSize: 12,
        maxWidth: "80%",
      }}
    >
      <Text type="secondary" style={{ fontSize: 12, flexShrink: 0 }}>
        {t("cognitiveRoute.branch")}:
      </Text>
      <Tag color="blue" style={{ marginInlineEnd: 0 }}>
        <Route size={11} style={{ verticalAlign: -1, marginRight: 3 }} />
        {data.routePath || t("cognitiveRoute.noRoute")}
      </Tag>
      <Tag style={{ marginInlineEnd: 0 }}>{modeLabel}</Tag>
      {data.selectedWorkflowName && (
        <Tag color="success" icon={<GitBranch size={11} />} style={{ marginInlineEnd: 0 }}>
          {data.selectedWorkflowName}
        </Tag>
      )}
      {data.selectedAgentProfile && (
        <Tag color="geekblue" icon={<Bot size={11} />} style={{ marginInlineEnd: 0 }}>
          {data.selectedAgentProfile.name}
          {data.selectedAgentProfile.role ? ` · ${data.selectedAgentProfile.role}` : ""}
        </Tag>
      )}
      {!hasDecision && (
        <Text type="secondary" style={{ fontSize: 12 }}>
          {t("cognitiveRoute.noDecision")}
        </Text>
      )}
    </div>
  );
}

// SPDX-License-Identifier: AGPL-3.0-only

import { type CognitiveDecisionInfo, strategyKind, type TaskShapeDecision } from "@/types";
import { Tag, theme, Tooltip, Typography } from "antd";
import { Bot, GitBranch, Route, Shield, SplitSquareHorizontal } from "lucide-react";
import { useTranslation } from "react-i18next";

const { Text } = Typography;

/** 决策标签的统一视图模型：仅兼容消息级 `decision` 一种数据源 */
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
  taskShape: TaskShapeDecision | null;
}

/** 认知编排决策分支轻量卡片：展示某条消息自己记录的那一次决策结果。
 * - 仅当传入的 `decision` 真实存在（该消息确实经历过一次认知编排并写入了决策标签）时才渲染。
 * - 无 `decision` 则不渲染——绝不回退到任何全局/最新观测，否则历史消息会串成最新决策，造成渲染混乱。
 * 让用户直观看到该轮选择了哪个工作流、哪个专家/角色、以何种模式执行。
 * P1: 同时展示任务形态决策（原则三标尺：上下文保留成本 × 安全隔离需求 + 推荐策略）。 */
export function CognitiveDecisionCard({ decision }: { decision?: CognitiveDecisionInfo | null }) {
  const { t } = useTranslation();
  const { token } = theme.useToken();

  const data: CognitiveDecisionView | null = decision
    ? {
      executionMode: decision.executionMode,
      routePath: decision.routePath,
      selectedWorkflowName: decision.selectedWorkflowName ?? null,
      selectedAgentProfile: decision.selectedAgentProfile ?? null,
      taskShape: decision.taskShape ?? null,
    }
    : null;

  // 无数据（该消息没有自己的决策标签）→ 不渲染
  if (!data) {
    return null;
  }

  const modeLabel = t(`cognitiveRoute.executionModeMap.${data.executionMode}`);
  const hasDecision = !!(data.selectedWorkflowName || data.selectedAgentProfile);

  // P1: 任务形态标尺标签（原则三输出）
  const ts = data.taskShape;
  const contextCostLabel = ts
    ? t(`cognitiveRoute.taskShape.contextCostMap.${ts.contextCost}`)
    : null;
  const isolationNeedLabel = ts
    ? t(`cognitiveRoute.taskShape.isolationNeedMap.${ts.isolationNeed}`)
    : null;
  const strategyLabel = ts
    ? t(`cognitiveRoute.taskShape.strategyMap.${strategyKind(ts.recommendedStrategy)}`)
    : null;
  // 合并/拆分倾向百分比（用于 Tooltip 详细信息）
  const mergePct = ts ? Math.round(ts.mergeScore * 100) : null;
  const splitPct = ts ? Math.round(ts.splitScore * 100) : null;

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
      {/* P1: 任务形态标尺标签（原则三：上下文成本 × 隔离需求 → 推荐策略） */}
      {ts && (
        <Tooltip
          title={
            <div style={{ fontSize: 12, lineHeight: 1.6 }}>
              <div>{`${t("cognitiveRoute.taskShape.mergeScore")}: ${mergePct}%`}</div>
              <div>{`${t("cognitiveRoute.taskShape.splitScore")}: ${splitPct}%`}</div>
              {ts.evidence.length > 0 && (
                <div style={{ marginTop: 4, opacity: 0.8 }}>
                  {ts.evidence.map((e, i) => <div key={i}>{e}</div>)}
                </div>
              )}
            </div>
          }
        >
          <Tag
            color="purple"
            icon={<SplitSquareHorizontal size={11} />}
            style={{ marginInlineEnd: 0 }}
          >
            {contextCostLabel} · {isolationNeedLabel}
          </Tag>
        </Tooltip>
      )}
      {ts && strategyLabel && (
        <Tag color="magenta" icon={<Shield size={11} />} style={{ marginInlineEnd: 0 }}>
          {strategyLabel}
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

// SPDX-License-Identifier: AGPL-3.0-only

import { useCognitiveRouteStore } from "@/stores";
import { Empty, Tag, theme, Typography } from "antd";
import {
  AlertTriangle,
  Bot,
  CheckCircle2,
  CircleDot,
  Clock,
  GitBranch,
  GitCommitHorizontal,
  Route,
  Shuffle,
} from "lucide-react";
import { useTranslation } from "react-i18next";

const { Text } = Typography;

const EXECUTION_MODE_COLORS: Record<string, string> = {
  ask: "blue",
  plan: "purple",
  act: "geekblue",
  workflow: "green",
  direct: "cyan",
  delegate: "orange",
  parameter_extract: "magenta",
  clarify: "gold",
};

/** 认知编排路由观测面板：展示最近一次 cognitive_query 的三层路由决策明细 */
export function CognitiveRoutePanel() {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const observation = useCognitiveRouteStore((s) => s.observation);

  if (!observation) {
    return (
      <div style={{ padding: 24 }}>
        <Empty description={t("cognitiveRoute.empty")} />
      </div>
    );
  }

  const circuitOpen = observation.circuitBroken;
  const confidencePct = Math.round(observation.confidence * 100);

  return (
    <div style={{ padding: "12px 16px" }}>
      {/* 路由概览 */}
      <div
        style={{
          padding: 12,
          borderRadius: 8,
          border: `1px solid ${circuitOpen ? token.colorErrorBorder : token.colorBorderSecondary}`,
          backgroundColor: circuitOpen
            ? `${token.colorErrorBg}`
            : token.colorBgLayout,
          display: "flex",
          flexDirection: "column",
          gap: 10,
        }}
      >
        <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
          <Route size={14} style={{ color: token.colorPrimary, flexShrink: 0 }} />
          <Text
            strong
            style={{ fontSize: 13, flex: 1, wordBreak: "break-all" }}
          >
            {observation.routePath || t("cognitiveRoute.noRoute")}
          </Text>
        </div>

        <div style={{ display: "flex", flexWrap: "wrap", gap: 6 }}>
          {observation.domain && <Tag>{t("cognitiveRoute.domain")}: {observation.domain}</Tag>}
          {observation.cluster && <Tag>{t("cognitiveRoute.cluster")}: {observation.cluster}</Tag>}
          <Tag
            color={EXECUTION_MODE_COLORS[observation.executionMode] ?? "default"}
          >
            {t("cognitiveRoute.executionMode")}: {t(`cognitiveRoute.executionModeMap.${observation.executionMode}`)}
          </Tag>
          <Tag color={confidencePct >= 80 ? "green" : confidencePct >= 60 ? "orange" : "red"}>
            {t("cognitiveRoute.confidence")}: {confidencePct}%
          </Tag>
          <Tag color="blue">
            <Clock size={11} style={{ verticalAlign: -1, marginRight: 2 }} />
            {observation.totalElapsedMs}ms
          </Tag>
        </div>

        {/* 状态徽标 */}
        <div style={{ display: "flex", flexWrap: "wrap", gap: 6 }}>
          {circuitOpen
            ? (
              <Tag color="error" icon={<AlertTriangle size={11} />}>
                {t("cognitiveRoute.circuitBroken")}
                {observation.circuitBreakReason
                  ? `: ${observation.circuitBreakReason}`
                  : ""}
              </Tag>
            )
            : (
              <Tag color="success" icon={<CheckCircle2 size={11} />}>
                {t("cognitiveRoute.circuitClosed")}
              </Tag>
            )}
          {observation.isLlmFallback && (
            <Tag color="warning" icon={<Shuffle size={11} />}>
              {t("cognitiveRoute.llmFallback")}
            </Tag>
          )}
          {observation.fallbackPath && (
            <Tag icon={<GitCommitHorizontal size={11} />}>
              {t("cognitiveRoute.fallbackPath")}: {observation.fallbackPath}
            </Tag>
          )}
          {observation.filteredCount > 0 && (
            <Tag color="warning" icon={<AlertTriangle size={11} />}>
              {t("cognitiveRoute.filteredCount")}: {observation.filteredCount}
            </Tag>
          )}
        </div>
      </div>

      {/* 执行决策：命中的工作流 / 选中的执行专家（模式中文映射） */}
      {(observation.selectedWorkflowName || observation.selectedAgentProfile) && (
        <div style={{ marginTop: 12 }}>
          <Text strong style={{ fontSize: 13, display: "block", marginBottom: 6 }}>
            {t("cognitiveRoute.branch")}
          </Text>
          <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
            {observation.selectedWorkflowName && (
              <div
                style={{
                  display: "flex",
                  alignItems: "center",
                  gap: 8,
                  padding: "8px 10px",
                  borderRadius: 6,
                  border: `1px solid ${token.colorBorderSecondary}`,
                  backgroundColor: token.colorBgLayout,
                }}
              >
                <GitBranch size={14} style={{ color: token.colorSuccess, flexShrink: 0 }} />
                <Text style={{ fontSize: 12.5, wordBreak: "break-all" }}>
                  {t("cognitiveRoute.workflowName")}: {observation.selectedWorkflowName}
                </Text>
              </div>
            )}
            {observation.selectedAgentProfile && (
              <div
                style={{
                  display: "flex",
                  alignItems: "center",
                  gap: 8,
                  padding: "8px 10px",
                  borderRadius: 6,
                  border: `1px solid ${token.colorBorderSecondary}`,
                  backgroundColor: token.colorBgLayout,
                  flexWrap: "wrap",
                }}
              >
                <Bot size={14} style={{ color: token.colorPrimary, flexShrink: 0 }} />
                <Text style={{ fontSize: 12.5, wordBreak: "break-all" }}>
                  {t("cognitiveRoute.agentProfile")}: {observation.selectedAgentProfile.name}
                </Text>
                {observation.selectedAgentProfile.role && (
                  <Tag>{t("cognitiveRoute.role")}: {observation.selectedAgentProfile.role}</Tag>
                )}
                {observation.selectedAgentProfile.expert && (
                  <Tag>{t("cognitiveRoute.expert")}: {observation.selectedAgentProfile.expert}</Tag>
                )}
              </div>
            )}
          </div>
        </div>
      )}

      {/* 候选能力详情 */}
      {observation.candidateDetails.length > 0 && (
        <div style={{ marginTop: 12 }}>
          <Text strong style={{ fontSize: 13, display: "block", marginBottom: 6 }}>
            {t("cognitiveRoute.candidates")} ({observation.candidateDetails.length})
          </Text>
          <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
            {observation.candidateDetails.map((c) => {
              const scorePct = Math.round(c.score * 100);
              return (
                <div
                  key={c.capabilityId}
                  style={{
                    padding: "8px 10px",
                    borderRadius: 6,
                    border: `1px solid ${token.colorBorderSecondary}`,
                    backgroundColor: token.colorBgLayout,
                  }}
                >
                  <div style={{ display: "flex", alignItems: "center", gap: 6, marginBottom: 4 }}>
                    <Text strong style={{ fontSize: 12.5, wordBreak: "break-all" }}>
                      {c.name || c.capabilityId}
                    </Text>
                    <Tag
                      style={{ fontSize: 11, marginLeft: "auto", flexShrink: 0 }}
                      color={scorePct >= 80 ? "green" : scorePct >= 60 ? "orange" : "red"}
                    >
                      {scorePct}%
                    </Tag>
                  </div>
                  {c.description && (
                    <Text
                      type="secondary"
                      style={{ fontSize: 12, display: "block", marginBottom: 4, lineHeight: 1.5 }}
                    >
                      {c.description}
                    </Text>
                  )}
                  <div style={{ display: "flex", gap: 4, flexWrap: "wrap" }}>
                    <Tag style={{ fontSize: 11 }}>{c.kind}</Tag>
                    {c.domain && <Tag style={{ fontSize: 11 }}>{c.domain}</Tag>}
                    {c.cluster && <Tag style={{ fontSize: 11 }}>{c.cluster}</Tag>}
                  </div>
                </div>
              );
            })}
          </div>
        </div>
      )}

      {/* 阶段执行记录 */}
      {observation.stageRecords.length > 0 && (
        <div style={{ marginTop: 12 }}>
          <Text strong style={{ fontSize: 13, display: "block", marginBottom: 6 }}>
            {t("cognitiveRoute.stageRecords")} ({observation.stageRecords.length})
          </Text>
          <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
            {observation.stageRecords.map((stage, idx) => {
              const failed = !stage.success;
              return (
                <div
                  key={`${stage.stage}-${idx}`}
                  style={{
                    display: "flex",
                    gap: 8,
                    padding: "8px 10px",
                    borderRadius: 6,
                    border: `1px solid ${failed ? token.colorErrorBorder : token.colorBorderSecondary}`,
                    backgroundColor: failed
                      ? `${token.colorErrorBg}`
                      : "transparent",
                  }}
                >
                  <div
                    style={{
                      marginTop: 2,
                      flexShrink: 0,
                      color: failed
                        ? token.colorError
                        : stage.success
                        ? token.colorSuccess
                        : token.colorTextTertiary,
                    }}
                  >
                    <CircleDot size={14} />
                  </div>
                  <div style={{ flex: 1, minWidth: 0 }}>
                    <div
                      style={{
                        display: "flex",
                        justifyContent: "space-between",
                        gap: 8,
                        alignItems: "center",
                      }}
                    >
                      <Text strong style={{ fontSize: 12.5, wordBreak: "break-all" }}>
                        {stage.stage}
                      </Text>
                      <Text type="secondary" style={{ fontSize: 11, flexShrink: 0 }}>
                        {stage.elapsedMs}ms
                      </Text>
                    </div>
                    {stage.summary && (
                      <Text
                        type="secondary"
                        style={{
                          fontSize: 12,
                          display: "block",
                          marginTop: 2,
                          lineHeight: 1.5,
                        }}
                      >
                        {stage.summary}
                      </Text>
                    )}
                    {!failed && (
                      <Text
                        type="secondary"
                        style={{ fontSize: 11, display: "block", marginTop: 2 }}
                      >
                        {t("cognitiveRoute.confidence")}: {Math.round(stage.confidence * 100)}%
                      </Text>
                    )}
                  </div>
                </div>
              );
            })}
          </div>
        </div>
      )}

      {/* 观测时间 */}
      <Text
        type="secondary"
        style={{ fontSize: 11, display: "block", marginTop: 10 }}
      >
        {t("cognitiveRoute.recordedAt")}: {new Date(observation.recordedAt).toLocaleTimeString()}
      </Text>
    </div>
  );
}

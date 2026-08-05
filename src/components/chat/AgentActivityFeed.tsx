// SPDX-License-Identifier: AGPL-3.0-only

import { Badge, Card, Empty, List, Segmented, Space, Tag, Timeline, Typography } from "antd";
import { Activity, CheckCircle, Clock, Loader2, PauseCircle, XCircle } from "lucide-react";
import React, { useCallback, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

const { Text } = Typography;

// ── 类型定义 ──

export type AgentStatus =
  | "idle"
  | "running"
  | "waiting_approval"
  | "completed"
  | "error";

export type EventType =
  | "step_started"
  | "step_completed"
  | "step_failed"
  | "agent_status_change"
  | "tool_invocation"
  | "user_interaction"
  | "milestone_reached"
  | "approval_required"
  | "feedback_collected";

export interface AgentProfile {
  id: string;
  name: string;
  icon: string;
  color: string;
  role: string;
}

export interface AgentEvent {
  id: string;
  timestamp: number;
  agent_id: string;
  agent_name: string;
  agent_icon: string;
  event_type: EventType;
  step_id?: string;
  step_title?: string;
  message: string;
  details?: Record<string, unknown>;
  status?: AgentStatus;
}

// ── Agent 配置映射 ──

const AGENT_PROFILES: Record<string, AgentProfile> = {
  code_auditor: { id: "code_auditor", name: "代码审计专家", icon: "🔍", color: "#52c41a", role: "代码审计专家" },
  architect_analyst: { id: "architect_analyst", name: "架构分析师", icon: "🏗️", color: "#1890ff", role: "架构分析师" },
  quality_expert: { id: "quality_expert", name: "代码质量专家", icon: "📊", color: "#722ed1", role: "代码质量专家" },
  behavior_tester: { id: "behavior_tester", name: "行为测试专家", icon: "🧪", color: "#13c2c2", role: "行为测试专家" },
  knowledge_engineer: {
    id: "knowledge_engineer",
    name: "知识提炼专家",
    icon: "💡",
    color: "#faad14",
    role: "知识提炼专家",
  },
  refactor_consultant: { id: "refactor_consultant", name: "重构顾问", icon: "⚠️", color: "#fa541c", role: "重构顾问" },
  solution_architect: { id: "solution_architect", name: "架构师", icon: "🎯", color: "#2f54eb", role: "架构师" },
  tech_project_manager: {
    id: "tech_project_manager",
    name: "技术项目经理",
    icon: "📋",
    color: "#eb2f96",
    role: "技术项目经理",
  },
  change_manager: { id: "change_manager", name: "变更管理专家", icon: "🔄", color: "#a0d911", role: "变更管理专家" },
  quality_engineer: { id: "quality_engineer", name: "质量工程师", icon: "✅", color: "#52c41a", role: "质量工程师" },
  devops_engineer: {
    id: "devops_engineer",
    name: "DevOps 工程师",
    icon: "⚙️",
    color: "#595959",
    role: "DevOps 工程师",
  },
  code_reviewer: { id: "code_reviewer", name: "代码审查员", icon: "👁️", color: "#2f54eb", role: "代码审查员" },
  senior_engineer: { id: "senior_engineer", name: "高级工程师", icon: "💻", color: "#1890ff", role: "高级工程师" },
  behavior_verifier: {
    id: "behavior_verifier",
    name: "行为验证专家",
    icon: "🔬",
    color: "#722ed1",
    role: "行为验证专家",
  },
  test_engineer: { id: "test_engineer", name: "测试工程师", icon: "🧪", color: "#13c2c2", role: "测试工程师" },
  integration_engineer: {
    id: "integration_engineer",
    name: "集成测试工程师",
    icon: "🔗",
    color: "#fa8c16",
    role: "集成测试工程师",
  },
  quality_director: { id: "quality_director", name: "质量总监", icon: "🎖️", color: "#cf1322", role: "质量总监" },
  tech_writer: { id: "tech_writer", name: "技术文档工程师", icon: "📝", color: "#faad14", role: "技术文档工程师" },
  ops_engineer: { id: "ops_engineer", name: "运维工程师", icon: "🔧", color: "#595959", role: "运维工程师" },
  project_manager: { id: "project_manager", name: "项目经理", icon: "📊", color: "#eb2f96", role: "项目经理" },
};

// ── 状态图标映射 ──

function StatusIcon({ status }: { status: AgentStatus; label?: string }) {
  switch (status) {
    case "running":
      return <Loader2 className="w-4 h-4 animate-spin text-blue-500" />;
    case "waiting_approval":
      return <PauseCircle className="w-4 h-4 text-amber-500" />;
    case "completed":
      return <CheckCircle className="w-4 h-4 text-green-500" />;
    case "error":
      return <XCircle className="w-4 h-4 text-red-500" />;
    default:
      return <Clock className="w-4 h-4 text-gray-400" />;
  }
}

// ── 事件类型标签 ──

function EventTypeTag({ type }: { type: EventType }) {
  const { t } = useTranslation();
  const config: Record<EventType, { color: string; key: string }> = {
    step_started: { color: "blue", key: "agentActivity.eventType.stepStarted" },
    step_completed: { color: "green", key: "agentActivity.eventType.stepCompleted" },
    step_failed: { color: "red", key: "agentActivity.eventType.stepFailed" },
    agent_status_change: { color: "purple", key: "agentActivity.eventType.agentStatusChange" },
    tool_invocation: { color: "cyan", key: "agentActivity.eventType.toolInvocation" },
    user_interaction: { color: "orange", key: "agentActivity.eventType.userInteraction" },
    milestone_reached: { color: "gold", key: "agentActivity.eventType.milestoneReached" },
    approval_required: { color: "magenta", key: "agentActivity.eventType.approvalRequired" },
    feedback_collected: { color: "default", key: "agentActivity.eventType.feedbackCollected" },
  };
  const cfg = config[type];
  return <Tag color={cfg.color}>{t(cfg.key)}</Tag>;
}

// ── 活动组件属性 ──

interface AgentActivityFeedProps {
  events: AgentEvent[];
  activeAgents: AgentProfile[];
  agentStatuses: Record<string, AgentStatus>;
  workflowId?: string;
}

// ── 主组件 ──

export const AgentActivityFeed: React.FC<AgentActivityFeedProps> = ({
  events,
  activeAgents,
  agentStatuses,
}) => {
  const { t } = useTranslation();
  const [filter, setFilter] = useState<"all" | "running" | "waiting" | "completed">("all");
  const [selectedAgent, setSelectedAgent] = useState<string | null>(null);

  // 过滤事件
  const filteredEvents = useMemo(() => {
    let result = events;

    if (selectedAgent) {
      result = result.filter((e) => e.agent_id === selectedAgent);
    }

    return result.slice(0, 100); // 最多显示 100 条
  }, [events, selectedAgent]);

  // 筛选活跃 Agent
  const filteredAgents = useMemo(() => {
    if (filter === "all") { return activeAgents; }
    const statusMap: Record<string, AgentStatus> = {
      running: "running",
      waiting: "waiting_approval",
      completed: "completed",
    };
    const targetStatus = statusMap[filter];
    return activeAgents.filter((a) => agentStatuses[a.id] === targetStatus);
  }, [activeAgents, agentStatuses, filter]);

  const formatTime = useCallback((timestamp: number) => {
    const date = new Date(timestamp);
    return date.toLocaleTimeString("zh-CN", { hour: "2-digit", minute: "2-digit", second: "2-digit" });
  }, []);

  return (
    <Card
      title={
        <Space>
          <Activity className="w-5 h-5 text-blue-500" />
          <span>{t("agentActivity.title")}</span>
          <Badge count={activeAgents.filter((a) => agentStatuses[a.id] === "running").length} />
        </Space>
      }
      size="small"
      className="agent-activity-feed"
    >
      {/* 过滤器 */}
      <div className="mb-4">
        <Segmented
          size="small"
          value={filter}
          onChange={(val) => setFilter(val as typeof filter)}
          options={[
            { label: t("agentActivity.filter.all"), value: "all" },
            { label: t("agentActivity.filter.running"), value: "running" },
            { label: t("agentActivity.filter.waiting"), value: "waiting" },
            { label: t("agentActivity.filter.completed"), value: "completed" },
          ]}
        />
      </div>

      {/* Agent 状态卡片 */}
      {filteredAgents.length > 0 && (
        <List
          size="small"
          className="mb-4"
          bordered
          dataSource={filteredAgents}
          renderItem={(agent) => {
            const status = agentStatuses[agent.id] || "idle";
            return (
              <List.Item
                onClick={() => setSelectedAgent(selectedAgent === agent.id ? null : agent.id)}
                style={{
                  cursor: "pointer",
                  padding: "8px 12px",
                  background: selectedAgent === agent.id ? `${agent.color}15` : undefined,
                  borderLeft: `3px solid ${agent.color}`,
                }}
              >
                <Space>
                  <span className="text-xl">{agent.icon}</span>
                  <div>
                    <Text strong>{agent.name}</Text>
                    <div className="text-xs text-gray-500">{agent.role}</div>
                  </div>
                </Space>
                <Space>
                  <StatusIcon status={status} label={t(`agentActivity.status.${status}`)} />
                  <Text type="secondary" className="text-xs">
                    {t(`agentActivity.status.${status}`)}
                  </Text>
                </Space>
              </List.Item>
            );
          }}
        />
      )}

      {/* 事件时间线 */}
      <div className="mt-4">
        <Text type="secondary" className="text-sm mb-2 block">
          {t("agentActivity.activityLog")}
        </Text>
        {filteredEvents.length > 0
          ? (
            <Timeline
              className="agent-timeline"
              items={filteredEvents.map((event) => ({
                color: event.status === "error"
                  ? "red"
                  : event.status === "completed"
                  ? "green"
                  : event.status === "waiting_approval"
                  ? "orange"
                  : "blue",
                children: (
                  <div className="py-1">
                    <div className="flex items-center gap-2">
                      <span className="text-base">{event.agent_icon}</span>
                      <Text strong className="text-sm">
                        {event.agent_name}
                      </Text>
                      <EventTypeTag type={event.event_type} />
                      <Text type="secondary" className="text-xs ml-auto">
                        {formatTime(event.timestamp)}
                      </Text>
                    </div>
                    <div className="ml-7 text-sm">
                      <Text>{event.message}</Text>
                      {event.step_title && (
                        <div className="text-xs text-gray-500 mt-1">
                          {t("agentActivity.stepLabel")}: {event.step_title}
                        </div>
                      )}
                    </div>
                  </div>
                ),
              }))}
            />
          )
          : (
            <Empty
              description={selectedAgent
                ? t("agentActivity.emptyState.noFilteredActivity")
                : t("agentActivity.emptyState.noActivity")}
              image={Empty.PRESENTED_IMAGE_SIMPLE}
            />
          )}
      </div>
    </Card>
  );
};

// ── 导出辅助函数 ──

export function createAgentEvent(
  agentId: string,
  eventType: EventType,
  message: string,
  stepId?: string,
  stepTitle?: string,
  status?: AgentStatus,
  details?: Record<string, unknown>,
): AgentEvent {
  const profile = AGENT_PROFILES[agentId] || {
    id: agentId,
    name: agentId,
    icon: "🤖",
    color: "#999",
    role: agentId,
  };

  return {
    id: `${Date.now()}-${Math.random().toString(36).slice(2, 9)}`,
    timestamp: Date.now(),
    agent_id: agentId,
    agent_name: profile.name,
    agent_icon: profile.icon,
    event_type: eventType,
    step_id: stepId,
    step_title: stepTitle,
    message,
    status,
    details,
  };
}

export function getAgentProfile(agentId: string): AgentProfile | undefined {
  return AGENT_PROFILES[agentId];
}

export function getAllAgentProfiles(): AgentProfile[] {
  return Object.values(AGENT_PROFILES);
}

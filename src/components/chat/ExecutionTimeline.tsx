import { useAgentStore, usePlanStore } from "@/stores";
import type { PlanStep } from "@/types";
import type { AgentPoolItem, ToolCallState } from "@/types/agent";

const _EMPTY: never[] = [];
import { SyncOutlined } from "@ant-design/icons";
import { Progress, Tag, theme, Timeline, Typography } from "antd";
import type { TimelineItemProps } from "antd";
import {
  AlertTriangle,
  Bot,
  CheckCircle2,
  Clock,
  FileEdit,
  GitBranch,
  ListChecks,
  Search,
  Terminal,
  Wrench,
  Zap,
} from "lucide-react";
import React, { useMemo } from "react";
import { useTranslation } from "react-i18next";

// ── Types ────────────────────────────────────────────────────────────────

type TimelineEventType = "plan" | "tool_call" | "agent_pool" | "agent_status";

interface TimelineEvent {
  id: string;
  type: TimelineEventType;
  timestamp: number;
  title: string;
  description?: string;
  status: "pending" | "running" | "completed" | "failed" | "cancelled";
  progress?: number;
  detail?: string;
  source: PlanStep | ToolCallState | AgentPoolItem | string;
}

// ── Event construction helpers ───────────────────────────────────────────

function planStepToEvent(step: PlanStep, planCreatedAt: number): TimelineEvent {
  return {
    id: `plan:${step.id}`,
    type: "plan",
    timestamp: planCreatedAt + (step.status === "completed" ? 1000 : 0),
    title: step.title,
    description: step.description || undefined,
    status: step.status === "approved" ? "pending" : step.status as TimelineEvent["status"],
    detail: step.result ?? undefined,
    source: step,
  };
}

function toolCallToEvent(tc: ToolCallState): TimelineEvent {
  const statusMap: Record<string, TimelineEvent["status"]> = {
    queued: "pending",
    running: "running",
    success: "completed",
    failed: "failed",
    cancelled: "cancelled",
  };
  const toolName = tc.toolName || "unknown";
  const inputSummary = typeof tc.input === "string"
    ? String(tc.input).slice(0, 80)
    : JSON.stringify(tc.input || {}).slice(0, 80);

  return {
    id: `tool:${tc.toolUseId}`,
    type: "tool_call",
    timestamp: Date.now() - (tc.executionStatus === "success" ? 2000 : 0),
    title: toolName,
    description: inputSummary + (inputSummary.length >= 80 ? "…" : ""),
    status: statusMap[tc.executionStatus] || "pending",
    detail: tc.output?.slice(0, 200),
    source: tc,
  };
}

function poolItemToEvent(item: AgentPoolItem): TimelineEvent {
  return {
    id: `pool:${item.id}`,
    type: "agent_pool",
    timestamp: item.startedAt || Date.now(),
    title: item.name || item.taskDescription || item.type,
    description: item.summary || item.taskDescription,
    status: item.status === "completed"
      ? "completed"
      : item.status === "failed"
      ? "failed"
      : item.status === "cancelled"
      ? "cancelled"
      : item.status === "running"
      ? "running"
      : "pending",
    progress: item.progress,
    detail: item.error || undefined,
    source: item,
  };
}

// ── Status colors and icons ──────────────────────────────────────────────

const statusConfig: Record<string, { color: string; icon: React.ReactNode }> = {
  pending: { color: "#d9d9d9", icon: <Clock size={12} /> },
  running: { color: "#1890ff", icon: <SyncOutlined spin style={{ fontSize: 12 }} /> },
  completed: { color: "#52c41a", icon: <CheckCircle2 size={12} /> },
  failed: { color: "#ff4d4f", icon: <AlertTriangle size={12} /> },
  cancelled: { color: "#8c8c8c", icon: <AlertTriangle size={12} /> },
};

const typeIcons: Record<TimelineEventType, React.ReactNode> = {
  plan: <ListChecks size={14} />,
  tool_call: <Wrench size={14} />,
  agent_pool: <Bot size={14} />,
  agent_status: <Zap size={14} />,
};

const typeColors: Record<TimelineEventType, string> = {
  plan: "#1890ff",
  tool_call: "#52c41a",
  agent_pool: "#fa8c16",
  agent_status: "#722ed1",
};

// ── Tool icon mapping ────────────────────────────────────────────────────

const toolIcons: Record<string, React.ReactNode> = {
  bash: <Terminal size={12} />,
  write: <FileEdit size={12} />,
  edit: <FileEdit size={12} />,
  read: <Search size={12} />,
  glob: <Search size={12} />,
  grep: <Search size={12} />,
  ls: <Search size={12} />,
};

function getToolIcon(name: string): React.ReactNode {
  const lower = name.toLowerCase();
  for (const [key, icon] of Object.entries(toolIcons)) {
    if (lower.includes(key)) { return icon; }
  }
  return <Wrench size={12} />;
}

// ── Component ────────────────────────────────────────────────────────────

interface ExecutionTimelineProps {
  conversationId: string;
  maxItems?: number;
}

export const ExecutionTimeline = React.memo(
  function ExecutionTimeline({ conversationId, maxItems = 50 }: ExecutionTimelineProps) {
    const { t } = useTranslation();
    const { token } = theme.useToken();

    // Read from all three stores
    const plan = usePlanStore((s) => s.activePlans[conversationId]);
    const toolCalls = useAgentStore((s) => s.toolCalls);
    const poolItems = useAgentStore((s) => s.agentPool[conversationId] ?? _EMPTY);
    const agentStatus = useAgentStore((s) => s.agentStatus[conversationId]);

    const events = useMemo(() => {
      const result: TimelineEvent[] = [];

      // Plan steps
      if (plan) {
        const planTs = plan.created_at * 1000;
        for (const step of plan.steps) {
          result.push(planStepToEvent(step, planTs));
        }
      }

      // Tool calls for this conversation
      for (const tc of Object.values(toolCalls)) {
        // Only include tool calls that belong to this conversation
        const poolItem = poolItems.find((p) => p.id === tc.toolUseId);
        if (poolItem) {
          result.push(toolCallToEvent(tc));
        }
      }

      // Agent pool items
      for (const item of poolItems) {
        result.push(poolItemToEvent(item));
      }

      // Agent status
      if (agentStatus) {
        result.push({
          id: "agent-status",
          type: "agent_status",
          timestamp: Date.now(),
          title: agentStatus,
          status: "running",
          source: agentStatus,
        });
      }

      // Sort by timestamp descending (newest first), running items stay on top
      result.sort((a, b) => {
        if (a.status === "running" && b.status !== "running") { return -1; }
        if (a.status !== "running" && b.status === "running") { return 1; }
        return b.timestamp - a.timestamp;
      });

      return result.slice(0, maxItems);
    }, [plan, toolCalls, poolItems, agentStatus, maxItems]);

    if (events.length === 0) {
      return (
        <div style={{ textAlign: "center", padding: 24, color: token.colorTextQuaternary, fontSize: 13 }}>
          {t("chat.timeline.empty", "暂无执行记录")}
        </div>
      );
    }

    const timelineItems: TimelineItemProps[] = events.map((evt) => {
      const sc = statusConfig[evt.status] || statusConfig.pending;
      const tc = typeColors[evt.type];

      return {
        key: evt.id,
        color: sc.color,
        dot: evt.status === "running"
          ? <SyncOutlined spin style={{ fontSize: 14, color: "#1890ff" }} />
          : (
            <span
              style={{
                display: "inline-flex",
                alignItems: "center",
                justifyContent: "center",
                width: 20,
                height: 20,
                borderRadius: "50%",
                backgroundColor: sc.color + "20",
                color: sc.color,
              }}
            >
              {sc.icon}
            </span>
          ),
        children: (
          <div
            style={{
              display: "flex",
              flexDirection: "column",
              gap: 4,
              opacity: evt.status === "cancelled" ? 0.5 : 1,
            }}
          >
            {/* Header: type badge + title + status */}
            <div style={{ display: "flex", alignItems: "center", gap: 6, flexWrap: "wrap" }}>
              <Tag
                color={tc}
                style={{ margin: 0, fontSize: 10, padding: "0 4px", lineHeight: "18px" }}
              >
                {typeIcons[evt.type]}
                <span style={{ marginLeft: 4 }}>
                  {evt.type === "plan"
                    ? t("chat.timeline.plan", "计划")
                    : evt.type === "tool_call"
                    ? t("chat.timeline.tool", "工具")
                    : evt.type === "agent_pool"
                    ? t("chat.timeline.agent", "智能体")
                    : t("chat.timeline.status", "状态")}
                </span>
              </Tag>
              {/* Tool-specific icon */}
              {evt.type === "tool_call" && (evt.source as ToolCallState).toolName && (
                <span style={{ display: "inline-flex", color: token.colorTextSecondary }}>
                  {getToolIcon((evt.source as ToolCallState).toolName)}
                </span>
              )}
              <Typography.Text strong style={{ fontSize: 13 }}>
                {evt.title}
              </Typography.Text>
              <Tag
                color={evt.status === "completed"
                  ? "green"
                  : evt.status === "failed"
                  ? "red"
                  : evt.status === "running"
                  ? "blue"
                  : "default"}
                style={{ margin: 0, fontSize: 10, padding: "0 4px", lineHeight: "18px" }}
              >
                {evt.status === "completed"
                  ? t("chat.timeline.completed", "完成")
                  : evt.status === "failed"
                  ? t("chat.timeline.failed", "失败")
                  : evt.status === "running"
                  ? t("chat.timeline.running", "运行中")
                  : evt.status === "cancelled"
                  ? t("chat.timeline.cancelled", "已取消")
                  : t("chat.timeline.pending", "等待中")}
              </Tag>
            </div>

            {/* Description */}
            {evt.description && (
              <Typography.Text
                type="secondary"
                style={{ fontSize: 11, fontFamily: "monospace" }}
                ellipsis
              >
                {evt.description}
              </Typography.Text>
            )}

            {/* Progress bar */}
            {evt.progress != null && evt.progress > 0 && evt.status === "running" && (
              <Progress
                percent={evt.progress}
                size="small"
                strokeColor={tc}
                style={{ maxWidth: 200 }}
              />
            )}

            {/* Detail / result */}
            {evt.detail && evt.status === "completed" && (
              <Typography.Text
                type="secondary"
                style={{ fontSize: 11, color: token.colorTextTertiary }}
                ellipsis
              >
                {evt.detail}
              </Typography.Text>
            )}

            {/* Error */}
            {evt.status === "failed" && evt.detail && (
              <div
                style={{
                  padding: "4px 8px",
                  fontSize: 11,
                  backgroundColor: token.colorErrorBg,
                  borderRadius: token.borderRadiusSM,
                  color: token.colorError,
                }}
              >
                {evt.detail}
              </div>
            )}
          </div>
        ),
      };
    });

    // Summary stats
    const completed = events.filter((e) => e.status === "completed").length;
    const running = events.filter((e) => e.status === "running").length;
    const failed = events.filter((e) => e.status === "failed").length;
    const total = events.length;

    return (
      <div style={{ padding: "0 4px" }} role="region" aria-label={t("chat.timeline.title", "执行时间线")}>
        {/* Summary bar */}
        <div
          style={{
            display: "flex",
            alignItems: "center",
            gap: 12,
            padding: "6px 12px",
            marginBottom: 8,
            borderRadius: token.borderRadiusSM,
            backgroundColor: token.colorFillQuaternary,
            fontSize: 12,
          }}
          role="status"
          aria-live="polite"
        >
          <GitBranch size={14} style={{ color: token.colorPrimary }} />
          <span style={{ fontWeight: 500 }}>
            {t("chat.timeline.title", "执行时间线")}
          </span>
          <span style={{ color: token.colorTextSecondary }}>
            {total} 个事件
          </span>
          {completed > 0 && (
            <Tag color="green" style={{ margin: 0, fontSize: 10 }}>
              {completed} 完成
            </Tag>
          )}
          {running > 0 && (
            <Tag color="blue" style={{ margin: 0, fontSize: 10 }}>
              {running} 运行中
            </Tag>
          )}
          {failed > 0 && (
            <Tag color="red" style={{ margin: 0, fontSize: 10 }}>
              {failed} 失败
            </Tag>
          )}
          <div style={{ flex: 1 }} />
          <Progress
            percent={total > 0 ? Math.round((completed / total) * 100) : 0}
            size="small"
            style={{ width: 80, margin: 0 }}
            strokeColor={token.colorSuccess}
          />
        </div>

        {/* Timeline */}
        <div style={{ maxHeight: 400, overflow: "auto", padding: "0 4px" }}>
          <Timeline items={timelineItems} />
        </div>
      </div>
    );
  },
);

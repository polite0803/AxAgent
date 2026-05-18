import { useExecutionStore } from "@/stores/feature/executionStore";
import type { AgentPoolItem, AgentPoolSummary, WorkerMessage } from "@/types";
import { CheckCircleOutlined, CloseCircleOutlined, LoadingOutlined, RightOutlined } from "@ant-design/icons";

const _EMPTY: never[] = [];
import { AlertTriangle, ChevronDown, ChevronRight, Clock, SkipForward } from "lucide-react";
import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router-dom";
import { useScrollToMessage } from "./ScrollToMessageContext";
import { SubAgentCard } from "./SubAgentCard";
import "./AgentPoolPanel.css";

// ---------------------------------------------------------------------------
// Agent 类型颜色和图标
// ---------------------------------------------------------------------------

const AGENT_COLORS: Record<string, string> = {
  explore: "#1890ff",
  general: "#722ed1",
  build: "#52c41a",
  plan: "#fa8c16",
  research: "#eb2f96",
  review: "#13c2c2",
};

function hashColor(str: string): string {
  let hash = 0;
  for (let i = 0; i < str.length; i++) {
    hash = str.charCodeAt(i) + ((hash << 5) - hash);
  }
  const hue = Math.abs(hash % 360);
  return `hsl(${hue}, 60%, 45%)`;
}

function getTypeColor(type: string, agentType?: string): string {
  switch (type) {
    case "sub_agent":
      return agentType
        ? (AGENT_COLORS[agentType] ?? hashColor(agentType))
        : "#722ed1";
    case "worker":
      return "#fa8c16";
    case "workflow_step":
      return "#13c2c2";
    default:
      return "#8c8c8c";
  }
}

function getTypeIcon(type: string, agentType?: string): string {
  switch (type) {
    case "sub_agent":
      if (agentType === "explore") {
        return "🔍";
      }
      if (agentType === "build") {
        return "🏗";
      }
      if (agentType === "plan") {
        return "📋";
      }
      if (agentType === "research") {
        return "🔬";
      }
      if (agentType === "review") {
        return "✅";
      }
      return "🔧";
    case "worker":
      return "⚡";
    case "workflow_step":
      return "📌";
    default:
      return "🤖";
  }
}

function getTypeLabel(type: string): string {
  switch (type) {
    case "sub_agent":
      return "Sub-Agent";
    case "worker":
      return "Worker";
    case "workflow_step":
      return "Step";
    default:
      return type;
  }
}

// ---------------------------------------------------------------------------
// 状态标记
// ---------------------------------------------------------------------------

function StatusBadge({ status }: { status: AgentPoolItem["status"] }) {
  const { t } = useTranslation();
  const config: Record<
    string,
    { icon: React.ReactNode; color: string; label: string }
  > = {
    pending: {
      icon: <Clock size={13} />,
      color: "#8c8c8c",
      label: t("agentPool.status.pending"),
    },
    running: {
      icon: <LoadingOutlined spin style={{ fontSize: 13 }} />,
      color: "#1890ff",
      label: t("agentPool.status.running"),
    },
    completed: {
      icon: <CheckCircleOutlined style={{ fontSize: 13 }} />,
      color: "#52c41a",
      label: t("agentPool.status.completed"),
    },
    failed: {
      icon: <CloseCircleOutlined style={{ fontSize: 13 }} />,
      color: "#ff4d4f",
      label: t("agentPool.status.failed"),
    },
    cancelled: {
      icon: <SkipForward size={13} />,
      color: "#faad14",
      label: t("agentPool.status.cancelled"),
    },
  };
  const c = config[status] || config.pending;
  return (
    <span className="pool-item__status" style={{ color: c.color }}>
      {c.icon}
      <span style={{ marginLeft: 4, fontSize: 12 }}>{c.label}</span>
    </span>
  );
}

// ---------------------------------------------------------------------------
// 消息日志（Worker 专用）
// ---------------------------------------------------------------------------

function WorkerMessageLog({ messages }: { messages?: WorkerMessage[] }) {
  const { t } = useTranslation();
  const [expanded, setExpanded] = useState(false);
  if (!messages || messages.length === 0) {
    return null;
  }

  return (
    <div className="worker-msg-log">
      <button
        type="button"
        className="worker-msg-log__toggle"
        onClick={(e) => {
          e.stopPropagation();
          setExpanded(!expanded);
        }}
      >
        {expanded ? <ChevronDown size={12} /> : <ChevronRight size={12} />}
        {t("agentPool.messageLog", { count: messages.length })}
      </button>
      {expanded && (
        <div className="worker-msg-log__list">
          {/* message log appended sequentially, safe to use index as key */}
          {messages.map((msg, i) => (
            <div
              key={`${msg.workerId}-${msg.timestamp || i}`}
              className={`worker-msg-log__entry worker-msg-log__entry--${msg.messageType}`}
            >
              <span className="worker-msg-log__type">{msg.messageType}</span>
              <span className="worker-msg-log__content">{msg.content}</span>
              {msg.timestamp && (
                <span className="worker-msg-log__time">
                  {new Date(msg.timestamp).toLocaleTimeString()}
                </span>
              )}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// 单条目卡片
// ---------------------------------------------------------------------------

function PoolItemCard({ item }: { item: AgentPoolItem }) {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const { scrollTo } = useScrollToMessage();
  const color = getTypeColor(item.type, item.agentType);
  const icon = getTypeIcon(item.type, item.agentType);
  const typeLabel = getTypeLabel(item.type);
  const isRunning = item.status === "running";
  const isCompleted = item.status === "completed";
  const isFailed = item.status === "failed";

  const handleClick = () => {
    // 优先跳转到关联消息
    if (item.messageId && !isRunning) {
      scrollTo(item.messageId);
      return;
    }
    // 子 Agent 回退：跳转子会话
    if (item.type === "sub_agent" && item.childConversationId && !isRunning) {
      navigate(`/chat/${item.childConversationId}`);
    }
  };

  const clickable = (!isRunning && item.messageId)
    || (item.type === "sub_agent" && item.childConversationId && !isRunning);

  return (
    <div
      className={`pool-item ${isRunning ? "pool-item--running" : ""} ${isCompleted ? "pool-item--completed" : ""} ${
        isFailed ? "pool-item--failed" : ""
      }`}
      onClick={handleClick}
      role="button"
      tabIndex={clickable ? 0 : -1}
      onKeyDown={(e) => {
        if ((e.key === "Enter" || e.key === " ") && clickable) {
          e.preventDefault();
          handleClick();
        }
      }}
      style={{ cursor: clickable ? "pointer" : "default" }}
      data-component="agent-pool-item"
    >
      {/* 头部 */}
      <div className="pool-item__header">
        <span className="pool-item__icon">{icon}</span>
        <span className="pool-item__name" style={{ color }}>
          {item.name}
        </span>
        <span
          className="pool-item__type-tag"
          style={{ borderColor: color + "40", color }}
        >
          {typeLabel}
        </span>
        <StatusBadge status={item.status} />
      </div>

      {/* 描述/摘要 */}
      {(item.summary || item.taskDescription) && (
        <div className="pool-item__desc">
          {item.summary || item.taskDescription}
        </div>
      )}

      {/* 错误 */}
      {item.error && (
        <div className="pool-item__error">
          <AlertTriangle size={12} /> {item.error}
        </div>
      )}

      {/* 进度条 */}
      {item.progress !== undefined && item.progress > 0 && (
        <div className="pool-item__progress">
          <div
            className="pool-item__progress-bar"
            style={{
              width: `${Math.min(item.progress, 100)}%`,
              backgroundColor: color,
            }}
          />
        </div>
      )}

      {/* 元信息 */}
      <div className="pool-item__meta">
        {item.agentRole && <span className="pool-item__role">{item.agentRole}</span>}
        {item.duration !== undefined && isCompleted && (
          <span className="pool-item__duration">
            {(item.duration / 1000).toFixed(1)}s
          </span>
        )}
        {item.attempts !== undefined && item.maxRetries !== undefined && (
          <span className="pool-item__attempts">
            {t("agentPool.attempts", {
              attempts: item.attempts,
              maxRetries: item.maxRetries,
            })}
          </span>
        )}
      </div>

      {/* Worker 消息日志 */}
      {item.type === "worker" && <WorkerMessageLog messages={item.messages} />}

      {/* 子会话跳转 */}
      {item.type === "sub_agent" && isCompleted && item.childConversationId && (
        <div className="pool-item__action">
          {t("agentPool.viewSubSession")} <RightOutlined />
        </div>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// 汇总栏
// ---------------------------------------------------------------------------

function PoolSummaryBar({ summary }: { summary: AgentPoolSummary }) {
  const { t } = useTranslation();
  if (summary.total === 0) {
    return null;
  }

  return (
    <div className="pool-summary">
      <div className="pool-summary__stats">
        <span className="pool-summary__label">
          Agent Pool ({summary.completed}/{summary.total})
        </span>
        <div className="pool-summary__bar">
          <div
            className="pool-summary__fill"
            style={{ width: `${summary.pctComplete}%` }}
          />
        </div>
        <div className="pool-summary__counts">
          {summary.running > 0 && (
            <span className="pool-summary__count pool-summary__count--running">
              {summary.running} {t("agentPool.running")}
            </span>
          )}
          {summary.pending > 0 && (
            <span className="pool-summary__count pool-summary__count--pending">
              {summary.pending} {t("agentPool.pending")}
            </span>
          )}
          {summary.failed > 0 && (
            <span className="pool-summary__count pool-summary__count--failed">
              {summary.failed} {t("agentPool.failed")}
            </span>
          )}
        </div>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// 主面板
// ---------------------------------------------------------------------------

interface AgentPoolPanelProps {
  conversationId: string;
  /** 是否在 agent 模式下显示 */
  visible?: boolean;
}

export function AgentPoolPanel({
  conversationId,
  visible = true,
}: AgentPoolPanelProps) {
  const pool = useExecutionStore((s) => s.agentPool[conversationId] || _EMPTY);

  const summary = useMemo(() => {
    const total = pool.length;
    const running = pool.filter((a) => a.status === "running").length;
    const completed = pool.filter((a) => a.status === "completed").length;
    const pending = pool.filter((a) => a.status === "pending").length;
    const failed = pool.filter((a) => a.status === "failed").length;
    const pctComplete = total > 0 ? Math.round((completed / total) * 100) : 0;
    return { total, running, completed, pending, failed, pctComplete };
  }, [pool]);

  // 按依赖关系排序：无依赖的在前，有依赖的在后
  const sorted = useMemo(() => {
    const items = [...pool];
    return items.sort((a, b) => {
      const aHasDeps = (a.dependsOn?.length || 0) > 0;
      const bHasDeps = (b.dependsOn?.length || 0) > 0;
      if (aHasDeps && !bHasDeps) {
        return 1;
      }
      if (!aHasDeps && bHasDeps) {
        return -1;
      }
      return (a.startedAt || 0) - (b.startedAt || 0);
    });
  }, [pool]);

  if (!visible || pool.length === 0) {
    return null;
  }

  return (
    <div className="agent-pool-panel" data-component="agent-pool-panel">
      <PoolSummaryBar summary={summary} />
      <div className="pool-items">
        {sorted.map((item) =>
          item.type === "sub_agent"
            ? (
              <SubAgentCard
                key={item.id}
                card={{
                  id: item.id,
                  conversationId: item.conversationId,
                  agentType: item.agentType || "general",
                  agentName: item.name,
                  description: item.summary || item.taskDescription || "",
                  status: item.status as "running" | "completed" | "failed",
                  childConversationId: item.childConversationId,
                  childSessionId: item.childSessionId,
                  isFork: item.isFork,
                }}
              />
            )
            : <PoolItemCard key={item.id} item={item} />
        )}
      </div>
    </div>
  );
}

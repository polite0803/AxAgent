import { SyncOutlined } from "@ant-design/icons";
import { Button, Dropdown, Input, Popover, Spin, Tooltip, Typography } from "antd";
import type { MenuProps } from "antd";
import type { InputRef } from "antd";
import {
  ArrowDownRight,
  ArrowUpRight,
  Bot,
  Brain,
  ChartNoAxesColumn,
  Clock,
  Coins,
  ListTodo,
  MessageSquare,
  Pencil,
  Share2,
  Sparkles,
  Timer,
  User,
  Zap,
} from "lucide-react";
import React from "react";
import { useTranslation } from "react-i18next";

import { useAgentProfileStore, useConversationStore } from "@/stores";
import type { ConversationStats } from "@/types";

import { formatDuration, formatSpeed, formatTokenCount } from "../gateway/tokenFormat";
import { ExpertBadge } from "./ExpertBadge";
import { GatewaySessionBadge } from "./GatewaySessionBadge";
import { AgentRoleSelect } from "./InputArea";
import { ModelSelector } from "./ModelSelector";
import { WorkflowBadge } from "./WorkflowBadge";

function StatsPopoverContent({ stats, t, token }: {
  stats: ConversationStats | null;
  t: (key: string) => string;
  token: Record<string, any>;
}) {
  if (!stats) {
    return (
      <div style={{ display: "flex", justifyContent: "center", padding: "24px 40px" }}>
        <Spin size="small" />
      </div>
    );
  }

  const items: Array<{
    icon: React.ReactNode;
    label: string;
    value: string;
    sub?: Array<{ icon: React.ReactNode; label: string; value: string }>;
  }> = [
    {
      icon: <MessageSquare size={14} />,
      label: t("chat.stats.totalMessages"),
      value: stats.total_messages.toLocaleString(),
      sub: [
        {
          icon: <User size={12} />,
          label: t("chat.stats.userMessages"),
          value: stats.total_user_messages.toLocaleString(),
        },
        {
          icon: <Bot size={12} />,
          label: t("chat.stats.assistantMessages"),
          value: stats.total_assistant_messages.toLocaleString(),
        },
      ],
    },
    {
      icon: <Coins size={14} />,
      label: t("chat.stats.totalTokens"),
      value: formatTokenCount(stats.total_tokens),
      sub: [
        {
          icon: <ArrowUpRight size={12} />,
          label: t("chat.stats.inputTokens"),
          value: formatTokenCount(stats.total_prompt_tokens),
        },
        {
          icon: <ArrowDownRight size={12} />,
          label: t("chat.stats.outputTokens"),
          value: formatTokenCount(stats.total_completion_tokens),
        },
      ],
    },
    ...(stats.avg_first_token_latency_ms != null
      ? [{
        icon: <Zap size={14} />,
        label: t("chat.stats.avgFirstToken"),
        value: formatDuration(stats.avg_first_token_latency_ms),
      }]
      : []),
    ...(stats.avg_response_time_ms != null
      ? [{
        icon: <Clock size={14} />,
        label: t("chat.stats.avgResponseTime"),
        value: formatDuration(stats.avg_response_time_ms),
      }]
      : []),
    ...(stats.avg_tokens_per_second != null
      ? [{
        icon: <Timer size={14} />,
        label: t("chat.stats.avgSpeed"),
        value: formatSpeed(stats.avg_tokens_per_second),
      }]
      : []),
  ];

  return (
    <div style={{ minWidth: 220, maxWidth: 280 }}>
      <div style={{ fontSize: 13, fontWeight: 600, marginBottom: 12, display: "flex", alignItems: "center", gap: 6 }}>
        <ChartNoAxesColumn size={14} />
        {t("chat.stats.title")}
      </div>
      <div style={{ display: "flex", flexDirection: "column", gap: 10 }}>
        {items.map((item, i) => (
          <div key={item.label}>
            <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", gap: 12 }}>
              <span
                style={{
                  display: "inline-flex",
                  alignItems: "center",
                  gap: 6,
                  fontSize: 13,
                  color: token.colorTextSecondary,
                }}
              >
                {item.icon}
                {item.label}
              </span>
              <span style={{ fontSize: 14, fontWeight: 600, fontVariantNumeric: "tabular-nums" }}>
                {item.value}
              </span>
            </div>
            {item.sub && (
              <div style={{ marginLeft: 20, marginTop: 4, display: "flex", flexDirection: "column", gap: 3 }}>
                {item.sub.map((s) => (
                  <div
                    key={s.label}
                    style={{ display: "flex", alignItems: "center", justifyContent: "space-between", gap: 12 }}
                  >
                    <span
                      style={{
                        display: "inline-flex",
                        alignItems: "center",
                        gap: 4,
                        fontSize: 12,
                        color: token.colorTextDescription,
                      }}
                    >
                      {s.icon}
                      {s.label}
                    </span>
                    <span style={{ fontSize: 12, color: token.colorTextSecondary, fontVariantNumeric: "tabular-nums" }}>
                      {s.value}
                    </span>
                  </div>
                ))}
              </div>
            )}
            {i < items.length - 1 && (
              <div style={{ borderBottom: `1px solid ${token.colorBorderSecondary}`, marginTop: 10 }} />
            )}
          </div>
        ))}
      </div>
    </div>
  );
}

export interface ChatViewToolbarProps {
  activeConversation: import("@/types").Conversation | undefined;
  activeConversationId: string | null;
  editingTitle: boolean;
  titleDraft: string;
  setTitleDraft: (v: string | ((prev: string) => string)) => void;
  titleInputRef: React.RefObject<InputRef | null>;
  handleTitleClick: () => void;
  handleTitleSave: () => void;
  handleRegenerateTitle: () => void;
  isTitleGenerating: boolean;
  renderConvIconForChat: (size: number, model_id?: string | null) => React.ReactNode;
  topicGroupEnabled: boolean;
  handleTopicGroupToggle: () => void;
  statsOpen: boolean;
  stats: ConversationStats | null;
  handleStatsOpenChange: (open: boolean) => void;
  exportMenuItems: MenuProps["items"];
  setExtractMemoriesOpen: (v: boolean) => void;
  setExpertOpen: (v: boolean) => void;
  streamingMessageId: string | null;
  token: Record<string, any>;
}

/**
 * Wrapper component for renderConvIconForChat render prop.
 * Fixes react-doctor/no-render-in-render by extracting the render prop call into a component.
 */
function ConvIconForChat({
  render,
  size,
}: {
  render: (size: number, model_id?: string | null) => React.ReactNode;
  size: number;
}) {
  return <>{render(size)}</>;
}

export function ChatViewToolbar({
  activeConversation,
  activeConversationId,
  editingTitle,
  titleDraft,
  setTitleDraft,
  titleInputRef,
  handleTitleClick,
  handleTitleSave,
  handleRegenerateTitle,
  isTitleGenerating,
  renderConvIconForChat,
  topicGroupEnabled,
  handleTopicGroupToggle,
  statsOpen,
  stats,
  handleStatsOpenChange,
  exportMenuItems,
  setExtractMemoriesOpen,
  setExpertOpen,
  streamingMessageId,
  token,
}: ChatViewToolbarProps) {
  const { t } = useTranslation();
  const updateConversation = useConversationStore((s) => s.updateConversation);
  const fetchConversation = useConversationStore((s) => s.fetchConversations);

  return (
    <div className="flex items-center gap-2 p-3">
      {activeConversation
        ? (
          <>
            <ConvIconForChat render={renderConvIconForChat} size={24} />
            {editingTitle
              ? (
                <div className="flex items-center gap-1">
                  <Input
                    id="chat-view-input-7"
                    ref={titleInputRef as React.Ref<InputRef>}
                    value={titleDraft}
                    onChange={(e) => setTitleDraft(e.target.value)}
                    onBlur={handleTitleSave}
                    onPressEnter={handleTitleSave}
                    size="small"
                    style={{ maxWidth: 240 }}
                  />
                  <Tooltip title={t("chat.aiGenerateTitle")}>
                    <Button
                      type="text"
                      size="small"
                      icon={isTitleGenerating ? <SyncOutlined spin /> : <Sparkles size={14} />}
                      disabled={isTitleGenerating}
                      onMouseDown={(e) => e.preventDefault()}
                      onClick={(e) => {
                        e.stopPropagation();
                        handleRegenerateTitle();
                      }}
                    />
                  </Tooltip>
                </div>
              )
              : (
                <Typography.Text
                  className="cursor-pointer select-none"
                  onClick={handleTitleClick}
                >
                  {activeConversation.title}
                  {isTitleGenerating
                    ? <SyncOutlined spin className="ml-1 text-xs opacity-50" />
                    : <Pencil size={12} className="ml-1 text-xs opacity-50" />}
                </Typography.Text>
              )}

            {activeConversation?.mode === "agent" && (
              <WorkflowBadge
                sessionType={activeConversation?.session_type ?? "conversation"}
                workflowTemplateId={activeConversation?.workflow_template_id}
                workflowStatus={activeConversation?.workflow_status}
                onSelectWorkflow={(templateId, workflowId) => {
                  if (activeConversation.id) {
                    if (workflowId) {
                      try {
                        localStorage.setItem(`axagent:workflow-id:${activeConversation.id}`, workflowId);
                        window.dispatchEvent(
                          new CustomEvent("axagent:workflow-changed", {
                            detail: { conversationId: activeConversation.id, workflowId },
                          }),
                        );
                      } catch {
                        // Ignore storage errors
                      }
                    } else {
                      try {
                        localStorage.removeItem(`axagent:workflow-id:${activeConversation.id}`);
                        window.dispatchEvent(
                          new CustomEvent("axagent:workflow-changed", {
                            detail: { conversationId: activeConversation.id, workflowId: null },
                          }),
                        );
                      } catch {
                        // Ignore storage errors
                      }
                    }
                  }
                  if (templateId === "") {
                    void updateConversation(activeConversation.id, {
                      session_type: "conversation",
                      workflow_template_id: null,
                    });
                  } else {
                    void updateConversation(activeConversation.id, {
                      session_type: "workflow",
                      workflow_template_id: workflowId || templateId,
                      expert_role_id: null,
                      agent_profile_id: null,
                    });
                  }
                  fetchConversation();
                }}
                onRemoveWorkflow={() => {
                  if (activeConversation.id) {
                    try {
                      localStorage.removeItem(`axagent:workflow-id:${activeConversation.id}`);
                      window.dispatchEvent(
                        new CustomEvent("axagent:workflow-changed", {
                          detail: { conversationId: activeConversation.id, workflowId: null },
                        }),
                      );
                    } catch {
                      // Ignore storage errors
                    }
                  }
                  void updateConversation(activeConversation.id, {
                    session_type: "conversation",
                    workflow_template_id: null,
                  });
                  fetchConversation();
                }}
                disabled={!!streamingMessageId}
              />
            )}
            {activeConversation?.mode === "agent"
              && activeConversation?.session_type !== "workflow" && (
              <>
                <ExpertBadge
                  expertRoleId={activeConversation.expert_role_id ?? null}
                  onClick={() => setExpertOpen(true)}
                />
                <AgentRoleSelect
                  value={activeConversation.agent_profile_id ?? ""}
                  onChange={(profileId) => {
                    const profile = useAgentProfileStore.getState().getProfileById(profileId);
                    updateConversation(activeConversation.id, {
                      agent_profile_id: profileId || null,
                      system_prompt: profile?.systemPrompt || undefined,
                      session_type: "conversation",
                      workflow_template_id: null,
                    });
                  }}
                />
              </>
            )}
            {activeConversation?.mode === "gateway" && (
              <GatewaySessionBadge
                platform={(() => {
                  const m = activeConversation.title.match(/^\[(\w+)\]/);
                  return m ? m[1] : "";
                })()}
              />
            )}

            <div className="flex-1" />

            <Tooltip
              title={topicGroupEnabled
                ? t("topicGroup.disableAutoGroup")
                : t("topicGroup.autoGroup")}
            >
              <Button
                type="text"
                size="small"
                icon={<ListTodo size={14} style={{ color: topicGroupEnabled ? token.colorPrimary : undefined }} />}
                onClick={handleTopicGroupToggle}
              />
            </Tooltip>
            <ModelSelector />
            <Popover
              content={<StatsPopoverContent stats={stats} t={t} token={token} />}
              trigger="click"
              open={statsOpen}
              onOpenChange={handleStatsOpenChange}
              placement="bottomRight"
            >
              <Tooltip title={t("chat.stats.title")}>
                <Button type="text" icon={<ChartNoAxesColumn size={14} />} size="small" />
              </Tooltip>
            </Popover>
            <Dropdown menu={{ items: exportMenuItems }} trigger={["click"]}>
              <Button type="text" icon={<Share2 size={14} />} size="small" />
            </Dropdown>
            <Tooltip title={t("chat.extractMemories")}>
              <Button
                type="text"
                icon={<Brain size={14} />}
                size="small"
                onClick={() => setExtractMemoriesOpen(true)}
                disabled={!activeConversationId}
              />
            </Tooltip>
          </>
        )
        : (
          <>
            <Typography.Text type="secondary">{t("chat.welcome")}</Typography.Text>
            <div className="flex-1" />
            <ModelSelector />
          </>
        )}
    </div>
  );
}

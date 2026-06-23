// SPDX-License-Identifier: AGPL-3.0-only

import { SyncOutlined } from "@ant-design/icons";
import {
  ArrowDownRight,
  ArrowUpRight,
  Brain,
  ChartBar,
  ChatText,
  Clock,
  Coin,
  Lightning,
  ListChecks,
  NotePencil,
  Robot,
  ShareNetwork,
  Sparkle,
  Timer,
  User,
} from "@phosphor-icons/react";
import { Button, Input, Popover, Spin, Typography } from "antd";
import type { GlobalToken, InputRef } from "antd";
import React from "react";
import { useTranslation } from "react-i18next";

import { useConversationStore } from "@/stores";
import type { ConversationStats } from "@/types";

import { type DropdownItem, DropdownMenu } from "@/components/layout/DropdownMenu";
import { Tooltip } from "@/components/layout/Tooltip";
import { formatDuration, formatSpeed, formatTokenCount } from "../gateway/tokenFormat";
import { ExpertBadge } from "./ExpertBadge";
import { AgentProfileSelect } from "./InputArea";
import { ModelSelector } from "./ModelSelector";
import { WorkflowBadge } from "./WorkflowBadge";

function StatsPopoverContent({
  stats,
  t,
  token,
}: {
  stats: ConversationStats | null;
  t: (key: string) => string;
  token: GlobalToken;
}) {
  if (!stats) {
    return (
      <div
        style={{
          display: "flex",
          justifyContent: "center",
          padding: "24px 40px",
        }}
      >
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
      icon: <ChatText size={14} />,
      label: t("chat.stats.totalMessages"),
      value: stats.total_messages.toLocaleString(),
      sub: [
        {
          icon: <User size={12} />,
          label: t("chat.stats.userMessages"),
          value: stats.total_user_messages.toLocaleString(),
        },
        {
          icon: <Robot size={12} />,
          label: t("chat.stats.assistantMessages"),
          value: stats.total_assistant_messages.toLocaleString(),
        },
      ],
    },
    {
      icon: <Coin size={14} />,
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
      ? [
        {
          icon: <Lightning size={14} />,
          label: t("chat.stats.avgFirstToken"),
          value: formatDuration(stats.avg_first_token_latency_ms),
        },
      ]
      : []),
    ...(stats.avg_response_time_ms != null
      ? [
        {
          icon: <Clock size={14} />,
          label: t("chat.stats.avgResponseTime"),
          value: formatDuration(stats.avg_response_time_ms),
        },
      ]
      : []),
    ...(stats.avg_tokens_per_second != null
      ? [
        {
          icon: <Timer size={14} />,
          label: t("chat.stats.avgSpeed"),
          value: formatSpeed(stats.avg_tokens_per_second),
        },
      ]
      : []),
  ];

  return (
    <div style={{ minWidth: 220, maxWidth: 280 }}>
      <div
        style={{
          fontSize: 13,
          fontWeight: 600,
          marginBottom: 12,
          display: "flex",
          alignItems: "center",
          gap: 6,
        }}
      >
        <ChartBar size={14} />
        {t("chat.stats.title")}
      </div>
      <div style={{ display: "flex", flexDirection: "column", gap: 10 }}>
        {items.map((item, i) => (
          <div key={item.label}>
            <div
              style={{
                display: "flex",
                alignItems: "center",
                justifyContent: "space-between",
                gap: 12,
              }}
            >
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
              <span
                style={{
                  fontSize: 14,
                  fontWeight: 600,
                  fontVariantNumeric: "tabular-nums",
                }}
              >
                {item.value}
              </span>
            </div>
            {item.sub && (
              <div
                style={{
                  marginLeft: 20,
                  marginTop: 4,
                  display: "flex",
                  flexDirection: "column",
                  gap: 3,
                }}
              >
                {item.sub.map((s) => (
                  <div
                    key={s.label}
                    style={{
                      display: "flex",
                      alignItems: "center",
                      justifyContent: "space-between",
                      gap: 12,
                    }}
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
                    <span
                      style={{
                        fontSize: 12,
                        color: token.colorTextSecondary,
                        fontVariantNumeric: "tabular-nums",
                      }}
                    >
                      {s.value}
                    </span>
                  </div>
                ))}
              </div>
            )}
            {i < items.length - 1 && (
              <div
                style={{
                  borderBottom: `1px solid ${token.colorBorderSecondary}`,
                  marginTop: 10,
                }}
              />
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
  renderConvIconForChat: (
    size: number,
    model_id?: string | null,
  ) => React.ReactNode;
  topicGroupEnabled: boolean;
  handleTopicGroupToggle: () => void;
  statsOpen: boolean;
  stats: ConversationStats | null;
  handleStatsOpenChange: (open: boolean) => void;
  exportMenuItems: Record<string, unknown>["items"];
  setExtractMemoriesOpen: (v: boolean) => void;
  setExpertOpen: (v: boolean) => void;
  streamingMessageId: string | null;
  token: GlobalToken;
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
    <div className="flex items-center gap-2 p-3 flex-wrap">
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
                      icon={isTitleGenerating ? <SyncOutlined spin /> : <Sparkle size={14} />}
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
                <div className="ax-truncate" style={{ flex: "1 1 auto", minWidth: 0 }}>
                  <Typography.Text
                    className="cursor-pointer select-none"
                    onClick={handleTitleClick}
                  >
                    {activeConversation.title}
                    {isTitleGenerating
                      ? <SyncOutlined spin className="ml-1 text-xs opacity-50" />
                      : <NotePencil size={12} className="ml-1 text-xs opacity-50" />}
                  </Typography.Text>
                </div>
              )}

            {activeConversation?.mode === "agent" && (
              <WorkflowBadge
                sessionType={activeConversation?.session_type ?? "conversation"}
                workflowTemplateId={activeConversation?.workflow_template_id}
                workflowStatus={activeConversation?.workflow_status}
                onSelectWorkflow={async (templateId, workflowId) => {
                  if (activeConversation.id) {
                    if (workflowId) {
                      try {
                        localStorage.setItem(
                          `axagent:workflow-id:${activeConversation.id}`,
                          workflowId,
                        );
                        window.dispatchEvent(
                          new CustomEvent("axagent:workflow-changed", {
                            detail: {
                              conversationId: activeConversation.id,
                              workflowId,
                            },
                          }),
                        );
                      } catch {
                        // Ignore storage errors
                      }
                    } else {
                      try {
                        localStorage.removeItem(
                          `axagent:workflow-id:${activeConversation.id}`,
                        );
                        window.dispatchEvent(
                          new CustomEvent("axagent:workflow-changed", {
                            detail: {
                              conversationId: activeConversation.id,
                              workflowId: null,
                            },
                          }),
                        );
                      } catch {
                        // Ignore storage errors
                      }
                    }
                  }
                  if (templateId === "") {
                    await updateConversation(activeConversation.id, {
                      session_type: "conversation",
                      workflow_template_id: null,
                    });
                  } else {
                    await updateConversation(activeConversation.id, {
                      session_type: "workflow",
                      workflow_template_id: workflowId || templateId,
                      agent_profile_id: null,
                    });
                  }
                  fetchConversation();
                }}
                onRemoveWorkflow={async () => {
                  if (activeConversation.id) {
                    try {
                      localStorage.removeItem(
                        `axagent:workflow-id:${activeConversation.id}`,
                      );
                      window.dispatchEvent(
                        new CustomEvent("axagent:workflow-changed", {
                          detail: {
                            conversationId: activeConversation.id,
                            workflowId: null,
                          },
                        }),
                      );
                    } catch {
                      // Ignore storage errors
                    }
                  }
                  await updateConversation(activeConversation.id, {
                    session_type: "conversation",
                    workflow_template_id: null,
                  });
                  fetchConversation();
                }}
                disabled={!!streamingMessageId}
              />
            )}
            {activeConversation?.mode === "agent"
              && activeConversation?.session_type !== "workflow"
              && (activeConversation?.work_strategy !== "plan"
                || !activeConversation?.workflow_template_id)
              && (
                <>
                  <ExpertBadge
                    agentProfileId={activeConversation.agent_profile_id ?? null}
                    onClick={() => setExpertOpen(true)}
                  />
                  <AgentProfileSelect
                    value={activeConversation.agent_profile_id ?? ""}
                    onChange={async (profileId) => {
                      await updateConversation(activeConversation.id, {
                        agent_profile_id: profileId || null,
                        session_type: "conversation",
                        workflow_template_id: null,
                      });
                      fetchConversation();
                    }}
                  />
                </>
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
                icon={
                  <ListChecks
                    size={14}
                    style={{
                      color: topicGroupEnabled ? token.colorPrimary : undefined,
                    }}
                  />
                }
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
                <Button
                  type="text"
                  icon={<ChartBar size={14} />}
                  size="small"
                />
              </Tooltip>
            </Popover>
            <DropdownMenu items={(exportMenuItems ?? []) as DropdownItem[]} trigger={["click"]}>
              <Button type="text" icon={<ShareNetwork size={14} />} size="small" />
            </DropdownMenu>
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
            <Typography.Text type="secondary">
              {t("chat.welcome")}
            </Typography.Text>
            <div className="flex-1" />
            <ModelSelector />
          </>
        )}
    </div>
  );
}

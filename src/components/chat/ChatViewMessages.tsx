// SPDX-License-Identifier: AGPL-3.0-only

// 本文件使用状态变量做渲染时缓存

// Local message types (replacing @ant-design/x Bubble)
import { type CSSProperties, type ReactNode } from "react";

interface BubbleItemType {
  key: string;
  role?: string;
  content?: ReactNode | import("@/stores/feature/topicGroupStore").TopicGroup;
  variant?: "filled" | "outlined" | "shadow" | "borderless";
}

type RoleType = Record<
  string,
  (item: BubbleItemType) => {
    placement?: "start" | "end";
    className?: string;
    variant?: string;
    style?: CSSProperties;
    avatar?: ReactNode;
    loading?: boolean;
    contentRender?: (content: ReactNode, item: BubbleItemType) => ReactNode;
    header?: ReactNode;
    footer?: ReactNode;
  }
>;

interface BubbleListRef {
  scrollBoxNativeElement?: HTMLElement | null;
}

/**
 * 从气泡 key 解析底层消息 id：
 * - user 消息的 key 即 msg.id
 * - assistant 消息的 key 为 `ai:<parentMessageId>:<msgId>`，最后一段为 msgId
 * （多模型版本共存时同 parentId 会有多个气泡，key 必须携带 msgId 区分）
 */
function parseBubbleMsgId(key: string): string {
  if (key.startsWith("ai:")) {
    const idx = key.lastIndexOf(":");
    return idx >= 0 ? key.slice(idx + 1) : key.slice(3);
  }
  return key;
}

// Local replacement for @ant-design/x Actions
interface ActionItem {
  key: string;
  icon?: ReactNode;
  label?: string;
  onItemClick?: () => void;
  actionRender?: () => ReactNode;
}
function Actions({ items, onActionClick }: { items: ActionItem[]; onActionClick?: (item: ActionItem) => void }) {
  return (
    <div className="msg-actions">
      {items.map((action) => {
        if (action.actionRender) {
          return <div key={action.key} className="msg-action-custom">{action.actionRender()}</div>;
        }
        return (
          <button
            key={action.key}
            className="msg-action-btn"
            title={action.label}
            onClick={() => {
              action.onItemClick?.();
              onActionClick?.(action);
            }}
          >
            {action.icon}
            {action.label && <span className="msg-action-label">{action.label}</span>}
          </button>
        );
      })}
    </div>
  );
}

// Popover 内格式按钮的统一样式
const styleBtn: CSSProperties = {
  padding: "4px 12px",
  border: "none",
  background: "transparent",
  cursor: "pointer",
  textAlign: "left",
  fontSize: 13,
  borderRadius: 4,
  color: "inherit",
};

import { ModelIcon } from "@lobehub/icons";
import { Alert, App, Avatar, Input, Modal, Popconfirm, Popover, Spin, Tag, theme, Typography } from "antd";
import {
  ArrowDown,
  ArrowLeftRight,
  Bot,
  Check,
  Coins,
  Copy,
  GitBranch,
  MessageSquare,
  Pencil,
  RotateCcw,
  Save,
  Scissors,
  TextCursorInput,
  Trash2,
  User,
  X,
  Zap,
} from "lucide-react";
import React, { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";

import { useCopyToClipboard } from "@/hooks/useCopyToClipboard";
import { useResolvedAvatarSrc } from "@/hooks/useResolvedAvatarSrc";
import { useResolvedDarkMode } from "@/hooks/useResolvedDarkMode";
import { type ChatMarkdownNode, parseChatMarkdown, stripAxAgentTags } from "@/lib/chatMarkdown";
import { hasMultipleModelVersions } from "@/lib/chatMultiModel";
import { getConvIcon } from "@/lib/convIcon";
import { invoke, isTauri } from "@/lib/invoke";
import { parseSearchContent } from "@/lib/searchUtils";
import {
  useAgentStore,
  useCompressStore,
  useConversationStore,
  useExecutionStore,
  useExpertStore,
  useProviderStore,
  useSettingsStore,
  useStreamStore,
  useTopicGroupStore,
  useUIStore,
  useUserProfileStore,
} from "@/stores";
import type { Message } from "@/types";

import { Tooltip } from "@/components/layout/Tooltip";
import { formatDuration, formatSpeed, formatTokenCount } from "../gateway/tokenFormat";
import { AskUserCard } from "./AskUserCard";
import { AttachmentPreview } from "./AttachmentPreview";
import { AssistantMarkdown, getChatCodeThemes, THINKING_LOADING_MARKER } from "./ChatMarkdownNodes";
import { getStreamingLoadingState, shouldRenderAssistantMarkdownFromContent } from "./chatStreaming";
import { CognitiveDecisionCard } from "./CognitiveDecisionCard";
import { DeleteLastVersionPopover } from "./DeleteLastVersionPopover";
import { ModelSelector } from "./ModelSelector";
import { ModelTags } from "./ModelTags";
import { LayoutSwitcher, MultiModelDisplay, type MultiModelDisplayMode } from "./MultiModelDisplay";
import { PermissionCard } from "./PermissionCard";
import { QuoteBlock } from "./QuoteBlock";
import { ToolCallBlockView } from "./ToolCallBlockView";
import { ToolCallCard } from "./ToolCallCard";
import { buildAssistantDisplayContent, shouldHideAssistantBubble } from "./toolCallDisplay";
import { TopicGroupDivider } from "./TopicGroupDivider";
import { VersionPagination } from "./VersionPagination";

function AssistantFooter({
  msg,
  conversationId,
  assistantCopyText,
  getModelDisplayInfo,
  isStreaming = false,
  displayMode,
  onDisplayModeChange,
  onMultiModelDetected,
  onQuoteReply,
}: {
  msg: Message;
  conversationId: string;
  assistantCopyText: string;
  getModelDisplayInfo: (
    modelId?: string | null,
    providerId?: string | null,
  ) => { modelName: string; providerName: string };
  isStreaming?: boolean;
  displayMode?: MultiModelDisplayMode;
  onDisplayModeChange?: (
    parentMsgId: string,
    mode: MultiModelDisplayMode,
  ) => void;
  onMultiModelDetected?: (parentMsgId: string, versions: Message[]) => void;
  /** 引用回复：点击引用按钮时回调 */
  onQuoteReply?: (messageId: string) => void;
  isDarkMode: boolean;
  codeBlockDarkTheme: string;
  codeBlockLightTheme: string;
  codeBlockThemes: readonly [string, string];
  codeFontFamily?: string;
}) {
  const { token } = theme.useToken();
  const { t } = useTranslation();
  const { message: messageApi } = App.useApp();
  const [allVersions, setAllVersions] = useState<Message[]>([]);
  const listMessageVersions = useConversationStore(
    (s) => s.listMessageVersions,
  );
  const regenerateMessage = useConversationStore((s) => s.regenerateMessage);
  const regenerateWithModel = useConversationStore(
    (s) => s.regenerateWithModel,
  );
  const deleteMessageGroup = useConversationStore((s) => s.deleteMessageGroup);
  const deleteMessage = useConversationStore((s) => s.deleteMessage);
  const branchConversation = useConversationStore((s) => s.branchConversation);
  const { copy: copyAssistant, isCopied: assistantCopied } = useCopyToClipboard();
  const [branchModalOpen, setBranchModalOpen] = useState(false);
  const [branchAsChild] = useState(false);
  const [branchTitle, setBranchTitle] = useState("");
  const currentConvTitle = useConversationStore(
    (s) => s.conversations.find((c) => c.id === conversationId)?.title ?? "",
  );
  const storeMessages = useConversationStore((s) => s.messages);

  useEffect(() => {
    if (msg.parentMessageId && conversationId) {
      let cancelled = false;
      listMessageVersions(conversationId, msg.parentMessageId).then((v) => {
        if (!cancelled && v) {
          setAllVersions(v);
        }
      });
      return () => {
        cancelled = true;
      };
    }
  }, [
    msg.parentMessageId,
    conversationId,
    listMessageVersions,
  ]);

  const mergedVersions = useMemo(() => {
    if (!msg.parentMessageId) {
      return allVersions;
    }
    const dbIds = new Set(allVersions.map((v) => v.id));
    const extra = storeMessages.filter(
      (m) =>
        m.parentMessageId === msg.parentMessageId
        && m.role === "assistant"
        && !dbIds.has(m.id)
        && m.modelId,
    );
    return extra.length > 0 ? [...allVersions, ...extra] : allVersions;
  }, [allVersions, storeMessages, msg.parentMessageId]);

  const hasMultiModels = useMemo(
    () => hasMultipleModelVersions(mergedVersions),
    [mergedVersions],
  );

  useEffect(() => {
    if (msg.parentMessageId && onMultiModelDetected) {
      onMultiModelDetected(msg.parentMessageId, mergedVersions);
    }
  }, [msg.parentMessageId, mergedVersions, onMultiModelDetected]);

  const currentModelOverride = useMemo(() => {
    if (msg.providerId && msg.modelId) {
      return { providerId: msg.providerId, modelId: msg.modelId };
    }
    return null;
  }, [msg.providerId, msg.modelId]);

  const handleModelSelect = useCallback(
    async (providerId: string, modelId: string) => {
      try {
        if (providerId === msg.providerId && modelId === msg.modelId) {
          await regenerateMessage(msg.id);
        } else {
          await regenerateWithModel(msg.id, providerId, modelId);
        }
      } catch (e) {
        messageApi.error(String(e));
      }
    },
    [
      msg.id,
      msg.providerId,
      msg.modelId,
      regenerateMessage,
      regenerateWithModel,
      messageApi,
    ],
  );

  const totalTokens = (msg.promptTokens ?? 0) + (msg.completionTokens ?? 0);

  return (
    <div style={{ display: "flex", flexDirection: "column" }}>
      {!isStreaming
        && (msg.promptTokens != null
          || msg.completionTokens != null
          || msg.tokensPerSecond != null
          || msg.firstTokenLatencyMs != null)
        && (
          <div
            style={{
              display: "flex",
              alignItems: "center",
              gap: 6,
              fontSize: 12,
              color: token.colorTextDescription,
              lineHeight: "16px",
              marginTop: -6,
              marginBottom: 4,
              flexWrap: "wrap",
            }}
          >
            {msg.promptTokens != null && (
              <span
                style={{ display: "inline-flex", alignItems: "center", gap: 2 }}
              >
                <ArrowDown size={10} />
                <span className="ax-glow-text">
                  {formatTokenCount(msg.promptTokens)}
                </span>{" "}
                {t("chat.tokens")}
              </span>
            )}
            {msg.completionTokens != null && (
              <span
                style={{ display: "inline-flex", alignItems: "center", gap: 2 }}
              >
                <ArrowDown size={10} />
                <span className="ax-glow-text">
                  {formatTokenCount(msg.completionTokens)}
                </span>{" "}
                {t("chat.tokens")}
              </span>
            )}
            {totalTokens > 0 && (
              <span
                style={{ display: "inline-flex", alignItems: "center", gap: 2 }}
              >
                <Coins size={10} />
                {t("chat.totalTokens")}:{" "}
                <span className="ax-glow-text">
                  {formatTokenCount(totalTokens)}
                </span>
              </span>
            )}
            {msg.tokensPerSecond != null && (
              <span
                style={{ display: "inline-flex", alignItems: "center", gap: 2 }}
              >
                <Zap size={10} />
                {formatSpeed(msg.tokensPerSecond)}
              </span>
            )}
            {msg.firstTokenLatencyMs != null && (
              <span
                style={{ display: "inline-flex", alignItems: "center", gap: 2 }}
              >
                <TextCursorInput size={10} />
                {formatDuration(msg.firstTokenLatencyMs)}
              </span>
            )}
          </div>
        )}
      {!isStreaming && (
        <div style={{ display: "flex", alignItems: "center" }}>
          <VersionPagination
            msg={msg}
            conversationId={conversationId}
            allVersions={mergedVersions}
          />
          <Actions
            items={[
              {
                key: "copy",
                icon: assistantCopied ? <Check size={14} style={{ color: token.colorSuccess }} /> : <Copy size={14} />,
                label: t("chat.copy"),
                onItemClick: () => {
                  void copyAssistant(assistantCopyText).then((ok) => {
                    if (ok) {
                      messageApi.success(t("chat.copied"));
                    }
                  });
                },
              },
              {
                key: "save",
                actionRender: () => {
                  const handleSaveAs = async (format: "md" | "docx" | "pdf") => {
                    try {
                      if (format === "md" || !isTauri()) {
                        const blob = new Blob([assistantCopyText], {
                          type: "text/markdown;charset=utf-8",
                        });
                        const url = URL.createObjectURL(blob);
                        const a = document.createElement("a");
                        a.href = url;
                        a.download = `${(currentConvTitle || "message").slice(0, 40)}.md`;
                        document.body.appendChild(a);
                        a.click();
                        document.body.removeChild(a);
                        URL.revokeObjectURL(url);
                        messageApi.success(t("chat.saved"));
                        return;
                      }
                      const { save } = await import("@tauri-apps/plugin-dialog");
                      const ext = format === "docx" ? "docx" : "pdf";
                      const name = `${(currentConvTitle || "message").slice(0, 40)}.${ext}`;
                      const filePath = await save({
                        defaultPath: name,
                        filters: format === "docx"
                          ? [{ name: t("stockAnalysis.docxFilterName"), extensions: ["docx"] }]
                          : [{ name: "PDF", extensions: ["pdf"] }],
                      });
                      if (!filePath) {
                        return;
                      }
                      await invoke<boolean>("export_content", {
                        markdown: assistantCopyText,
                        outputPath: filePath,
                        format,
                        title: currentConvTitle || "Message",
                      });
                      messageApi.success(t("chat.saved"));
                    } catch (e) {
                      messageApi.error(String(e));
                    }
                  };
                  return (
                    <Popover
                      trigger="click"
                      placement="top"
                      content={
                        <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
                          <button style={styleBtn} onClick={() => handleSaveAs("md")}>Markdown (.md)</button>
                          <button style={styleBtn} onClick={() => handleSaveAs("docx")}>Word (.docx)</button>
                          <button style={styleBtn} onClick={() => handleSaveAs("pdf")}>PDF (.pdf)</button>
                        </div>
                      }
                    >
                      <Tooltip title={t("chat.saveAs")}>
                        <span
                          className="axagent-action-item"
                          role="button"
                          tabIndex={0}
                          style={{ color: token.colorTextSecondary }}
                        >
                          <Save size={14} />
                        </span>
                      </Tooltip>
                    </Popover>
                  );
                },
              },
              ...(onQuoteReply
                ? [
                  {
                    key: "quote",
                    icon: <MessageSquare size={14} />,
                    label: t("chat.quote.reply"),
                    onItemClick: () => {
                      onQuoteReply(msg.id);
                    },
                  },
                ]
                : []),
              {
                key: "regenerate",
                icon: <RotateCcw size={14} />,
                label: t("chat.regenerate"),
                onItemClick: async () => {
                  try {
                    await regenerateMessage(msg.id);
                  } catch (e) {
                    messageApi.error(String(e));
                  }
                },
              },
              ...(msg.role === "assistant" && msg.status !== "partial"
                ? [
                  {
                    key: "continue",
                    icon: <MessageSquare size={14} />,
                    label: t("continuation.continueFromHere"),
                    onItemClick: async () => {
                      try {
                        await useStreamStore
                          .getState()
                          .startContinue(conversationId, msg.id, true);
                      } catch (e) {
                        messageApi.error(String(e));
                      }
                    },
                  },
                ]
                : []),
              // 编辑消息仅对用户自己的消息有意义，AI 回复不提供编辑操作
              {
                key: "model",
                actionRender: () => (
                  <ModelSelector
                    onSelect={handleModelSelect}
                    overrideCurrentModel={currentModelOverride}
                  >
                    <Tooltip title={t("chat.switchModel")}>
                      <span
                        className="axagent-action-item"
                        style={{ color: token.colorTextSecondary }}
                      >
                        <ArrowLeftRight size={14} />
                      </span>
                    </Tooltip>
                  </ModelSelector>
                ),
              },
              {
                key: "branch",
                actionRender: () => (
                  <Tooltip title={t("chat.branchConversation")}>
                    <span
                      className="axagent-action-item"
                      role="button"
                      tabIndex={0}
                      onKeyDown={(e) => {
                        if (e.key === "Enter" || e.key === " ") {
                          e.preventDefault();
                          setBranchModalOpen(true);
                        }
                      }}
                      style={{ color: token.colorTextSecondary }}
                      onClick={() => setBranchModalOpen(true)}
                    >
                      <GitBranch size={14} />
                    </span>
                  </Tooltip>
                ),
              },
              {
                key: "delete",
                actionRender: () => {
                  const isLastVersion = mergedVersions.filter((v) => v.id !== msg.id).length === 0;
                  if (isLastVersion) {
                    return (
                      <DeleteLastVersionPopover
                        msg={msg}
                        conversationId={conversationId}
                        deleteMessage={deleteMessage}
                        deleteMessageGroup={deleteMessageGroup}
                      />
                    );
                  }
                  return (
                    <Popconfirm
                      title={t("chat.confirmDeleteVersion")}
                      onConfirm={async () => {
                        try {
                          await deleteMessage(msg.id);
                        } catch (e) {
                          messageApi.error(String(e));
                        }
                      }}
                      okText={t("common.confirm")}
                      cancelText={t("common.cancel")}
                    >
                      <Tooltip title={t("chat.delete")}>
                        <span
                          className="axagent-action-item"
                          style={{ color: token.colorError }}
                        >
                          <Trash2 size={14} />
                        </span>
                      </Tooltip>
                    </Popconfirm>
                  );
                },
              },
            ]}
          />
        </div>
      )}
      <div
        style={{ display: "flex", alignItems: "center", gap: 6, marginTop: 4 }}
      >
        {hasMultiModels
          && displayMode
          && onDisplayModeChange
          && msg.parentMessageId && (
          <LayoutSwitcher
            currentMode={displayMode}
            onModeChange={(mode) => onDisplayModeChange(msg.parentMessageId!, mode)}
          />
        )}
        <ModelTags
          msg={msg}
          conversationId={conversationId}
          allVersions={mergedVersions}
          getModelDisplayInfo={getModelDisplayInfo}
        />
      </div>
      <Modal
        open={branchModalOpen}
        title={t("chat.branchConversation")}
        onCancel={() => setBranchModalOpen(false)}
        onOk={async () => {
          try {
            const title = branchTitle.trim() || currentConvTitle;
            await branchConversation(
              conversationId,
              msg.id,
              branchAsChild,
              title,
            );
            messageApi.success(t("chat.branchCreated"));
            setBranchModalOpen(false);
          } catch (e) {
            messageApi.error(String(e));
          }
        }}
        okText={t("common.confirm")}
        cancelText={t("common.cancel")}
        width={400}
        destroyOnHidden
      >
        <Input
          id="chat-view-input-6"
          value={branchTitle}
          onChange={(e) => setBranchTitle(e.target.value)}
          placeholder={t("chat.branchTitlePlaceholder")}
          onPressEnter={async () => {
            try {
              const title = branchTitle.trim() || currentConvTitle;
              await branchConversation(
                conversationId,
                msg.id,
                branchAsChild,
                title,
              );
              messageApi.success(t("chat.branchCreated"));
              setBranchModalOpen(false);
            } catch (e) {
              messageApi.error(String(e));
            }
          }}
        />
      </Modal>
    </div>
  );
}

export interface ChatViewMessagesProps {
  activeConversationId: string | null;
  activeConversation: import("@/types").Conversation | undefined;
  messages: Message[];
  streaming: boolean;
  compressing: boolean;
  bubbleStyle: string;
  bubbleListRef: React.RefObject<BubbleListRef | null>;
  handleEditMessage: (
    messageId: string,
    content: string,
    role: "user" | "assistant",
  ) => void;
}

export function useChatViewMessages({
  activeConversationId,
  activeConversation,
  messages,
  streaming,
  compressing,
  bubbleStyle,
  bubbleListRef: _bubbleListRef,
  handleEditMessage,
}: ChatViewMessagesProps) {
  const { t, i18n } = useTranslation();
  const { token } = theme.useToken();
  const { message: messageApi } = App.useApp();

  const settings = useSettingsStore((s) => s.settings);
  const profile = useUserProfileStore((s) => s.profile);
  const resolvedAvatarSrc = useResolvedAvatarSrc(
    profile.avatarType,
    profile.avatarValue,
  );
  const isDarkMode = useResolvedDarkMode(settings.themeMode);
  const { copy: copyMessage, isCopiedFor: isUserMsgCopied } = useCopyToClipboard();
  const {
    darkTheme: codeBlockDarkTheme,
    lightTheme: codeBlockLightTheme,
    themes: codeBlockThemes,
  } = useMemo(
    () => getChatCodeThemes(settings.codeTheme, settings.codeThemeLight),
    [settings.codeTheme, settings.codeThemeLight],
  );

  const streamingMessageId = useStreamStore((s) => s.streamingMessageId);
  const multiModelParentId = useConversationStore((s) => s.multiModelParentId);
  const multiModelDoneMessageIds = useConversationStore(
    (s) => s.multiModelDoneMessageIds,
  );
  const thinkingActiveMessageIds = useStreamStore(
    (s) => s.thinkingActiveMessageIds,
  );
  const deleteMessage = useConversationStore((s) => s.deleteMessage);
  const deleteMessageGroup = useConversationStore((s) => s.deleteMessageGroup);
  const switchMessageVersion = useConversationStore(
    (s) => s.switchMessageVersion,
  );
  const regenerateMessage = useConversationStore((s) => s.regenerateMessage);
  const removeContextClear = useConversationStore((s) => s.removeContextClear);
  const setQuotedMessageId = useUIStore((s) => s.setQuotedMessageId);
  const getCompressionSummary = useCompressStore(
    (s) => s.getCompressionSummary,
  );

  // 引用回复：滚动到被引用消息
  const handleJumpToMessage = useCallback((messageId: string) => {
    const el = document.querySelector(`[data-axagent-msg="${messageId}"]`);
    if (el) {
      el.scrollIntoView({ behavior: "smooth", block: "center" });
    }
  }, []);

  // 引用回复：根据 quoted_message_id 查找被引用消息
  const messageById = useMemo(
    () => new Map(messages.map((msg) => [msg.id, msg])),
    [messages],
  );
  const getQuotedMessage = useCallback(
    (quotedId: string | null | undefined): Message | null => {
      if (!quotedId) { return null; }
      return messageById.get(quotedId) ?? null;
    },
    [messageById],
  );
  const deleteCompression = useCompressStore((s) => s.deleteCompression);
  const providers = useProviderStore((s) => s.providers);
  const agentToolCalls = useExecutionStore((s) => s.toolCalls);
  const agentPendingPermissions = useAgentStore((s) => s.pendingPermissions);
  const agentPendingAskUser = useAgentStore((s) => s.pendingAskUser);
  const consumeSwitch = useExpertStore((s) => s.consumeSwitch);
  const getRoleById = useExpertStore((s) => s.getRoleById);

  const [summaryModalOpen, setSummaryModalOpen] = useState(false);
  const [summaryModalText, setSummaryModalText] = useState("");

  // ── AI 消息折叠状态 ──
  const [collapsedAiIds, setCollapsedAiIds] = useState<Record<string, boolean>>({});
  const prevStreamingRef = useRef<string | null>(null);

  useEffect(() => {
    const prev = prevStreamingRef.current;
    const curr = streamingMessageId;
    if (prev === curr) { return; }
    prevStreamingRef.current = curr;

    setCollapsedAiIds((prevMap) => {
      const next = { ...prevMap };
      // 仅在有新流式开始时折叠旧消息；流式结束（curr→null）不折叠，
      // 避免刚完成的回复被立刻折叠导致用户还没看就只剩 3 行
      if (curr && prev && prev !== curr) { next[prev] = true; }
      if (curr) { delete next[curr]; }
      return next;
    });
  }, [streamingMessageId]);
  // FE-I10 修复：命令式标记集合改用 useRef（引用稳定，内容变化不触发渲染）。
  const contentRendererMessageIds = useRef<Set<string>>(new Set());

  useEffect(() => {
    if (!streaming || !streamingMessageId) {
      return;
    }
    contentRendererMessageIds.current.add(streamingMessageId);
  }, [streaming, streamingMessageId]);

  const activeMessages = useMemo(
    () => messages.filter((msg) => msg.isActive !== false),
    [messages],
  );

  const multiModelResponseParents = useMemo(() => {
    const modelsByParent = new Map<string, Set<string>>();
    for (const msg of messages) {
      if (msg.role === "assistant" && msg.parentMessageId) {
        if (!modelsByParent.has(msg.parentMessageId)) {
          modelsByParent.set(msg.parentMessageId, new Set());
        }
        modelsByParent
          .get(msg.parentMessageId)!
          .add(msg.modelId || `__no_model_${msg.id}`);
      }
    }
    const result = new Set<string>();
    for (const [parentId, models] of modelsByParent) {
      if (models.size > 1) {
        result.add(parentId);
      }
    }
    return result;
  }, [messages]);

  // FE-I10 修复：multi-version 缓存改用 useRef（命令式缓存，引用稳定）。
  const multiModelVersions = useRef<Map<string, Message[]>>(new Map());
  const prevConvIdRef = useRef<string | undefined>(undefined);
  useEffect(() => {
    if (prevConvIdRef.current !== undefined && prevConvIdRef.current !== activeConversationId) {
      multiModelVersions.current.clear();
    }
    prevConvIdRef.current = activeConversationId ?? undefined;
  }, [activeConversationId]);

  const [displayModeOverrides, setDisplayModeOverrides] = useState<
    Map<string, MultiModelDisplayMode>
  >(new Map());
  const handleDisplayModeOverride = useCallback(
    (parentMsgId: string, mode: MultiModelDisplayMode) => {
      setDisplayModeOverrides((prev) => {
        const next = new Map(prev);
        next.set(parentMsgId, mode);
        return next;
      });
    },
    [setDisplayModeOverrides],
  );
  const handleMultiModelDetected = useCallback(
    (parentMsgId: string, versions: Message[]) => {
      const hadCached = multiModelVersions.current.has(parentMsgId);
      const stillMultiModel = hasMultipleModelVersions(versions);
      if (stillMultiModel) {
        multiModelVersions.current.set(parentMsgId, versions);
      } else {
        multiModelVersions.current.delete(parentMsgId);
      }
      // 只在缓存状态真正变化时才更新 displayModeOverrides，
      // 且仅当需要移除已存在的 override 时才创建新 Map，避免引用变化触发循环
      if (hadCached !== stillMultiModel) {
        setDisplayModeOverrides((prev) => {
          if (!stillMultiModel && prev.has(parentMsgId)) {
            const next = new Map(prev);
            next.delete(parentMsgId);
            return next;
          }
          // 无需变更时返回同一引用，避免触发下游 useCallback/useMemo 重建
          return prev;
        });
      }
    },
    [multiModelResponseParents],
  );

  const userSearchContentById = useMemo(() => {
    const next = new Map<string, ReturnType<typeof parseSearchContent>>();
    for (const msg of activeMessages) {
      if (msg.role === "user") {
        next.set(msg.id, parseSearchContent(msg.content));
      }
    }
    return next;
  }, [activeMessages]);

  // FE-I9 修复：bubble 缓存改用 useRef，useMemo 内重建后整体替换引用，
  // 避免渲染期原位 mutate useState 的 Map（引用不变导致下游不重算）。
  const bubbleItemCache = useRef<
    Map<string, { signature: string; item: BubbleItemType }>
  >(new Map());
  const bubbleItems: BubbleItemType[] = useMemo(() => {
    const cache = bubbleItemCache.current;
    const nextCache = new Map<
      string,
      { signature: string; item: BubbleItemType }
    >();
    const nextItems: BubbleItemType[] = [];

    for (const msg of activeMessages) {
      if (msg.role === "tool") {
        continue;
      }

      if (msg.role === "system" && msg.content === "<!-- context-clear -->") {
        const signature = "context-clear";
        const cached = cache.get(msg.id);
        const item = cached?.signature === signature
          ? cached.item
          : {
            key: msg.id,
            role: "context-clear",
            content: msg.id,
            variant: "borderless" as const,
          };
        nextCache.set(msg.id, { signature, item });
        nextItems.push(item);
        continue;
      }

      if (
        msg.role === "system"
        && msg.content === "<!-- context-compressed -->"
      ) {
        const signature = "context-compressed";
        const cached = cache.get(msg.id);
        const item = cached?.signature === signature
          ? cached.item
          : {
            key: msg.id,
            role: "context-compressed",
            content: msg.id,
            variant: "borderless" as const,
          };
        nextCache.set(msg.id, { signature, item });
        nextItems.push(item);
        continue;
      }

      if (msg.role === "user") {
        const { userContent } = userSearchContentById.get(msg.id) ?? parseSearchContent(msg.content);
        const signature = `user:${userContent}`;
        const cached = cache.get(msg.id);
        const item = cached?.signature === signature
          ? cached.item
          : { key: msg.id, role: "user", content: userContent };
        nextCache.set(msg.id, { signature, item });
        nextItems.push(item);
        continue;
      }

      let aiContent = msg.role === "assistant"
        ? buildAssistantDisplayContent(msg, activeMessages)
        : msg.content;
      if (shouldHideAssistantBubble(msg, aiContent)) {
        continue;
      }
      // js-set-map-lookups: 单次子串检查，Set 无优化收益
      if (
        msg.role === "assistant"
        && thinkingActiveMessageIds.has(msg.id)
        && aiContent.includes("<think")
      ) {
        const lastOpen = aiContent.lastIndexOf("<think");
        const lastClose = aiContent.lastIndexOf("```");
        if (lastClose < lastOpen) {
          aiContent += THINKING_LOADING_MARKER + "\n```\n\n";
        }
      }
      // js-set-map-lookups: 单次子串检查，Set 无优化收益
      if (msg.role === "assistant" && !aiContent.includes('data-axagent="1"')) {
        const parentSearch = msg.parentMessageId
          ? userSearchContentById.get(msg.parentMessageId)
          : undefined;
        if (parentSearch?.hasSearch && parentSearch.sources.length > 0) {
          const { sources } = parentSearch;
          const resultsJson = JSON.stringify(
            sources.map((s) => ({ title: s.title, url: s.url })),
          );
          aiContent = `<web-search status="done" data-axagent="1">\n${resultsJson}\n</web-search>\n\n${aiContent}`;
        }
      }

      const stableKey = msg.parentMessageId
        ? `ai:${msg.parentMessageId}:${msg.id}`
        : msg.id;
      if (nextCache.has(stableKey)) {
        continue;
      }
      const signature = `ai:${msg.id}:${aiContent}`;
      const cached = cache.get(stableKey);
      const item = cached?.signature === signature
        ? cached.item
        : { key: stableKey, role: "ai", content: aiContent };
      nextCache.set(stableKey, { signature, item });
      nextItems.push(item);
    }

    // FE-I9 修复：整体替换缓存引用，替代原位 clear/set。
    bubbleItemCache.current = nextCache;
    return nextItems;
  }, [activeMessages, thinkingActiveMessageIds, userSearchContentById]);

  const [expertSwitchBubble, setExpertSwitchBubble] = useState<BubbleItemType | null>(null);
  const expertSwitchCounterRef = useRef(0);
  useEffect(() => {
    if (!activeConversationId) {
      return;
    }
    const sw = consumeSwitch(activeConversationId);
    if (!sw) {
      return;
    }
    const role = getRoleById(sw.roleId);
    const name = role?.name ?? t("chat.generalAssistant");
    const icon = role?.icon ?? "\uD83E\uDD16";
    setExpertSwitchBubble({
      key: `__expert-switch__${sw.roleId}__${++expertSwitchCounterRef.current}`,
      role: "expert-switch",
      content: JSON.stringify({ icon, name: t("chat.switchedTo", { name }) }),
      variant: "borderless" as const,
    } as BubbleItemType);
  }, [activeConversationId, consumeSwitch, getRoleById, t]);

  const topicGroupEnabledByConv = useTopicGroupStore((s) =>
    activeConversationId
      ? s.enabledByConversation[activeConversationId]
      : undefined
  );
  const topicGroupsByConv = useTopicGroupStore((s) =>
    activeConversationId ? s.groupsByConversation[activeConversationId] : null
  );

  // 主题分组启用后，随消息变化增量归组（新增 user 消息开新组、新增回复并入当前组）。
  // 依赖最后一条消息 id：流式期间同 id 内容更新不会重复触发，只有新消息才会。
  const lastActiveMsgId = activeMessages[activeMessages.length - 1]?.id;
  useEffect(() => {
    if (!activeConversationId || !topicGroupEnabledByConv) {
      return;
    }
    useTopicGroupStore.getState().autoDetect(activeConversationId);
  }, [activeConversationId, topicGroupEnabledByConv, lastActiveMsgId]);

  // 跨天分隔条文案：今天/昨天/其余按本地化日期（当年省略年份）
  const formatDividerLabel = useCallback(
    (d: Date) => {
      const now = new Date();
      const startOfToday = new Date(now.getFullYear(), now.getMonth(), now.getDate());
      const startOfDay = new Date(d.getFullYear(), d.getMonth(), d.getDate());
      const diffDays = Math.round(
        (startOfToday.getTime() - startOfDay.getTime()) / 86_400_000,
      );
      if (diffDays === 0) {
        return t("chat.today");
      }
      if (diffDays === 1) {
        return t("chat.yesterday");
      }
      const locale = i18n.language.startsWith("zh")
        ? "zh-CN"
        : "en-US";
      const sameYear = d.getFullYear() === now.getFullYear();
      return d.toLocaleDateString(locale, {
        month: "long",
        day: "numeric",
        ...(sameYear ? {} : { year: "numeric" }),
      });
    },
    [i18n.language, t],
  );

  const allBubbleItems = useMemo(() => {
    let items = bubbleItems;
    const topicEnabled = activeConversationId && topicGroupEnabledByConv;
    const topicGroups = activeConversationId ? topicGroupsByConv : null;
    if (topicEnabled && topicGroups && topicGroups.length > 0) {
      const msgKeyToGroup = new Map<string, (typeof topicGroups)[0]>();
      for (const g of topicGroups) {
        for (const mid of g.messageIds) {
          msgKeyToGroup.set(mid, g);
        }
      }
      const enhanced: typeof items = [];
      let lastGroupId: string | null = null;
      for (const item of items) {
        const key = String(item.key);
        // 分组表键为真实消息 id，气泡 key 可能是 `ai:<parentId>:<msgId>`，
        // 必须解析出真实 msgId 再匹配，否则 AI 消息永远命中不了分组。
        const group = msgKeyToGroup.get(parseBubbleMsgId(key));
        if (group && group.id !== lastGroupId) {
          lastGroupId = group.id;
          enhanced.push({
            key: `__topic-group__${group.id}`,
            role: "topic-group",
            content: group,
            variant: "borderless" as const,
          } as BubbleItemType);
        }
        if (group && group.collapsed) {
          continue;
        }
        enhanced.push(item);
      }
      items = enhanced;
    }
    if (expertSwitchBubble) {
      items = [...items, expertSwitchBubble];
    }
    if (compressing) {
      items = [
        ...items,
        {
          key: "__compressing__",
          role: "context-compressing",
          content: "",
          variant: "borderless" as const,
        },
      ];
    }

    // 跨天分组：在每天的第一条真实消息前插入日期分隔条
    const resolveItemTime = (item: BubbleItemType): number | null => {
      if (item.role === "user" || item.role === "ai") {
        const msg = messageById.get(parseBubbleMsgId(String(item.key)));
        return msg?.createdAt ?? null;
      }
      return null;
    };
    const dated: typeof items = [];
    let lastDayKey = "";
    for (const item of items) {
      const ts = resolveItemTime(item);
      if (ts != null) {
        const d = new Date(ts * 1000);
        const dayKey = `${d.getFullYear()}-${d.getMonth()}-${d.getDate()}`;
        if (dayKey !== lastDayKey) {
          dated.push({
            key: `__date-divider__${dayKey}`,
            role: "date-divider",
            content: formatDividerLabel(d),
            variant: "borderless" as const,
          });
          lastDayKey = dayKey;
        }
      }
      dated.push(item);
    }
    items = dated;

    return items;
  }, [
    bubbleItems,
    compressing,
    activeConversationId,
    expertSwitchBubble,
    topicGroupEnabledByConv,
    topicGroupsByConv,
    messageById,
    formatDividerLabel,
  ]);

  const visibleBubbleItems = useMemo(() => {
    return [...allBubbleItems].reverse();
  }, [allBubbleItems]);

  // FE-I9 修复：AI 内容解析缓存改用 useRef（命令式缓存，引用稳定）。
  const aiContentNodesCache = useRef<
    Map<string, { content: string; nodes: ChatMarkdownNode[] }>
  >(new Map());
  const aiContentNodesById = useMemo(() => {
    const cache = aiContentNodesCache.current;
    const next = new Map<string, ChatMarkdownNode[]>();
    for (const item of bubbleItems) {
      if (item.role !== "ai" || typeof item.content !== "string") {
        continue;
      }
      const msg = messageById.get(parseBubbleMsgId(String(item.key)));
      if (msg?.status === "error") {
        continue;
      }
      const shouldRenderFromContent = shouldRenderAssistantMarkdownFromContent(
        streaming && msg?.id === streamingMessageId,
        Boolean(msg?.id && contentRendererMessageIds.current.has(msg.id)),
      );
      if (shouldRenderFromContent) {
        continue;
      }
      const messageId = String(item.key);
      const cached = cache.get(messageId);
      if (cached && cached.content === item.content) {
        next.set(messageId, cached.nodes);
        continue;
      }
      const nodes = parseChatMarkdown(item.content);
      if (cache.size >= 100) {
        const firstKey = cache.keys().next().value;
        if (firstKey !== undefined) {
          cache.delete(firstKey);
        }
      }
      cache.set(messageId, { content: item.content, nodes });
      next.set(messageId, nodes);
    }
    for (const messageId of Array.from(cache.keys())) {
      if (!next.has(messageId)) {
        cache.delete(messageId);
      }
    }
    return next;
  }, [
    bubbleItems,
    messageById,
    streaming,
    streamingMessageId,
  ]);

  // 后端 now_ts() 返回秒级时间戳，需转毫秒再构造 Date
  const formatTime = useCallback((ts: number) => {
    const d = new Date(ts * 1000);
    const now = new Date();
    const isToday = d.getFullYear() === now.getFullYear()
      && d.getMonth() === now.getMonth()
      && d.getDate() === now.getDate();
    const timeStr = `${String(d.getHours()).padStart(2, "0")}:${String(d.getMinutes()).padStart(2, "0")}`;
    if (isToday) {
      return timeStr;
    }
    return `${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")} ${timeStr}`;
  }, []);

  // 完整时间戳（含年份、秒），用于悬停气泡展示
  const formatFullTimestamp = useCallback((ts: number) => {
    const d = new Date(ts * 1000);
    const pad = (n: number) => String(n).padStart(2, "0");
    return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${
      pad(
        d.getHours(),
      )
    }:${pad(d.getMinutes())}:${pad(d.getSeconds())}`;
  }, []);

  const getModelDisplayInfo = useCallback(
    (modelId?: string | null, providerId?: string | null) => {
      const mid = modelId ?? activeConversation?.modelId;
      const pid = providerId ?? activeConversation?.providerId;
      if (!mid) {
        return { modelName: "AI", providerName: "" };
      }
      const provider = providers.find((p) => p.id === pid);
      const model = provider?.models.find((m) => m.modelId === mid);
      return {
        modelName: model?.name ?? mid,
        providerName: provider?.name ?? "",
      };
    },
    [activeConversation, providers],
  );

  const getBubbleVariant = useCallback(
    (
      isUser: boolean,
    ): {
      variant: "filled" | "outlined" | "shadow" | "borderless";
      style?: React.CSSProperties;
    } => {
      switch (bubbleStyle) {
        case "compact":
          return { variant: "borderless", style: { borderLeft: "2px solid var(--color-primary)", padding: "4px 8px" } };
        case "minimal":
          return { variant: "borderless", style: { padding: "4px 8px", borderLeft: "2px solid var(--color-primary)" } };
        case "modern":
        default:
          return { variant: isUser ? "shadow" : "outlined" };
      }
    },
    [bubbleStyle],
  );

  const renderUserAvatar = useCallback(() => {
    const size = 32;
    if (profile.avatarType === "emoji" && profile.avatarValue) {
      return (
        <div
          style={{
            width: size,
            height: size,
            borderRadius: "50%",
            backgroundColor: token.colorFillSecondary,
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            fontSize: 16,
          }}
        >
          {profile.avatarValue}
        </div>
      );
    }
    if (
      (profile.avatarType === "url" || profile.avatarType === "file")
      && profile.avatarValue
    ) {
      const src = profile.avatarType === "file" ? resolvedAvatarSrc : profile.avatarValue;
      return <Avatar size={size} src={src} />;
    }
    return (
      <Avatar
        size={size}
        icon={<User size={16} />}
        style={{ backgroundColor: token.colorPrimary }}
      />
    );
  }, [profile, token, resolvedAvatarSrc]);
  const userAvatar = useMemo(() => renderUserAvatar(), [renderUserAvatar]);

  const userRole = useCallback(
    (bubbleData: BubbleItemType) => {
      const msg = messageById.get(String(bubbleData.key));
      const attachments = msg?.attachments ?? [];
      const quotedMessage = getQuotedMessage(msg?.quotedMessageId);
      return {
        placement: "end" as const,
        className: "msg-row user",
        ...getBubbleVariant(true),
        avatar: userAvatar,
        contentRender: attachments.length > 0
          ? (content: ReactNode) => {
            const text = content as string;
            return (
              <div style={{ textAlign: "right" }}>
                <span
                  data-axagent-msg={msg?.id}
                  style={{ height: 0, overflow: "hidden", lineHeight: 0 }}
                />
                {quotedMessage && (
                  <div style={{ textAlign: "left" }}>
                    <QuoteBlock quotedMessage={quotedMessage} onJump={handleJumpToMessage} />
                  </div>
                )}
                {text
                  && (settings.renderUserMarkdown
                    ? (
                      <AssistantMarkdown
                        content={text}
                        isDarkMode={isDarkMode}
                        isStreaming={false}
                        codeBlockDarkTheme={codeBlockDarkTheme}
                        codeBlockLightTheme={codeBlockLightTheme}
                        codeBlockThemes={codeBlockThemes}
                        codeFontFamily={settings.codeFontFamily || undefined}
                      />
                    )
                    : <div style={{ whiteSpace: "pre-wrap" }}>{text}</div>)}
                <div
                  style={{
                    display: "flex",
                    flexWrap: "wrap",
                    gap: 8,
                    marginTop: text ? 8 : 0,
                    justifyContent: "flex-end",
                  }}
                >
                  {attachments.map((att, i) => (
                    <AttachmentPreview
                      key={att.id || `${att.fileName}-${i}`}
                      att={att}
                      themeColor={token.colorPrimary}
                    />
                  ))}
                </div>
              </div>
            );
          }
          : (content: ReactNode) => {
            const text = content as string;
            return (
              <>
                <span
                  data-axagent-msg={msg?.id}
                  style={{ height: 0, overflow: "hidden", lineHeight: 0 }}
                />
                {quotedMessage && (
                  <div style={{ textAlign: "left", marginBottom: 4 }}>
                    <QuoteBlock quotedMessage={quotedMessage} onJump={handleJumpToMessage} />
                  </div>
                )}
                {settings.renderUserMarkdown
                  ? (
                    <AssistantMarkdown
                      content={text}
                      isDarkMode={isDarkMode}
                      isStreaming={false}
                      codeBlockDarkTheme={codeBlockDarkTheme}
                      codeBlockLightTheme={codeBlockLightTheme}
                      codeBlockThemes={codeBlockThemes}
                      codeFontFamily={settings.codeFontFamily || undefined}
                    />
                  )
                  : text}
              </>
            );
          },
        header: (
          <div>
            <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
              <Typography.Text style={{ fontSize: 13 }}>
                {profile.name || t("chat.you")}
              </Typography.Text>
              {msg && (
                <Tooltip title={formatFullTimestamp(msg.createdAt)}>
                  <Typography.Text type="secondary" style={{ fontSize: 12 }}>
                    {formatTime(msg.createdAt)}
                  </Typography.Text>
                </Tooltip>
              )}
            </div>
          </div>
        ),
        footer: (
          <Actions
            items={[
              {
                key: "copy",
                icon: (() => {
                  const ct = stripAxAgentTags(String(bubbleData.content ?? ""));
                  return isUserMsgCopied(ct)
                    ? <Check size={14} style={{ color: token.colorSuccess }} />
                    : <Copy size={14} />;
                })(),
                label: t("chat.copy"),
                onItemClick: () => {
                  void copyMessage(
                    stripAxAgentTags(String(bubbleData.content ?? "")),
                  ).then((ok) => {
                    if (ok) {
                      messageApi.success(t("chat.copied"));
                    }
                  });
                },
              },
              {
                key: "save",
                actionRender: () => {
                  const userMd = stripAxAgentTags(String(bubbleData.content ?? ""));
                  const handleSaveAs = async (format: "md" | "docx" | "pdf") => {
                    const title = activeConversation?.title || "message";
                    try {
                      if (format === "md" || !isTauri()) {
                        const blob = new Blob([userMd], {
                          type: "text/markdown;charset=utf-8",
                        });
                        const url = URL.createObjectURL(blob);
                        const a = document.createElement("a");
                        a.href = url;
                        a.download = `${title.slice(0, 40)}.md`;
                        document.body.appendChild(a);
                        a.click();
                        document.body.removeChild(a);
                        URL.revokeObjectURL(url);
                        messageApi.success(t("chat.saved"));
                        return;
                      }
                      const { save } = await import("@tauri-apps/plugin-dialog");
                      const ext = format === "docx" ? "docx" : "pdf";
                      const name = `${title.slice(0, 40)}.${ext}`;
                      const filePath = await save({
                        defaultPath: name,
                        filters: format === "docx"
                          ? [{ name: t("stockAnalysis.docxFilterName"), extensions: ["docx"] }]
                          : [{ name: "PDF", extensions: ["pdf"] }],
                      });
                      if (!filePath) {
                        return;
                      }
                      await invoke<boolean>("export_content", {
                        markdown: userMd,
                        outputPath: filePath,
                        format,
                        title,
                      });
                      messageApi.success(t("chat.saved"));
                    } catch (e) {
                      messageApi.error(String(e));
                    }
                  };
                  return (
                    <Popover
                      trigger="click"
                      placement="top"
                      content={
                        <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
                          <button style={styleBtn} onClick={() => handleSaveAs("md")}>Markdown (.md)</button>
                          <button style={styleBtn} onClick={() => handleSaveAs("docx")}>Word (.docx)</button>
                          <button style={styleBtn} onClick={() => handleSaveAs("pdf")}>PDF (.pdf)</button>
                        </div>
                      }
                    >
                      <Tooltip title={t("chat.saveAs")}>
                        <span
                          className="axagent-action-item"
                          role="button"
                          tabIndex={0}
                          style={{ color: token.colorTextSecondary }}
                        >
                          <Save size={14} />
                        </span>
                      </Tooltip>
                    </Popover>
                  );
                },
              },
              {
                key: "edit",
                icon: <Pencil size={14} />,
                label: t("chat.editMessage"),
                onItemClick: () => {
                  if (msg) {
                    handleEditMessage(msg.id, msg.content, "user");
                  }
                },
              },
              {
                key: "quote",
                icon: <MessageSquare size={14} />,
                label: t("chat.quote.reply"),
                onItemClick: () => {
                  if (msg) {
                    setQuotedMessageId(msg.id);
                  }
                },
              },
              {
                key: "regenerate",
                icon: <RotateCcw size={14} />,
                label: t("chat.regenerate"),
                onItemClick: async () => {
                  if (!msg) { return; }
                  try {
                    // 必须传入本条 user 消息 id，否则 store 会回退到「最后一条
                    // user 消息」，中间消息点重生成会错误地重生成最后一条。
                    await regenerateMessage(msg.id);
                  } catch (e) {
                    messageApi.error(String(e));
                  }
                },
              },
              {
                key: "delete",
                actionRender: () => (
                  <Popconfirm
                    title={t("chat.confirmDeleteMessage")}
                    onConfirm={async () => {
                      if (msg && activeConversationId) {
                        try {
                          await deleteMessageGroup(
                            activeConversationId,
                            msg.id,
                          );
                        } catch (e) {
                          messageApi.error(String(e));
                        }
                      }
                    }}
                    okText={t("common.confirm")}
                    cancelText={t("common.cancel")}
                  >
                    <Tooltip title={t("chat.delete")}>
                      <span
                        className="axagent-action-item"
                        style={{ color: token.colorError }}
                      >
                        <Trash2 size={14} />
                      </span>
                    </Tooltip>
                  </Popconfirm>
                ),
              },
            ]}
          />
        ),
      };
    },
    [
      activeConversationId,
      codeBlockDarkTheme,
      codeBlockLightTheme,
      codeBlockThemes,
      copyMessage,
      deleteMessageGroup,
      formatTime,
      getBubbleVariant,
      getQuotedMessage,
      handleEditMessage,
      handleJumpToMessage,
      isDarkMode,
      isUserMsgCopied,
      messageApi,
      messageById,
      profile.name,
      regenerateMessage,
      settings.codeFontFamily,
      settings.renderUserMarkdown,
      setQuotedMessageId,
      t,
      token.colorError,
      token.colorPrimary,
      token.colorSuccess,
      userAvatar,
    ],
  );

  const renderConvIconForChat = useCallback(
    (size: number, modelId?: string | null) => {
      if (!activeConversation) {
        // 无会话时渲染默认 Bot 头像
        return (
          <Avatar
            icon={<Bot size={16} />}
            style={{ background: token.colorPrimary }}
            size={size}
          />
        );
      }
      const customIcon = getConvIcon(activeConversation.id);
      if (customIcon) {
        if (customIcon.type === "emoji") {
          return (
            <Avatar
              size={size}
              style={{
                fontSize: Math.round(size * 0.5),
                backgroundColor: token.colorPrimaryBg,
              }}
            >
              {customIcon.value}
            </Avatar>
          );
        }
        return <Avatar size={size} src={customIcon.value} />;
      }
      const mid = modelId ?? activeConversation.modelId;
      if (mid) {
        return <ModelIcon model={mid} size={size} type="avatar" />;
      }
      return (
        <Avatar
          icon={<Bot size={16} />}
          style={{ background: token.colorPrimary }}
          size={size}
        />
      );
    },
    [activeConversation, token.colorPrimary, token.colorPrimaryBg],
  );

  const aiRole = useCallback(
    (bubbleData: BubbleItemType) => {
      // 气泡 key 携带 msgId（`ai:<parentId>:<msgId>`），必须解析出真实消息 id 再查表，
      // 不能直接用 key 匹配 assistantByParentId（其 key 为 `ai:<parentId>`）。
      const msg = messageById.get(parseBubbleMsgId(String(bubbleData.key)));
      const isStreaming = streaming && msg?.id === streamingMessageId;
      const shouldRenderFromContent = shouldRenderAssistantMarkdownFromContent(
        isStreaming,
        Boolean(msg?.id && contentRendererMessageIds.current.has(msg.id)),
      );
      const assistantCopyText = stripAxAgentTags(
        msg?.content
          ?? (typeof bubbleData.content === "string" ? bubbleData.content : ""),
      );
      const parsedNodes = shouldRenderFromContent
        ? undefined
        : aiContentNodesById.get(String(bubbleData.key));
      const { bubbleLoading: rawBubbleLoading, footerLoading } = getStreamingLoadingState(
        isStreaming,
        bubbleData.content,
      );
      const isMultiModelMsg = !!multiModelParentId && msg?.parentMessageId === multiModelParentId;
      const isAgentMsg = activeConversation?.mode === "agent";
      const bubbleLoading = isMultiModelMsg || isAgentMsg ? false : rawBubbleLoading;

      const parentId = msg?.parentMessageId;
      const hasMultiModels = !!parentId
        && (multiModelResponseParents.has(parentId)
          || multiModelVersions.current.has(parentId));
      const effectiveDisplayMode: MultiModelDisplayMode = hasMultiModels
        ? (displayModeOverrides.get(parentId)
          ?? settings.multiModelDisplayMode
          ?? "tabs")
        : "tabs";
      const isNonTabsMultiModel = hasMultiModels && effectiveDisplayMode !== "tabs";

      return {
        placement: "start" as const,
        className: "msg-row assistant",
        ...getBubbleVariant(false),
        avatar: isNonTabsMultiModel
          ? undefined
          : renderConvIconForChat(32, msg?.modelId),
        loading: bubbleLoading,
        contentRender: (content: ReactNode) => {
          const text = content as string;
          const msgMarker = (
            <span
              data-axagent-msg={msg?.id}
              style={{ height: 0, overflow: "hidden", lineHeight: 0 }}
            />
          );
          // 引用回复：被引用消息预览块（streaming 时不显示，避免干扰）
          const quotedMessage = !isStreaming ? getQuotedMessage(msg?.quotedMessageId) : null;
          const quoteBlock = quotedMessage
            ? <QuoteBlock quotedMessage={quotedMessage} onJump={handleJumpToMessage} />
            : null;
          if (msg?.status === "error") {
            return (
              <>
                {msgMarker}
                {quoteBlock}
                <Alert
                  type="error"
                  title={text.length > 200 ? text.slice(0, 200) + "…" : text}
                  description={text.length > 100
                    ? (
                      <div
                        style={{
                          maxHeight: 500,
                          overflowY: "auto",
                          marginTop: 4,
                        }}
                      >
                        <AssistantMarkdown
                          content={text}
                          isDarkMode={isDarkMode}
                          isStreaming={false}
                          codeBlockDarkTheme={codeBlockDarkTheme}
                          codeBlockLightTheme={codeBlockLightTheme}
                          codeBlockThemes={codeBlockThemes}
                          codeFontFamily={settings.codeFontFamily || undefined}
                        />
                      </div>
                    )
                    : undefined}
                  showIcon
                />
              </>
            );
          }

          if (isNonTabsMultiModel && parentId && activeConversationId) {
            const refVersions = multiModelVersions.current.get(parentId);
            const storeVersions = messages.filter(
              (m) => m.parentMessageId === parentId && m.role === "assistant",
            );
            const allVersions = refVersions && refVersions.length > storeVersions.length
              ? refVersions
              : storeVersions;
            return (
              <>
                {msgMarker}
                {quoteBlock}
                <MultiModelDisplay
                  versions={allVersions}
                  activeMessageId={msg!.id}
                  mode={effectiveDisplayMode as "side-by-side" | "stacked"}
                  conversationId={activeConversationId}
                  onSwitchVersion={(pid, mid) => switchMessageVersion(activeConversationId, pid, mid)}
                  onDeleteVersion={(mid) => deleteMessage(mid)}
                  streamingMessageId={streamingMessageId}
                  multiModelDoneMessageIds={multiModelDoneMessageIds}
                  getModelDisplayInfo={getModelDisplayInfo}
                  renderContent={(vMsg, isVersionStreaming) => (
                    <>
                      <AssistantMarkdown
                        content={buildAssistantDisplayContent(
                          vMsg,
                          activeMessages,
                        )}
                        isDarkMode={isDarkMode}
                        isStreaming={isVersionStreaming}
                        codeBlockDarkTheme={codeBlockDarkTheme}
                        codeBlockLightTheme={codeBlockLightTheme}
                        codeBlockThemes={codeBlockThemes}
                        codeFontFamily={settings.codeFontFamily || undefined}
                        messageId={vMsg?.id}
                      />
                      {vMsg?.blocks
                        && vMsg.blocks.some((b) => b.type === "tool_use")
                        && !isVersionStreaming
                        && <ToolCallBlockView blocks={vMsg.blocks} createdAt={vMsg.createdAt} />}
                    </>
                  )}
                />
              </>
            );
          }

          // Compute agent permissions/askUsers early for unified loading decision
          const isAgentMode = activeConversation?.mode === "agent";
          const msgPermissions = isAgentMode && msg && activeConversationId
            ? Object.values(agentPendingPermissions).filter(
              (pr) =>
                pr.conversationId === activeConversationId
                && (pr.assistantMessageId === msg.id
                  || (pr.assistantMessageId === ""
                    && msg.id === streamingMessageId)),
            )
            : [];
          const msgAskUsers = isAgentMode && msg && activeConversationId
            ? Object.values(agentPendingAskUser).filter(
              (ask) =>
                ask.conversationId === activeConversationId
                && (ask.assistantMessageId === msg.id
                  || (ask.assistantMessageId === ""
                    && msg.id === streamingMessageId)),
            )
            : [];

          // Unified loading dots: show unless Agent mode has pending permissions/askUsers
          if (rawBubbleLoading && (!isAgentMsg || (msgPermissions.length === 0 && msgAskUsers.length === 0))) {
            return (
              <>
                {msgMarker}
                {quoteBlock}
                <span className="axagent-streaming-dots" aria-hidden="true">
                  <span />
                  <span />
                  <span />
                </span>
              </>
            );
          }

          // ── AI 消息折叠 ──
          const msgId = msg?.id;
          const isCollapsed = !!msgId && !isStreaming && collapsedAiIds[msgId];

          const handleToggleCollapse = () => {
            if (!msgId) { return; }
            setCollapsedAiIds((prev) => {
              const next = { ...prev };
              if (next[msgId]) {
                delete next[msgId];
              } else {
                next[msgId] = true;
              }
              return next;
            });
          };

          const handleExpand = () => {
            if (!msgId) { return; }
            setCollapsedAiIds((prev) => {
              const next = { ...prev };
              delete next[msgId];
              return next;
            });
          };

          return (
            <>
              {msgMarker}
              {quoteBlock}
              <div className={`ai-content-wrapper${isCollapsed ? " ai-content-collapsed" : ""}`}>
                <AssistantMarkdown
                  content={text}
                  nodes={parsedNodes}
                  isDarkMode={isDarkMode}
                  isStreaming={isStreaming}
                  codeBlockDarkTheme={codeBlockDarkTheme}
                  codeBlockLightTheme={codeBlockLightTheme}
                  codeBlockThemes={codeBlockThemes}
                  codeFontFamily={settings.codeFontFamily || undefined}
                  messageId={msg?.id}
                />
                {isCollapsed && (
                  <div
                    className="ai-collapse-overlay"
                    onClick={handleExpand}
                  >
                    <span className="ai-collapse-label">{t("chat.expandMessage")}</span>
                  </div>
                )}
                {/* 未折叠时显示 toggle 按钮允许主动折叠；折叠时由 overlay 处理展开，不再重复 */}
                {!isStreaming && !!msgId && !isCollapsed && (
                  <button
                    className="ai-collapse-toggle"
                    onClick={handleToggleCollapse}
                  >
                    {t("common.collapse")}
                  </button>
                )}
              </div>
              {msgPermissions.map((pr) => {
                const resolvedTc = agentToolCalls[pr.toolUseId];
                const permStatus = resolvedTc?.approvalStatus === "approved"
                  ? "approved"
                  : resolvedTc?.approvalStatus === "denied"
                  ? "denied"
                  : "pending";
                return (
                  <PermissionCard
                    key={pr.toolUseId}
                    conversationId={pr.conversationId}
                    toolUseId={pr.toolUseId}
                    toolName={pr.toolName}
                    input={pr.input}
                    status={permStatus}
                  />
                );
              })}
              {msgAskUsers.map((ask) => (
                <AskUserCard
                  key={ask.askId}
                  askId={ask.askId}
                  question={ask.question}
                  options={ask.options}
                />
              ))}
              {/* Persisted tool calls from message blocks */}
              {msg?.blocks && msg.blocks.some((b) => b.type === "tool_use") && !isStreaming && (
                <ToolCallBlockView blocks={msg.blocks} createdAt={msg.createdAt} />
              )}
              {isAgentMode
                && msg
                && activeConversationId
                && Object.values(agentToolCalls).filter(
                    (tc) =>
                      tc.assistantMessageId === msg.id
                      && tc.executionStatus !== "queued",
                  ).length > 0
                && (
                  <ToolCallCard
                    toolCalls={Object.values(agentToolCalls).filter(
                      (tc) =>
                        tc.assistantMessageId === msg.id
                        && tc.executionStatus !== "queued",
                    )}
                  />
                )}
              {isAgentMsg && isStreaming && !footerLoading && (
                <div
                  className="axagent-streaming-dots"
                  aria-hidden="true"
                  style={{ marginTop: 8 }}
                >
                  <span />
                  <span />
                  <span />
                </div>
              )}
              {/* 认知编排决策标签：每条（历史）消息独立展示各自决策信息 */}
              <CognitiveDecisionCard decision={msg?.decision} />
            </>
          );
        },
        header: (() => {
          if (isNonTabsMultiModel) {
            return null;
          }
          const { modelName, providerName } = getModelDisplayInfo(
            msg?.modelId,
            msg?.providerId,
          );
          return (
            <div style={{ display: "flex", flexDirection: "column", gap: 2 }}>
              <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
                {providerName && (
                  <Tag
                    style={{
                      fontSize: 12,
                      margin: 0,
                      padding: "0 4px",
                      lineHeight: "18px",
                      color: token.colorPrimary,
                      backgroundColor: token.colorPrimaryBg,
                      border: "none",
                    }}
                  >
                    {providerName}
                  </Tag>
                )}
                <Typography.Text style={{ fontSize: 13 }}>
                  {modelName}
                </Typography.Text>
                {msg && (
                  <Tooltip title={formatFullTimestamp(msg.createdAt)}>
                    <Typography.Text type="secondary" style={{ fontSize: 12 }}>
                      {formatTime(msg.createdAt)}
                    </Typography.Text>
                  </Tooltip>
                )}
                {msg?.status === "partial"
                  && !isStreaming
                  && !(
                    multiModelParentId
                    && msg.parentMessageId === multiModelParentId
                  ) && (
                  <Tag
                    color="orange"
                    style={{
                      fontSize: 12,
                      margin: 0,
                      padding: "0 4px",
                      lineHeight: "16px",
                      border: "none",
                    }}
                  >
                    {t("chat.partial")}
                  </Tag>
                )}
              </div>
            </div>
          );
        })(),
        footer: msg && activeConversationId
          ? (
            <div style={{ display: "flex", flexDirection: "column" }}>
              {footerLoading && !isNonTabsMultiModel && (
                <div
                  style={{
                    display: "inline-flex",
                    alignItems: "center",
                    color: token.colorPrimary,
                  }}
                  aria-label={t("chat.generating")}
                >
                  <span className="axagent-streaming-dots" aria-hidden="true">
                    <span />
                    <span />
                    <span />
                  </span>
                </div>
              )}
              <AssistantFooter
                msg={msg}
                conversationId={activeConversationId}
                assistantCopyText={assistantCopyText}
                getModelDisplayInfo={getModelDisplayInfo}
                isStreaming={isStreaming}
                displayMode={effectiveDisplayMode}
                onDisplayModeChange={handleDisplayModeOverride}
                onMultiModelDetected={handleMultiModelDetected}
                onQuoteReply={(messageId) => setQuotedMessageId(messageId)}
                isDarkMode={isDarkMode}
                codeBlockDarkTheme={codeBlockDarkTheme}
                codeBlockLightTheme={codeBlockLightTheme}
                codeBlockThemes={codeBlockThemes}
                codeFontFamily={settings.codeFontFamily || undefined}
              />
            </div>
          )
          : footerLoading
          ? (
            <div
              style={{
                display: "inline-flex",
                alignItems: "center",
                color: token.colorPrimary,
              }}
              aria-label={t("chat.generating")}
            >
              <span className="axagent-streaming-dots" aria-hidden="true">
                <span />
                <span />
                <span />
              </span>
            </div>
          )
          : null,
      };
    },
    [
      activeConversation,
      activeConversationId,
      activeMessages,
      agentPendingAskUser,
      agentPendingPermissions,
      agentToolCalls,
      aiContentNodesById,
      codeBlockDarkTheme,
      codeBlockLightTheme,
      codeBlockThemes,
      collapsedAiIds,
      deleteMessage,
      displayModeOverrides,
      formatTime,
      getBubbleVariant,
      getModelDisplayInfo,
      getQuotedMessage,
      handleDisplayModeOverride,
      handleEditMessage,
      handleJumpToMessage,
      handleMultiModelDetected,
      isDarkMode,
      messageById,
      messages,
      multiModelDoneMessageIds,
      multiModelParentId,
      multiModelResponseParents,
      renderConvIconForChat,
      settings,
      setCollapsedAiIds,
      setQuotedMessageId,
      streaming,
      streamingMessageId,
      switchMessageVersion,
      t,
      token.colorPrimary,
      token.colorPrimaryBg,
    ],
  );

  const contextClearRole = useCallback(
    (bubbleData: BubbleItemType) => {
      const msgId = String(bubbleData.content ?? "");
      return {
        placement: "start" as const,
        variant: "borderless" as const,
        className: "context-clear-bubble",
        contentRender: () => (
          <div
            style={{
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
              padding: "8px 0",
              width: "100%",
            }}
          >
            <div
              style={{
                flex: 1,
                height: 1,
                borderTop: `1px dashed ${token.colorBorderSecondary}`,
              }}
            />
            <span
              style={{
                margin: "0 12px",
                color: token.colorTextQuaternary,
                fontSize: 12,
                display: "inline-flex",
                alignItems: "center",
                whiteSpace: "nowrap",
                userSelect: "none",
              }}
            >
              <Scissors size={14} style={{ marginRight: 4 }} /> {t("chat.contextCleared")}
              <X
                size={14}
                style={{ marginLeft: 6, cursor: "pointer" }}
                onClick={() => {
                  void removeContextClear(msgId).catch((err) => {
                    messageApi.error(String(err));
                  });
                }}
              />
            </span>
            <div
              style={{
                flex: 1,
                height: 1,
                borderTop: `1px dashed ${token.colorBorderSecondary}`,
              }}
            />
          </div>
        ),
      };
    },
    [
      messageApi,
      removeContextClear,
      t,
      token.colorBorderSecondary,
      token.colorTextQuaternary,
    ],
  );

  const contextCompressedRole = useCallback(
    (_bubbleData: BubbleItemType) => {
      return {
        placement: "start" as const,
        variant: "borderless" as const,
        className: "context-clear-bubble",
        contentRender: () => (
          <div
            style={{
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
              padding: "8px 0",
              width: "100%",
            }}
          >
            <div
              style={{
                flex: 1,
                height: 1,
                borderTop: `1px dashed ${token.colorPrimaryBorder}`,
              }}
            />
            <span
              style={{
                margin: "0 12px",
                color: token.colorPrimary,
                fontSize: 12,
                display: "inline-flex",
                alignItems: "center",
                whiteSpace: "nowrap",
                userSelect: "none",
                cursor: "pointer",
                gap: 4,
              }}
            >
              <span
                role="button"
                tabIndex={0}
                onKeyDown={(e) => {
                  if (e.key === "Enter" || e.key === " ") {
                    e.preventDefault(); /* handler runs onClick */
                  }
                }}
                style={{ display: "inline-flex", alignItems: "center", gap: 4 }}
                onClick={async () => {
                  const convId = activeConversationId;
                  if (!convId) {
                    return;
                  }
                  const summary = await getCompressionSummary(convId);
                  setSummaryModalText(
                    summary?.summaryText ?? t("chat.noSummary"),
                  );
                  setSummaryModalOpen(true);
                }}
              >
                <Zap size={14} /> {t("chat.contextCompressed")}
              </span>
              <Popconfirm
                title={t("chat.deleteCompressionConfirm")}
                onConfirm={async () => {
                  try {
                    await deleteCompression();
                  } catch {
                    // error already logged in store
                  }
                }}
                okText={t("common.confirm")}
                cancelText={t("common.cancel")}
              >
                <X
                  size={14}
                  style={{
                    cursor: "pointer",
                    color: token.colorTextTertiary,
                    flexShrink: 0,
                  }}
                  onClick={(e) => e.stopPropagation()}
                />
              </Popconfirm>
            </span>
            <div
              style={{
                flex: 1,
                height: 1,
                borderTop: `1px dashed ${token.colorPrimaryBorder}`,
              }}
            />
          </div>
        ),
      };
    },
    [
      activeConversationId,
      deleteCompression,
      getCompressionSummary,
      t,
      token.colorPrimary,
      token.colorPrimaryBorder,
      token.colorTextTertiary,
    ],
  );

  const contextCompressingRole = useCallback(() => {
    return {
      placement: "start" as const,
      variant: "borderless" as const,
      className: "context-clear-bubble",
      contentRender: () => (
        <div
          style={{
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            padding: "8px 0",
            width: "100%",
          }}
        >
          <div
            style={{
              flex: 1,
              height: 1,
              borderTop: `1px dashed ${token.colorPrimaryBorder}`,
            }}
          />
          <span
            style={{
              margin: "0 12px",
              color: token.colorPrimary,
              fontSize: 12,
              display: "inline-flex",
              alignItems: "center",
              whiteSpace: "nowrap",
              userSelect: "none",
            }}
          >
            <Spin size="small" style={{ marginRight: 6 }} /> {t("chat.compressing")}
          </span>
          <div
            style={{
              flex: 1,
              height: 1,
              borderTop: `1px dashed ${token.colorPrimaryBorder}`,
            }}
          />
        </div>
      ),
    };
  }, [t, token.colorPrimary, token.colorPrimaryBorder]);

  const expertSwitchRole = useCallback(
    (bubbleData: BubbleItemType) => {
      let icon = "\uD83E\uDD16";
      let name = t("chat.switchedTo", { name: t("chat.generalAssistant") });
      try {
        const data = JSON.parse(String(bubbleData.content ?? "{}"));
        icon = data.icon || icon;
        name = data.name || name;
      } catch {
        /* use defaults */
      }
      return {
        placement: "start" as const,
        variant: "borderless" as const,
        className: "context-clear-bubble",
        contentRender: () => (
          <div
            style={{
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
              padding: "12px 0",
              width: "100%",
            }}
          >
            <div
              style={{
                flex: 1,
                height: 1,
                borderTop: `1px dashed ${token.colorPrimaryBorder}`,
              }}
            />
            <span
              style={{
                margin: "0 12px",
                color: token.colorPrimary,
                fontSize: 12,
                display: "inline-flex",
                alignItems: "center",
                whiteSpace: "nowrap",
                userSelect: "none",
              }}
            >
              <span style={{ marginRight: 4 }}>{icon}</span> {name}
            </span>
            <div
              style={{
                flex: 1,
                height: 1,
                borderTop: `1px dashed ${token.colorPrimaryBorder}`,
              }}
            />
          </div>
        ),
      };
    },
    [t, token.colorPrimary, token.colorPrimaryBorder],
  );

  const topicGroupRole = useCallback(
    (bubbleData: BubbleItemType) => {
      const group = bubbleData.content as import("@/stores/feature/topicGroupStore").TopicGroup;
      if (!group || !activeConversationId) {
        return {
          placement: "start" as const,
          variant: "borderless" as const,
          className: "context-clear-bubble",
          contentRender: () => null,
        };
      }
      return {
        placement: "start" as const,
        variant: "borderless" as const,
        className: "context-clear-bubble",
        contentRender: () => (
          <TopicGroupDivider
            conversationId={activeConversationId}
            group={group}
          />
        ),
      };
    },
    [activeConversationId],
  );

  const dateDividerRole = useCallback(
    (bubbleData: BubbleItemType) => {
      const label = typeof bubbleData.content === "string"
        ? bubbleData.content
        : "";
      return {
        placement: "start" as const,
        variant: "borderless" as const,
        className: "context-clear-bubble",
        contentRender: () => (
          <div
            style={{
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
              padding: "10px 0",
              width: "100%",
            }}
          >
            <div
              style={{
                flex: 1,
                height: 1,
                borderTop: `1px solid ${token.colorSplit}`,
              }}
            />
            <span
              style={{
                margin: "0 12px",
                color: token.colorTextTertiary,
                fontSize: 12,
                display: "inline-flex",
                alignItems: "center",
                whiteSpace: "nowrap",
                userSelect: "none",
              }}
            >
              {label}
            </span>
            <div
              style={{
                flex: 1,
                height: 1,
                borderTop: `1px solid ${token.colorSplit}`,
              }}
            />
          </div>
        ),
      };
    },
    [token.colorSplit, token.colorTextTertiary],
  );

  const roles: RoleType = useMemo(
    () => ({
      user: userRole,
      ai: aiRole,
      "date-divider": dateDividerRole,
      "context-clear": contextClearRole,
      "context-compressed": contextCompressedRole,
      "context-compressing": contextCompressingRole,
      "expert-switch": expertSwitchRole,
      "topic-group": topicGroupRole,
    }),
    [
      aiRole,
      contextClearRole,
      contextCompressedRole,
      contextCompressingRole,
      dateDividerRole,
      expertSwitchRole,
      userRole,
      topicGroupRole,
    ],
  );

  const lastBubbleKey = allBubbleItems.length > 0
    ? String(allBubbleItems[allBubbleItems.length - 1].key)
    : "";

  return {
    allBubbleItems,
    visibleBubbleItems,
    lastBubbleKey,
    roles,
    bubbleItems,
    summaryModalOpen,
    setSummaryModalOpen,
    summaryModalText,
    renderConvIconForChat,
  };
}

// SPDX-License-Identifier: AGPL-3.0-only

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

// Local replacement for @ant-design/x Actions
interface ActionItem {
  key: string;
  icon?: ReactNode;
  label?: string;
  onItemClick?: () => void;
  actionRender?: () => ReactNode;
}
// eslint-disable-next-line react-refresh/only-export-components
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
import { ModelIcon } from "@lobehub/icons";
import { Alert, App, Avatar, Input, Modal, Popconfirm, Spin, Tag, theme, Typography } from "antd";
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
  useUserProfileStore,
} from "@/stores";
import { useStreamStore } from "@/stores";
import type { Message } from "@/types";

import { Tooltip } from "@/components/layout/Tooltip";
import { formatDuration, formatSpeed, formatTokenCount } from "../gateway/tokenFormat";
import { AskUserCard } from "./AskUserCard";
import { AttachmentPreview } from "./AttachmentPreview";
import { AssistantMarkdown, getChatCodeThemes, THINKING_LOADING_MARKER } from "./ChatMarkdownNodes";
import { getStreamingLoadingState, shouldRenderAssistantMarkdownFromContent } from "./chatStreaming";
import { DeleteLastVersionPopover } from "./DeleteLastVersionPopover";
import { ModelSelector } from "./ModelSelector";
import { ModelTags } from "./ModelTags";
import { LayoutSwitcher, MultiModelDisplay, type MultiModelDisplayMode } from "./MultiModelDisplay";
import { PermissionCard } from "./PermissionCard";
import { ToolCallCard } from "./ToolCallCard";
import { buildAssistantDisplayContent, shouldHideAssistantBubble } from "./toolCallDisplay";
import { TopicGroupDivider } from "./TopicGroupDivider";
import { VersionPagination } from "./VersionPagination";

// eslint-disable-next-line react-refresh/only-export-components
function AssistantFooter({
  msg,
  conversationId,
  assistantCopyText,
  getModelDisplayInfo,
  onEditMessage,
  isStreaming = false,
  displayMode,
  onDisplayModeChange,
  onMultiModelDetected,
}: {
  msg: Message;
  conversationId: string;
  assistantCopyText: string;
  getModelDisplayInfo: (
    model_id?: string | null,
    providerId?: string | null,
  ) => { modelName: string; providerName: string };
  onEditMessage: (
    messageId: string,
    content: string,
    role: "user" | "assistant",
  ) => void;
  isStreaming?: boolean;
  displayMode?: MultiModelDisplayMode;
  onDisplayModeChange?: (
    parentMsgId: string,
    mode: MultiModelDisplayMode,
  ) => void;
  onMultiModelDetected?: (parentMsgId: string, versions: Message[]) => void;
  isDarkMode: boolean;
  codeBlockDarkTheme: string;
  codeBlockLightTheme: string;
  codeBlockThemes: string[];
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
    if (msg.parent_message_id && conversationId) {
      let cancelled = false;
      listMessageVersions(conversationId, msg.parent_message_id).then((v) => {
        if (!cancelled && v) {
          setAllVersions(v);
        }
      });
      return () => {
        cancelled = true;
      };
    }
  }, [
    msg.parent_message_id,
    conversationId,
    listMessageVersions,
  ]);

  const mergedVersions = useMemo(() => {
    if (!msg.parent_message_id) {
      return allVersions;
    }
    const dbIds = new Set(allVersions.map((v) => v.id));
    const extra = storeMessages.filter(
      (m) =>
        m.parent_message_id === msg.parent_message_id
        && m.role === "assistant"
        && !dbIds.has(m.id)
        && m.model_id,
    );
    return extra.length > 0 ? [...allVersions, ...extra] : allVersions;
  }, [allVersions, storeMessages, msg.parent_message_id]);

  const hasMultiModels = useMemo(
    () => hasMultipleModelVersions(mergedVersions),
    [mergedVersions],
  );

  useEffect(() => {
    if (msg.parent_message_id && onMultiModelDetected) {
      onMultiModelDetected(msg.parent_message_id, mergedVersions);
    }
  }, [msg.parent_message_id, mergedVersions, onMultiModelDetected]);

  const currentModelOverride = useMemo(() => {
    if (msg.provider_id && msg.model_id) {
      return { providerId: msg.provider_id, model_id: msg.model_id };
    }
    return null;
  }, [msg.provider_id, msg.model_id]);

  const handleModelSelect = useCallback(
    async (providerId: string, model_id: string) => {
      try {
        if (providerId === msg.provider_id && model_id === msg.model_id) {
          await regenerateMessage(msg.id);
        } else {
          await regenerateWithModel(msg.id, providerId, model_id);
        }
      } catch (e) {
        messageApi.error(String(e));
      }
    },
    [
      msg.id,
      msg.provider_id,
      msg.model_id,
      regenerateMessage,
      regenerateWithModel,
      messageApi,
    ],
  );

  const totalTokens = (msg.prompt_tokens ?? 0) + (msg.completion_tokens ?? 0);

  return (
    <div style={{ display: "flex", flexDirection: "column" }}>
      {!isStreaming
        && (msg.prompt_tokens != null
          || msg.completion_tokens != null
          || msg.tokens_per_second != null
          || msg.first_token_latency_ms != null)
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
            {msg.prompt_tokens != null && (
              <span
                style={{ display: "inline-flex", alignItems: "center", gap: 2 }}
              >
                <ArrowDown size={10} />
                <span className="ax-glow-text">
                  {formatTokenCount(msg.prompt_tokens)}
                </span>{" "}
                {t("chat.tokens")}
              </span>
            )}
            {msg.completion_tokens != null && (
              <span
                style={{ display: "inline-flex", alignItems: "center", gap: 2 }}
              >
                <ArrowDown size={10} />
                <span className="ax-glow-text">
                  {formatTokenCount(msg.completion_tokens)}
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
            {msg.tokens_per_second != null && (
              <span
                style={{ display: "inline-flex", alignItems: "center", gap: 2 }}
              >
                <Zap size={10} />
                {formatSpeed(msg.tokens_per_second)}
              </span>
            )}
            {msg.first_token_latency_ms != null && (
              <span
                style={{ display: "inline-flex", alignItems: "center", gap: 2 }}
              >
                <TextCursorInput size={10} />
                {formatDuration(msg.first_token_latency_ms)}
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
              ...(msg.role === "assistant"
                ? [
                  {
                    key: "edit",
                    icon: <Pencil size={14} />,
                    label: t("chat.editMessage"),
                    onItemClick: () => {
                      onEditMessage(msg.id, msg.content, "assistant");
                    },
                  },
                ]
                : []),
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
          && msg.parent_message_id && (
          <LayoutSwitcher
            currentMode={displayMode}
            onModeChange={(mode) => onDisplayModeChange(msg.parent_message_id!, mode)}
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
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const { message: messageApi } = App.useApp();

  const settings = useSettingsStore((s) => s.settings);
  const profile = useUserProfileStore((s) => s.profile);
  const resolvedAvatarSrc = useResolvedAvatarSrc(
    profile.avatarType,
    profile.avatarValue,
  );
  const isDarkMode = useResolvedDarkMode(settings.theme_mode);
  const { copy: copyMessage, isCopiedFor: isUserMsgCopied } = useCopyToClipboard();
  const {
    darkTheme: codeBlockDarkTheme,
    lightTheme: codeBlockLightTheme,
    themes: codeBlockThemes,
  } = useMemo(
    () => getChatCodeThemes(settings.code_theme, settings.code_theme_light),
    [settings.code_theme, settings.code_theme_light],
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
  const getCompressionSummary = useCompressStore(
    (s) => s.getCompressionSummary,
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
  const contentRendererMessageIdsRef = useRef<Set<string>>(new Set());

  useEffect(() => {
    if (!streaming || !streamingMessageId) {
      return;
    }
    contentRendererMessageIdsRef.current.add(streamingMessageId);
  }, [streaming, streamingMessageId]);

  const activeMessages = useMemo(
    () => messages.filter((msg) => msg.is_active !== false),
    [messages],
  );
  const messageById = useMemo(
    () => new Map(messages.map((msg) => [msg.id, msg])),
    [messages],
  );
  const assistantByParentId = useMemo(() => {
    const map = new Map<string, Message>();
    for (const msg of messages) {
      if (
        msg.role === "assistant"
        && msg.parent_message_id
        && msg.is_active !== false
      ) {
        map.set(`ai:${msg.parent_message_id}`, msg);
      }
    }
    return map;
  }, [messages]);

  const multiModelResponseParents = useMemo(() => {
    const modelsByParent = new Map<string, Set<string>>();
    for (const msg of messages) {
      if (msg.role === "assistant" && msg.parent_message_id) {
        if (!modelsByParent.has(msg.parent_message_id)) {
          modelsByParent.set(msg.parent_message_id, new Set());
        }
        modelsByParent
          .get(msg.parent_message_id)!
          .add(msg.model_id || `__no_model_${msg.id}`);
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

  const multiModelVersionsRef = useRef<Map<string, Message[]>>(new Map());
  const prevConvIdRef = useRef<string | undefined>(undefined);
  useEffect(() => {
    if (prevConvIdRef.current !== undefined && prevConvIdRef.current !== activeConversationId) {
      multiModelVersionsRef.current.clear();
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
      const hadCached = multiModelVersionsRef.current.has(parentMsgId);
      const stillMultiModel = hasMultipleModelVersions(versions);
      if (stillMultiModel) {
        multiModelVersionsRef.current.set(parentMsgId, versions);
      } else {
        multiModelVersionsRef.current.delete(parentMsgId);
      }
      if (
        hadCached !== stillMultiModel
        || !multiModelResponseParents.has(parentMsgId)
      ) {
        setDisplayModeOverrides((prev) => new Map(prev));
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

  const bubbleItemCacheRef = useRef<
    Map<string, { signature: string; item: BubbleItemType }>
  >(new Map());
  const bubbleItems: BubbleItemType[] = useMemo(() => {
    const cache = bubbleItemCacheRef.current;
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
        const parentSearch = msg.parent_message_id
          ? userSearchContentById.get(msg.parent_message_id)
          : undefined;
        if (parentSearch?.hasSearch && parentSearch.sources.length > 0) {
          const { sources } = parentSearch;
          const resultsJson = JSON.stringify(
            sources.map((s) => ({ title: s.title, url: s.url })),
          );
          aiContent = `<web-search status="done" data-axagent="1">\n${resultsJson}\n</web-search>\n\n${aiContent}`;
        }
      }

      const stableKey = msg.parent_message_id
        ? `ai:${msg.parent_message_id}`
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

    bubbleItemCacheRef.current = nextCache;
    return nextItems;
  }, [activeMessages, thinkingActiveMessageIds, userSearchContentById]);

  const [expertSwitchBubble, setExpertSwitchBubble] = useState<BubbleItemType | null>(null);
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
      key: `__expert-switch__${sw.roleId}__${Date.now()}`,
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
        const group = msgKeyToGroup.get(key);
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
    return items;
  }, [
    bubbleItems,
    compressing,
    activeConversationId,
    expertSwitchBubble,
    topicGroupEnabledByConv,
    topicGroupsByConv,
  ]);

  const visibleBubbleItems = useMemo(() => {
    return allBubbleItems;
  }, [allBubbleItems]);

  const hiddenEarlierCount = 0;

  const aiContentNodesCacheRef = useRef<
    Map<string, { content: string; nodes: ChatMarkdownNode[] }>
  >(new Map());
  const aiContentNodesById = useMemo(() => {
    const cache = aiContentNodesCacheRef.current;
    const next = new Map<string, ChatMarkdownNode[]>();
    for (const item of bubbleItems) {
      if (item.role !== "ai" || typeof item.content !== "string") {
        continue;
      }
      const msg = assistantByParentId.get(String(item.key))
        ?? messageById.get(String(item.key));
      if (msg?.status === "error") {
        continue;
      }
      const shouldRenderFromContent = shouldRenderAssistantMarkdownFromContent(
        streaming && msg?.id === streamingMessageId,
          Boolean(msg?.id && contentRendererMessageIdsRef.current.has(msg.id)),
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
    assistantByParentId,
    messageById,
    streaming,
    streamingMessageId,
  ]);

  const formatTime = useCallback((ts: number) => {
    const d = new Date(ts);
    return `${String(d.getHours()).padStart(2, "0")}:${String(d.getMinutes()).padStart(2, "0")}`;
  }, []);

  const getModelDisplayInfo = useCallback(
    (model_id?: string | null, providerId?: string | null) => {
      const mid = model_id ?? activeConversation?.model_id;
      const pid = providerId ?? activeConversation?.provider_id;
      if (!mid) {
        return { modelName: "AI", providerName: "" };
      }
      const provider = providers.find((p) => p.id === pid);
      const model = provider?.models.find((m) => m.model_id === mid);
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
          return { variant: "borderless" };
        case "minimal":
          return { variant: "borderless", style: { padding: "4px 8px" } };
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
                {text
                  && (settings.render_user_markdown
                    ? (
                      <AssistantMarkdown
                        content={text}
                        isDarkMode={isDarkMode}
                        isStreaming={false}
                        codeBlockDarkTheme={codeBlockDarkTheme}
                        codeBlockLightTheme={codeBlockLightTheme}
                        codeBlockThemes={codeBlockThemes}
                        codeFontFamily={settings.code_font_family || undefined}
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
                      key={att.id || `${att.file_name}-${i}`}
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
                {settings.render_user_markdown
                  ? (
                    <AssistantMarkdown
                      content={text}
                      isDarkMode={isDarkMode}
                      isStreaming={false}
                      codeBlockDarkTheme={codeBlockDarkTheme}
                      codeBlockLightTheme={codeBlockLightTheme}
                      codeBlockThemes={codeBlockThemes}
                      codeFontFamily={settings.code_font_family || undefined}
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
                <Typography.Text type="secondary" style={{ fontSize: 12 }}>
                  {formatTime(msg.created_at)}
                </Typography.Text>
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
                key: "regenerate",
                icon: <RotateCcw size={14} />,
                label: t("chat.regenerate"),
                onItemClick: async () => {
                  try {
                    await regenerateMessage();
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
      handleEditMessage,
      isDarkMode,
      isUserMsgCopied,
      messageApi,
      messageById,
      profile.name,
      regenerateMessage,
      settings.code_font_family,
      settings.render_user_markdown,
      t,
      token.colorError,
      token.colorPrimary,
      token.colorSuccess,
      userAvatar,
    ],
  );

  const renderConvIconForChat = useCallback(
    (size: number, model_id?: string | null) => {
      if (!activeConversation) {
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
      const mid = model_id ?? activeConversation.model_id;
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
      const msg = assistantByParentId.get(String(bubbleData.key))
        ?? (() => {
          const key = String(bubbleData.key);
          if (key.startsWith("ai:")) {
            const parentId = key.slice(3);
            return (
              messages.find(
                (m) =>
                  m.parent_message_id === parentId
                  && m.role === "assistant"
                  && m.is_active !== false,
              ) ?? messageById.get(key)
            );
          }
          return messageById.get(key);
        })();
      const isStreaming = streaming && msg?.id === streamingMessageId;
      const shouldRenderFromContent = shouldRenderAssistantMarkdownFromContent(
        isStreaming,
        Boolean(msg?.id && contentRendererMessageIdsRef.current.has(msg.id)),
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
      const isMultiModelMsg = !!multiModelParentId && msg?.parent_message_id === multiModelParentId;
      const isAgentMsg = activeConversation?.mode === "agent";
      const bubbleLoading = isMultiModelMsg || isAgentMsg ? false : rawBubbleLoading;

      const parentId = msg?.parent_message_id;
      const hasMultiModels = !!parentId
        && (multiModelResponseParents.has(parentId)
          || multiModelVersionsRef.current.has(parentId));
      const effectiveDisplayMode: MultiModelDisplayMode = hasMultiModels
        ? (displayModeOverrides.get(parentId)
          ?? settings.multi_model_display_mode
          ?? "tabs")
        : "tabs";
      const isNonTabsMultiModel = hasMultiModels && effectiveDisplayMode !== "tabs";

      return {
        placement: "start" as const,
        className: "msg-row assistant",
        ...getBubbleVariant(false),
        avatar: isNonTabsMultiModel
          ? undefined
          : renderConvIconForChat(32, msg?.model_id),
        loading: bubbleLoading,
        contentRender: (content: ReactNode) => {
          const text = content as string;
          const msgMarker = (
            <span
              data-axagent-msg={msg?.id}
              style={{ height: 0, overflow: "hidden", lineHeight: 0 }}
            />
          );
          if (msg?.status === "error") {
            return (
              <>
                {msgMarker}
                <Alert
                  type="error"
                  message={text.length > 200 ? text.slice(0, 200) + "…" : text}
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
                          codeFontFamily={settings.code_font_family || undefined}
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
            const refVersions = multiModelVersionsRef.current.get(parentId);
            const storeVersions = messages.filter(
              (m) => m.parent_message_id === parentId && m.role === "assistant",
            );
            const allVersions = refVersions && refVersions.length > storeVersions.length
              ? refVersions
              : storeVersions;
            return (
              <>
                {msgMarker}
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
                      codeFontFamily={settings.code_font_family || undefined}
                    />
                  )}
                />
              </>
            );
          }

          if (isMultiModelMsg && rawBubbleLoading) {
            return (
              <>
                {msgMarker}
                <span className="axagent-streaming-dots" aria-hidden="true">
                  <span />
                  <span />
                  <span />
                </span>
              </>
            );
          }

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

          if (
            isAgentMsg
            && rawBubbleLoading
            && msgPermissions.length === 0
            && msgAskUsers.length === 0
          ) {
            return (
              <>
                {msgMarker}
                <span className="axagent-streaming-dots" aria-hidden="true">
                  <span />
                  <span />
                  <span />
                </span>
              </>
            );
          }

          return (
            <>
              {msgMarker}
              <AssistantMarkdown
                content={text}
                nodes={parsedNodes}
                isDarkMode={isDarkMode}
                isStreaming={isStreaming}
                codeBlockDarkTheme={codeBlockDarkTheme}
                codeBlockLightTheme={codeBlockLightTheme}
                codeBlockThemes={codeBlockThemes}
                codeFontFamily={settings.code_font_family || undefined}
              />
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
                  conversationId={ask.conversationId}
                  question={ask.question}
                  options={ask.options}
                />
              ))}
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
            </>
          );
        },
        header: (() => {
          if (isNonTabsMultiModel) {
            return null;
          }
          const { modelName, providerName } = getModelDisplayInfo(
            msg?.model_id,
            msg?.provider_id,
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
                  <Typography.Text type="secondary" style={{ fontSize: 12 }}>
                    {formatTime(msg.created_at)}
                  </Typography.Text>
                )}
                {msg?.status === "partial"
                  && !isStreaming
                  && !(
                    multiModelParentId
                    && msg.parent_message_id === multiModelParentId
                  ) && (
                  <Tag
                    color="warning"
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
                onEditMessage={handleEditMessage}
                isStreaming={isStreaming}
                displayMode={effectiveDisplayMode}
                onDisplayModeChange={handleDisplayModeOverride}
                onMultiModelDetected={handleMultiModelDetected}
                isDarkMode={isDarkMode}
                codeBlockDarkTheme={codeBlockDarkTheme}
                codeBlockLightTheme={codeBlockLightTheme}
                codeBlockThemes={codeBlockThemes}
                codeFontFamily={settings.code_font_family || undefined}
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
      assistantByParentId,
      codeBlockDarkTheme,
      codeBlockLightTheme,
      codeBlockThemes,
      deleteMessage,
      displayModeOverrides,
      formatTime,
      getBubbleVariant,
      getModelDisplayInfo,
      handleDisplayModeOverride,
      handleEditMessage,
      handleMultiModelDetected,
      isDarkMode,
      messageById,
      messages,
      multiModelDoneMessageIds,
      multiModelParentId,
      multiModelResponseParents,
      renderConvIconForChat,
      settings,
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
                    summary?.summary_text ?? t("chat.noSummary"),
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

  const roles: RoleType = useMemo(
    () => ({
      user: userRole,
      ai: aiRole,
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
    hiddenEarlierCount,
    lastBubbleKey,
    roles,
    bubbleItems,
    summaryModalOpen,
    setSummaryModalOpen,
    summaryModalText,
    renderConvIconForChat,
  };
}

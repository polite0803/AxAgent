// SPDX-License-Identifier: AGPL-3.0-only

import { App, type InputRef } from "antd";
import type { MenuProps } from "antd";
import { Copy, FileCode, FileImage, FileText, FileType, Globe } from "lucide-react";
import React, { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";

import { translateBackendError } from "@/lib/errorI18n";
import {
  copyTranscript,
  exportAsHTML,
  exportAsJSON,
  exportAsMarkdown,
  exportAsPNG,
  exportAsText,
} from "@/lib/exportChat";
import { invoke, logIpcError } from "@/lib/invoke";
import { useConversationStore, useProviderStore, useSettingsStore, useTopicGroupStore } from "@/stores";
import type { ConversationStats, Message } from "@/types";

export interface UseChatViewActionsParams {
  activeConversationId: string | null;
  activeConversation: import("@/types").Conversation | undefined;
  messages: Message[];
  messageAreaRef: React.RefObject<HTMLDivElement | null>;
}

export interface UseChatViewActionsReturn {
  editingTitle: boolean;
  setEditingTitle: (v: boolean) => void;
  titleDraft: string;
  setTitleDraft: React.Dispatch<React.SetStateAction<string>>;
  titleInputRef: React.RefObject<InputRef | null>;
  isTitleGenerating: boolean;
  handleTitleClick: () => void;
  handleTitleSave: () => void;
  handleRegenerateTitle: () => void;
  editingMessageId: string | null;
  editingMessageRole: "user" | "assistant" | null;
  editingContent: string;
  editSaving: boolean;
  handleEditMessage: (
    messageId: string,
    content: string,
    role: "user" | "assistant",
  ) => void;
  handleEditSaveOnly: () => void;
  handleEditSaveAndResend: () => void;
  resetEditing: () => void;
  setEditingContent: (v: string) => void;
  handlePromptClick: (info: {
    data: { label?: unknown; scenario?: string };
  }) => void;
  handleTopicGroupToggle: () => void;
  handleStatsOpenChange: (open: boolean) => void;
  statsOpen: boolean;
  stats: ConversationStats | null;
  exportMenuItems: MenuProps["items"];
  extractMemoriesOpen: boolean;
  setExtractMemoriesOpen: (v: boolean) => void;
  toolCount: number;
}

export function useChatViewActions({
  activeConversationId,
  activeConversation,
  messages,
  messageAreaRef,
}: UseChatViewActionsParams): UseChatViewActionsReturn {
  const { t } = useTranslation();
  const { message: messageApi } = App.useApp();

  const updateConversation = useConversationStore((s) => s.updateConversation);
  const regenerateTitle = useConversationStore((s) => s.regenerateTitle);
  const regenerateMessage = useConversationStore((s) => s.regenerateMessage);
  const updateMessageContent = useConversationStore(
    (s) => s.updateMessageContent,
  );
  const createConversation = useConversationStore((s) => s.createConversation);
  const titleGeneratingConversationId = useConversationStore(
    (s) => s.titleGeneratingConversationId,
  );
  const providers = useProviderStore((s) => s.providers);
  const providersLoading = useProviderStore((s) => s.loading);
  const settings = useSettingsStore((s) => s.settings);

  const isTitleGenerating = activeConversationId != null
    && titleGeneratingConversationId === activeConversationId;

  const [editingTitle, setEditingTitle] = useState(false);
  const [titleDraft, setTitleDraft] = useState("");
  const titleInputRef = useRef<InputRef>(null);
  const skipTitleSaveRef = useRef(false);

  const [editingMessageId, setEditingMessageId] = useState<string | null>(null);
  const [editingMessageRole, setEditingMessageRole] = useState<
    "user" | "assistant" | null
  >(null);
  const [editingContent, setEditingContent] = useState("");
  const [editSaving, setEditSaving] = useState(false);

  const [extractMemoriesOpen, setExtractMemoriesOpen] = useState(false);

  const [statsOpen, setStatsOpen] = useState(false);
  const [stats, setStats] = useState<ConversationStats | null>(null);

  // 顶部「X 个工具」Tag 的数据源。
  // - 无 profile 时：返回全局已启用工具总数（兼容旧行为）
  // - 有 profile 时：返回按该 profile 工具域筛选后的工具数（与 agent_query 一致）
  // 关键：依赖 agent_profile_id，切换专家后自动刷新显示
  const agentProfileId = activeConversation?.agentProfileId ?? undefined;
  const [toolCount, setToolCount] = useState(0);
  useEffect(() => {
    invoke<number>("get_tool_count", { agentProfileId })
      .then(setToolCount)
      .catch(logIpcError("get_tool_count"));
  }, [agentProfileId]);

  useEffect(() => {
    if (editingTitle && titleInputRef.current) {
      titleInputRef.current.focus();
    }
  }, [editingTitle]);

  const topicGroupEnabled = useTopicGroupStore((s) =>
    activeConversationId
      ? s.enabledByConversation[activeConversationId]
      : false
  );

  const handleTitleClick = useCallback(() => {
    if (!activeConversation) {
      return;
    }
    setTitleDraft(activeConversation.title);
    setEditingTitle(true);
  }, [activeConversation]);

  const handleTitleSave = useCallback(async () => {
    if (skipTitleSaveRef.current) {
      skipTitleSaveRef.current = false;
      return;
    }
    setEditingTitle(false);
    const trimmed = titleDraft.trim();
    if (
      !trimmed
      || !activeConversation
      || trimmed === activeConversation.title
    ) {
      return;
    }
    await updateConversation(activeConversation.id, { title: trimmed });
  }, [titleDraft, activeConversation, updateConversation]);

  const handleRegenerateTitle = useCallback(async () => {
    if (!activeConversation || isTitleGenerating) {
      return;
    }
    skipTitleSaveRef.current = true;
    setEditingTitle(false);
    await regenerateTitle(activeConversation.id);
  }, [activeConversation, isTitleGenerating, regenerateTitle]);

  const handleEditMessage = useCallback(
    (messageId: string, content: string, role: "user" | "assistant") => {
      if (!messageId) {
        setEditingMessageId(null);
        setEditingMessageRole(null);
        setEditingContent("");
        return;
      }
      setEditingMessageId(messageId);
      setEditingMessageRole(role);
      setEditingContent(content);
    },
    [],
  );

  const resetEditing = useCallback(() => {
    setEditingMessageId(null);
    setEditingMessageRole(null);
    setEditingContent("");
  }, []);

  const handleEditSaveOnly = useCallback(async () => {
    if (!editingMessageId) {
      return;
    }
    setEditSaving(true);
    try {
      await updateMessageContent(editingMessageId, editingContent);
      setEditingMessageId(null);
      setEditingMessageRole(null);
      setEditingContent("");
    } catch (e) {
      messageApi.error(translateBackendError(e));
    } finally {
      setEditSaving(false);
    }
  }, [editingMessageId, editingContent, updateMessageContent, messageApi]);

  const handleEditSaveAndResend = useCallback(async () => {
    if (!editingMessageId) {
      return;
    }
    setEditSaving(true);
    try {
      await updateMessageContent(editingMessageId, editingContent);
      const msgs = useConversationStore.getState().messages;
      const aiMsg = msgs.find(
        (m) => m.parentMessageId === editingMessageId && m.isActive,
      );
      setEditingMessageId(null);
      setEditingMessageRole(null);
      setEditingContent("");
      // Fix: when no active AI message exists (e.g. all versions were deleted),
      // fall back to any AI response for the edited user message, then to
      // undefined (which triggers regenerateMessage's built-in last-user fallback).
      const targetMsgId = aiMsg?.id
        ?? msgs.find(
          (m) => m.parentMessageId === editingMessageId && m.role === "assistant",
        )?.id;
      await regenerateMessage(targetMsgId);
    } catch (e) {
      messageApi.error(translateBackendError(e));
    } finally {
      setEditSaving(false);
    }
  }, [
    editingMessageId,
    editingContent,
    updateMessageContent,
    regenerateMessage,
    messageApi,
  ]);

  const handlePromptClick = useCallback(
    async (info: { data: { label?: unknown; scenario?: string } }) => {
      const label = info.data.label;
      const text = typeof label === "string" ? label : "";
      const scenario = info.data.scenario;
      if (!text) {
        return;
      }

      try {
        if (!activeConversationId) {
          if (providersLoading || providers.length === 0) {
            messageApi.warning(t("chat.noModel"));
            return;
          }
          let provider = settings.defaultModel
            ? providers.find(
              (p) => p.id === settings.defaultModel?.a && p.enabled,
            )
            : undefined;
          let model = provider?.models.find(
            (m) => m.modelId === settings.defaultModel?.b && m.enabled,
          );
          if (!provider || !model) {
            provider = providers.find(
              (p) => p.enabled && p.models.some((m) => m.enabled),
            );
            model = provider?.models.find((m) => m.enabled);
          }
          if (!provider || !model) {
            messageApi.warning(t("chat.noModel"));
            return;
          }
          await createConversation(
            text.slice(0, 30),
            model.modelId,
            provider.id,
            { scenario },
          );
        }

        useConversationStore.getState().setPendingPromptText(text);
      } catch (e) {
        messageApi.error(translateBackendError(e));
      }
    },
    [
      activeConversationId,
      providers,
      providersLoading,
      settings,
      createConversation,
      messageApi,
      t,
    ],
  );

  const handleTopicGroupToggle = useCallback(() => {
    if (!activeConversationId) {
      return;
    }
    const enabled = !topicGroupEnabled;
    useTopicGroupStore.getState().setEnabled(activeConversationId, enabled);
    if (enabled) {
      useTopicGroupStore.getState().autoDetect(activeConversationId);
    }
  }, [activeConversationId, topicGroupEnabled]);

  const handleStatsOpenChange = useCallback(
    async (open: boolean) => {
      setStatsOpen(open);
      if (open && activeConversationId) {
        setStats(null);
        try {
          const data = await invoke<ConversationStats>(
            "get_conversation_stats",
            {
              conversationId: activeConversationId,
            },
            5_000,
          );
          setStats(data);
        } catch {
          setStats(null);
        }
      }
    },
    [activeConversationId],
  );

  const exportMenuItems = useMemo<MenuProps["items"]>(
    () => {
      const runFormatExport = async (
        exporter: (
          msgs: Message[],
          title: string,
          opts?: { includeThinking?: boolean },
        ) => Promise<boolean>,
        includeThinking: boolean,
      ) => {
        if (messages.length === 0) {
          messageApi.warning(t("chat.noMessages"));
          return;
        }
        try {
          const ok = await exporter(
            messages,
            activeConversation?.title ?? "chat",
            includeThinking ? undefined : { includeThinking: false },
          );
          if (ok) {
            messageApi.success(t("chat.exportSuccess"));
          }
        } catch {
          messageApi.error(t("chat.exportFailed"));
        }
      };

      const FORMAT_TARGETS: Array<{
        key: string;
        noThinkingKey: string;
        labelKey: string;
        noThinkingLabelKey: string;
        icon: React.ReactNode;
        exporter: (
          msgs: Message[],
          title: string,
          opts?: { includeThinking?: boolean },
        ) => Promise<boolean>;
      }> = [
        {
          key: "md",
          noThinkingKey: "export-md-no-thinking",
          labelKey: "chat.exportMd",
          noThinkingLabelKey: "chat.exportMdNoThinking",
          icon: <FileCode size={14} />,
          exporter: exportAsMarkdown,
        },
        {
          key: "txt",
          noThinkingKey: "export-txt-no-thinking",
          labelKey: "chat.exportTxt",
          noThinkingLabelKey: "chat.exportTxtNoThinking",
          icon: <FileType size={14} />,
          exporter: exportAsText,
        },
        {
          key: "json",
          noThinkingKey: "export-json-no-thinking",
          labelKey: "chat.exportJson",
          noThinkingLabelKey: "chat.exportJsonNoThinking",
          icon: <FileText size={14} />,
          exporter: exportAsJSON,
        },
        {
          key: "html",
          noThinkingKey: "export-html-no-thinking",
          labelKey: "chat.exportHtml",
          noThinkingLabelKey: "chat.exportHtmlNoThinking",
          icon: <Globe size={14} />,
          exporter: exportAsHTML,
        },
      ];

      // 完整的导出菜单：复制 + PNG + 各格式（含/不含思考过程）
      return [
        {
          key: "copy-md",
          label: t("chat.copyMarkdown"),
          icon: <Copy size={14} />,
          onClick: async () => {
            if (messages.length === 0) {
              messageApi.warning(t("chat.noMessages"));
              return;
            }
            try {
              const ok = await copyTranscript(
                messages,
                activeConversation?.title ?? "chat",
                "markdown",
                {
                  includeThinking: false,
                },
              );
              if (ok) {
                messageApi.success(t("chat.copied"));
              }
            } catch {
              messageApi.error(t("chat.exportFailed"));
            }
          },
        },
        {
          key: "png",
          label: t("chat.exportPng"),
          icon: <FileImage size={14} />,
          onClick: async () => {
            try {
              const ok = await exportAsPNG(
                messageAreaRef.current,
                activeConversation?.title ?? "chat",
              );
              if (ok) {
                messageApi.success(t("chat.exportSuccess"));
              }
            } catch {
              messageApi.error(t("chat.exportFailed"));
            }
          },
        },
        ...FORMAT_TARGETS.flatMap((target) => [
          {
            key: target.key,
            label: t(target.labelKey),
            icon: target.icon,
            onClick: () => runFormatExport(target.exporter, true),
          },
          {
            key: target.noThinkingKey,
            label: t(target.noThinkingLabelKey),
            icon: target.icon,
            onClick: () => runFormatExport(target.exporter, false),
          },
        ]),
      ];
    },
    [messages, activeConversation, t, messageApi], // eslint-disable-line react-hooks/exhaustive-deps
  );

  return {
    editingTitle,
    setEditingTitle,
    titleDraft,
    setTitleDraft,
    titleInputRef,
    isTitleGenerating,
    handleTitleClick,
    handleTitleSave,
    handleRegenerateTitle,
    editingMessageId,
    editingMessageRole,
    editingContent,
    editSaving,
    handleEditMessage,
    handleEditSaveOnly,
    handleEditSaveAndResend,
    resetEditing,
    setEditingContent,
    handlePromptClick,
    handleTopicGroupToggle,
    handleStatsOpenChange,
    statsOpen,
    stats,
    exportMenuItems,
    extractMemoriesOpen,
    setExtractMemoriesOpen,
    toolCount,
  };
}

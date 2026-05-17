import { App, type InputRef } from "antd";
import { Copy, FileCode, FileImage, FileText, FileType, Globe } from "lucide-react";
import React, { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";

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

import { CHAT_SCROLL_IS_REVERSED, getScrollTopAfterPrepend } from "./chatScroll";

export interface UseChatViewActionsParams {
  activeConversationId: string | null;
  activeConversation: import("@/types").Conversation | undefined;
  messages: Message[];
  bubbleListRef: React.RefObject<any | null>;
  messageAreaRef: React.RefObject<HTMLDivElement | null>;
  loadOlderMessages: () => Promise<void>;
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
  handleEditMessage: (messageId: string, content: string, role: "user" | "assistant") => void;
  handleEditSaveOnly: () => void;
  handleEditSaveAndResend: () => void;
  resetEditing: () => void;
  setEditingContent: (v: string) => void;
  handleLoadOlderMessages: () => Promise<void>;
  handlePromptClick: (info: { data: { label?: unknown; scenario?: string } }) => void;
  handleTopicGroupToggle: () => void;
  handleStatsOpenChange: (open: boolean) => void;
  statsOpen: boolean;
  stats: ConversationStats | null;
  exportMenuItems: Array<Record<string, unknown>>;
  extractMemoriesOpen: boolean;
  setExtractMemoriesOpen: (v: boolean) => void;
  toolCount: number;
  expertOpen: boolean;
  setExpertOpen: (v: boolean) => void;
}

export function useChatViewActions({
  activeConversationId,
  activeConversation,
  messages,
  bubbleListRef,
  messageAreaRef,
  loadOlderMessages,
}: UseChatViewActionsParams): UseChatViewActionsReturn {
  const { t } = useTranslation();
  const { message: messageApi } = App.useApp();

  const updateConversation = useConversationStore((s) => s.updateConversation);
  const regenerateTitle = useConversationStore((s) => s.regenerateTitle);
  const regenerateMessage = useConversationStore((s) => s.regenerateMessage);
  const updateMessageContent = useConversationStore((s) => s.updateMessageContent);
  const createConversation = useConversationStore((s) => s.createConversation);
  const titleGeneratingConversationId = useConversationStore((s) => s.titleGeneratingConversationId);
  const providers = useProviderStore((s) => s.providers);
  const providersLoading = useProviderStore((s) => s.loading);
  const settings = useSettingsStore((s) => s.settings);

  const isTitleGenerating = activeConversationId != null && titleGeneratingConversationId === activeConversationId;

  const [editingTitle, setEditingTitle] = useState(false);
  const [titleDraft, setTitleDraft] = useState("");
  const titleInputRef = useRef<InputRef>(null);
  const skipTitleSaveRef = useRef(false);

  const [editingMessageId, setEditingMessageId] = useState<string | null>(null);
  const [editingMessageRole, setEditingMessageRole] = useState<"user" | "assistant" | null>(null);
  const [editingContent, setEditingContent] = useState("");
  const [editSaving, setEditSaving] = useState(false);

  const [extractMemoriesOpen, setExtractMemoriesOpen] = useState(false);
  const [expertOpen, setExpertOpen] = useState(false);

  const [statsOpen, setStatsOpen] = useState(false);
  const [stats, setStats] = useState<ConversationStats | null>(null);

  const [toolCount, setToolCount] = useState(0);
  useEffect(() => {
    invoke<number>("get_tool_count").then(setToolCount).catch(logIpcError("get_tool_count"));
  }, []);

  useEffect(() => {
    if (editingTitle && titleInputRef.current) {
      titleInputRef.current.focus();
    }
  }, [editingTitle]);

  const topicGroupEnabled = useTopicGroupStore((s) =>
    activeConversationId ? s.enabledByConversation[activeConversationId] : false
  );

  const handleTitleClick = useCallback(() => {
    if (!activeConversation) { return; }
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
    if (!trimmed || !activeConversation || trimmed === activeConversation.title) { return; }
    await updateConversation(activeConversation.id, { title: trimmed });
  }, [titleDraft, activeConversation, updateConversation]);

  const handleRegenerateTitle = useCallback(async () => {
    if (!activeConversation || isTitleGenerating) { return; }
    skipTitleSaveRef.current = true;
    setEditingTitle(false);
    await regenerateTitle(activeConversation.id);
  }, [activeConversation, isTitleGenerating, regenerateTitle]);

  const handleLoadOlderMessages = useCallback(async () => {
    const scrollContainer = bubbleListRef.current?.scrollBoxNativeElement as HTMLDivElement | null | undefined;
    const previousScrollHeight = scrollContainer?.scrollHeight ?? 0;
    const previousScrollTop = scrollContainer?.scrollTop ?? 0;
    await loadOlderMessages();
    window.requestAnimationFrame(() => {
      window.requestAnimationFrame(() => {
        if (!scrollContainer) { return; }
        scrollContainer.scrollTop = getScrollTopAfterPrepend(
          previousScrollTop,
          previousScrollHeight,
          scrollContainer.scrollHeight,
          CHAT_SCROLL_IS_REVERSED,
        );
      });
    });
  }, [loadOlderMessages]);

  const handleEditMessage = useCallback((messageId: string, content: string, role: "user" | "assistant") => {
    if (!messageId) {
      setEditingMessageId(null);
      setEditingMessageRole(null);
      setEditingContent("");
      return;
    }
    setEditingMessageId(messageId);
    setEditingMessageRole(role);
    setEditingContent(content);
  }, []);

  const resetEditing = useCallback(() => {
    setEditingMessageId(null);
    setEditingMessageRole(null);
    setEditingContent("");
  }, []);

  const handleEditSaveOnly = useCallback(async () => {
    if (!editingMessageId) { return; }
    setEditSaving(true);
    try {
      await updateMessageContent(editingMessageId, editingContent);
      setEditingMessageId(null);
      setEditingMessageRole(null);
      setEditingContent("");
    } catch (e) {
      messageApi.error(String(e));
    } finally {
      setEditSaving(false);
    }
  }, [editingMessageId, editingContent, updateMessageContent, messageApi]);

  const handleEditSaveAndResend = useCallback(async () => {
    if (!editingMessageId) { return; }
    setEditSaving(true);
    try {
      await updateMessageContent(editingMessageId, editingContent);
      const msgs = useConversationStore.getState().messages;
      const aiMsg = msgs.find(m => m.parent_message_id === editingMessageId && m.is_active);
      setEditingMessageId(null);
      setEditingMessageRole(null);
      setEditingContent("");
      await regenerateMessage(aiMsg?.id);
    } catch (e) {
      messageApi.error(String(e));
    } finally {
      setEditSaving(false);
    }
  }, [editingMessageId, editingContent, updateMessageContent, regenerateMessage, messageApi]);

  const handlePromptClick = useCallback(
    async (info: { data: { label?: unknown; scenario?: string } }) => {
      const label = info.data.label;
      const text = typeof label === "string" ? label : "";
      const scenario = info.data.scenario;
      if (!text) { return; }

      try {
        if (!activeConversationId) {
          if (providersLoading || providers.length === 0) {
            messageApi.warning(t("chat.noModel"));
            return;
          }
          let provider = settings.default_provider_id
            ? providers.find((p) => p.id === settings.default_provider_id && p.enabled)
            : undefined;
          let model = provider?.models.find(
            (m) => m.model_id === settings.default_model_id && m.enabled,
          );
          if (!provider || !model) {
            provider = providers.find((p) => p.enabled && p.models.some((m) => m.enabled));
            model = provider?.models.find((m) => m.enabled);
          }
          if (!provider || !model) {
            messageApi.warning(t("chat.noModel"));
            return;
          }
          await createConversation(text.slice(0, 30), model.model_id, provider.id, { scenario });
        }

        useConversationStore.getState().setPendingPromptText(text);
      } catch (e) {
        console.error("[handlePromptClick] error:", e);
        messageApi.error(String(e));
      }
    },
    [activeConversationId, providers, providersLoading, settings, createConversation, messageApi, t],
  );

  const handleTopicGroupToggle = useCallback(() => {
    if (!activeConversationId) { return; }
    const enabled = !topicGroupEnabled;
    useTopicGroupStore.getState().setEnabled(activeConversationId, enabled);
    if (enabled) {
      useTopicGroupStore.getState().autoDetect(activeConversationId);
    }
  }, [activeConversationId, topicGroupEnabled]);

  const handleStatsOpenChange = useCallback(async (open: boolean) => {
    setStatsOpen(open);
    if (open && activeConversationId) {
      setStats(null);
      try {
        const data = await invoke<ConversationStats>("get_conversation_stats", {
          conversationId: activeConversationId,
        }, 5_000);
        setStats(data);
      } catch {
        setStats(null);
      }
    }
  }, [activeConversationId]);

  const exportMenuItems = useMemo(
    () => [
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
            const ok = await copyTranscript(messages, activeConversation?.title ?? "chat", "markdown", {
              includeThinking: false,
            });
            if (ok) { messageApi.success(t("chat.copied")); }
          } catch (e) {
            console.error("Copy MD failed:", e);
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
            const ok = await exportAsPNG(messageAreaRef.current, activeConversation?.title ?? "chat");
            if (ok) { messageApi.success(t("chat.exportSuccess")); }
          } catch (e) {
            console.error("Export PNG failed:", e);
            messageApi.error(t("chat.exportFailed"));
          }
        },
      },
      {
        key: "md",
        label: t("chat.exportMd"),
        icon: <FileCode size={14} />,
        onClick: async () => {
          if (messages.length === 0) {
            messageApi.warning(t("chat.noMessages"));
            return;
          }
          try {
            const ok = await exportAsMarkdown(messages, activeConversation?.title ?? "chat");
            if (ok) { messageApi.success(t("chat.exportSuccess")); }
          } catch (e) {
            console.error("Export MD failed:", e);
            messageApi.error(t("chat.exportFailed"));
          }
        },
      },
      {
        key: "export-md-no-thinking",
        label: t("chat.exportMdNoThinking"),
        icon: <FileCode size={14} />,
        onClick: async () => {
          if (messages.length === 0) {
            messageApi.warning(t("chat.noMessages"));
            return;
          }
          try {
            const ok = await exportAsMarkdown(messages, activeConversation?.title ?? "chat", {
              includeThinking: false,
            });
            if (ok) { messageApi.success(t("chat.exportSuccess")); }
          } catch (e) {
            console.error("Export MD (no thinking) failed:", e);
            messageApi.error(t("chat.exportFailed"));
          }
        },
      },
      {
        key: "txt",
        label: t("chat.exportTxt"),
        icon: <FileType size={14} />,
        onClick: async () => {
          if (messages.length === 0) {
            messageApi.warning(t("chat.noMessages"));
            return;
          }
          try {
            const ok = await exportAsText(messages, activeConversation?.title ?? "chat");
            if (ok) { messageApi.success(t("chat.exportSuccess")); }
          } catch (e) {
            console.error("Export TXT failed:", e);
            messageApi.error(t("chat.exportFailed"));
          }
        },
      },
      {
        key: "export-txt-no-thinking",
        label: t("chat.exportTxtNoThinking"),
        icon: <FileType size={14} />,
        onClick: async () => {
          if (messages.length === 0) {
            messageApi.warning(t("chat.noMessages"));
            return;
          }
          try {
            const ok = await exportAsText(messages, activeConversation?.title ?? "chat", { includeThinking: false });
            if (ok) { messageApi.success(t("chat.exportSuccess")); }
          } catch (e) {
            console.error("Export TXT (no thinking) failed:", e);
            messageApi.error(t("chat.exportFailed"));
          }
        },
      },
      {
        key: "json",
        label: t("chat.exportJson"),
        icon: <FileText size={14} />,
        onClick: async () => {
          if (messages.length === 0) {
            messageApi.warning(t("chat.noMessages"));
            return;
          }
          try {
            const ok = await exportAsJSON(messages, activeConversation?.title ?? "chat");
            if (ok) { messageApi.success(t("chat.exportSuccess")); }
          } catch (e) {
            console.error("Export JSON failed:", e);
            messageApi.error(t("chat.exportFailed"));
          }
        },
      },
      {
        key: "export-json-no-thinking",
        label: t("chat.exportJsonNoThinking"),
        icon: <FileText size={14} />,
        onClick: async () => {
          if (messages.length === 0) {
            messageApi.warning(t("chat.noMessages"));
            return;
          }
          try {
            const ok = await exportAsJSON(messages, activeConversation?.title ?? "chat", { includeThinking: false });
            if (ok) { messageApi.success(t("chat.exportSuccess")); }
          } catch (e) {
            console.error("Export JSON (no thinking) failed:", e);
            messageApi.error(t("chat.exportFailed"));
          }
        },
      },
      {
        key: "html",
        label: t("chat.exportHtml"),
        icon: <Globe size={14} />,
        onClick: async () => {
          if (messages.length === 0) {
            messageApi.warning(t("chat.noMessages"));
            return;
          }
          try {
            const ok = await exportAsHTML(messages, activeConversation?.title ?? "chat");
            if (ok) { messageApi.success(t("chat.exportSuccess")); }
          } catch (e) {
            console.error("Export HTML failed:", e);
            messageApi.error(t("chat.exportFailed"));
          }
        },
      },
      {
        key: "export-html-no-thinking",
        label: t("chat.exportHtmlNoThinking"),
        icon: <Globe size={14} />,
        onClick: async () => {
          if (messages.length === 0) {
            messageApi.warning(t("chat.noMessages"));
            return;
          }
          try {
            const ok = await exportAsHTML(messages, activeConversation?.title ?? "chat", { includeThinking: false });
            if (ok) { messageApi.success(t("chat.exportSuccess")); }
          } catch (e) {
            console.error("Export HTML (no thinking) failed:", e);
            messageApi.error(t("chat.exportFailed"));
          }
        },
      },
    ],
    [messages, activeConversation, t, messageApi],
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
    handleLoadOlderMessages,
    handlePromptClick,
    handleTopicGroupToggle,
    handleStatsOpenChange,
    statsOpen,
    stats,
    exportMenuItems,
    extractMemoriesOpen,
    setExtractMemoriesOpen,
    toolCount,
    expertOpen,
    setExpertOpen,
  };
}

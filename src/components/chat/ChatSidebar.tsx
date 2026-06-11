// SPDX-License-Identifier: AGPL-3.0-only

import { SessionSearchPanel } from "@/components/search/SessionSearchPanel";
import { useDebounce } from "@/hooks/useDebounce";
import { getConvIcon } from "@/lib/convIcon";
import {
  copyTranscript,
  exportAsHTML,
  exportAsJSON,
  exportAsMarkdown,
  exportAsPNG,
  exportAsText,
} from "@/lib/exportChat";
import { invoke, logIpcError } from "@/lib/invoke";
import { formatShortcutForDisplay, getShortcutBinding } from "@/lib/shortcuts";
import type { ShortcutAction } from "@/lib/shortcuts";
import {
  useConversationStore,
  useKnowledgeStore,
  useProviderStore,
  useSettingsStore,
  useStreamStore,
  useUIStore,
  useWorkflowEditorStore,
} from "@/stores";
import { isSidebarAutoSelectSuppressed, resetSidebarAutoSelectSuppression } from "@/stores/domain/conversationStore";
import type { Conversation, Message } from "@/types";
import type { ConversationItemType } from "@ant-design/x/es/conversations/interface";
import { ModelIcon } from "@lobehub/icons";
import {
  App,
  Avatar,
  Button,
  Checkbox,
  Dropdown,
  Empty,
  Input,
  type MenuProps,
  Modal,
  Radio,
  Space,
  theme,
  Tooltip,
} from "antd";
import {
  Archive,
  ArrowLeft,
  Bot,
  ChevronRight,
  Copy,
  FileCode,
  FileImage,
  FileText,
  FileType,
  FolderOpen,
  GitBranch,
  GitFork,
  Link2,
  ListTodo,
  Loader,
  MessageSquarePlus,
  PanelLeftClose,
  PanelLeftOpen,
  Pencil,
  Pin,
  PinOff,
  Search,
  Share,
  Sparkles,
  Trash2,
  Undo2,
  X,
} from "lucide-react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { CategoryManagerModal } from "./CategoryManagerModal";

function getDateGroup(timestamp: number): string {
  const now = new Date();
  const date = new Date(timestamp * 1000);

  const startOfToday = new Date(
    now.getFullYear(),
    now.getMonth(),
    now.getDate(),
  );
  const startOfYesterday = new Date(startOfToday.getTime() - 86400000);
  const dayOfWeek = startOfToday.getDay();
  const startOfWeek = new Date(startOfToday.getTime() - dayOfWeek * 86400000);
  const startOfMonth = new Date(now.getFullYear(), now.getMonth(), 1);

  if (date >= startOfToday) {
    return "today";
  }
  if (date >= startOfYesterday) {
    return "yesterday";
  }
  if (date >= startOfWeek) {
    return "thisWeek";
  }
  if (date >= startOfMonth) {
    return "thisMonth";
  }
  return "earlier";
}

export function ChatSidebar({
  onCollapseChange,
}: {
  onCollapseChange?: (collapsed: boolean) => void;
}) {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const { message: messageApi, modal } = App.useApp();

  // Reset sidebar auto-select suppression on mount to prevent stale flags
  // from surviving page navigation (e.g. delete → navigate to settings → back).
  useEffect(() => {
    resetSidebarAutoSelectSuppression();
  }, []);

  const conversations = useConversationStore((s) => s.conversations);
  const activeConversationId = useConversationStore(
    (s) => s.activeConversationId,
  );
  const setActiveConversation = useConversationStore(
    (s) => s.setActiveConversation,
  );
  const createConversation = useConversationStore((s) => s.createConversation);
  const deleteConversation = useConversationStore((s) => s.deleteConversation);
  const updateConversation = useConversationStore((s) => s.updateConversation);
  const regenerateTitle = useConversationStore((s) => s.regenerateTitle);
  const titleGeneratingConversationId = useConversationStore(
    (s) => s.titleGeneratingConversationId,
  );
  const forkConversation = useConversationStore((s) => s.forkConversation);
  const togglePin = useConversationStore((s) => s.togglePin);
  const toggleArchive = useConversationStore((s) => s.toggleArchive);
  const archiveToKnowledgeBase = useConversationStore(
    (s) => s.archiveToKnowledgeBase,
  );
  const archivedConversations = useConversationStore(
    (s) => s.archivedConversations,
  );
  const fetchArchivedConversations = useConversationStore(
    (s) => s.fetchArchivedConversations,
  );
  const batchDelete = useConversationStore((s) => s.batchDelete);
  const batchArchive = useConversationStore((s) => s.batchArchive);
  const knowledgeBases = useKnowledgeStore((s) => s.bases);
  const loadKnowledgeBases = useKnowledgeStore((s) => s.loadBases);
  const activeStreams = useStreamStore((s) => s.activeStreams);
  const loadConversationWorkflowPreview = useWorkflowEditorStore(
    (s) => s.loadConversationWorkflowPreview,
  );
  const openWorkflowEditor = useUIStore((s) => s.openWorkflowEditor);

  const providers = useProviderStore((s) => s.providers);
  const settings = useSettingsStore((s) => s.settings);
  const settingsLoading = useSettingsStore((s) => s.loading);

  const shortcutHint = useCallback(
    (label: string, action: ShortcutAction) => {
      if (!settings) {
        return label;
      }
      const binding = getShortcutBinding(settings, action);
      return `${label} (${formatShortcutForDisplay(binding)})`;
    },
    [settings],
  );

  const [searchText, setSearchText] = useState("");
  const debouncedSearch = useDebounce(searchText.trim(), 300);
  const [fts5ResultIds, setFts5ResultIds] = useState<string[] | null>(null);

  useEffect(() => {
    // 以下分支互斥：无搜索词直接返回 null，异步搜索后 then/catch 各自只有一次 setState
    if (!debouncedSearch) {
      // eslint-disable-next-line react-hooks/set-state-in-effect
      setFts5ResultIds(null);
      return;
    }
    let cancelled = false;
    invoke<Array<{ id: string }>>("search_conversations", {
      query: debouncedSearch,
    })
      .then((results) => {
        if (!cancelled) {
          setFts5ResultIds(results.map((r) => r.id));
        }
      })
      .catch(() => {
        if (!cancelled) {
          setFts5ResultIds(null);
        }
      });
    return () => {
      cancelled = true;
    };
  }, [debouncedSearch]);
  const [searchVisible, setSearchVisible] = useState(false);
  const [advancedSearchVisible, setAdvancedSearchVisible] = useState(false);
  const [multiSelectMode, setMultiSelectMode] = useState(false);
  const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set());
  const [showArchived, setShowArchived] = useState(false);
  const [archivedSelectedIds, setArchivedSelectedIds] = useState<Set<string>>(
    new Set(),
  );
  const [archivedMultiSelect, setArchivedMultiSelect] = useState(false);
  const [rightClickedConvId, setRightClickedConvId] = useState<string | null>(
    null,
  );
  const [expandedParentIds, setExpandedParentIds] = useState<Set<string>>(
    new Set(),
  );
  const [isCollapsed, setIsCollapsed] = useState(false);
  const [archiveKbModalOpen, setArchiveKbModalOpen] = useState(false);
  const [archiveTargetId, setArchiveTargetId] = useState<string | null>(null);
  const [archiveTargetIds, setArchiveTargetIds] = useState<string[]>([]);
  const [selectedKbId, setSelectedKbId] = useState<string | null>(null);
  const [archiveLoading, setArchiveLoading] = useState(false);
  const [categoryManagerOpen, setCategoryManagerOpen] = useState(false);
  const conversationsLoading = useConversationStore((s) => s.loading);

  // Auto-expand parent when active conversation is a child
  useEffect(() => {
    if (!activeConversationId) {
      return;
    }
    const active = conversations.find((c) => c.id === activeConversationId);
    if (
      active?.parent_conversation_id
      && !expandedParentIds.has(active.parent_conversation_id)
    ) {
      // eslint-disable-next-line react-hooks/set-state-in-effect
      setExpandedParentIds((prev) => new Set(prev).add(active.parent_conversation_id!));
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [activeConversationId, conversations]);

  // Auto-select conversation: restore last selected, or fall back to first
  useEffect(() => {
    // Suppress auto-select when the active conversation was just deleted/archived.
    // The user explicitly closed the conversation and should see the welcome screen.
    if (isSidebarAutoSelectSuppressed()) {
      resetSidebarAutoSelectSuppression();
      return;
    }
    if (!activeConversationId && conversations.length > 0 && !settingsLoading) {
      const lastId = settings.last_selected_conversation_id;
      const lastConv = lastId
        ? conversations.find((c) => c.id === lastId)
        : null;
      if (lastConv) {
        setActiveConversation(lastConv.id);
      } else {
        const sorted = conversations.toSorted((a, b) => {
          if (a.is_pinned !== b.is_pinned) {
            return a.is_pinned ? -1 : 1;
          }
          return b.updated_at - a.updated_at;
        });
        setActiveConversation(sorted[0].id);
      }
    }
  }, [
    activeConversationId,
    conversations,
    setActiveConversation,
    settings.last_selected_conversation_id,
    settingsLoading,
  ]);

  // Persist last selected conversation
  useEffect(() => {
    if (
      activeConversationId
      && activeConversationId !== settings.last_selected_conversation_id
    ) {
      void useSettingsStore
        .getState()
        .saveSettings({ last_selected_conversation_id: activeConversationId });
    }
  }, [activeConversationId, settings.last_selected_conversation_id]);

  const handleNewConversation = useCallback(async () => {
    let provider: (typeof providers)[0] | undefined;
    let model: (typeof providers)[0]["models"][0] | undefined;

    if (settings.default_provider_id && settings.default_model_id) {
      provider = providers.find(
        (p) => p.id === settings.default_provider_id && p.enabled,
      );
      model = provider?.models.find(
        (m) => m.model_id === settings.default_model_id && m.enabled,
      );
    }

    if (!provider || !model) {
      const activeConv = conversations.find(
        (c) => c.id === activeConversationId,
      );
      if (activeConv?.provider_id && activeConv?.model_id) {
        provider = providers.find(
          (p) => p.id === activeConv.provider_id && p.enabled,
        );
        model = provider?.models.find(
          (m) => m.model_id === activeConv.model_id && m.enabled,
        );
      }
    }

    if (!provider || !model) {
      provider = providers.find(
        (p) => p.enabled && p.models.some((m) => m.enabled),
      );
      model = provider?.models.find((m) => m.enabled);
    }

    if (!provider || !model) {
      messageApi.warning(t("chat.noModelsAvailable"));
      return;
    }

    await createConversation(
      t("chat.newConversation"),
      model.model_id,
      provider.id,
    );
  }, [
    providers,
    settings,
    conversations,
    activeConversationId,
    createConversation,
    messageApi,
    t,
  ]);

  useEffect(() => {
    const onShortcutNewConversation = () => {
      void handleNewConversation();
    };
    window.addEventListener(
      "axagent:new-conversation",
      onShortcutNewConversation,
    );
    return () => {
      window.removeEventListener(
        "axagent:new-conversation",
        onShortcutNewConversation,
      );
    };
  }, [handleNewConversation]);

  const handleSearch = useCallback((value: string) => {
    setSearchText(value);
  }, []);

  const filteredConversations = useMemo(() => {
    let filtered = conversations;
    if (searchText.trim()) {
      if (fts5ResultIds !== null) {
        const idSet = new Set(fts5ResultIds);
        filtered = filtered.filter((c: Conversation) => idSet.has(c.id));
      } else {
        const query = searchText.toLowerCase();
        filtered = filtered.filter((c: Conversation) => c.title.toLowerCase().includes(query));
      }
    }
    filtered.sort((a, b) => {
      if (a.is_pinned !== b.is_pinned) {
        return a.is_pinned ? -1 : 1;
      }
      return b.updated_at - a.updated_at;
    });
    return filtered;
  }, [conversations, searchText, fts5ResultIds]);

  const toggleSelect = useCallback((id: string) => {
    setSelectedIds((prev) => {
      const next = new Set(prev);
      if (next.has(id)) {
        next.delete(id);
      } else {
        next.add(id);
      }
      return next;
    });
  }, []);

  const exitMultiSelect = useCallback(() => {
    setMultiSelectMode(false);
    setSelectedIds(new Set());
  }, []);

  // Ctrl+A / Escape keyboard shortcuts for multi-select mode
  useEffect(() => {
    if (!multiSelectMode) {
      return;
    }
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        exitMultiSelect();
        return;
      }
      if ((e.ctrlKey || e.metaKey) && e.key === "a") {
        e.preventDefault();
        const targets = showArchived ? archivedConversations : conversations;
        const currentIds = showArchived ? archivedSelectedIds : selectedIds;
        const allSelected = targets.every((c) => currentIds.has(c.id));
        if (showArchived) {
          setArchivedSelectedIds(
            allSelected ? new Set() : new Set(targets.map((c) => c.id)),
          );
        } else {
          setSelectedIds(
            allSelected ? new Set() : new Set(targets.map((c) => c.id)),
          );
        }
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [
    multiSelectMode,
    showArchived,
    conversations,
    archivedConversations,
    selectedIds,
    archivedSelectedIds,
    exitMultiSelect,
  ]);

  const isAllSelected = useMemo(
    () =>
      filteredConversations.length > 0
      && selectedIds.size === filteredConversations.length,
    [filteredConversations, selectedIds],
  );

  const handleSelectAll = useCallback(() => {
    if (isAllSelected) {
      setSelectedIds(new Set());
    } else {
      setSelectedIds(new Set(filteredConversations.map((c) => c.id)));
    }
  }, [isAllSelected, filteredConversations]);

  const isAllArchivedSelected = useMemo(
    () =>
      archivedConversations.length > 0
      && archivedSelectedIds.size === archivedConversations.length,
    [archivedConversations, archivedSelectedIds],
  );

  const handleSelectAllArchived = useCallback(() => {
    if (isAllArchivedSelected) {
      setArchivedSelectedIds(new Set());
    } else {
      setArchivedSelectedIds(new Set(archivedConversations.map((c) => c.id)));
    }
  }, [isAllArchivedSelected, archivedConversations]);

  const handleBatchDelete = useCallback(async () => {
    const ids = Array.from(selectedIds);
    if (ids.length === 0) {
      return;
    }
    modal.confirm({
      title: t("chat.deleteConfirm"),
      content: t("chat.batchDeleteContent", { count: ids.length }),
      mask: { enabled: true, blur: true },
      okButtonProps: { danger: true },
      onOk: async () => {
        await batchDelete(ids);
        exitMultiSelect();
      },
    });
  }, [selectedIds, batchDelete, exitMultiSelect, modal, t]);

  const handleArchiveSingle = useCallback(
    async (convId: string) => {
      // Open knowledge base selector for single archive
      setArchiveTargetId(convId);
      setArchiveTargetIds([]);
      setSelectedKbId(null);
      await loadKnowledgeBases();
      setArchiveKbModalOpen(true);
    },
    [loadKnowledgeBases],
  );

  const handleSaveAsWorkflow = useCallback(
    async (convId: string) => {
      try {
        await loadConversationWorkflowPreview(convId);
        openWorkflowEditor();
      } catch (e) {
        const errMsg = String(e);
        if (errMsg.includes("WORKFLOW_NO_SKILL_EXECUTIONS")) {
          messageApi.warning(t("chat.noSkillExecutions"));
        } else {
          messageApi.error(errMsg);
        }
      }
    },
    [loadConversationWorkflowPreview, openWorkflowEditor, messageApi, t],
  );

  const handleArchiveConfirm = useCallback(async () => {
    if (!selectedKbId) {
      return;
    }
    setArchiveLoading(true);
    try {
      if (archiveTargetId) {
        // Single archive
        await archiveToKnowledgeBase(archiveTargetId, selectedKbId);
      } else if (archiveTargetIds.length > 0) {
        // Batch archive — run in parallel
        await Promise.all(
          archiveTargetIds.map((id) => archiveToKnowledgeBase(id, selectedKbId)),
        );
        exitMultiSelect();
      }
      messageApi.success(
        t("chat.archivedSuccess", {
          count: archiveTargetId ? 1 : archiveTargetIds.length,
        }),
      );
      setArchiveKbModalOpen(false);
      setArchiveTargetId(null);
      setArchiveTargetIds([]);
      setSelectedKbId(null);
    } catch (e) {
      messageApi.error(String(e));
    } finally {
      setArchiveLoading(false);
    }
  }, [
    selectedKbId,
    archiveTargetId,
    archiveTargetIds,
    archiveToKnowledgeBase,
    exitMultiSelect,
    messageApi,
    t,
  ]);

  const handleShowArchived = useCallback(async () => {
    await fetchArchivedConversations();
    setShowArchived(true);
    setArchivedMultiSelect(false);
    setArchivedSelectedIds(new Set());
  }, [fetchArchivedConversations]);

  const handleBackFromArchived = useCallback(() => {
    setShowArchived(false);
    setArchivedMultiSelect(false);
    setArchivedSelectedIds(new Set());
  }, []);

  const toggleArchivedSelect = useCallback((id: string) => {
    setArchivedSelectedIds((prev) => {
      const next = new Set(prev);
      if (next.has(id)) {
        next.delete(id);
      } else {
        next.add(id);
      }
      return next;
    });
  }, []);

  const handleBatchUnarchive = useCallback(async () => {
    const ids = Array.from(archivedSelectedIds);
    if (ids.length === 0) {
      return;
    }
    await Promise.all(ids.map((id) => toggleArchive(id)));
    await fetchArchivedConversations();
    setArchivedSelectedIds(new Set());
    setArchivedMultiSelect(false);
  }, [archivedSelectedIds, toggleArchive, fetchArchivedConversations]);

  const handleBatchDeleteArchived = useCallback(async () => {
    const ids = Array.from(archivedSelectedIds);
    if (ids.length === 0) {
      return;
    }
    modal.confirm({
      title: t("chat.deleteConfirm"),
      content: t("chat.batchDeleteContent", { count: ids.length }),
      mask: { enabled: true, blur: true },
      okButtonProps: { danger: true },
      onOk: async () => {
        await batchDelete(ids);
        await fetchArchivedConversations();
        setArchivedSelectedIds(new Set());
        setArchivedMultiSelect(false);
      },
    });
  }, [archivedSelectedIds, batchDelete, fetchArchivedConversations, modal, t]);

  const buildIcon = useCallback(
    (conv: Conversation) => {
      const isStreaming = conv.id in activeStreams;
      const customIcon = getConvIcon(conv.id);
      let icon: React.ReactNode;
      if (customIcon) {
        if (customIcon.type === "emoji") {
          icon = (
            <Avatar
              size={20}
              style={{ fontSize: 12, backgroundColor: token.colorPrimaryBg }}
            >
              {customIcon.value}
            </Avatar>
          );
        } else {
          icon = <Avatar size={20} src={customIcon.value} />;
        }
      } else if (conv.mode === "agent") {
        icon = (
          <Avatar
            size={20}
            icon={<Bot size={12} />}
            style={{
              backgroundColor: token.colorPrimaryBg,
              color: token.colorPrimary,
            }}
          />
        );
      } else if (conv.mode === "gateway") {
        icon = (
          <Avatar
            size={20}
            icon={<Link2 size={12} />}
            style={{ backgroundColor: token.colorPrimaryBg, color: token.colorPrimary }}
          />
        );
      } else if (conv.model_id) {
        icon = <ModelIcon model={conv.model_id} size={20} type="avatar" />;
      } else {
        icon = (
          <Avatar
            size={20}
            style={{
              fontSize: 12,
              backgroundColor: token.colorPrimaryBg,
              color: token.colorPrimary,
            }}
          >
            {(conv.title || t("chat.sidebar.fallbackTitle"))[0]}
          </Avatar>
        );
      }
      if (isStreaming) {
        icon = (
          <span style={{ position: "relative", display: "inline-flex" }}>
            {icon}
            <Loader
              size={10}
              style={{
                position: "absolute",
                bottom: -3,
                right: -3,
                color: token.colorPrimary,
                background: token.colorBgContainer,
                borderRadius: "50%",
                animation: "spin 1s linear infinite",
              }}
            />
          </span>
        );
      }
      return icon;
    },
    [
      activeStreams,
      token.colorPrimary,
      token.colorPrimaryBg,
      token.colorBgContainer,
      t,
    ],
  );

  const conversationItems: ConversationItemType[] = useMemo(() => {
    const items: ConversationItemType[] = [];

    // Build parent→children map (supports arbitrary depth)
    const childrenMap = new Map<string, Conversation[]>();
    const topLevel: Conversation[] = [];
    filteredConversations.forEach((conv) => {
      if (conv.parent_conversation_id) {
        const arr = childrenMap.get(conv.parent_conversation_id) ?? [];
        arr.push(conv);
        childrenMap.set(conv.parent_conversation_id, arr);
      } else {
        topLevel.push(conv);
      }
    });

    // Group conversations by workspace_dir
    const convsByWorkspaceDir = new Map<string, Conversation[]>();
    const uncategorizedConvs: Conversation[] = [];
    topLevel.forEach((conv) => {
      if (conv.workspace_dir) {
        const arr = convsByWorkspaceDir.get(conv.workspace_dir) ?? [];
        arr.push(conv);
        convsByWorkspaceDir.set(conv.workspace_dir, arr);
      } else {
        uncategorizedConvs.push(conv);
      }
    });

    const hasChildren = (convId: string) => (childrenMap.get(convId)?.length ?? 0) > 0;
    const isExpanded = (convId: string) => expandedParentIds.has(convId);

    const buildConvItem = (
      conv: Conversation,
      group: string,
      depth = 0,
    ): ConversationItemType => {
      const icon = buildIcon(conv);
      const childCount = childrenMap.get(conv.id)?.length ?? 0;
      const expanded = isExpanded(conv.id);

      const isTitleGen = titleGeneratingConversationId === conv.id;
      let label: React.ReactNode;
      if (conv.is_pinned && depth === 0) {
        label = (
          <span className="flex items-center gap-1">
            <span className="truncate">{conv.title}</span>
            {isTitleGen && (
              <Loader
                size={10}
                style={{
                  flexShrink: 0,
                  animation: "spin 1s linear infinite",
                  color: token.colorTextQuaternary,
                }}
              />
            )}
            <Pin
              size={12}
              style={{ color: token.colorTextQuaternary, flexShrink: 0 }}
            />
          </span>
        );
      } else {
        label = (
          <span className="flex items-center gap-1">
            <span className="truncate">{conv.title}</span>
            {isTitleGen && (
              <Loader
                size={10}
                style={{
                  flexShrink: 0,
                  animation: "spin 1s linear infinite",
                  color: token.colorTextQuaternary,
                }}
              />
            )}
          </span>
        );
      }

      // Wrap label with expand/collapse toggle for parents with children
      if (childCount > 0) {
        label = (
          <span
            className="flex items-center gap-1"
            style={{ overflow: "hidden" }}
          >
            <span
              onClick={(e) => {
                e.stopPropagation();
                setExpandedParentIds((prev) => {
                  const next = new Set(prev);
                  if (next.has(conv.id)) {
                    next.delete(conv.id);
                  } else {
                    next.add(conv.id);
                  }
                  return next;
                });
              }}
              role="button"
              tabIndex={0}
              onKeyDown={(e) => {
                if (e.key === "Enter" || e.key === " ") {
                  e.preventDefault();
                  e.stopPropagation();
                  setExpandedParentIds((prev) => {
                    const next = new Set(prev);
                    if (next.has(conv.id)) {
                      next.delete(conv.id);
                    } else {
                      next.add(conv.id);
                    }
                    return next;
                  });
                }
              }}
              style={{
                cursor: "pointer",
                display: "flex",
                alignItems: "center",
                flexShrink: 0,
              }}
            >
              <ChevronRight
                size={12}
                style={{
                  color: token.colorTextQuaternary,
                  transition: "transform 0.2s",
                  transform: expanded ? "rotate(90deg)" : "rotate(0deg)",
                }}
              />
            </span>
            <span className="truncate">{conv.title}</span>
          </span>
        );
      }

      const indent = depth > 0 ? { paddingLeft: 20 + (depth - 1) * 16 } : {};

      if (multiSelectMode) {
        return {
          key: conv.id,
          label,
          icon: (
            <span className="flex items-center gap-1.5">
              <Checkbox
                checked={selectedIds.has(conv.id)}
                onChange={() => toggleSelect(conv.id)}
                onClick={(e: React.MouseEvent) => e.stopPropagation()}
              />
              {icon}
            </span>
          ),
          group,
          "data-conv-id": conv.id,
          ...(depth > 0 ? { style: indent } : {}),
        };
      }
      return {
        key: conv.id,
        label,
        icon,
        group,
        "data-conv-id": conv.id,
        ...(depth > 0 ? { style: indent } : {}),
      };
    };

    // Helper: recursively push a conversation and its descendants
    const pushConvWithChildren = (
      conv: Conversation,
      group: string,
      depth = 0,
    ) => {
      items.push(buildConvItem(conv, group, depth));
      if (hasChildren(conv.id) && isExpanded(conv.id)) {
        const children = childrenMap.get(conv.id)!;
        children.forEach((child) => pushConvWithChildren(child, group, depth + 1));
      }
    };

    // Add workspace directory groups (sorted alphabetically)
    Array.from(convsByWorkspaceDir.keys())
      .sort()
      .forEach((wsDir) => {
        const wsConvs = convsByWorkspaceDir.get(wsDir)!;
        wsConvs.forEach((conv) => pushConvWithChildren(conv, `ws:${wsDir}`));
      });

    // Add uncategorized conversations (no workspace_dir) by date groups
    uncategorizedConvs.forEach((conv) => {
      const group = conv.is_pinned ? "pinned" : getDateGroup(conv.updated_at);
      pushConvWithChildren(conv, group);
    });

    return items;
  }, [
    filteredConversations,
    multiSelectMode,
    selectedIds,
    buildIcon,
    toggleSelect,
    token.colorTextQuaternary,
    titleGeneratingConversationId,
    expandedParentIds,
  ]);

  const groupLabels: Record<string, string> = useMemo(() => {
    const labels: Record<string, string> = {
      pinned: t("chat.pinned"),
      today: t("chat.today"),
      yesterday: t("chat.yesterday"),
      thisWeek: t("chat.thisWeek"),
      thisMonth: t("chat.thisMonth"),
      earlier: t("chat.earlier"),
    };
    return labels;
  }, [t]);

  // Local state for expanded group keys (drives the UI immediately)
  const [expandedKeys, setExpandedKeys] = useState<string[]>([]);

  // Auto-expand workspace groups on first load
  const wsAutoExpandDoneRef = useRef(false);
  useEffect(() => {
    if (wsAutoExpandDoneRef.current || conversationItems.length === 0) {
      return;
    }
    const wsKeys = new Set<string>();
    conversationItems.forEach((item) => {
      if (item.group?.startsWith("ws:")) {
        wsKeys.add(item.group);
      }
    });
    if (wsKeys.size > 0) {
      // eslint-disable-next-line react-hooks/set-state-in-effect
      setExpandedKeys(Array.from(wsKeys));
      wsAutoExpandDoneRef.current = true;
    }
  }, [conversationItems]);

  const handleGroupExpand = useCallback((keys: string[]) => {
    setExpandedKeys(keys);
  }, []);

  const abbreviateWsPath = useCallback(
    (path: string): string => {
      const segments = path.replace(/\\/g, "/").split("/").filter(Boolean);
      if (segments.length <= 1) {
        return path;
      }
      if (segments.length === 2) {
        return segments.join("/");
      }
      // Show last 2 segments, but if duplicate exists among all ws dirs, extend to 3
      const short2 = segments.slice(-2).join("/");
      const wsPaths = Array.from(
        new Set(
          conversations.flatMap((c) => {
            const dir = c.workspace_dir;
            return dir ? [dir] : [];
          }),
        ),
      );
      const hasConflict = wsPaths.some((p) => {
        const s = p.replace(/\\/g, "/").split("/").filter(Boolean);
        return s.length >= 2 && s.slice(-2).join("/") === short2 && p !== path;
      });
      if (hasConflict && segments.length >= 3) {
        return "…/" + segments.slice(-3).join("/");
      }
      return "…/" + short2;
    },
    [conversations],
  );

  // Count conversations per workspace for group badge
  const wsCountMap = useMemo(() => {
    const map = new Map<string, number>();
    conversations.forEach((c) => {
      if (c.workspace_dir && !c.parent_conversation_id) {
        map.set(c.workspace_dir, (map.get(c.workspace_dir) ?? 0) + 1);
      }
    });
    return map;
  }, [conversations]);

  const renderGroupLabel = useCallback(
    (group: string) => {
      if (group.startsWith("ws:")) {
        const wsPath = group.slice(3);
        return (
          <Tooltip title={wsPath}>
            <span
              style={{
                display: "inline-flex",
                alignItems: "center",
                justifyContent: "space-between",
                width: "100%",
                padding: "2px 6px 2px 0",
                borderRadius: 4,
                background: token.colorFillTertiary,
                border: `1px solid ${token.colorBorderSecondary}`,
              }}
            >
              <span
                style={{
                  fontSize: 12,
                  fontWeight: 600,
                  color: token.colorText,
                  display: "inline-flex",
                  alignItems: "center",
                  gap: 5,
                  userSelect: "none",
                  minWidth: 0,
                  flex: 1,
                }}
              >
                <FolderOpen
                  size={13}
                  style={{ flexShrink: 0, color: token.colorPrimary }}
                />
                <span className="truncate">{abbreviateWsPath(wsPath)}</span>
                <span
                  style={{
                    fontSize: 12,
                    fontWeight: 400,
                    color: token.colorTextQuaternary,
                    flexShrink: 0,
                    marginLeft: 2,
                  }}
                >
                  ({wsCountMap.get(wsPath) ?? 0})
                </span>
              </span>
              <Tooltip title={t("chat.newConversation")}>
                <span
                  onClick={(e) => {
                    e.stopPropagation();
                    void handleNewConversation();
                  }}
                  role="button"
                  tabIndex={0}
                  onKeyDown={(e) => {
                    if (e.key === "Enter" || e.key === " ") {
                      e.preventDefault();
                      e.stopPropagation();
                      void handleNewConversation();
                    }
                  }}
                  style={{
                    cursor: "pointer",
                    display: "inline-flex",
                    alignItems: "center",
                    padding: "1px 4px",
                    borderRadius: 3,
                    flexShrink: 0,
                  }}
                  onMouseEnter={(e) => {
                    e.currentTarget.style.background = token.colorFillSecondary;
                  }}
                  onMouseLeave={(e) => {
                    e.currentTarget.style.background = "transparent";
                  }}
                >
                  <MessageSquarePlus
                    size={14}
                    style={{ color: token.colorTextSecondary }}
                  />
                </span>
              </Tooltip>
            </span>
          </Tooltip>
        );
      }
      return groupLabels[group] ?? group;
    },
    [groupLabels, token, abbreviateWsPath, handleNewConversation, wsCountMap, t],
  );

  const handleRename = useCallback(
    (item: ConversationItemType) => {
      const conv = conversations.find((c) => c.id === String(item.key));
      const title = conv?.title ?? "";
      let newTitle = title;
      modal.confirm({
        title: t("chat.rename"),
        mask: { enabled: true, blur: true },
        content: (
          <Input
            id="chat-sidebar-input-4"
            defaultValue={title}
            onChange={(e) => {
              newTitle = e.target.value;
            }}
          />
        ),
        onOk: async () => {
          if (newTitle.trim()) {
            await updateConversation(String(item.key), {
              title: newTitle.trim(),
            });
          }
        },
      });
    },
    [updateConversation, t, modal, conversations],
  );

  const handleDelete = useCallback(
    (item: ConversationItemType) => {
      modal.confirm({
        title: t("chat.deleteConfirm"),
        mask: { enabled: true, blur: true },
        okButtonProps: { danger: true },
        onOk: () => deleteConversation(String(item.key)),
      });
    },
    [deleteConversation, t, modal],
  );

  const buildExportChildren = useCallback(
    (convId: string, title: string) => [
      {
        key: "export-png",
        label: t("chat.exportPng"),
        icon: <FileImage size={14} />,
        onClick: async () => {
          try {
            const el = document.querySelector(
              "[data-message-area]",
            ) as HTMLElement;
            if (!el) {
              messageApi.warning(t("chat.noMessages"));
              return;
            }
            const ok = await exportAsPNG(el, title);
            if (ok) {
              messageApi.success(t("chat.exportSuccess"));
            }
          } catch (e) {
            logIpcError("Export PNG")(e);
            messageApi.error(t("chat.exportFailed"));
          }
        },
      },
      {
        key: "export-md",
        label: t("chat.exportMd"),
        icon: <FileCode size={14} />,
        onClick: async () => {
          try {
            const msgs = await invoke<Message[]>("list_messages", {
              conversationId: convId,
            });
            if (msgs.length === 0) {
              messageApi.warning(t("chat.noMessages"));
              return;
            }
            const ok = await exportAsMarkdown(msgs, title);
            if (ok) {
              messageApi.success(t("chat.exportSuccess"));
            }
          } catch (e) {
            logIpcError("Export MD")(e);
            messageApi.error(t("chat.exportFailed"));
          }
        },
      },
      {
        key: "export-txt",
        label: t("chat.exportTxt"),
        icon: <FileType size={14} />,
        onClick: async () => {
          try {
            const msgs = await invoke<Message[]>("list_messages", {
              conversationId: convId,
            });
            if (msgs.length === 0) {
              messageApi.warning(t("chat.noMessages"));
              return;
            }
            const ok = await exportAsText(msgs, title);
            if (ok) {
              messageApi.success(t("chat.exportSuccess"));
            }
          } catch (e) {
            logIpcError("Export TXT")(e);
            messageApi.error(t("chat.exportFailed"));
          }
        },
      },
      {
        key: "export-json",
        label: t("chat.exportJson"),
        icon: <FileText size={14} />,
        onClick: async () => {
          try {
            const msgs = await invoke<Message[]>("list_messages", {
              conversationId: convId,
            });
            if (msgs.length === 0) {
              messageApi.warning(t("chat.noMessages"));
              return;
            }
            const ok = await exportAsJSON(msgs, title);
            if (ok) {
              messageApi.success(t("chat.exportSuccess"));
            }
          } catch (e) {
            logIpcError("Export JSON")(e);
            messageApi.error(t("chat.exportFailed"));
          }
        },
      },
      {
        key: "export-html",
        label: t("chat.exportHtml"),
        icon: <FileCode size={14} />,
        onClick: async () => {
          try {
            const msgs = await invoke<Message[]>("list_messages", {
              conversationId: convId,
            });
            if (msgs.length === 0) {
              messageApi.warning(t("chat.noMessages"));
              return;
            }
            const ok = await exportAsHTML(msgs, title);
            if (ok) {
              messageApi.success(t("chat.exportSuccess"));
            }
          } catch (e) {
            logIpcError("Export HTML")(e);
            messageApi.error(t("chat.exportFailed"));
          }
        },
      },
    ],
    [t, messageApi],
  );

  // Compute distinct workspace directories for "move to workspace" menu
  const wsDirs = useMemo(() => {
    const dirs = new Set<string>();
    conversations.forEach((c) => {
      if (c.workspace_dir) {
        dirs.add(c.workspace_dir);
      }
    });
    return Array.from(dirs).sort();
  }, [conversations]);

  const menuConfig = useCallback(
    (item: ConversationItemType) => {
      if (multiSelectMode) {
        return { items: [] };
      }
      const conv = conversations.find((c) => c.id === String(item.key));
      if (!conv) {
        return { items: [] };
      }
      const isPinned = conv.is_pinned ?? false;
      const title = conv.title ?? "";
      const hasParent = !!conv.parent_conversation_id;
      const parentId = conv.parent_conversation_id;

      // Build "move to workspace" submenu
      const wsChildren = wsDirs.flatMap((d) =>
        d !== conv.workspace_dir
          ? [
            {
              key: `move-ws:${d}`,
              label: (
                <span
                  className="truncate"
                  style={{ maxWidth: 180, display: "inline-block" }}
                >
                  {d}
                </span>
              ),
            },
          ]
          : []
      );
      if (conv.workspace_dir) {
        wsChildren.unshift({
          key: "remove-ws",
          label: (
            <span style={{ fontStyle: "italic", opacity: 0.6 }}>
              {t("chat.removeFromWorkspace")}
            </span>
          ),
        });
      }
      const wsItems: MenuProps["items"] = wsChildren.length > 0
        ? [
          {
            key: "move-workspace",
            label: (
              <span
                style={{
                  display: "inline-flex",
                  alignItems: "center",
                  gap: 8,
                }}
              >
                <FolderOpen size={14} />
                {t("chat.moveToWorkspace")}
              </span>
            ),
            children: wsChildren.slice(0, 15),
          },
        ]
        : [];

      return {
        items: [
          {
            key: "pin",
            label: isPinned ? t("chat.unpin") : t("chat.pin"),
            icon: isPinned ? <PinOff size={14} /> : <Pin size={14} />,
          },
          {
            key: "archive",
            label: t("chat.archive"),
            icon: <Archive size={14} />,
          },
          {
            key: "ai-title",
            label: t("chat.aiGenerateTitle"),
            icon: <Sparkles size={14} />,
          },
          {
            key: "fork",
            label: t("chat.forkConversation"),
            icon: <GitFork size={14} />,
          },
          {
            key: "copy-id",
            label: t("chat.copyConversationId"),
            icon: <Copy size={14} />,
          },
          {
            key: "copy-transcript",
            label: (
              <span
                style={{ display: "inline-flex", alignItems: "center", gap: 8 }}
              >
                <Copy size={14} />
                {t("chat.copyTranscript")}
              </span>
            ),
            children: [
              {
                key: "copy-md",
                label: "Markdown",
                icon: <FileCode size={14} />,
              },
              {
                key: "copy-txt",
                label: t("chat.exportTxt"),
                icon: <FileType size={14} />,
              },
            ],
          },
          ...wsItems,
          ...(hasParent
            ? [
              {
                key: "detach-parent",
                label: t("chat.detachFromParent"),
                icon: <Link2 size={14} style={{ transform: "rotate(45deg)" }} />,
              },
            ]
            : []),
          ...(hasParent
            ? [
              {
                key: "go-parent",
                label: t("chat.goToParent"),
                icon: (
                  <ChevronRight
                    size={14}
                    style={{ transform: "rotate(180deg)" }}
                  />
                ),
              },
            ]
            : []),
          {
            key: "rename",
            label: t("chat.rename"),
            icon: <Pencil size={14} />,
          },
          {
            key: "export",
            label: (
              <span
                style={{ display: "inline-flex", alignItems: "center", gap: 8 }}
              >
                <Share size={14} />
                {t("chat.export")}
              </span>
            ),
            children: buildExportChildren(conv.id, title),
          },
          {
            key: "delete",
            label: t("chat.delete"),
            icon: <Trash2 size={14} />,
            danger: true,
          },
        ],
        onClick: (menuInfo: { key: string }) => {
          if (menuInfo.key.startsWith("move-ws:")) {
            void invoke("agent_update_session", {
              request: {
                conversationId: conv.id,
                cwd: menuInfo.key.slice("move-ws:".length),
              },
            });
            return;
          }
          if (menuInfo.key === "remove-ws") {
            void invoke("agent_update_session", {
              request: {
                conversationId: conv.id,
                cwd: null,
              },
            });
            return;
          }
          if (menuInfo.key === "fork") {
            void forkConversation(conv.id);
            return;
          }
          if (menuInfo.key === "copy-id") {
            void navigator.clipboard
              .writeText(conv.id)
              .then(() => messageApi.success(t("chat.copied")));
            return;
          }
          if (menuInfo.key === "copy-md" || menuInfo.key === "copy-txt") {
            (async () => {
              try {
                const msgs = await invoke<Message[]>("list_messages", {
                  conversationId: conv.id,
                });
                if (msgs.length === 0) {
                  messageApi.warning(t("chat.noMessages"));
                  return;
                }
                const format = menuInfo.key === "copy-md" ? "markdown" : "text";
                await copyTranscript(
                  msgs,
                  title,
                  format as "markdown" | "text",
                );
                messageApi.success(t("chat.copied"));
              } catch {
                messageApi.error(t("chat.copyFailed"));
              }
            })();
            return;
          }
          if (menuInfo.key === "detach-parent") {
            void updateConversation(conv.id, { parent_conversation_id: null });
            return;
          }
          if (menuInfo.key === "go-parent" && parentId) {
            setActiveConversation(parentId);
            return;
          }
          switch (menuInfo.key) {
            case "pin":
              togglePin(conv.id);
              break;
            case "archive":
              void handleArchiveSingle(conv.id);
              break;
            case "ai-title":
              void regenerateTitle(conv.id);
              break;
            case "rename":
              handleRename(item);
              break;
            case "delete":
              handleDelete(item);
              break;
          }
        },
      };
    },
    [
      t,
      conversations,
      wsDirs,
      multiSelectMode,
      regenerateTitle,
      forkConversation,
      updateConversation,
      handleRename,
      handleDelete,
      togglePin,
      handleArchiveSingle,
      buildExportChildren,
      setActiveConversation,
      messageApi,
    ],
  );

  const handleConversationClick = useCallback(
    (key: string) => {
      if (multiSelectMode) {
        toggleSelect(key);
      } else {
        setActiveConversation(key);
      }
    },
    [multiSelectMode, toggleSelect, setActiveConversation],
  );

  const rightClickMenuConfig = useMemo(() => {
    if (!rightClickedConvId) {
      return { items: [] as MenuProps["items"] };
    }
    const conv = conversations.find((c) => c.id === rightClickedConvId);
    if (!conv) {
      return { items: [] as MenuProps["items"] };
    }
    const isPinned = conv.is_pinned ?? false;
    const title = conv.title ?? "";
    const hasParent = !!conv.parent_conversation_id;
    const parentId = conv.parent_conversation_id;

    const wsChildren = wsDirs.flatMap((d) =>
      d !== conv.workspace_dir
        ? [
          {
            key: `move-ws:${d}`,
            label: (
              <span
                className="truncate"
                style={{ maxWidth: 180, display: "inline-block" }}
              >
                {d}
              </span>
            ),
          },
        ]
        : []
    );
    if (conv.workspace_dir) {
      wsChildren.unshift({
        key: "remove-ws",
        label: (
          <span style={{ fontStyle: "italic", opacity: 0.6 }}>
            {t("chat.removeFromWorkspace")}
          </span>
        ),
      });
    }
    const wsItems: MenuProps["items"] = wsChildren.length > 0
      ? [
        {
          key: "move-workspace",
          label: (
            <span
              style={{
                display: "inline-flex",
                alignItems: "center",
                gap: 8,
              }}
            >
              <FolderOpen size={14} />
              {t("chat.moveToWorkspace")}
            </span>
          ),
          children: wsChildren.slice(0, 15),
        },
      ]
      : [];

    const item = { key: conv.id, label: title } as ConversationItemType;
    return {
      items: [
        {
          key: "pin",
          label: isPinned ? t("chat.unpin") : t("chat.pin"),
          icon: isPinned ? <PinOff size={14} /> : <Pin size={14} />,
        },
        {
          key: "archive",
          label: t("chat.archive"),
          icon: <Archive size={14} />,
        },
        {
          key: "ai-title",
          label: t("chat.aiGenerateTitle"),
          icon: <Sparkles size={14} />,
        },
        {
          key: "fork",
          label: t("chat.forkConversation"),
          icon: <GitFork size={14} />,
        },
        {
          key: "copy-id",
          label: t("chat.copyConversationId"),
          icon: <Copy size={14} />,
        },
        {
          key: "copy-transcript",
          label: (
            <span
              style={{ display: "inline-flex", alignItems: "center", gap: 8 }}
            >
              <Copy size={14} />
              {t("chat.copyTranscript")}
            </span>
          ),
          children: [
            { key: "copy-md", label: "Markdown", icon: <FileCode size={14} /> },
            {
              key: "copy-txt",
              label: t("chat.exportTxt"),
              icon: <FileType size={14} />,
            },
          ],
        },
        ...wsItems,
        ...(hasParent
          ? [
            {
              key: "detach-parent",
              label: t("chat.detachFromParent"),
              icon: <Link2 size={14} style={{ transform: "rotate(45deg)" }} />,
            },
          ]
          : []),
        ...(hasParent
          ? [
            {
              key: "go-parent",
              label: t("chat.goToParent"),
              icon: (
                <ChevronRight
                  size={14}
                  style={{ transform: "rotate(180deg)" }}
                />
              ),
            },
          ]
          : []),
        { key: "rename", label: t("chat.rename"), icon: <Pencil size={14} /> },
        {
          key: "export",
          label: (
            <span
              style={{ display: "inline-flex", alignItems: "center", gap: 8 }}
            >
              <Share size={14} />
              {t("chat.export")}
            </span>
          ),
          children: buildExportChildren(conv.id, title),
        },
        {
          key: "delete",
          label: t("chat.delete"),
          icon: <Trash2 size={14} />,
          danger: true,
        },
      ],
      onClick: (menuInfo: { key: string }) => {
        if (menuInfo.key.startsWith("move-ws:")) {
          void invoke("agent_update_session", {
            request: {
              conversationId: conv.id,
              cwd: menuInfo.key.slice("move-ws:".length),
            },
          });
          return;
        }
        if (menuInfo.key === "remove-ws") {
          void invoke("agent_update_session", {
            request: {
              conversationId: conv.id,
              cwd: null,
            },
          });
          return;
        }
        if (menuInfo.key === "fork") {
          void forkConversation(conv.id);
          return;
        }
        if (menuInfo.key === "copy-id") {
          void navigator.clipboard
            .writeText(conv.id)
            .then(() => messageApi.success(t("chat.copied")));
          return;
        }
        if (menuInfo.key === "copy-md" || menuInfo.key === "copy-txt") {
          (async () => {
            try {
              const msgs = await invoke<Message[]>("list_messages", {
                conversationId: conv.id,
              });
              if (msgs.length === 0) {
                messageApi.warning(t("chat.noMessages"));
                return;
              }
              const format = menuInfo.key === "copy-md" ? "markdown" : "text";
              await copyTranscript(msgs, title, format as "markdown" | "text");
              messageApi.success(t("chat.copied"));
            } catch {
              messageApi.error(t("chat.copyFailed"));
            }
          })();
          return;
        }
        if (menuInfo.key === "detach-parent") {
          void updateConversation(conv.id, { parent_conversation_id: null });
          return;
        }
        if (menuInfo.key === "go-parent" && parentId) {
          setActiveConversation(parentId);
          return;
        }
        switch (menuInfo.key) {
          case "pin":
            togglePin(conv.id);
            break;
          case "archive":
            void handleArchiveSingle(conv.id);
            break;
          case "ai-title":
            void regenerateTitle(conv.id);
            break;
          case "rename":
            handleRename(item);
            break;
          case "delete":
            handleDelete(item);
            break;
        }
      },
    };
  }, [
    rightClickedConvId,
    conversations,
    wsDirs,
    t,
    regenerateTitle,
    forkConversation,
    updateConversation,
    togglePin,
    handleArchiveSingle,
    handleRename,
    handleDelete,
    buildExportChildren,
    setActiveConversation,
    messageApi,
  ]);

  if (isCollapsed) {
    return (
      <div
        className="flex flex-col items-center h-full"
        style={{
          width: "48px",
          paddingTop: 8,
        }}
      >
        <Tooltip title={t("common.expand")} placement="right">
          <Button
            type="text"
            icon={<PanelLeftOpen size={16} />}
            size="small"
            onClick={() => {
              setIsCollapsed(false);
              onCollapseChange?.(false);
            }}
          />
        </Tooltip>
      </div>
    );
  }

  return (
    <div
      className="flex flex-col h-full transition-all duration-200"
      data-tutorial="chat-sidebar"
    >
      {/* Toolbar */}
      <div
        className="flex items-center justify-between"
        style={{
          padding: "8px 12px",
          borderBottom: "1px solid var(--border-color)",
        }}
      >
        <div className="flex items-center gap-1" style={{ flex: 1, minWidth: 0, flexWrap: "wrap" }}>
          {showArchived
            ? (
              archivedMultiSelect
                ? (
                  <>
                    <Tooltip title={t("common.cancel")}>
                      <Button
                        type="text"
                        icon={<X size={16} />}
                        size="small"
                        onClick={() => {
                          setArchivedMultiSelect(false);
                          setArchivedSelectedIds(new Set());
                        }}
                        style={{ color: token.colorPrimary }}
                      />
                    </Tooltip>
                    <Tooltip title={t("chat.selectAll")}>
                      <Checkbox
                        checked={isAllArchivedSelected}
                        indeterminate={archivedSelectedIds.size > 0 && !isAllArchivedSelected}
                        onChange={handleSelectAllArchived}
                        style={{ marginLeft: 4 }}
                      />
                    </Tooltip>
                    <span style={{ fontSize: 12, color: token.colorTextSecondary }}>
                      {archivedSelectedIds.size} {t("chat.selected")}
                    </span>
                  </>
                )
                : (
                  <>
                    <Button
                      type="text"
                      icon={<ArrowLeft size={16} />}
                      size="small"
                      onClick={handleBackFromArchived}
                      style={{ color: token.colorPrimary }}
                    />
                    <span style={{ fontSize: 13, fontWeight: 500 }}>
                      {t("chat.archived")} ({archivedConversations.length})
                    </span>
                  </>
                )
            )
            : multiSelectMode
            ? (
              <>
                <Tooltip title={t("common.cancel")}>
                  <Button
                    type="text"
                    icon={<X size={16} />}
                    size="small"
                    onClick={exitMultiSelect}
                    style={{ color: token.colorPrimary }}
                  />
                </Tooltip>
                <Tooltip title={t("chat.selectAll")}>
                  <Checkbox
                    checked={isAllSelected}
                    indeterminate={selectedIds.size > 0 && !isAllSelected}
                    onChange={handleSelectAll}
                    style={{ marginLeft: 4 }}
                  />
                </Tooltip>
                <span style={{ fontSize: 12, color: token.colorTextSecondary }}>
                  {selectedIds.size} {t("chat.selected")}
                </span>
              </>
            )
            : (
              <>
                <Tooltip title={t("chat.searchPlaceholder")}>
                  <Button
                    type="text"
                    icon={<Search size={16} />}
                    size="small"
                    onClick={() => setSearchVisible((v) => !v)}
                    style={{ color: token.colorPrimary }}
                  />
                </Tooltip>
                <Tooltip title={t("chat.searchPastSessions")}>
                  <Button
                    type="text"
                    size="small"
                    onClick={() => setAdvancedSearchVisible(true)}
                    style={{
                      color: token.colorTextSecondary,
                      fontSize: 12,
                      maxWidth: 88,
                      overflow: "hidden",
                      textOverflow: "ellipsis",
                      whiteSpace: "nowrap",
                      display: "inline-block",
                      verticalAlign: "middle",
                    }}
                  >
                    {t("chat.searchPastSessions")}
                  </Button>
                </Tooltip>
                <Tooltip title={t("chat.archived")}>
                  <Button
                    type="text"
                    icon={<Archive size={16} />}
                    size="small"
                    onClick={handleShowArchived}
                    style={{ color: token.colorPrimary }}
                  />
                </Tooltip>
                <Tooltip
                  title={shortcutHint(
                    t("chat.newConversation"),
                    "newConversation",
                  )}
                >
                  <Button
                    type="text"
                    data-testid="new-conversation-btn"
                    icon={<MessageSquarePlus size={16} />}
                    size="small"
                    onClick={() => {
                      void handleNewConversation();
                    }}
                    style={{ color: token.colorPrimary }}
                  />
                </Tooltip>
                <Tooltip title={t("chat.multiSelect")}>
                  <Button
                    type="text"
                    icon={<ListTodo size={16} />}
                    size="small"
                    onClick={() => setMultiSelectMode(true)}
                    style={{ color: token.colorPrimary }}
                  />
                </Tooltip>
                <Tooltip title={t("chat.manageCategories")}>
                  <Button
                    type="text"
                    icon={<FolderOpen size={16} />}
                    size="small"
                    onClick={() => setCategoryManagerOpen(true)}
                    style={{ color: token.colorPrimary }}
                  />
                </Tooltip>
              </>
            )}
        </div>
        <div style={{ flexShrink: 0 }}>
          {showArchived
            ? (
              archivedMultiSelect
                ? (
                  <div className="flex items-center gap-1">
                    <Tooltip title={t("chat.unarchive")}>
                      <Button
                        type="text"
                        icon={<Undo2 size={16} />}
                        size="small"
                        disabled={archivedSelectedIds.size === 0}
                        onClick={handleBatchUnarchive}
                        style={{ color: token.colorPrimary }}
                      />
                    </Tooltip>
                    <Tooltip title={t("chat.delete")}>
                      <Button
                        type="text"
                        danger
                        icon={<Trash2 size={16} />}
                        size="small"
                        disabled={archivedSelectedIds.size === 0}
                        onClick={handleBatchDeleteArchived}
                      />
                    </Tooltip>
                  </div>
                )
                : (
                  <Tooltip title={t("chat.multiSelect")}>
                    <Button
                      type="text"
                      icon={<ListTodo size={16} />}
                      size="small"
                      onClick={() => setArchivedMultiSelect(true)}
                      style={{ color: token.colorPrimary }}
                    />
                  </Tooltip>
                )
            )
            : multiSelectMode
            ? (
              <div className="flex items-center gap-1">
                <Tooltip title={t("chat.archive")}>
                  <Button
                    type="text"
                    icon={<Archive size={16} />}
                    size="small"
                    disabled={selectedIds.size === 0}
                    onClick={async () => {
                      const ids = Array.from(selectedIds);
                      if (ids.length === 0) {
                        return;
                      }
                      modal.confirm({
                        title: t("chat.archiveConfirm"),
                        content: t("chat.batchArchiveContent", {
                          count: ids.length,
                        }),
                        mask: { enabled: true, blur: true },
                        onOk: async () => {
                          await batchArchive(ids);
                          exitMultiSelect();
                        },
                      });
                    }}
                    style={{ color: token.colorPrimary }}
                  />
                </Tooltip>
                <Tooltip title={t("chat.delete")}>
                  <Button
                    type="text"
                    danger
                    icon={<Trash2 size={16} />}
                    size="small"
                    disabled={selectedIds.size === 0}
                    onClick={handleBatchDelete}
                  />
                </Tooltip>
              </div>
            )
            : (
              <Tooltip
                title={isCollapsed ? t("common.expand") : t("common.collapse")}
              >
                <Button
                  type="text"
                  icon={isCollapsed ? <PanelLeftOpen size={16} /> : <PanelLeftClose size={16} />}
                  size="small"
                  onClick={() => {
                    setIsCollapsed((v) => !v);
                    onCollapseChange?.(!isCollapsed);
                  }}
                  style={{ color: token.colorPrimary }}
                />
              </Tooltip>
            )}
        </div>
      </div>

      {/* Collapsible search */}
      {!showArchived && searchVisible && !multiSelectMode && (
        <div
          className="chat-sidebar-search"
          style={{ padding: "4px 12px 8px" }}
        >
          <Input
            id="chat-sidebar-input-5"
            prefix={<Search size={14} />}
            placeholder={t("chat.searchPlaceholder")}
            allowClear
            value={searchText}
            onChange={(e) => handleSearch(e.target.value)}
            size="small"
          />
        </div>
      )}

      {showArchived
        ? (
          <div className="flex-1 overflow-y-auto">
            {archivedConversations.length > 0
              ? (
                <div style={{ padding: "4px 0" }}>
                  {archivedConversations.map((conv) => (
                    <div
                      key={conv.id}
                      className="flex items-center gap-2 cursor-pointer"
                      role="button"
                      tabIndex={0}
                      onKeyDown={(e) => {
                        if (e.key === "Enter" || e.key === " ") {
                          e.preventDefault();
                          if (archivedMultiSelect) {
                            toggleArchivedSelect(conv.id);
                          } else {
                            setActiveConversation(conv.id);
                            setShowArchived(false);
                          }
                        }
                      }}
                      style={{
                        padding: "8px 12px",
                        borderRadius: 6,
                        margin: "0 8px",
                      }}
                      onMouseEnter={(e) => {
                        e.currentTarget.style.backgroundColor = token.colorFillContent;
                      }}
                      onMouseLeave={(e) => {
                        e.currentTarget.style.backgroundColor = "";
                      }}
                      onClick={() => {
                        if (archivedMultiSelect) {
                          toggleArchivedSelect(conv.id);
                        } else {
                          setActiveConversation(conv.id);
                          setShowArchived(false);
                        }
                      }}
                    >
                      {archivedMultiSelect && (
                        <Checkbox
                          checked={archivedSelectedIds.has(conv.id)}
                          onChange={() => toggleArchivedSelect(conv.id)}
                          onClick={(e) => e.stopPropagation()}
                        />
                      )}
                      {buildIcon(conv)}
                      <span className="flex-1 truncate text-sm">{conv.title}</span>
                      {!archivedMultiSelect && (
                        <div className="flex items-center gap-1">
                          <Tooltip title={t("chat.unarchive")}>
                            <Button
                              type="text"
                              size="small"
                              icon={<Undo2 size={14} />}
                              onClick={async (e) => {
                                e.stopPropagation();
                                await toggleArchive(conv.id);
                                await fetchArchivedConversations();
                              }}
                            />
                          </Tooltip>
                          <Tooltip title={t("chat.delete")}>
                            <Button
                              type="text"
                              size="small"
                              danger
                              icon={<Trash2 size={14} />}
                              onClick={(e) => {
                                e.stopPropagation();
                                modal.confirm({
                                  title: t("chat.deleteConfirm"),
                                  mask: { enabled: true, blur: true },
                                  okButtonProps: { danger: true },
                                  onOk: async () => {
                                    await deleteConversation(conv.id);
                                    await fetchArchivedConversations();
                                  },
                                });
                              }}
                            />
                          </Tooltip>
                        </div>
                      )}
                    </div>
                  ))}
                </div>
              )
              : (
                <div
                  className="flex items-center justify-center py-8"
                  style={{ color: token.colorTextSecondary }}
                >
                  {t("chat.noArchivedConversations")}
                </div>
              )}
          </div>
        )
        : (
          <Dropdown
            menu={rightClickMenuConfig}
            trigger={["contextMenu"]}
            onOpenChange={(open) => {
              if (!open) {
                setRightClickedConvId(null);
              }
            }}
          >
            <div className="flex-1 overflow-y-auto">
              <div
                onContextMenu={(e) => {
                  if (multiSelectMode) {
                    e.preventDefault();
                    e.stopPropagation();
                    return;
                  }
                  const listItem = (e.target as HTMLElement).closest(
                    "[data-conv-id]",
                  ) as HTMLElement;
                  if (!listItem) {
                    e.preventDefault();
                    e.stopPropagation();
                    return;
                  }
                  const convId = listItem.getAttribute("data-conv-id");
                  if (!convId) {
                    e.preventDefault();
                    e.stopPropagation();
                    return;
                  }
                  setRightClickedConvId(convId);
                }}
              >
                <style>
                  {`
                @keyframes spin {
                  from { transform: rotate(0deg); }
                  to { transform: rotate(360deg); }
                }
              `}
                </style>
                {conversationsLoading && conversations.length === 0
                  ? (
                    <div style={{ padding: "8px 12px" }}>
                      {Array.from({ length: 6 }).map((_, i) => (
                        <div
                          key={i}
                          className="ax-skeleton"
                          style={{
                            height: 36,
                            marginBottom: 8,
                            borderRadius: 6,
                            opacity: 1 - i * 0.12,
                          }}
                        />
                      ))}
                    </div>
                  )
                  : conversationItems.length > 0
                  ? (
                    <div className="conv-list">
                      {(() => {
                        const entries: Array<{ group: string; items: typeof conversationItems }> = [];
                        const seen = new Map<string, typeof conversationItems>();
                        conversationItems.forEach((item) => {
                          const g = item.group ?? "__nogroup__";
                          if (!seen.has(g)) {
                            const arr: typeof conversationItems = [];
                            seen.set(g, arr);
                            entries.push({ group: g, items: arr });
                          }
                          seen.get(g)!.push(item);
                        });
                        return entries.map(({ group, items }) => {
                          const isWsGroup = group.startsWith("ws:");
                          const isExpanded = !isWsGroup || expandedKeys.includes(group);
                          return (
                            <div key={group} className="conv-group">
                              {group !== "__nogroup__" && (
                                <div
                                  className="conv-group-header"
                                  onClick={() => {
                                    if (isWsGroup) {
                                      handleGroupExpand(
                                        isExpanded
                                          ? expandedKeys.filter((k) => k !== group)
                                          : [...expandedKeys, group],
                                      );
                                    }
                                  }}
                                >
                                  {isWsGroup && (
                                    <span className={"conv-group-chevron" + (isExpanded ? " expanded" : "")}>
                                      <ChevronRight size={12} />
                                    </span>
                                  )}
                                  {renderGroupLabel(group)}
                                </div>
                              )}
                              {isExpanded && items.map((item) => (
                                <div
                                  key={item.key}
                                  className={"conv-item" + (activeConversationId === item.key ? " active" : "")}
                                  data-conv-id={item["data-conv-id"]}
                                  style={item.style}
                                  onClick={() => handleConversationClick(item.key)}
                                  role="button"
                                  tabIndex={0}
                                  onKeyDown={(e) => {
                                    if (e.key === "Enter" || e.key === " ") {
                                      e.preventDefault();
                                      handleConversationClick(item.key);
                                    }
                                  }}
                                >
                                  <span className="conv-item-icon">{item.icon}</span>
                                  <span className="conv-item-label">{item.label}</span>
                                  {!multiSelectMode && (
                                    <Dropdown menu={menuConfig(item)} trigger={["click"]} placement="bottomRight">
                                      <button
                                        className="conv-item-menu-btn"
                                        onClick={(e) =>
                                          e.stopPropagation()}
                                        aria-label="Menu"
                                      >
                                        <Pencil size={12} />
                                      </button>
                                    </Dropdown>
                                  )}
                                </div>
                              ))}
                            </div>
                          );
                        });
                      })()}
                    </div>
                  )
                  : (
                    <div className="flex items-center justify-center h-full">
                      <Empty
                        description={t("chat.noConversations")}
                        image={Empty.PRESENTED_IMAGE_SIMPLE}
                      />
                    </div>
                  )}
              </div>
            </div>
          </Dropdown>
        )}

      <Modal
        title={archiveTargetIds.length > 1
          ? t("chat.batchArchiveToKnowledgeBase")
          : t("chat.archiveToKnowledgeBase")}
        open={archiveKbModalOpen}
        onCancel={() => setArchiveKbModalOpen(false)}
        footer={[
          <Button
            key="workflow"
            icon={<GitBranch size={14} />}
            onClick={async () => {
              setArchiveKbModalOpen(false);
              if (archiveTargetId) {
                await handleSaveAsWorkflow(archiveTargetId);
              } else if (archiveTargetIds.length === 1) {
                await handleSaveAsWorkflow(Array.from(archiveTargetIds)[0]);
              } else {
                messageApi.warning(t("chat.selectOneConversation"));
              }
            }}
          >
            {t("chat.saveAsWorkflow")}
          </Button>,
          <Button
            key="archive"
            type="primary"
            disabled={!selectedKbId}
            loading={archiveLoading}
            onClick={handleArchiveConfirm}
          >
            {t("chat.archive")}
          </Button>,
        ]}
        destroyOnHidden
      >
        <Radio.Group
          value={selectedKbId}
          onChange={(e) => setSelectedKbId(e.target.value)}
          style={{ width: "100%" }}
        >
          <Space direction="vertical" style={{ width: "100%" }}>
            {knowledgeBases.length === 0 && (
              <span style={{ color: token.colorTextSecondary }}>
                {t("chat.noKnowledgeBases")}
              </span>
            )}
            {knowledgeBases.map((kb) => (
              <Radio key={kb.id} value={kb.id} style={{ width: "100%" }}>
                {kb.name}
              </Radio>
            ))}
          </Space>
        </Radio.Group>
      </Modal>

      <SessionSearchPanel
        visible={advancedSearchVisible}
        onClose={() => setAdvancedSearchVisible(false)}
        onSelectResult={(result) => {
          setActiveConversation(result.session_id);
          setAdvancedSearchVisible(false);
        }}
      />

      <CategoryManagerModal
        open={categoryManagerOpen}
        onClose={() => setCategoryManagerOpen(false)}
      />
    </div>
  );
}

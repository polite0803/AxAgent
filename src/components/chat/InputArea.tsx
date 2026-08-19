// SPDX-License-Identifier: AGPL-3.0-only

import { DropdownMenu } from "@/components/layout/DropdownMenu";
import type { DropdownItem } from "@/components/layout/DropdownMenu";
import { Tooltip } from "@/components/layout/Tooltip";
import { PROVIDER_TYPE_LABELS, SearchProviderTypeIcon } from "@/components/shared/SearchProviderIcon";
import { SkillToolbar } from "@/components/skill/SkillToolbar";
import { useVoiceWakeup } from "@/hooks/useVoiceWakeup";
import { invoke, isTauri, logIpcError } from "@/lib/invoke";
import { findModelByIds, modelHasCapability, supportsReasoning } from "@/lib/modelCapabilities";
import { formatShortcutForDisplay, getShortcutBinding } from "@/lib/shortcuts";
import type { ShortcutAction } from "@/lib/shortcuts";
import { estimateMessageTokens, estimateTokens } from "@/lib/tokenEstimator";
import {
  useAgentStore,
  useCompressStore,
  useConversationStore,
  useExecutionStore,
  useGatewayLinkStore,
  useKnowledgeStore,
  useMcpStore,
  useMemoryStore,
  usePlanStore,
  useProviderStore,
  useSearchStore,
  useSettingsStore,
  useStreamStore,
  useUIStore,
  useVoicePreferenceStore,
} from "@/stores";
import { useGatewayStore } from "@/stores/feature/gatewayStore";
import { useLlmWikiStore } from "@/stores/feature/llmWikiStore";
import { usePromptTemplateStore } from "@/stores/feature/promptTemplateStore";
import type { PromptTemplate } from "@/types";
import { type AttachmentInput, type RealtimeConfig } from "@/types";
import { AudioOutlined, TeamOutlined } from "@ant-design/icons";
import { open } from "@tauri-apps/plugin-dialog";
import { App, Button, Popover, Tag, theme } from "antd";
import {
  Check,
  Eraser,
  FileText,
  Film,
  GitCompareArrows,
  Globe,
  GripHorizontal,
  Image as ImageIcon,
  Mic,
  Paperclip,
  Scissors,
  Shrink,
  SlidersHorizontal,
  Upload,
  Zap,
  ZapOff,
} from "lucide-react";
import React, { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { CommandSuggest } from "./CommandSuggest";
import { CompanionModelTags } from "./CompanionModelTags";
import { ContextSourceMenu } from "./ContextSourceMenu";
import { ConversationSettingsModal } from "./ConversationSettingsModal";
import { DelegateTaskModal } from "./DelegateTaskModal";
import { GatewayMenu } from "./GatewayMenu";
import { InputAreaFileList } from "./InputAreaFileList";
import { fileToAttachmentInput } from "./InputAreaUtils";
import { McpMenu } from "./McpMenu";
import { ModelSelector } from "./ModelSelector";
import { PermissionMenu } from "./PermissionMenu";
import { PlanHistoryPanel } from "./PlanHistoryPanel";
import { PromptTemplateSelector } from "./PromptTemplateSelector";
import { QuotePreviewBar } from "./QuotePreviewBar";
import { SendControls } from "./SendControls";
import { ThinkingMenu } from "./ThinkingMenu";
import { VoiceCall } from "./VoiceCall";
import { WorkspaceDirMenu } from "./WorkspaceDirMenu";

// In-memory draft cache: persists input text per-conversation across component unmounts
const _draftCache = new Map<string, string>();
// Cache is module-level, conversation switch clears by key mismatch

export function InputArea() {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const { message: messageApi, modal } = App.useApp();
  const [value, setValue] = useState(() => {
    const convId = useConversationStore.getState().activeConversationId;
    return convId ? _draftCache.get(convId) || "" : "";
  });
  const [attachedFiles, setAttachedFiles] = useState<File[]>([]);

  const objectUrlsRef = useRef<string[]>([]);
  // 仅为缓存上一次的 URLs（用于 cleanup），不能在 render phase 写入 ref
  const attachmentObjectUrls = useMemo(
    () => attachedFiles.map((f) => URL.createObjectURL(f)),
    [attachedFiles],
  );
  useEffect(() => {
    // 同步最新 URLs 到 ref，供后续 cleanup 读取
    objectUrlsRef.current = attachmentObjectUrls;
    return () => {
      // 组件卸载或 attachedFiles 更新前，释放旧 blob URL
      objectUrlsRef.current.forEach((url) => URL.revokeObjectURL(url));
    };
  }, [attachmentObjectUrls]);
  const [voiceCallVisible, setVoiceCallVisible] = useState(false);
  const [voiceApiKey, setVoiceApiKey] = useState("");

  // 语音唤醒：轻量常驻监听，命中后打开语音通话浮层（通话已打开时不重复触发）
  const voiceCallVisibleRef = useRef(false);
  voiceCallVisibleRef.current = voiceCallVisible;
  const voiceWakeup = useVoiceWakeup({
    onWake: () => {
      if (!voiceCallVisibleRef.current) {
        setVoiceCallVisible(true);
      }
    },
  });
  const gatewayKeys = useGatewayStore((s) => s.keys);
  const photoInputRef = useRef<HTMLInputElement>(null);
  const audioInputRef = useRef<HTMLInputElement>(null);
  const videoInputRef = useRef<HTMLInputElement>(null);
  const [settingsOpen, setSettingsOpen] = useState(false);

  const [searchDropdownOpen, setSearchDropdownOpen] = useState(false);
  const [delegateModalOpen, setDelegateModalOpen] = useState(false);
  const fileInputRef = useRef<HTMLInputElement>(null);
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const valueRef = useRef(value);
  // eslint-disable-next-line react-hooks/refs
  valueRef.current = value;
  const [cursorPosition, setCursorPosition] = useState(0);
  const [showSuggest, setShowSuggest] = useState(false);
  const prevConvIdRef = useRef<string | null>(
    useConversationStore.getState().activeConversationId ?? null,
  );

  // Drag-to-resize state: userMinHeight controls the minimum visible height of the textarea
  const INITIAL_MIN_HEIGHT = 44;
  const ABSOLUTE_MAX_HEIGHT = 600;
  const [userMinHeight, setUserMinHeight] = useState(INITIAL_MIN_HEIGHT);
  const userMinHeightRef = useRef(userMinHeight);
  // eslint-disable-next-line react-hooks/refs
  userMinHeightRef.current = userMinHeight;
  const dragStateRef = useRef<{ startY: number; startH: number } | null>(null);
  const hasUserResizedRef = useRef(false);
  const containerRef = useRef<HTMLDivElement>(null);

  // Multi-model companion state
  const [companionModels, setCompanionModels] = useState<
    Array<{ providerId: string; modelId: string }>
  >([]);
  const [multiModelOpen, setMultiModelOpen] = useState(false);
  const sendMultiModelMessage = useConversationStore(
    (s) => s.sendMultiModelMessage,
  );

  const activeConversationId = useConversationStore(
    (s) => s.activeConversationId,
  );
  const activeStreams = useStreamStore((s) => s.activeStreams);
  const streaming = activeConversationId
    ? activeConversationId in activeStreams
    : false;
  const compressing = useCompressStore((s) => s.compressing);
  const cancelCurrentStream = useStreamStore((s) => s.cancelCurrentStream);
  const sendMessage = useConversationStore((s) => s.sendMessage);
  const createConversation = useConversationStore((s) => s.createConversation);
  const messagesLength = useConversationStore((s) => s.messages.length);
  const totalActiveCount = useConversationStore((s) => s.totalActiveCount);
  const hasOlderMessages = useConversationStore((s) => s.hasOlderMessages);
  const contextCount = useMemo(() => {
    const msgs = useConversationStore.getState().messages;
    const activeMessages = msgs.filter((m) => m.isActive !== false);
    const lastMarkerIdx = activeMessages.reduce((maxIdx, m, i) => {
      if (
        m.content === "<!-- context-clear -->"
        || m.content === "<!-- context-compressed -->"
      ) {
        return i;
      }
      return maxIdx;
    }, -1);
    if (lastMarkerIdx !== -1) {
      return activeMessages.slice(lastMarkerIdx + 1).length;
    }
    if (hasOlderMessages && totalActiveCount > 0) {
      return totalActiveCount;
    }
    return activeMessages.length;
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [messagesLength, hasOlderMessages, totalActiveCount]);

  const conversations = useConversationStore((s) => s.conversations);
  const providers = useProviderStore((s) => s.providers);
  const providersLoading = useProviderStore((s) => s.loading);
  const settings = useSettingsStore((s) => s.settings);

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

  // Search state
  const searchEnabled = useConversationStore((s) => s.searchEnabled);
  const searchProviderId = useConversationStore((s) => s.searchProviderId);
  const setSearchEnabled = useConversationStore((s) => s.setSearchEnabled);
  const setSearchProviderId = useConversationStore(
    (s) => s.setSearchProviderId,
  );
  const searchProviders = useSearchStore((s) => s.providers);
  // 兜底策略（三级）：
  // 1. 用户明确选择的 → 直接使用
  // 2. 未选但有启用的 → 自动取第一个
  // 3. 全未启用/无配置 → 仍传非空值，后端 DuckDuckGo 免费搜索兜底
  const effectiveSearchProviderId = useMemo(() => {
    if (searchProviderId) { return searchProviderId; }
    const enabled = (searchProviders || []).filter((p) => p.enabled);
    if (enabled.length > 0) { return enabled[0].id; }
    // 没有任何可用服务商时，传一个非空占位让后端走 DDG 免费搜索
    return searchEnabled ? "__ddg_fallback__" : null;
  }, [searchEnabled, searchProviderId, searchProviders]);

  // Agent permission mode state
  const [agentPermissionMode, setAgentPermissionMode] = useState<string>("default");

  // Agent working directory state
  const [agentCwd, setAgentCwd] = useState<string | null>(null);

  // Gateway links state
  const [selectedGatewayId, setSelectedGatewayId] = useState<string | null>(
    null,
  );

  // Prompt template state
  const [templatePopoverOpen, setTemplatePopoverOpen] = useState(false);

  // Context clear
  const insertContextClear = useConversationStore((s) => s.insertContextClear);
  const clearAllMessages = useConversationStore((s) => s.clearAllMessages);
  const updateConversation = useConversationStore((s) => s.updateConversation);
  const compressContext = useCompressStore((s) => s.compressContext);

  // Track the last mode choice from the dropdown when no conversation is active.
  // This allows handleSend to create a conversation in the correct mode
  // even when the user hasn't created one yet.
  const pendingModeRef = useRef<"chat" | "agent" | null>(null);

  const activeConversation = conversations.find(
    (c) => c.id === activeConversationId,
  );
  // Use pendingModeRef when no conversation exists so the UI (mode badge, send routing)
  // correctly reflects the user's last mode dropdown choice.
  // eslint-disable-next-line react-hooks/refs
  const currentMode = activeConversation?.mode || pendingModeRef.current || "chat";

  // Reset pending mode refs when a conversation becomes active
  useEffect(() => {
    if (activeConversationId) {
      pendingModeRef.current = null;
    }
  }, [activeConversationId]);

  // 认知编排器模式下不再提供 act/plan/ask/auto 手动模式选择，
  // 执行模式统一交由认知编排器路由自动决策（始终以 auto 下发）

  // 引用回复：当前被引用的消息 ID
  const quotedMessageId = useUIStore((s) => s.quotedMessageId);
  const quotedMessage = useMemo(() => {
    if (!quotedMessageId) { return null; }
    return useConversationStore.getState().messages.find((m) => m.id === quotedMessageId) ?? null;
  }, [quotedMessageId]);

  // 引用回复：切换会话时清除引用状态，避免跨会话残留
  useEffect(() => {
    useUIStore.getState().setQuotedMessageId(null);
  }, [activeConversationId]);

  // Mount 时加载各 store 数据：依赖必须为空数组，避免 store 在 IPC 失败时
  // set 新空数组导致引用变化 → useEffect 重新执行 → 无限循环
  useEffect(() => {
    if ((useSearchStore.getState().providers ?? []).length === 0) {
      useSearchStore.getState().loadProviders();
    }
  }, []);

  useEffect(() => {
    if ((useMcpStore.getState().servers ?? []).length === 0) {
      useMcpStore.getState().loadServers();
    }
  }, []);

  useEffect(() => {
    if ((useKnowledgeStore.getState().bases ?? []).length === 0) {
      useKnowledgeStore.getState().loadBases();
    }
  }, []);

  useEffect(() => {
    if ((useMemoryStore.getState().namespaces ?? []).length === 0) {
      useMemoryStore.getState().loadNamespaces();
    }
  }, []);

  useEffect(() => {
    if ((useLlmWikiStore.getState().wikis ?? []).length === 0) {
      useLlmWikiStore.getState().loadWikis();
    }
  }, []);

  useEffect(() => {
    if ((useGatewayLinkStore.getState().links ?? []).length === 0) {
      useGatewayLinkStore.getState().fetchLinks();
    }
  }, []);

  // Set default workspace directory when in agent mode and no conversation is active
  useEffect(() => {
    if (
      !activeConversationId
      && currentMode === "agent"
      && settings.defaultWorkspaceDir
    ) {
      // eslint-disable-next-line react-hooks/set-state-in-effect
      setAgentCwd(settings.defaultWorkspaceDir);
    }
  }, [activeConversationId, currentMode, settings.defaultWorkspaceDir]);

  // Fetch agent permission mode on mount/conversation switch
  useEffect(() => {
    if (currentMode === "agent" && activeConversationId) {
      invoke("agent_get_session", {
        request: { conversationId: activeConversationId },
      })
        .then((session: unknown) => {
          const s = session as { permissionMode?: string; cwd?: string | null } | null;
          if (s) {
            setAgentPermissionMode(s.permissionMode || "default");
            setAgentCwd(s.cwd || null);
          }
        })
        .catch(logIpcError("IPC: load agent session info"));
    }
  }, [currentMode, activeConversationId]);

  // Draft persistence: save old draft & restore new when conversation changes
  useEffect(() => {
    const prev = prevConvIdRef.current;
    if (prev && prev !== activeConversationId) {
      const draft = valueRef.current;
      if (draft) {
        _draftCache.set(prev, draft);
      } else {
        _draftCache.delete(prev);
      }
    }
    // eslint-disable-next-line react-hooks/set-state-in-effect
    setValue(
      activeConversationId ? _draftCache.get(activeConversationId) || "" : "",
    );
    prevConvIdRef.current = activeConversationId ?? null;
  }, [activeConversationId]);

  // Save draft on unmount (navigating away from chat page)
  useEffect(() => {
    return () => {
      const convId = prevConvIdRef.current;
      if (convId && valueRef.current) {
        _draftCache.set(convId, valueRef.current);
      }
    };
  }, []);

  // Persist companion models per conversation in localStorage
  const companionStorageKeyRef = useRef(
    activeConversationId
      ? `axagent:companion-models:${activeConversationId}`
      : null,
  );
  // eslint-disable-next-line react-hooks/refs
  companionStorageKeyRef.current = activeConversationId
    ? `axagent:companion-models:${activeConversationId}`
    : null;
  // eslint-disable-next-line react-hooks/refs
  const companionStorageKey = companionStorageKeyRef.current
    ? `axagent:companion-models:${activeConversationId}`
    : null;

  // Load companion models when conversation changes
  useEffect(() => {
    if (!companionStorageKey) {
      // eslint-disable-next-line react-hooks/set-state-in-effect
      setCompanionModels([]);
      return;
    }
    try {
      const saved = localStorage.getItem(companionStorageKey);
      setCompanionModels(saved ? JSON.parse(saved) : []);
    } catch {
      setCompanionModels([]);
    }
  }, [companionStorageKey]);

  // Pick up pending prompt text from welcome cards and populate the input field
  const pendingPromptText = useConversationStore((s) => s.pendingPromptText);
  useEffect(() => {
    if (!pendingPromptText) {
      return;
    }
    const text = pendingPromptText;
    useConversationStore.getState().setPendingPromptText(null);
    // eslint-disable-next-line react-hooks/set-state-in-effect
    setValue(text);
  }, [pendingPromptText]);

  // Search dropdown menu items
  const searchMenuItems = useMemo((): DropdownItem[] => {
    const available = searchProviders;
    if (available.length === 0) {
      return [
        {
          key: "__empty",
          label: (
            <span style={{ color: token.colorTextSecondary, fontSize: 12 }}>
              {t("chat.search.noProviders")}
            </span>
          ),
          disabled: true,
        },
      ];
    }
    return available.map((p) => ({
      key: p.id,
      label: (
        <div className="flex items-center gap-2" style={{ minWidth: 140 }}>
          <Tag
            color="blue"
            style={{
              margin: 0,
              fontSize: 12,
              lineHeight: "18px",
              padding: "0 6px",
              display: "inline-flex",
              alignItems: "center",
              gap: 3,
            }}
          >
            <SearchProviderTypeIcon type={p.providerType} size={14} />
            {PROVIDER_TYPE_LABELS[p.providerType] || p.providerType}
          </Tag>
          <span className="flex-1" style={{ fontSize: 13 }}>
            {p.name}
          </span>
          {searchEnabled && searchProviderId === p.id && <Check size={14} style={{ color: token.colorPrimary }} />}
        </div>
      ),
      onClick: () => {
        setSearchEnabled(true);
        setSearchProviderId(p.id);
      },
    }));
  }, [searchProviders, searchEnabled, searchProviderId, setSearchEnabled, setSearchProviderId, token, t]);

  // 认知编排器模式下专家/角色由路由自动选择，不再提供手动专家选择

  // Agent permission mode items
  const handlePermissionModeChange = useCallback(
    async (mode: string) => {
      if (!activeConversationId) {
        return;
      }

      const applyChange = async () => {
        try {
          await invoke("agent_update_session", {
            request: {
              conversationId: activeConversationId,
              permissionMode: mode,
            },
          });
          setAgentPermissionMode(mode);
        } catch (e) {
          logIpcError("Failed to update permission mode")(e);
        }
      };

      if (mode === "accept_edits" || mode === "full_access") {
        const isFullAccess = mode === "full_access";
        modal.confirm({
          title: isFullAccess
            ? t("agent.permissionFullAccessWarningTitle")
            : t("agent.permissionAcceptEditsWarningTitle"),
          content: isFullAccess
            ? t("agent.permissionFullAccessWarning")
            : t("agent.permissionAcceptEditsWarning"),
          okText: t("common.confirm"),
          cancelText: t("common.cancel"),
          okButtonProps: isFullAccess ? { danger: true } : undefined,
          onOk: applyChange,
        });
      } else {
        await applyChange();
      }
    },
    [activeConversationId, modal, t],
  );

  // ── Work Strategy ──────────────────────────────────────────────────
  // 认知编排器模式下工作策略由路由自动决策，不再提供手动直接/计划切换

  const handleSelectCwd = useCallback(async () => {
    try {
      let selected: string | null = null;
      if (isTauri()) {
        const result = await open({
          directory: true,
          multiple: false,
          title: t("common.selectDirectory"),
        });
        if (result && typeof result === "string") {
          selected = result;
        }
      } else {
        try {
          const handle = await window.showDirectoryPicker();
          selected = handle.name;
        } catch {
          // User cancelled or browser doesn't support showDirectoryPicker
          return;
        }
      }
      if (selected) {
        if (activeConversationId) {
          await invoke("agent_update_session", {
            request: { conversationId: activeConversationId, cwd: selected },
          });
        }
        setAgentCwd(selected);
      }
    } catch (e) {
      logIpcError("Failed to select working directory")(e);
    }
  }, [activeConversationId, t]);

  const incrementUsage = usePromptTemplateStore((s) => s.incrementUsage);

  const handleTemplateSelect = useCallback(
    (template: PromptTemplate, filledContent: string) => {
      setValue((prev) => prev ? prev + "\n\n" + filledContent : filledContent);
      setTemplatePopoverOpen(false);
      textareaRef.current?.focus();
      incrementUsage(template.id);
    },
    [incrementUsage],
  );

  const templatePopoverContent = useMemo(() => {
    return <PromptTemplateSelector onSelect={handleTemplateSelect} />;
  }, [handleTemplateSelect]);

  const currentModel = React.useMemo(() => {
    if (activeConversation) {
      return findModelByIds(
        providers,
        activeConversation.providerId,
        activeConversation.modelId,
      );
    }

    if (settings.defaultProviderId && settings.defaultModelId) {
      const defaultModel = findModelByIds(
        providers,
        settings.defaultProviderId,
        settings.defaultModelId,
      );
      if (defaultModel?.enabled) {
        return defaultModel;
      }
    }

    for (const provider of providers) {
      if (!provider.enabled) {
        continue;
      }
      for (const item of provider.models) {
        if (item.enabled) {
          return item;
        }
      }
    }

    return null;
  }, [
    activeConversation,
    providers,
    settings.defaultProviderId,
    settings.defaultModelId,
  ]);

  // Context token usage calculation
  const getCompressionSummary = useCompressStore(
    (s) => s.getCompressionSummary,
  );
  const [summaryTokenCount, setSummaryTokenCount] = useState<number>(0);

  useEffect(() => {
    if (!activeConversationId || !activeConversation?.contextCompression) {
      // eslint-disable-next-line react-hooks/set-state-in-effect
      setSummaryTokenCount(0);
      return;
    }
    getCompressionSummary(activeConversationId).then((s) => {
      setSummaryTokenCount(s?.tokenCount ?? 0);
    });
  }, [
    activeConversationId,
    activeConversation?.contextCompression,
    getCompressionSummary,
    messagesLength,
  ]);

  const contextTokenUsage = useMemo(() => {
    const maxTokens = currentModel?.maxTokens;
    if (!maxTokens) {
      return null;
    }

    const msgs = useConversationStore.getState().messages;
    const activeMessages = msgs.filter((m) => m.isActive !== false);
    const lastMarkerIdx = activeMessages.reduce((maxIdx, m, i) => {
      if (
        m.content === "<!-- context-clear -->"
        || m.content === "<!-- context-compressed -->"
      ) {
        return i;
      }
      return maxIdx;
    }, -1);
    const effectiveMessages = lastMarkerIdx === -1
      ? activeMessages
      : activeMessages.slice(lastMarkerIdx + 1);
    let usedTokens = effectiveMessages.reduce(
      (sum, m) => sum + estimateMessageTokens(m.role, m.content),
      0,
    );

    if (activeConversation?.systemPrompt) {
      usedTokens += estimateTokens(activeConversation.systemPrompt) + 4;
    }

    usedTokens += summaryTokenCount;

    const isEstimate = hasOlderMessages && lastMarkerIdx === -1;
    const percent = Math.min(Math.round((usedTokens / maxTokens) * 100), 100);
    return { usedTokens, maxTokens, percent, isEstimate };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [
    messagesLength,
    currentModel?.maxTokens,
    activeConversation?.systemPrompt,
    summaryTokenCount,
    hasOlderMessages,
  ]);

  // 语音通话入口是否可用：实时语音走 AxAgent 本地网关 + STT/TTS 编排，
  // 与具体模型的 RealtimeVoice 能力无关（普通聊天模型不可编辑该能力，导致按钮永不出现），
  // 也不依赖某个已选中的会话（realtime 通话使用独立 config，与 conversation 解耦）。
  // 仅在「工作流会话」下隐藏；无活跃会话或普通会话均显示语音入口。
  const { voiceAvailable, hasReasoning, hasVision } = React.useMemo(
    () => ({
      // 语音入口对所有会话可用（工作流手动绑定已移除）
      voiceAvailable: true,
      hasReasoning: supportsReasoning(currentModel),
      hasVision: modelHasCapability(currentModel, "Vision"),
    }),
    [currentModel],
  );

  // Current model key for excluding from multi-select (no longer used - users can select any model)

  const companionDisplayInfos = useMemo(() => {
    return companionModels.map((cm) => {
      const provider = providers.find((p) => p.id === cm.providerId);
      const model = provider?.models.find((m) => m.modelId === cm.modelId);
      return {
        ...cm,
        modelName: model?.name ?? cm.modelId,
        providerName: provider?.name ?? "",
      };
    });
  }, [companionModels, providers]);

  const handleMultiModelSelect = useCallback(
    (models: Array<{ providerId: string; modelId: string }>) => {
      setCompanionModels(models);
      if (companionStorageKey) {
        try {
          if (models.length > 0) {
            localStorage.setItem(companionStorageKey, JSON.stringify(models));
          } else {
            localStorage.removeItem(companionStorageKey);
          }
        } catch {
          // localStorage quota exceeded or unavailable
        }
      }
    },
    [companionStorageKey],
  );

  const removeCompanionModel = useCallback(
    (index: number) => {
      setCompanionModels((prev) => {
        const next = prev.filter((_, i) => i !== index);
        if (companionStorageKey) {
          try {
            if (next.length > 0) {
              localStorage.setItem(companionStorageKey, JSON.stringify(next));
            } else {
              localStorage.removeItem(companionStorageKey);
            }
          } catch {
            // localStorage quota exceeded or unavailable
          }
        }
        return next;
      });
    },
    [companionStorageKey],
  );

  const clearAllCompanionModels = useCallback(() => {
    setCompanionModels([]);
    if (companionStorageKey) {
      localStorage.removeItem(companionStorageKey);
    }
  }, [companionStorageKey]);

  const ttsVoice = useVoicePreferenceStore((s) => s.ttsVoice);
  const sttProviderId = useVoicePreferenceStore((s) => s.sttProviderId);
  const ttsProviderId = useVoicePreferenceStore((s) => s.ttsProviderId);

  const voiceConfig: RealtimeConfig = React.useMemo(
    () => ({
      modelId: activeConversation?.modelId ?? "",
      voice: ttsVoice,
      audioFormat: { sampleRate: 24000, channels: 1, encoding: "Pcm16" },
      sttProviderId: sttProviderId || null,
      ttsProviderId: ttsProviderId || null,
    }),
    [activeConversation?.modelId, ttsVoice, sttProviderId, ttsProviderId],
  );

  // Mutex to prevent concurrent mode switches (e.g. rapid double-clicks)
  const isSwitchingModeRef = useRef(false);

  const handleModeSwitch = useCallback(
    async (mode: "chat" | "agent") => {
      if (isSwitchingModeRef.current) {
        return;
      }
      isSwitchingModeRef.current = true;
      try {
        if (!activeConversation) {
          // No active conversation: store the mode choice so handleSend creates the right type
          if (mode === "agent") {
            pendingModeRef.current = "agent";
            messageApi.info(
              t(
                "chat.switchAgentModeNoConversationInfo",
              ),
            );
          } else {
            pendingModeRef.current = null;
          }
          return;
        }

        // Prevent switching while the current conversation is streaming
        const { activeStreams } = useStreamStore.getState();
        if (activeConversation.id in activeStreams) {
          return;
        }

        try {
          await updateConversation(activeConversation.id, { mode });
        } catch (e) {
          const errorMsg = String(e);
          if (errorMsg.includes("Not found: Conversation")) {
            logIpcError("ModeSwitch: conversation not found, refreshing")(e);
            messageApi.warning(t("chat.conversationNotFound"));
            await useConversationStore
              .getState()
              .fetchConversations()
              .catch(logIpcError("IPC: fetch conversations after not-found"));
            const { conversations } = useConversationStore.getState();
            if (conversations.length > 0) {
              useConversationStore
                .getState()
                .setActiveConversation(conversations[0].id);
            } else {
              useConversationStore.getState().setActiveConversation(null);
            }
          } else {
            logIpcError("ModeSwitch: updateConversation failed")(e);
          }
          return;
        }

        if (mode === "agent") {
          // Clear multi-model companion models — not applicable in agent mode
          if (companionModels.length > 0) {
            setCompanionModels([]);
            if (companionStorageKey) {
              localStorage.removeItem(companionStorageKey);
            }
          }
          try {
            // agent_update_session is a lightweight DB query, give it 10s timeout
            const session = await invoke<{ cwd: string | null }>(
              "agent_update_session",
              {
                request: { conversationId: activeConversation.id },
              },
              10_000,
            );
            if (!session.cwd) {
              // agent_ensure_workspace is a filesystem operation, give it 15s timeout
              // (default 5-min timeout is excessive and masks backend connection issues)
              const workspaceResult = await invoke<{ workspacePath: string }>(
                "agent_ensure_workspace",
                {
                  request: { conversationId: activeConversation.id },
                },
                15_000,
              );
              const workspacePath = workspaceResult.workspacePath;
              await invoke(
                "agent_update_session",
                {
                  request: {
                    conversationId: activeConversation.id,
                    cwd: workspacePath,
                  },
                },
                10_000,
              );
              setAgentCwd(workspacePath);
            } else {
              setAgentCwd(session.cwd);
            }
          } catch (e) {
            const errMsg = String(e);
            const isTransient = errMsg.includes("connection")
              || errMsg.includes("refused")
              || errMsg.includes("timeout")
              || errMsg.includes("fetch")
              || errMsg.includes("IPC")
              || errMsg.includes("backend");
            logIpcError("ModeSwitch: init agent session")(e);

            if (isTransient) {
              // Transient IPC error: backend may be temporarily unavailable.
              // Do NOT rollback to chat mode — the conversation mode stays as "agent"
              // so the user doesn't need to manually re-switch when backend recovers.
              messageApi.warning(
                t(
                  "chat.agentInitTransient",
                ),
              );
            } else {
              // Genuine session init failure: rollback to chat mode
              try {
                await updateConversation(activeConversation.id, {
                  mode: "chat",
                });
              } catch (rollbackErr) {
                logIpcError("ModeSwitch: rollback mode")(rollbackErr);
              }
              messageApi.error(t("chat.agentInitFailed"));
            }
          }
        } else {
          // Switching to chat mode: clear agent-related stores to prevent stale UI state
          const { clearConversation } = useAgentStore.getState();
          clearConversation(activeConversation.id);
          useExecutionStore.getState().clearConversation(activeConversation.id);
          usePlanStore.getState().clearActivePlan(activeConversation.id);
        }
      } finally {
        isSwitchingModeRef.current = false;
      }
    },
    [
      activeConversation,
      updateConversation,
      companionModels,
      companionStorageKey,
      messageApi,
      t,
    ],
  );

  // ── Unified Mode (Ask / Plan / Action) ──
  // 认知编排器模式下执行模式由路由自动决策，不再提供手动 Ask / Plan / Action 选择

  // 解析默认 provider + model：优先 settings 默认值，回退到第一个启用的 provider/model。
  // 统一供 handleSend 与「新建会话」动作复用，避免各处重复且行为不一致。
  const resolveDefaultProviderModel = useCallback(() => {
    if (providersLoading || (providers ?? []).length === 0) {
      return null;
    }
    let provider = settings.defaultProviderId
      ? providers.find(
        (p) => p.id === settings.defaultProviderId && p.enabled,
      )
      : undefined;
    let model = provider?.models.find(
      (m) => m.modelId === settings.defaultModelId && m.enabled,
    );
    if (!provider || !model) {
      provider = providers.find(
        (p) => p.enabled && p.models.some((m) => m.enabled),
      );
      model = provider?.models.find((m) => m.enabled);
    }
    return provider && model ? { provider, model } : null;
  }, [providers, providersLoading, settings.defaultProviderId, settings.defaultModelId]);

  const handleSend = useCallback(async () => {
    const trimmed = value.trim();
    if (!trimmed || streaming) {
      return;
    }

    const submittedFiles = attachedFiles;

    try {
      if (!activeConversationId) {
        if (currentMode === "gateway" && selectedGatewayId) {
          const conversationId = await useGatewayLinkStore
            .getState()
            .createGatewayConversation(selectedGatewayId);
          useConversationStore.getState().setActiveConversation(conversationId);
        } else {
          const resolved = resolveDefaultProviderModel();
          if (!resolved) {
            messageApi.warning(t("chat.noModelsAvailable"));
            return;
          }
          await createConversation(
            trimmed.slice(0, 30),
            resolved.model.modelId,
            resolved.provider.id,
            {
              mode: pendingModeRef.current ?? undefined,
            },
          );
          pendingModeRef.current = null;
        }
      }

      let attachments: AttachmentInput[] | undefined;
      if (submittedFiles.length > 0) {
        attachments = await Promise.all(
          submittedFiles.map(fileToAttachmentInput),
        );
      }

      setValue("");
      setAttachedFiles([]);
      // Reset textarea height and drag state after clearing content
      hasUserResizedRef.current = false;
      setUserMinHeight(INITIAL_MIN_HEIGHT);
      userMinHeightRef.current = INITIAL_MIN_HEIGHT;
      requestAnimationFrame(() => {
        if (textareaRef.current) {
          textareaRef.current.style.height = "auto";
        }
      });
      // 统一入口：所有会话（chat / agent / plan）统一走 cognitive_query 认知编排器，
      // modeHint 仅在用户显式选择时覆盖（ask/plan/act），否则交由路由自动决策
      if (companionModels.length > 0) {
        await sendMultiModelMessage(
          trimmed,
          companionModels,
          attachments,
          effectiveSearchProviderId,
        );
      } else {
        await sendMessage(
          trimmed,
          attachments,
          effectiveSearchProviderId,
          quotedMessageId,
          "auto",
        );
      }
      // 引用回复：发送成功后清除引用状态
      useUIStore.getState().setQuotedMessageId(null);
    } catch (e) {
      setValue((current) => current || trimmed);
      setAttachedFiles((current) => current.length > 0 ? current : submittedFiles);
      logIpcError("handleSend")(e);
      messageApi.error(String(e));
      // Re-expand textarea after restoring content
      requestAnimationFrame(() => {
        const textarea = textareaRef.current;
        if (textarea) {
          textarea.style.height = "auto";
          const desired = hasUserResizedRef.current
            ? userMinHeightRef.current
            : Math.max(textarea.scrollHeight, userMinHeightRef.current);
          textarea.style.height = Math.min(desired, ABSOLUTE_MAX_HEIGHT) + "px";
        }
      });
    }
  }, [
    value,
    attachedFiles,
    streaming,
    sendMessage,
    sendMultiModelMessage,
    companionModels,
    activeConversationId,
    resolveDefaultProviderModel,
    createConversation,
    messageApi,
    t,
    currentMode,
    selectedGatewayId,
    effectiveSearchProviderId,
    quotedMessageId,
  ]);

  const handleFillLastMessage = useCallback(() => {
    if (streaming) {
      return;
    }
    const msgs = useConversationStore.getState().messages;
    const lastUserMessage = [...msgs]
      .reverse()
      .find((message) => message.role === "user" && message.status !== "error");
    if (!lastUserMessage?.content) {
      return;
    }
    setValue(lastUserMessage.content);
    hasUserResizedRef.current = false;
    requestAnimationFrame(() => {
      const textarea = textareaRef.current;
      if (!textarea) {
        return;
      }
      textarea.focus();
      textarea.style.height = "auto";
      const desired = Math.max(textarea.scrollHeight, userMinHeightRef.current);
      textarea.style.height = Math.min(desired, ABSOLUTE_MAX_HEIGHT) + "px";
    });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [messagesLength, streaming]);

  const handleCancel = useCallback(() => {
    cancelCurrentStream(activeConversationId ?? undefined);
  }, [cancelCurrentStream, activeConversationId]);

  const handleFileSelect = useCallback(() => {
    fileInputRef.current?.click();
  }, []);

  const handlePhotoSelect = useCallback(() => {
    photoInputRef.current?.click();
  }, []);

  const handleAudioSelect = useCallback(() => {
    audioInputRef.current?.click();
  }, []);

  const handleVideoSelect = useCallback(() => {
    videoInputRef.current?.click();
  }, []);

  const handleFileChange = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      const files = e.target.files;
      if (files) {
        setAttachedFiles((prev) => [...prev, ...Array.from(files)]);
      }
      if (fileInputRef.current) {
        fileInputRef.current.value = "";
      }
    },
    [],
  );

  const handlePhotoChange = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      const files = e.target.files;
      if (files) {
        setAttachedFiles((prev) => [...prev, ...Array.from(files)]);
      }
      if (photoInputRef.current) {
        photoInputRef.current.value = "";
      }
    },
    [],
  );

  const handleAudioChange = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      const files = e.target.files;
      if (files) {
        setAttachedFiles((prev) => [...prev, ...Array.from(files)]);
      }
      if (audioInputRef.current) {
        audioInputRef.current.value = "";
      }
    },
    [],
  );

  const handleVideoChange = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      const files = e.target.files;
      if (files) {
        setAttachedFiles((prev) => [...prev, ...Array.from(files)]);
      }
      if (videoInputRef.current) {
        videoInputRef.current.value = "";
      }
    },
    [],
  );

  const removeFile = useCallback((index: number) => {
    setAttachedFiles((prev) => prev.filter((_, i) => i !== index));
  }, []);

  const handlePaste = useCallback(
    (e: React.ClipboardEvent<HTMLTextAreaElement>) => {
      if (!hasVision) {
        return;
      }
      const items = e.clipboardData?.items;
      if (!items) {
        return;
      }
      const files: File[] = [];
      for (const item of items) {
        if (item.kind === "file") {
          const file = item.getAsFile();
          if (file) {
            files.push(file);
          }
        }
      }
      if (files.length > 0) {
        e.preventDefault();
        setAttachedFiles((prev) => [...prev, ...files]);
      }
    },
    [hasVision],
  );

  // Drag-and-drop overlay (Tauri native)
  const [isDragging, setIsDragging] = useState(false);
  const unlistenRef = useRef<(() => void) | null>(null);

  useEffect(() => {
    if (!hasVision) {
      return;
    }
    if (!isTauri()) {
      return; // Skip drag-drop in browser mode
    }

    (async () => {
      try {
        const { getCurrentWebview } = await import("@tauri-apps/api/webview");
        const { readFile } = await import("@tauri-apps/plugin-fs");

        const unlisten = await getCurrentWebview().onDragDropEvent(async (event) => {
          const { type } = event.payload;
          if (type === "enter") {
            setIsDragging(true);
          } else if (type === "leave") {
            setIsDragging(false);
          } else if (type === "drop") {
            setIsDragging(false);
            const { paths } = event.payload;
            const mimeMap: Record<string, string> = {
              png: "image/png",
              jpg: "image/jpeg",
              jpeg: "image/jpeg",
              gif: "image/gif",
              webp: "image/webp",
              svg: "image/svg+xml",
              bmp: "image/bmp",
              ico: "image/x-icon",
              pdf: "application/pdf",
              txt: "text/plain",
              json: "application/json",
              csv: "text/csv",
              md: "text/markdown",
              html: "text/html",
              js: "text/javascript",
              ts: "text/typescript",
              zip: "application/zip",
            };
            const fileResults = await Promise.all(
              paths.map(async (filePath) => {
                try {
                  const fileName = filePath.split(/[\\/]/).pop() || "file";
                  const ext = fileName.split(".").pop()?.toLowerCase() || "";
                  const mimeType = mimeMap[ext] || "application/octet-stream";
                  const bytes = await readFile(filePath);
                  const blob = new Blob([bytes], { type: mimeType });
                  return new globalThis.File([blob], fileName);
                } catch (err) {
                  logIpcError("drag-drop: read file")(err);
                  return null;
                }
              }),
            );
            const files = fileResults.filter((f): f is File => f !== null);
            if (files.length > 0) {
              setAttachedFiles((prev) => [...prev, ...files]);
            }
          }
        });
        unlistenRef.current = unlisten;
      } catch (error) {
        logIpcError("InputArea: setup drag-drop")(error);
      }
    })();

    return () => {
      unlistenRef.current?.();
      unlistenRef.current = null;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [hasVision, isTauri]);

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
      if (
        e.nativeEvent.isComposing
        || e.key === "Process"
        || e.keyCode === 229
      ) {
        return;
      }
      if (e.key === "Enter" && !e.shiftKey) {
        e.preventDefault();
        handleSend();
      }
    },
    [handleSend],
  );

  // Auto-resize textarea: height = max(userMinHeight, contentHeight), capped at ABSOLUTE_MAX
  // When user has explicitly dragged to resize, lock height to userMinHeight (content scrolls)
  /** Resize textarea to fit content. Uses refs instead of state to avoid re-render loop. */
  const autoResizeTextarea = useCallback((el: HTMLTextAreaElement) => {
    el.style.height = "auto";
    const desired = hasUserResizedRef.current
      ? userMinHeightRef.current
      : Math.max(el.scrollHeight, userMinHeightRef.current);
    el.style.height = Math.min(desired, ABSOLUTE_MAX_HEIGHT) + "px";
  }, []);

  const handleInput = useCallback(
    (e: React.ChangeEvent<HTMLTextAreaElement>) => {
      setValue(e.target.value);
      autoResizeTextarea(e.target);
    },
    [autoResizeTextarea],
  );

  // Drag-to-resize: changes userMinHeight so the textarea grows even with short content
  const handleResizeMouseDown = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    const textarea = textareaRef.current;
    const startHeight = textarea
      ? textarea.offsetHeight
      : userMinHeightRef.current;
    dragStateRef.current = { startY: e.clientY, startH: startHeight };
    const onMouseMove = (ev: MouseEvent) => {
      if (!dragStateRef.current) {
        return;
      }
      const delta = dragStateRef.current.startY - ev.clientY;
      const newH = Math.max(
        INITIAL_MIN_HEIGHT,
        Math.min(ABSOLUTE_MAX_HEIGHT, dragStateRef.current.startH + delta),
      );
      hasUserResizedRef.current = true;
      setUserMinHeight(newH);
      userMinHeightRef.current = newH;
      if (textarea) {
        textarea.style.height = newH + "px";
      }
    };
    const onMouseUp = () => {
      dragStateRef.current = null;
      document.removeEventListener("mousemove", onMouseMove);
      document.removeEventListener("mouseup", onMouseUp);
      document.body.style.cursor = "";
      document.body.style.userSelect = "";
    };
    document.addEventListener("mousemove", onMouseMove);
    document.addEventListener("mouseup", onMouseUp);
    document.body.style.cursor = "ns-resize";
    document.body.style.userSelect = "none";
  }, []);

  // Listen for Escape to close voice overlay
  // 加载 gateway API key 用于语音通话鉴权
  React.useEffect(() => {
    if (gatewayKeys.length === 0) {
      useGatewayStore.getState().fetchKeys();
    }
  }, [gatewayKeys.length]);

  // 稳定 messageApi / t 引用，避免其不稳定导致下方解密 useEffect 反复触发
  const messageApiRef = useRef(messageApi);
  messageApiRef.current = messageApi;
  const tRef = useRef(t);
  tRef.current = t;
  // 解密尝试标记：防止 decryptKey 失败后无限重试（避免 Maximum update depth exceeded）
  const decryptAttemptedRef = useRef(false);

  React.useEffect(() => {
    if (gatewayKeys.length === 0 || voiceApiKey || decryptAttemptedRef.current) {
      return;
    }
    decryptAttemptedRef.current = true;
    const enabledKey = gatewayKeys.find((k) => k.enabled) || gatewayKeys[0];
    // P1-10：解密失败时给用户可见的反馈，而不是静默吞错（之前 .catch(() => {}) 会让语音入口无任何提示直接失效）
    useGatewayStore.getState().decryptKey(enabledKey.id)
      .then(setVoiceApiKey)
      .catch(() => {
        // 仅在首次失败提示；用 ref 读取最新的 messageApi/t，避免把不稳定引用放入依赖
        messageApiRef.current.error(tRef.current("voice.decryptKeyFailed"));
      });
    // 依赖只用 length 和 voiceApiKey（稳定值）；messageApi/t 通过 ref 读取
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [gatewayKeys.length, voiceApiKey]);

  React.useEffect(() => {
    const onEscape = () => setVoiceCallVisible(false);
    window.addEventListener("axagent:escape", onEscape);
    return () => window.removeEventListener("axagent:escape", onEscape);
  }, []);

  React.useEffect(() => {
    const onFillLast = () => handleFillLastMessage();
    const onClearContext = () => {
      if (activeConversationId && !streaming) {
        void insertContextClear();
      }
    };
    const onClearConversation = () => {
      if (!activeConversationId || streaming || messagesLength === 0) {
        return;
      }
      modal.confirm({
        title: t("chat.clearConversationConfirmTitle"),
        content: t("chat.clearConversationConfirmContent"),
        okButtonProps: { danger: true },
        okText: t("common.confirm"),
        cancelText: t("common.cancel"),
        onOk: async () => {
          await clearAllMessages();
        },
      });
    };

    window.addEventListener("axagent:fill-last-message", onFillLast);
    window.addEventListener("axagent:clear-context", onClearContext);
    window.addEventListener(
      "axagent:clear-conversation-messages",
      onClearConversation,
    );
    return () => {
      window.removeEventListener("axagent:fill-last-message", onFillLast);
      window.removeEventListener("axagent:clear-context", onClearContext);
      window.removeEventListener(
        "axagent:clear-conversation-messages",
        onClearConversation,
      );
    };
  }, [
    activeConversationId,
    clearAllMessages,
    handleFillLastMessage,
    insertContextClear,
    messagesLength,
    modal,
    streaming,
    t,
  ]);

  // Listen for "fill input" events from GlobalCopyMenu
  React.useEffect(() => {
    const onFillInput = (e: Event) => {
      const text = (e as CustomEvent).detail;
      if (typeof text !== "string" || !text) {
        return;
      }
      setValue((prev) => (prev ? prev + "\n" + text : text));
      requestAnimationFrame(() => {
        const textarea = textareaRef.current;
        if (!textarea) {
          return;
        }
        textarea.focus();
        textarea.style.height = "auto";
        const desired = hasUserResizedRef.current
          ? userMinHeightRef.current
          : Math.max(textarea.scrollHeight, userMinHeightRef.current);
        textarea.style.height = Math.min(desired, ABSOLUTE_MAX_HEIGHT) + "px";
      });
    };
    window.addEventListener("axagent:fill-input", onFillInput);
    return () => window.removeEventListener("axagent:fill-input", onFillInput);
  }, []);

  // Listen for mode toggle shortcut
  React.useEffect(() => {
    const onToggleMode = () => {
      const nextMode = currentMode === "chat" ? "agent" : "chat";
      handleModeSwitch(nextMode);
    };
    window.addEventListener("axagent:toggle-mode", onToggleMode);
    return () => window.removeEventListener("axagent:toggle-mode", onToggleMode);
    // eslint-disable-next-line react-hooks/refs
  }, [currentMode, handleModeSwitch]);

  return (
    <div className="chat-input-area" data-tutorial="chat-input">
      <input
        ref={fileInputRef}
        type="file"
        multiple
        style={{ display: "none" }}
        onChange={handleFileChange}
        aria-label={t("input.uploadFile")}
      />
      <input
        ref={photoInputRef}
        type="file"
        accept="image/*"
        capture="environment"
        style={{ display: "none" }}
        onChange={handlePhotoChange}
        aria-label={t("input.takePhoto")}
      />
      <input
        ref={audioInputRef}
        type="file"
        accept="audio/*"
        capture
        style={{ display: "none" }}
        onChange={handleAudioChange}
        aria-label={t("input.recordAudio")}
      />
      <input
        ref={videoInputRef}
        type="file"
        accept="video/*"
        capture
        style={{ display: "none" }}
        onChange={handleVideoChange}
      />

      {/* Attachment preview */}
      <InputAreaFileList
        attachedFiles={attachedFiles}
        attachmentObjectUrls={attachmentObjectUrls}
        removeFile={removeFile}
        token={token}
      />

      {/* Main input container */}
      <div
        ref={containerRef}
        style={{
          position: "relative",
          overflow: "hidden",
        }}
      >
        {/* Drag-to-resize handle */}
        <div
          onMouseDown={handleResizeMouseDown}
          role="separator"
          aria-label={t("inputArea.resizeHandle")}
          tabIndex={0}
          onKeyDown={(e) => {
            if (e.key === "Enter" || e.key === " ") {
              e.preventDefault();
            }
          }}
          style={{
            height: 10,
            cursor: "ns-resize",
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            flexShrink: 0,
          }}
        >
          <GripHorizontal
            size={14}
            style={{ color: token.colorTextQuaternary, opacity: 0.5 }}
          />
        </div>
        {/* Companion model tags */}
        {currentMode !== "agent" && companionModels.length > 0 && (
          <CompanionModelTags
            infos={companionDisplayInfos}
            onRemove={removeCompanionModel}
            onClearAll={clearAllCompanionModels}
          />
        )}

        {/* 引用回复预览条：显示当前被引用的消息 */}
        {quotedMessage && (
          <QuotePreviewBar
            content={quotedMessage.content}
            onCancel={() => useUIStore.getState().setQuotedMessageId(null)}
          />
        )}

        {/* Textarea with command suggest */}
        <div className="chat-input-box">
          <CommandSuggest
            value={value}
            cursorPosition={cursorPosition}
            onSelect={(replacement) => {
              // Find the trigger position and replace from there
              const textBeforeCursor = value.slice(0, cursorPosition);
              const lastSlash = textBeforeCursor.lastIndexOf("/");
              const lastAt = textBeforeCursor.lastIndexOf("@");
              const triggerPos = Math.max(lastSlash, lastAt);
              if (triggerPos >= 0) {
                const before = value.slice(0, triggerPos);
                const after = value.slice(cursorPosition);
                const newValue = before + replacement + after;
                setValue(newValue);
                setShowSuggest(false);
                // Set cursor after replacement
                setTimeout(() => {
                  if (textareaRef.current) {
                    const newPos = triggerPos + replacement.length;
                    textareaRef.current.selectionStart = newPos;
                    textareaRef.current.selectionEnd = newPos;
                    textareaRef.current.focus();
                  }
                }, 0);
              }
            }}
            onExecute={async (actionId) => {
              setShowSuggest(false);
              switch (actionId) {
                case "compact":
                  try {
                    await compressContext();
                    messageApi.success(t("chat.compressSuccess"));
                  } catch {
                    messageApi.error(t("chat.compressFailed"));
                  }
                  break;
                case "clear":
                  await clearAllMessages();
                  messageApi.success(t("chat.clearHistoryDone"));
                  break;
                case "new": {
                  // 从当前对话继承模型/provider，或使用回退逻辑
                  const fallbackProviderId = activeConversation?.providerId
                    ?? settings.defaultProviderId
                    ?? "";
                  const fallbackModelId = activeConversation?.modelId
                    ?? settings.defaultModelId
                    ?? "";
                  await createConversation(
                    "",
                    fallbackModelId,
                    fallbackProviderId,
                    { mode: "chat" },
                  );
                  break;
                }
                case "stop":
                  cancelCurrentStream(activeConversationId ?? undefined);
                  break;
              }
            }}
            visible={showSuggest}
          />
          <textarea
            className="axagent-input-textarea"
            ref={textareaRef}
            data-testid="message-input"
            value={value}
            onChange={handleInput}
            onKeyDown={handleKeyDown}
            onPaste={handlePaste}
            placeholder={t("chat.inputPlaceholder")}
            rows={1}
            style={{
              color: token.colorText,
              minHeight: userMinHeight,
              maxHeight: ABSOLUTE_MAX_HEIGHT,
            }}
            onKeyUp={() => {
              if (textareaRef.current) {
                setCursorPosition(textareaRef.current.selectionStart);
                const textBefore = value.slice(
                  0,
                  textareaRef.current.selectionStart,
                );
                // 仅行首或空格后触发 / 和 @，且后面至少跟了 1 个非空白符（过滤裸符号和 URL）
                const atLineStart = textBefore === ""
                  || textBefore.endsWith(" ")
                  || textBefore.endsWith("\n");
                const hasActiveSlash = atLineStart && /\/\S{1,}$/.test(textBefore);
                const hasActiveAt = atLineStart && /@\S{1,}$/.test(textBefore);
                setShowSuggest(hasActiveSlash || hasActiveAt);
              }
            }}
            onClick={() => {
              if (textareaRef.current) {
                setCursorPosition(textareaRef.current.selectionStart);
              }
            }}
          />
          <SendControls
            streaming={streaming}
            hasContent={value.trim().length > 0}
            onSend={handleSend}
            onCancel={handleCancel}
          />
        </div>

        {/* Bottom action bar */}
        <div className="chat-input-tools">
          <div className="flex items-center gap-0.5">
            <SkillToolbar />
            {searchEnabled
              ? (
                <Tooltip title={t("chat.search.title")}>
                  <Button
                    type="text"
                    size="small"
                    icon={<Globe size={14} />}
                    style={{ color: token.colorPrimary }}
                    onClick={() => {
                      setSearchEnabled(false);
                      setSearchProviderId(null);
                    }}
                  />
                </Tooltip>
              )
              : (
                <DropdownMenu
                  items={searchMenuItems}
                  open={searchDropdownOpen}
                  onOpenChange={setSearchDropdownOpen}
                >
                  <Button
                    type="text"
                    size="small"
                    icon={<Globe size={14} />}
                    style={searchEnabled ? { color: token.colorPrimary } : undefined}
                    onClick={() => setSearchDropdownOpen((p) => !p)}
                  />
                </DropdownMenu>
              )}
            <ThinkingMenu hasReasoning={hasReasoning} />
            {hasVision && (
              <DropdownMenu
                items={[
                  {
                    key: "file",
                    icon: <Paperclip size={14} />,
                    label: t("chat.attachFile"),
                    onClick: handleFileSelect,
                  },
                  {
                    key: "photo",
                    icon: <ImageIcon size={14} />,
                    label: t("chat.takePhoto"),
                    onClick: handlePhotoSelect,
                  },
                  {
                    key: "audio",
                    icon: <Mic size={14} />,
                    label: t("chat.recordAudio"),
                    onClick: handleAudioSelect,
                  },
                  {
                    key: "video",
                    icon: <Film size={14} />,
                    label: t("chat.recordVideo"),
                    onClick: handleVideoSelect,
                  },
                ]}
              >
                <Tooltip title={t("chat.attachFile")}>
                  <Button type="text" size="small" icon={<Paperclip size={14} />} />
                </Tooltip>
              </DropdownMenu>
            )}
            <McpMenu />
            <ContextSourceMenu />
            <Popover
              trigger="click"
              placement="topLeft"
              content={templatePopoverContent}
              arrow={false}
              open={templatePopoverOpen}
              onOpenChange={setTemplatePopoverOpen}
            >
              <Tooltip
                title={t("promptTemplates.title")}
                open={templatePopoverOpen ? false : undefined}
              >
                <Button
                  type="text"
                  size="small"
                  icon={<FileText size={14} />}
                  style={{
                    color: templatePopoverOpen ? token.colorPrimary : undefined,
                  }}
                />
              </Tooltip>
            </Popover>
            {currentMode !== "agent" && (
              <Tooltip title={t("chat.multiModel.selectTitle")}>
                <Button
                  type="text"
                  size="small"
                  icon={<GitCompareArrows size={14} />}
                  onClick={() => setMultiModelOpen(true)}
                  style={companionModels.length > 0
                    ? { color: token.colorPrimary }
                    : undefined}
                />
              </Tooltip>
            )}
            <DropdownMenu
              items={[
                {
                  key: "auto",
                  icon: activeConversation?.contextCompression ? <ZapOff size={14} /> : <Zap size={14} />,
                  label: activeConversation?.contextCompression
                    ? t("chat.disableAutoCompression")
                    : t("chat.enableAutoCompression"),
                  onClick: () => {
                    if (!activeConversationId || !activeConversation) {
                      return;
                    }
                    updateConversation(activeConversationId, {
                      contextCompression: !activeConversation.contextCompression,
                    });
                  },
                },
                {
                  key: "manual",
                  icon: <Shrink size={14} />,
                  label: t("chat.manualCompress"),
                  disabled: !activeConversationId
                    || streaming
                    || compressing
                    || messagesLength === 0,
                  onClick: async () => {
                    if (!activeConversationId) {
                      return;
                    }
                    try {
                      await compressContext();
                      messageApi.success(t("chat.compressSuccess"));
                    } catch {
                      messageApi.error(t("chat.compressFailed"));
                    }
                  },
                },
              ]}
            >
              <Tooltip title={t("chat.contextCompression")}>
                <Button
                  type="text"
                  size="small"
                  icon={<Zap size={14} />}
                  loading={compressing}
                  disabled={!activeConversationId}
                  style={activeConversation?.contextCompression
                    ? { color: token.colorPrimary }
                    : undefined}
                />
              </Tooltip>
            </DropdownMenu>
            <Tooltip
              title={shortcutHint(t("chat.clearContext"), "clearContext")}
            >
              <Button
                type="text"
                size="small"
                icon={<Scissors size={14} />}
                onClick={insertContextClear}
                disabled={!activeConversationId
                  || streaming
                  || messagesLength === 0
                  || useConversationStore.getState().messages[messagesLength - 1]
                      ?.content === "<!-- context-clear -->"}
              />
            </Tooltip>
            <Tooltip
              title={shortcutHint(
                t("chat.clearConversation"),
                "clearConversationMessages",
              )}
            >
              <Button
                type="text"
                size="small"
                icon={<Eraser size={14} />}
                onClick={() => {
                  if (!activeConversationId) {
                    return;
                  }
                  modal.confirm({
                    title: t("chat.clearConversationConfirmTitle"),
                    content: t("chat.clearConversationConfirmContent"),
                    okButtonProps: { danger: true },
                    okText: t("common.confirm"),
                    cancelText: t("common.cancel"),
                    onOk: async () => {
                      await clearAllMessages();
                    },
                  });
                }}
                disabled={!activeConversationId || streaming || messagesLength === 0}
              />
            </Tooltip>
            <Tooltip title={t("chat.conversationSettings")}>
              <Button
                type="text"
                size="small"
                icon={<SlidersHorizontal size={14} />}
                onClick={() => setSettingsOpen(true)}
              />
            </Tooltip>
            {currentMode === "agent" && (
              <Tooltip title={t("multiAgent.delegateBtn")}>
                <Button
                  type="text"
                  size="small"
                  icon={<TeamOutlined style={{ fontSize: 14 }} />}
                  onClick={() => setDelegateModalOpen(true)}
                />
              </Tooltip>
            )}
            <GatewayMenu onSelect={setSelectedGatewayId} />
            {currentMode === "agent" && activeConversationId && (
              <PlanHistoryPanel conversationId={activeConversationId} />
            )}
            {currentMode === "agent" && (
              <WorkspaceDirMenu
                cwd={agentCwd}
                disabled={messagesLength > 0}
                onSelect={handleSelectCwd}
                onOpen={async (cwd) => {
                  try {
                    const { revealItemInDir } = await import("@tauri-apps/plugin-opener");
                    await revealItemInDir(cwd);
                  } catch (e) {
                    logIpcError("open directory")(e);
                  }
                }}
              />
            )}
            {voiceAvailable && (
              <>
                <Tooltip title={voiceWakeup.active ? t("voice.wakeupActive") : t("voice.wakeup")}>
                  <Button
                    type="text"
                    size="small"
                    icon={
                      <span className={voiceWakeup.active ? "animate-pulse" : ""}>
                        <AudioOutlined style={{ fontSize: 14 }} />
                      </span>
                    }
                    onClick={() => (voiceWakeup.active ? voiceWakeup.stop() : voiceWakeup.start())}
                    style={voiceWakeup.active
                      ? { color: token.colorPrimary, background: token.colorPrimaryBg }
                      : undefined}
                  />
                </Tooltip>
                <Tooltip title={t("voice.startCall")}>
                  <Button
                    type="text"
                    size="small"
                    icon={<Mic size={14} />}
                    onClick={() => setVoiceCallVisible(true)}
                  />
                </Tooltip>
              </>
            )}
          </div>
          <div className="flex items-center gap-2 ml-auto">
            {currentMode === "agent" && (
              <PermissionMenu
                permissionMode={agentPermissionMode}
                onChange={handlePermissionModeChange}
              />
            )}
            {contextTokenUsage
              ? (() => {
                const r = 8,
                  stroke = 2.5,
                  size = (r + stroke) * 2;
                const circ = 2 * Math.PI * r;
                const offset = circ * (1 - contextTokenUsage.percent / 100);
                const color = contextTokenUsage.percent > 80
                  ? token.colorError
                  : contextTokenUsage.percent > 60
                  ? token.colorWarning
                  : token.colorPrimary;
                return (
                  <Popover
                    content={
                      <span style={{ fontSize: 12 }}>
                        {contextTokenUsage.isEstimate && "~"}
                        {contextTokenUsage.usedTokens.toLocaleString()} / {contextTokenUsage.maxTokens.toLocaleString()}
                        {" "}
                        tokens ({contextTokenUsage.percent}%)
                        {contextCount > 0 && (
                          <>
                            {" · "}
                            {contextCount} {t("chat.contextMessages")}
                          </>
                        )}
                      </span>
                    }
                  >
                    <svg
                      width={size}
                      height={size}
                      style={{ display: "block", cursor: "pointer" }}
                    >
                      <circle
                        cx={r + stroke}
                        cy={r + stroke}
                        r={r}
                        fill="none"
                        stroke={token.colorBorderSecondary}
                        strokeWidth={stroke}
                      />
                      <circle
                        cx={r + stroke}
                        cy={r + stroke}
                        r={r}
                        fill="none"
                        stroke={color}
                        strokeWidth={stroke}
                        strokeDasharray={circ}
                        strokeDashoffset={offset}
                        strokeLinecap="round"
                        transform={`rotate(-90 ${r + stroke} ${r + stroke})`}
                      />
                    </svg>
                  </Popover>
                );
              })()
              : contextCount > 0
              ? (
                <span style={{ fontSize: 12, color: token.colorTextSecondary }}>
                  {contextCount} {t("chat.contextMessages")}
                </span>
              )
              : null}
          </div>
        </div>
      </div>
      <ConversationSettingsModal
        open={settingsOpen}
        onClose={() => setSettingsOpen(false)}
      />

      <DelegateTaskModal
        open={delegateModalOpen}
        onClose={() => setDelegateModalOpen(false)}
        initialTask={value}
      />

      {/* ModelRoutingConfigPanel removed */}

      {voiceAvailable && (
        <VoiceCall
          visible={voiceCallVisible}
          onClose={() => setVoiceCallVisible(false)}
          config={voiceConfig}
          apiKey={voiceApiKey}
        />
      )}

      {/* Drag-and-drop overlay */}
      {isDragging && (
        <div
          style={{
            position: "fixed",
            inset: 0,
            zIndex: "var(--z-modal)",
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            backgroundColor: token.colorBgMask,
            backdropFilter: "blur(4px)",
          }}
        >
          <div
            style={{
              display: "flex",
              flexDirection: "column",
              alignItems: "center",
              gap: 12,
              padding: "40px 60px",
              borderRadius: 16,
              border: `2px dashed ${token.colorPrimary}`,
              backgroundColor: token.colorBgElevated,
            }}
          >
            <Upload size={48} style={{ color: token.colorPrimary }} />
            <span
              style={{ fontSize: 16, fontWeight: 500, color: token.colorText }}
            >
              {t("chat.dropToAttach")}
            </span>
          </div>
        </div>
      )}

      {/* Multi-model selector (trigger hidden, controlled via multiModelOpen state) */}
      <ModelSelector
        multiSelect
        open={multiModelOpen}
        onOpenChange={setMultiModelOpen}
        onMultiSelect={handleMultiModelSelect}
        defaultSelectedModels={companionModels}
      >
        <span />
      </ModelSelector>
    </div>
  );
}

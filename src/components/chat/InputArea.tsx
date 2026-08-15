// SPDX-License-Identifier: AGPL-3.0-only

import { DropdownMenu } from "@/components/layout/DropdownMenu";
import type { DropdownItem } from "@/components/layout/DropdownMenu";
import { Tooltip } from "@/components/layout/Tooltip";
import { McpServerIcon } from "@/components/shared/McpServerIcon";
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
import { useMultiAgentStore } from "@/stores/feature/multiAgentStore";
import { usePromptTemplateStore } from "@/stores/feature/promptTemplateStore";
import type { PromptTemplate } from "@/types";
import { type AttachmentInput, type CreateMcpServerInput, type McpServer, type RealtimeConfig } from "@/types";
import { AudioOutlined, TeamOutlined } from "@ant-design/icons";
import { ModelIcon } from "@lobehub/icons";
import { open } from "@tauri-apps/plugin-dialog";
import {
  App,
  Badge,
  Button,
  Checkbox,
  Form,
  Image,
  Input,
  Modal,
  Popover,
  Segmented,
  Select,
  Space,
  Tag,
  theme,
  Typography,
} from "antd";
import {
  ArrowUp,
  Atom,
  Check,
  CircleOff,
  Database,
  Eraser,
  ExternalLink,
  File,
  FileText,
  Film,
  FolderOpen,
  GitCompareArrows,
  Globe,
  GripHorizontal,
  Image as ImageIcon,
  MessageSquare,
  Mic,
  Music,
  Paperclip,
  Plug,
  Scissors,
  Shield,
  ShieldAlert,
  ShieldCheck,
  Shrink,
  Signal,
  SignalHigh,
  SignalLow,
  SignalMedium,
  SlidersHorizontal,
  Square,
  Trash2,
  Upload,
  X,
  Zap,
  ZapOff,
} from "lucide-react";
import React, { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router-dom";
import { CommandSuggest } from "./CommandSuggest";
import { ConversationSettingsModal } from "./ConversationSettingsModal";
import { ModelSelector } from "./ModelSelector";
import { PlanHistoryPanel } from "./PlanHistoryPanel";
import { PromptTemplateSelector } from "./PromptTemplateSelector";
import { SourcePickerPanel } from "./SourcePickerPanel";
import { VoiceCall } from "./VoiceCall";

async function fileToAttachmentInput(file: File): Promise<AttachmentInput> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => {
      const base64 = (reader.result as string).split(",")[1] || "";
      resolve({
        file_name: file.name,
        file_type: file.type || "application/octet-stream",
        file_size: file.size,
        data: base64,
      });
    };
    reader.onerror = () => {
      reject(new Error(`Failed to read file: ${file.name}`));
    };
    reader.readAsDataURL(file);
  });
}

type FileTypeCategory = "image" | "video" | "audio" | "document" | "other";

function getFileTypeCategory(mimeType: string): FileTypeCategory {
  if (mimeType.startsWith("image/")) {
    return "image";
  }
  if (mimeType.startsWith("video/")) {
    return "video";
  }
  if (mimeType.startsWith("audio/")) {
    return "audio";
  }
  if (
    mimeType.startsWith("text/")
    || mimeType === "application/pdf"
    || mimeType.includes("document")
    || mimeType.includes("spreadsheet")
    || mimeType.includes("presentation")
    || mimeType.includes("word")
  ) {
    return "document";
  }
  return "other";
}

function formatFileSize(bytes: number): string {
  if (bytes === 0) {
    return "0 B";
  }
  const k = 1024;
  const sizes = ["B", "KB", "MB", "GB"];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${parseFloat((bytes / Math.pow(k, i)).toFixed(1))} ${sizes[i]}`;
}

function getFileIcon(category: FileTypeCategory) {
  switch (category) {
    case "image":
      return <ImageIcon size={16} />;
    case "video":
      return <Film size={16} />;
    case "audio":
      return <Music size={16} />;
    case "document":
      return <FileText size={16} />;
    default:
      return <File size={16} />;
  }
}

// In-memory draft cache: persists input text per-conversation across component unmounts
const _draftCache = new Map<string, string>();
// Cache is module-level, conversation switch clears by key mismatch

export function AgentProfileSelect({
  value,
  onChange,
}: {
  value: string;
  onChange: (profileId: string) => void;
}) {
  const { t } = useTranslation();
  const [profiles, setProfiles] = useState<{ id: string; name: string }[]>([]);

  useEffect(() => {
    invoke<{ id: string; name: string }[]>("list_agent_profiles")
      .then(setProfiles)
      .catch(logIpcError("AgentProfileSelect: load profiles"));
  }, []);

  return (
    <Select
      size="small"
      style={{ minWidth: 120 }}
      value={value || undefined}
      onChange={(v) => onChange(v)}
      placeholder={t("chat.workflow.agentProfileRole")}
      options={profiles.map((p) => ({ value: p.id, label: p.name }))}
      allowClear
    />
  );
}

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
  // const [modelRoutingOpen, setModelRoutingOpen] = useState(false); // removed
  const [mcpPopoverOpen, setMcpPopoverOpen] = useState(false);
  const [connectorModalOpen, setConnectorModalOpen] = useState(false);
  const [editingMcpServer, setEditingMcpServer] = useState<McpServer | null>(null);
  const [mcpForm] = Form.useForm();

  // 连接器 modal 打开时设置初始值
  useEffect(() => {
    if (connectorModalOpen) {
      if (editingMcpServer) {
        mcpForm.setFieldsValue({
          name: editingMcpServer.name,
          transport: editingMcpServer.transport,
          command: editingMcpServer.command || "",
          args: editingMcpServer.argsJson
            ? editingMcpServer.argsJson.split(/\s+/).filter(Boolean).join(" ")
            : "",
          endpoint: editingMcpServer.endpoint || "",
        });
      } else {
        mcpForm.resetFields();
      }
    }
  }, [connectorModalOpen, editingMcpServer, mcpForm]);

  const [searchDropdownOpen, setSearchDropdownOpen] = useState(false);
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
    Array<{ providerId: string; model_id: string }>
  >([]);
  const [multiModelOpen, setMultiModelOpen] = useState(false);
  const sendMultiModelMessage = useConversationStore(
    (s) => s.sendMultiModelMessage,
  );

  const multiAgentStore = useMultiAgentStore();
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
    const activeMessages = msgs.filter((m) => m.is_active !== false);
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

  // MCP state
  const mcpServers = useMcpStore((s) => s.servers);
  const createMcpServer = useMcpStore((s) => s.createServer);
  const updateMcpServer = useMcpStore((s) => s.updateServer);
  const enabledMcpServerIds = useConversationStore(
    (s) => s.enabledMcpServerIds,
  );
  const toggleMcpServer = useConversationStore((s) => s.toggleMcpServer);
  const mcpMode = useConversationStore((s) => s.mcpMode);
  const setMcpMode = useConversationStore((s) => s.setMcpMode);

  // Thinking state
  const thinkingBudget = useConversationStore((s) => s.thinkingBudget);
  const setThinkingBudget = useConversationStore((s) => s.setThinkingBudget);
  const [thinkingDropdownOpen, setThinkingDropdownOpen] = useState(false);

  // Agent permission mode state
  const [agentPermissionMode, setAgentPermissionMode] = useState<string>("default");

  // Agent working directory state
  const [agentCwd, setAgentCwd] = useState<string | null>(null);

  // Gateway links state
  const gatewayLinks = useGatewayLinkStore((s) => s.links);
  const [selectedGatewayId, setSelectedGatewayId] = useState<string | null>(
    null,
  );

  // Gateway 链接选择（独立入口）：认知编排器模式下不再提供 act/plan/ask/auto 手动模式选择
  const gatewayMenuItems = useMemo((): DropdownItem[] => {
    const connectedGateways = gatewayLinks.filter(
      (l) => l.enabled && l.status === "connected",
    );
    return connectedGateways.map((gw) => ({
      key: `gateway:${gw.id}`,
      icon: <Globe size={14} />,
      label: gw.name,
      onClick: () => setSelectedGatewayId(gw.id),
    }));
  }, [gatewayLinks]);

  // Knowledge base state
  const knowledgeBases = useKnowledgeStore((s) => s.bases);
  const enabledKnowledgeBaseIds = useConversationStore(
    (s) => s.enabledKnowledgeBaseIds,
  );
  const toggleKnowledgeBase = useConversationStore(
    (s) => s.toggleKnowledgeBase,
  );
  const [sourceModalOpen, setSourceModalOpen] = useState(false);

  // Memory state
  const memoryNamespaces = useMemoryStore((s) => s.namespaces);
  const activeMemoryNamespaceId = useConversationStore(
    (s) => s.activeMemoryNamespaceId,
  );
  const setActiveMemoryNamespace = useConversationStore(
    (s) => s.setActiveMemoryNamespace,
  );

  // Wiki vault state
  const wikis = useLlmWikiStore((s) => s.wikis);
  const enabledWikiIds = useConversationStore((s) => s.enabledWikiIds);
  const toggleWiki = useConversationStore((s) => s.toggleWiki);

  // Prompt template state
  const [templatePopoverOpen, setTemplatePopoverOpen] = useState(false);

  // Delegate task state
  const [delegateModalOpen, setDelegateModalOpen] = useState(false);
  const [delegateRole, setDelegateRole] = useState("");
  const [delegateTask, setDelegateTask] = useState("");

  // Fetch roles when delegate modal opens
  useEffect(() => {
    if (delegateModalOpen) {
      if (multiAgentStore.roles.length === 0) {
        multiAgentStore.fetchRoles();
      }
      if (!delegateRole && multiAgentStore.roles.length > 0) {
        setDelegateRole(multiAgentStore.roles[0].id);
      }
    }
  }, [delegateModalOpen]);

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

  const navigate = useNavigate();
  const setSettingsSection = useUIStore((s) => s.setSettingsSection);
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
      && settings.default_workspace_dir
    ) {
      // eslint-disable-next-line react-hooks/set-state-in-effect
      setAgentCwd(settings.default_workspace_dir);
    }
  }, [activeConversationId, currentMode, settings.default_workspace_dir]);

  // Fetch agent permission mode on mount/conversation switch
  useEffect(() => {
    if (currentMode === "agent" && activeConversationId) {
      invoke("agent_get_session", {
        request: { conversationId: activeConversationId },
      })
        .then((session: unknown) => {
          const s = session as { permission_mode?: string; cwd?: string | null } | null;
          if (s) {
            setAgentPermissionMode(s.permission_mode || "default");
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

  // MCP popover content — mode selector + checkboxes with alias/description
  const mcpPopoverContent = useMemo(() => {
    const enabledServers = mcpServers.filter((s) => s.enabled);
    if (enabledServers.length === 0) {
      return (
        <div style={{ padding: "8px 0", minWidth: 220 }}>
          <div
            style={{
              color: token.colorTextSecondary,
              fontSize: 12,
              marginBottom: 8,
            }}
          >
            {t("chat.connector.noServers")}
          </div>
          <Button
            type="link"
            size="small"
            style={{ padding: 0, fontSize: 12 }}
            onClick={() => {
              setMcpPopoverOpen(false);
              setSettingsSection("mcpServers");
              navigate("/settings");
            }}
          >
            {t("chat.connector.goConfig")}
          </Button>
        </div>
      );
    }

    const builtinServers = enabledServers.filter((s) => s.source === "builtin");
    const customServers = enabledServers.filter((s) => s.source === "custom");
    const isManual = mcpMode === "manual";

    const renderGroup = (title: string, servers: typeof mcpServers) => (
      <div key={title}>
        <div
          style={{
            fontSize: 12,
            color: token.colorTextSecondary,
            padding: "4px 0",
            fontWeight: 600,
          }}
        >
          {title}
        </div>
        {servers.map((server) => (
          <div key={server.id} style={{ padding: "3px 0" }}>
            <Checkbox
              checked={enabledMcpServerIds.includes(server.id)}
              disabled={!isManual}
              onChange={() => toggleMcpServer(server.id)}
            >
              <span
                style={{
                  fontSize: 13,
                  display: "inline-flex",
                  alignItems: "center",
                  gap: 6,
                }}
              >
                <McpServerIcon server={server} size={18} />
                <span>
                  <span style={{ fontWeight: 500 }}>
                    {server.alias || server.name}
                  </span>
                  {server.description && (
                    <span
                      style={{
                        display: "block",
                        fontSize: 12,
                        color: token.colorTextSecondary,
                        lineHeight: "16px",
                      }}
                    >
                      {server.description}
                    </span>
                  )}
                  {server.alias && (
                    <span
                      style={{
                        display: "block",
                        fontSize: 10,
                        color: token.colorTextQuaternary,
                        lineHeight: "14px",
                      }}
                    >
                      {server.name}
                    </span>
                  )}
                </span>
              </span>
            </Checkbox>
          </div>
        ))}
      </div>
    );

    return (
      <div
        style={{
          minWidth: 260,
          maxHeight: 360,
          overflowY: "auto",
          padding: "4px 0",
        }}
      >
        {/* Mode selector */}
        <div
          style={{
            padding: "4px 0 8px",
            borderBottom: `1px solid ${token.colorBorderSecondary}`,
            marginBottom: 8,
          }}
        >
          <div
            style={{
              fontSize: 12,
              color: token.colorTextSecondary,
              marginBottom: 6,
            }}
          >
            {t("chat.mcp.mode")}
          </div>
          <div style={{ display: "flex", gap: 4 }}>
            {(["auto", "manual", "disabled"] as const).map((mode) => (
              <Button
                key={mode}
                size="small"
                type={mcpMode === mode ? "primary" : "default"}
                onClick={() => setMcpMode(mode)}
                style={{ flex: 1, fontSize: 12 }}
              >
                {mode === "auto"
                  ? t("chat.mcp.modeAuto")
                  : mode === "manual"
                  ? t("chat.mcp.modeManual")
                  : t("chat.mcp.modeDisabled")}
              </Button>
            ))}
          </div>
          <div
            style={{
              fontSize: 10,
              color: token.colorTextQuaternary,
              marginTop: 4,
            }}
          >
            {mcpMode === "auto"
              ? t("chat.mcp.modeAutoDesc")
              : mcpMode === "manual"
              ? t("chat.mcp.modeManualDesc")
              : t("chat.mcp.modeDisabledDesc")}
          </div>
        </div>
        {builtinServers.length > 0
          && renderGroup(t("settings.mcp.builtin"), builtinServers)}
        {builtinServers.length > 0 && customServers.length > 0 && (
          <div
            style={{
              borderTop: `1px solid ${token.colorBorderSecondary}`,
              margin: "6px 0",
            }}
          />
        )}
        {customServers.length > 0
          && renderGroup(t("settings.mcp.custom"), customServers)}
        <div
          style={{
            marginTop: 12,
            borderTop: `1px solid ${token.colorBorderSecondary}`,
            paddingTop: 8,
            display: "flex",
            gap: 8,
          }}
        >
          <Button
            type="link"
            size="small"
            style={{ padding: 0, fontSize: 12 }}
            onClick={() => {
              setMcpPopoverOpen(false);
              setEditingMcpServer(null);
              setConnectorModalOpen(true);
            }}
          >
            {t("chat.connector.add")}
          </Button>
          <Button
            type="link"
            size="small"
            style={{ padding: 0, fontSize: 12 }}
            onClick={() => {
              setMcpPopoverOpen(false);
              const customServer = customServers.length > 0 ? customServers[0] : null;
              setEditingMcpServer(customServer);
              setConnectorModalOpen(true);
            }}
          >
            {t("chat.connector.custom")}
          </Button>
        </div>
      </div>
    );
  }, [
    mcpServers,
    enabledMcpServerIds,
    toggleMcpServer,
    mcpMode,
    setMcpMode,
    navigate,
    setSettingsSection,
    token,
    t,
  ]);

  const thinkingOptions = useMemo(
    () => [
      { key: "default", label: t("chat.thinking.default"), value: null },
      { key: "none", label: t("chat.thinking.none"), value: 0 },
      { key: "low", label: t("chat.thinking.low"), value: 1024 },
      { key: "medium", label: t("chat.thinking.medium"), value: 4096 },
      { key: "high", label: t("chat.thinking.high"), value: 8192 },
      { key: "xhigh", label: t("chat.thinking.xhigh"), value: 16384 },
    ],
    [t],
  );

  const selectedThinkingOption = useMemo(
    () =>
      thinkingOptions.find((opt) => opt.value === thinkingBudget)
        ?? thinkingOptions[0],
    [thinkingBudget, thinkingOptions],
  );

  const thinkingIcon = useMemo(() => {
    switch (selectedThinkingOption.key) {
      case "none":
        return <CircleOff size={14} />;
      case "low":
        return <SignalLow size={14} />;
      case "medium":
        return <SignalMedium size={14} />;
      case "high":
        return <SignalHigh size={14} />;
      case "xhigh":
        return <Signal size={14} />;
      default:
        return <Atom size={14} />;
    }
  }, [selectedThinkingOption.key]);

  const thinkingMenuItems = useMemo((): DropdownItem[] =>
    thinkingOptions.map((opt) => ({
      key: opt.key,
      label: opt.label,
      icon: (() => {
        switch (opt.key) {
          case "none":
            return <CircleOff size={14} />;
          case "default":
            return <Atom size={14} />;
          case "low":
            return <SignalLow size={14} />;
          case "medium":
            return <SignalMedium size={14} />;
          case "high":
            return <SignalHigh size={14} />;
          case "xhigh":
            return <Signal size={14} />;
          default:
            return <Atom size={14} />;
        }
      })(),
      onClick: () => handleThinkingSelect(opt.key),
      // eslint-disable-next-line react-hooks/exhaustive-deps
    })), [thinkingOptions]);

  const handleThinkingSelect = useCallback(
    (key: string) => {
      const selected = thinkingOptions.find((opt) => opt.key === key);
      if (selected) {
        setThinkingBudget(selected.value);
        setThinkingDropdownOpen(false);
      }
    },
    [setThinkingBudget, thinkingOptions, setThinkingDropdownOpen],
  );

  // 认知编排器模式下专家/角色由路由自动选择，不再提供手动专家选择

  // Agent permission mode menu items
  const permissionModeItems = useMemo((): DropdownItem[] => [
    {
      key: "default",
      label: (
        <span className="flex items-center gap-2">
          {t("common.permissionDefault")}
          {agentPermissionMode === "default" && <Check size={14} style={{ color: token.colorPrimary }} />}
        </span>
      ),
      icon: <Shield size={14} />,
      onClick: () => handlePermissionModeChange("default"),
    },
    {
      key: "accept_edits",
      label: (
        <span className="flex items-center gap-2">
          {t("common.permissionAcceptEdits")}
          {agentPermissionMode === "accept_edits" && <Check size={14} style={{ color: token.colorPrimary }} />}
        </span>
      ),
      icon: <ShieldCheck size={14} style={{ color: token.colorPrimary }} />,
      onClick: () => handlePermissionModeChange("accept_edits"),
    },
    {
      key: "full_access",
      label: (
        <span className="flex items-center gap-2">
          {t("common.permissionFullAccess")}
          {agentPermissionMode === "full_access" && <Check size={14} style={{ color: token.colorError }} />}
        </span>
      ),
      icon: <ShieldAlert size={14} style={{ color: token.colorError }} />,
      onClick: () => handlePermissionModeChange("full_access"),
    },
    // eslint-disable-next-line react-hooks/exhaustive-deps
  ], [t, agentPermissionMode, token.colorPrimary, token.colorError]);

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

  const permissionModeIcon = useMemo(() => {
    switch (agentPermissionMode) {
      case "accept_edits":
        return <ShieldCheck size={14} style={{ color: token.colorPrimary }} />;
      case "full_access":
        return <ShieldAlert size={14} style={{ color: token.colorError }} />;
      default:
        return <Shield size={14} />;
    }
  }, [agentPermissionMode, token.colorPrimary, token.colorError]);

  const permissionModeLabel = useMemo(() => {
    switch (agentPermissionMode) {
      case "accept_edits":
        return t("common.permissionAcceptEdits");
      case "full_access":
        return t("common.permissionFullAccess");
      default:
        return t("common.permissionDefault");
    }
  }, [agentPermissionMode, t]);

  // ── Work Strategy ──────────────────────────────────────────────────
  // 认知编排器模式下工作策略由路由自动决策，不再提供手动直接/计划切换

  // Agent CWD helpers
  const abbreviatePath = useCallback((path: string): string => {
    const normalized = path.replace(/\\/g, "/");
    const segments = normalized.split("/").filter(Boolean);
    if (segments.length <= 3 || normalized.length <= 45) {
      return path;
    }
    // 保留盘符（如 D:）+ 最后 3 段
    const drive = segments[0].endsWith(":") ? segments[0] : null;
    const tail = segments.slice(-3);
    const abbreviated = drive
      ? [drive, "…", ...tail].join("/")
      : "…/" + tail.join("/");
    return abbreviated;
  }, []);

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

  const sourcePopoverContent = useMemo(() => {
    const safeKb = knowledgeBases ?? [];
    const safeMem = memoryNamespaces ?? [];
    const safeWikis = wikis ?? [];
    const totalSources = safeKb.length + safeMem.length + safeWikis.length;
    if (totalSources === 0) {
      return (
        <div style={{ padding: "8px 0", minWidth: 200 }}>
          <div
            style={{
              color: token.colorTextSecondary,
              fontSize: 12,
              marginBottom: 8,
            }}
          >
            {t("chat.sources.empty")}
          </div>
          <Button
            type="link"
            size="small"
            style={{ padding: 0, fontSize: 12 }}
            onClick={() => {
              setSourceModalOpen(false);
              navigate("/knowledge");
            }}
          >
            {t("chat.connector.goConfig")}
          </Button>
        </div>
      );
    }
    return (
      <SourcePickerPanel
        conversationId={activeConversationId}
        knowledgeBases={safeKb}
        memoryNamespaces={safeMem}
        wikis={safeWikis}
        enabledKnowledgeBaseIds={enabledKnowledgeBaseIds}
        activeMemoryNamespaceId={activeMemoryNamespaceId}
        enabledWikiIds={enabledWikiIds}
        onToggleKb={toggleKnowledgeBase}
        onSetActiveMemory={setActiveMemoryNamespace}
        onToggleWiki={toggleWiki}
        onGoConfig={() => {
          setSourceModalOpen(false);
          navigate("/knowledge");
        }}
      />
    );
  }, [
    knowledgeBases,
    memoryNamespaces,
    wikis,
    enabledKnowledgeBaseIds,
    activeMemoryNamespaceId,
    enabledWikiIds,
    toggleKnowledgeBase,
    setActiveMemoryNamespace,
    toggleWiki,
    token,
    t,
    navigate,
    activeConversationId,
  ]);

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
        activeConversation.provider_id,
        activeConversation.model_id,
      );
    }

    if (settings.default_provider_id && settings.default_model_id) {
      const defaultModel = findModelByIds(
        providers,
        settings.default_provider_id,
        settings.default_model_id,
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
    settings.default_provider_id,
    settings.default_model_id,
  ]);

  // Context token usage calculation
  const getCompressionSummary = useCompressStore(
    (s) => s.getCompressionSummary,
  );
  const [summaryTokenCount, setSummaryTokenCount] = useState<number>(0);

  useEffect(() => {
    if (!activeConversationId || !activeConversation?.context_compression) {
      // eslint-disable-next-line react-hooks/set-state-in-effect
      setSummaryTokenCount(0);
      return;
    }
    getCompressionSummary(activeConversationId).then((s) => {
      setSummaryTokenCount(s?.token_count ?? 0);
    });
  }, [
    activeConversationId,
    activeConversation?.context_compression,
    getCompressionSummary,
    messagesLength,
  ]);

  const contextTokenUsage = useMemo(() => {
    const maxTokens = currentModel?.max_tokens;
    if (!maxTokens) {
      return null;
    }

    const msgs = useConversationStore.getState().messages;
    const activeMessages = msgs.filter((m) => m.is_active !== false);
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

    if (activeConversation?.system_prompt) {
      usedTokens += estimateTokens(activeConversation.system_prompt) + 4;
    }

    usedTokens += summaryTokenCount;

    const isEstimate = hasOlderMessages && lastMarkerIdx === -1;
    const percent = Math.min(Math.round((usedTokens / maxTokens) * 100), 100);
    return { usedTokens, maxTokens, percent, isEstimate };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [
    messagesLength,
    currentModel?.max_tokens,
    activeConversation?.system_prompt,
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
      const model = provider?.models.find((m) => m.model_id === cm.model_id);
      return {
        ...cm,
        modelName: model?.name ?? cm.model_id,
        providerName: provider?.name ?? "",
      };
    });
  }, [companionModels, providers]);

  const handleMultiModelSelect = useCallback(
    (models: Array<{ providerId: string; model_id: string }>) => {
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
      model_id: activeConversation?.model_id ?? "",
      voice: ttsVoice,
      audio_format: { sample_rate: 24000, channels: 1, encoding: "Pcm16" },
      stt_provider_id: sttProviderId || null,
      tts_provider_id: ttsProviderId || null,
    }),
    [activeConversation?.model_id, ttsVoice, sttProviderId, ttsProviderId],
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
          if (providersLoading || (providers ?? []).length === 0) {
            messageApi.warning(t("chat.noModelsAvailable"));
            return;
          }
          let provider = settings.default_provider_id
            ? providers.find(
              (p) => p.id === settings.default_provider_id && p.enabled,
            )
            : undefined;
          let model = provider?.models.find(
            (m) => m.model_id === settings.default_model_id && m.enabled,
          );
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
            trimmed.slice(0, 30),
            model.model_id,
            provider.id,
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
    providers,
    providersLoading,
    settings,
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
      {attachedFiles.length > 0 && (
        <div className="flex flex-wrap gap-2 mb-2">
          {attachedFiles.map((file, idx) => {
            const fileCategory = getFileTypeCategory(file.type);
            const isImage = fileCategory === "image";
            const isPreviewable = isImage
              && file.type !== "image/gif"
              && file.type !== "image/svg+xml";

            return (
              <div
                key={`${file.name}-${file.size}-${file.lastModified}`}
                className="relative group"
                style={{
                  backgroundColor: token.colorFillTertiary,
                  borderRadius: token.borderRadius,
                  border: `1px solid ${token.colorBorderSecondary}`,
                  overflow: "hidden",
                  maxWidth: isImage ? 120 : 200,
                }}
              >
                {isImage && (
                  <div
                    style={{
                      width: 120,
                      height: 80,
                      display: "flex",
                      alignItems: "center",
                      justifyContent: "center",
                      backgroundColor: token.colorFillSecondary,
                      overflow: "hidden",
                    }}
                  >
                    {isPreviewable
                      ? (
                        <Image
                          src={attachmentObjectUrls[idx]}
                          alt={file.name}
                          style={{
                            width: "100%",
                            height: "100%",
                            objectFit: "cover",
                          }}
                          preview={{ mask: { blur: true }, scaleStep: 0.5 }}
                        />
                      )
                      : (
                        <img
                          src={attachmentObjectUrls[idx]}
                          alt={file.name}
                          style={{
                            width: "100%",
                            height: "100%",
                            objectFit: "cover",
                          }}
                        />
                      )}
                  </div>
                )}
                <div
                  className={`flex items-center gap-1.5 px-2 py-1 ${isImage ? "" : ""}`}
                  style={!isImage ? { maxWidth: 200 } : undefined}
                >
                  {!isImage && (
                    <span style={{ color: token.colorPrimary, flexShrink: 0 }}>
                      {getFileIcon(fileCategory)}
                    </span>
                  )}
                  <span
                    className="text-xs truncate"
                    style={{
                      color: token.colorText,
                      flex: 1,
                      maxWidth: isImage ? 100 : 140,
                    }}
                    title={file.name}
                  >
                    {file.name}
                  </span>
                  <span
                    className="text-xs"
                    style={{ color: token.colorTextSecondary, flexShrink: 0 }}
                  >
                    {formatFileSize(file.size)}
                  </span>
                  <Trash2
                    size={14}
                    className="cursor-pointer shrink-0"
                    style={{ color: token.colorTextSecondary }}
                    onClick={() => removeFile(idx)}
                  />
                </div>
              </div>
            );
          })}
        </div>
      )}

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
          <div className="flex flex-wrap gap-1.5 px-3 pt-3 pb-1">
            <span
              className="inline-flex items-center px-2 py-0.5 text-xs"
              style={{ color: token.colorTextTertiary }}
            >
              {t("chat.multiModel.selectTitle")}:
            </span>
            {companionDisplayInfos.map((cm, idx) => (
              <span
                key={`${cm.providerId}-${cm.model_id}`}
                className="inline-flex items-center gap-1.5 pl-1.5 pr-1 py-0.5 text-xs"
                style={{
                  backgroundColor: token.colorFillSecondary,
                  borderRadius: token.borderRadiusSM,
                  color: token.colorText,
                }}
              >
                <ModelIcon model={cm.model_id} size={14} type="avatar" />
                <span
                  style={{
                    maxWidth: 120,
                    overflow: "hidden",
                    textOverflow: "ellipsis",
                    whiteSpace: "nowrap",
                  }}
                >
                  {cm.modelName}
                </span>
                {cm.providerName && (
                  <span
                    style={{ color: token.colorTextQuaternary, fontSize: 12 }}
                  >
                    {cm.providerName}
                  </span>
                )}
                <X
                  size={12}
                  className="cursor-pointer shrink-0"
                  style={{ color: token.colorTextTertiary }}
                  onClick={() => removeCompanionModel(idx)}
                />
              </span>
            ))}
            {/* Clear all companion models */}
            <span
              className="inline-flex items-center gap-1 px-1.5 py-0.5 text-xs cursor-pointer"
              role="button"
              tabIndex={0}
              onKeyDown={(e) => {
                if (e.key === "Enter" || e.key === " ") {
                  e.preventDefault();
                  clearAllCompanionModels();
                }
              }}
              style={{
                borderRadius: token.borderRadiusSM,
                color: token.colorTextTertiary,
              }}
              onClick={clearAllCompanionModels}
            >
              <Trash2 size={11} />
              {t("chat.clearAll")}
            </span>
          </div>
        )}

        {/* 引用回复预览条：显示当前被引用的消息 */}
        {quotedMessage && (
          <div
            className="quote-preview-bar"
            style={{
              display: "flex",
              alignItems: "center",
              gap: 8,
              padding: "6px 10px",
              marginBottom: 6,
              backgroundColor: token.colorFillTertiary,
              borderLeft: `3px solid ${token.colorPrimary}`,
              borderRadius: token.borderRadiusSM,
            }}
          >
            <MessageSquare size={14} style={{ color: token.colorPrimary, flexShrink: 0 }} />
            <div style={{ minWidth: 0, flex: 1, overflow: "hidden" }}>
              <Typography.Text
                style={{ fontSize: 12, color: token.colorTextTertiary, display: "block" }}
              >
                {t("chat.quote.replyingTo")}
              </Typography.Text>
              <Typography.Text
                style={{
                  fontSize: 13,
                  color: token.colorTextSecondary,
                  whiteSpace: "nowrap",
                  overflow: "hidden",
                  textOverflow: "ellipsis",
                  display: "block",
                }}
              >
                {quotedMessage.content.length > 100
                  ? `${quotedMessage.content.slice(0, 100)}…`
                  : quotedMessage.content}
              </Typography.Text>
            </div>
            <Tooltip title={t("chat.quote.cancel")}>
              <Button
                type="text"
                size="small"
                icon={<X size={14} />}
                onClick={() => useUIStore.getState().setQuotedMessageId(null)}
                style={{ color: token.colorTextTertiary, flexShrink: 0 }}
              />
            </Tooltip>
          </div>
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
                case "new":
                  if (!activeConversationId) { return; }
                  await createConversation("", "", "", { mode: "chat" });
                  break;
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
          {streaming
            ? (
              <Button
                shape="circle"
                size="small"
                danger
                data-testid="stop-generation-btn"
                icon={<Square size={14} />}
                onClick={handleCancel}
                style={{ flexShrink: 0, alignSelf: "flex-end" }}
              />
            )
            : (
              <Button
                type="primary"
                shape="circle"
                size="small"
                data-testid="send-btn"
                aria-label={t("chat.sendMessage")}
                icon={<ArrowUp size={16} />}
                onClick={handleSend}
                disabled={!value.trim() || streaming}
                style={{ flexShrink: 0, alignSelf: "flex-end", width: 36, height: 36 }}
                className={value.trim() && !streaming
                  ? "ax-glow-shadow"
                  : ""}
              />
            )}
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
            {hasReasoning && (
              <DropdownMenu
                items={thinkingMenuItems}
                open={thinkingDropdownOpen}
                onOpenChange={setThinkingDropdownOpen}
              >
                <Tooltip title={t("chat.thinkingIntensity")}>
                  <Button
                    type="text"
                    size="small"
                    icon={thinkingIcon}
                    style={thinkingBudget === 0
                      ? { color: token.colorError }
                      : thinkingBudget !== null
                      ? { color: token.colorPrimary }
                      : undefined}
                  />
                </Tooltip>
              </DropdownMenu>
            )}
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
            <Popover
              trigger="click"
              placement="topLeft"
              content={mcpPopoverContent}
              arrow={false}
              open={mcpPopoverOpen}
              onOpenChange={setMcpPopoverOpen}
            >
              <Tooltip
                title={t("chat.connector.title")}
                open={mcpPopoverOpen ? false : undefined}
              >
                <Badge
                  count={enabledMcpServerIds.filter((id) => mcpServers.some((s) => s.id === id && s.enabled)).length}
                  size="small"
                  offset={[-4, 4]}
                  color={token.colorPrimary}
                >
                  <Button
                    type="text"
                    size="small"
                    icon={<Plug size={14} />}
                    style={enabledMcpServerIds.some((id) => mcpServers.some((s) => s.id === id && s.enabled))
                      ? { color: token.colorPrimary }
                      : undefined}
                  />
                </Badge>
              </Tooltip>
            </Popover>
            <Tooltip title={t("chat.sources.title")}>
              <Badge
                count={enabledKnowledgeBaseIds.length
                  + (activeMemoryNamespaceId ? 1 : 0)
                  + enabledWikiIds.length}
                size="small"
                offset={[-4, 4]}
                color={token.colorPrimary}
              >
                <Button
                  type="text"
                  size="small"
                  icon={<Database size={14} />}
                  onClick={() => setSourceModalOpen(true)}
                  style={enabledKnowledgeBaseIds.length
                        + (activeMemoryNamespaceId ? 1 : 0)
                        + enabledWikiIds.length
                      > 0
                    ? { color: token.colorPrimary }
                    : undefined}
                />
              </Badge>
            </Tooltip>
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
                  icon: activeConversation?.context_compression ? <ZapOff size={14} /> : <Zap size={14} />,
                  label: activeConversation?.context_compression
                    ? t("chat.disableAutoCompression")
                    : t("chat.enableAutoCompression"),
                  onClick: () => {
                    if (!activeConversationId || !activeConversation) {
                      return;
                    }
                    updateConversation(activeConversationId, {
                      context_compression: !activeConversation.context_compression,
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
                  style={activeConversation?.context_compression
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
                  onClick={() => {
                    setDelegateTask(value);
                    setDelegateModalOpen(true);
                  }}
                />
              </Tooltip>
            )}
            {gatewayMenuItems.length > 0 && (
              <DropdownMenu items={gatewayMenuItems}>
                <Tooltip title={t("chat.mode.gateway")}>
                  <Button type="text" size="small" icon={<Globe size={14} />} />
                </Tooltip>
              </DropdownMenu>
            )}
            {currentMode === "agent" && activeConversationId && (
              <PlanHistoryPanel conversationId={activeConversationId} />
            )}
            {currentMode === "agent" && (
              <Tooltip
                title={messagesLength > 0
                  ? t("chat.workspaceLocked")
                  : agentCwd || t("common.workingDirectory")}
              >
                <Button
                  type="text"
                  size="small"
                  icon={<FolderOpen size={14} />}
                  onClick={handleSelectCwd}
                  disabled={messagesLength > 0}
                  style={{
                    display: "flex",
                    alignItems: "center",
                    gap: 4,
                    maxWidth: 400,
                  }}
                >
                  <span
                    style={{
                      overflow: "hidden",
                      textOverflow: "ellipsis",
                      whiteSpace: "nowrap",
                      fontSize: 12,
                    }}
                  >
                    {agentCwd
                      ? abbreviatePath(agentCwd)
                      : t("common.selectDirectory")}
                  </span>
                </Button>
              </Tooltip>
            )}
            {currentMode === "agent" && agentCwd && (
              <Tooltip title={t("common.openDirectory")}>
                <Button
                  type="text"
                  size="small"
                  icon={<ExternalLink size={14} />}
                  onClick={async () => {
                    try {
                      const { revealItemInDir } = await import("@tauri-apps/plugin-opener");
                      await revealItemInDir(agentCwd);
                    } catch (e) {
                      logIpcError("open directory")(e);
                    }
                  }}
                  style={{ fontSize: 12, minWidth: "auto", padding: "0 4px" }}
                />
              </Tooltip>
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
              <DropdownMenu items={permissionModeItems}>
                <Button
                  type="text"
                  size="small"
                  icon={permissionModeIcon}
                  style={{
                    display: "flex",
                    alignItems: "center",
                    gap: 4,
                    fontSize: 12,
                    ...(agentPermissionMode === "full_access"
                      ? { color: token.colorError }
                      : {}),
                  }}
                >
                  {permissionModeLabel}
                </Button>
              </DropdownMenu>
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

      {/* 委派任务弹窗 */}
      <Modal
        title={t("multiAgent.delegateTitle")}
        open={delegateModalOpen}
        onCancel={() => setDelegateModalOpen(false)}
        confirmLoading={multiAgentStore.delegating}
        onOk={async () => {
          if (!delegateRole || !delegateTask.trim()) {
            messageApi.warning(t("multiAgent.fillRequired"));
            return;
          }
          try {
            await multiAgentStore.delegateTask({
              roleName: delegateRole,
              task: delegateTask.trim(),
              providerId: activeConversation?.provider_id || "",
              modelId: activeConversation?.model_id || "",
            });
            messageApi.success(t("multiAgent.delegateSuccess"));
            setDelegateModalOpen(false);
          } catch (e) {
            messageApi.error(`${t("multiAgent.delegateFailed")}: ${e}`);
          }
        }}
        okText={t("multiAgent.delegateBtn")}
        destroyOnHidden
      >
        <Space orientation="vertical" style={{ width: "100%" }} size="middle">
          <div>
            <Typography.Text type="secondary">{t("multiAgent.selectRole")}</Typography.Text>
            <Segmented
              block
              value={delegateRole}
              onChange={(v) => setDelegateRole(v as string)}
              options={multiAgentStore.roles.map((r) => ({
                label: r.name,
                value: r.id,
              }))}
            />
          </div>
          <div>
            <Typography.Text type="secondary">{t("multiAgent.taskDescription")}</Typography.Text>
            <Input.TextArea
              value={delegateTask}
              onChange={(e) => setDelegateTask(e.target.value)}
              rows={4}
              placeholder={t("multiAgent.taskPlaceholder")}
            />
          </div>
        </Space>
      </Modal>

      <Modal
        title={editingMcpServer
          ? t("chat.connector.custom")
          : t("chat.connector.add")}
        open={connectorModalOpen}
        onCancel={() => setConnectorModalOpen(false)}
        onOk={async () => {
          try {
            const values = await mcpForm.validateFields();
            const input: CreateMcpServerInput = {
              name: values.name,
              transport: values.transport as "stdio" | "http" | "sse",
              command: values.command,
              args: values.args
                ? values.args.split(/\s+/).filter(Boolean)
                : undefined,
              endpoint: values.endpoint,
              enabled: false,
            };
            if (editingMcpServer) {
              await updateMcpServer(editingMcpServer.id, input);
              messageApi.success(t("common.saved"));
            } else {
              await createMcpServer(input);
              messageApi.success(t("common.saved"));
            }
            mcpForm.resetFields();
            setConnectorModalOpen(false);
            setEditingMcpServer(null);
          } catch {
            // validation error, form will show errors
          }
        }}
        destroyOnHidden
      >
        <Form
          form={mcpForm}
          layout="vertical"
          size="small"
          initialValues={{
            transport: "stdio",
          }}
        >
          <Form.Item
            name="name"
            label={t("common.name")}
            rules={[{ required: true }]}
          >
            <Input placeholder={t("chat.connector.placeholderName")} />
          </Form.Item>
          <Form.Item
            name="transport"
            label={t("common.type")}
            rules={[{ required: true }]}
          >
            <Select
              options={[
                { label: "stdio", value: "stdio" },
                { label: "HTTP", value: "http" },
                { label: "SSE", value: "sse" },
              ]}
            />
          </Form.Item>
          <Form.Item
            name="command"
            label={t("chat.connector.command")}
            rules={[{ required: true }]}
          >
            <Input placeholder={t("chat.connector.placeholderCommand")} />
          </Form.Item>
          <Form.Item name="args" label={t("chat.connector.args")}>
            <Input placeholder={t("chat.connector.placeholderArgs")} />
          </Form.Item>
          <Form.Item
            noStyle
            shouldUpdate={(prev, cur) => prev.transport !== cur.transport}
          >
            {({ getFieldValue }) =>
              getFieldValue("transport") !== "stdio" && (
                <Form.Item
                  name="endpoint"
                  label={t("chat.connector.endpoint")}
                  rules={[{ required: true }]}
                >
                  <Input placeholder={t("chat.connector.placeholderEndpoint")} />
                </Form.Item>
              )}
          </Form.Item>
        </Form>
      </Modal>

      <Modal
        title={t("chat.sources.title")}
        open={sourceModalOpen}
        onCancel={() => setSourceModalOpen(false)}
        footer={
          <Button type="primary" onClick={() => setSourceModalOpen(false)}>
            {t("common.confirm")}
          </Button>
        }
        width={420}
        destroyOnHidden
      >
        {sourcePopoverContent}
      </Modal>

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

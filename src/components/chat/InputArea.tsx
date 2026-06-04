import { ContextHelp } from "@/components/help/ContextHelp";
import { DropdownMenu } from "@/components/layout/DropdownMenu";
import type { DropdownItem } from "@/components/layout/DropdownMenu";
import { Tooltip } from "@/components/layout/Tooltip";
import { KnowledgeBaseIcon } from "@/components/shared/KnowledgeBaseIcon";
import { McpServerIcon } from "@/components/shared/McpServerIcon";
import { NamespaceIcon } from "@/components/shared/NamespaceIcon";
import { PROVIDER_TYPE_LABELS, SearchProviderTypeIcon } from "@/components/shared/SearchProviderIcon";
import { SkillToolbar } from "@/components/skill/SkillToolbar";
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
} from "@/stores";
import { useExpertStore } from "@/stores/feature/expertStore";
import { useLlmWikiStore } from "@/stores/feature/llmWikiStore";
import { usePromptTemplateStore } from "@/stores/feature/promptTemplateStore";
import type { PromptTemplate } from "@/types";
import type { AttachmentInput, Model, ProviderConfig, RealtimeConfig } from "@/types";
import { ModelIcon } from "@lobehub/icons";
import { open } from "@tauri-apps/plugin-dialog";
import { App, Badge, Button, Checkbox, Image, Popover, Radio, Select, Tag, theme } from "antd";
import {
  ArrowUp,
  Atom,
  BookOpen,
  Bot,
  Brain,
  Check,
  CircleOff,
  ClipboardList,
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
  Library,
  MessageSquare,
  Mic,
  Music,
  Paperclip,
  Play,
  Plug,
  Route,
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
  const [value, setValue] = useState(() => {
    const convId = useConversationStore.getState().activeConversationId;
    return convId ? _draftCache.get(convId) || "" : "";
  });
  const [attachedFiles, setAttachedFiles] = useState<File[]>([]);

  const objectUrlsRef = useRef<string[]>([]);
  const attachmentObjectUrls = useMemo(() => {
    objectUrlsRef.current.forEach((url) => URL.revokeObjectURL(url));
    const urls = attachedFiles.map((f) => URL.createObjectURL(f));
    objectUrlsRef.current = urls;
    return urls;
  }, [attachedFiles]);
  useEffect(() => {
    return () => {
      objectUrlsRef.current.forEach((url) => URL.revokeObjectURL(url));
    };
  }, []);
  const [voiceCallVisible, setVoiceCallVisible] = useState(false);
  const photoInputRef = useRef<HTMLInputElement>(null);
  const audioInputRef = useRef<HTMLInputElement>(null);
  const videoInputRef = useRef<HTMLInputElement>(null);
  const [settingsOpen, setSettingsOpen] = useState(false);
  // const [modelRoutingOpen, setModelRoutingOpen] = useState(false); // removed
  const [mcpPopoverOpen, setMcpPopoverOpen] = useState(false);
  const [searchDropdownOpen, setSearchDropdownOpen] = useState(false);
  const fileInputRef = useRef<HTMLInputElement>(null);
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const valueRef = useRef(value);
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

  const { message: messageApi, modal } = App.useApp();
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
  const sendAgentMessage = useConversationStore((s) => s.sendAgentMessage);
  const sendPlanMessage = useConversationStore((s) => s.sendPlanMessage);
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
  const loadSearchProviders = useSearchStore((s) => s.loadProviders);

  // MCP state
  const mcpServers = useMcpStore((s) => s.servers);
  const loadMcpServers = useMcpStore((s) => s.loadServers);
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

  // Work strategy state (for plan mode)
  const [workStrategy, setWorkStrategy] = useState<"direct" | "plan">("direct");

  // Gateway links state
  const gatewayLinks = useGatewayLinkStore((s) => s.links);
  const fetchGatewayLinks = useGatewayLinkStore((s) => s.fetchLinks);
  const [selectedGatewayId, setSelectedGatewayId] = useState<string | null>(
    null,
  );

  // Knowledge base state
  const knowledgeBases = useKnowledgeStore((s) => s.bases);
  const loadKnowledgeBases = useKnowledgeStore((s) => s.loadBases);
  const enabledKnowledgeBaseIds = useConversationStore(
    (s) => s.enabledKnowledgeBaseIds,
  );
  const toggleKnowledgeBase = useConversationStore(
    (s) => s.toggleKnowledgeBase,
  );
  const [sourcePopoverOpen, setSourcePopoverOpen] = useState(false);

  // Memory state
  const memoryNamespaces = useMemoryStore((s) => s.namespaces);
  const loadMemoryNamespaces = useMemoryStore((s) => s.loadNamespaces);
  const activeMemoryNamespaceId = useConversationStore(
    (s) => s.activeMemoryNamespaceId,
  );
  const setActiveMemoryNamespace = useConversationStore(
    (s) => s.setActiveMemoryNamespace,
  );

  // Wiki vault state
  const wikis = useLlmWikiStore((s) => s.wikis);
  const loadWikis = useLlmWikiStore((s) => s.loadWikis);
  const enabledWikiIds = useConversationStore((s) => s.enabledWikiIds);
  const toggleWiki = useConversationStore((s) => s.toggleWiki);

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
  const pendingWorkStrategyRef = useRef<"direct" | "plan" | null>(null);

  const activeConversation = conversations.find(
    (c) => c.id === activeConversationId,
  );
  // Use pendingModeRef when no conversation exists so the UI (mode badge, send routing)
  // correctly reflects the user's last mode dropdown choice.
  const currentMode = activeConversation?.mode || pendingModeRef.current || "chat";

  // Reset pending mode refs when a conversation becomes active
  useEffect(() => {
    if (activeConversationId) {
      pendingModeRef.current = null;
      pendingWorkStrategyRef.current = null;
    }
  }, [activeConversationId]);

  // Sync work strategy from conversation (also fires on mode switch)
  useEffect(() => {
    const strategy = activeConversation?.work_strategy as
      | "direct"
      | "plan"
      | undefined;
    setWorkStrategy(strategy || "direct");
  }, [activeConversation?.work_strategy, activeConversation?.mode]);

  // Unified mode: ask | plan | action
  const unifiedMode = useMemo((): "ask" | "plan" | "action" => {
    if (currentMode === "chat") return "ask";
    if (currentMode === "agent" && workStrategy === "plan") return "plan";
    if (currentMode === "agent" && workStrategy === "direct") return "action";
    return "ask";
  }, [currentMode, workStrategy]);

  const navigate = useNavigate();
  const setSettingsSection = useUIStore((s) => s.setSettingsSection);

  // Load search providers on mount
  useEffect(() => {
    if ((searchProviders ?? []).length === 0) {
      loadSearchProviders();
    }
  }, [searchProviders, loadSearchProviders]);

  // Load MCP servers on mount
  useEffect(() => {
    if ((mcpServers ?? []).length === 0) {
      loadMcpServers();
    }
  }, [mcpServers, loadMcpServers]);

  // Load knowledge bases on mount
  useEffect(() => {
    if ((knowledgeBases ?? []).length === 0) {
      loadKnowledgeBases();
    }
  }, [knowledgeBases, loadKnowledgeBases]);

  // Load memory namespaces on mount
  useEffect(() => {
    if ((memoryNamespaces ?? []).length === 0) {
      loadMemoryNamespaces();
    }
  }, [memoryNamespaces, loadMemoryNamespaces]);

  // Load wiki vaults on mount
  useEffect(() => {
    if ((wikis ?? []).length === 0) {
      loadWikis();
    }
  }, [wikis, loadWikis]);

  // Load gateway links on mount
  useEffect(() => {
    if ((gatewayLinks ?? []).length === 0) {
      fetchGatewayLinks();
    }
  }, [gatewayLinks, fetchGatewayLinks]);

  // Set default workspace directory when in agent mode and no conversation is active
  useEffect(() => {
    if (
      !activeConversationId
      && currentMode === "agent"
      && settings.default_workspace_dir
    ) {
      setAgentCwd(settings.default_workspace_dir);
    }
  }, [activeConversationId, currentMode, settings.default_workspace_dir]);

  // Fetch agent permission mode on mount/conversation switch
  useEffect(() => {
    if (currentMode === "agent" && activeConversationId) {
      invoke("agent_get_session", {
        request: { conversationId: activeConversationId },
      })
        .then((session: any) => {
          if (session) {
            setAgentPermissionMode(session.permission_mode || "default");
            setAgentCwd(session.cwd || null);
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
  companionStorageKeyRef.current = activeConversationId
    ? `axagent:companion-models:${activeConversationId}`
    : null;
  const companionStorageKey = companionStorageKeyRef.current
    ? `axagent:companion-models:${activeConversationId}`
    : null;

  // Load companion models when conversation changes
  useEffect(() => {
    if (!companionStorageKey) {
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
  }, [searchProviders, searchEnabled, searchProviderId, token, t]);

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
              setSettingsSection("mcpServers");
              navigate("/settings");
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
              setSettingsSection("mcpServers");
              navigate("/settings");
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

  // Expert menu items — 专家角色选择（所有模式通用）
  // 通过 selector 订阅 store 状态变更，确保 useMemo 响应式更新
  const expertBuiltinRoles = useExpertStore((s) => s.builtinRoles);
  const agencyRoles = useExpertStore((s) => s.agencyRoles);
  const customRoles = useExpertStore((s) => s.customRoles);
  const expertMenuItems = useMemo((): DropdownItem[] => {
    const grouped = useExpertStore.getState().getRolesByCategory();
    const items: DropdownItem[] = [];

    for (const [category, categoryRoles] of Object.entries(grouped)) {
      if (items.length > 0) {
        items.push({ key: "div-1", divider: true });
      }
      items.push({
        key: `category-${category}`,
        label: t("expertCategory." + category) || category,
        disabled: true,
      });
      for (const role of categoryRoles) {
        items.push({
          key: `expert-${role.id}`,
          label: (
            <span
              style={{ display: "inline-flex", alignItems: "center", gap: 6 }}
            >
              <span>{role.icon}</span>
              <span>{role.name}</span>
            </span>
          ),
          onClick: () => handleExpertSelect(role.id),
        });
      }
    }
    return items;
  }, [expertBuiltinRoles, agencyRoles, customRoles, t]);

  // Unified mode menu items (Ask / Plan / Action)
  const unifiedModeMenuItems = useMemo((): DropdownItem[] => {
    const items: DropdownItem[] = [
      {
        key: "ask",
        icon: <MessageSquare size={14} />,
        label: (
          <span className="flex items-center gap-2">
            {t("chat.mode.ask")}
            {unifiedMode === "ask" && <Check size={14} style={{ color: token.colorPrimary }} />}
          </span>
        ),
        onClick: () => handleUnifiedModeChange("ask"),
      },
      {
        key: "plan",
        icon: <ClipboardList size={14} />,
        label: (
          <span className="flex items-center gap-2">
            {t("chat.mode.plan")}
            {unifiedMode === "plan" && <Check size={14} style={{ color: token.colorPrimary }} />}
          </span>
        ),
        onClick: () => handleUnifiedModeChange("plan"),
      },
      {
        key: "action",
        icon: <Play size={14} />,
        label: (
          <span className="flex items-center gap-2">
            {t("chat.mode.action")}
            {unifiedMode === "action" && <Check size={14} style={{ color: token.colorPrimary }} />}
          </span>
        ),
        onClick: () => handleUnifiedModeChange("action"),
      },
    ];
    // Add Gateway options if connected
    const connectedGateways = gatewayLinks.filter(
      (l) => l.enabled && l.status === "connected",
    );
    if (connectedGateways.length > 0) {
      items.push({ key: "div-gw", divider: true });
      connectedGateways.forEach((gw) => {
        items.push({
          key: `gateway:${gw.id}`,
          icon: <Globe size={14} />,
          label: gw.name,
          onClick: () => setSelectedGatewayId(gw.id),
        });
      });
    }
    return items;
  }, [t, unifiedMode, gatewayLinks, selectedGatewayId, token.colorPrimary]);

  // Handle expert selection
  const handleExpertSelect = useCallback(
    async (roleId: string) => {
      const role = useExpertStore.getState().getRoleById(roleId);
      if (!role) {
        return;
      }

      let provider: ProviderConfig | undefined;
      let model: Model | undefined;
      if (role.suggestedProviderId && role.suggestedModelId) {
        provider = providers.find((p) => p.id === role.suggestedProviderId);
        model = provider?.models.find(
          (m: Model) => m.model_id === role.suggestedModelId,
        );
      }
      if (!provider || !model) {
        provider = providers.find(
          (p) => p.enabled && p.models.some((m: Model) => m.enabled),
        );
        model = provider?.models.find((m: Model) => m.enabled);
      }
      if (!provider || !model) {
        messageApi.warning(t("chat.noModelsAvailable"));
        return;
      }

      // 确保 AgentProfile 在 DB 中存在
      invoke("ensure_agent_profile", {
        id: roleId,
        name: role.name,
        expertId: role.source === "agency" ? roleId : (role.expertId ?? null),
        agentRole: role.agentRole ?? null,
      }).catch(() => {/* profile 可能已存在 */});

      await createConversation(role.name, model.model_id, provider.id, {
        mode: "agent",
        agent_profile_id: roleId,
      });
    },
    [createConversation, providers, messageApi, t],
  );

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
  ], [t, agentPermissionMode, token.colorPrimary]);

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
    [activeConversationId, t],
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
  }, [agentPermissionMode]);

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
  const isSwitchingStrategyRef = useRef(false);

  const handleWorkStrategyChange = useCallback(
    async (strategy: "direct" | "plan") => {
      if (!activeConversationId || !activeConversation) {
        return;
      }
      if (isSwitchingStrategyRef.current) {
        return;
      }
      isSwitchingStrategyRef.current = true;
      try {
        setWorkStrategy(strategy);
        await updateConversation(activeConversationId, {
          work_strategy: strategy,
        });
      } catch (e) {
        logIpcError("WorkStrategy: update work strategy")(e);
        // Revert
        setWorkStrategy(
          (activeConversation.work_strategy as "direct" | "plan") || "direct",
        );
      } finally {
        isSwitchingStrategyRef.current = false;
      }
    },
    [activeConversationId, activeConversation, updateConversation],
  );

  // Agent CWD helpers
  const abbreviatePath = useCallback((path: string): string => {
    const segments = path.replace(/\\/g, "/").split("/").filter(Boolean);
    if (segments.length <= 2) {
      return path;
    }
    return "…/" + segments.slice(-2).join("/");
  }, []);

  const handleSelectCwd = useCallback(async () => {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        title: t("common.selectDirectory"),
      });
      if (selected && typeof selected === "string") {
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
              setSourcePopoverOpen(false);
              navigate("/knowledge");
            }}
          >
            {t("chat.connector.goConfig")}
          </Button>
        </div>
      );
    }
    return (
      <div style={{ minWidth: 220, maxHeight: 400, overflowY: "auto" }}>
        {safeKb.length > 0 && (
          <div
            style={{
              marginBottom: safeKb.length > 0
                  && (safeMem.length > 0 || safeWikis.length > 0)
                ? 8
                : 0,
            }}
          >
            <div
              style={{
                fontSize: 12,
                fontWeight: 600,
                color: token.colorTextSecondary,
                textTransform: "uppercase",
                letterSpacing: 0.5,
                marginBottom: 4,
                display: "flex",
                alignItems: "center",
                gap: 4,
              }}
            >
              <BookOpen size={11} />
              {t("chat.knowledge.title")}
            </div>
            {safeKb.map((kb) => (
              <div key={kb.id} style={{ padding: "2px 0" }}>
                <Checkbox
                  checked={enabledKnowledgeBaseIds.includes(kb.id)}
                  onChange={() => toggleKnowledgeBase(kb.id)}
                >
                  <span
                    style={{
                      display: "inline-flex",
                      alignItems: "center",
                      gap: 6,
                      fontSize: 13,
                    }}
                  >
                    <KnowledgeBaseIcon kb={kb} size={14} />
                    {kb.name}
                  </span>
                </Checkbox>
              </div>
            ))}
          </div>
        )}
        {safeMem.length > 0 && (
          <div
            style={{
              marginBottom: safeMem.length > 0 && safeWikis.length > 0 ? 8 : 0,
            }}
          >
            <div
              style={{
                fontSize: 12,
                fontWeight: 600,
                color: token.colorTextSecondary,
                textTransform: "uppercase",
                letterSpacing: 0.5,
                marginBottom: 4,
                display: "flex",
                alignItems: "center",
                gap: 4,
              }}
            >
              <Brain size={11} />
              {t("chat.memory.title")}
            </div>
            <Radio.Group
              value={activeMemoryNamespaceId}
              onChange={(e) => setActiveMemoryNamespace(e.target.value || null)}
              style={{ display: "flex", flexDirection: "column", gap: 2 }}
            >
              {safeMem.map((ns) => (
                <Radio key={ns.id} value={ns.id}>
                  <span
                    style={{
                      fontSize: 13,
                      display: "inline-flex",
                      alignItems: "center",
                      gap: 6,
                    }}
                  >
                    <NamespaceIcon ns={ns} size={16} />
                    {ns.name}
                  </span>
                </Radio>
              ))}
            </Radio.Group>
          </div>
        )}
        {safeWikis.length > 0 && (
          <div>
            <div
              style={{
                fontSize: 12,
                fontWeight: 600,
                color: token.colorTextSecondary,
                textTransform: "uppercase",
                letterSpacing: 0.5,
                marginBottom: 4,
                display: "flex",
                alignItems: "center",
                gap: 4,
              }}
            >
              <Library size={11} />
              {t("chat.wiki.title")}
            </div>
            {safeWikis.map((wiki) => (
              <div key={wiki.id} style={{ padding: "2px 0" }}>
                <Checkbox
                  checked={enabledWikiIds.includes(wiki.id)}
                  onChange={() => toggleWiki(wiki.id)}
                >
                  <span
                    style={{
                      fontSize: 13,
                      display: "inline-flex",
                      alignItems: "center",
                      gap: 6,
                    }}
                  >
                    <Library size={14} />
                    {wiki.name}
                  </span>
                </Checkbox>
              </div>
            ))}
          </div>
        )}
      </div>
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
  }, [
    messagesLength,
    currentModel?.max_tokens,
    activeConversation?.system_prompt,
    summaryTokenCount,
    hasOlderMessages,
  ]);

  const { hasRealtimeVoice, hasReasoning, hasVision } = React.useMemo(
    () => ({
      hasRealtimeVoice: activeConversation
        ? !!findModelByIds(
          providers,
          activeConversation.provider_id,
          activeConversation.model_id,
        )?.capabilities.includes("RealtimeVoice")
        : false,
      hasReasoning: supportsReasoning(currentModel),
      hasVision: modelHasCapability(currentModel, "Vision"),
    }),
    [activeConversation, currentModel, providers],
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
        if (models.length > 0) {
          localStorage.setItem(companionStorageKey, JSON.stringify(models));
        } else {
          localStorage.removeItem(companionStorageKey);
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
          if (next.length > 0) {
            localStorage.setItem(companionStorageKey, JSON.stringify(next));
          } else {
            localStorage.removeItem(companionStorageKey);
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

  const voiceConfig: RealtimeConfig = React.useMemo(
    () => ({
      model_id: activeConversation?.model_id ?? "",
      voice: null,
      audio_format: { sample_rate: 24000, channels: 1, encoding: "Pcm16" },
    }),
    [activeConversation?.model_id],
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
                "Switched to Agent mode. Send a message to create an Agent conversation.",
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
                  "Agent session initialization failed due to a temporary connection issue. You can try sending again in a moment.",
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
          if (
            activeConversation.session_type === "workflow"
            || activeConversation.workflow_template_id
          ) {
            await updateConversation(activeConversation.id, {
              session_type: "conversation",
              workflow_template_id: null,
            });
          }
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
    ],
  );

  // ── Unified Mode (Ask / Plan / Action) ──
  const handleUnifiedModeChange = useCallback(
    async (mode: "ask" | "plan" | "action") => {
      if (mode === "ask") {
        await handleModeSwitch("chat");
        if (activeConversationId) {
          try {
            await updateConversation(activeConversationId, {
              work_strategy: "direct" as any,
            });
          } catch (e) {
            logIpcError("UnifiedMode: switch to ask")(e);
          }
        } else {
          pendingModeRef.current = "chat";
          pendingWorkStrategyRef.current = null;
        }
      } else {
        await handleModeSwitch("agent");
        const strategy = mode === "plan" ? "plan" : "direct";
        await handleWorkStrategyChange(strategy);
        if (!activeConversationId) {
          pendingWorkStrategyRef.current = strategy;
        }
      }
    },
    [activeConversationId, activeConversation, handleWorkStrategyChange, updateConversation],
  );

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
              work_strategy: pendingWorkStrategyRef.current ?? undefined,
            },
          );
          pendingModeRef.current = null;
          pendingWorkStrategyRef.current = null;
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
      if (currentMode === "agent" && workStrategy === "plan") {
        await sendPlanMessage(
          trimmed,
          attachments,
          effectiveSearchProviderId,
        );
      } else if (currentMode === "agent") {
        await sendAgentMessage(
          trimmed,
          attachments,
          effectiveSearchProviderId,
        );
      } else if (companionModels.length > 0) {
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
        );
      }
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
    sendAgentMessage,
    sendPlanMessage,
    sendMultiModelMessage,
    companionModels,
    activeConversationId,
    providers,
    providersLoading,
    settings,
    createConversation,
    messageApi,
    t,
    searchEnabled,
    searchProviderId,
    currentMode,
    workStrategy,
    selectedGatewayId,
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
          aria-label="resize handle"
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
                disabled={!value.trim() || streaming
                  || (activeConversation?.session_type === "workflow"
                    && activeConversation?.workflow_status === "completed")}
                style={{ flexShrink: 0, alignSelf: "flex-end", width: 36, height: 36 }}
                className={value.trim() && !streaming
                    && !(activeConversation?.session_type === "workflow"
                      && activeConversation?.workflow_status === "completed")
                  ? "ax-glow-shadow"
                  : ""}
              />
            )}
        </div>

        {/* Bottom action bar */}
        <div className="chat-input-tools">
          <div className="flex items-center gap-0.5">
            <SkillToolbar position="left" />
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
            {!activeConversationId && (
              <DropdownMenu items={expertMenuItems}>
                <Button type="text" size="small" icon={<Bot size={14} />} />
              </DropdownMenu>
            )}
            {hasReasoning && (
              <DropdownMenu
                items={thinkingMenuItems}
                open={thinkingDropdownOpen}
                onOpenChange={setThinkingDropdownOpen}
              >
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
                <Button type="text" size="small" icon={<Paperclip size={14} />} />
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
            <Popover
              trigger="click"
              placement="topLeft"
              content={sourcePopoverContent}
              arrow={false}
              open={sourcePopoverOpen}
              onOpenChange={setSourcePopoverOpen}
            >
              <Tooltip
                title={t("chat.sources.title")}
                open={sourcePopoverOpen ? false : undefined}
              >
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
                    style={enabledKnowledgeBaseIds.length
                          + (activeMemoryNamespaceId ? 1 : 0)
                          + enabledWikiIds.length
                        > 0
                      ? { color: token.colorPrimary }
                      : undefined}
                  />
                </Badge>
              </Tooltip>
            </Popover>
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
            {activeConversation?.session_type !== "workflow" && (
              <DropdownMenu items={unifiedModeMenuItems}>
                <Button
                  type="text"
                  size="small"
                  data-tutorial="agent-mode"
                  icon={
                    unifiedMode === "ask" ? <MessageSquare size={14} /> :
                    unifiedMode === "plan" ? <ClipboardList size={14} /> :
                    <Play size={14} />
                  }
                  style={{ display: "flex", alignItems: "center", gap: 4 }}
                />
              </DropdownMenu>
            )}
            <ContextHelp helpKey="agent" section="agent" />
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
                    maxWidth: 200,
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
            {currentMode === "agent"
              && activeConversationId
              && activeConversation?.session_type !== "workflow" && (
              <Tooltip title={t("chat.modelRouting")}>
                <Button
                  type="text"
                  size="small"
                  icon={<Route size={14} />}
                  onClick={undefined}
                />
              </Tooltip>
            )}
            {hasRealtimeVoice && (
              <Tooltip
                title={t("voice.startCall") + " - " + t("common.comingSoon")}
              >
                <Button
                  type="text"
                  size="small"
                  icon={<Mic size={14} />}
                  disabled
                />
              </Tooltip>
            )}
          </div>
          <div className="flex items-center gap-2">
            <SkillToolbar position="right" />
          </div>
        </div>
      </div>

      {/* Mode controls bar — below input container */}
      <div className="flex items-center justify-between px-1 pt-1">
        <div className="flex items-center gap-1">
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
          {contextCount > 0 && (
            <span style={{ fontSize: 12, color: token.colorTextSecondary }}>
              {contextCount} {t("chat.contextMessages")}
            </span>
          )}
          {contextTokenUsage
            && (() => {
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
                      tokens (
                      {contextTokenUsage.percent}%)
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
            })()}
        </div>
      </div>

      <ConversationSettingsModal
        open={settingsOpen}
        onClose={() => setSettingsOpen(false)}
      />

      {/* ModelRoutingConfigPanel removed */}

      {hasRealtimeVoice && (
        <VoiceCall
          visible={voiceCallVisible}
          onClose={() => setVoiceCallVisible(false)}
          config={voiceConfig}
        />
      )}

      {/* Drag-and-drop overlay */}
      {isDragging && (
        <div
          style={{
            position: "fixed",
            inset: 0,
            zIndex: 9999,
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

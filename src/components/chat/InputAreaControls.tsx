// SPDX-License-Identifier: AGPL-3.0-only

import { DropdownMenu } from "@/components/layout/DropdownMenu";
import type { DropdownItem } from "@/components/layout/DropdownMenu";
import { Tooltip } from "@/components/layout/Tooltip";
import { SkillToolbar } from "@/components/skill/SkillToolbar";
import type { ShortcutAction } from "@/lib/shortcuts";
import type { McpServer } from "@/types";
import { Badge, Button, Popover, theme } from "antd";
import type { GlobalToken } from "antd";
import {
  Bot,
  ClipboardList,
  Database,
  Eraser,
  ExternalLink,
  FileText,
  Film,
  FolderOpen,
  GitCompareArrows,
  Globe,
  Image as ImageIcon,
  MessageSquare,
  Mic,
  Paperclip,
  Play,
  Plug,
  Scissors,
  Shrink,
  SlidersHorizontal,
  Zap,
  ZapOff,
} from "lucide-react";
import { PlanHistoryPanel } from "./PlanHistoryPanel";

export function InputAreaControls(props: {
  token: GlobalToken;
  t: (key: string) => string;
  shortcutHint: (label: string, action: ShortcutAction) => string;

  // Search
  searchEnabled: boolean;
  searchMenuItems: DropdownItem[];
  searchDropdownOpen: boolean;
  setSearchDropdownOpen: React.Dispatch<React.SetStateAction<boolean>>;
  setSearchEnabled: (v: boolean) => void;
  setSearchProviderId: (v: string | null) => void;

  // Mode
  unifiedMode: "ask" | "plan" | "action";
  activeConversation:
    | { session_type?: string; workflow_status?: string | null; context_compression?: boolean }
    | undefined;
  unifiedModeMenuItems: DropdownItem[];

  // Expert
  expertMenuItems: DropdownItem[];

  // Thinking
  hasReasoning: boolean;
  thinkingMenuItems: DropdownItem[];
  thinkingDropdownOpen: boolean;
  setThinkingDropdownOpen: (v: boolean) => void;
  thinkingIcon: React.ReactNode;
  thinkingBudget: number | null;

  // Vision / Attachments
  hasVision: boolean;
  handleFileSelect: () => void;
  handlePhotoSelect: () => void;
  handleAudioSelect: () => void;
  handleVideoSelect: () => void;

  // MCP
  mcpPopoverContent: React.ReactNode;
  mcpPopoverOpen: boolean;
  setMcpPopoverOpen: (v: boolean) => void;
  enabledMcpServerIds: string[];
  mcpServers: McpServer[];

  // Sources
  enabledKnowledgeBaseIds: string[];
  activeMemoryNamespaceId: string | null;
  enabledWikiIds: string[];
  setSourceModalOpen: (v: boolean) => void;

  // Templates
  templatePopoverContent: React.ReactNode;
  templatePopoverOpen: boolean;
  setTemplatePopoverOpen: (v: boolean) => void;

  // Multi-model
  currentMode: string;
  companionModels: Array<{ providerId: string; model_id: string }>;
  setMultiModelOpen: (v: boolean) => void;

  // Context compression
  activeConversationId: string | null;
  streaming: boolean;
  compressing: boolean;
  messagesLength: number;
  updateConversation: (id: string, data: Partial<unknown>) => Promise<void>;
  compressContext: () => Promise<void>;
  messageApi: { success: (msg: string) => void; error: (msg: string) => void };

  // Clear / commit
  insertContextClear: () => void;
  clearAllMessages: () => void;
  modalConfirm: (opts: {
    title: string;
    content: string;
    okButtonProps?: { danger?: boolean };
    okText: string;
    cancelText: string;
    onOk: () => void | Promise<void>;
  }) => void;

  // Settings
  setSettingsOpen: (v: boolean) => void;

  // Plan history
  PlanHistoryPanel?: React.ReactNode;

  // Agent CWD
  agentCwd: string | null;
  handleSelectCwd: () => Promise<void>;
  abbreviatePath: (path: string) => string;

  // Agent permission mode
  agentPermissionMode: string;
  permissionModeItems: DropdownItem[];
  permissionModeIcon: React.ReactNode;
  permissionModeLabel: string;

  // Context token usage
  contextTokenUsage: {
    usedTokens: number;
    maxTokens: number;
    percent: number;
    isEstimate: boolean;
  } | null;
  contextCount: number;

  // Voice
  hasRealtimeVoice: boolean;
  onVoiceCallClick: () => void;
}) {
  const { token } = theme.useToken();
  const {
    t,
    shortcutHint,
    searchEnabled,
    searchMenuItems,
    searchDropdownOpen,
    setSearchDropdownOpen,
    setSearchEnabled,
    setSearchProviderId,
    unifiedMode,
    activeConversation,
    unifiedModeMenuItems,
    expertMenuItems,
    hasReasoning,
    thinkingMenuItems,
    thinkingDropdownOpen,
    setThinkingDropdownOpen,
    thinkingIcon,
    thinkingBudget,
    hasVision,
    handleFileSelect,
    handlePhotoSelect,
    handleAudioSelect,
    handleVideoSelect,
    mcpPopoverContent,
    mcpPopoverOpen,
    setMcpPopoverOpen,
    enabledMcpServerIds,
    mcpServers,
    enabledKnowledgeBaseIds,
    activeMemoryNamespaceId,
    enabledWikiIds,
    setSourceModalOpen,
    templatePopoverContent,
    templatePopoverOpen,
    setTemplatePopoverOpen,
    currentMode,
    companionModels,
    setMultiModelOpen,
    activeConversationId,
    streaming,
    compressing,
    messagesLength,
    updateConversation,
    compressContext,
    messageApi,
    insertContextClear,
    clearAllMessages,
    modalConfirm,
    setSettingsOpen,
    agentCwd,
    handleSelectCwd,
    abbreviatePath,
    agentPermissionMode,
    permissionModeItems,
    permissionModeIcon,
    permissionModeLabel,
    contextTokenUsage,
    contextCount,
    hasRealtimeVoice,
  } = props;

  return (
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
        {unifiedMode === "action" && activeConversation?.session_type !== "workflow" && (
          <DropdownMenu items={expertMenuItems}>
            <Tooltip title={t("expertBadge.selectExpert")}>
              <Button type="text" size="small" icon={<Bot size={14} />} />
            </Tooltip>
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
              || messagesLength === 0}
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
              modalConfirm({
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
            <Tooltip title={t("chat.mode.title")}>
              <Button
                type="text"
                size="small"
                data-tutorial="agent-mode"
                icon={unifiedMode === "ask"
                  ? <MessageSquare size={14} />
                  : unifiedMode === "plan"
                  ? <ClipboardList size={14} />
                  : <Play size={14} />}
                style={{ display: "flex", alignItems: "center", gap: 4 }}
              />
            </Tooltip>
          </DropdownMenu>
        )}
        {currentMode === "agent" && activeConversationId && <PlanHistoryPanel conversationId={activeConversationId} />}
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
                } catch {
                  // silent
                }
              }}
              style={{ fontSize: 12, minWidth: "auto", padding: "0 4px" }}
            />
          </Tooltip>
        )}
        {hasRealtimeVoice && (
          <Tooltip title={t("voice.startCall")}>
            <Button
              type="text"
              size="small"
              icon={<Mic size={14} />}
              onClick={props.onVoiceCallClick}
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
  );
}

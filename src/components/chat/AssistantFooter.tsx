// SPDX-License-Identifier: AGPL-3.0-only

// 从 ChatViewMessages.tsx 抽离的 AssistantFooter 组件 + Actions/ActionItem 共享定义。
// 主文件 ChatViewMessages.tsx 与本文件共同复用 Actions 组件，避免重复实现。

import { App, Input, Modal, Popconfirm, Popover, theme } from "antd";
import {
  ArrowDown,
  ArrowLeftRight,
  Check,
  Coins,
  Copy,
  GitBranch,
  MessageSquare,
  Pencil,
  RotateCcw,
  Save,
  TextCursorInput,
  Trash2,
  Zap,
} from "lucide-react";
import { type ReactNode } from "react";
import { useCallback, useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

import { useCopyToClipboard } from "@/hooks/useCopyToClipboard";
import { hasMultipleModelVersions } from "@/lib/chatMultiModel";
import { invoke, isTauri } from "@/lib/invoke";
import { useConversationStore, useStreamStore } from "@/stores";
import type { Message } from "@/types";

import { Tooltip } from "@/components/layout/Tooltip";
import { formatDuration, formatSpeed, formatTokenCount } from "../gateway/tokenFormat";
import { DeleteLastVersionPopover } from "./DeleteLastVersionPopover";
import { ModelSelector } from "./ModelSelector";
import { ModelTags } from "./ModelTags";
import { LayoutSwitcher, type MultiModelDisplayMode } from "./MultiModelDisplay";
import { VersionPagination } from "./VersionPagination";

// Popover 内格式按钮的统一样式
const styleBtn: React.CSSProperties = {
  padding: "4px 12px",
  border: "none",
  background: "transparent",
  cursor: "pointer",
  textAlign: "left",
  fontSize: 13,
  borderRadius: 4,
  color: "inherit",
};

// === 共享：Actions 组件 + ActionItem 类型 ===
// Local replacement for @ant-design/x Actions
export interface ActionItem {
  key: string;
  icon?: ReactNode;
  label?: string;
  onItemClick?: () => void;
  actionRender?: () => ReactNode;
}

export function Actions({
  items,
  onActionClick,
}: {
  items: ActionItem[];
  onActionClick?: (item: ActionItem) => void;
}) {
  return (
    <div className="msg-actions">
      {items.map((action) => {
        if (action.actionRender) {
          return (
            <div key={action.key} className="msg-action-custom">
              {action.actionRender()}
            </div>
          );
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

// === AssistantFooter 组件 ===
export interface AssistantFooterProps {
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
}

export function AssistantFooter({
  msg,
  conversationId,
  assistantCopyText,
  getModelDisplayInfo,
  onEditMessage,
  isStreaming = false,
  displayMode,
  onDisplayModeChange,
  onMultiModelDetected,
}: AssistantFooterProps) {
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
    async (providerId: string, model_id: string) => {
      try {
        if (providerId === msg.providerId && model_id === msg.modelId) {
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
                icon: assistantCopied
                  ? <Check size={14} style={{ color: token.colorSuccess }} />
                  : <Copy size={14} />,
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
                        // Markdown 或浏览器环境：走纯前端下载
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
                      // DOCX / PDF：走后端 Tauri 命令
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
                          <button
                            style={styleBtn}
                            onClick={() => handleSaveAs("md")}
                          >
                            Markdown (.md)
                          </button>
                          <button
                            style={styleBtn}
                            onClick={() => handleSaveAs("docx")}
                          >
                            Word (.docx)
                          </button>
                          <button
                            style={styleBtn}
                            onClick={() => handleSaveAs("pdf")}
                          >
                            PDF (.pdf)
                          </button>
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

// SPDX-License-Identifier: AGPL-3.0-only

import { findModelByIds } from "@/lib/modelCapabilities";
import { useConversationStore, useProviderStore, useSettingsStore } from "@/stores";
import { useAgentPanelStore } from "@/stores/shared/agentPanelStore";
import type { Model, ProviderConfig } from "@/types";
import { App, Button, Input, Spin } from "antd";
import { Bot, Ellipsis, Expand, Plus, Send } from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { useTranslation } from "react-i18next";

const MINI_WIDTH = 380;
const MINI_HEIGHT = 500;
const BUTTON_MARGIN = 20;
const STORAGE_KEY_POSITION = "axagent:agentMiniPanel:position";

function loadPersistedPosition(): { x: number; y: number } {
  try {
    const raw = localStorage.getItem(STORAGE_KEY_POSITION);
    if (raw) {
      const parsed = JSON.parse(raw) as { x: number; y: number };
      if (typeof parsed.x === "number" && typeof parsed.y === "number") {
        return {
          x: Math.max(0, Math.min(window.innerWidth - MINI_WIDTH, parsed.x)),
          y: Math.max(0, Math.min(window.innerHeight - MINI_HEIGHT, parsed.y)),
        };
      }
    }
  } catch {
    // 忽略
  }
  return {
    x: window.innerWidth - MINI_WIDTH - BUTTON_MARGIN,
    y: window.innerHeight - MINI_HEIGHT - 80,
  };
}

function persistPosition(x: number, y: number): void {
  try {
    localStorage.setItem(STORAGE_KEY_POSITION, JSON.stringify({ x, y }));
  } catch {
    // 忽略
  }
}

/**
 * Agent 迷你面板
 *
 * - 右下角浮动（fixed 定位），可拖拽移动，位置持久化
 * - 未打开时显示浮动圆形按钮，点击展开迷你面板
 * - 响应窗口 resize，自动修正位置避免跑出屏幕
 * - 显示迷你对话输入框 + 最近消息预览
 * - 底部工具栏：展开全尺寸面板按钮 + 新建对话 + 更多操作
 */
export function AgentMiniPanel() {
  const { t } = useTranslation();
  const { message: messageApi } = App.useApp();
  const isOpen = useAgentPanelStore((s) => s.isOpen);
  const isMiniMode = useAgentPanelStore((s) => s.isMiniMode);
  const open = useAgentPanelStore((s) => s.open);

  // 迷你对话复用 conversation store（与 AgentChatTab 同一发送链路）
  const activeConversationId = useConversationStore((s) => s.activeConversationId);
  const messages = useConversationStore((s) => s.messages);
  const loading = useConversationStore((s) => s.loading);
  const createConversation = useConversationStore((s) => s.createConversation);
  const setActiveConversation = useConversationStore((s) => s.setActiveConversation);
  const sendMessage = useConversationStore((s) => s.sendMessage);
  const fetchMessages = useConversationStore((s) => s.fetchMessages);
  const providers = useProviderStore((s) => s.providers);
  const settings = useSettingsStore((s) => s.settings);

  const [position, setPosition] = useState(loadPersistedPosition);
  const [isMiniExpanded, setIsMiniExpanded] = useState(false);
  const [input, setInput] = useState("");
  const [sending, setSending] = useState(false);

  const draggingRef = useRef(false);
  const dragOffsetRef = useRef({ x: 0, y: 0 });
  const positionRef = useRef(position);

  // 展开时若活跃会话尚无消息，加载最近消息
  useEffect(() => {
    if (isMiniExpanded && activeConversationId && messages.length === 0) {
      fetchMessages(activeConversationId).catch(() => {});
    }
  }, [isMiniExpanded, activeConversationId, fetchMessages, messages.length]);

  // 同步 positionRef，避免拖拽闭包读取过期值
  useEffect(() => {
    positionRef.current = position;
  }, [position]);

  const clampPosition = useCallback((x: number, y: number) => {
    return {
      x: Math.max(0, Math.min(window.innerWidth - MINI_WIDTH, x)),
      y: Math.max(0, Math.min(window.innerHeight - MINI_HEIGHT, y)),
    };
  }, []);

  useEffect(() => {
    const handleResize = () => {
      setPosition((prev) => clampPosition(prev.x, prev.y));
    };
    window.addEventListener("resize", handleResize);
    return () => window.removeEventListener("resize", handleResize);
  }, [clampPosition]);

  // 拖拽时注册 document 监听器，卸载时自动清理（防止拖拽中组件卸载导致泄漏）
  const dragListenersRef = useRef<{ move: (ev: MouseEvent) => void; up: () => void } | null>(null);

  const cleanupDragListeners = useCallback(() => {
    if (dragListenersRef.current) {
      document.removeEventListener("mousemove", dragListenersRef.current.move);
      document.removeEventListener("mouseup", dragListenersRef.current.up);
      dragListenersRef.current = null;
    }
    document.body.style.cursor = "";
    document.body.style.userSelect = "";
  }, []);

  useEffect(() => {
    return () => {
      cleanupDragListeners();
    };
  }, [cleanupDragListeners]);

  const handleHeaderMouseDown = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    draggingRef.current = true;
    dragOffsetRef.current = {
      x: e.clientX - position.x,
      y: e.clientY - position.y,
    };

    const handleMouseMove = (ev: MouseEvent) => {
      if (!draggingRef.current) { return; }
      const next = clampPosition(
        ev.clientX - dragOffsetRef.current.x,
        ev.clientY - dragOffsetRef.current.y,
      );
      setPosition(next);
    };

    const handleMouseUp = () => {
      draggingRef.current = false;
      cleanupDragListeners();
      // 从 positionRef 读取最新值，而非 mousedown 时捕获的过期值
      persistPosition(positionRef.current.x, positionRef.current.y);
    };

    dragListenersRef.current = { move: handleMouseMove, up: handleMouseUp };
    document.body.style.cursor = "move";
    document.body.style.userSelect = "none";
    document.addEventListener("mousemove", handleMouseMove);
    document.addEventListener("mouseup", handleMouseUp);
  }, [position, clampPosition, cleanupDragListeners]);

  const handleNewConversation = useCallback(async () => {
    // 优先取系统默认模型
    let provider: ProviderConfig | undefined;
    let model: Model | undefined;

    if (settings?.defaultProviderId && settings?.defaultModelId) {
      const defaultModel = findModelByIds(
        providers,
        settings.defaultProviderId,
        settings.defaultModelId,
      );
      if (defaultModel?.enabled) {
        provider = providers.find((p) => p.id === settings.defaultProviderId);
        model = defaultModel;
      }
    }

    // 回退到第一个启用的 provider
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

    try {
      const conv = await createConversation(
        t("agentPanel.chatDefaultTitle"),
        model.modelId,
        provider.id,
        { mode: "agent" },
      );
      setActiveConversation(conv.id);
    } catch {
      messageApi.error(t("chat.createConversationFailed"));
    }
  }, [createConversation, providers, settings, messageApi, setActiveConversation, t]);

  const handleSend = useCallback(async () => {
    const text = input.trim();
    if (!text || !activeConversationId) { return; }
    setSending(true);
    setInput("");
    try {
      await sendMessage(text);
    } catch {
      messageApi.error(t("common.failed"));
    } finally {
      setSending(false);
    }
  }, [input, activeConversationId, sendMessage, messageApi, t]);

  const handleKeyDown = useCallback((e: React.KeyboardEvent) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      handleSend();
    }
  }, [handleSend]);

  if (!isMiniMode || isOpen) {
    return null;
  }

  if (!isMiniExpanded) {
    return createPortal(
      <button
        type="button"
        className="fixed z-[999] w-12 h-12 rounded-full bg-[var(--color-primary)] text-white shadow-lg flex items-center justify-center hover:scale-110 transition-transform cursor-pointer"
        style={{
          right: BUTTON_MARGIN,
          bottom: 80,
        }}
        onClick={() => setIsMiniExpanded(true)}
        aria-label={t("agentPanel.openMiniPanel")}
      >
        <Bot size={22} />
      </button>,
      document.body,
    );
  }

  return createPortal(
    <div
      className="fixed z-[998] bg-[var(--color-bg-elevated)] rounded-lg shadow-2xl border border-[var(--border-color)] flex flex-col overflow-hidden"
      style={{
        left: position.x,
        top: position.y,
        width: MINI_WIDTH,
        height: MINI_HEIGHT,
      }}
    >
      <div
        className="flex items-center justify-between px-3 py-2 bg-[var(--color-bg-container)] border-b border-[var(--border-color)] cursor-move shrink-0"
        onMouseDown={handleHeaderMouseDown}
      >
        <div className="flex items-center gap-2">
          <Bot size={16} className="text-[var(--color-primary)]" />
          <span className="text-sm font-medium">{t("agentPanel.panelTitle")}</span>
        </div>
        <button
          type="button"
          className="p-0.5 rounded hover:bg-[var(--color-fill-alter)] text-[var(--color-text-secondary)]"
          onClick={() => setIsMiniExpanded(false)}
          aria-label={t("common.close")}
        >
          <svg
            width="14"
            height="14"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth={2}
            strokeLinecap="round"
          >
            <line x1={18} y1={6} x2={6} y2={18} />
            <line x1={6} y1={6} x2={18} y2={18} />
          </svg>
        </button>
      </div>

      <div className="flex-1 flex flex-col overflow-hidden">
        {!activeConversationId
          ? (
            <div className="flex-1 flex flex-col items-center justify-center gap-3 p-4">
              <Bot size={40} className="opacity-30 text-(--color-text-secondary)" />
              <Button
                type="primary"
                size="small"
                icon={<Plus size={14} />}
                onClick={handleNewConversation}
              >
                {t("agentPanel.newChat")}
              </Button>
            </div>
          )
          : (
            <>
              <div className="flex-1 overflow-auto px-3 py-2 space-y-2">
                {loading
                  ? (
                    <div className="flex items-center justify-center h-full">
                      <Spin size="small" />
                    </div>
                  )
                  : messages.length === 0
                  ? (
                    <div className="flex items-center justify-center h-full">
                      <span className="text-xs text-(--color-text-secondary)">
                        {t("agentPanel.chatEmptyHint")}
                      </span>
                    </div>
                  )
                  : messages.slice(-6).map((msg) => (
                    <div
                      key={msg.id}
                      className={`flex ${msg.role === "user" ? "justify-end" : "justify-start"}`}
                    >
                      <div
                        className={`max-w-[85%] rounded-lg px-2.5 py-1.5 text-xs ${
                          msg.role === "user"
                            ? "bg-(--color-primary) text-white"
                            : "bg-(--color-fill-alter) text-(--color-text)"
                        }`}
                      >
                        {msg.content}
                      </div>
                    </div>
                  ))}
              </div>
              <div className="shrink-0 border-t border-(--border-color) p-2 flex gap-2">
                <Input
                  size="small"
                  value={input}
                  onChange={(e) => setInput(e.target.value)}
                  onKeyDown={handleKeyDown}
                  placeholder={t("agentPanel.chatInputPlaceholder")}
                  disabled={sending}
                />
                <Button
                  type="primary"
                  size="small"
                  icon={<Send size={14} />}
                  onClick={handleSend}
                  loading={sending}
                  disabled={!input.trim()}
                />
              </div>
            </>
          )}
      </div>

      <div className="flex items-center justify-between px-3 py-2 border-t border-[var(--border-color)] bg-[var(--color-bg-container)] shrink-0">
        <button
          type="button"
          className="p-1.5 rounded hover:bg-[var(--color-fill-alter)] text-[var(--color-text-secondary)]"
          title={t("agentPanel.newChat")}
        >
          <Plus size={16} />
        </button>
        <div className="flex items-center gap-1">
          <button
            type="button"
            className="p-1.5 rounded hover:bg-[var(--color-fill-alter)] text-[var(--color-text-secondary)]"
            title={t("common.more")}
          >
            <Ellipsis size={16} />
          </button>
          <button
            type="button"
            className="p-1.5 rounded hover:bg-[var(--color-primary)]/10 text-[var(--color-primary)]"
            title={t("agentPanel.expandFullPanel")}
            onClick={() => {
              setIsMiniExpanded(false);
              open();
            }}
          >
            <Expand size={16} />
          </button>
        </div>
      </div>
    </div>,
    document.body,
  );
}

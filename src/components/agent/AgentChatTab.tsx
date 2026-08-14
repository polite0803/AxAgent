// SPDX-License-Identifier: AGPL-3.0-only

import { findModelByIds } from "@/lib/modelCapabilities";
import { useConversationStore, useProviderStore, useSettingsStore } from "@/stores";
import type { Model, ProviderConfig } from "@/types";
import { App, Button, Empty, Input, Spin, Typography } from "antd";
import { Bot, Plus, Send } from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";

const { Text } = Typography;

/**
 * 对话标签页 — 简易 Agent 对话
 *
 * - 无活跃会话时：显示"新建对话"按钮
 * - 有活跃会话时：显示消息列表 + 输入框
 * - 使用 sendAgentMessage 发送消息
 */
export function AgentChatTab() {
  const { t } = useTranslation();
  const { message: messageApi } = App.useApp();

  const activeConversationId = useConversationStore((s) => s.activeConversationId);
  const messages = useConversationStore((s) => s.messages);
  const loading = useConversationStore((s) => s.loading);
  const createConversation = useConversationStore((s) => s.createConversation);
  const setActiveConversation = useConversationStore((s) => s.setActiveConversation);
  const sendMessage = useConversationStore((s) => s.sendMessage);
  const fetchMessages = useConversationStore((s) => s.fetchMessages);
  const providers = useProviderStore((s) => s.providers);
  const settings = useSettingsStore((s) => s.settings);

  const [input, setInput] = useState("");
  const [sending, setSending] = useState(false);
  const listRef = useRef<HTMLDivElement>(null);

  // 切换活跃会话时加载消息
  useEffect(() => {
    if (activeConversationId && messages.length === 0) {
      fetchMessages(activeConversationId).catch(() => {});
    }
  }, [activeConversationId, fetchMessages, messages.length]);

  // 新消息时自动滚动到底部
  useEffect(() => {
    if (listRef.current) {
      listRef.current.scrollTop = listRef.current.scrollHeight;
    }
  }, [messages]);

  const handleNewConversation = useCallback(async () => {
    // 优先取系统默认模型
    let provider: ProviderConfig | undefined;
    let model: Model | undefined;

    if (settings?.default_provider_id && settings?.default_model_id) {
      const defaultModel = findModelByIds(
        providers,
        settings.default_provider_id,
        settings.default_model_id,
      );
      if (defaultModel?.enabled) {
        provider = providers.find((p) => p.id === settings.default_provider_id);
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
        model.model_id,
        provider.id,
        { mode: "agent" },
      );
      setActiveConversation(conv.id);
    } catch {
      messageApi.error(t("chat.createConversationFailed"));
    }
  }, [createConversation, providers, settings, messageApi, setActiveConversation, t]);

  // 新建后若仍未创建成功（无 provider），显示设置引导
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

  // ── 无活跃会话：新建入口 ──
  if (!activeConversationId) {
    const hasModels = providers.some(
      (p) => p.enabled && p.models.some((m) => m.enabled),
    );

    if (!hasModels) {
      return (
        <div className="flex items-center justify-center h-full p-6">
          <Empty
            image={Empty.PRESENTED_IMAGE_SIMPLE}
            description={
              <span className="text-(--color-text-secondary)">
                {t("chat.noModelsAvailable")}
              </span>
            }
          />
        </div>
      );
    }

    return (
      <div className="flex flex-col items-center justify-center h-full p-6 gap-3">
        <Bot size={40} className="text-(--color-text-quaternary)" />
        <Text type="secondary" className="text-sm">{t("agentPanel.chatStartHint")}</Text>
        <Button
          type="primary"
          icon={<Plus size={14} />}
          onClick={handleNewConversation}
        >
          {t("agentPanel.chatNewConversation")}
        </Button>
      </div>
    );
  }

  // ── 加载中 ──
  if (loading) {
    return (
      <div className="flex items-center justify-center h-full">
        <Spin />
      </div>
    );
  }

  // ── 消息区 + 输入框 ──
  return (
    <div className="flex flex-col h-full">
      <div ref={listRef} className="flex-1 overflow-auto px-3 py-2 space-y-2">
        {messages.length === 0 && (
          <div className="flex items-center justify-center h-full">
            <Text type="secondary" className="text-xs">{t("agentPanel.chatEmptyHint")}</Text>
          </div>
        )}
        {messages.map((msg) => (
          <div
            key={msg.id}
            className={`flex ${msg.role === "user" ? "justify-end" : "justify-start"}`}
          >
            <div
              className={`max-w-[85%] rounded-lg px-3 py-1.5 text-sm ${
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
    </div>
  );
}

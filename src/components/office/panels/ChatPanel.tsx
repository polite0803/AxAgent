// i18n-exempt: 业务逻辑/格式化/日志字符串，非 UI 展示文本
// SPDX-License-Identifier: AGPL-3.0-only

/**
 * ChatPanel — 群聊智能路由面板。
 *
 * 用户输入消息 → 调用 store.dispatch → 展示返回的事件流
 * （routing / agent_message / complete / error）。
 */

import { useOfficeStore, useStockAnalysisStore } from "@/stores";
import type { DispatchEvent } from "@/types";
import { Button, Input, Space, Spin, Tag, theme, Tooltip, Typography } from "antd";
import { Send } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";

const { Text } = Typography;

export function ChatPanel({ fleetId }: { fleetId: string }) {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const dispatch = useOfficeStore((s) => s.dispatch);
  const events = useOfficeStore((s) => s.dispatchEvents);
  const loading = useOfficeStore((s) => s.loading);
  const clearEvents = useOfficeStore((s) => s.clearDispatchEvents);

  // 注入股票业务上下文：用户在 stock-analysis 页选中的当前股票
  const stockCode = useStockAnalysisStore((s) => s.stockCode);
  const stockName = useStockAnalysisStore((s) => s.stockName);
  const quote = useStockAnalysisStore((s) => s.quote);

  const [input, setInput] = useState("");
  const [sending, setSending] = useState(false);
  const listRef = useRef<HTMLDivElement>(null);

  // 切换 fleet 时清空事件流
  useEffect(() => {
    clearEvents();
  }, [fleetId, clearEvents]);

  // 事件流更新时滚动到底部
  useEffect(() => {
    if (listRef.current) {
      listRef.current.scrollTop = listRef.current.scrollHeight;
    }
  }, [events]);

  // 构造业务上下文后缀：若当前选中有股票，自动追加到 user_message 末尾
  // 让 dispatcher LLM 能感知"用户正在看哪只股票"
  const buildBusinessContextSuffix = (): string => {
    if (!stockCode) {
      return "";
    }
    const parts: string[] = [`[当前股票上下文] 代码=${stockCode}`];
    if (stockName) {
      parts.push(`名称=${stockName}`);
    }
    if (quote) {
      parts.push(`现价=${quote.price}`);
      parts.push(`涨跌幅=${quote.changePct}%`);
      if (quote.pe != null) {
        parts.push(`PE=${quote.pe}`);
      }
      if (quote.pb != null) {
        parts.push(`PB=${quote.pb}`);
      }
    }
    return `\n${parts.join(" ")}`;
  };

  const handleSend = async () => {
    const msg = input.trim();
    if (!msg || sending) {
      return;
    }
    const ctxSuffix = buildBusinessContextSuffix();
    const finalMsg = ctxSuffix ? `${msg}\n${ctxSuffix}` : msg;
    setSending(true);
    setInput("");
    try {
      await dispatch({ fleetId, userMessage: finalMsg });
    } finally {
      setSending(false);
    }
  };

  // 显示当前注入的股票上下文提示
  const hasContext = Boolean(stockCode);

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%", gap: 8 }}>
      {/* 事件流 */}
      <div
        ref={listRef}
        style={{
          flex: 1,
          overflow: "auto",
          padding: 8,
          background: token.colorBgLayout,
          borderRadius: 6,
          border: `1px solid ${token.colorBorderSecondary}`,
          minHeight: 200,
        }}
      >
        {events.length === 0
          ? (
            <div style={{ textAlign: "center", color: token.colorTextQuaternary, fontSize: 12, padding: 24 }}>
              {t("office.chat.emptyHint")}
            </div>
          )
          : <EventList events={events} />}
      </div>

      {/* 股票上下文提示条 */}
      {hasContext && (
        <Tooltip title={t("office.chat.contextInjectedHint")}>
          <div
            style={{
              display: "flex",
              alignItems: "center",
              gap: 6,
              padding: "4px 8px",
              background: token.colorPrimaryBg,
              borderRadius: 4,
              fontSize: 11,
              color: token.colorPrimary,
            }}
          >
            <Tag color="blue" style={{ margin: 0, fontSize: 10 }}>
              {stockCode}
            </Tag>
            <span>
              {stockName}
              {quote ? ` · ${quote.price} (${quote.changePct}%)` : ""}
            </span>
          </div>
        </Tooltip>
      )}

      {/* 输入栏 */}
      <Space.Compact style={{ width: "100%" }}>
        <Input.TextArea
          value={input}
          onChange={(e) => setInput(e.target.value)}
          placeholder={t("office.chat.inputPlaceholder")}
          autoSize={{ minRows: 1, maxRows: 4 }}
          disabled={sending}
          onPressEnter={(e) => {
            if (!e.shiftKey) {
              e.preventDefault();
              void handleSend();
            }
          }}
        />
        <Button
          type="primary"
          icon={<Send size={14} />}
          loading={sending}
          onClick={handleSend}
          disabled={!input.trim()}
        >
          {t("office.chat.send")}
        </Button>
      </Space.Compact>

      {/* 状态栏 */}
      <div style={{ display: "flex", justifyContent: "space-between", fontSize: 11, color: token.colorTextTertiary }}>
        <Text type="secondary" style={{ fontSize: 11 }}>
          {t("office.chat.routingHint")}
        </Text>
        {loading && <Spin size="small" />}
      </div>
    </div>
  );
}

function EventList({ events }: { events: DispatchEvent[] }) {
  const { t } = useTranslation();
  const { token } = theme.useToken();

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
      {events.map((e, i) => {
        switch (e.type) {
          case "routing":
            return (
              <div key={i} style={{ fontSize: 11, color: token.colorTextTertiary }}>
                <Tag color="blue" style={{ fontSize: 10 }}>
                  {t("office.chat.tagRouting")}
                </Tag>
                <span>
                  {t("office.chat.routingMessage", { slug: e.agentSlug })}
                </span>
              </div>
            );
          case "agent_message":
            return (
              <div
                key={i}
                style={{
                  fontSize: 12,
                  padding: "6px 8px",
                  background: token.colorBgContainer,
                  borderRadius: 4,
                  border: `1px solid ${token.colorBorderSecondary}`,
                }}
              >
                <div style={{ fontWeight: 600, color: token.colorPrimary, marginBottom: 2, fontSize: 11 }}>
                  {e.agentSlug}
                </div>
                <div style={{ color: token.colorText, whiteSpace: "pre-wrap" }}>
                  {e.content}
                </div>
              </div>
            );
          case "process":
            return (
              <div key={i} style={{ fontSize: 11, color: token.colorTextQuaternary }}>
                <Tag color="purple" style={{ fontSize: 10 }}>
                  {t("office.chat.tagProcess")}
                </Tag>
                <span>{e.status}</span>
              </div>
            );
          case "token_usage":
            return (
              <div key={i} style={{ fontSize: 10, color: token.colorTextQuaternary }}>
                {t("office.chat.tokenUsage", {
                  slug: e.agentSlug,
                  input: e.inputTokens,
                  output: e.outputTokens,
                })}
              </div>
            );
          case "complete":
            return (
              <div key={i} style={{ fontSize: 11, color: "#52c41a", fontWeight: 500 }}>
                ✓ {t("office.chat.completed")}
              </div>
            );
          case "error":
            return (
              <div key={i} style={{ fontSize: 11, color: "#ff4d4f" }}>
                ✗ {e.message}
              </div>
            );
          default:
            return null;
        }
      })}
    </div>
  );
}

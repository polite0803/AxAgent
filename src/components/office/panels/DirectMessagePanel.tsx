// SPDX-License-Identifier: AGPL-3.0-only

/**
 * DirectMessagePanel — 直接 DM 指定 agent 面板。
 *
 * 用户在 AgentCard 上点击 → 切换到此面板 → 输入消息 →
 * store.directMessage → 展示该 agent 的回复。
 */

import { useOfficeStore } from "@/stores";
import type { DispatchEvent, FleetMember } from "@/types";
import { Button, Input, Space, Tag, theme, Typography } from "antd";
import { Send } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";

const { Text } = Typography;

export interface DirectMessagePanelProps {
  fleetId: string;
  target: FleetMember | null;
  /** 返回成员列表的回调（用户点击「返回列表」） */
  onBack?: () => void;
}

export function DirectMessagePanel({ fleetId, target, onBack }: DirectMessagePanelProps) {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const directMessage = useOfficeStore((s) => s.directMessage);
  const events = useOfficeStore((s) => s.dispatchEvents);
  const clearEvents = useOfficeStore((s) => s.clearDispatchEvents);

  const [input, setInput] = useState("");
  const [sending, setSending] = useState(false);
  const listRef = useRef<HTMLDivElement>(null);

  // 切换目标 agent 时清空
  useEffect(() => {
    clearEvents();
  }, [target?.id, clearEvents]);

  useEffect(() => {
    if (listRef.current) {
      listRef.current.scrollTop = listRef.current.scrollHeight;
    }
  }, [events]);

  const handleSend = async () => {
    if (!target) { return; }
    const msg = input.trim();
    if (!msg || sending) {
      return;
    }
    setSending(true);
    setInput("");
    try {
      await directMessage({ fleetId, agentSlug: target.agentSlug, userMessage: msg });
    } finally {
      setSending(false);
    }
  };

  if (!target) {
    return (
      <div style={{ padding: 24, textAlign: "center", color: token.colorTextQuaternary, fontSize: 12 }}>
        {t("office.dm.selectTarget")}
        {onBack && (
          <div style={{ marginTop: 12 }}>
            <Button size="small" onClick={onBack}>
              {t("office.dm.backToList")}
            </Button>
          </div>
        )}
      </div>
    );
  }

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%", gap: 8 }}>
      {/* Header */}
      <div style={{ display: "flex", alignItems: "center", gap: 8, padding: "8px 4px" }}>
        {onBack && (
          <Button size="small" type="text" onClick={onBack}>
            ←
          </Button>
        )}
        <Tag color="blue">{target.agentSlug}</Tag>
        <Text strong style={{ fontSize: 13 }}>
          {target.displayName}
        </Text>
      </div>

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
              {t("office.dm.emptyHint", { slug: target.agentSlug })}
            </div>
          )
          : <DMEventList events={events} />}
      </div>

      {/* 输入栏 */}
      <Space.Compact style={{ width: "100%" }}>
        <Input.TextArea
          value={input}
          onChange={(e) => setInput(e.target.value)}
          placeholder={t("office.dm.inputPlaceholder", { slug: target.agentSlug })}
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
          {t("office.dm.send")}
        </Button>
      </Space.Compact>
    </div>
  );
}

function DMEventList({ events }: { events: DispatchEvent[] }) {
  const { token } = theme.useToken();
  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
      {events.map((e, i) => {
        if (e.type === "agent_message") {
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
        }
        if (e.type === "error") {
          return (
            <div key={i} style={{ fontSize: 11, color: "#ff4d4f" }}>
              ✗ {e.message}
            </div>
          );
        }
        if (e.type === "complete") {
          return (
            <div key={i} style={{ fontSize: 11, color: "#52c41a" }}>
              ✓
            </div>
          );
        }
        return null;
      })}
    </div>
  );
}

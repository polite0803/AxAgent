// SPDX-License-Identifier: AGPL-3.0-only

// 通知铃铛 — 显示 Agent 生命周期通知和未读计数
// 使用统一的 notification 工具库，与 NotificationCenter 共享数据

/* eslint-disable react-refresh/only-export-components */
import { DropdownMenu } from "@/components/layout/DropdownMenu";
import { useCopyToClipboard } from "@/hooks/useCopyToClipboard";
import {
  addNotification,
  clearAllNotifications,
  getNotifications,
  markAllAsRead,
  type Notification,
} from "@/lib/notification";
import { BellOutlined } from "@ant-design/icons";
import { Badge, Empty, theme, Typography } from "antd";
import { Check, CheckCheck, Copy, Trash2 } from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";

const { Text } = Typography;

/** 推送一条通知（兼容旧 API，供 agentStore 等调用） */
export function pushNotification(
  type: Notification["type"],
  message: string,
) {
  addNotification({ type, title: message });
}

export function NotificationBell() {
  const [notifications, setNotifications] = useState<Notification[]>(getNotifications);
  const [open, setOpen] = useState(false);
  const { t, i18n } = useTranslation();
  const { token } = theme.useToken();
  const { copy } = useCopyToClipboard();
  const [copiedId, setCopiedId] = useState<string | null>(null);
  const copyTimerRef = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);

  // 监听 CustomEvent 实时更新
  const handleNotificationEvent = useCallback(() => {
    setNotifications(getNotifications());
  }, []);

  useEffect(() => {
    window.addEventListener("axagent:notification", handleNotificationEvent);
    return () => {
      window.removeEventListener("axagent:notification", handleNotificationEvent);
    };
  }, [handleNotificationEvent]);

  const unreadCount = notifications.filter((n) => !n.read).length;

  const handleMarkAllRead = useCallback(() => {
    setNotifications(markAllAsRead());
  }, []);

  const handleClearAll = useCallback(() => {
    setNotifications(clearAllNotifications());
  }, []);

  const handleCopy = useCallback(
    async (n: Notification) => {
      const text = [n.title, n.message].filter(Boolean).join("\n");
      const ok = await copy(text);
      if (!ok) {
        return;
      }
      setCopiedId(n.id);
      if (copyTimerRef.current) {
        clearTimeout(copyTimerRef.current);
      }
      copyTimerRef.current = setTimeout(() => {
        setCopiedId((cur) => (cur === n.id ? null : cur));
      }, 1500);
    },
    [copy],
  );

  useEffect(() => {
    return () => {
      if (copyTimerRef.current) {
        clearTimeout(copyTimerRef.current);
      }
    };
  }, []);

  const actionItems = notifications.length === 0 ? [] : [
    {
      key: "markAllRead",
      label: <span>{t("notification.markAllRead")}</span>,
      icon: <CheckCheck size={14} />,
      onClick: handleMarkAllRead,
      disabled: unreadCount === 0,
    },
    {
      key: "clearAll",
      label: <span style={{ color: token.colorError }}>{t("notification.clear")}</span>,
      icon: <Trash2 size={14} color={token.colorError} />,
      onClick: handleClearAll,
      danger: true,
    },
    { key: "divider-actions", divider: true },
  ];

  const notificationItems = notifications.length === 0
    ? [
      {
        key: "empty",
        label: (
          <Empty
            description={t("notification.empty")}
            image={Empty.PRESENTED_IMAGE_SIMPLE}
          />
        ),
        disabled: true,
      },
    ]
    : notifications.slice(0, 20).map((n) => ({
      key: n.id,
      label: (
        <div
          style={{
            display: "flex",
            alignItems: "flex-start",
            gap: 8,
            maxWidth: 320,
            padding: "4px 0",
          }}
        >
          <div style={{ flex: 1, minWidth: 0 }}>
            <Text
              style={{
                fontSize: 12,
                color: n.type === "error"
                  ? token.colorError
                  : n.type === "warning"
                  ? token.colorWarning
                  : token.colorSuccess,
              }}
            >
              {n.type === "error" ? "❌" : n.type === "warning" ? "⚠️" : "✅"} {n.title}
            </Text>
            <div>
              <Text type="secondary" style={{ fontSize: 12 }}>
                {new Date(n.timestamp).toLocaleTimeString(i18n.language)}
              </Text>
            </div>
          </div>
          <span
            role="button"
            tabIndex={0}
            aria-label={t("common.copy")}
            title={t("common.copy")}
            onClick={(e) => {
              e.stopPropagation();
              e.preventDefault();
              void handleCopy(n);
            }}
            onKeyDown={(e) => {
              if (e.key === "Enter" || e.key === " ") {
                e.stopPropagation();
                e.preventDefault();
                void handleCopy(n);
              }
            }}
            style={{
              cursor: "pointer",
              display: "flex",
              alignItems: "center",
              flexShrink: 0,
              marginTop: 2,
              color: copiedId === n.id
                ? token.colorSuccess
                : token.colorTextQuaternary,
            }}
          >
            {copiedId === n.id ? <Check size={13} /> : <Copy size={13} />}
          </span>
        </div>
      ),
    }));

  const items = [...actionItems, ...notificationItems];

  return (
    <DropdownMenu items={items} open={open} onOpenChange={setOpen} trigger={["click"]}>
      <Badge count={unreadCount} size="small" offset={[-2, 2]}>
        <BellOutlined style={{ fontSize: 16, cursor: "pointer", padding: 4 }} />
      </Badge>
    </DropdownMenu>
  );
}

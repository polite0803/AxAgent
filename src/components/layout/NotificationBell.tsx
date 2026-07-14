// SPDX-License-Identifier: AGPL-3.0-only

// 通知铃铛 — 显示 Agent 生命周期通知和未读计数
// 使用统一的 notification 工具库，与 NotificationCenter 共享数据

import { DropdownMenu } from "@/components/layout/DropdownMenu";
import { addNotification, getNotifications, type Notification } from "@/lib/notification";
import { BellOutlined } from "@ant-design/icons";
import { Badge, Empty, theme, Typography } from "antd";
import { useCallback, useEffect, useState } from "react";
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

  const items = notifications.length === 0
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
        <div style={{ maxWidth: 320, padding: "4px 0" }}>
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
      ),
    }));

  return (
    <DropdownMenu items={items} open={open} onOpenChange={setOpen} trigger={["click"]}>
      <Badge count={unreadCount} size="small" offset={[-2, 2]}>
        <BellOutlined style={{ fontSize: 16, cursor: "pointer", padding: 4 }} />
      </Badge>
    </DropdownMenu>
  );
}

// 通知铃铛 — 显示 Agent 生命周期通知和未读计数

import { BellOutlined } from "@ant-design/icons";
import { Badge, Dropdown, Empty, Typography } from "antd";
import { useCallback, useState } from "react";

const { Text } = Typography;

interface NotificationItem {
  id: string;
  type: "success" | "error" | "warning" | "info";
  message: string;
  time: number;
}

// 全局通知列表（模块级，跨组件共享）
const globalNotifications: NotificationItem[] = [];
let listeners: Array<() => void> = [];

export function pushNotification(type: NotificationItem["type"], message: string) {
  const item: NotificationItem = {
    id: `${Date.now()}-${Math.random().toString(36).slice(2, 8)}`,
    type,
    message,
    time: Date.now(),
  };
  globalNotifications.unshift(item);
  // 保留最近 50 条
  if (globalNotifications.length > 50) { globalNotifications.length = 50; }
  listeners.forEach((fn) => fn());
}

export function NotificationBell() {
  const [, setTick] = useState(0);
  const [open, setOpen] = useState(false);

  // 订阅全局通知变化
  const refresh = useCallback(() => setTick((n) => n + 1), []);
  useState(() => {
    listeners.push(refresh);
    return () => {
      listeners = listeners.filter((l) => l !== refresh);
    };
  });

  const unreadCount = globalNotifications.length;

  const items = globalNotifications.length === 0
    ? [{ key: "empty", label: <Empty description="暂无通知" image={Empty.PRESENTED_IMAGE_SIMPLE} />, disabled: true }]
    : globalNotifications.slice(0, 20).map((n) => ({
      key: n.id,
      label: (
        <div style={{ maxWidth: 320, padding: "4px 0" }}>
          <Text
            style={{
              fontSize: 12,
              color: n.type === "error" ? "#ff4d4f" : n.type === "warning" ? "#faad14" : "#52c41a",
            }}
          >
            {n.type === "error" ? "❌" : n.type === "warning" ? "⚠️" : "✅"} {n.message}
          </Text>
          <div>
            <Text type="secondary" style={{ fontSize: 11 }}>
              {new Date(n.time).toLocaleTimeString("zh-CN")}
            </Text>
          </div>
        </div>
      ),
    }));

  return (
    <Dropdown menu={{ items }} open={open} onOpenChange={setOpen} trigger={["click"]} placement="bottomRight">
      <Badge count={unreadCount} size="small" offset={[-2, 2]}>
        <BellOutlined style={{ fontSize: 16, cursor: "pointer", padding: 4 }} />
      </Badge>
    </Dropdown>
  );
}

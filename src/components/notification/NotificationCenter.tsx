import {
  Badge,
  Button,
  Empty,
  List,
  Popover,
  Space,
  Typography,
} from "antd";
import {
  Bell,
  Check,
  CheckCheck,
  Info,
  AlertTriangle,
  Trash2,
  X,
} from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

const { Text, Title } = Typography;

export interface Notification {
  id: string;
  type: "info" | "success" | "warning" | "error";
  title: string;
  message?: string;
  timestamp: number;
  read: boolean;
  persistent?: boolean;
  action?: {
    label: string;
    onClick: () => void;
  };
}

interface NotificationCenterProps {
  trigger?: React.ReactNode;
}

export default function NotificationCenter({ trigger }: NotificationCenterProps) {
  const { t } = useTranslation();
  const [notifications, setNotifications] = useState<Notification[]>([]);
  const [visible, setVisible] = useState(false);

  useEffect(() => {
    const stored = localStorage.getItem("axagent-notifications");
    if (stored) {
      try {
        setNotifications(JSON.parse(stored));
      } catch {
        setNotifications([]);
      }
    }
  }, []);

  useEffect(() => {
    localStorage.setItem("axagent-notifications", JSON.stringify(notifications));
  }, [notifications]);

  const unreadCount = notifications.filter((n) => !n.read).length;

  const handleMarkAsRead = (id: string) => {
    setNotifications((prev) =>
      prev.map((n) => (n.id === id ? { ...n, read: true } : n))
    );
  };

  const handleMarkAllAsRead = () => {
    setNotifications((prev) => prev.map((n) => ({ ...n, read: true })));
  };

  const handleDismiss = (id: string) => {
    setNotifications((prev) => prev.filter((n) => n.id !== id));
  };

  const handleClearAll = () => {
    setNotifications((prev) => prev.filter((n) => n.persistent));
  };

  const getIcon = (type: Notification["type"]) => {
    switch (type) {
      case "success":
        return <Check size={16} color="#a6e3a1" />;
      case "warning":
        return <AlertTriangle size={16} color="#f9e2af" />;
      case "error":
        return <X size={16} color="#f38ba8" />;
      default:
        return <Info size={16} color="#89b4fa" />;
    }
  };

  const getColor = (type: Notification["type"]) => {
    switch (type) {
      case "success":
        return "#a6e3a1";
      case "warning":
        return "#f9e2af";
      case "error":
        return "#f38ba8";
      default:
        return "#89b4fa";
    }
  };

  const formatTime = (timestamp: number) => {
    const now = Date.now();
    const diff = now - timestamp;

    if (diff < 60000) return t("notification.justNow");
    if (diff < 3600000) return `${Math.floor(diff / 60000)} ${t("notification.minutesAgo")}`;
    if (diff < 86400000) return `${Math.floor(diff / 3600000)} ${t("notification.hoursAgo")}`;
    return new Date(timestamp).toLocaleDateString();
  };

  const content = (
    <div style={{ width: 360, maxHeight: 480, overflow: "auto" }}>
      <div
        style={{
          display: "flex",
          justifyContent: "space-between",
          alignItems: "center",
          padding: "8px 12px",
          borderBottom: "1px solid var(--border)",
        }}
      >
        <Title level={5} style={{ margin: 0 }}>
          {t("notification.title")}
        </Title>
        <Space>
          {unreadCount > 0 && (
            <Button
              type="text"
              size="small"
              icon={<CheckCheck size={14} />}
              onClick={handleMarkAllAsRead}
            >
              {t("notification.markAllRead")}
            </Button>
          )}
          {notifications.length > 0 && (
            <Button
              type="text"
              size="small"
              danger
              icon={<Trash2 size={14} />}
              onClick={handleClearAll}
            >
              {t("notification.clear")}
            </Button>
          )}
        </Space>
      </div>

      {notifications.length === 0 ? (
        <Empty
          image={Empty.PRESENTED_IMAGE_SIMPLE}
          description={t("notification.empty")}
          style={{ padding: 40 }}
        />
      ) : (
        <List
          dataSource={notifications}
          renderItem={(notification) => (
            <List.Item
              style={{
                padding: "12px 16px",
                background: notification.read ? "transparent" : "var(--surface-hover)",
                cursor: "pointer",
                borderLeft: `3px solid ${getColor(notification.type)}`,
              }}
              onClick={() => handleMarkAsRead(notification.id)}
            >
              <div style={{ width: "100%" }}>
                <div
                  style={{
                    display: "flex",
                    justifyContent: "space-between",
                    alignItems: "flex-start",
                  }}
                >
                  <Space>
                    {getIcon(notification.type)}
                    <div>
                      <Text strong style={{ display: "block" }}>
                        {notification.title}
                      </Text>
                      {notification.message && (
                        <Text
                          type="secondary"
                          style={{ fontSize: 12, display: "block" }}
                        >
                          {notification.message}
                        </Text>
                      )}
                    </div>
                  </Space>
                  <Button
                    type="text"
                    size="small"
                    icon={<X size={12} />}
                    onClick={(e) => {
                      e.stopPropagation();
                      handleDismiss(notification.id);
                    }}
                    style={{ marginLeft: 8 }}
                  />
                </div>
                <div
                  style={{
                    marginTop: 4,
                    display: "flex",
                    justifyContent: "space-between",
                    alignItems: "center",
                  }}
                >
                  <Text type="secondary" style={{ fontSize: 11 }}>
                    {formatTime(notification.timestamp)}
                  </Text>
                  {notification.action && (
                    <Button
                      type="link"
                      size="small"
                      onClick={(e) => {
                        e.stopPropagation();
                        notification.action?.onClick();
                      }}
                    >
                      {notification.action.label}
                    </Button>
                  )}
                </div>
              </div>
            </List.Item>
          )}
        />
      )}
    </div>
  );

  return (
    <Popover
      content={content}
      trigger="click"
      open={visible}
      onOpenChange={setVisible}
      placement="bottomRight"
    >
      {trigger || (
        <Badge count={unreadCount} size="small" offset={[-4, 4]}>
          <Button
            type="text"
            icon={<Bell size={18} />}
            style={{ display: "flex", alignItems: "center", justifyContent: "center" }}
          />
        </Badge>
      )}
    </Popover>
  );
}

export function addNotification(notification: Omit<Notification, "id" | "timestamp" | "read">) {
  const newNotification: Notification = {
    ...notification,
    id: `notif-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`,
    timestamp: Date.now(),
    read: false,
  };

  const stored = localStorage.getItem("axagent-notifications");
  const existing: Notification[] = stored ? JSON.parse(stored) : [];

  existing.unshift(newNotification);
  if (existing.length > 50) {
    existing.splice(50);
  }

  localStorage.setItem("axagent-notifications", JSON.stringify(existing));

  window.dispatchEvent(
    new CustomEvent("axagent:notification", { detail: newNotification })
  );
}

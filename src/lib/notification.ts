// SPDX-License-Identifier: AGPL-3.0-only

// 统一通知工具 — NotificationCenter 和 NotificationBell 共享的通知基础设施

const STORAGE_KEY = "axagent-notifications";
const MAX_NOTIFICATIONS = 50;

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

/** 从 localStorage 读取所有通知 */
export function getNotifications(): Notification[] {
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    return stored ? JSON.parse(stored) : [];
  } catch {
    return [];
  }
}

/** 写入通知到 localStorage */
function saveNotifications(list: Notification[]): void {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(list));
  } catch (e) {
    console.warn("Failed to persist notifications", e);
  }
}

/** 添加一条新通知，同时写入 localStorage 并派发 CustomEvent */
export function addNotification(
  notification: Omit<Notification, "id" | "timestamp" | "read">,
): Notification {
  const newNotification: Notification = {
    ...notification,
    id: `notif-${Date.now()}-${Math.random().toString(36).slice(2, 11)}`,
    timestamp: Date.now(),
    read: false,
  };

  const existing = getNotifications();
  existing.unshift(newNotification);
  if (existing.length > MAX_NOTIFICATIONS) {
    existing.splice(MAX_NOTIFICATIONS);
  }

  saveNotifications(existing);

  window.dispatchEvent(
    new CustomEvent("axagent:notification", { detail: newNotification }),
  );

  return newNotification;
}

/** 标记通知为已读 */
export function markAsRead(id: string): Notification[] {
  const list = getNotifications().map((n) => n.id === id ? { ...n, read: true } : n);
  saveNotifications(list);
  return list;
}

/** 标记全部通知为已读 */
export function markAllAsRead(): Notification[] {
  const list = getNotifications().map((n) => ({ ...n, read: true }));
  saveNotifications(list);
  return list;
}

/** 删除单条通知 */
export function dismissNotification(id: string): Notification[] {
  const list = getNotifications().filter((n) => n.id !== id);
  saveNotifications(list);
  return list;
}

/** 清除所有通知（保留 persistent） */
export function clearAllNotifications(): Notification[] {
  const list = getNotifications().filter((n) => n.persistent);
  saveNotifications(list);
  return list;
}

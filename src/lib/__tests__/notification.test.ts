// SPDX-License-Identifier: AGPL-3.0-only

import { beforeEach, describe, expect, it, vi } from "vitest";

import {
  addNotification,
  clearAllNotifications,
  dismissNotification,
  getNotifications,
  markAllAsRead,
  markAsRead,
  type Notification,
} from "../notification";

function makeNotif(overrides: Partial<Notification> = {}): Notification {
  return {
    id: "notif-1",
    type: "info",
    title: "Test",
    timestamp: Date.now(),
    read: false,
    ...overrides,
  };
}

describe("notification", () => {
  beforeEach(() => {
    localStorage.clear();
  });

  describe("getNotifications", () => {
    it("无存储数据时应返回空数组", () => {
      expect(getNotifications()).toEqual([]);
    });

    it("应返回存储的通知列表", () => {
      const notifs: Notification[] = [makeNotif()];
      localStorage.setItem("axagent-notifications", JSON.stringify(notifs));
      expect(getNotifications()).toEqual(notifs);
    });

    it("解析失败时应返回空数组", () => {
      localStorage.setItem("axagent-notifications", "invalid{{");
      expect(getNotifications()).toEqual([]);
    });
  });

  describe("addNotification", () => {
    it("应创建通知并生成 id、timestamp", () => {
      const notif = addNotification({ type: "success", title: "完成" });

      expect(notif.id).toMatch(/^notif-\d+-[a-z0-9]+$/);
      expect(notif.timestamp).toBeGreaterThan(0);
      expect(notif.read).toBe(false);
      expect(notif.type).toBe("success");
      expect(notif.title).toBe("完成");
    });

    it("应持久化到 localStorage", () => {
      addNotification({ type: "info", title: "Test" });
      const stored = getNotifications();
      expect(stored).toHaveLength(1);
      expect(stored[0].title).toBe("Test");
    });

    it("应派发 CustomEvent", () => {
      const handler = vi.fn();
      window.addEventListener("axagent:notification", handler);

      const notif = addNotification({ type: "info", title: "Test" });

      expect(handler).toHaveBeenCalledTimes(1);
      expect(handler.mock.calls[0][0].detail).toEqual(notif);

      window.removeEventListener("axagent:notification", handler);
    });

    it("应限制最大通知数为 50", () => {
      for (let i = 0; i < 55; i++) {
        addNotification({ type: "info", title: `Notif ${i}` });
      }
      const stored = getNotifications();
      expect(stored).toHaveLength(50);
    });

    it("新通知应排在前面", () => {
      addNotification({ type: "info", title: "First" });
      addNotification({ type: "info", title: "Second" });
      const stored = getNotifications();
      expect(stored[0].title).toBe("Second");
      expect(stored[1].title).toBe("First");
    });
  });

  describe("markAsRead", () => {
    it("应标记指定通知为已读", () => {
      addNotification({ type: "info", title: "A" });
      const notif = addNotification({ type: "info", title: "B" });

      const updated = markAsRead(notif.id);
      const target = updated.find((n) => n.id === notif.id);
      expect(target?.read).toBe(true);
    });

    it("不应影响其他通知", () => {
      const a = addNotification({ type: "info", title: "A" });
      addNotification({ type: "info", title: "B" });

      const updated = markAsRead(a.id);
      const other = updated.find((n) => n.title === "B");
      expect(other?.read).toBe(false);
    });
  });

  describe("markAllAsRead", () => {
    it("应标记所有通知为已读", () => {
      addNotification({ type: "info", title: "A" });
      addNotification({ type: "info", title: "B" });

      const updated = markAllAsRead();
      expect(updated.every((n) => n.read)).toBe(true);
    });
  });

  describe("dismissNotification", () => {
    it("应删除指定通知", () => {
      const notif = addNotification({ type: "info", title: "A" });
      addNotification({ type: "info", title: "B" });

      const updated = dismissNotification(notif.id);
      expect(updated).toHaveLength(1);
      expect(updated[0].title).toBe("B");
    });
  });

  describe("clearAllNotifications", () => {
    it("应保留 persistent 通知", () => {
      addNotification({ type: "info", title: "Normal" });
      addNotification({ type: "info", title: "Persistent", persistent: true });

      const updated = clearAllNotifications();
      expect(updated).toHaveLength(1);
      expect(updated[0].title).toBe("Persistent");
    });
  });
});

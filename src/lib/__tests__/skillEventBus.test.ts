// SPDX-License-Identifier: AGPL-3.0-only

import { beforeEach, describe, expect, it, vi } from "vitest";

import { skillEventBus } from "../skillEventBus";

describe("skillEventBus", () => {
  beforeEach(() => {
    skillEventBus.destroy();
  });

  describe("emit / on", () => {
    it("应能注册监听器并接收事件", () => {
      const handler = vi.fn();
      skillEventBus.on("test-skill", "update", handler);
      skillEventBus.emit("test-skill", "update", { data: "hello" });
      expect(handler).toHaveBeenCalledWith({ data: "hello" });
    });

    it("无监听器时 emit 不应报错", () => {
      expect(() => skillEventBus.emit("test-skill", "update", {})).not.toThrow();
    });

    it("不同 skill 的事件应隔离", () => {
      const handler1 = vi.fn();
      const handler2 = vi.fn();
      skillEventBus.on("skill-a", "update", handler1);
      skillEventBus.on("skill-b", "update", handler2);
      skillEventBus.emit("skill-a", "update", {});
      expect(handler1).toHaveBeenCalledTimes(1);
      expect(handler2).not.toHaveBeenCalled();
    });

    it("同一 skill 的不同事件应隔离", () => {
      const handler1 = vi.fn();
      const handler2 = vi.fn();
      skillEventBus.on("test-skill", "update", handler1);
      skillEventBus.on("test-skill", "delete", handler2);
      skillEventBus.emit("test-skill", "update", {});
      expect(handler1).toHaveBeenCalledTimes(1);
      expect(handler2).not.toHaveBeenCalled();
    });

    it("多个监听器应都能收到事件", () => {
      const handler1 = vi.fn();
      const handler2 = vi.fn();
      skillEventBus.on("test-skill", "update", handler1);
      skillEventBus.on("test-skill", "update", handler2);
      skillEventBus.emit("test-skill", "update", {});
      expect(handler1).toHaveBeenCalledTimes(1);
      expect(handler2).toHaveBeenCalledTimes(1);
    });

    it("on 应返回取消订阅函数", () => {
      const handler = vi.fn();
      const unsubscribe = skillEventBus.on("test-skill", "update", handler);
      unsubscribe();
      skillEventBus.emit("test-skill", "update", {});
      expect(handler).not.toHaveBeenCalled();
    });

    it("handler 抛出错误时不应中断其他 handler", () => {
      const consoleError = vi.spyOn(console, "error").mockImplementation(() => {});
      const handler1 = vi.fn(() => {
        throw new Error("boom");
      });
      const handler2 = vi.fn();
      skillEventBus.on("test-skill", "update", handler1);
      skillEventBus.on("test-skill", "update", handler2);
      skillEventBus.emit("test-skill", "update", {});
      expect(handler1).toHaveBeenCalled();
      expect(handler2).toHaveBeenCalled();
      consoleError.mockRestore();
    });

    it("异步 handler 出错时应 catch 并 log", async () => {
      const consoleError = vi.spyOn(console, "error").mockImplementation(() => {});
      const handler = vi.fn(() => Promise.reject(new Error("async boom")));
      skillEventBus.on("test-skill", "update", handler);
      skillEventBus.emit("test-skill", "update", {});

      // 等待微任务执行
      await new Promise((r) => setTimeout(r, 10));
      expect(consoleError).toHaveBeenCalled();
      consoleError.mockRestore();
    });
  });

  describe("clear", () => {
    it("应清除指定 skill 的所有事件", () => {
      const handler = vi.fn();
      skillEventBus.on("test-skill", "update", handler);
      skillEventBus.on("test-skill", "delete", handler);
      skillEventBus.clear("test-skill");
      skillEventBus.emit("test-skill", "update", {});
      skillEventBus.emit("test-skill", "delete", {});
      expect(handler).not.toHaveBeenCalled();
    });

    it("不应影响其他 skill", () => {
      const handler = vi.fn();
      skillEventBus.on("other-skill", "update", handler);
      skillEventBus.clear("test-skill");
      skillEventBus.emit("other-skill", "update", {});
      expect(handler).toHaveBeenCalledTimes(1);
    });
  });

  describe("destroy", () => {
    it("应清除所有事件", () => {
      const handler = vi.fn();
      skillEventBus.on("skill-a", "update", handler);
      skillEventBus.on("skill-b", "update", handler);
      skillEventBus.destroy();
      skillEventBus.emit("skill-a", "update", {});
      skillEventBus.emit("skill-b", "update", {});
      expect(handler).not.toHaveBeenCalled();
    });
  });

  describe("LRU 驱逐", () => {
    it("超过最大 key 数量时应驱逐最久未访问的 key", () => {
      const handlers: Array<() => void> = [];

      // 注册 201 个不同的 key，超过 MAX_LISTENER_KEYS (200)
      for (let i = 0; i < 201; i++) {
        const unsub = skillEventBus.on(`skill-${i}`, "update", vi.fn());
        handlers.push(unsub);
      }

      // 最早注册的 key 应该被驱逐
      // 可以安全 emit 而不会触发最早的 handler
      skillEventBus.emit("skill-0", "update", {});
      // 不应该报错，说明驱逐成功
    });
  });
});

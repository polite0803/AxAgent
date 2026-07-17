// SPDX-License-Identifier: AGPL-3.0-only

import { describe, expect, it, vi } from "vitest";

import { emitBrowserEvent, onBrowserEvent } from "../browserEvents";

describe("browserEvents", () => {
  describe("emitBrowserEvent", () => {
    it("应派发事件", () => {
      const handler = vi.fn();
      const unsubscribe = onBrowserEvent("test:event", handler);
      emitBrowserEvent("test:event", { data: "hello" });
      expect(handler).toHaveBeenCalledTimes(1);
      expect(handler).toHaveBeenCalledWith({ data: "hello" });
      unsubscribe();
    });

    it("多次派发应分别触发", () => {
      const handler = vi.fn();
      const unsubscribe = onBrowserEvent("test:multi", handler);
      emitBrowserEvent("test:multi", 1);
      emitBrowserEvent("test:multi", 2);
      expect(handler).toHaveBeenCalledTimes(2);
      unsubscribe();
    });
  });

  describe("onBrowserEvent", () => {
    it("应能订阅事件", () => {
      const handler = vi.fn();
      const unsubscribe = onBrowserEvent("test:subscribe", handler);
      emitBrowserEvent("test:subscribe", "payload");
      expect(handler).toHaveBeenCalledWith("payload");
      unsubscribe();
    });

    it("取消订阅后不应再收到事件", () => {
      const handler = vi.fn();
      const unsubscribe = onBrowserEvent("test:unsubscribe", handler);
      unsubscribe();
      emitBrowserEvent("test:unsubscribe", "payload");
      expect(handler).not.toHaveBeenCalled();
    });

    it("不同事件应隔离", () => {
      const handlerA = vi.fn();
      const handlerB = vi.fn();
      onBrowserEvent("test:a", handlerA);
      onBrowserEvent("test:b", handlerB);
      emitBrowserEvent("test:a", {});
      expect(handlerA).toHaveBeenCalledTimes(1);
      expect(handlerB).not.toHaveBeenCalled();
    });
  });

  describe("EventTarget 单例", () => {
    it("多次调用应复用同一个 EventTarget", () => {
      const handler = vi.fn();
      const unsub1 = onBrowserEvent("test:singleton", handler);
      const unsub2 = onBrowserEvent("test:singleton", vi.fn());
      emitBrowserEvent("test:singleton", {});
      expect(handler).toHaveBeenCalledTimes(1);
      unsub1();
      unsub2();
    });
  });
});

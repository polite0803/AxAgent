// SPDX-License-Identifier: AGPL-3.0-only

import { describe, expect, it, vi } from "vitest";

import type { DynamicAction, EventHandler } from "@/types";

import { executeActions, getLifecycleHandlers, handleEvents } from "../dynamicUI/EventHandlerEngine";

vi.mock("@/lib/actionRouter", () => {
  const mockExecute = vi.fn().mockResolvedValue(undefined);
  return {
    getActionRouter: () => ({
      execute: mockExecute,
    }),
    getDefaultNavigate: () => vi.fn(),
  };
});

describe("EventHandlerEngine", () => {
  describe("handleEvents", () => {
    it("应返回空对象当 handlers 为空", () => {
      const bindings = handleEvents([]);
      expect(bindings).toEqual({});
    });

    it("应为每个 handler 生成绑定函数", () => {
      const handlers: EventHandler[] = [
        { trigger: "onClick", actions: [{ type: "navigate", config: { path: "/test" } }] },
        { trigger: "onChange", actions: [{ type: "store", config: {} }] },
      ];
      const bindings = handleEvents(handlers);
      expect(bindings).toHaveProperty("onClick");
      expect(bindings).toHaveProperty("onChange");
      expect(typeof bindings.onClick).toBe("function");
      expect(typeof bindings.onChange).toBe("function");
    });

    it("应跳过 onMount / onUnmount 触发器", () => {
      const handlers: EventHandler[] = [
        { trigger: "onMount", actions: [{ type: "store", config: {} }] },
        { trigger: "onUnmount", actions: [{ type: "store", config: {} }] },
        { trigger: "onClick", actions: [{ type: "navigate", config: { path: "/" } }] },
      ];
      const bindings = handleEvents(handlers);
      expect(bindings).not.toHaveProperty("onMount");
      expect(bindings).not.toHaveProperty("onUnmount");
      expect(bindings).toHaveProperty("onClick");
    });

    it("绑定的函数被调用时不应抛错", () => {
      const handlers: EventHandler[] = [
        { trigger: "onClick", actions: [{ type: "navigate", config: { path: "/test" } }] },
      ];
      const bindings = handleEvents(handlers);
      expect(() => bindings.onClick({ target: "button" })).not.toThrow();
    });
  });

  describe("getLifecycleHandlers", () => {
    it("空数组应返回空 actions", () => {
      const { onMount, onUnmount } = getLifecycleHandlers([]);
      expect(onMount).toEqual([]);
      expect(onUnmount).toEqual([]);
    });

    it("应提取 onMount actions", () => {
      const onMountAction: DynamicAction = { type: "store", config: {} };
      const handlers: EventHandler[] = [
        { trigger: "onMount", actions: [onMountAction] },
      ];
      const { onMount, onUnmount } = getLifecycleHandlers(handlers);
      expect(onMount).toEqual([onMountAction]);
      expect(onUnmount).toEqual([]);
    });

    it("应提取 onUnmount actions", () => {
      const onUnmountAction: DynamicAction = { type: "store", config: {} };
      const handlers: EventHandler[] = [
        { trigger: "onUnmount", actions: [onUnmountAction] },
      ];
      const { onMount, onUnmount } = getLifecycleHandlers(handlers);
      expect(onMount).toEqual([]);
      expect(onUnmount).toEqual([onUnmountAction]);
    });

    it("应同时提取 onMount 和 onUnmount", () => {
      const mountAction: DynamicAction = { type: "store", config: {} };
      const unmountAction: DynamicAction = { type: "navigate", config: { path: "/" } };
      const handlers: EventHandler[] = [
        { trigger: "onMount", actions: [mountAction] },
        { trigger: "onUnmount", actions: [unmountAction] },
        { trigger: "onClick", actions: [{ type: "store", config: {} }] },
      ];
      const { onMount, onUnmount } = getLifecycleHandlers(handlers);
      expect(onMount).toEqual([mountAction]);
      expect(onUnmount).toEqual([unmountAction]);
    });
  });

  describe("executeActions", () => {
    it("空 actions 数组应不抛错", async () => {
      await expect(executeActions([])).resolves.not.toThrow();
    });

    it("应调用 onAction 回调", async () => {
      const onAction = vi.fn();
      const action: DynamicAction = { type: "navigate", config: { path: "/" } };
      await executeActions([action], { onAction });
      expect(onAction).toHaveBeenCalledWith(action);
    });

    it("update-schema 类型应派发 CustomEvent", async () => {
      const handler = vi.fn();
      window.addEventListener("dynamic-ui:schema-update", handler);

      const action: DynamicAction = {
        type: "update-schema",
        config: { schemaId: "test-1", operation: "replace" },
      };
      await executeActions([action]);

      expect(handler).toHaveBeenCalledTimes(1);
      const event = handler.mock.calls[0][0] as CustomEvent;
      expect(event.detail.schemaId).toBe("test-1");
      expect(event.detail.operation).toBe("replace");

      window.removeEventListener("dynamic-ui:schema-update", handler);
    });

    it("update-schema 应使用 scope 隔离", async () => {
      const handler = vi.fn();
      window.addEventListener("dynamic-ui:schema-update", handler);

      const action: DynamicAction = {
        type: "update-schema",
        config: { schemaId: "test-2", operation: "append" },
      };
      await executeActions([action], { scope: "renderer-1" });

      const event = handler.mock.calls[0][0] as CustomEvent;
      expect(event.detail.scope).toBe("renderer-1");

      window.removeEventListener("dynamic-ui:schema-update", handler);
    });
  });
});

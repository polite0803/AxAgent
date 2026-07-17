// SPDX-License-Identifier: AGPL-3.0-only

import { beforeEach, describe, expect, it } from "vitest";

import type { ComponentRegistryEntry } from "@/types";
import { componentRegistry } from "../dynamicUI/ComponentRegistry";

function makeEntry(
  overrides: Partial<ComponentRegistryEntry> = {},
): ComponentRegistryEntry {
  return {
    type: "Input",
    component: () => null,
    category: "form",
    label: "Input",
    ...overrides,
  } as ComponentRegistryEntry;
}

describe("ComponentRegistry", () => {
  beforeEach(() => {
    componentRegistry.clear();
  });

  describe("register / get", () => {
    it("应能注册并获取组件", () => {
      const entry = makeEntry();
      componentRegistry.register(entry);
      expect(componentRegistry.get("Input")).toEqual(entry);
    });

    it("应能通过 namespace 注册和获取", () => {
      const entry = makeEntry();
      componentRegistry.register(entry, "myPlugin");
      expect(componentRegistry.get("myPlugin:Input")).toEqual(expect.objectContaining({ namespace: "myPlugin" }));
      expect(componentRegistry.get("Input")).toEqual(expect.objectContaining({ namespace: "myPlugin" }));
    });

    it("未注册时应返回 undefined", () => {
      expect(componentRegistry.get("Unknown")).toBeUndefined();
    });
  });

  describe("registerBatch", () => {
    it("应批量注册", () => {
      componentRegistry.registerBatch([makeEntry({ type: "Input" }), makeEntry({ type: "Button" })]);
      expect(componentRegistry.has("Input")).toBe(true);
      expect(componentRegistry.has("Button")).toBe(true);
    });
  });

  describe("resolve", () => {
    it("应优先使用 namespace 查找", () => {
      const globalEntry = makeEntry({ type: "Input", category: "form" });
      const nsEntry = makeEntry({ type: "Input", category: "misc" });
      componentRegistry.register(globalEntry);
      componentRegistry.register(nsEntry, "myPlugin");

      const resolved = componentRegistry.resolve("Input", "myPlugin");
      expect(resolved?.category).toBe("misc");
    });

    it("namespace 无匹配时应回退到全局", () => {
      const entry = makeEntry();
      componentRegistry.register(entry);
      expect(componentRegistry.resolve("Input", "otherPlugin")).toEqual(entry);
    });
  });

  describe("getByCategory", () => {
    it("应返回指定分类的组件", () => {
      componentRegistry.register(makeEntry({ type: "Input", category: "form" }));
      componentRegistry.register(makeEntry({ type: "Button", category: "form" }));
      componentRegistry.register(makeEntry({ type: "Card", category: "container" }));

      const formComponents = componentRegistry.getByCategory("form");
      expect(formComponents.length).toBe(2);
    });

    it("无匹配时应返回空数组", () => {
      expect(componentRegistry.getByCategory("nonexistent")).toEqual([]);
    });
  });

  describe("has", () => {
    it("已注册时应返回 true", () => {
      componentRegistry.register(makeEntry());
      expect(componentRegistry.has("Input")).toBe(true);
    });

    it("未注册时应返回 false", () => {
      expect(componentRegistry.has("Unknown")).toBe(false);
    });
  });

  describe("unregister", () => {
    it("应能取消注册", () => {
      componentRegistry.register(makeEntry());
      componentRegistry.unregister("Input");
      expect(componentRegistry.has("Input")).toBe(false);
    });

    it("应能取消 namespace 注册", () => {
      componentRegistry.register(makeEntry(), "myPlugin");
      componentRegistry.unregister("Input", "myPlugin");
      expect(componentRegistry.get("myPlugin:Input")).toBeUndefined();
    });
  });

  describe("unregisterNamespace", () => {
    it("应移除整个 namespace 的注册", () => {
      componentRegistry.register(makeEntry({ type: "Input" }), "myPlugin");
      componentRegistry.register(makeEntry({ type: "Button" }), "myPlugin");
      componentRegistry.unregisterNamespace("myPlugin");
      expect(componentRegistry.get("myPlugin:Input")).toBeUndefined();
      expect(componentRegistry.get("myPlugin:Button")).toBeUndefined();
    });
  });

  describe("getAllTypes", () => {
    it("应返回所有非 namespace 类型", () => {
      componentRegistry.register(makeEntry({ type: "Input" }));
      componentRegistry.register(makeEntry({ type: "Button" }));
      componentRegistry.register(makeEntry({ type: "Card" }), "myPlugin");

      const types = componentRegistry.getAllTypes();
      expect(types).toContain("Input");
      expect(types).toContain("Button");
      expect(types).not.toContain("myPlugin:Card");
    });
  });

  describe("clear", () => {
    it("应清空所有注册", () => {
      componentRegistry.register(makeEntry());
      componentRegistry.clear();
      expect(componentRegistry.has("Input")).toBe(false);
    });
  });
});

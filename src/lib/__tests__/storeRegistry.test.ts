// SPDX-License-Identifier: AGPL-3.0-only

import { beforeEach, describe, expect, it, vi } from "vitest";

import { getStoreRegistry, registerStore, unregisterStore } from "../storeRegistry";

describe("storeRegistry", () => {
  beforeEach(() => {
    // 清空注册表（通过 unregister 逐个移除）
    const registry = getStoreRegistry();
    for (const key of [...registry.keys()]) {
      unregisterStore(key);
    }
  });

  describe("registerStore", () => {
    it("应注册 store", () => {
      const accessor = {
        get: vi.fn().mockReturnValue({ key: "value" }),
        set: vi.fn(),
        update: vi.fn(),
        subscribe: vi.fn(),
      };
      registerStore("testStore", accessor);
      expect(getStoreRegistry().get("testStore")).toBe(accessor);
    });
  });

  describe("getStoreRegistry", () => {
    it("应返回注册表", () => {
      expect(getStoreRegistry()).toBeInstanceOf(Map);
    });
  });

  describe("unregisterStore", () => {
    it("应取消注册", () => {
      const accessor = {
        get: vi.fn(),
        set: vi.fn(),
        update: vi.fn(),
        subscribe: vi.fn(),
      };
      registerStore("testStore", accessor);
      unregisterStore("testStore");
      expect(getStoreRegistry().get("testStore")).toBeUndefined();
    });
  });

  describe("store accessor", () => {
    it("get 应返回 store 数据", () => {
      const accessor = {
        get: vi.fn().mockReturnValue({ name: "test" }),
        set: vi.fn(),
        update: vi.fn(),
        subscribe: vi.fn(),
      };
      registerStore("myStore", accessor);
      const store = getStoreRegistry().get("myStore")!;
      expect(store.get()).toEqual({ name: "test" });
    });

    it("set 应调用 setter", () => {
      const set = vi.fn();
      const accessor = { get: vi.fn(), set, update: vi.fn(), subscribe: vi.fn() };
      registerStore("myStore", accessor);
      getStoreRegistry().get("myStore")!.set({ name: "updated" });
      expect(set).toHaveBeenCalledWith({ name: "updated" });
    });
  });
});

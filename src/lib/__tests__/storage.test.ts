// SPDX-License-Identifier: AGPL-3.0-only

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { createDebouncedStorage, storage } from "../storage";

describe("storage", () => {
  beforeEach(() => {
    localStorage.clear();
  });

  describe("get", () => {
    it("应返回解析后的 JSON 对象", () => {
      localStorage.setItem("test-key", JSON.stringify({ name: "hello" }));
      const result = storage.get<{ name: string }>("test-key");
      expect(result).toEqual({ name: "hello" });
    });

    it("key 不存在时应返回 null", () => {
      const result = storage.get("nonexistent");
      expect(result).toBeNull();
    });

    it("解析失败时应返回 null", () => {
      localStorage.setItem("bad", "not-json{{");
      const result = storage.get("bad");
      expect(result).toBeNull();
    });

    it("应返回原始字符串（非 JSON 格式）", () => {
      localStorage.setItem("raw", '"plain string"');
      const result = storage.get<string>("raw");
      expect(result).toBe("plain string");
    });

    it("应正确处理数组", () => {
      localStorage.setItem("arr", JSON.stringify([1, 2, 3]));
      const result = storage.get<number[]>("arr");
      expect(result).toEqual([1, 2, 3]);
    });
  });

  describe("set", () => {
    it("应将值序列化后存储到 localStorage", () => {
      storage.set("test-key", { count: 42 });
      const raw = localStorage.getItem("test-key");
      expect(raw).toBe('{"count":42}');
    });

    it("应正确存储字符串", () => {
      storage.set("test-key", "hello world");
      const raw = localStorage.getItem("test-key");
      expect(raw).toBe('"hello world"');
    });

    it("数据过大时应发出 console.warn", () => {
      const warn = vi.spyOn(console, "warn").mockImplementation(() => {});
      const largeData = "x".repeat(500 * 1024 + 1);
      storage.set("large", largeData);
      expect(warn).toHaveBeenCalledWith(
        expect.stringContaining("[storage] 数据过大"),
      );
      warn.mockRestore();
    });

    it("QuotaExceededError 时应捕获并发出 console.warn", () => {
      const warn = vi.spyOn(console, "warn").mockImplementation(() => {});
      const setItem = vi
        .spyOn(Storage.prototype, "setItem")
        .mockImplementation(() => {
          const err = new DOMException("Quota exceeded", "QuotaExceededError");
          throw err;
        });

      expect(() => storage.set("test", "data")).not.toThrow();
      expect(warn).toHaveBeenCalledWith(
        expect.stringContaining("存储空间不足"),
      );

      setItem.mockRestore();
      warn.mockRestore();
    });

    it("非 QuotaExceededError 的异常应向上抛出", () => {
      const setItem = vi
        .spyOn(Storage.prototype, "setItem")
        .mockImplementation(() => {
          throw new Error("Other error");
        });

      expect(() => storage.set("test", "data")).toThrow("Other error");

      setItem.mockRestore();
    });
  });

  describe("remove", () => {
    it("应删除指定的 key", () => {
      localStorage.setItem("test-key", "value");
      storage.remove("test-key");
      expect(localStorage.getItem("test-key")).toBeNull();
    });
  });

  describe("clear", () => {
    it("应清空所有 localStorage", () => {
      localStorage.setItem("a", "1");
      localStorage.setItem("b", "2");
      storage.clear();
      expect(localStorage.length).toBe(0);
    });
  });

  describe("namespace", () => {
    it("应自动添加前缀", () => {
      const ns = storage.namespace("settings");
      ns.set("theme", "dark");
      expect(localStorage.getItem("settings:theme")).toBe('"dark"');
    });

    it("get 应正确读取带前缀的 key", () => {
      localStorage.setItem("settings:theme", '"dark"');
      const ns = storage.namespace("settings");
      expect(ns.get<string>("theme")).toBe("dark");
    });

    it("remove 应删除带前缀的 key", () => {
      localStorage.setItem("settings:theme", "dark");
      const ns = storage.namespace("settings");
      ns.remove("theme");
      expect(localStorage.getItem("settings:theme")).toBeNull();
    });
  });
});

describe("createDebouncedStorage", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    localStorage.clear();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("应在延迟后写入", () => {
    const db = createDebouncedStorage("test", 300);
    db.set("key", "value");

    expect(localStorage.getItem("test:key")).toBeNull();

    vi.advanceTimersByTime(300);
    expect(localStorage.getItem("test:key")).toBe('"value"');
  });

  it("多次 set 应只写入最后一次的值", () => {
    const db = createDebouncedStorage("test", 300);
    db.set("key", "a");
    db.set("key", "b");
    db.set("key", "c");

    vi.advanceTimersByTime(300);
    expect(localStorage.getItem("test:key")).toBe('"c"');
  });

  it("flush 应立即写入所有待处理数据", () => {
    const db = createDebouncedStorage("test", 300);
    db.set("key1", "a");
    db.set("key2", "b");

    db.flush();

    expect(localStorage.getItem("test:key1")).toBe('"a"');
    expect(localStorage.getItem("test:key2")).toBe('"b"');
  });

  it("remove 应取消待写入并删除已有数据", () => {
    localStorage.setItem("test:key", '"old"');
    const db = createDebouncedStorage("test", 300);
    db.set("key", "new");
    db.remove("key");

    vi.advanceTimersByTime(300);
    expect(localStorage.getItem("test:key")).toBeNull();
  });

  it("get 应能读取已写入的值", () => {
    localStorage.setItem("test:key", '"stored"');
    const db = createDebouncedStorage("test", 300);
    expect(db.get<string>("key")).toBe("stored");
  });
});

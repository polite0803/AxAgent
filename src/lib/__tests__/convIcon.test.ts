import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { CONV_ICON_KEY, getConvIcon, type ConvIcon } from "../convIcon";

describe("convIcon", () => {
  beforeEach(() => {
    localStorage.clear();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  // ═══════════════════════════════════════════════════════════════
  // CONV_ICON_KEY
  // ═══════════════════════════════════════════════════════════════
  describe("CONV_ICON_KEY", () => {
    it("应生成正确格式的 localStorage key", () => {
      const key = CONV_ICON_KEY("conv-123");
      expect(key).toBe("axagent_conv_icon_conv-123");
    });

    it("不同 ID 应生成不同 key", () => {
      const key1 = CONV_ICON_KEY("a");
      const key2 = CONV_ICON_KEY("b");
      expect(key1).not.toBe(key2);
    });
  });

  // ═══════════════════════════════════════════════════════════════
  // getConvIcon
  // ═══════════════════════════════════════════════════════════════
  describe("getConvIcon", () => {
    it("localStorage 无数据时应返回 null", () => {
      const result = getConvIcon("nonexistent");
      expect(result).toBeNull();
    });

    it("有效的 JSON 数据应正确解析为 ConvIcon", () => {
      const icon: ConvIcon = { type: "emoji", value: "🚀" };
      localStorage.setItem(CONV_ICON_KEY("conv-1"), JSON.stringify(icon));

      const result = getConvIcon("conv-1");

      expect(result).toEqual(icon);
      expect(result!.type).toBe("emoji");
      expect(result!.value).toBe("🚀");
    });

    it("应支持 model 类型的图标", () => {
      const icon: ConvIcon = { type: "model", value: "gpt-4" };
      localStorage.setItem(CONV_ICON_KEY("conv-2"), JSON.stringify(icon));

      const result = getConvIcon("conv-2");

      expect(result).toEqual(icon);
    });

    it("应支持 url 类型的图标", () => {
      const icon: ConvIcon = { type: "url", value: "https://example.com/icon.png" };
      localStorage.setItem(CONV_ICON_KEY("conv-3"), JSON.stringify(icon));

      const result = getConvIcon("conv-3");

      expect(result).toEqual(icon);
    });

    it("应支持 file 类型的图标", () => {
      const icon: ConvIcon = { type: "file", value: "/path/to/icon.svg" };
      localStorage.setItem(CONV_ICON_KEY("conv-4"), JSON.stringify(icon));

      const result = getConvIcon("conv-4");

      expect(result).toEqual(icon);
    });

    it("损坏的 JSON 应返回 null（不抛异常）", () => {
      localStorage.setItem(CONV_ICON_KEY("conv-5"), "{ broken json");

      const result = getConvIcon("conv-5");

      expect(result).toBeNull();
    });

    it("有效 JSON 但类型不匹配也应正常返回", () => {
      localStorage.setItem(CONV_ICON_KEY("conv-6"), JSON.stringify({ foo: "bar" }));

      const result = getConvIcon("conv-6");

      // JSON.parse 成功，返回解析结果（类型转换由调用方负责）
      expect(result).not.toBeNull();
    });
  });
});

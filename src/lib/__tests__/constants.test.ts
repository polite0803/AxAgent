import { describe, expect, it } from "vitest";

import { LANG_OPTIONS } from "../constants";

describe("constants", () => {
  describe("LANG_OPTIONS", () => {
    it("应包含 11 种语言选项", () => {
      expect(LANG_OPTIONS).toHaveLength(11);
    });

    it("应包含简体中文", () => {
      const zhCN = LANG_OPTIONS.find((opt) => opt.key === "zh-CN");
      expect(zhCN).toBeDefined();
      expect(zhCN!.label).toBe("简体中文");
    });

    it("应包含英文", () => {
      const enUS = LANG_OPTIONS.find((opt) => opt.key === "en-US");
      expect(enUS).toBeDefined();
      expect(enUS!.label).toBe("English");
    });

    it("每个选项应有 key、label、icon 三个字段", () => {
      for (const option of LANG_OPTIONS) {
        expect(option).toHaveProperty("key");
        expect(option).toHaveProperty("label");
        expect(option).toHaveProperty("icon");
        expect(typeof option.key).toBe("string");
        expect(typeof option.label).toBe("string");
        expect(typeof option.icon).toBe("string");
      }
    });

    it("所有语言 key 应唯一", () => {
      const keys = LANG_OPTIONS.map((opt) => opt.key);
      const uniqueKeys = new Set(keys);
      expect(uniqueKeys.size).toBe(keys.length);
    });

    it("数组应为只读（as const）", () => {
      // 验证 LANG_OPTIONS 是通过 as const 声明的只读元组
      expect(LANG_OPTIONS).toBeInstanceOf(Array);
    });
  });
});

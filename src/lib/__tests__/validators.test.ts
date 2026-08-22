/**
 * tests/validators.test.ts
 *
 * 验证工具库的单元测试
 * 确保 ID 验证逻辑正确处理各种边界情况
 */

import { describe, expect, it } from "vitest";
import { isValidId, safeJoinIds, safeParseIdPair, sanitizeId, validateModelRef } from "../validators";

describe("isValidId", () => {
  it("应该接受正常字符串", () => {
    expect(isValidId("abc123")).toBe(true);
    expect(isValidId("75dba36a-69b9-4790-954d-6d43520c417c")).toBe(true);
  });

  it("应该拒绝 undefined", () => {
    expect(isValidId(undefined)).toBe(false);
  });

  it("应该拒绝 null", () => {
    expect(isValidId(null)).toBe(false);
  });

  it("应该拒绝数字", () => {
    expect(isValidId(123)).toBe(false);
  });

  it("应该拒绝布尔值", () => {
    expect(isValidId(true)).toBe(false);
    expect(isValidId(false)).toBe(false);
  });

  it("应该拒绝空字符串", () => {
    expect(isValidId("")).toBe(false);
  });

  it("应该拒绝纯空白字符串", () => {
    expect(isValidId("   ")).toBe(false);
    expect(isValidId("\t")).toBe(false);
  });

  it("应该拒绝字面量 'undefined'", () => {
    expect(isValidId("undefined")).toBe(false);
  });

  it("应该拒绝字面量 'null'", () => {
    expect(isValidId("null")).toBe(false);
  });
});

describe("sanitizeId", () => {
  it("应该返回有效 ID", () => {
    expect(sanitizeId("abc123")).toBe("abc123");
  });

  it("应该将 undefined 转为 null", () => {
    expect(sanitizeId(undefined)).toBeNull();
  });

  it("应该将 null 转为 null", () => {
    expect(sanitizeId(null)).toBeNull();
  });

  it("应该将 'undefined' 字符串转为 null", () => {
    expect(sanitizeId("undefined")).toBeNull();
  });

  it("应该将 'null' 字符串转为 null", () => {
    expect(sanitizeId("null")).toBeNull();
  });

  it("应该将空字符串转为 null", () => {
    expect(sanitizeId("")).toBeNull();
  });
});

describe("validateModelRef", () => {
  it("应该接受两个有效 ID", () => {
    const result = validateModelRef("provider-1", "model-1");
    expect(result).toEqual({ providerId: "provider-1", modelId: "model-1" });
  });

  it("应该接受两个 null", () => {
    const result = validateModelRef(null, null);
    expect(result).toBeNull();
  });

  it("应该拒绝一个有效一个无效的情况", () => {
    const result = validateModelRef("provider-1", undefined);
    expect(result).toBeNull();
  });

  it("应该拒绝 providerId 为 'undefined' 字符串", () => {
    const result = validateModelRef("undefined", "model-1");
    expect(result).toBeNull();
  });

  it("应该拒绝 modelId 为 'undefined' 字符串", () => {
    const result = validateModelRef("provider-1", "undefined");
    expect(result).toBeNull();
  });

  it("应该拒绝两个都是 'undefined' 字符串", () => {
    const result = validateModelRef("undefined", "undefined");
    expect(result).toBeNull();
  });
});

describe("safeJoinIds", () => {
  it("应该正常拼接有效 ID", () => {
    expect(safeJoinIds(["provider-1", "model-1"])).toBe("provider-1::model-1");
  });

  it("应该跳过 undefined 值", () => {
    expect(safeJoinIds(["provider-1", undefined])).toBe("provider-1");
  });

  it("应该跳过 'undefined' 字符串", () => {
    expect(safeJoinIds(["provider-1", "undefined"])).toBe("provider-1");
  });

  it("应该跳过 null 值", () => {
    expect(safeJoinIds([null, "model-1"])).toBe("model-1");
  });

  it("应该跳过 'null' 字符串", () => {
    expect(safeJoinIds(["null", "model-1"])).toBe("model-1");
  });

  it("所有值无效时应该返回空字符串", () => {
    expect(safeJoinIds([undefined, null, "undefined", "null"])).toBe("");
  });

  it("应该支持自定义分隔符", () => {
    expect(safeJoinIds(["a", "b", "c"], "/")).toBe("a/b/c");
  });
});

describe("safeParseIdPair", () => {
  it("应该正确解析有效字符串", () => {
    const result = safeParseIdPair("provider-1::model-1");
    expect(result).toEqual({ first: "provider-1", second: "model-1" });
  });

  it("应该拒绝 undefined", () => {
    expect(safeParseIdPair(undefined)).toBeNull();
  });

  it("应该拒绝 null", () => {
    expect(safeParseIdPair(null)).toBeNull();
  });

  it("应该拒绝 'undefined::model' 格式", () => {
    expect(safeParseIdPair("undefined::model-1")).toBeNull();
  });

  it("应该拒绝 'provider::undefined' 格式", () => {
    expect(safeParseIdPair("provider-1::undefined")).toBeNull();
  });

  it("应该拒绝没有分隔符的字符串", () => {
    expect(safeParseIdPair("invalid")).toBeNull();
  });

  it("应该拒绝超过两个部分的字符串", () => {
    expect(safeParseIdPair("a::b::c")).toBeNull();
  });

  it("应该支持自定义分隔符", () => {
    const result = safeParseIdPair("a/b", "/");
    expect(result).toEqual({ first: "a", second: "b" });
  });
});

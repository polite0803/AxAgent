// SPDX-License-Identifier: AGPL-3.0-only

import { beforeAll, describe, expect, it, vi } from "vitest";

import i18n from "@/i18n";
import { getBackendErrorCategory, parseBackendError, showBackendError, translateBackendError } from "../errorI18n";

// 使用 zh-CN 源语言（同步 bundle），确保翻译命中真实 locale 数据。
beforeAll(async () => {
  await i18n.changeLanguage("zh-CN");
});

describe("parseBackendError", () => {
  it("解析带合法 code 的对象（Tauri 直接序列化 ErrorResponse）", () => {
    const parsed = parseBackendError({
      code: "CONVERSATION_NOT_FOUND",
      category: "unrecoverable",
      detail: "id=42",
    });
    expect(parsed.code).toBe("CONVERSATION_NOT_FOUND");
    expect(parsed.category).toBe("unrecoverable");
    expect(parsed.detail).toBe("id=42");
  });

  it("解析 message 为 JSON 字符串的 Error", () => {
    const err = new Error(JSON.stringify({ code: "TOOL_NOT_FOUND", category: "validation" }));
    const parsed = parseBackendError(err);
    expect(parsed.code).toBe("TOOL_NOT_FOUND");
    expect(parsed.category).toBe("validation");
  });

  it("解析纯 JSON 字符串", () => {
    const parsed = parseBackendError('{"code":"COMMON_INTERNAL","detail":"boom"}');
    expect(parsed.code).toBe("COMMON_INTERNAL");
    expect(parsed.detail).toBe("boom");
  });

  it("纯字符串无 code 时仅返回 raw", () => {
    const parsed = parseBackendError("something went wrong");
    expect(parsed.code).toBeUndefined();
    expect(parsed.raw).toBe("something went wrong");
  });

  it("非法 code 格式不被识别（小写/单段）", () => {
    expect(parseBackendError({ code: "notACode" }).code).toBeUndefined();
    expect(parseBackendError({ code: "LOWER" }).code).toBeUndefined();
  });

  it("非法 category 被丢弃", () => {
    const parsed = parseBackendError({ code: "TOOL_NOT_FOUND", category: "bogus" });
    expect(parsed.code).toBe("TOOL_NOT_FOUND");
    expect(parsed.category).toBeUndefined();
  });

  it("null / undefined 安全处理", () => {
    expect(parseBackendError(null).raw).toBe("");
    expect(parseBackendError(undefined).raw).toBe("");
  });
});

describe("translateBackendError", () => {
  it("已知码翻译为 zh-CN 文本", () => {
    expect(translateBackendError({ code: "CONVERSATION_NOT_FOUND" })).toBe("会话未找到");
    expect(translateBackendError({ code: "TOOL_NOT_FOUND" })).toBe("工具未找到");
  });

  it("未知码回退 detail", () => {
    expect(translateBackendError({ code: "TOTALLY_UNKNOWN_CODE", detail: "fallback detail" }))
      .toBe("fallback detail");
  });

  it("未知码无 detail 时回退原始文本", () => {
    expect(translateBackendError("plain error text")).toBe("plain error text");
  });

  it("单花括号占位符被手动替换", () => {
    // AGENT_STATUS_STEER_APPLIED -> "已应用 {count} 条引导指令"
    const text = translateBackendError({
      code: "AGENT_STATUS_STEER_APPLIED",
      params: { count: "3" },
    });
    expect(text).toBe("已应用 3 条引导指令");
  });

  it("Error 对象的 JSON message 也能翻译", () => {
    const err = new Error(JSON.stringify({ code: "CONVERSATION_NOT_FOUND" }));
    expect(translateBackendError(err)).toBe("会话未找到");
  });
});

describe("getBackendErrorCategory", () => {
  it("提取合法分类", () => {
    expect(getBackendErrorCategory({ code: "TOOL_NOT_FOUND", category: "retryable" }))
      .toBe("retryable");
  });

  it("无分类返回 undefined", () => {
    expect(getBackendErrorCategory("plain")).toBeUndefined();
  });
});

describe("showBackendError", () => {
  it("retryable 分类走 warning", () => {
    const message = { error: vi.fn(), warning: vi.fn() };
    const text = showBackendError(message, { code: "TOOL_NOT_FOUND", category: "retryable" });
    expect(message.warning).toHaveBeenCalledWith("工具未找到", undefined);
    expect(message.error).not.toHaveBeenCalled();
    expect(text).toBe("工具未找到");
  });

  it("非 retryable 分类走 error", () => {
    const message = { error: vi.fn(), warning: vi.fn() };
    showBackendError(message, { code: "CONVERSATION_NOT_FOUND", category: "unrecoverable" });
    expect(message.error).toHaveBeenCalledWith("会话未找到", undefined);
    expect(message.warning).not.toHaveBeenCalled();
  });

  it("纯字符串错误走 error 并原样展示", () => {
    const message = { error: vi.fn(), warning: vi.fn() };
    showBackendError(message, "raw failure");
    expect(message.error).toHaveBeenCalledWith("raw failure", undefined);
  });
});

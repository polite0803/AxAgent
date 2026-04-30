import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

// ─── Mock 设置 ────────────────────────────────────────────────────

const mockPreloadIcons = vi.fn();

vi.mock("markstream-react", () => ({
  preloadExtendedLanguageIcons: mockPreloadIcons,
}));

// 模拟 stream-monaco 动态导入
vi.mock("stream-monaco", () => ({
  default: { version: "1.0.0" },
}));

// 需要重新导入被测模块（因为 preloadChatRenderers 使用顶层闭包变量）
// 每个测试需要独立的模块状态，使用 vi.resetModules 重建

describe("preloadChatRenderers", () => {
  beforeEach(() => {
    vi.resetModules();
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("首次调用应触发动态 import 并调用 preloadExtendedLanguageIcons", async () => {
    const { preloadChatRenderers } = await import("../preloadChatRenderers");

    await preloadChatRenderers();

    expect(mockPreloadIcons).toHaveBeenCalled();
  });

  it("多次调用应复用同一个 Promise（不会重复导入）", async () => {
    const { preloadChatRenderers } = await import("../preloadChatRenderers");

    const p1 = preloadChatRenderers();
    const p2 = preloadChatRenderers();
    const p3 = preloadChatRenderers();

    expect(p1).toBe(p2);
    expect(p2).toBe(p3);

    await p1;
    // preloadExtendedLanguageIcons 只应调用一次
    expect(mockPreloadIcons).toHaveBeenCalledTimes(1);
  });

  it("动态 import 失败时应优雅降级（不抛异常）", async () => {
    // 模拟 stream-monaco 导入失败
    vi.doMock("stream-monaco", () => {
      throw new Error("Module not found");
    });

    const consoleWarnSpy = vi.spyOn(console, "warn").mockImplementation(() => {});

    const { preloadChatRenderers } = await import("../preloadChatRenderers");

    // 不应抛出异常
    await expect(preloadChatRenderers()).resolves.toBeUndefined();

    consoleWarnSpy.mockRestore();
  });

  it("返回的 Promise 类型应为 Promise<void>", async () => {
    const { preloadChatRenderers } = await import("../preloadChatRenderers");

    const result = preloadChatRenderers();

    expect(result).toBeInstanceOf(Promise);
    await expect(result).resolves.toBeUndefined();
  });
});

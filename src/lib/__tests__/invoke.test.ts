import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

// ─── Mock 设置 ────────────────────────────────────────────────────
// 必须在导入前 mock，因为 invoke.ts 顶层有 import

const mockTauriInvoke = vi.fn();
const mockHandleCommand = vi.fn();
const mockTauriListen = vi.fn();

vi.mock("@tauri-apps/api/core", () => ({
  invoke: mockTauriInvoke,
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: mockTauriListen,
}));

vi.mock("../browserMock", () => ({
  handleCommand: mockHandleCommand,
}));

import {
  DEFAULT_INVOKE_TIMEOUT_MS,
  getInvokeMetrics,
  invoke,
  isTauri,
  listen,
} from "../invoke";

// ─── 辅助函数 ──────────────────────────────────────────────────────

/** 在 window 上设置 __TAURI_INTERNALS__ 来模拟 Tauri 环境 */
function enableTauriMode() {
  (window as any).__TAURI_INTERNALS__ = {};
}

/** 清除 window 上的 __TAURI_INTERNALS__ 来模拟浏览器环境 */
function disableTauriMode() {
  delete (window as any).__TAURI_INTERNALS__;
}

// ─── 测试套件 ──────────────────────────────────────────────────────

describe("invoke.ts", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    disableTauriMode();
  });

  afterEach(() => {
    disableTauriMode();
  });

  // ═══════════════════════════════════════════════════════════════
  // isTauri
  // ═══════════════════════════════════════════════════════════════
  describe("isTauri", () => {
    it("浏览器环境下应返回 false", () => {
      expect(isTauri()).toBe(false);
    });

    it("Tauri 环境下（有 __TAURI_INTERNALS__）应返回 true", () => {
      enableTauriMode();
      expect(isTauri()).toBe(true);
    });
  });

  // ═══════════════════════════════════════════════════════════════
  // DEFAULT_INVOKE_TIMEOUT_MS 常量
  // ═══════════════════════════════════════════════════════════════
  describe("DEFAULT_INVOKE_TIMEOUT_MS", () => {
    it("应为 5 分钟（300000ms）", () => {
      expect(DEFAULT_INVOKE_TIMEOUT_MS).toBe(5 * 60 * 1000);
    });
  });

  // ═══════════════════════════════════════════════════════════════
  // invoke — 浏览器模式
  // ═══════════════════════════════════════════════════════════════
  describe("invoke 浏览器模式", () => {
    it("非 Tauri 环境应委托给 browserMock.handleCommand", async () => {
      mockHandleCommand.mockResolvedValueOnce({ data: "result" });

      const result = await invoke<{ data: string }>("my_command", { key: "val" });

      expect(mockHandleCommand).toHaveBeenCalledWith("my_command", { key: "val" });
      expect(result).toEqual({ data: "result" });
    });

    it("非 Tauri 环境不应调用 Tauri invoke", async () => {
      mockHandleCommand.mockResolvedValueOnce("ok");

      await invoke("test_cmd");

      expect(mockTauriInvoke).not.toHaveBeenCalled();
    });

    it("handleCommand 异常应向上抛出", async () => {
      mockHandleCommand.mockRejectedValueOnce(new Error("Command failed"));

      await expect(invoke("bad_cmd")).rejects.toThrow("Command failed");
    });
  });

  // ═══════════════════════════════════════════════════════════════
  // invoke — Tauri 模式
  // ═══════════════════════════════════════════════════════════════
  describe("invoke Tauri 模式", () => {
    it("Tauri 环境应委托给 @tauri-apps/api/core 的 invoke", async () => {
      enableTauriMode();
      mockTauriInvoke.mockResolvedValueOnce({ value: 42 });

      const result = await invoke<{ value: number }>("read_config");

      expect(mockTauriInvoke).toHaveBeenCalledWith("read_config", undefined);
      expect(result).toEqual({ value: 42 });
    });

    it("Tauri 环境应传递 args 参数", async () => {
      enableTauriMode();
      mockTauriInvoke.mockResolvedValueOnce("done");

      await invoke("write_file", { path: "/tmp/test", content: "hello" });

      expect(mockTauriInvoke).toHaveBeenCalledWith("write_file", {
        path: "/tmp/test",
        content: "hello",
      });
    });

    it("Tauri 环境 invoke 异常应向上抛出", async () => {
      enableTauriMode();
      mockTauriInvoke.mockRejectedValueOnce(new Error("Backend error"));

      await expect(invoke("crash_cmd")).rejects.toThrow("Backend error");
    });
  });

  // ═══════════════════════════════════════════════════════════════
  // invoke — 超时处理
  // ═══════════════════════════════════════════════════════════════
  describe("invoke 超时处理", () => {
    it("timeoutMs=0 时应禁用超时（不 reject）", async () => {
      enableTauriMode();
      mockTauriInvoke.mockResolvedValueOnce("fast");

      const result = await invoke("fast_cmd", undefined, 0);

      expect(result).toBe("fast");
    });

    it("自定义 timeoutMs 应在超时后 reject", async () => {
      vi.useFakeTimers();
      enableTauriMode();

      // 这个 invoke 永远不会 resolve
      mockTauriInvoke.mockImplementationOnce(
        () => new Promise(() => {}), // 永不 resolve
      );

      const promise = invoke("slow_cmd", undefined, 1000);
      vi.advanceTimersByTime(1000);

      await expect(promise).rejects.toThrow("slow_cmd");
      await expect(promise).rejects.toThrow("timed out");
      await expect(promise).rejects.toThrow("1.0s");

      vi.useRealTimers();
    });

    it("在超时前返回不应触发 timeout", async () => {
      vi.useFakeTimers();
      enableTauriMode();

      mockTauriInvoke.mockImplementationOnce(
        () =>
          new Promise((resolve) => {
            setTimeout(() => resolve("done"), 500);
          }),
      );

      const promise = invoke("medium_cmd", undefined, 2000);
      vi.advanceTimersByTime(500);
      await promise; // 不应抛出

      vi.useRealTimers();
    });
  });

  // ═══════════════════════════════════════════════════════════════
  // invoke — 连接错误转换
  // ═══════════════════════════════════════════════════════════════
  describe("invoke 连接错误转换", () => {
    it("IPC 连接错误应转换为友好提示", async () => {
      enableTauriMode();
      mockTauriInvoke.mockRejectedValueOnce(new Error("Connection refused"));

      await expect(invoke("connect_cmd")).rejects.toThrow("Backend connection failed");
      await expect(invoke("connect_cmd")).rejects.toThrow("connect_cmd");
    });

    it("fetch 相关错误应转换为友好提示", async () => {
      enableTauriMode();
      mockTauriInvoke.mockRejectedValueOnce(new Error("Fetch error occurred"));

      await expect(invoke("fetch_cmd")).rejects.toThrow("Backend connection failed");
    });

    it("protocol 相关错误应转换为友好提示", async () => {
      enableTauriMode();
      mockTauriInvoke.mockRejectedValueOnce(new Error("Protocol error"));

      await expect(invoke("proto_cmd")).rejects.toThrow("Backend connection failed");
    });

    it("普通业务错误不应被转换", async () => {
      enableTauriMode();
      mockTauriInvoke.mockRejectedValueOnce(new Error("Validation failed"));

      await expect(invoke("validate_cmd")).rejects.toThrow("Validation failed");
    });
  });

  // ═══════════════════════════════════════════════════════════════
  // getInvokeMetrics — 调用指标
  // ═══════════════════════════════════════════════════════════════
  describe("getInvokeMetrics", () => {
    it("初始状态应返回空指标", () => {
      const metrics = getInvokeMetrics();

      expect(metrics.totalCalls).toBe(0);
      expect(metrics.totalFailed).toBe(0);
      expect(metrics.byCommand).toEqual([]);
      expect(metrics.recentErrors).toEqual([]);
    });

    it("成功调用后应更新指标", async () => {
      disableTauriMode();
      mockHandleCommand.mockResolvedValueOnce("ok");

      await invoke("cmd_a");

      const metrics = getInvokeMetrics();
      expect(metrics.totalCalls).toBeGreaterThanOrEqual(1);
      expect(metrics.totalFailed).toBe(0);
      expect(metrics.byCommand.length).toBeGreaterThanOrEqual(1);
      const cmdStats = metrics.byCommand.find((c) => c.command === "cmd_a");
      expect(cmdStats).toBeDefined();
      expect(cmdStats!.total).toBeGreaterThanOrEqual(1);
      expect(cmdStats!.failed).toBe(0);
    });

    it("失败调用后应记录错误指标", async () => {
      disableTauriMode();
      mockHandleCommand.mockRejectedValueOnce(new Error("boom"));

      try {
        await invoke("cmd_b");
      } catch {
        // expected
      }

      const metrics = getInvokeMetrics();
      expect(metrics.totalFailed).toBeGreaterThanOrEqual(1);
      const cmdStats = metrics.byCommand.find((c) => c.command === "cmd_b");
      expect(cmdStats).toBeDefined();
      expect(cmdStats!.failed).toBeGreaterThanOrEqual(1);
      expect(metrics.recentErrors.length).toBeGreaterThanOrEqual(1);
    });

    it("多次调用同一命令应正确聚合统计", async () => {
      disableTauriMode();
      mockHandleCommand.mockResolvedValueOnce("a");
      mockHandleCommand.mockResolvedValueOnce("b");
      mockHandleCommand.mockRejectedValueOnce(new Error("c"));

      await invoke("multi_cmd");
      await invoke("multi_cmd");
      try { await invoke("multi_cmd"); } catch {}

      const metrics = getInvokeMetrics();
      const cmdStats = metrics.byCommand.find((c) => c.command === "multi_cmd");
      expect(cmdStats).toBeDefined();
      expect(cmdStats!.total).toBe(3);
      expect(cmdStats!.failed).toBe(1);
    });
  });

  // ═══════════════════════════════════════════════════════════════
  // listen
  // ═══════════════════════════════════════════════════════════════
  describe("listen", () => {
    it("浏览器环境下应返回空的 unlisten 函数", async () => {
      const handler = vi.fn();
      const unlisten = await listen("my_event", handler);

      expect(typeof unlisten).toBe("function");
      // 调用 unlisten 不应报错
      expect(() => unlisten()).not.toThrow();
      // handler 不应被调用（浏览器模式下无事件系统）
      expect(handler).not.toHaveBeenCalled();
    });

    it("Tauri 环境下应委托给 @tauri-apps/api/event 的 listen", async () => {
      enableTauriMode();
      const mockUnlisten = vi.fn();
      mockTauriListen.mockResolvedValueOnce(mockUnlisten);

      const handler = vi.fn();
      const unlisten = await listen("tauri_event", handler);

      expect(mockTauriListen).toHaveBeenCalledWith("tauri_event", handler);
      expect(unlisten).toBe(mockUnlisten);
    });
  });

  // ═══════════════════════════════════════════════════════════════
  // getInvokeMetrics — 分位数计算
  // ═══════════════════════════════════════════════════════════════
  describe("getInvokeMetrics 分位数", () => {
    it("p50/p95/p99 在无数据时应为 0", () => {
      const metrics = getInvokeMetrics();
      // 无数据时 byCommand 为空
      expect(metrics.byCommand.length).toBe(0);
    });

    it("应计算各命令的执行时长百分位", async () => {
      disableTauriMode();
      // 模拟多次不同时长的调用
      mockHandleCommand.mockResolvedValueOnce("1");
      mockHandleCommand.mockResolvedValueOnce("2");
      mockHandleCommand.mockResolvedValueOnce("3");

      await invoke("pct_cmd");
      await invoke("pct_cmd");
      await invoke("pct_cmd");

      const metrics = getInvokeMetrics();
      const cmdStats = metrics.byCommand.find((c) => c.command === "pct_cmd");
      expect(cmdStats).toBeDefined();
      // 百分位值应为有效数字
      expect(typeof cmdStats!.p50Ms).toBe("number");
      expect(typeof cmdStats!.p95Ms).toBe("number");
      expect(typeof cmdStats!.p99Ms).toBe("number");
    });
  });
});

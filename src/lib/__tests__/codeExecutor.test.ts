import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

// 必须在导入前 mock，因为 codeExecutor.ts 顶层调用了 import
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

import { invoke } from "@tauri-apps/api/core";
import { codeExecutor } from "../codeExecutor";

const mockedInvoke = vi.mocked(invoke);

// 辅助函数：创建模拟的 Pyodide 接口
function createMockPyodide(): { runPythonAsync: ReturnType<typeof vi.fn> } {
  return { runPythonAsync: vi.fn().mockResolvedValue(undefined) };
}

describe("CodeExecutor", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
    // 清理 window 上的 Pyodide 残留
    delete (window as any).loadPyodide;
    // 清理之前注入的 script 标签
    document.head.innerHTML = "";
  });

  // ═══════════════════════════════════════════════════════════════════
  // execute() — 语言分发
  // ═══════════════════════════════════════════════════════════════════
  describe("execute() 语言分发", () => {
    it("javascript 语言应调用 executeJS（通过 Tauri invoke）", async () => {
      mockedInvoke.mockResolvedValueOnce({
        stdout: "hello",
        stderr: "",
        exit_code: 0,
      });

      const result = await codeExecutor.execute({
        language: "javascript",
        code: "console.log('hello')",
      });

      expect(mockedInvoke).toHaveBeenCalledWith("execute_sandbox", {
        code: "console.log('hello')",
        language: "javascript",
      });
      expect(result.stdout).toBe("hello");
      expect(result.exit_code).toBe(0);
    });

    it("typescript 语言应调用 executeJS（与 javascript 同路径）", async () => {
      mockedInvoke.mockResolvedValueOnce({
        stdout: "ts output",
        stderr: "",
        exit_code: 0,
      });

      const result = await codeExecutor.execute({
        language: "typescript",
        code: "const x: number = 1;",
      });

      expect(mockedInvoke).toHaveBeenCalledWith("execute_sandbox", {
        code: "const x: number = 1;",
        language: "javascript",
      });
      expect(result.stdout).toBe("ts output");
    });

    it("python 语言应走 executePython 路径", async () => {
      // 模拟 Pyodide 脚本加载
      const mockPyodide = createMockPyodide();
      (window as any).loadPyodide = vi.fn().mockResolvedValue(mockPyodide);

      // 触发 script onload
      const origCreateElement = document.createElement.bind(document);
      vi.spyOn(document, "createElement").mockImplementation((tag: string) => {
        const el = origCreateElement(tag);
        if (tag === "script") {
          setTimeout(() => {
            (el as any).onload?.({} as Event);
          }, 0);
        }
        return el;
      });

      mockPyodide.runPythonAsync
        .mockResolvedValueOnce(undefined) // io 重定向 setup
        .mockResolvedValueOnce(undefined) // 用户代码
        .mockResolvedValueOnce("hello python\n") // stdout
        .mockResolvedValueOnce(""); // stderr

      const result = await codeExecutor.execute({
        language: "python",
        code: "print('hello python')",
      });

      expect(result.stdout).toBe("hello python\n");
      expect(result.exit_code).toBe(0);
      expect(result.stderr).toBe("");
    });

    it("不支持的语言应返回错误", async () => {
      const result = await codeExecutor.execute({
        language: "ruby" as any,
        code: "puts 'hello'",
      });

      expect(result.exit_code).toBe(-1);
      expect(result.stderr).toContain("Unsupported language");
      expect(result.stderr).toContain("ruby");
      expect(result.stdout).toBe("");
    });
  });

  // ═══════════════════════════════════════════════════════════════════
  // executeJS — JavaScript 执行
  // ═══════════════════════════════════════════════════════════════════
  describe("executeJS", () => {
    it("应返回 duration_ms 时间差", async () => {
      mockedInvoke.mockImplementationOnce(
        () =>
          new Promise((resolve) => {
            vi.advanceTimersByTime(100);
            resolve({ stdout: "ok", stderr: "", exit_code: 0 });
          }),
      );

      const result = await codeExecutor.execute({
        language: "javascript",
        code: "1+1",
      });

      expect(result.duration_ms).toBeGreaterThanOrEqual(0);
      expect(typeof result.duration_ms).toBe("number");
    });

    it("Tauri invoke 异常时应在 stderr 中返回错误信息", async () => {
      mockedInvoke.mockRejectedValueOnce(new Error("Sandbox crashed"));

      const result = await codeExecutor.execute({
        language: "javascript",
        code: "throw new Error('bad')",
      });

      expect(result.exit_code).toBe(-1);
      expect(result.stdout).toBe("");
      expect(result.stderr).toContain("Sandbox crashed");
    });

    it("非 Error 类型的异常应转为字符串放入 stderr", async () => {
      mockedInvoke.mockRejectedValueOnce("raw string error");

      const result = await codeExecutor.execute({
        language: "javascript",
        code: "foo",
      });

      expect(result.exit_code).toBe(-1);
      expect(result.stderr).toContain("raw string error");
    });
  });

  // ═══════════════════════════════════════════════════════════════════
  // executePython — Python 执行
  // ═══════════════════════════════════════════════════════════════════
  describe("executePython", () => {
    it("Pyodide 未加载时返回错误", async () => {
      // 不设置 window.loadPyodide，模拟加载失败
      const origCreateElement = document.createElement.bind(document);
      vi.spyOn(document, "createElement").mockImplementation((tag: string) => {
        const el = origCreateElement(tag);
        if (tag === "script") {
          setTimeout(() => {
            (el as any).onerror?.({} as Event);
          }, 0);
        }
        return el;
      });

      const result = await codeExecutor.execute({
        language: "python",
        code: "print(1)",
      });

      expect(result.exit_code).toBe(-1);
      expect(result.stderr).toContain("Pyodide");
    });

    it("Python 代码异常时应在 stderr 中返回错误", async () => {
      const mockPyodide = createMockPyodide();
      (window as any).loadPyodide = vi.fn().mockResolvedValue(mockPyodide);

      const origCreateElement = document.createElement.bind(document);
      vi.spyOn(document, "createElement").mockImplementation((tag: string) => {
        const el = origCreateElement(tag);
        if (tag === "script") {
          setTimeout(() => (el as any).onload?.({} as Event), 0);
        }
        return el;
      });

      mockPyodide.runPythonAsync
        .mockResolvedValueOnce(undefined) // setup
        .mockRejectedValueOnce(new Error("Python runtime error"));

      const result = await codeExecutor.execute({
        language: "python",
        code: "1/0",
      });

      expect(result.exit_code).toBe(-1);
      expect(result.stderr).toContain("Python runtime error");
    });

    it("initPyodide 应避免重复加载（幂等性）", async () => {
      // 第一次加载成功
      const mockPyodide = createMockPyodide();
      (window as any).loadPyodide = vi.fn().mockResolvedValue(mockPyodide);

      let scriptCount = 0;
      const origCreateElement = document.createElement.bind(document);
      vi.spyOn(document, "createElement").mockImplementation((tag: string) => {
        const el = origCreateElement(tag);
        if (tag === "script") {
          scriptCount++;
          setTimeout(() => (el as any).onload?.({} as Event), 0);
        }
        return el;
      });

      mockPyodide.runPythonAsync
        .mockResolvedValueOnce(undefined)
        .mockResolvedValueOnce(undefined)
        .mockResolvedValueOnce("first\n")
        .mockResolvedValueOnce("")
        .mockResolvedValueOnce(undefined)
        .mockResolvedValueOnce(undefined)
        .mockResolvedValueOnce("second\n")
        .mockResolvedValueOnce("");

      // 第一次执行 Python
      await codeExecutor.execute({ language: "python", code: "print('first')" });
      // 第二次执行 Python（不应再次创建 script 标签）
      await codeExecutor.execute({ language: "python", code: "print('second')" });

      // script 标签只创建了一次
      expect(scriptCount).toBe(1);
    });
  });

  // ═══════════════════════════════════════════════════════════════════
  // ExecutionResult 结构
  // ═══════════════════════════════════════════════════════════════════
  describe("ExecutionResult 结构", () => {
    it("成功结果应包含所有必需字段", async () => {
      mockedInvoke.mockResolvedValueOnce({
        stdout: "success output",
        stderr: "some warning",
        exit_code: 0,
      });

      const result = await codeExecutor.execute({
        language: "javascript",
        code: "test",
      });

      expect(result).toHaveProperty("stdout");
      expect(result).toHaveProperty("stderr");
      expect(result).toHaveProperty("exit_code");
      expect(result).toHaveProperty("duration_ms");
      expect(result.stdout).toBe("success output");
      expect(result.stderr).toBe("some warning");
    });
  });
});

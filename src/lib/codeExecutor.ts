// SPDX-License-Identifier: AGPL-3.0-only

import { invoke, logIpcError } from "@/lib/invoke";

export interface ExecutionResult {
  stdout: string;
  stderr: string;
  exit_code: number;
  duration_ms?: number;
}

export interface CodeExecutorOptions {
  language: "javascript" | "typescript" | "python";
  code: string;
  timeout?: number;
}

declare global {
  interface Window {
    loadPyodide: (config: { indexURL: string }) => Promise<PyodideInterface>;
  }
}

export interface PyodideInterface {
  runPythonAsync: (code: string) => Promise<string>;
}

const PYODIDE_CDN = "https://cdn.jsdelivr.net/pyodide/v0.24.1/full/";
const PYTHON_EXECUTION_TIMEOUT_MS = 30_000;

class CodeExecutor {
  private pyodide: PyodideInterface | null = null;
  private pyodideLoading: Promise<void> | null = null;
  private pyodideLoadFailed = false;

  async initPyodide(): Promise<void> {
    if (this.pyodide) {
      return;
    }
    if (this.pyodideLoadFailed) {
      this.pyodideLoadFailed = false;
      this.pyodideLoading = null;
    }
    if (this.pyodideLoading) {
      await this.pyodideLoading;
      return;
    }

    this.pyodideLoading = (async () => {
      try {
        await new Promise<void>((resolve, reject) => {
          const script = document.createElement("script");
          script.src = `${PYODIDE_CDN}pyodide.js`;
          script.crossOrigin = "anonymous";
          script.onload = () => resolve();
          script.onerror = () => reject(new Error("Failed to load Pyodide script"));
          document.head.appendChild(script);
        });

        this.pyodide = await window.loadPyodide({
          indexURL: PYODIDE_CDN,
        });
      } catch (e) {
        logIpcError("CodeExecutor.loadPyodide")(e);
        this.pyodide = null;
        this.pyodideLoadFailed = true;
        this.pyodideLoading = null;
      }
    })();

    await this.pyodideLoading;
  }

  async executeJS(code: string): Promise<ExecutionResult> {
    const start = performance.now();

    try {
      const result = await invoke<ExecutionResult>("execute_sandbox", {
        code,
        language: "javascript",
      });

      return {
        ...result,
        duration_ms: performance.now() - start,
      };
    } catch (error) {
      return {
        stdout: "",
        stderr: error instanceof Error ? error.message : String(error),
        exit_code: -1,
        duration_ms: performance.now() - start,
      };
    }
  }

  async executePython(code: string): Promise<ExecutionResult> {
    const start = performance.now();

    try {
      await this.initPyodide();

      if (!this.pyodide) {
        return {
          stdout: "",
          stderr: "Pyodide failed to load. Please try again.",
          exit_code: -1,
          duration_ms: performance.now() - start,
        };
      }

      const timeoutPromise = new Promise<never>((_, reject) => {
        setTimeout(
          () => reject(new Error("Python execution timed out")),
          PYTHON_EXECUTION_TIMEOUT_MS,
        );
      });

      const execPromise = (async () => {
        const encodedCode = btoa(
          Array.from(new TextEncoder().encode(code), (byte) => String.fromCharCode(byte)).join(""),
        );
        const result = await this.pyodide!.runPythonAsync(`
import sys, json, base64
from io import StringIO
sys.stdout = StringIO()
sys.stderr = StringIO()
_code = base64.b64decode("${encodedCode}")
try:
    exec(_code)
finally:
    _stdout = sys.stdout.getvalue()
    _stderr = sys.stderr.getvalue()
    sys.stdout = sys.__stdout__
    sys.stderr = sys.__stderr__
json.dumps({"stdout": _stdout, "stderr": _stderr})
`);
        const parsed = JSON.parse(result);
        return { stdout: parsed.stdout, stderr: parsed.stderr };
      })();

      const { stdout, stderr } = await Promise.race([
        execPromise,
        timeoutPromise,
      ]);

      return {
        stdout,
        stderr,
        exit_code: 0,
        duration_ms: performance.now() - start,
      };
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      return {
        stdout: "",
        stderr: message.includes("timed out")
          ? "Python execution timed out (30s limit)"
          : message,
        exit_code: -1,
        duration_ms: performance.now() - start,
      };
    }
  }

  async execute(options: CodeExecutorOptions): Promise<ExecutionResult> {
    switch (options.language) {
      case "javascript":
      case "typescript":
        return this.executeJS(options.code);
      case "python":
        return this.executePython(options.code);
      default:
        return {
          stdout: "",
          stderr: `Unsupported language: ${options.language}`,
          exit_code: -1,
          duration_ms: 0,
        };
    }
  }
}

export const codeExecutor = new CodeExecutor();

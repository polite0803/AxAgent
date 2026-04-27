import { invoke } from "@tauri-apps/api/core";

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

interface PyodideInterface {
  runPythonAsync: (code: string) => Promise<string>;
}

class CodeExecutor {
  private pyodide: PyodideInterface | null = null;
  private pyodideLoading: Promise<void> | null = null;

  async initPyodide(): Promise<void> {
    if (this.pyodide) { return; }
    if (this.pyodideLoading) {
      await this.pyodideLoading;
      return;
    }

    this.pyodideLoading = (async () => {
      try {
        await new Promise<void>((resolve, reject) => {
          const script = document.createElement("script");
          script.src = "https://cdn.jsdelivr.net/pyodide/v0.24.1/full/pyodide.js";
          script.onload = () => resolve();
          script.onerror = () => reject(new Error("Failed to load Pyodide script"));
          document.head.appendChild(script);
        });

        this.pyodide = await window.loadPyodide({
          indexURL: "https://cdn.jsdelivr.net/pyodide/v0.24.1/full/",
        });
      } catch (e) {
        console.error("Failed to load Pyodide:", e);
        this.pyodide = null;
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
        stderr: String(error),
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
          stderr: "Pyodide failed to load",
          exit_code: -1,
          duration_ms: performance.now() - start,
        };
      }

      await this.pyodide.runPythonAsync(`
import sys
from io import StringIO
sys.stdout = StringIO()
sys.stderr = StringIO()
      `);

      await this.pyodide.runPythonAsync(code);

      const stdout = await this.pyodide.runPythonAsync("sys.stdout.getvalue()");
      const stderr = await this.pyodide.runPythonAsync("sys.stderr.getvalue()");

      return {
        stdout,
        stderr,
        exit_code: 0,
        duration_ms: performance.now() - start,
      };
    } catch (error) {
      return {
        stdout: "",
        stderr: String(error),
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

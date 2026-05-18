import { invoke } from "@/lib/invoke";

export interface ExecutionResult {
  stdout: string;
  stderr: string;
  exit_code: number;
  duration_ms?: number;
}

interface CodeExecutorOptions {
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

const PYODIDE_CDN = "https://cdn.jsdelivr.net/pyodide/v0.24.1/full/";
// TODO: Set SRI hash for the Pyodide script to prevent supply-chain attacks
const PYODIDE_SRI = "";
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
          if (PYODIDE_SRI) {
            script.integrity = PYODIDE_SRI;
          }
          script.crossOrigin = "anonymous";
          script.onload = () => resolve();
          script.onerror = () => reject(new Error("Failed to load Pyodide script"));
          document.head.appendChild(script);
        });

        this.pyodide = await window.loadPyodide({
          indexURL: PYODIDE_CDN,
        });
      } catch (e) {
        console.error("Failed to load Pyodide:", e);
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
        stderr: "Execution failed. Check your code for errors.",
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
try:
    exec(base64.b64decode("${encodedCode}").decode("utf-8"))
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
      return {
        stdout: "",
        stderr: error instanceof Error && error.message.includes("timed out")
          ? "Python execution timed out (30s limit)"
          : "Execution failed. Check your code for errors.",
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

// Helper to offload heavy operations to a Web Worker.
//
// Usage:
//   import { estimateTokensAsync, processMarkdownAsync } from "@/lib/workers";
//
//   const tokens = await estimateTokensAsync(largeText);
//   const clean = await processMarkdownAsync(markdown, true);

type TaskType = "estimateTokens" | "processMarkdown" | "formatExport";

interface PendingTask {
  resolve: (value: unknown) => void;
  reject: (reason: unknown) => void;
}

let _worker: Worker | null = null;
let _nextId = 1;
const _pending = new Map<number, PendingTask>();

function getWorker(): Worker {
  if (!_worker) {
    _worker = new Worker(
      new URL("./heavy.worker.ts", import.meta.url),
      { type: "module" },
    );
    _worker.onmessage = (event: MessageEvent<{ id: number; result?: unknown; error?: string }>) => {
      const { id, result, error } = event.data;
      const task = _pending.get(id);
      if (!task) return;
      _pending.delete(id);
      if (error) {
        task.reject(new Error(error));
      } else {
        task.resolve(result);
      }
    };
    _worker.onerror = (e) => {
      console.error("[heavy.worker] Worker error:", e);
      // Reject all pending tasks
      for (const [id, task] of _pending) {
        task.reject(new Error("Worker crashed"));
        _pending.delete(id);
      }
    };
  }
  return _worker;
}

function postTask<T>(type: TaskType, payload: unknown): Promise<T> {
  return new Promise((resolve, reject) => {
    const id = _nextId++;
    _pending.set(id, { resolve: resolve as (v: unknown) => void, reject });
    getWorker().postMessage({ id, type, payload });
  });
}

/** Estimate tokens for a text string in a Web Worker (non-blocking). */
export async function estimateTokensAsync(text: string): Promise<number> {
  if (!text) return 0;
  // For very short texts, do it synchronously to avoid worker overhead
  if (text.length < 500) {
    return Math.max(1, Math.ceil(text.length / 4));
  }
  const result = await postTask<{ tokens: number }>("estimateTokens", { text });
  return result.tokens;
}

/** Process markdown (strip axagent tags) in a Web Worker. */
export async function processMarkdownAsync(text: string, stripTags = true): Promise<string> {
  if (!text) return "";
  if (text.length < 1000) {
    // Inline for small texts
    if (stripTags) {
      return text.replace(/<think\s+data-axagent=.+?<\/think>|<\/?web-search[^>]*>/gs, "").trim();
    }
    return text;
  }
  const result = await postTask<{ result: string }>("processMarkdown", { text, stripTags });
  return result.result;
}

/** Format messages for export in a Web Worker. */
export async function formatExportAsync(
  messages: Array<{ role: string; content: string }>,
  format: "markdown" | "text",
): Promise<string> {
  if (messages.length < 50) {
    // Inline for small conversations
    return format === "markdown"
      ? "# Chat Export\n\n" + messages.map((m) => `## ${m.role}\n\n${m.content}`).join("\n\n")
      : messages.map((m) => `[${m.role}] ${m.content}`).join("\n\n");
  }
  const result = await postTask<{ result: string }>("formatExport", { messages, format });
  return result.result;
}

/** Terminate the worker (e.g., on app close). */
export function terminateWorker() {
  if (_worker) {
    _worker.terminate();
    _worker = null;
    _pending.clear();
  }
}

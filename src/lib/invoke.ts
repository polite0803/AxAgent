import { invoke as tauriInvoke } from "@tauri-apps/api/core";
import { listen as tauriListen } from "@tauri-apps/api/event";
import { handleCommand } from "./browserMock";

export type UnlistenFn = () => void;

/** Default timeout for Tauri invoke calls (5 minutes). Set to 0 to disable. */
export const DEFAULT_INVOKE_TIMEOUT_MS = 5 * 60 * 1000;

// ─── Invocation monitoring / metrics ───

interface InvokeRecord {
  command: string;
  durationMs: number;
  success: boolean;
  timestamp: number;
  error?: string;
}

const _invokeHistory: InvokeRecord[] = [];
const MAX_INVOKE_HISTORY = 500;
const _invokeCounts = new Map<string, { total: number; failed: number; totalDurationMs: number }>();

export interface InvokeMetricsSnapshot {
  byCommand: Array<{
    command: string;
    total: number;
    failed: number;
    avgDurationMs: number;
    p50Ms: number;
    p95Ms: number;
    p99Ms: number;
  }>;
  recentErrors: InvokeRecord[];
  totalCalls: number;
  totalFailed: number;
}

function recordInvocation(cmd: string, durationMs: number, success: boolean, errorMsg?: string) {
  const entry: InvokeRecord = { command: cmd, durationMs, success, timestamp: Date.now(), error: errorMsg };
  _invokeHistory.push(entry);
  if (_invokeHistory.length > MAX_INVOKE_HISTORY) {
    _invokeHistory.shift();
  }

  const stats = _invokeCounts.get(cmd) || { total: 0, failed: 0, totalDurationMs: 0 };
  stats.total++;
  stats.totalDurationMs += durationMs;
  if (!success) { stats.failed++; }
  _invokeCounts.set(cmd, stats);
}

function percentile(sorted: number[], pct: number): number {
  if (sorted.length === 0) { return 0; }
  const idx = Math.ceil(pct / 100 * sorted.length) - 1;
  return sorted[Math.max(0, idx)];
}

/**
 * Get a snapshot of invocation metrics for debugging and performance monitoring.
 */
export function getInvokeMetrics(): InvokeMetricsSnapshot {
  const byCommand = Array.from(_invokeCounts.entries()).map(([command, stats]) => {
    const durations = _invokeHistory
      .filter((r) => r.command === command)
      .map((r) => r.durationMs)
      .sort((a, b) => a - b);
    return {
      command,
      total: stats.total,
      failed: stats.failed,
      avgDurationMs: stats.total > 0 ? Math.round(stats.totalDurationMs / stats.total) : 0,
      p50Ms: percentile(durations, 50),
      p95Ms: percentile(durations, 95),
      p99Ms: percentile(durations, 99),
    };
  }).sort((a, b) => b.total - a.total);

  return {
    byCommand,
    recentErrors: _invokeHistory.filter((r) => !r.success).slice(-20),
    totalCalls: _invokeHistory.length,
    totalFailed: _invokeHistory.filter((r) => !r.success).length,
  };
}

// Slow-call threshold (3 seconds) — log warnings to console
const SLOW_CALL_THRESHOLD_MS = 3000;

export function isTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

/**
 * Invoke a Tauri command with optional timeout.
 * If the timeout elapses, the promise rejects with a TimeoutError.
 */
export async function invoke<T>(cmd: string, args?: Record<string, unknown>, timeoutMs?: number): Promise<T> {
  const start = performance.now();
  try {
    let result: T;
    if (isTauri()) {
      const effectiveTimeout = timeoutMs ?? DEFAULT_INVOKE_TIMEOUT_MS;
      result = await withTimeout<T>(
        () => tauriInvoke<T>(cmd, args),
        effectiveTimeout,
        cmd,
      );
    } else {
      result = await handleCommand<T>(cmd, args);
    }
    const elapsed = Math.round(performance.now() - start);
    recordInvocation(cmd, elapsed, true);
    if (elapsed > SLOW_CALL_THRESHOLD_MS) {
      console.warn(`[invoke] Slow call: "${cmd}" took ${elapsed}ms`);
    }
    return result;
  } catch (e) {
    const elapsed = Math.round(performance.now() - start);
    recordInvocation(cmd, elapsed, false, String(e));
    throw e;
  }
}

/**
 * Wrap a Tauri invoke call with a timeout.
 * If the call takes longer than `timeoutMs`, it rejects with a descriptive error.
 */
async function withTimeout<T>(
  fn: () => Promise<T>,
  timeoutMs: number,
  cmdName: string,
): Promise<T> {
  if (timeoutMs <= 0) {
    return fn();
  }

  // Create an AbortController-compatible timeout
  let timer: ReturnType<typeof setTimeout> | undefined;
  let timedOut = false;

  const timeoutPromise = new Promise<never>((_, reject) => {
    timer = setTimeout(() => {
      timedOut = true;
      reject(
        new Error(
          `Command "${cmdName}" timed out after ${(timeoutMs / 1000).toFixed(1)}s. `
            + `The operation may still be running in the backend. You can try again later.`,
        ),
      );
    }, timeoutMs);
  });

  try {
    const result = await Promise.race([fn(), timeoutPromise]);
    return result;
  } catch (e) {
    // Transform IPC connection errors into user-friendly messages
    const msg = String(e).toLowerCase();
    if (
      !timedOut
      && (msg.includes("connection") || msg.includes("refused") || msg.includes("fetch") || msg.includes("ipc")
        || msg.includes("protocol"))
    ) {
      throw new Error(
        `Backend connection failed for "${cmdName}". The AxAgent backend may not be running or has crashed. Please restart the application using 'npm run tauri dev'.`,
      );
    }
    throw e;
  } finally {
    if (timer !== undefined) {
      clearTimeout(timer);
    }
  }
}

export async function listen<T>(
  event: string,
  handler: (event: { payload: T }) => void,
): Promise<UnlistenFn> {
  if (isTauri()) {
    return tauriListen<T>(event, handler);
  }
  // Browser mode: no-op listener
  return () => {};
}

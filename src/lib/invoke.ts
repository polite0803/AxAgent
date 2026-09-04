// SPDX-License-Identifier: AGPL-3.0-only

import type { AutoLearnResult, Personality, PersonalityInfo } from "@/types";
import { invoke as tauriInvoke } from "@tauri-apps/api/core";
import { listen as tauriListen } from "@tauri-apps/api/event";
import { onBrowserEvent } from "./browserEvents";
import { handleCommand } from "./browserMock";

declare global {
  interface Window {
    isTauri?: boolean;
  }
  var isTauri: boolean | undefined;
}

export type UnlistenFn = () => void;

/** Default timeout for Tauri invoke calls (5 minutes). Set to 0 to disable. */
export const DEFAULT_INVOKE_TIMEOUT_MS = 5 * 60 * 1000;

/**
 * Error thrown when an IPC call exceeds its timeout.
 * Unlike transient network errors, timeout indicates the operation is too heavy
 * and should NOT be retried automatically.
 */
export class TimeoutError extends Error {
  constructor(
    public readonly cmdName: string,
    public readonly timeoutMs: number,
  ) {
    super(
      `Command "${cmdName}" timed out after ${(timeoutMs / 1000).toFixed(1)}s. `
        + `The backend operation may still be running. `
        + `Consider increasing the timeout or optimizing the operation.`,
    );
    this.name = "TimeoutError";
  }
}

// ─── 指数退避重试 ───

/** 默认重试配置 */
export interface RetryOptions {
  /** 最大重试次数（默认 2，即总共最多 3 次尝试） */
  maxRetries?: number;
  /** 初始退避延迟（毫秒，默认 1000） */
  baseDelayMs?: number;
  /** 最大退避延迟（毫秒，默认 30000） */
  maxDelayMs?: number;
  /** 退避倍数（默认 2） */
  backoffMultiplier?: number;
  /** 超时时间（毫秒），每次尝试的超时。默认使用 DEFAULT_INVOKE_TIMEOUT_MS */
  timeoutMs?: number;
}

function isRetryableError(error: unknown): boolean {
  // TimeoutError should NEVER be retried — it indicates the operation is too heavy.
  if (error instanceof TimeoutError) {
    return false;
  }
  return classifyIpcError(error) === "transient";
}

/**
 * 带指数退避的 IPC 调用重试。
 *
 * 只对瞬时网络错误（连接拒绝、超时等）进行重试，
 * 业务逻辑错误（如 NotFound、ValidationError）直接抛出。
 *
 * @example
 * const messages = await invokeWithRetry<Message[]>("list_messages", { conversationId });
 */
export async function invokeWithRetry<T>(
  cmd: string,
  args?: Record<string, unknown>,
  options?: RetryOptions,
): Promise<T> {
  const maxRetries = options?.maxRetries ?? 2;
  const baseDelayMs = options?.baseDelayMs ?? 1000;
  const maxDelayMs = options?.maxDelayMs ?? 30000;
  const multiplier = options?.backoffMultiplier ?? 2;
  const timeoutMs = options?.timeoutMs;

  let lastError: unknown;

  // 指数退避重试循环：每次重试依赖前一次失败后才执行，且每次间隔
  // 基于前一次尝试次数计算退避延迟，必须顺序执行，不能并行。
  for (let attempt = 0; attempt <= maxRetries; attempt++) {
    try {
      return await invoke<T>(cmd, args, timeoutMs);
    } catch (e) {
      lastError = e;

      // 最后一次尝试不再重试
      if (attempt >= maxRetries) {
        break;
      }

      // 非瞬时错误不重试
      if (!isRetryableError(e)) {
        throw e;
      }

      // 2.7: jitter range expanded from ±5% to ±25% to spread high-concurrency spikes
      const delay = Math.min(
        baseDelayMs * Math.pow(multiplier, attempt),
        maxDelayMs,
      );
      const jitter = delay * 0.5 * (Math.random() - 0.5);
      const actualDelay = Math.round(delay + jitter);

      console.warn(
        `[IPC 重试] "${cmd}" 第 ${attempt + 1}/${maxRetries} 次重试，等待 ${actualDelay}ms:`,
        String(e).slice(0, 120),
      );

      await sleep(actualDelay);
    }
  }

  throw lastError;
}

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

// ─── 统一 IPC 错误分类 (3.5) ───

/** IPC error category for unified error handling. */
export type IpcErrorCategory = "connection" | "transient" | "other";

/** Connection-level error patterns — the backend is unreachable. */
const CONNECTION_ERROR_PATTERNS = [
  /connection.*refused/i,
  /connection.*reset/i,
  /connection.*closed/i,
  /connection.*aborted/i,
  /fetch.*failed/i,
  /network.*error/i,
  /econnrefused/i,
  /econnreset/i,
  /socket.*hang.*up/i,
  /broken.*pipe/i,
  /protocol.*error/i,
] as const;

/** Transient error patterns that warrant a retry. */
const TRANSIENT_ERROR_PATTERNS = [
  /temporarily/i,
  /etimedout/i,
  /eagain/i,
  /resource.*busy/i,
  /too many.*requests/i,
] as const;

/**
 * Classify an IPC error into one of three categories.
 * Checks structured Error.code first, then falls back to message pattern matching.
 */
export function classifyIpcError(error: unknown): IpcErrorCategory {
  // Check structured error codes first (Tauri v2 sets .code on IPC errors)
  const code = (error as { code?: string } | undefined)?.code;
  if (code) {
    const lowered = code.toLowerCase();
    if (
      lowered.includes("connection")
      || lowered.includes("refused")
      || lowered.includes("reset")
    ) {
      return "connection";
    }
    if (lowered.includes("timeout") || lowered.includes("temporary")) {
      return "transient";
    }
  }

  const msg = error instanceof Error ? error.message : String(error);
  if (CONNECTION_ERROR_PATTERNS.some((p) => p.test(msg))) {
    return "connection";
  }
  if (TRANSIENT_ERROR_PATTERNS.some((p) => p.test(msg))) {
    return "transient";
  }
  return "other";
}

/** True if the error indicates a connection-level failure (backend unreachable). */
export function isConnectionError(error: unknown): boolean {
  return classifyIpcError(error) === "connection";
}

// ─── Invocation monitoring / metrics ───

const _invokeDurations = new Map<string, number[]>();
const MAX_INVOKE_COUNTS = 200;
const MAX_DURATIONS_PER_CMD = 200;
const MAX_RECENT_ERRORS = 50;
const _invokeCounts = new Map<
  string,
  { total: number; failed: number; totalDurationMs: number }
>();
const _recentErrors: Array<{ command: string; error?: string; timestamp: number }> = [];

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
  recentErrors: Array<{ command: string; error?: string; timestamp: number }>;
  totalCalls: number;
  totalFailed: number;
}

function recordInvocation(
  cmd: string,
  durationMs: number,
  success: boolean,
  errorMsg?: string,
) {
  const durations = _invokeDurations.get(cmd) || [];
  durations.push(durationMs);
  if (durations.length > MAX_DURATIONS_PER_CMD) {
    durations.shift();
  }
  _invokeDurations.set(cmd, durations);

  const stats = _invokeCounts.get(cmd) || {
    total: 0,
    failed: 0,
    totalDurationMs: 0,
  };
  stats.total++;
  stats.totalDurationMs += durationMs;
  if (!success) {
    stats.failed++;
    _recentErrors.push({ command: cmd, error: errorMsg, timestamp: Date.now() });
    if (_recentErrors.length > MAX_RECENT_ERRORS) {
      _recentErrors.shift();
    }
  }
  _invokeCounts.set(cmd, stats);
  if (_invokeCounts.size > MAX_INVOKE_COUNTS) {
    const oldestKey = _invokeCounts.keys().next().value;
    if (oldestKey !== undefined) {
      _invokeCounts.delete(oldestKey);
      _invokeDurations.delete(oldestKey);
    }
  }
}

// 清空 invoke 历史记录和统计计数器
export function clearInvokeHistory() {
  _invokeDurations.clear();
  _invokeCounts.clear();
  _recentErrors.length = 0;
}

function percentile(sorted: number[], pct: number): number {
  if (sorted.length === 0) {
    return 0;
  }
  const idx = Math.ceil((pct / 100) * sorted.length) - 1;
  return sorted[Math.max(0, idx)];
}

/**
 * Get a snapshot of invocation metrics for debugging and performance monitoring.
 */
export function getInvokeMetrics(): InvokeMetricsSnapshot {
  const byCommand = Array.from(_invokeCounts.entries())
    .map(([command, stats]) => {
      const durations = (_invokeDurations.get(command) || []).slice().sort((a, b) => a - b);
      return {
        command,
        total: stats.total,
        failed: stats.failed,
        avgDurationMs: stats.total > 0 ? Math.round(stats.totalDurationMs / stats.total) : 0,
        p50Ms: percentile(durations, 50),
        p95Ms: percentile(durations, 95),
        p99Ms: percentile(durations, 99),
      };
    })
    .sort((a, b) => b.total - a.total);

  const totalCalls = Array.from(_invokeCounts.values()).reduce((s, c) => s + c.total, 0);
  const totalFailed = Array.from(_invokeCounts.values()).reduce((s, c) => s + c.failed, 0);

  return {
    byCommand,
    recentErrors: [..._recentErrors],
    totalCalls,
    totalFailed,
  };
}

// Slow-call threshold (3 seconds) — log warnings to console
const SLOW_CALL_THRESHOLD_MS = 3000;

// ─── 启动诊断 ───

interface IpcDiagEntry {
  index: number;
  cmd: string;
  timestamp: number;
  timeSinceStartup: number;
  isTauri: boolean;
  success: boolean;
  durationMs?: number;
  error?: string;
}

interface IpcConnectionError {
  timestamp: number;
  cmd: string;
  error: string;
  tauriInternalsKeys?: string[];
}

interface IpcDiagState {
  startupTimestamp: number;
  firstInvokeTimestamp: number | null;
  isTauriAtStartup: boolean | null;
  calls: IpcDiagEntry[];
  connectionErrors: IpcConnectionError[];
  tauriInternalsFirstSeen: number | null;
}

function initDiagState(): IpcDiagState {
  return {
    startupTimestamp: Date.now(),
    firstInvokeTimestamp: null,
    isTauriAtStartup: null,
    calls: [],
    connectionErrors: [],
    tauriInternalsFirstSeen: null,
  };
}

function ensureDiag(): IpcDiagState {
  if (typeof window === "undefined") {
    return initDiagState();
  }
  // SAFE: diagnostic storage on window for IPC diagnostics — deliberately untyped
  const key = "__AXAGENT_IPC_DIAG__" as keyof Window & "__AXAGENT_IPC_DIAG__";
  if (!(window as unknown as Record<string, unknown>)[key]) {
    (window as unknown as Record<string, unknown>)[key] = initDiagState();
  }
  return (window as unknown as Record<string, unknown>)[key] as IpcDiagState;
}

let _diagCallIndex = 0;

function recordDiag(
  cmd: string,
  success: boolean,
  durationMs?: number,
  error?: string,
) {
  try {
    const diag = ensureDiag();
    if (diag.firstInvokeTimestamp === null) {
      diag.firstInvokeTimestamp = Date.now();
      diag.isTauriAtStartup = isTauri();
      if (diag.isTauriAtStartup) {
        diag.tauriInternalsFirstSeen = Date.now();
      }
    }
    if (diag.calls.length < 200) {
      diag.calls.push({
        index: _diagCallIndex++,
        cmd,
        timestamp: Date.now(),
        timeSinceStartup: Date.now() - diag.startupTimestamp,
        isTauri: isTauri(),
        success,
        durationMs,
        error: error?.slice(0, 200),
      });
    }
    if (!success && error) {
      if (classifyIpcError(new Error(error)) === "connection") {
        if (diag.connectionErrors.length < 50) {
          // SAFE: checking for Tauri runtime internals on window object
          const internalsObj = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window
            ? ((window as unknown as Record<string, unknown>)
              .__TAURI_INTERNALS__ as Record<string, unknown>)
            : undefined;
          diag.connectionErrors.push({
            timestamp: Date.now(),
            cmd,
            error: error.slice(0, 300),
            tauriInternalsKeys: internalsObj
              ? Object.keys(internalsObj).slice(0, 20)
              : undefined,
          });
        }
      }
    }
  } catch {
    /* diagnostic: ignore self-check errors */
  }
}

/**
 * 检查 IPC 通道健康状态。
 * 在 Tauri 环境尝试一次轻量 IPC，带 5 秒超时。
 */
export async function checkIpcHealth(): Promise<{
  ok: boolean;
  detail: string;
  isTauri: boolean;
}> {
  if (!isTauri()) {
    return {
      ok: false,
      detail: "Not a Tauri environment, __TAURI_INTERNALS__ not injected",
      isTauri: false,
    };
  }
  try {
    await invoke<unknown>("get_settings", undefined, 5000);
    return { ok: true, detail: "IPC channel OK", isTauri: true };
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e);
    return { ok: false, detail: msg.slice(0, 200), isTauri: true };
  }
}

export function isTauri(): boolean {
  return (
    !!globalThis.isTauri || !!(typeof window !== "undefined" && window.isTauri)
  );
}

/**
 * Invoke a Tauri command with optional timeout.
 * If the timeout elapses, the promise rejects with a TimeoutError.
 */
export async function invoke<T>(
  cmd: string,
  args?: Record<string, unknown>,
  timeoutMs?: number,
): Promise<T> {
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
    recordDiag(cmd, true, elapsed);
    if (elapsed > SLOW_CALL_THRESHOLD_MS) {
      console.warn(`[invoke] Slow call: "${cmd}" took ${elapsed}ms`);
    }
    return result;
  } catch (e) {
    const elapsed = Math.round(performance.now() - start);
    const errorMsg = String(e);
    recordInvocation(cmd, elapsed, false, errorMsg);
    recordDiag(cmd, false, elapsed, errorMsg);
    throw e;
  }
}

/**
 * Wrap a Tauri invoke call with a timeout, implementing soft cancellation.
 *
 * - On timeout: throws `TimeoutError`, and the cancellation token prevents
 *   the original promise result from ever being processed (even if it resolves
 *   after the timeout, the result is discarded).
 * - The backend operation continues running (Tauri v2 IPC has no abort
 *   mechanism), but the frontend will not update state with stale results.
 * - TimeoutError is NOT retried by `invokeWithRetry` — the operation is too
 *   heavy or the backend is stuck, and retrying would only compound the problem.
 */
async function withTimeout<T>(
  fn: () => Promise<T>,
  timeoutMs: number,
  cmdName: string,
): Promise<T> {
  if (timeoutMs <= 0) {
    return fn();
  }

  let timer: ReturnType<typeof setTimeout> | undefined;
  const cancelled = { value: false };

  // Wrap fn() so that if it resolves after timeout, the result is discarded.
  const guardedFn = fn().then((result) => {
    if (cancelled.value) {
      // Silently discard — the caller already received a TimeoutError.
      return new Promise<never>(() => {});
    }
    return result;
  });

  const timeoutPromise = new Promise<never>((_resolve, reject) => {
    timer = setTimeout(() => {
      cancelled.value = true;
      reject(new TimeoutError(cmdName, timeoutMs));
    }, timeoutMs);
  });

  try {
    const result = await Promise.race([guardedFn, timeoutPromise]);
    return result;
  } catch (e) {
    // Only attempt connection-error rewrapping for non-timeout errors.
    if (!cancelled.value && isConnectionError(e)) {
      throw new Error(
        `Backend connection failed for "${cmdName}". The AxAgent backend may not be running or has crashed. Please restart the application using 'npm run tauri dev'.`,
        { cause: e },
      );
    }
    throw e;
  } finally {
    if (timer !== undefined) {
      clearTimeout(timer);
    }
  }
}

/**
 * 创建统一的 IPC 错误日志回调，替代散布各处的 .catch(() => {})
 * 用法: invoke("command", args).catch(logIpcError("操作描述"))
 *
 * 当 notify=true 时，同时推送到错误通知 store 让用户可见。
 */
export function logIpcError(
  context: string,
  options?: { notify?: boolean; retryFn?: () => Promise<unknown> },
): (err: unknown) => void {
  return (err: unknown) => {
    const message = err instanceof Error ? err.message : String(err);
    console.warn(`[IPC] ${context}: ${message}`);

    if (options?.notify) {
      (async () => {
        const { useErrorNotificationStore } = await import("@/stores/shared/errorNotificationStore");
        useErrorNotificationStore.getState().pushError({
          message,
          context,
          retryFn: options.retryFn,
        });
      })().catch((err) => {
        console.warn("[invoke]", err);
      });
    }
  };
}

/**
 * Create an error handler for React components that sets an error state.
 * Combines logIpcError logging with a state setter for UI feedback.
 *
 * @example
 * const [error, setError] = useState<string | null>(null);
 * invoke("command", args).catch(createErrorHandler("operation", setError));
 */
export function createErrorHandler(
  context: string,
  setError: (error: string | null) => void,
): (err: unknown) => void {
  return (err: unknown) => {
    const message = err instanceof Error ? err.message : String(err);
    console.warn(`[IPC] ${context}: ${message}`);
    setError(message);
  };
}

/**
 * Create an error handler that logs and shows a toast notification.
 * Use this for fire-and-forget operations where the user should be notified.
 *
 * @example
 * invoke("command", args).catch(logAndNotify("operation"));
 */
export function logAndNotify(context: string): (err: unknown) => void {
  return (err: unknown) => {
    const message = err instanceof Error ? err.message : String(err);
    console.warn(`[IPC] ${context}: ${message}`);
    import("antd").then(({ message: messageApi }) => {
      messageApi.error(`${context} failed: ${message.slice(0, 100)}`);
    }).catch((err) => {
      console.warn("[invoke]", err);
    });
  };
}

export async function listen<T>(
  event: string,
  handler: (event: { payload: T }) => void,
): Promise<UnlistenFn> {
  if (isTauri()) {
    return tauriListen<T>(event, handler);
  }
  // Browser mode: 经由内存事件总线订阅，使 browserMock 能派发事件（如计划确认闸门），
  // 从而支持事件驱动的 UI 流程在 e2e 中被真实触发。
  return onBrowserEvent(event, (payload) => handler({ payload: payload as T }));
}

// ── Personality / Persona ───────────────────────────────────────────────

export interface PersonalityCreateBootstrapInput {
  name: string;
  soul?: string;
  identity?: string;
  user?: string;
}

export function personalityList(): Promise<PersonalityInfo[]> {
  return invoke("personality_list");
}

export function personalityGet(name: string): Promise<Personality> {
  return invoke("personality_get", { name });
}

export function personalitySwitch(name: string): Promise<void> {
  return invoke("personality_switch", { name });
}

export function personalityCreateBootstrap(input: PersonalityCreateBootstrapInput): Promise<void> {
  return invoke("personality_create_bootstrap", input as unknown as Record<string, unknown>);
}

export function personalityUpdateIdentity(name: string, identity: string): Promise<void> {
  return invoke("personality_update_identity", { name, identity });
}

export function personalityUpdateUser(name: string, user: string): Promise<void> {
  return invoke("personality_update_user", { name, user });
}

/**
 * 从指定对话中自动学习用户风格，更新当前激活 Persona 的 USER.md
 *
 * 内部会调用 trajectory 的 StyleExtractor + StyleVectorizer，
 * 将提取的代码风格、命名约定、沟通偏好回写到 USER.md。
 */
export function personalityAutoLearnFromConversation(
  conversationId: string,
): Promise<AutoLearnResult> {
  return invoke("personality_auto_learn_from_conversation", { conversationId });
}

// ── 工作流反思 / 进化 / 优化命令(阶段 5 wiring 暴露) ──
//
// 以下 6 个函数对应后端 `commands::workflow_reflection` 6 个 Tauri 命令。
// 错误处理:命令层返回 `Result<T, String>`,String 为 ErrorResponse 的 JSON 序列化。
// 调用方应通过 `@/lib/errorI18n` 的 `showBackendError()` 解析错误码并走 i18n 翻译,
// 详见 `hooks/workflow/useWorkflowReflection`。

/** 反思结果(与后端 `axagent_harness::reflection_types::Reflection` 对齐)。 */
export interface WorkflowReflection {
  task_id: string;
  timestamp: string;
  quality_score: number;
  quality_analysis: string;
  efficiency_analysis: string;
  error_patterns: string[];
  reusable_patterns: string[];
  knowledge_suggestions: string[];
  improvement_suggestions: string[];
  overall_summary: string;
  quality_metrics?: {
    task_success_score: number;
    tool_efficiency_score: number;
    iteration_efficiency_score: number;
    time_efficiency_score: number;
    error_recovery_score: number;
    goal_completion_score: number;
    overall_weighted_score: number;
  };
  metadata?: unknown;
}

/** 优化建议分类(与后端 `SuggestionCategory` 对齐)。 */
export type WorkflowSuggestionCategory =
  | "NodeConfig"
  | "NodeReplacement"
  | "EdgeRewire"
  | "PromptRefine"
  | "ErrorHandling"
  | "VariableMisconfig"
  | "ResourceTuning";

/** 优化建议优先级。 */
export type WorkflowSuggestionPriority = "Critical" | "High" | "Medium" | "Low";

/** 单条优化建议(与后端 `WorkflowSuggestion` 对齐)。 */
export interface WorkflowSuggestion {
  id: string;
  category: WorkflowSuggestionCategory;
  priority: WorkflowSuggestionPriority;
  target_node_id: string | null;
  description: string;
  proposed_change: Record<string, unknown>;
  confidence: number;
  estimated_impact?: number;
}

/** 进化统计(与后端 `EvolutionStats` 对齐)。 */
export interface WorkflowEvolutionStats {
  generation: number;
  best_fitness: number;
  avg_fitness: number;
  fitness_history: number[];
  converged: boolean;
}

/** 沙箱验证结果。 */
export interface SandboxValidationResult {
  passed: boolean;
  success_rate: number;
  execution_errors: string[];
  avg_execution_time_ms: number;
}

/** 进化修改结果。 */
export interface WorkflowModification {
  evolved_genome: unknown;
  changes: unknown[];
  validation: SandboxValidationResult;
}

/**
 * 基于单次反思生成工作流优化建议。
 *
 * 调用后端 `workflow_optimize_suggest` 命令。
 * 失败时抛出字符串(JSON 形式的 ErrorResponse),调用方应通过 `@/lib/errorI18n` 的 `showBackendError()` 解析。
 */
export function workflowOptimizeSuggest(
  template: unknown,
  reflection: WorkflowReflection,
): Promise<WorkflowSuggestion[]> {
  return invoke<WorkflowSuggestion[]>("workflow_optimize_suggest", {
    request: { template, reflection },
  });
}

/**
 * 批量应用优化建议到模板,返回新模板(不修改原模板)。
 *
 * 调用后端 `workflow_optimize_apply` 命令。
 * 返回应用建议后的新模板,调用方决定是否持久化。
 */
export function workflowOptimizeApply(
  template: unknown,
  suggestions: WorkflowSuggestion[],
): Promise<unknown> {
  return invoke<unknown>("workflow_optimize_apply", {
    request: { template, suggestions },
  });
}

/**
 * 触发工作流模板进化(基于反思批量进化,返回最终修改结果)。
 *
 * 调用后端 `workflow_evolve_template` 命令。
 * 建议在前端以异步任务形式调用(可能耗时较长)。
 */
export function workflowEvolveTemplate(
  templateId: string,
  reflections: WorkflowReflection[],
): Promise<WorkflowModification> {
  return invoke<WorkflowModification>("workflow_evolve_template", {
    request: { template_id: templateId, reflections },
  });
}

/** 查询工作流进化器的统计信息(当前代数、最佳 / 平均适应度、是否收敛)。 */
export function workflowEvolutionStats(): Promise<WorkflowEvolutionStats> {
  return invoke<WorkflowEvolutionStats>("workflow_evolution_stats");
}

/** 查询进化器是否正在执行(用于前端防重入)。 */
export function workflowEvolutionIsRunning(): Promise<boolean> {
  return invoke<boolean>("workflow_evolution_is_running");
}

/**
 * 查询是否应自动触发进化(基于近期失败率与使用次数)。
 *
 * 注意:依赖 evolver 内部的 `recent_reflections` 历史,
 * 若 wiring 层未注入反思历史记录机制,本命令始终返回 false。
 */
export function workflowShouldAutoEvolve(templateId: string): Promise<boolean> {
  return invoke<boolean>("workflow_should_auto_evolve", { templateId });
}

// ============================================================================
// Agent Session Broker (MCP / 前端会话管理面板)
// ============================================================================

import type { AgentSessionStatusView } from "@/types";

/** 查询指定 agent 会话的当前状态。 */
export function agentSessionStatus(sessionId: string): Promise<AgentSessionStatusView> {
  return invoke<AgentSessionStatusView>("agent_session_status", { sessionId });
}

/** 取消指定 agent 会话的执行 (底层会给 ReActEngine 发取消信号)。 */
export function agentSessionCancel(sessionId: string): Promise<void> {
  return invoke<void>("agent_session_cancel", { sessionId });
}

/** 列出所有已知 agent 会话 ID (含 DB 中已完成的历史会话)。 */
export function agentSessionList(): Promise<string[]> {
  return invoke<string[]>("agent_session_list");
}

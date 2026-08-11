// SPDX-License-Identifier: AGPL-3.0-only

// ! 后端错误码 → 前端 i18n 统一翻译层（阶段 1）
// !
// ! 后端命令以结构化 JSON 返回错误（见 src-tauri/src/commands/error.rs 的 `ErrorResponse`）：
// ! `{ code, category, detail?, params? }`。前端在此统一：
// !   1. 归一化各种形态的错误（Tauri 序列化对象 / Error / JSON 字符串 / 纯字符串）；
// !   2. 按 `error.${code}` 查 locale 翻译，命中则用；未命中回退 detail / 原始文本；
// !   3. 按 `category` 做智能分支提示（可重试 → warning，其余 → error）。
// !
// ! 用法：
// ! ```ts
// ! const { message } = App.useApp();
// ! try { await invoke("cmd"); }
// ! catch (e) { showBackendError(message, e); }
// ! ```

import i18n from "@/i18n";

/** 后端错误分类，与 src-tauri/src/commands/error.rs 的 `ErrorCategory`（snake_case）一致。 */
export type BackendErrorCategory =
  | "retryable"
  | "permission_denied"
  | "unrecoverable"
  | "validation"
  | "general";

const KNOWN_CATEGORIES: ReadonlySet<string> = new Set([
  "retryable",
  "permission_denied",
  "unrecoverable",
  "validation",
  "general",
]);

/** 归一化后的后端错误结构。 */
export interface ParsedBackendError {
  /** 错误码（形如 CONVERSATION_NOT_FOUND），仅当匹配码格式时存在。 */
  code?: string;
  /** 错误分类，供智能分支。 */
  category?: BackendErrorCategory;
  /** 技术详情（调试用，通常不直接展示给用户）。 */
  detail?: string;
  /** 翻译占位符参数。 */
  params?: Record<string, string>;
  /** 原始文本（无法结构化时的兜底展示内容）。 */
  raw: string;
}

/** 后端错误码格式：全大写 + 下划线，至少两段（如 TOOL_NOT_FOUND）。 */
const ERROR_CODE_RE = /^[A-Z][A-Z0-9]+(?:_[A-Z0-9]+)+$/;

/** 单花括号占位符 `{key}`（兼容 locale 中非 i18next 标准的单括号写法）。 */
const SINGLE_BRACE_RE = /\{([a-zA-Z][a-zA-Z0-9_]*)\}/g;

/** 从任意错误取原始字符串。 */
function toRawText(e: unknown): string {
  if (e == null) {
    return "";
  }
  if (typeof e === "string") {
    return e;
  }
  if (e instanceof Error) {
    return e.message;
  }
  if (typeof e === "object") {
    // Tauri 可能把 ErrorResponse 直接序列化为对象，先尝试 JSON 化用于兜底展示。
    try {
      return JSON.stringify(e);
    } catch {
      return String(e);
    }
  }
  return String(e);
}

/** 把候选对象归一化为 ParsedBackendError（若含合法 code）。 */
function fromObject(obj: Record<string, unknown>, raw: string): ParsedBackendError | null {
  const code = typeof obj.code === "string" ? obj.code : undefined;
  if (!code || !ERROR_CODE_RE.test(code)) {
    return null;
  }
  const category = typeof obj.category === "string" && KNOWN_CATEGORIES.has(obj.category)
    ? (obj.category as BackendErrorCategory)
    : undefined;
  const detail = typeof obj.detail === "string" ? obj.detail : undefined;
  let params: Record<string, string> | undefined;
  if (obj.params && typeof obj.params === "object") {
    params = {};
    for (const [k, v] of Object.entries(obj.params as Record<string, unknown>)) {
      params[k] = typeof v === "string" ? v : String(v);
    }
  }
  return { code, category, detail, params, raw };
}

/**
 * 归一化后端错误：兼容 Tauri 序列化对象、Error(message 为 JSON)、JSON 字符串、纯字符串。
 * 无法结构化时返回仅含 `raw` 的对象。
 */
export function parseBackendError(e: unknown): ParsedBackendError {
  const raw = toRawText(e);

  // 1) 已经是对象且带合法 code（Tauri 直接序列化 ErrorResponse 的情况）
  if (e && typeof e === "object" && !(e instanceof Error)) {
    const parsed = fromObject(e as Record<string, unknown>, raw);
    if (parsed) {
      return parsed;
    }
  }

  // 2) 字符串（含 Error.message）尝试当 JSON 解析
  const jsonCandidate = raw.trim();
  if (jsonCandidate.startsWith("{") && jsonCandidate.endsWith("}")) {
    try {
      const obj = JSON.parse(jsonCandidate) as Record<string, unknown>;
      const parsed = fromObject(obj, raw);
      if (parsed) {
        return parsed;
      }
    } catch {
      // 非 JSON，走兜底
    }
  }

  // 3) 兜底：无结构化信息
  return { raw };
}

/** 手动替换单花括号占位符 `{key}`（i18next 已处理双花括号 `{{key}}`）。 */
function applySingleBraceParams(text: string, params?: Record<string, string>): string {
  if (!params) {
    return text;
  }
  return text.replace(SINGLE_BRACE_RE, (match, key: string) => {
    return Object.prototype.hasOwnProperty.call(params, key) ? params[key] : match;
  });
}

/**
 * 把后端错误翻译成用户可见文本。
 * - 命中 `error.${code}` → 返回翻译（双花括号由 i18next 插值，单花括号手动补插）；
 * - 未命中 → 回退 detail，再回退原始文本。
 */
export function translateBackendError(e: unknown): string {
  const parsed = parseBackendError(e);

  if (parsed.code && ERROR_CODE_RE.test(parsed.code)) {
    // COMMON_INTERNAL 是 `from_error` 的兜底码，无具体 i18n 翻译价值，
    // 直接展示原始 detail / 文本，保持与迁移前（裸串透传）一致的UX，
    // 避免用户只看到通用的“操作失败，请稍后重试”而丢失真实报错。
    if (parsed.code !== "COMMON_INTERNAL") {
      const key = `error.${parsed.code}`;
      const translated = i18n.t(key, { ...parsed.params, defaultValue: "" });
      if (translated && translated !== key) {
        return applySingleBraceParams(translated, parsed.params);
      }
    }
  }

  return parsed.detail || parsed.raw || String(e);
}

/** 取后端错误分类（无则 undefined）。 */
export function getBackendErrorCategory(e: unknown): BackendErrorCategory | undefined {
  return parseBackendError(e).category;
}

/** 最小 antd message API 形状（兼容 App.useApp() 与静态 import 两种来源）。 */
export interface MessageApiLike {
  error: (content: string, duration?: number) => void;
  warning: (content: string, duration?: number) => void;
}

/** showBackendError 选项。 */
export interface ShowBackendErrorOptions {
  /** 上下文描述，仅用于 console 定位问题（不展示给用户）。 */
  context?: string;
  /** toast 时长（秒）。 */
  duration?: number;
}

/**
 * 翻译后端错误并按分类弹出 toast：
 * - `retryable` → warning（用户可稍后重试）；
 * - 其余分类 → error。
 *
 * 返回翻译后的文本，便于调用方复用（如同时写入表单错误态）。
 */
export function showBackendError(
  message: MessageApiLike,
  e: unknown,
  options?: ShowBackendErrorOptions,
): string {
  const parsed = parseBackendError(e);
  const text = translateBackendError(e);

  if (options?.context) {
    const dbg = parsed.detail || parsed.raw;
    console.warn(`[backendError] ${options.context}: ${parsed.code ?? "?"} ${dbg}`.trim());
  }

  if (parsed.category === "retryable") {
    message.warning(text, options?.duration);
  } else {
    message.error(text, options?.duration);
  }

  return text;
}

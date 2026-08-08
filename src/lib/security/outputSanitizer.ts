// SPDX-License-Identifier: AGPL-3.0-only

import { escapeHtml } from "./injectionDetector";

/** 输出风险级别 */
export type OutputRiskLevel = "safe" | "warning" | "blocked";

/** 输出净化结果 */
export interface SanitizedOutput {
  content: string;
  riskLevel: OutputRiskLevel;
  flags: string[];
  actions: string[];
  warnings: string[];
}

/** 输出安全过滤器 */
export class OutputSanitizer {
  private contentFilters: Array<(content: string) => string>;
  private flagChecks: Array<{ pattern: RegExp; flag: string }>;

  constructor() {
    this.contentFilters = [
      (c) => escapeHtml(c),
      (c) =>
        c.replace(
          /(?:system|assistant|user)\s*:\s*(?:ignore|disregard|forget)\s+(?:all\s+)?(?:previous|prior)\s+(?:instructions?|prompts?|rules?)/gi,
          "[安全过滤: 提示注入尝试已移除]",
        ),
      (c) =>
        c.replace(
          /DAN\s*(?:mode)?|do\s+anything\s+now/i,
          "[安全过滤: 越狱尝试已移除]",
        ),
    ];

    this.flagChecks = [
      { pattern: /<script[\s>]/i, flag: "contains_script_tag" },
      { pattern: /javascript\s*:/i, flag: "contains_javascript_protocol" },
      { pattern: /<iframe[\s>]/i, flag: "contains_iframe" },
      { pattern: /on\w+\s*=/i, flag: "contains_event_handler" },
      { pattern: /eval\s*\(/i, flag: "contains_eval" },
      { pattern: /\b(?:api|webhook|endpoint)[^\s]*\b.*(?:send|post|upload)\b/i, flag: "potential_data_exfiltration" },
      { pattern: /(?:password|secret|api_key|token)\s*[:=]\s*\S+/i, flag: "contains_sensitive_data" },
    ];
  }

  /** 添加自定义过滤器 */
  addFilter(filter: (content: string) => string): void {
    this.contentFilters.push(filter);
  }

  /** 添加自定义标志检查 */
  addFlagCheck(pattern: RegExp, flag: string): void {
    this.flagChecks.push({ pattern, flag });
  }

  /** 净化输出内容 */
  sanitize(content: string): SanitizedOutput {
    let sanitized = content;
    const flags: string[] = [];
    const warnings: string[] = [];
    const actions: string[] = [];

    for (const check of this.flagChecks) {
      if (check.pattern.test(content)) {
        flags.push(check.flag);
      }
    }

    for (const filter of this.contentFilters) {
      sanitized = filter(sanitized);
    }

    let riskLevel: OutputRiskLevel = "safe";

    if (flags.some((f) => f.includes("script") || f.includes("javascript"))) {
      riskLevel = "blocked";
      warnings.push("检测到脚本注入风险，已自动过滤");
      actions.push("script_blocked");
    } else if (flags.includes("potential_data_exfiltration")) {
      riskLevel = "warning";
      warnings.push("检测到可能的数据外泄意图");
      actions.push("exfiltration_warning");
    } else if (flags.includes("contains_sensitive_data")) {
      riskLevel = "warning";
      warnings.push("输出中可能包含敏感信息");
      actions.push("sensitive_data_warning");
    } else if (flags.length > 0) {
      riskLevel = "warning";
      warnings.push(`检测到 ${flags.length} 个安全标志`);
      actions.push("flagged");
    }

    const MAX_OUTPUT_LENGTH = 100_000;
    if (sanitized.length > MAX_OUTPUT_LENGTH) {
      sanitized = sanitized.slice(0, MAX_OUTPUT_LENGTH) + "\n\n[输出已截断以保护性能]";
      warnings.push(`输出超过 ${MAX_OUTPUT_LENGTH} 字符，已截断`);
      riskLevel = riskLevel === "safe" ? "warning" : riskLevel;
      actions.push("truncated");
    }

    return {
      content: sanitized,
      riskLevel,
      flags,
      actions,
      warnings,
    };
  }

  /** 安全渲染（React 场景） */
  sanitizeForRender(content: string): {
    safeHtml: string;
    wasModified: boolean;
    warnings: string[];
  } {
    const result = this.sanitize(content);
    return {
      safeHtml: result.content,
      wasModified: result.riskLevel !== "safe",
      warnings: result.warnings,
    };
  }
}

let _sanitizer: OutputSanitizer | null = null;
export function getOutputSanitizer(): OutputSanitizer {
  if (!_sanitizer) {
    _sanitizer = new OutputSanitizer();
  }
  return _sanitizer;
}

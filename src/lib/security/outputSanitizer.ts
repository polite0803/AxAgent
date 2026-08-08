// SPDX-License-Identifier: AGPL-3.0-only

import { escapeHtml } from "./injectionDetector";

/** Output risk level */
export type OutputRiskLevel = "safe" | "warning" | "blocked";

/** Sanitized output */
export interface SanitizedOutput {
  content: string;
  riskLevel: OutputRiskLevel;
  flags: string[];
  actions: string[];
  warnings: string[];
}

/** Output sanitizer */
export class OutputSanitizer {
  private contentFilters: Array<(content: string) => string>;
  private flagChecks: Array<{ pattern: RegExp; flag: string }>;

  constructor() {
    this.contentFilters = [
      (c) => escapeHtml(c),
      (c) =>
        c.replace(
          /(?:system|assistant|user)\s*:\s*(?:ignore|disregard|forget)\s+(?:all\s+)?(?:previous|prior)\s+(?:instructions?|prompts?|rules?)/gi,
          "[SAFE FILTERED: Prompt injection attempt removed]",
        ),
      (c) =>
        c.replace(
          /DAN\s*(?:mode)?|do\s+anything\s+now/i,
          "[SAFE FILTERED: Jailbreak attempt removed]",
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

  /** Add custom filter */
  addFilter(filter: (content: string) => string): void {
    this.contentFilters.push(filter);
  }

  /** Add custom flag check */
  addFlagCheck(pattern: RegExp, flag: string): void {
    this.flagChecks.push({ pattern, flag });
  }

  /** Sanitize output content */
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
      warnings.push("Script injection risk detected, automatically filtered");
      actions.push("script_blocked");
    } else if (flags.includes("potential_data_exfiltration")) {
      riskLevel = "warning";
      warnings.push("Potential data exfiltration intent detected");
      actions.push("exfiltration_warning");
    } else if (flags.includes("contains_sensitive_data")) {
      riskLevel = "warning";
      warnings.push("Output may contain sensitive information");
      actions.push("sensitive_data_warning");
    } else if (flags.length > 0) {
      riskLevel = "warning";
      warnings.push(`${flags.length} security flag(s) detected`);
      actions.push("flagged");
    }

    const MAX_OUTPUT_LENGTH = 100_000;
    if (sanitized.length > MAX_OUTPUT_LENGTH) {
      sanitized = sanitized.slice(0, MAX_OUTPUT_LENGTH) + "\n\n[Output truncated for performance]";
      warnings.push(`Output exceeded ${MAX_OUTPUT_LENGTH} characters, truncated`);
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

  /** Safe rendering (React scenario) */
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

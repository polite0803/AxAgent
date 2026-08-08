// SPDX-License-Identifier: AGPL-3.0-only

/**
 * StyleSanitizer — DynamicUI schema.style 安全过滤。
 *
 * 背景：DynamicUIRenderer 渲染 schema 时，schema.style 会被直接展开为
 * React.CSSProperties（见 Container/MarkdownView 等组件）。虽然内联 style
 * 不会执行脚本，但危险 CSS 值（url(javascript:)、expression()、behavior、
 * @import 等）存在样式注入与隐私探测面，且 agent 生成的 schema 不可信。
 *
 * 策略：值级过滤为主（拒绝危险模式），属性黑名单为辅——尽量不破坏合法布局。
 * 与 OutputSanitizer（内容净化）形成两层防御。
 */

/** 危险 CSS 值模式：命中任一即丢弃该属性 */
const DANGEROUS_CSS_VALUE_PATTERNS: RegExp[] = [
  /url\s*\(/i,
  /expression\s*\(/i,
  /javascript\s*:/i,
  /vbscript\s*:/i,
  /@import/i,
  /behavior\s*:/i,
  /-moz-binding/i,
  /-webkit-user-drag/i,
  /content\s*:\s*["']?/i,
];

/** 禁止的 CSS 属性（即便值安全也拒绝，防止 UI 欺骗/遮挡） */
const BLOCKED_CSS_PROPERTIES: ReadonlySet<string> = new Set([
  "position",
  "zIndex",
  "pointerEvents",
  "userSelect",
  "filter",
  "backdropFilter",
  "mixBlendMode",
  "clipPath",
]);

export interface SanitizeStyleResult {
  /** 过滤后的安全样式 */
  style: Record<string, string | number> | undefined;
  /** 被拒绝的属性名列表 */
  blocked: string[];
}

export class StyleSanitizer {
  /** 过滤单个样式对象，返回安全子集与拒绝清单 */
  sanitize(
    style: Record<string, string | number> | undefined,
  ): SanitizeStyleResult {
    if (!style || typeof style !== "object") {
      return { style: undefined, blocked: [] };
    }

    const safe: Record<string, string | number> = {};
    const blocked: string[] = [];

    for (const [key, rawValue] of Object.entries(style)) {
      const prop = key.trim();
      if (BLOCKED_CSS_PROPERTIES.has(prop)) {
        blocked.push(prop);
        continue;
      }

      const value = String(rawValue);
      if (DANGEROUS_CSS_VALUE_PATTERNS.some((re) => re.test(value))) {
        blocked.push(prop);
        continue;
      }

      safe[key] = rawValue;
    }

    return { style: Object.keys(safe).length > 0 ? safe : undefined, blocked };
  }
}

let _instance: StyleSanitizer | null = null;

/** 单例获取（与 OutputSanitizer 的 getOutputSanitizer 风格保持一致） */
export function getStyleSanitizer(): StyleSanitizer {
  if (!_instance) {
    _instance = new StyleSanitizer();
  }
  return _instance;
}

// SPDX-License-Identifier: AGPL-3.0-only

// LaTeX 数学公式渲染组件 + Markdown 内容中 $...$ / $$...$$ 公式分割工具
// 集成入口：ChatMarkdownNodes.tsx 的 AssistantMarkdown

import { BlockMath, InlineMath } from "react-katex";

// 引入 KaTeX 样式（含字体）；Vite/Esm 会自动去重，多次实例化不会重复打包
import "katex/dist/katex.min.css";

import type { ReactNode } from "react";

interface LatexRendererProps {
  // LaTeX 公式原文（不含首尾 $ 或 $$）
  content: string;
  // true = 块级公式（居中独占一行），false = 行内公式
  displayMode?: boolean;
}

/**
 * LaTeX 渲染失败的回退节点：红色边框 + 原始公式文本
 * 方便用户定位问题并复制原文
 */
function renderLatexError(content: string): (error: Error) => ReactNode {
  return (_error: Error) => (
    <span
      style={{
        display: "inline-block",
        border: "1px solid #ff4d4f",
        color: "#ff4d4f",
        padding: "2px 6px",
        borderRadius: 4,
        fontFamily: "var(--font-mono, 'JetBrains Mono', ui-monospace, monospace)",
        fontSize: "0.9em",
        backgroundColor: "rgba(255, 77, 79, 0.06)",
        whiteSpace: "pre-wrap",
        wordBreak: "break-all",
      }}
      title="LaTeX 解析失败"
    >
      {content}
    </span>
  );
}

/**
 * LaTeX 公式渲染组件
 *
 * 使用 react-katex 的 InlineMath / BlockMath 渲染。
 * 解析失败时回退显示原始 LaTeX 文本（红色边框提示）。
 */
export function LatexRenderer({ content, displayMode = false }: LatexRendererProps) {
  const errorRenderer = renderLatexError(content);
  if (displayMode) {
    return <BlockMath math={content} renderError={errorRenderer} />;
  }
  return <InlineMath math={content} renderError={errorRenderer} />;
}

// ── Markdown 内容分割工具 ────────────────────────────────────────────────

// 分割后的段落类型：
// - text: 普通文本，走原 Markdown 渲染器
// - inline-math: 行内公式 $...$，走 LatexRenderer
// - block-math: 块级公式 $$...$$，走 LatexRenderer
export type LatexSegment =
  | { type: "text"; content: string }
  | { type: "inline-math"; content: string }
  | { type: "block-math"; content: string };

// 匹配顺序（优先级从高到低）：
// 1. ```代码块``` —— 内部 $ 不解析
// 2. `行内代码` —— 内部 $ 不解析
// 3. \$ —— 转义美元，当作普通文本（不再作为公式分隔符）
// 4. $$...$$ —— 块级公式（可跨行）
// 5. $...$ —— 行内公式（单行内，不可跨行）
const LATEX_TOKEN_RE = /(```[\s\S]*?```|`[^`\n]+`|\\\$|\$\$[\s\S]+?\$\$|\$[^\$\n]+?\$)/g;

// 把文本段追加到 segments，若上一段也是 text 则合并（减少渲染单元）
function pushText(segments: LatexSegment[], text: string): void {
  if (!text) {
    return;
  }
  const last = segments[segments.length - 1];
  if (last && last.type === "text") {
    last.content += text;
  } else {
    segments.push({ type: "text", content: text });
  }
}

/**
 * 将 Markdown 文本按 LaTeX 公式分割为若干段落。
 *
 * 解析规则：
 * - 块级公式 $$...$$ 优先匹配（避免被行内解析破坏）
 * - 行内公式 $...$ 仅在单行内匹配，不跨行
 * - 代码块 / 行内代码 / 转义 \$ 内部的 $ 视为字面量，不解析
 * - 相邻文本段会合并，减少渲染开销
 *
 * 不含 $ 的内容走快速路径，原样返回单段 text。
 */
export function splitContentWithLatex(content: string): LatexSegment[] {
  // 快速路径：无 $ 直接返回，避免正则开销
  if (!content.includes("$")) {
    return [{ type: "text", content }];
  }

  const segments: LatexSegment[] = [];
  // 重置正则 lastIndex（全局正则复用安全）
  LATEX_TOKEN_RE.lastIndex = 0;
  let lastIndex = 0;
  let match: RegExpExecArray | null;

  while ((match = LATEX_TOKEN_RE.exec(content)) !== null) {
    const start = match.index;
    // 匹配项前面的普通文本
    if (start > lastIndex) {
      pushText(segments, content.slice(lastIndex, start));
    }
    const token = match[0];
    if (token.startsWith("$$")) {
      // 块级公式：去掉首尾 $$
      segments.push({ type: "block-math", content: token.slice(2, -2) });
    } else if (token.startsWith("$")) {
      // 行内公式：去掉首尾 $
      segments.push({ type: "inline-math", content: token.slice(1, -1) });
    } else {
      // 代码块 / 行内代码 / 转义美元 -> 当作普通文本走 Markdown
      pushText(segments, token);
    }
    lastIndex = start + token.length;
  }

  // 尾部剩余文本
  if (lastIndex < content.length) {
    pushText(segments, content.slice(lastIndex));
  }

  // 全部为文本时，统一返回单段（保持与无 $ 快速路径一致的结构）
  if (segments.length > 0 && segments.every((s) => s.type === "text")) {
    return [{ type: "text", content: segments.map((s) => s.content).join("") }];
  }

  return segments;
}

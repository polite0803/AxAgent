// SPDX-License-Identifier: AGPL-3.0-only

import { type BaseNode, getMarkdown, parseMarkdownToStructure } from "stream-markdown-parser";

export type ChatMarkdownNode = BaseNode;

export const CHAT_CUSTOM_HTML_TAGS = [
  "think",
  "web-search",
  "knowledge-retrieval",
  "memory-retrieval",
  "wiki-retrieval",
  "tool-call",
  "cron-result",
  "cite-ref",
  "viz-block",
] as const;

/**
 * 引用追溯：把 LLM 回复中的 `[cite:N]` token 替换为自定义标签
 * `<cite-ref n="N" data-axagent="1"></cite-ref>`，由 markstream-react 渲染为可点击 chip。
 * 必须在 `parseChatMarkdown` 之前调用，且仅在 assistant 内容上生效。
 */
export function injectCiteRefTags(content: string): string {
  return content.replace(/\[cite:(\d+)\]/g, '<cite-ref n="$1" data-axagent="1"></cite-ref>');
}

/**
 * Strip all axagent-injected custom tags (with `data-axagent="1"` attribute) and
 * MCP tool call fenced blocks (`:::mcp ... :::`) from content.
 * Used when copying message text so display-only tags don't pollute the clipboard.
 */
export function stripAxAgentTags(content: string): string {
  return content
    .replace(/<think[^>]*>[\s\S]*?<\/think>\s*/g, "")
    .replace(
      /<knowledge-retrieval [^>]*data-axagent="1"[^>]*>[\s\S]*?<\/knowledge-retrieval>\s*/g,
      "",
    )
    .replace(
      /<memory-retrieval [^>]*data-axagent="1"[^>]*>[\s\S]*?<\/memory-retrieval>\s*/g,
      "",
    )
    .replace(
      /<wiki-retrieval [^>]*data-axagent="1"[^>]*>[\s\S]*?<\/wiki-retrieval>\s*/g,
      "",
    )
    .replace(
      /<web-search [^>]*data-axagent="1"[^>]*>[\s\S]*?<\/web-search>\s*/g,
      "",
    )
    .replace(
      /<tool-call [^>]*data-axagent="1"[^>]*>[\s\S]*?<\/tool-call>\s*/g,
      "",
    )
    .replace(
      /<cron-result [^>]*data-axagent="1"[^>]*>[\s\S]*?<\/cron-result>\s*/g,
      "",
    )
    .replace(
      /<viz-block [^>]*data-axagent="1"[^>]*>[\s\S]*?<\/viz-block>\s*/g,
      "",
    )
    .replace(/<cite-ref [^>]*data-axagent="1"[^>]*>\s*<\/cite-ref>\s*/g, "")
    .replace(/\n*:::mcp [^\n]*\n[\s\S]*?:::\n*/g, "\n")
    .trim();
}

const chatMarkdown = getMarkdown("axagent-chat", {
  customHtmlTags: CHAT_CUSTOM_HTML_TAGS,
});

export function parseChatMarkdown(content: string): ChatMarkdownNode[] {
  return parseMarkdownToStructure(injectCiteRefTags(content), chatMarkdown, {
    customHtmlTags: [...CHAT_CUSTOM_HTML_TAGS],
  });
}

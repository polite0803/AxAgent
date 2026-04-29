import type { Message } from "@/types";

/**
 * Build the display content for an assistant message.
 * When structured blocks are available (Part-based model), extracts text blocks.
 * Otherwise falls back to the flat content string.
 */
export function buildAssistantDisplayContent(message: Message, _messages: Message[]): string {
  if (message.blocks && message.blocks.length > 0) {
    const textBlocks = message.blocks
      .filter((b) => b.type === "text")
      .map((b) => (b as { type: "text"; text: string }).text);
    if (textBlocks.length > 0) {
      return textBlocks.join("\n\n");
    }
  }
  return message.content;
}

/**
 * Determine whether the assistant bubble should be hidden.
 * A bubble is hidden when it has no text content but only tool calls.
 */
export function shouldHideAssistantBubble(message: Message, displayContent: string): boolean {
  if (message.role !== "assistant") {
    return false;
  }

  if (displayContent.trim()) {
    return false;
  }

  // With blocks: hide if there are only tool_use blocks (no text blocks)
  if (message.blocks && message.blocks.length > 0) {
    const hasText = message.blocks.some((b) => b.type === "text");
    if (hasText) return false;
    const hasToolUse = message.blocks.some((b) => b.type === "tool_use");
    return hasToolUse;
  }

  return !message.content.trim() && Boolean(message.tool_calls_json);
}

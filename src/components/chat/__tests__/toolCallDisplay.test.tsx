// SPDX-License-Identifier: AGPL-3.0-only

import { describe, expect, it } from "vitest";

import type { Message } from "@/types";
import { buildAssistantDisplayContent, shouldHideAssistantBubble } from "../toolCallDisplay";

function makeMessage(overrides: Partial<Message>): Message {
  return {
    id: "msg-1",
    conversationId: "conv-1",
    role: "assistant",
    content: "",
    providerId: null,
    modelId: null,
    tokenCount: null,
    attachments: [],
    thinking: null,
    toolCallsJson: null,
    toolCallId: null,
    createdAt: 1,
    parentMessageId: null,
    versionIndex: 0,
    isActive: true,
    status: "complete",
    ...overrides,
  };
}

describe("buildAssistantDisplayContent", () => {
  it("returns content as-is for assistant messages", () => {
    const msg = makeMessage({ content: "Hello world" });
    expect(buildAssistantDisplayContent(msg, [])).toBe("Hello world");
  });

  it("returns content with :::mcp containers unchanged", () => {
    const content = ':::mcp {"name":"@axagent/fetch","tool":"fetch_url"}\nresult\n:::\n';
    const msg = makeMessage({ content });
    expect(buildAssistantDisplayContent(msg, [])).toBe(content);
  });

  it("does not hide an empty normal assistant placeholder bubble while streaming", () => {
    const assistant = makeMessage({
      id: "assistant-streaming",
      content: "",
      toolCallsJson: null,
    });

    expect(shouldHideAssistantBubble(assistant, "")).toBe(false);
  });

  it("hides assistant bubble with empty content but present tool_calls_json", () => {
    const assistant = makeMessage({
      content: "",
      toolCallsJson: '[{"id":"call-1","type":"function","function":{"name":"fetch","arguments":"{}"}}]',
    });

    expect(shouldHideAssistantBubble(assistant, "")).toBe(true);
  });
});

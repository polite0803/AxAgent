import { describe, expect, it } from "vitest";

import type { Message } from "@/types";
import {
  buildJsonTranscript,
  buildMarkdownTranscript,
  buildTextTranscript,
} from "../exportChat";

const mockMessage = (overrides: Partial<Message> = {}): Message => ({
  id: "msg-1",
  conversation_id: "conv-1",
  role: "assistant",
  content: "Hello, how can I help you?",
  provider_id: null,
  model_id: null,
  token_count: null,
  attachments: [],
  thinking: null,
  tool_calls_json: null,
  tool_call_id: null,
  created_at: 1714500000000,
  parent_message_id: null,
  version_index: 0,
  is_active: true,
  status: "complete",
  ...overrides,
});

describe("buildMarkdownTranscript", () => {
  it("includes title as H1", () => {
    const result = buildMarkdownTranscript([], "My Chat");
    expect(result).toContain("# My Chat");
  });

  it("formats user messages as ## User", () => {
    const msg = mockMessage({ role: "user", content: "Hi" });
    const result = buildMarkdownTranscript([msg], "Chat");
    expect(result).toContain("## User");
    expect(result).toContain("Hi");
  });

  it("formats assistant messages as ## Assistant", () => {
    const msg = mockMessage({ role: "assistant", content: "Hello" });
    const result = buildMarkdownTranscript([msg], "Chat");
    expect(result).toContain("## Assistant");
    expect(result).toContain("Hello");
  });

  it("formats system messages as ## System", () => {
    const msg = mockMessage({ role: "system", content: "You are a helper" });
    const result = buildMarkdownTranscript([msg], "Chat");
    expect(result).toContain("## System");
  });

  it("includes proper separators between messages", () => {
    const msgs = [
      mockMessage({ role: "user", content: "Q1" }),
      mockMessage({ role: "assistant", content: "A1" }),
    ];
    const result = buildMarkdownTranscript(msgs, "Chat");
    expect(result).toContain("---");
  });
});

describe("buildTextTranscript", () => {
  it("includes title with underline", () => {
    const result = buildTextTranscript([], "My Chat");
    expect(result).toContain("My Chat");
    expect(result).toContain("=======");
  });

  it("formats user messages with [User] label", () => {
    const msg = mockMessage({ role: "user", content: "Hi" });
    const result = buildTextTranscript([msg], "Chat");
    expect(result).toContain("[User]");
    expect(result).toContain("Hi");
  });

  it("formats assistant messages with [Assistant] label", () => {
    const msg = mockMessage({ role: "assistant", content: "Hello" });
    const result = buildTextTranscript([msg], "Chat");
    expect(result).toContain("[Assistant]");
  });

  it("handles multiple messages in sequence", () => {
    const msgs = [
      mockMessage({ role: "user", content: "Q1", created_at: 1 }),
      mockMessage({ role: "assistant", content: "A1", created_at: 2 }),
      mockMessage({ role: "user", content: "Q2", created_at: 3 }),
    ];
    const result = buildTextTranscript(msgs, "History");
    expect(result).toContain("[User]");
    expect(result).toContain("[Assistant]");
    expect(result).toContain("Q1");
    expect(result).toContain("A1");
    expect(result).toContain("Q2");
  });
});

describe("buildJsonTranscript", () => {
  it("includes title and export timestamp", () => {
    const result = buildJsonTranscript([], "Chat");
    const parsed = JSON.parse(result);
    expect(parsed.title).toBe("Chat");
    expect(parsed.exported_at).toBeDefined();
  });

  it("serializes messages with role and content", () => {
    const msg = mockMessage({ role: "user", content: "Hello" });
    const result = buildJsonTranscript([msg], "Chat");
    const parsed = JSON.parse(result);
    expect(parsed.messages).toHaveLength(1);
    expect(parsed.messages[0].role).toBe("user");
    expect(parsed.messages[0].content).toBe("Hello");
  });

  it("excludes thinking when includeThinking is false", () => {
    const msg = mockMessage({
      role: "assistant",
      content: "Result",
      thinking: "Let me think about this...",
    });
    const result = buildJsonTranscript([msg], "Chat", { includeThinking: false });
    const parsed = JSON.parse(result);
    expect(parsed.messages[0].thinking).toBeUndefined();
  });

  it("includes thinking by default", () => {
    const msg = mockMessage({
      role: "assistant",
      content: "Result",
      thinking: "Let me think about this...",
    });
    const result = buildJsonTranscript([msg], "Chat");
    const parsed = JSON.parse(result);
    expect(parsed.messages[0].thinking).toBe("Let me think about this...");
  });

  it("includes created_at timestamp", () => {
    const msg = mockMessage({ created_at: 1714500000000 });
    const result = buildJsonTranscript([msg], "Chat");
    const parsed = JSON.parse(result);
    expect(parsed.messages[0].created_at).toBe(1714500000000);
  });
});

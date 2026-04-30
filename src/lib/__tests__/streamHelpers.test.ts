import { describe, expect, it } from "vitest";

import type { Message } from "@/types";
import {
  createOptimisticUserMessage,
  createPlaceholderAssistant,
  replaceActiveVersion,
} from "../streamHelpers";

describe("createOptimisticUserMessage", () => {
  it("creates a user message with temp- prefix", () => {
    const msg = createOptimisticUserMessage("conv-1", "Hello");
    expect(msg.id).toMatch(/^temp-user-/);
    expect(msg.conversation_id).toBe("conv-1");
    expect(msg.role).toBe("user");
    expect(msg.content).toBe("Hello");
    expect(msg.status).toBe("complete");
  });

  it("includes attachments when provided", () => {
    const msg = createOptimisticUserMessage("conv-1", "Check this file", [
      { file_name: "test.ts", file_type: "text/typescript", file_size: 1024 },
    ]);
    expect(msg.attachments).toHaveLength(1);
    expect(msg.attachments[0].file_name).toBe("test.ts");
    expect(msg.attachments[0].file_type).toBe("text/typescript");
    expect(msg.attachments[0].file_size).toBe(1024);
  });

  it("handles empty attachments array", () => {
    const msg = createOptimisticUserMessage("conv-1", "Hello", []);
    expect(msg.attachments).toHaveLength(0);
  });

  it("sets default attachment properties", () => {
    const msg = createOptimisticUserMessage("conv-1", "", [
      { file_name: "img.png", file_type: "image/png" },
    ]);
    expect(msg.attachments[0].file_size).toBe(0);
    expect(msg.attachments[0].file_path).toBe("");
  });
});

describe("createPlaceholderAssistant", () => {
  it("creates assistant placeholder with temp- prefix", () => {
    const msg = createPlaceholderAssistant("conv-1", "parent-1");
    expect(msg.id).toMatch(/^temp-assistant-/);
    expect(msg.role).toBe("assistant");
    expect(msg.parent_message_id).toBe("parent-1");
    expect(msg.status).toBe("partial");
  });

  it("sets initial content to empty string by default", () => {
    const msg = createPlaceholderAssistant("conv-1", "parent-1");
    expect(msg.content).toBe("");
  });

  it("accepts optional initial content", () => {
    const msg = createPlaceholderAssistant("conv-1", "parent-1", "Thinking...");
    expect(msg.content).toBe("Thinking...");
  });

  it("accepts optional provider and model IDs", () => {
    const msg = createPlaceholderAssistant("conv-1", "parent-1", "", "p-openai", "gpt-4");
    expect(msg.provider_id).toBe("p-openai");
    expect(msg.model_id).toBe("gpt-4");
  });
});

describe("replaceActiveVersion", () => {
  const placeholder: Message = {
    id: "new-version-1",
    conversation_id: "conv-1",
    role: "assistant",
    content: "",
    provider_id: null,
    model_id: null,
    token_count: null,
    attachments: [],
    thinking: null,
    tool_calls_json: null,
    tool_call_id: null,
    created_at: Date.now(),
    parent_message_id: "parent-1",
    version_index: 1,
    is_active: true,
    status: "partial",
  };

  it("deactivates old active message and inserts new placeholder", () => {
    const messages: Message[] = [
      {
        ...placeholder,
        id: "old-version-1",
        version_index: 0,
        is_active: true,
        content: "Old response",
        status: "complete",
      },
    ];

    const result = replaceActiveVersion(messages, "parent-1", placeholder);
    expect(result.inserted).toBe(true);
    expect(result.updatedMessages).toHaveLength(2);
    // Old message should be deactivated
    const oldMsg = result.updatedMessages.find((m) => m.id === "old-version-1");
    expect(oldMsg!.is_active).toBe(false);
    // New placeholder should be present
    const newMsg = result.updatedMessages.find((m) => m.id === "new-version-1");
    expect(newMsg).toBeDefined();
    expect(newMsg!.is_active).toBe(true);
  });

  it("inserts placeholder even when no active message exists for parent", () => {
    const messages: Message[] = [
      {
        ...placeholder,
        id: "other-msg",
        parent_message_id: "other-parent",
        is_active: true,
      },
    ];

    const result = replaceActiveVersion(messages, "parent-1", placeholder);
    expect(result.inserted).toBe(false);
    // Should still append the placeholder
    expect(result.updatedMessages).toHaveLength(2);
    expect(result.updatedMessages[1].id).toBe("new-version-1");
  });

  it("handles empty messages array", () => {
    const result = replaceActiveVersion([], "parent-1", placeholder);
    expect(result.inserted).toBe(false);
    expect(result.updatedMessages).toHaveLength(1);
    expect(result.updatedMessages[0].id).toBe("new-version-1");
  });

  it("deactivates only messages matching parent_id", () => {
    const messages: Message[] = [
      {
        ...placeholder,
        id: "msg-other",
        parent_message_id: "other-parent",
        is_active: true,
        content: "Keep me active",
        status: "complete",
      },
      {
        ...placeholder,
        id: "msg-target",
        parent_message_id: "parent-1",
        is_active: true,
        content: "Deactivate me",
        status: "complete",
      },
    ];

    const result = replaceActiveVersion(messages, "parent-1", placeholder);
    const other = result.updatedMessages.find((m) => m.id === "msg-other");
    expect(other!.is_active).toBe(true); // Should remain active
    const target = result.updatedMessages.find((m) => m.id === "msg-target");
    expect(target!.is_active).toBe(false); // Should be deactivated
  });
});

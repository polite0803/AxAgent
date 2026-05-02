import { describe, expect, it } from "vitest";

import type { Message } from "@/types";
import { hasMultipleModelVersions, shouldRenderStandaloneAssistantError } from "../chatMultiModel";

// 最小化的 Message mock 工厂函数
function msg(overrides: Partial<Message> = {}): Message {
  return {
    id: "msg-1",
    conversation_id: "conv-1",
    role: "user",
    content: "hello",
    provider_id: null,
    model_id: null,
    token_count: null,
    attachments: [],
    thinking: null,
    tool_calls_json: null,
    tool_call_id: null,
    created_at: 1700000000000,
    parent_message_id: null,
    version_index: 0,
    is_active: true,
    status: "sent",
    ...overrides,
  } as Message;
}

describe("hasMultipleModelVersions", () => {
  it("空数组应返回 false", () => {
    expect(hasMultipleModelVersions([])).toBe(false);
  });

  it("单个消息应返回 false", () => {
    expect(
      hasMultipleModelVersions([msg({ model_id: "gpt-4" })]),
    ).toBe(false);
  });

  it("多个消息但 model_id 都相同时应返回 false", () => {
    expect(
      hasMultipleModelVersions([
        msg({ model_id: "gpt-4" }),
        msg({ model_id: "gpt-4" }),
        msg({ model_id: "gpt-4" }),
      ]),
    ).toBe(false);
  });

  it("多个消息有不同 model_id 时应返回 true", () => {
    expect(
      hasMultipleModelVersions([
        msg({ model_id: "gpt-4" }),
        msg({ model_id: "claude-3" }),
      ]),
    ).toBe(true);
  });

  it("包含 null model_id 的消息不应影响判断", () => {
    // 只有一个有效 model_id，不算多模型
    expect(
      hasMultipleModelVersions([
        msg({ model_id: null }),
        msg({ model_id: "gpt-4" }),
        msg({ model_id: null }),
      ]),
    ).toBe(false);
  });

  it("全部 model_id 为 null 时应返回 false", () => {
    expect(
      hasMultipleModelVersions([
        msg({ model_id: null }),
        msg({ model_id: null }),
      ]),
    ).toBe(false);
  });

  it("三个不同模型应返回 true", () => {
    expect(
      hasMultipleModelVersions([
        msg({ model_id: "gpt-4" }),
        msg({ model_id: "claude-3" }),
        msg({ model_id: "gemini-pro" }),
      ]),
    ).toBe(true);
  });
});

describe("shouldRenderStandaloneAssistantError", () => {
  it("assistant 且 status=error 时应返回 true", () => {
    expect(
      shouldRenderStandaloneAssistantError(
        msg({ role: "assistant", status: "error" }),
      ),
    ).toBe(true);
  });

  it("assistant 但 status 非 error 时应返回 false", () => {
    expect(
      shouldRenderStandaloneAssistantError(
        msg({ role: "assistant", status: "complete" }),
      ),
    ).toBe(false);
  });

  it("user 角色即使 status=error 也应返回 false", () => {
    expect(
      shouldRenderStandaloneAssistantError(
        msg({ role: "user", status: "error" }),
      ),
    ).toBe(false);
  });

  it("system 角色 status=error 时应返回 false", () => {
    expect(
      shouldRenderStandaloneAssistantError(
        msg({ role: "system", status: "error" }),
      ),
    ).toBe(false);
  });

  it("assistant 且 status=streaming 时应返回 false", () => {
    expect(
      shouldRenderStandaloneAssistantError(
        msg({ role: "assistant", status: "partial" }),
      ),
    ).toBe(false);
  });
});

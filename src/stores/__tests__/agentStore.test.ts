// SPDX-License-Identifier: AGPL-3.0-only

import { beforeEach, describe, expect, it, vi } from "vitest";

import { listen } from "@/lib/invoke";
import { setupAgentEventListeners, useAgentStore } from "@/stores";
import { _injectPreferenceStore } from "@/stores/domain/conversationStore";
import { usePreferenceStore } from "@/stores/domain/preferenceStore";

vi.mock("@/lib/invoke", () => ({
  invoke: vi.fn(),
  listen: vi.fn(() => Promise.resolve(() => {})),
  isTauri: () => false,
}));

vi.mock("@/lib/toast", () => ({
  message: {
    success: vi.fn(),
    error: vi.fn(),
    info: vi.fn(),
    warning: vi.fn(),
    loading: vi.fn(),
    open: vi.fn(),
    destroy: vi.fn(),
  },
}));

vi.mock("@/components/layout/NotificationBell", () => ({
  pushNotification: vi.fn(),
}));

// 注入 preferenceStore 引用（Vitest 模块加载器下循环依赖不会自动触发注入）
_injectPreferenceStore(usePreferenceStore);

// Zustand store 不依赖 React，直接通过 getState() 调用 actions，避免
// renderHook 引入 react-dom → scheduler setImmediate → jsdom 销毁后抛 ReferenceError

describe("agentStore event handling", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    // 重置 store 状态
    const store = useAgentStore.getState();
    for (const convId of Object.keys(store.agentStatus)) {
      store.clearStatus(convId);
    }
  });

  it("should handle tool use event (sets isExecuting)", () => {
    const store = useAgentStore.getState();

    const toolUseEvent = {
      conversationId: "conv1",
      assistantMessageId: "msg1",
      toolUseId: "tool1",
      toolName: "echo",
      input: { text: "Hello" },
      executionId: "exec1",
    };

    store.handleToolUse(toolUseEvent);

    expect(useAgentStore.getState().isExecuting["conv1"]).toBe(true);
    expect(useAgentStore.getState().executingConversationIds).toContain("conv1");
  });

  it("should handle tool start event (no-op, does not throw)", () => {
    const store = useAgentStore.getState();

    store.handleToolUse({
      conversationId: "conv1",
      assistantMessageId: "msg1",
      toolUseId: "tool1",
      toolName: "echo",
      input: { text: "Hello" },
    });

    // agentStore.handleToolStart 是 no-op（委托给 executionStore）
    expect(() => {
      store.handleToolStart({
        conversationId: "conv1",
        assistantMessageId: "msg1",
        toolUseId: "tool1",
        toolName: "echo",
        input: { text: "Hello" },
      });
    }).not.toThrow();
  });

  it("should handle tool result event (clears isExecuting)", () => {
    const store = useAgentStore.getState();

    store.handleToolUse({
      conversationId: "conv1",
      assistantMessageId: "msg1",
      toolUseId: "tool1",
      toolName: "echo",
      input: { text: "Hello" },
    });

    expect(useAgentStore.getState().isExecuting["conv1"]).toBe(true);

    store.handleToolResult({
      conversationId: "conv1",
      assistantMessageId: "msg1",
      toolUseId: "tool1",
      toolName: "echo",
      content: "Hello",
      isError: false,
    });

    expect(useAgentStore.getState().isExecuting["conv1"]).toBeUndefined();
  });

  it("should handle permission request event", () => {
    const store = useAgentStore.getState();

    const permissionEvent = {
      conversationId: "conv1",
      assistantMessageId: "msg1",
      toolUseId: "tool1",
      toolName: "write",
      input: { path: "test.txt", content: "Hello" },
      riskLevel: "write" as const,
      requestId: "perm_1",
    };

    store.handlePermissionRequest(permissionEvent);

    // 用 requestId 作为 key
    expect(useAgentStore.getState().pendingPermissions["perm_1"]).toEqual(
      permissionEvent,
    );
  });

  it("should handle permission resolved", () => {
    const store = useAgentStore.getState();

    store.handlePermissionRequest({
      conversationId: "conv1",
      assistantMessageId: "msg1",
      toolUseId: "tool1",
      toolName: "write",
      input: { path: "test.txt", content: "Hello" },
      requestId: "req1",
      riskLevel: "write" as const,
    });

    expect(useAgentStore.getState().pendingPermissions["req1"]).toBeDefined();

    // handlePermissionResolved 按 toolUseId 清除 pendingPermissions 条目
    // （注意：实际 key 为 requestId 时此清除可能不生效——这是已知的工单问题）
    store.handlePermissionResolved("req1", "allow_once");

    expect(useAgentStore.getState().pendingPermissions["req1"]).toBeUndefined();
  });

  it("should handle done event and record queryStats", () => {
    const store = useAgentStore.getState();

    store.handleDone({
      conversationId: "conv1",
      assistantMessageId: "msg1",
      text: "Hello World!",
      usage: { input_tokens: 10, output_tokens: 5 },
      numTurns: 1,
    });

    expect(useAgentStore.getState().queryStats["msg1"]).toEqual({
      numTurns: 1,
      inputTokens: 10,
      outputTokens: 5,
    });
  });

  it("should handle done event with cost", () => {
    const store = useAgentStore.getState();

    store.handleDone({
      conversationId: "conv1",
      assistantMessageId: "msg1",
      text: "Hello World!",
      usage: { input_tokens: 100, output_tokens: 50 },
      numTurns: 3,
      costUsd: 0.005,
    });

    expect(useAgentStore.getState().queryStats["msg1"]).toEqual({
      numTurns: 3,
      inputTokens: 100,
      outputTokens: 50,
      costUsd: 0.005,
    });
  });

  it("should handle cancelled event", () => {
    const store = useAgentStore.getState();

    // agentStatus 由 executionStore 管理，agentStore.handleStatus 是 no-op
    store.handleCancelled({ conversationId: "conv1", reason: "User cancelled" });

    expect(useAgentStore.getState().isExecuting["conv1"]).toBeUndefined();
  });

  it("should handle rate limit event", () => {
    const store = useAgentStore.getState();

    const rateLimitEvent = {
      conversationId: "conv1",
      retryAfterMs: 5000,
      message: "Rate limited, retry in 5s",
    };

    store.handleRateLimit(rateLimitEvent);

    expect(useAgentStore.getState().rateLimitInfo["conv1"]).toEqual(rateLimitEvent);
  });

  it("should clear conversation state", () => {
    const store = useAgentStore.getState();

    store.handlePermissionRequest({
      conversationId: "conv1",
      assistantMessageId: "msg1",
      toolUseId: "tool1",
      toolName: "write",
      input: { path: "test.txt" },
      requestId: "req2",
      riskLevel: "write" as const,
    });

    expect(Object.keys(useAgentStore.getState().pendingPermissions).length).toBeGreaterThan(0);
    expect(useAgentStore.getState().isExecuting["conv1"]).toBeUndefined();

    store.clearConversation("conv1");

    expect(useAgentStore.getState().isExecuting["conv1"]).toBeUndefined();
    expect(useAgentStore.getState().pendingPermissions["req2"]).toBeUndefined();
  });

  it("should setup event listeners", () => {
    const unlistenFn = vi.fn();
    // listen 已由 vi.mock 提供为 vi.fn()
    vi.mocked(listen).mockResolvedValue(unlistenFn);

    const cleanup = setupAgentEventListeners();

    expect(vi.mocked(listen)).toHaveBeenCalledTimes(26);

    cleanup();
  });
});

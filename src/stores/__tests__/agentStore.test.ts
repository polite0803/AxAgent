import { listen } from "@/lib/invoke";
import { setupAgentEventListeners, useAgentStore } from "@/stores";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("@/lib/invoke", () => ({
  invoke: vi.fn(),
  listen: vi.fn(() => Promise.resolve(() => {})),
  isTauri: () => false,
}));

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

  it("should handle tool use event", () => {
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

    expect(useAgentStore.getState().toolCalls["tool1"]).toEqual({
      toolUseId: "tool1",
      toolName: "echo",
      input: { text: "Hello" },
      assistantMessageId: "msg1",
      executionStatus: "queued",
    });

    expect(useAgentStore.getState().toolCalls["exec1"]).toEqual({
      toolUseId: "exec1",
      toolName: "echo",
      input: { text: "Hello" },
      assistantMessageId: "msg1",
      executionStatus: "queued",
    });

    expect(useAgentStore.getState().sdkIdToExecId["tool1"]).toBe("exec1");
  });

  it("should handle tool start event", () => {
    const store = useAgentStore.getState();

    store.handleToolUse({
      conversationId: "conv1",
      assistantMessageId: "msg1",
      toolUseId: "tool1",
      toolName: "echo",
      input: { text: "Hello" },
    });

    store.handleToolStart({
      conversationId: "conv1",
      assistantMessageId: "msg1",
      toolUseId: "tool1",
      toolName: "echo",
      input: { text: "Hello" },
    });

    expect(useAgentStore.getState().toolCalls["tool1"].executionStatus).toBe("running");
  });

  it("should handle tool result event", () => {
    const store = useAgentStore.getState();

    store.handleToolUse({
      conversationId: "conv1",
      assistantMessageId: "msg1",
      toolUseId: "tool1",
      toolName: "echo",
      input: { text: "Hello" },
    });

    store.handleToolResult({
      conversationId: "conv1",
      assistantMessageId: "msg1",
      toolUseId: "tool1",
      toolName: "echo",
      content: "Hello",
      isError: false,
    });

    expect(useAgentStore.getState().toolCalls["tool1"].executionStatus).toBe("success");
    expect(useAgentStore.getState().toolCalls["tool1"].output).toBe("Hello");
    expect(useAgentStore.getState().toolCalls["tool1"].isError).toBe(false);
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

    store.handleToolUse({
      conversationId: "conv1",
      assistantMessageId: "msg1",
      toolUseId: "tool1",
      toolName: "write",
      input: { path: "test.txt", content: "Hello" },
    });

    store.handlePermissionResolved("tool1", "allow_once");

    expect(useAgentStore.getState().pendingPermissions["tool1"]).toBeUndefined();
    expect(useAgentStore.getState().toolCalls["tool1"].approvalStatus).toBe("approved");
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

    store.handleStatus("conv1", "Running tool...");
    expect(useAgentStore.getState().agentStatus["conv1"]).toBe("Running tool...");

    store.handleCancelled({ conversationId: "conv1", reason: "User cancelled" });

    expect(useAgentStore.getState().agentStatus["conv1"]).toBeUndefined();
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

    store.handleStatus("conv1", "Running...");
    store.handlePermissionRequest({
      conversationId: "conv1",
      assistantMessageId: "msg1",
      toolUseId: "tool1",
      toolName: "write",
      input: { path: "test.txt" },
      requestId: "req2",
      riskLevel: "write" as const,
    });

    expect(useAgentStore.getState().agentStatus["conv1"]).toBe("Running...");
    expect(Object.keys(useAgentStore.getState().pendingPermissions).length).toBeGreaterThan(0);

    store.clearConversation("conv1");

    expect(useAgentStore.getState().agentStatus["conv1"]).toBeUndefined();
    expect(Object.keys(useAgentStore.getState().pendingPermissions).length).toBe(0);
  });

  it("should setup event listeners", () => {
    const unlistenFn = vi.fn();
    (listen as unknown as ReturnType<typeof vi.fn>).mockReturnValue(
      Promise.resolve(unlistenFn),
    );

    const cleanup = setupAgentEventListeners();

    expect(listen).toHaveBeenCalledTimes(24);

    cleanup();
  });
});

// SPDX-License-Identifier: AGPL-3.0-only

import { beforeEach, describe, expect, it, vi } from "vitest";

const { invokeMock, listenMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
  listenMock: vi.fn(() => Promise.resolve(() => {})),
}));

vi.mock("@/lib/invoke", () => ({
  invoke: invokeMock,
  listen: listenMock,
  isTauri: () => false,
  logIpcError: vi.fn(() => vi.fn()),
}));

import { useExecutionStore } from "@/stores/feature/executionStore";

describe("executionStore", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useExecutionStore.setState({
      phases: {},
      currentToolCall: null,
      agentStatus: {},
      toolCalls: {},
      sdkIdToExecId: {},
      agentPool: {},
      trajectoriesByConversation: {},
      trajectoryDetails: {},
      loadingTrajectories: false,
      loadingTrajectoryDetail: {},
    });
  });

  describe("phase machine (transition)", () => {
    it("transitions from idle to planning", () => {
      const store = useExecutionStore.getState();
      store.transition("conv-1", "planning");
      expect(useExecutionStore.getState().phases["conv-1"]).toBe("planning");
    });

    it("transitions from idle to executing", () => {
      const store = useExecutionStore.getState();
      store.transition("conv-1", "executing");
      expect(useExecutionStore.getState().phases["conv-1"]).toBe("executing");
    });

    it("does not transition to an invalid phase (waiting_permission from idle)", () => {
      const store = useExecutionStore.getState();
      store.transition("conv-1", "waiting_permission");
      expect(useExecutionStore.getState().phases["conv-1"]).toBeUndefined();
    });

    it("self-transition is a no-op (executing → executing)", () => {
      const store = useExecutionStore.getState();
      store.transition("conv-1", "executing");
      store.transition("conv-1", "executing");
      expect(useExecutionStore.getState().phases["conv-1"]).toBe("executing");
    });

    it("transitions from planning to executing", () => {
      const store = useExecutionStore.getState();
      store.transition("conv-1", "planning");
      store.transition("conv-1", "executing");
      expect(useExecutionStore.getState().phases["conv-1"]).toBe("executing");
    });

    it("transitions from executing to completed", () => {
      const store = useExecutionStore.getState();
      store.transition("conv-1", "executing");
      store.transition("conv-1", "completed");
      expect(useExecutionStore.getState().phases["conv-1"]).toBe("completed");
    });
  });

  describe("isActive", () => {
    it("returns true for active phases", () => {
      const store = useExecutionStore.getState();
      store.transition("conv-1", "planning");
      expect(useExecutionStore.getState().isActive("conv-1")).toBe(true);

      useExecutionStore.setState({ phases: {} });
      store.transition("conv-1", "executing");
      expect(useExecutionStore.getState().isActive("conv-1")).toBe(true);

      useExecutionStore.setState({ phases: {} });
      store.transition("conv-1", "planning");
      store.transition("conv-1", "waiting_permission");
      expect(useExecutionStore.getState().isActive("conv-1")).toBe(true);
    });

    it("returns false for terminal phases", () => {
      const store = useExecutionStore.getState();
      store.transition("conv-1", "completed");
      expect(useExecutionStore.getState().isActive("conv-1")).toBe(false);
    });

    it("returns false for unregistered conversation", () => {
      expect(useExecutionStore.getState().isActive("conv-unknown")).toBe(false);
    });
  });

  describe("isTerminal", () => {
    it("returns true for terminal phases", () => {
      const store = useExecutionStore.getState();
      store.transition("conv-1", "completed");
      expect(useExecutionStore.getState().isTerminal("conv-1")).toBe(true);
    });

    it("returns false for active phases", () => {
      const store = useExecutionStore.getState();
      store.transition("conv-1", "executing");
      expect(useExecutionStore.getState().isTerminal("conv-1")).toBe(false);
    });

    it("returns false for unregistered conversation", () => {
      expect(useExecutionStore.getState().isTerminal("conv-unknown")).toBe(false);
    });
  });

  describe("getActiveConversations", () => {
    it("returns all conversations currently in active phases", () => {
      const store = useExecutionStore.getState();
      store.transition("conv-1", "executing");
      store.transition("conv-2", "planning");
      store.transition("conv-3", "completed");

      const active = useExecutionStore.getState().getActiveConversations();
      expect(active).toContain("conv-1");
      expect(active).toContain("conv-2");
      expect(active).not.toContain("conv-3");
    });

    it("returns empty array when no active conversations", () => {
      expect(useExecutionStore.getState().getActiveConversations()).toEqual([]);
    });
  });

  describe("agent status", () => {
    it("setAgentStatus sets status for a conversation", () => {
      useExecutionStore.getState().setAgentStatus("conv-1", "Processing...");
      expect(useExecutionStore.getState().agentStatus["conv-1"]).toBe("Processing...");
    });

    it("clearAgentStatus removes status for a conversation", () => {
      useExecutionStore.getState().setAgentStatus("conv-1", "Processing...");
      useExecutionStore.getState().clearAgentStatus("conv-1");
      expect(useExecutionStore.getState().agentStatus["conv-1"]).toBeUndefined();
    });

    it("clearAgentStatus does not affect other conversations", () => {
      useExecutionStore.getState().setAgentStatus("conv-1", "A");
      useExecutionStore.getState().setAgentStatus("conv-2", "B");
      useExecutionStore.getState().clearAgentStatus("conv-1");
      expect(useExecutionStore.getState().agentStatus["conv-1"]).toBeUndefined();
      expect(useExecutionStore.getState().agentStatus["conv-2"]).toBe("B");
    });
  });

  describe("currentToolCall", () => {
    it("setCurrentTool sets the current tool call", () => {
      const tc = {
        toolName: "echo",
        toolUseId: "tool-1",
        conversationId: "conv-1",
        startedAt: Date.now(),
      };
      useExecutionStore.getState().setCurrentTool(tc);
      expect(useExecutionStore.getState().currentToolCall).toEqual(tc);
    });

    it("setCurrentTool clears the current tool call when null", () => {
      useExecutionStore.getState().setCurrentTool({
        toolName: "echo",
        toolUseId: "tool-1",
        conversationId: "conv-1",
        startedAt: Date.now(),
      });
      useExecutionStore.getState().setCurrentTool(null);
      expect(useExecutionStore.getState().currentToolCall).toBeNull();
    });
  });

  describe("agent pool", () => {
    it("upsertPoolItem adds a new item", () => {
      useExecutionStore.getState().upsertPoolItem({
        id: "item-1",
        conversationId: "conv-1",
        type: "worker",
        name: "Worker-1",
        status: "running",
        startedAt: Date.now(),
      });
      const pool = useExecutionStore.getState().agentPool["conv-1"];
      expect(pool).toHaveLength(1);
      expect(pool[0].id).toBe("item-1");
    });

    it("upsertPoolItem updates an existing item", () => {
      useExecutionStore.getState().upsertPoolItem({
        id: "item-1",
        conversationId: "conv-1",
        type: "worker",
        name: "Worker-1",
        status: "running",
        startedAt: Date.now(),
      });
      useExecutionStore.getState().upsertPoolItem({
        id: "item-1",
        conversationId: "conv-1",
        type: "worker",
        name: "Worker-1",
        status: "completed",
        summary: "Done",
      });
      const pool = useExecutionStore.getState().agentPool["conv-1"];
      expect(pool).toHaveLength(1);
      expect(pool[0].status).toBe("completed");
      expect(pool[0].summary).toBe("Done");
    });

    it("removePoolItem removes an item", () => {
      useExecutionStore.getState().upsertPoolItem({
        id: "item-1",
        conversationId: "conv-1",
        type: "worker",
        name: "Worker-1",
        status: "running",
        startedAt: Date.now(),
      });
      useExecutionStore.getState().removePoolItem("conv-1", "item-1");
      expect(useExecutionStore.getState().agentPool["conv-1"]).toHaveLength(0);
    });

    it("removePoolItem is a no-op for non-existent item", () => {
      useExecutionStore.getState().removePoolItem("conv-1", "nonexistent");
      expect(useExecutionStore.getState().agentPool["conv-1"]).toBeUndefined();
    });
  });

  describe("getPoolSummary", () => {
    it("returns zero summary for empty pool", () => {
      const summary = useExecutionStore.getState().getPoolSummary("conv-1");
      expect(summary).toEqual({
        total: 0,
        completed: 0,
        running: 0,
        pending: 0,
        failed: 0,
        pctComplete: 0,
      });
    });

    it("returns correct counts for mixed pool", () => {
      const items = [
        {
          id: "1",
          conversationId: "conv-1",
          type: "worker" as const,
          name: "W1",
          status: "completed" as const,
          startedAt: Date.now(),
        },
        {
          id: "2",
          conversationId: "conv-1",
          type: "worker" as const,
          name: "W2",
          status: "completed" as const,
          startedAt: Date.now(),
        },
        {
          id: "3",
          conversationId: "conv-1",
          type: "worker" as const,
          name: "W3",
          status: "running" as const,
          startedAt: Date.now(),
        },
        {
          id: "4",
          conversationId: "conv-1",
          type: "worker" as const,
          name: "W4",
          status: "failed" as const,
          startedAt: Date.now(),
        },
      ];
      for (const item of items) {
        useExecutionStore.getState().upsertPoolItem(item);
      }
      const summary = useExecutionStore.getState().getPoolSummary("conv-1");
      expect(summary.total).toBe(4);
      expect(summary.completed).toBe(2);
      expect(summary.running).toBe(1);
      expect(summary.failed).toBe(1);
      expect(summary.pctComplete).toBe(50);
    });
  });

  describe("handleToolUse", () => {
    it("creates tool call state and transitions to executing", () => {
      const store = useExecutionStore.getState();
      store.handleToolUse({
        conversationId: "conv-1",
        assistantMessageId: "msg-1",
        toolUseId: "tool-1",
        toolName: "echo",
        input: { text: "hello" },
        executionId: "exec-1",
      });

      const state = useExecutionStore.getState();
      expect(state.toolCalls["tool-1"]).toBeDefined();
      expect(state.toolCalls["tool-1"].toolName).toBe("echo");
      expect(state.toolCalls["tool-1"].executionStatus).toBe("queued");
      expect(state.currentToolCall?.toolUseId).toBe("tool-1");
      expect(state.phases["conv-1"]).toBe("executing");
    });

    it("maps sdk id to execution id", () => {
      useExecutionStore.getState().handleToolUse({
        conversationId: "conv-1",
        assistantMessageId: "msg-1",
        toolUseId: "tool-1",
        toolName: "echo",
        input: { text: "hello" },
        executionId: "exec-1",
      });
      expect(useExecutionStore.getState().sdkIdToExecId["tool-1"]).toBe("exec-1");
    });

    it("does not re-transition if already in executing phase", () => {
      useExecutionStore.getState().transition("conv-1", "executing");
      useExecutionStore.getState().handleToolUse({
        conversationId: "conv-1",
        assistantMessageId: "msg-1",
        toolUseId: "tool-2",
        toolName: "bash",
        input: {},
      });
      expect(useExecutionStore.getState().phases["conv-1"]).toBe("executing");
    });
  });

  describe("handleToolStart", () => {
    it("updates tool call status to running and adds to pool", () => {
      useExecutionStore.getState().handleToolUse({
        conversationId: "conv-1",
        assistantMessageId: "msg-1",
        toolUseId: "tool-1",
        toolName: "echo",
        input: { text: "hello" },
      });

      useExecutionStore.getState().handleToolStart({
        conversationId: "conv-1",
        assistantMessageId: "msg-1",
        toolUseId: "tool-1",
        toolName: "echo",
        input: { text: "hello" },
      });

      const state = useExecutionStore.getState();
      expect(state.toolCalls["tool-1"].executionStatus).toBe("running");
      expect(state.toolCalls["tool-1"].startedAt).toBeDefined();
      expect(state.agentPool["conv-1"]).toBeDefined();
      expect(state.agentPool["conv-1"][0].status).toBe("running");
    });
  });

  describe("handleToolResult", () => {
    it("updates tool call to success status", () => {
      useExecutionStore.getState().handleToolUse({
        conversationId: "conv-1",
        assistantMessageId: "msg-1",
        toolUseId: "tool-1",
        toolName: "echo",
        input: { text: "hello" },
      });
      useExecutionStore.getState().handleToolStart({
        conversationId: "conv-1",
        assistantMessageId: "msg-1",
        toolUseId: "tool-1",
        toolName: "echo",
        input: { text: "hello" },
      });

      useExecutionStore.getState().handleToolResult({
        conversationId: "conv-1",
        assistantMessageId: "msg-1",
        toolUseId: "tool-1",
        toolName: "echo",
        content: "hello world",
        isError: false,
      });

      const state = useExecutionStore.getState();
      expect(state.toolCalls["tool-1"].executionStatus).toBe("success");
      expect(state.toolCalls["tool-1"].output).toBe("hello world");
      expect(state.toolCalls["tool-1"].isError).toBe(false);
    });

    it("handles error tool results", () => {
      useExecutionStore.getState().handleToolUse({
        conversationId: "conv-1",
        assistantMessageId: "msg-1",
        toolUseId: "tool-1",
        toolName: "bash",
        input: { command: "rm -rf /" },
      });

      useExecutionStore.getState().handleToolResult({
        conversationId: "conv-1",
        assistantMessageId: "msg-1",
        toolUseId: "tool-1",
        toolName: "bash",
        content: "Permission denied",
        isError: true,
      });

      const state = useExecutionStore.getState();
      expect(state.toolCalls["tool-1"].executionStatus).toBe("failed");
      expect(state.toolCalls["tool-1"].isError).toBe(true);
    });

    it("clears currentToolCall when the result matches the current tool", () => {
      useExecutionStore.getState().handleToolUse({
        conversationId: "conv-1",
        assistantMessageId: "msg-1",
        toolUseId: "tool-1",
        toolName: "echo",
        input: { text: "hello" },
      });
      expect(useExecutionStore.getState().currentToolCall?.toolUseId).toBe("tool-1");

      useExecutionStore.getState().handleToolResult({
        conversationId: "conv-1",
        assistantMessageId: "msg-1",
        toolUseId: "tool-1",
        toolName: "echo",
        content: "hello",
        isError: false,
      });

      expect(useExecutionStore.getState().currentToolCall).toBeNull();
    });
  });

  describe("handleWorkerEvent", () => {
    it("creates a new pool item for a new worker", () => {
      useExecutionStore.getState().handleWorkerEvent({
        conversationId: "conv-1",
        workerId: "worker-1",
        taskId: "task-1",
        messageType: "progress",
        content: "Working...",
      });

      const pool = useExecutionStore.getState().agentPool["conv-1"];
      expect(pool).toHaveLength(1);
      expect(pool[0].id).toBe("worker-worker-1");
      expect(pool[0].status).toBe("running");
      expect(pool[0].messages).toHaveLength(1);
    });

    it("updates existing worker item on completion", () => {
      useExecutionStore.getState().handleWorkerEvent({
        conversationId: "conv-1",
        workerId: "worker-1",
        taskId: "task-1",
        messageType: "progress",
        content: "Working...",
      });
      useExecutionStore.getState().handleWorkerEvent({
        conversationId: "conv-1",
        workerId: "worker-1",
        taskId: "task-1",
        messageType: "completion",
        content: "Done!",
        status: "completed",
      });

      const pool = useExecutionStore.getState().agentPool["conv-1"];
      expect(pool[0].status).toBe("completed");
      expect(pool[0].messages).toHaveLength(2);
    });

    it("sets failed status on error worker event", () => {
      useExecutionStore.getState().handleWorkerEvent({
        conversationId: "conv-1",
        workerId: "worker-1",
        taskId: "task-1",
        messageType: "progress",
        content: "Working...",
      });
      useExecutionStore.getState().handleWorkerEvent({
        conversationId: "conv-1",
        workerId: "worker-1",
        taskId: "task-1",
        messageType: "error",
        content: "Boom!",
        status: "failed",
      });

      const pool = useExecutionStore.getState().agentPool["conv-1"];
      expect(pool[0].status).toBe("failed");
      expect(pool[0].error).toBe("Boom!");
    });
  });

  describe("handleSubAgentCard", () => {
    it("adds a sub-agent card to the pool", () => {
      useExecutionStore.getState().handleSubAgentCard({
        conversationId: "conv-1",
        agentName: "CodeReviewer",
        agentType: "expert",
        childConversationId: "child-1",
        childSessionId: "session-1",
        status: "running",
        description: "Reviewing code...",
        isFork: false,
      });

      const pool = useExecutionStore.getState().agentPool["conv-1"];
      expect(pool).toHaveLength(1);
      expect(pool[0].type).toBe("sub_agent");
      expect(pool[0].name).toBe("CodeReviewer");
      expect(pool[0].status).toBe("running");
    });

    it("updates existing sub-agent card", () => {
      useExecutionStore.getState().handleSubAgentCard({
        conversationId: "conv-1",
        agentName: "CodeReviewer",
        agentType: "expert",
        childConversationId: "child-1",
        status: "running",
        description: "Reviewing code...",
        isFork: false,
      });
      useExecutionStore.getState().handleSubAgentCard({
        conversationId: "conv-1",
        agentName: "CodeReviewer",
        agentType: "expert",
        childConversationId: "child-1",
        status: "completed",
        description: "Done reviewing",
        isFork: false,
      });

      const pool = useExecutionStore.getState().agentPool["conv-1"];
      expect(pool[0].status).toBe("completed");
      expect(pool[0].summary).toBe("Done reviewing");
    });
  });

  describe("handleDone", () => {
    it("transitions to completed phase", () => {
      useExecutionStore.getState().transition("conv-1", "executing");
      useExecutionStore.getState().handleDone({ conversationId: "conv-1", assistantMessageId: "msg-1", text: "done" });
      expect(useExecutionStore.getState().phases["conv-1"]).toBe("completed");
    });

    it("does not transition if already in terminal phase", () => {
      useExecutionStore.getState().transition("conv-1", "completed");
      useExecutionStore.getState().handleDone({ conversationId: "conv-1", assistantMessageId: "msg-1", text: "done" });
      expect(useExecutionStore.getState().phases["conv-1"]).toBe("completed");
    });

    it("clears agent status", () => {
      useExecutionStore.getState().setAgentStatus("conv-1", "Processing...");
      useExecutionStore.getState().transition("conv-1", "executing");
      useExecutionStore.getState().handleDone({ conversationId: "conv-1", assistantMessageId: "msg-1", text: "done" });
      expect(useExecutionStore.getState().agentStatus["conv-1"]).toBe("");
    });
  });

  describe("handleError", () => {
    it("transitions to failed phase", () => {
      useExecutionStore.getState().transition("conv-1", "executing");
      useExecutionStore.getState().handleError({
        conversationId: "conv-1",
        message: "Something went wrong",
      });
      expect(useExecutionStore.getState().phases["conv-1"]).toBe("failed");
      expect(useExecutionStore.getState().agentStatus["conv-1"]).toBe("Something went wrong");
    });

    it("does not transition if already in terminal phase", () => {
      useExecutionStore.getState().transition("conv-1", "failed");
      useExecutionStore.getState().handleError({
        conversationId: "conv-1",
        message: "Another error",
      });
      expect(useExecutionStore.getState().phases["conv-1"]).toBe("failed");
    });
  });

  describe("handleCancelled", () => {
    it("transitions to cancelled phase and clears tool call", () => {
      useExecutionStore.getState().transition("conv-1", "executing");
      useExecutionStore.getState().setCurrentTool({
        toolName: "echo",
        toolUseId: "tool-1",
        conversationId: "conv-1",
        startedAt: Date.now(),
      });
      useExecutionStore.getState().handleCancelled({ conversationId: "conv-1", reason: "user cancelled" });

      expect(useExecutionStore.getState().phases["conv-1"]).toBe("cancelled");
      expect(useExecutionStore.getState().currentToolCall).toBeNull();
      expect(useExecutionStore.getState().agentStatus["conv-1"]).toBe("");
    });
  });

  describe("clearConversation", () => {
    it("removes all state for a conversation", () => {
      useExecutionStore.getState().transition("conv-1", "executing");
      useExecutionStore.getState().setAgentStatus("conv-1", "Processing...");
      useExecutionStore.getState().upsertPoolItem({
        id: "item-1",
        conversationId: "conv-1",
        type: "worker",
        name: "W1",
        status: "running",
        startedAt: Date.now(),
      });
      useExecutionStore.getState().setCurrentTool({
        toolName: "echo",
        toolUseId: "tool-1",
        conversationId: "conv-1",
        startedAt: Date.now(),
      });

      useExecutionStore.getState().clearConversation("conv-1");

      const state = useExecutionStore.getState();
      expect(state.phases["conv-1"]).toBeUndefined();
      expect(state.agentStatus["conv-1"]).toBeUndefined();
      expect(state.agentPool["conv-1"]).toBeUndefined();
      expect(state.currentToolCall).toBeNull();
    });

    it("does not affect other conversations", () => {
      useExecutionStore.getState().transition("conv-1", "executing");
      useExecutionStore.getState().transition("conv-2", "planning");

      useExecutionStore.getState().clearConversation("conv-1");

      expect(useExecutionStore.getState().phases["conv-1"]).toBeUndefined();
      expect(useExecutionStore.getState().phases["conv-2"]).toBe("planning");
    });
  });

  describe("clearConversationUI", () => {
    it("clears currentToolCall if it belongs to the conversation", () => {
      useExecutionStore.getState().setCurrentTool({
        toolName: "echo",
        toolUseId: "tool-1",
        conversationId: "conv-1",
        startedAt: Date.now(),
      });
      useExecutionStore.getState().clearConversationUI("conv-1");
      expect(useExecutionStore.getState().currentToolCall).toBeNull();
    });

    it("preserves currentToolCall if it belongs to a different conversation", () => {
      const tc = {
        toolName: "echo",
        toolUseId: "tool-1",
        conversationId: "conv-2",
        startedAt: Date.now(),
      };
      useExecutionStore.getState().setCurrentTool(tc);
      useExecutionStore.getState().clearConversationUI("conv-1");
      expect(useExecutionStore.getState().currentToolCall).toEqual(tc);
    });
  });
});

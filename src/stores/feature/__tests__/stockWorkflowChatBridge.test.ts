import { useConversationStore } from "@/stores/domain/conversationStore";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const { invokeMock, listenMock, unlistenMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
  listenMock: vi.fn(),
  unlistenMock: vi.fn(),
}));

vi.mock("@/lib/invoke", () => ({
  invoke: invokeMock,
  listen: listenMock,
  isTauri: () => false,
}));

import { startStockWorkflowChatBridge, stopStockWorkflowChatBridge } from "@/stores/feature/stockWorkflowChatBridge";

describe("stockWorkflowChatBridge", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.clearAllMocks();
    useConversationStore.setState({
      activeConversationId: "conv-1",
      messages: [],
    });
    listenMock.mockImplementation(() => Promise.resolve(unlistenMock));
    invokeMock.mockResolvedValue({ id: "agg-1", content: "initial" });
  });

  afterEach(() => {
    vi.runOnlyPendingTimers();
    vi.useRealTimers();
    stopStockWorkflowChatBridge("conv-1");
  });

  it("maps bull-researcher / bear-researcher to debate rounds and updates aggregate content", async () => {
    let stepHandler:
      | ((
        event: {
          payload: {
            workflowId: string;
            nodeId: string;
            status: string;
            totalNodes: number;
            completedNodes: number;
            output?: unknown;
          };
        },
      ) => void)
      | undefined;
    listenMock.mockImplementation((event: string, handler: unknown) => {
      if (event === "workflow-step-done") {
        stepHandler = handler as typeof stepHandler;
      }
      return Promise.resolve(unlistenMock);
    });

    await startStockWorkflowChatBridge("conv-1");
    expect(invokeMock).toHaveBeenCalledWith(
      "send_system_message",
      expect.objectContaining({ conversationId: "conv-1" }),
    );
    expect(stepHandler).toBeDefined();

    stepHandler?.({
      payload: {
        workflowId: "wf-1",
        nodeId: "bull-researcher",
        status: "completed",
        totalNodes: 10,
        completedNodes: 1,
        output: { content: "多方看涨" },
      },
    });
    stepHandler?.({
      payload: {
        workflowId: "wf-1",
        nodeId: "bear-researcher",
        status: "completed",
        totalNodes: 10,
        completedNodes: 2,
        output: { content: "空方谨慎" },
      },
    });

    vi.runOnlyPendingTimers();

    expect(invokeMock).toHaveBeenCalledWith(
      "update_message_content",
      expect.objectContaining({
        id: "agg-1",
        content: expect.stringContaining("多方看涨"),
      }),
    );
    expect(invokeMock).toHaveBeenCalledWith(
      "update_message_content",
      expect.objectContaining({
        id: "agg-1",
        content: expect.stringContaining("空方谨慎"),
      }),
    );
  });

  it("captures analyst node output into the aggregate card", async () => {
    let stepHandler:
      | ((
        event: {
          payload: {
            workflowId: string;
            nodeId: string;
            status: string;
            totalNodes: number;
            completedNodes: number;
            output?: unknown;
          };
        },
      ) => void)
      | undefined;
    listenMock.mockImplementation((event: string, handler: unknown) => {
      if (event === "workflow-step-done") {
        stepHandler = handler as typeof stepHandler;
      }
      return Promise.resolve(unlistenMock);
    });

    await startStockWorkflowChatBridge("conv-1");
    expect(stepHandler).toBeDefined();

    stepHandler?.({
      payload: {
        workflowId: "wf-1",
        nodeId: "a-market-analyst",
        status: "completed",
        totalNodes: 10,
        completedNodes: 1,
        output: { content: "技术面分析：成交量放大，趋势向上" },
      },
    });

    vi.runOnlyPendingTimers();

    expect(invokeMock).toHaveBeenCalledWith(
      "update_message_content",
      expect.objectContaining({
        id: "agg-1",
        content: expect.stringContaining("技术面分析"),
      }),
    );
  });
});

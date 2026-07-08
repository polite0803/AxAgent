import { beforeEach, describe, expect, it, vi } from "vitest";

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

import { useStockAnalysisStore } from "@/stores/feature/stockAnalysisStore";

describe("stockAnalysisStore", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useStockAnalysisStore.setState({
      searchKeyword: "",
      searchResults: [],
      analysisId: null,
      workflowId: null,
      stockCode: "",
      stockName: "",
      analysisDate: "",
      status: "idle",
      quote: null,
      klineData: [],
      analystReports: {},
      debateRounds: [],
      riskAssessments: {},
      decision: null,
      error: null,
      history: [],
      currentStage: 0,
      llmStatus: "unknown",
      _unlisten: null,
      _searchTimer: null,
    });
    invokeMock.mockResolvedValue({
      analysisId: "test-id",
      workflowId: "wf-1",
      stockCode: "600519",
      stockName: "茅台",
    });
    listenMock.mockResolvedValue(unlistenMock);
  });

  describe("startAnalysis", () => {
    it("starts analysis and sets loading then running status", async () => {
      await useStockAnalysisStore.getState().startAnalysis("600519");

      const state = useStockAnalysisStore.getState();
      expect(state.status).toBe("running");
      expect(state.stockCode).toBe("600519");
      expect(state.analysisId).toBe("test-id");
      expect(invokeMock).toHaveBeenCalledWith("get_workflow_template", {
        id: "stock-analysis",
      });
      expect(invokeMock).toHaveBeenCalledWith("run_stock_workflow", {
        stockCode: "600519",
        dryRun: false,
        asOfDate: null,
      });
    });

    it("ignores duplicate start when status is loading", async () => {
      useStockAnalysisStore.setState({ status: "loading" });
      await useStockAnalysisStore.getState().startAnalysis("600519");

      expect(invokeMock).not.toHaveBeenCalled();
    });

    it("ignores duplicate start when status is running", async () => {
      useStockAnalysisStore.setState({ status: "running" });
      await useStockAnalysisStore.getState().startAnalysis("000001");

      expect(invokeMock).not.toHaveBeenCalled();
    });

    it("resets analysis fields before starting", async () => {
      useStockAnalysisStore.setState({
        decision: { action: "买入", confidence: 80 } as any,
        analystReports: { "test-analyst": "old report" },
        error: "previous error",
      });

      await useStockAnalysisStore.getState().startAnalysis("600519");

      const state = useStockAnalysisStore.getState();
      expect(state.decision).toBeNull();
      expect(state.analystReports).toEqual({});
      expect(state.error).toBeNull();
    });
  });

  describe("reset", () => {
    it("clears all state and calls unlisten", () => {
      useStockAnalysisStore.setState({
        analysisId: "test-id",
        stockCode: "600519",
        stockName: "茅台",
        status: "running",
        decision: { action: "买入", confidence: 80 } as any,
        _unlisten: unlistenMock,
      });

      useStockAnalysisStore.getState().reset();

      const state = useStockAnalysisStore.getState();
      expect(state.analysisId).toBeNull();
      expect(state.stockCode).toBe("");
      expect(state.status).toBe("idle");
      expect(state.decision).toBeNull();
      expect(unlistenMock).toHaveBeenCalled();
    });

    it("handles reset when no unlisten is registered", () => {
      useStockAnalysisStore.setState({ _unlisten: null });
      expect(() => useStockAnalysisStore.getState().reset()).not.toThrow();
    });
  });

  describe("setupEventListener", () => {
    it("registers event listeners once", async () => {
      await useStockAnalysisStore.getState().setupEventListener();
      expect(listenMock).toHaveBeenCalledTimes(4);
      expect(listenMock).toHaveBeenCalledWith("workflow-step-done", expect.any(Function));
      expect(listenMock).toHaveBeenCalledWith("workflow-completed", expect.any(Function));
      expect(listenMock).toHaveBeenCalledWith("workflow-error", expect.any(Function));
      expect(listenMock).toHaveBeenCalledWith("stock-monitor-t0-rerun-requested", expect.any(Function));

      await useStockAnalysisStore.getState().setupEventListener();
      expect(listenMock).toHaveBeenCalledTimes(4); // 4 events registered, 2nd call is no-op
    });

    it("handles workflow-completed event with AgentExecutor JSON results", async () => {
      let completeHandler: (
        event: { payload: { workflowId: string; results: Record<string, { role: string; content: string }> } },
      ) => void = () => {};
      listenMock.mockImplementation((event: string, handler) => {
        if (event === "workflow-completed") { completeHandler = handler; }
        return Promise.resolve(unlistenMock);
      });

      await useStockAnalysisStore.getState().setupEventListener();

      completeHandler({
        payload: {
          workflowId: "wf-1",
          results: {
            "a-market-analyst": { role: "market-analyst", content: "技术面看好" },
            "bull-r1": { role: "bull-researcher", content: "多方看涨" },
            "bear-r1": { role: "bear-researcher", content: "空方谨慎" },
            "risk-agg": { role: "aggressive-debator", content: "高风险" },
            "trader": { role: "trader", content: "建议轻仓" },
            "portfolio-mgr": {
              role: "portfolio-manager",
              content: JSON.stringify({ action: "BUY", confidence: 85 }),
            },
          },
        },
      });

      const state = useStockAnalysisStore.getState();
      expect(state.status).toBe("completed");
      expect(state.analystReports["market-analyst"]).toBe("技术面看好");
      expect(state.debateRounds).toHaveLength(1);
      expect(state.debateRounds[0]).toEqual({ round: 1, bull: "多方看涨", bear: "空方谨慎" });
    });

    it("handles workflow-error event", async () => {
      let errorHandler: (event: { payload: { workflowId: string; error: string } }) => void = () => {};
      listenMock.mockImplementation((event: string, handler) => {
        if (event === "workflow-error") { errorHandler = handler; }
        return Promise.resolve(unlistenMock);
      });

      await useStockAnalysisStore.getState().setupEventListener();

      errorHandler({
        payload: { workflowId: "wf-1", error: "网络超时" },
      });

      const state = useStockAnalysisStore.getState();
      expect(state.error).toBe("网络超时");
      expect(state.status).toBe("error");
    });
  });

  describe("cancelAnalysis", () => {
    it("cancels and resets status to idle", async () => {
      useStockAnalysisStore.setState({ workflowId: "wf-1", status: "running" });
      invokeMock.mockResolvedValue(undefined);

      await useStockAnalysisStore.getState().cancelAnalysis();

      expect(invokeMock).toHaveBeenCalledWith("cancel_stock_workflow", { workflowId: "wf-1" });
      expect(useStockAnalysisStore.getState().status).toBe("cancelled");
    });
  });
});

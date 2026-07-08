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

import { inferStage, useStockAnalysisStore } from "@/stores/feature/stockAnalysisStore";

/**
 * 测试覆盖范围（弥补审计中的"Store 逻辑、事件处理、辩论同步、错误路径、进度计算"无覆盖的问题）：
 *   1. inferStage 节点 ID → 阶段号 映射
 *   2. normalizeDecision confidence 字段处理（修复 #14 后）
 *   3. parseWorkflowResults 间接通过 loadAnalysis 测 blackboardSnapshot 解析（修复 #7 后）
 *   4. workflow-error 事件错误路径（修复 #9 后）
 *   5. inferStage 进度不漏节点（修复 #2/#6 后）
 *
 * 修复未落地时使用 `it.todo` 占位 —— 任务允许存在 todo 项；运行命令仍应报告 Test Files 1 passed。
 */
describe("stockAnalysisStore - feature coverage", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useStockAnalysisStore.setState({
      analysisId: null,
      workflowId: null,
      stockCode: "",
      stockName: "",
      status: "idle",
      quote: null,
      klineData: [],
      analystReports: {},
      debateRounds: [],
      riskAssessments: {},
      decision: null,
      error: null,
      currentStage: 0,
      progressPct: 0,
      llmStatus: "unknown",
      _unlisten: null,
      timeline: [],
      highlightedPanel: null,
    });
    listenMock.mockResolvedValue(unlistenMock);
  });

  // ────────────────────────────────────────────────────────────
  // 1. inferStage 阶段映射
  // ────────────────────────────────────────────────────────────
  describe("inferStage - 节点 ID → 阶段号映射", () => {
    it("a-* 分析师节点 → 阶段 1", () => {
      expect(inferStage("a-market-analyst")).toBe(1);
      expect(inferStage("a-sentiment")).toBe(1);
      expect(inferStage("a-news")).toBe(1);
      expect(inferStage("a-fundamentals")).toBe(1);
      expect(inferStage("a-policy")).toBe(1);
      expect(inferStage("a-hot-money")).toBe(1);
      expect(inferStage("a-lockup")).toBe(1);
      expect(inferStage("a-research")).toBe(1);
      expect(inferStage("a-sector")).toBe(1);
    });

    it("bull-r* / bear-r* 辩论节点 → 阶段 2", () => {
      expect(inferStage("bull-r1")).toBe(2);
      expect(inferStage("bear-r1")).toBe(2);
      expect(inferStage("bull-r2")).toBe(2);
      expect(inferStage("bear-r2")).toBe(2);
    });

    it("risk-* / research-mgr 风险评估节点 → 阶段 3", () => {
      expect(inferStage("risk-agg")).toBe(3);
      expect(inferStage("risk-con")).toBe(3);
      expect(inferStage("research-mgr")).toBe(3);
    });

    it("trader / portfolio-mgr 决策节点 → 阶段 4", () => {
      expect(inferStage("trader")).toBe(4);
      expect(inferStage("portfolio-mgr")).toBe(4);
    });

    // 关键新增（修复 #2/#6 后）— 当前 inferStage 未包含这些节点的映射
    it("agg-risk 决策后处理节点 → 阶段 4 (修复 #2/#6 后启用)", () => {
      expect(inferStage("agg-risk")).toBe(4);
    });
    it("cls-risk-level 决策后处理节点 → 阶段 4 (修复 #2/#6 后启用)", () => {
      expect(inferStage("cls-risk-level")).toBe(4);
    });
    it("v-validate 决策后处理节点 → 阶段 4 (修复 #2/#6 后启用)", () => {
      expect(inferStage("v-validate")).toBe(4);
    });
    it("notify-result 决策后处理节点 → 阶段 4 (修复 #2/#6 后启用)", () => {
      expect(inferStage("notify-result")).toBe(4);
    });

    // 修复 P2 Bug #4: 补充缺失的 inferStage 映射
    it("value-investor 巴菲特框架 → 阶段 3 (修复 P2 #4 后)", () => {
      expect(inferStage("value-investor")).toBe(3);
    });
    it("t-* 工具节点 → 阶段 1 (修复 P2 #4 后)", () => {
      expect(inferStage("t-fundamentals-data")).toBe(1);
      expect(inferStage("t-news-data")).toBe(1);
      expect(inferStage("t-policy-data")).toBe(1);
      expect(inferStage("t-research-data")).toBe(1);
      expect(inferStage("t-scoring")).toBe(1);
      expect(inferStage("t-valuation")).toBe(1);
      expect(inferStage("t-risk")).toBe(1);
    });
    it("debate-bull-bear 装饰容器 → 阶段 2 (修复 P2 #4 后)", () => {
      expect(inferStage("debate-bull-bear")).toBe(2);
    });
    it("p-analysts / p-risk-assess 装饰容器 → 阶段 1/3 (修复 P2 #4 后)", () => {
      expect(inferStage("p-analysts")).toBe(1);
      expect(inferStage("p-risk-assess")).toBe(3);
    });
    it("trigger 入口节点 → 阶段 0 (修复 P2 #4 后)", () => {
      expect(inferStage("trigger")).toBe(0);
    });
    // 修复 P2 Bug #3: bull-researcher 旧命名（已不再生成）保留兼容映射
    it("bull-researcher / bear-researcher 旧别名 → 阶段 2 (修复 P2 #3 后保留兼容)", () => {
      expect(inferStage("bull-researcher")).toBe(2);
      expect(inferStage("bear-researcher")).toBe(2);
    });

    // P3 (real-nodes): 3 个新节点阶段映射
    it("data-quality 数据质量检查 → 阶段 3", () => {
      expect(inferStage("data-quality")).toBe(3);
    });
    it("raw-data 原始数据聚合 → 阶段 3", () => {
      expect(inferStage("raw-data")).toBe(3);
    });
    it("rule-check 规则检查 → 阶段 4", () => {
      expect(inferStage("rule-check")).toBe(4);
    });

    // 修复 #2/#6 后
    it("所有已知 DAG 节点 ID 都能映射到 1-4 阶段之一, 不允许返回 -1 (修复 #2/#6 后启用)", () => {
      const allNodeIds = [
        "a-market-analyst",
        "a-sentiment",
        "a-news",
        "a-fundamentals",
        "a-policy",
        "a-hot-money",
        "a-lockup",
        "a-research",
        "a-sector",
        // P0 修复新增的分析师节点
        "value-investor",
        // 工具节点 t-*
        "t-fundamentals-data",
        "t-news-data",
        "t-policy-data",
        "t-research-data",
        "t-scoring",
        "t-valuation",
        "t-risk",
        // 辩论节点（实际命名 + 旧别名）
        "bull-r1",
        "bear-r1",
        "bull-r2",
        "bear-r2",
        "bull-r3",
        "bear-r3",
        "bull-researcher",
        "bear-researcher",
        "debate-bull-bear",
        // 装饰容器
        "p-analysts",
        "p-risk-assess",
        // 风险/研究/决策
        "risk-agg",
        "risk-con",
        "risk-neu",
        "research-mgr",
        "trader",
        "portfolio-mgr",
        "agg-risk",
        "cls-risk-level",
        "v-validate",
        "notify-result",
        // P3 (real-nodes) 决策辅助节点
        "data-quality",
        "raw-data",
        "rule-check",
        // 入口
        "trigger",
      ];
      for (const id of allNodeIds) {
        expect(inferStage(id), `nodeId=${id}`).not.toBe(-1);
      }
    });
  });

  // ────────────────────────────────────────────────────────────
  // 2. normalizeDecision (通过 loadAnalysis 间接测试)
  //    normalizeDecision 在 store 内未 export，但其行为通过
  //    loadAnalysis → JSON.parse(decisionJson) → normalizeDecision 路径可见。
  // ────────────────────────────────────────────────────────────
  describe("normalizeDecision (via loadAnalysis) - confidence 字段处理", () => {
    const setupLoadAnalysisMock = (decision: Record<string, unknown> | null) => {
      invokeMock.mockResolvedValueOnce({
        id: "an-1",
        stockCode: "600519",
        stockName: "茅台",
        decisionJson: decision ? JSON.stringify(decision) : null,
        blackboardSnapshot: null,
      });
    };

    it("数字 confidence 80 → 80 (保持不变)", async () => {
      setupLoadAnalysisMock({ action: "BUY", confidence: 80 });
      await useStockAnalysisStore.getState().loadAnalysis("an-1");
      expect(useStockAnalysisStore.getState().decision?.confidence).toBe(80);
    });

    it("字符串 confidence '80' → 数字 80 (字符串转换)", async () => {
      setupLoadAnalysisMock({ action: "BUY", confidence: "80" });
      await useStockAnalysisStore.getState().loadAnalysis("an-1");
      expect(useStockAnalysisStore.getState().decision?.confidence).toBe(80);
    });

    it("undefined confidence → 0 (默认值)", async () => {
      setupLoadAnalysisMock({ action: "BUY" });
      await useStockAnalysisStore.getState().loadAnalysis("an-1");
      expect(useStockAnalysisStore.getState().decision?.confidence).toBe(0);
    });

    it("越界 confidence 150 → clamp 到 100", async () => {
      setupLoadAnalysisMock({ action: "BUY", confidence: 150 });
      await useStockAnalysisStore.getState().loadAnalysis("an-1");
      expect(useStockAnalysisStore.getState().decision?.confidence).toBe(100);
    });

    it("负值 confidence -10 → clamp 到 0", async () => {
      setupLoadAnalysisMock({ action: "BUY", confidence: -10 });
      await useStockAnalysisStore.getState().loadAnalysis("an-1");
      expect(useStockAnalysisStore.getState().decision?.confidence).toBe(0);
    });

    it("snake_case position_pct 能正确解析", async () => {
      setupLoadAnalysisMock({ action: "BUY", confidence: 50, position_pct: 25 });
      await useStockAnalysisStore.getState().loadAnalysis("an-1");
      expect(useStockAnalysisStore.getState().decision?.positionPct).toBe(25);
    });

    it("camelCase positionPct 能正确解析", async () => {
      setupLoadAnalysisMock({ action: "BUY", confidence: 50, positionPct: 30 });
      await useStockAnalysisStore.getState().loadAnalysis("an-1");
      expect(useStockAnalysisStore.getState().decision?.positionPct).toBe(30);
    });

    it("snake_case target_price 能正确解析", async () => {
      setupLoadAnalysisMock({ action: "BUY", confidence: 50, target_price: 1500 });
      await useStockAnalysisStore.getState().loadAnalysis("an-1");
      expect(useStockAnalysisStore.getState().decision?.targetPrice).toBe(1500);
    });

    it("camelCase targetPrice 能正确解析", async () => {
      setupLoadAnalysisMock({ action: "BUY", confidence: 50, targetPrice: 1500 });
      await useStockAnalysisStore.getState().loadAnalysis("an-1");
      expect(useStockAnalysisStore.getState().decision?.targetPrice).toBe(1500);
    });

    // 全零空壳：loadAnalysis 不应把全零对象塞进 store
    it("空对象 decisionJson {} → decision 保持 null（不污染 store）", async () => {
      setupLoadAnalysisMock({});
      await useStockAnalysisStore.getState().loadAnalysis("an-1");
      expect(useStockAnalysisStore.getState().decision).toBeNull();
    });

    it("只有空字段 { action: null } → decision 保持 null", async () => {
      setupLoadAnalysisMock({ action: null, confidence: 0, positionPct: 0 });
      await useStockAnalysisStore.getState().loadAnalysis("an-1");
      expect(useStockAnalysisStore.getState().decision).toBeNull();
    });

    it("HOLD 是合法决策 → 保留", async () => {
      setupLoadAnalysisMock({ action: "HOLD" });
      await useStockAnalysisStore.getState().loadAnalysis("an-1");
      expect(useStockAnalysisStore.getState().decision?.action).toBe("HOLD");
    });
  });

  // ────────────────────────────────────────────────────────────
  // 3. parseWorkflowResults (通过 loadAnalysis 间接测试 blackboardSnapshot)
  // ────────────────────────────────────────────────────────────
  describe("parseWorkflowResults (via loadAnalysis blackboardSnapshot) - 分析师/辩论/风险同步", () => {
    it("report.* 键填充到 analystReports", async () => {
      invokeMock.mockResolvedValueOnce({
        id: "an-1",
        stockCode: "600519",
        stockName: "茅台",
        decisionJson: null,
        blackboardSnapshot: JSON.stringify({
          "report.market-analyst": "技术面分析报告",
          "report.sentiment": "情绪面分析报告",
        }),
      });
      await useStockAnalysisStore.getState().loadAnalysis("an-1");
      const reports = useStockAnalysisStore.getState().analystReports;
      expect(reports["market-analyst"]).toBe("技术面分析报告");
      expect(reports["sentiment"]).toBe("情绪面分析报告");
    });

    it("debate.bull.round_X + debate.bear.round_X 填充到 debateRounds[0]", async () => {
      invokeMock.mockResolvedValueOnce({
        id: "an-1",
        stockCode: "600519",
        stockName: "茅台",
        decisionJson: null,
        blackboardSnapshot: JSON.stringify({
          "debate.bull.round_1": "多方看涨",
          "debate.bear.round_1": "空方谨慎",
        }),
      });
      await useStockAnalysisStore.getState().loadAnalysis("an-1");
      const rounds = useStockAnalysisStore.getState().debateRounds;
      expect(rounds).toHaveLength(1);
      expect(rounds[0]).toEqual({ round: 1, bull: "多方看涨", bear: "空方谨慎" });
    });

    it("risk.agg / risk.con 键填充到 riskAssessments", async () => {
      invokeMock.mockResolvedValueOnce({
        id: "an-1",
        stockCode: "600519",
        stockName: "茅台",
        decisionJson: null,
        blackboardSnapshot: JSON.stringify({
          "risk.agg": "激进观点",
          "risk.con": "保守观点",
        }),
      });
      await useStockAnalysisStore.getState().loadAnalysis("an-1");
      const risks = useStockAnalysisStore.getState().riskAssessments;
      expect(risks["agg"]).toBe("激进观点");
      expect(risks["con"]).toBe("保守观点");
    });

    // 修复 #7 后: loadAnalysis 应额外恢复 value.assessment / rule_check.passed /
    // raw.objective_score / data_quality_summary
    it("value.assessment 字段能被正确恢复 (修复 #7 后启用)", async () => {
      invokeMock.mockResolvedValueOnce({
        id: "an-1",
        stockCode: "600519",
        stockName: "茅台",
        decisionJson: null,
        blackboardSnapshot: JSON.stringify({
          "value.assessment": "DCF 估值偏低估",
          "value.dcf_intrinsic": "1650",
        }),
      });
      await useStockAnalysisStore.getState().loadAnalysis("an-1");
      const values = useStockAnalysisStore.getState().valueAssessments;
      expect(values["assessment"]).toBe("DCF 估值偏低估");
      expect(values["dcf_intrinsic"]).toBe("1650");
    });
    it("rule_check.passed 字段能被正确恢复 (修复 #7 后启用)", async () => {
      invokeMock.mockResolvedValueOnce({
        id: "an-1",
        stockCode: "600519",
        stockName: "茅台",
        decisionJson: null,
        blackboardSnapshot: JSON.stringify({
          "rule_check.passed": "true",
          "rule_check.violations": "[]",
        }),
      });
      await useStockAnalysisStore.getState().loadAnalysis("an-1");
      const checks = useStockAnalysisStore.getState().ruleCheckResults;
      expect(checks["passed"]).toBe("true");
      expect(checks["violations"]).toBe("[]");
    });
    it("raw.objective_score 字段能被正确恢复 (修复 #7 后启用)", async () => {
      invokeMock.mockResolvedValueOnce({
        id: "an-1",
        stockCode: "600519",
        stockName: "茅台",
        decisionJson: null,
        blackboardSnapshot: JSON.stringify({
          "raw.objective_score": "85",
          "raw.sector_info": "白酒",
        }),
      });
      await useStockAnalysisStore.getState().loadAnalysis("an-1");
      const raws = useStockAnalysisStore.getState().rawData;
      expect(raws["objective_score"]).toBe("85");
      expect(raws["sector_info"]).toBe("白酒");
    });
    it("data_quality_summary 字段能被正确恢复 (修复 #7 后启用)", async () => {
      invokeMock.mockResolvedValueOnce({
        id: "an-1",
        stockCode: "600519",
        stockName: "茅台",
        decisionJson: null,
        blackboardSnapshot: JSON.stringify({
          "data_quality_summary": "数据完整度 95%",
        }),
      });
      await useStockAnalysisStore.getState().loadAnalysis("an-1");
      expect(useStockAnalysisStore.getState().dataQualitySummary).toBe("数据完整度 95%");
    });
  });

  // ────────────────────────────────────────────────────────────
  // 4. workflow-error 事件处理（错误路径）
  // ────────────────────────────────────────────────────────────
  describe("workflow-error 事件处理 - 错误路径", () => {
    const setupErrorHandler = async (): Promise<
      (event: { payload: { workflowId: string; error: string; errorCode?: string } }) => void
    > => {
      let errorHandler: (event: { payload: { workflowId: string; error: string; errorCode?: string } }) => void =
        () => {};
      listenMock.mockImplementation((event: string, handler) => {
        if (event === "workflow-error") { errorHandler = handler; }
        return Promise.resolve(unlistenMock);
      });
      await useStockAnalysisStore.getState().setupEventListener();
      return errorHandler;
    };

    it("错误信息不包含 'LLM' → status: 'error'", async () => {
      const errorHandler = await setupErrorHandler();

      errorHandler({
        payload: { workflowId: "wf-1", error: "网络超时" },
      });

      const state = useStockAnalysisStore.getState();
      expect(state.status).toBe("error");
      expect(state.error).toBe("网络超时");
      expect(state.llmStatus).toBe("unknown");
    });

    it("错误信息包含 'LLM' → status: 'completed' (LLM 回退时工作流已终止)", async () => {
      const errorHandler = await setupErrorHandler();

      errorHandler({
        payload: { workflowId: "wf-1", error: "LLM timeout, falling back to placeholder" },
      });

      const state = useStockAnalysisStore.getState();
      expect(state.status).toBe("completed");
      expect(state.llmStatus).toBe("placeholder");
    });

    // 修复 #4: LLM 错误时 status 为 completed（llmStatus="placeholder" 已表达降级语义）
    it("使用 errorCode: 'LLM_FALLBACK' 也能触发 completed 状态", async () => {
      const errorHandler = await setupErrorHandler();

      errorHandler({
        payload: { workflowId: "wf-1", error: "downstream timeout", errorCode: "LLM_FALLBACK" },
      });

      const state = useStockAnalysisStore.getState();
      expect(state.status).toBe("completed");
      expect(state.llmStatus).toBe("placeholder");
      expect(state.errorCode).toBe("LLM_FALLBACK");
    });
  });

  // ────────────────────────────────────────────────────────────
  // 5. 阶段副作用（通过 workflow-step-done 事件间接验证 inferStage）
  // ────────────────────────────────────────────────────────────
  describe("setupEventListener - workflow-step-done 阶段副作用", () => {
    const setupStepHandler = async (): Promise<
      (
        event: {
          payload: { workflowId: string; nodeId: string; status: string; totalNodes: number; completedNodes: number };
        },
      ) => void
    > => {
      let stepHandler: (
        event: {
          payload: { workflowId: string; nodeId: string; status: string; totalNodes: number; completedNodes: number };
        },
      ) => void = () => {};
      listenMock.mockImplementation((event: string, handler) => {
        if (event === "workflow-step-done") { stepHandler = handler; }
        return Promise.resolve(unlistenMock);
      });
      await useStockAnalysisStore.getState().setupEventListener();
      return stepHandler;
    };

    it("节点 a-market-analyst 完成 → currentStage 提升到 1", async () => {
      const stepHandler = await setupStepHandler();

      stepHandler({
        payload: {
          workflowId: "wf-1",
          nodeId: "a-market-analyst",
          status: "completed",
          totalNodes: 10,
          completedNodes: 1,
        },
      });

      expect(useStockAnalysisStore.getState().currentStage).toBe(1);
      expect(useStockAnalysisStore.getState().progressPct).toBe(10);
    });

    it("节点 portfolio-mgr 完成 → currentStage 提升到 4 (阶段最大值)", async () => {
      const stepHandler = await setupStepHandler();

      stepHandler({
        payload: {
          workflowId: "wf-1",
          nodeId: "portfolio-mgr",
          status: "completed",
          totalNodes: 10,
          completedNodes: 10,
        },
      });

      expect(useStockAnalysisStore.getState().currentStage).toBe(4);
      expect(useStockAnalysisStore.getState().progressPct).toBe(100);
    });

    it("未知节点 ID → currentStage 保持不变 (无 -1 污染)", async () => {
      useStockAnalysisStore.setState({ currentStage: 2 });
      const stepHandler = await setupStepHandler();

      stepHandler({
        payload: {
          workflowId: "wf-1",
          nodeId: "unknown-future-node",
          status: "completed",
          totalNodes: 10,
          completedNodes: 5,
        },
      });

      // currentStage 不应被覆写为 -1；进度可继续推进
      expect(useStockAnalysisStore.getState().currentStage).toBe(2);
    });
  });

  // ────────────────────────────────────────────────────────────
  // 6. workflow-completed 事件 → parseWorkflowResults 填充 4 个新字段
  //    修复 Bug #2: value-investor / rule-check / data-quality / raw-data
  // ────────────────────────────────────────────────────────────
  describe("workflow-completed 事件 - parseWorkflowResults 扩展节点 (修复 Bug #2)", () => {
    const setupCompleteHandler = async (): Promise<
      (event: { payload: { workflowId: string; results: Record<string, { content: string }> } }) => void
    > => {
      let handler: (event: { payload: { workflowId: string; results: Record<string, { content: string }> } }) => void =
        () => {};
      listenMock.mockImplementation((event: string, h) => {
        if (event === "workflow-completed") { handler = h; }
        return Promise.resolve(unlistenMock);
      });
      await useStockAnalysisStore.getState().setupEventListener();
      return handler;
    };

    it("value-investor 节点输出填充到 valueAssessments", async () => {
      const handler = await setupCompleteHandler();

      handler({
        payload: {
          workflowId: "wf-1",
          results: {
            "value-investor": { content: "DCF 估值 1650，Margin of Safety 充足" },
          },
        },
      });

      const values = useStockAnalysisStore.getState().valueAssessments;
      expect(values["value-investor"]).toBe("DCF 估值 1650，Margin of Safety 充足");
    });

    it("rule-check / data-quality / raw-data 节点也填充到对应字段", async () => {
      const handler = await setupCompleteHandler();

      handler({
        payload: {
          workflowId: "wf-1",
          results: {
            "rule-check": { content: "全部规则通过" },
            "data-quality": { content: "数据完整度 92%" },
            "raw-data": { content: "PE=24, PB=6.2, ROE=18%" },
          },
        },
      });

      const state = useStockAnalysisStore.getState();
      expect(state.ruleCheckResults["rule-check"]).toBe("全部规则通过");
      expect(state.dataQualitySummary).toBe("数据完整度 92%");
      expect(state.rawData["raw-data"]).toBe("PE=24, PB=6.2, ROE=18%");
    });

    it("4 个新节点解析不会影响原有 analystReports / debateRounds", async () => {
      const handler = await setupCompleteHandler();

      handler({
        payload: {
          workflowId: "wf-1",
          results: {
            "a-fundamentals": { content: "基本面分析报告" },
            "value-investor": { content: "巴菲特框架评估" },
          },
        },
      });

      const state = useStockAnalysisStore.getState();
      expect(state.analystReports["fundamentals"]).toBe("基本面分析报告");
      expect(state.valueAssessments["value-investor"]).toBe("巴菲特框架评估");
    });
  });

  // ────────────────────────────────────────────────────────────
  // 7. Decision Timeline (Phase 8)
  // ────────────────────────────────────────────────────────────
  describe("Decision Timeline (Phase 8)", () => {
    it("pushTimelineNode 追加新节点", () => {
      const store = useStockAnalysisStore.getState();
      store.pushTimelineNode({
        id: "t-news-data",
        phase: "scan",
        agentId: "t-news-data",
        agentName: "News Data",
        title: "News Data",
        summary: "抓取了 12 条新闻",
        confidence: 0.5,
        status: "done",
        evidenceRefs: [],
      });
      store.pushTimelineNode({
        id: "a-tech-analyst",
        phase: "diagnose",
        agentId: "a-tech-analyst",
        agentName: "Tech Analyst",
        title: "Tech Analyst",
        summary: "技术面偏多",
        confidence: 0.7,
        status: "done",
        evidenceRefs: [],
      });
      expect(useStockAnalysisStore.getState().timeline).toHaveLength(2);
      expect(useStockAnalysisStore.getState().timeline[0].phase).toBe("scan");
      expect(useStockAnalysisStore.getState().timeline[1].phase).toBe("diagnose");
    });

    it("pushTimelineNode 同 id 视为 update（去重）", () => {
      const store = useStockAnalysisStore.getState();
      store.pushTimelineNode({
        id: "trader",
        phase: "decide",
        agentId: "trader",
        agentName: "Trader",
        title: "Trader",
        summary: "v1",
        confidence: 0.4,
        status: "running",
        evidenceRefs: [],
      });
      store.pushTimelineNode({
        id: "trader",
        phase: "decide",
        agentId: "trader",
        agentName: "Trader",
        title: "Trader",
        summary: "v2 updated",
        confidence: 0.8,
        status: "done",
        evidenceRefs: [],
      });
      const tl = useStockAnalysisStore.getState().timeline;
      expect(tl).toHaveLength(1);
      expect(tl[0].summary).toBe("v2 updated");
      expect(tl[0].status).toBe("done");
      expect(tl[0].confidence).toBe(0.8);
    });

    it("updateTimelineNode 局部更新字段", () => {
      const store = useStockAnalysisStore.getState();
      store.pushTimelineNode({
        id: "bull-r1",
        phase: "debate",
        agentId: "bull-r1",
        agentName: "Bull R1",
        title: "Bull R1",
        summary: "多方论点 v1",
        confidence: 0.5,
        status: "running",
        evidenceRefs: [],
      });
      store.updateTimelineNode("bull-r1", { status: "done", summary: "多方论点最终版" });
      const node = useStockAnalysisStore.getState().timeline[0];
      expect(node.status).toBe("done");
      expect(node.summary).toBe("多方论点最终版");
      expect(node.confidence).toBe(0.5); // 未改
    });

    it("updateTimelineNode 不存在的 id 是 no-op", () => {
      const before = useStockAnalysisStore.getState().timeline.length;
      useStockAnalysisStore.getState().updateTimelineNode("nonexistent", { status: "done" });
      expect(useStockAnalysisStore.getState().timeline.length).toBe(before);
    });

    it("clearTimeline 清空", () => {
      const store = useStockAnalysisStore.getState();
      store.pushTimelineNode({
        id: "a-market",
        phase: "diagnose",
        agentId: "a-market",
        agentName: "Market",
        title: "Market",
        summary: "x",
        confidence: 0.5,
        status: "done",
        evidenceRefs: [],
      });
      expect(useStockAnalysisStore.getState().timeline).toHaveLength(1);
      useStockAnalysisStore.getState().clearTimeline();
      expect(useStockAnalysisStore.getState().timeline).toHaveLength(0);
    });
  });
});

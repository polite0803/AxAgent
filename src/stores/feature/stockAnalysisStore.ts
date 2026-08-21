// i18n-exempt: 业务逻辑/API 描述/日志字符串，非 UI 展示文本
import i18n from "@/i18n";
import {
  extractContent,
  extractDecision,
  extractLlmField,
  normalizeDecision,
  parseJsonLoose,
  reconstructVerdictTag,
  tryParseDecision,
} from "@/lib/agentOutput";
import { buildDecisionInputsReport, type DecisionInputsReport } from "@/lib/decisionInputDiagnosis";
import { invoke, listen } from "@/lib/invoke";
import type { UnlistenFn } from "@/lib/invoke";
import { computeStockConsensus, parseAction, parseRiskLevel } from "@/lib/stock-analysis-utils";
import { detectFutureReferencesForNode } from "@/lib/timeTravel/futureReferenceDetector";
import { useTimeAnchorStore } from "@/stores/feature/timeAnchorStore";
import type {
  AnalysisStatus,
  AnalysisSummary,
  DashboardReport,
  EarningsEvent,
  KLine,
  StockConsensus,
  StockDecision,
  StockQuote,
  StockSearchResult,
  TimelineNode,
  TimelinePhase,
} from "@/types/stock-analysis";
import { create } from "zustand";

// ── 模块级缓存 ──
// Bug #P0-3: 模块级变量在 reset() 后不清空，
// 改为 store 内 state 字段管理生命周期。
const EARNINGS_CACHE_TTL_MS = 10 * 60 * 1000; // 10 分钟

// #11 请求去重：报价请求序列号 + 目标代码。
// 自动刷新（30s 轮询）与手动调用 / 切股会并发触发 getStockQuote，
// 慢响应可能覆盖快响应。仅当本请求仍是「最新一次」时才写入 store。
let latestQuoteReqId = 0;
let latestQuoteCode = "";

// #21 工作流错误自动重试：仅对「瞬态」错误重试一次，避免无限循环与重复浪费
const MAX_WORKFLOW_ERROR_RETRIES = 1;
function isRetryableWorkflowError(errorCode: string, msg: string): boolean {
  const lc = `${errorCode} ${msg}`.toLowerCase();
  return (
    lc.includes("timeout")
    || lc.includes("timed out")
    || lc.includes("network")
    || lc.includes("econn")
    || lc.includes("unavailable")
    || lc.includes("503")
    || lc.includes("502")
    || lc.includes("504")
    || lc.includes("temporar")
    || lc.includes("busy")
    || lc.includes("reset by peer")
  );
}

/**
 * parseWorkflowResults 同款策略:从后端 blackboard snapshot 还原各分类字段。
 * snapshot 里 debate/risk/value 节点是 AgentResult 包装({content, model, role, ...}),
 * 真正的 LLM 输出在 content 字段。loadAnalysis 用 extractContent 解包,
 * 与 live 模式 routeNodeOutput / parseWorkflowResults 行为保持一致。
 */
function parseWorkflowResults(results: Record<string, unknown>) {
  const analystReports: Record<string, string> = {};
  const debateRounds: Array<{ round: number; bull: string; bear: string }> = [];
  const riskAssessments: Record<string, string> = {};
  // 修复 Bug #2: 补充 value-investor / rule-check / data-quality / raw-data
  // 四个节点的解析分支。这四个节点类型在前端 store 已有对应字段
  // (valueAssessments / ruleCheckResults / dataQualitySummary / rawData)，
  // 但 parseWorkflowResults 没把它们填进去，导致 workflow-completed 后
  // 对应字段一直为空。
  const valueAssessments: Record<string, string> = {};
  const ruleCheckResults: Record<string, string> = {};
  let dataQualitySummary = "";
  const rawData: Record<string, string> = {};
  let decision: StockDecision | null = null;
  // V55: 提取后端注入的 __untrusted 标记（strict_mode 兜底节点）
  const untrustedNodes: Record<string, true> = {};

  for (const [stepId, raw] of Object.entries(results)) {
    const output = extractContent(raw);

    // V55: 检测 strict_mode 兜底标记。raw 可能是 NodeOutput 包装或纯对象，
    // 尝试读取 __untrusted 字段（任意真值即视为不可信）。
    if (raw && typeof raw === "object") {
      const r = raw as { __untrusted?: boolean; strict_mode_fallback?: boolean };
      if (r.__untrusted === true || r.strict_mode_fallback === true) {
        untrustedNodes[stepId] = true;
      }
    }

    if (stepId.startsWith("a-") && !stepId.includes("bull") && !stepId.includes("bear")) {
      analystReports[stepId.slice(2)] = reconstructVerdictTag(output);
    } else if (stepId === "bull-researcher" || (stepId.startsWith("bull-r") && stepId !== "bull-researcher")) {
      // 辩论子节点: 实际 nodeId 为 "bull-researcher" (DAG 引擎单次执行)
      // 兼容未来多轮模式: bull-r1, bull-r2...
      const round = stepId === "bull-researcher" ? 1 : parseInt(stepId.slice(6), 10);
      const bearKey = stepId === "bull-researcher" ? "bear-researcher" : `bear-r${round}`;
      const bullContent = output;
      const bearContent = extractContent(results[bearKey] ?? "");
      // 允许单边数据:单边空可能是 LLM 失败/超时,仍展示已有数据而非静默丢弃。
      debateRounds.push({
        round,
        bull: bullContent,
        bear: bearContent,
      });
    } else if (stepId === "bear-researcher" || (stepId.startsWith("bear-r") && stepId !== "bear-researcher")) {
      continue;
    } else if ((stepId.startsWith("risk-") && stepId !== "risk-aggregated") || stepId === "research-mgr") {
      riskAssessments[stepId] = output;
    } else if (stepId === "trader") {
      analystReports["investment-plan"] = output;
    } else if (stepId === "portfolio-mgr") {
      // 不要构造全 0 假决策——解析失败就保持 null，
      // 让调用方决定如何处理缺失决策。
      const parsed = extractDecision(raw);
      if (parsed) { decision = parsed; }
    } else if (stepId === "quality-fallback") {
      // V40 修复: quality-gate 判定 D/F 时，降级决策由 quality-fallback 节点生成。
      // 如果 portfolio-mgr 决策为空（降级路径），用 quality-fallback 的保守决策替代。
      // quality-fallback 输出格式: {"action":"持有/减持/卖出","positionPct":0-20,"reasoning":"..."}
      if (!decision) {
        const fallbackParsed = extractDecision(raw);
        if (fallbackParsed) {
          fallbackParsed.isFallback = true; // 标记为降级决策
          decision = fallbackParsed;
        }
      }
    } else if (stepId === "value-investor") {
      // 巴菲特框架评估（与 risk-evaluator 并行，在辩论之后运行）
      valueAssessments[stepId] = output;
    } else if (stepId === "rule-check") {
      ruleCheckResults[stepId] = output;
    } else if (stepId === "data-quality") {
      // V41 修复: data-quality 是 CodeNode + RHAI，原始 raw 形如
      //   {status, language, result: {grade, score, diagnostics, ...}, input_params, node_id, params}
      // extractContent 收到的 raw 会 JSON.stringify 整个包装对象，让 output 变成包装对象的 JSON。
      // 真正的诊断报告在 raw.result 字段中，DecisionBanner 解析时找不到顶层 grade 字段
      // （grade 在嵌套的 .result 里）→ 返回 null → 触发"数据质量诊断未渲染"降级面板。
      // 优先从 raw.result 提取，让 store 存的是纯诊断报告 JSON。
      let content = output;
      if (raw && typeof raw === "object") {
        const r = raw as Record<string, unknown>;
        if (r.result != null) {
          content = typeof r.result === "string" ? r.result : JSON.stringify(r.result);
        }
      }
      dataQualitySummary = content;
      if (import.meta.env.DEV) {
        console.debug("[DQ] parseWorkflowResults stepId=data-quality", {
          rawType: raw ? typeof raw : "(null)",
          rawKeys: raw && typeof raw === "object" ? Object.keys(raw as object) : null,
          hasResult: raw && typeof raw === "object" ? !!(raw as Record<string, unknown>).result : null,
          contentLen: content.length,
          contentPreview: content.slice(0, 200),
        });
      }
    } else if (stepId === "raw-data") {
      rawData[stepId] = output;
    }
  }

  debateRounds.sort((a, b) => a.round - b.round);
  return {
    analystReports,
    debateRounds,
    riskAssessments,
    valueAssessments,
    ruleCheckResults,
    dataQualitySummary,
    rawData,
    decision,
    untrustedNodes,
  };
}

// ── Store ──

/** R1 复盘→进化：dashboard 中单条 (strategy, period) 的统计行 */
export interface EvolutionStrategyStatRow {
  strategyId: string;
  period: string;
  oldWeight: number;
  newWeight: number;
  deltaPct: number;
  winRate: number;
  sampleSize: number;
  rationale: string;
}

/** R1 复盘→进化：recent changes 中的一条历史调整 */
export interface EvolutionRecentChangeRow {
  id: number;
  strategyId: string;
  period: string;
  oldWeight: number;
  newWeight: number;
  deltaPct: number;
  trigger: string;
  appliedAt: number;
  rationale: string;
}

/** R1 复盘→进化：strategy 维度聚合卡片 */
export interface EvolutionStrategySummaryRow {
  strategyId: string;
  avgWeight: number;
  avgWinRate: number;
  totalSamples: number;
  trend: "up" | "down" | "stable";
}

/** R1 复盘→进化：dashboard 完整结构 */
export interface EvolutionDriftDashboard {
  currentWeights: Record<string, number>;
  lastRecalcAt: number;
  stats: EvolutionStrategyStatRow[];
  recentChanges: EvolutionRecentChangeRow[];
  strategySummary: EvolutionStrategySummaryRow[];
}

// ── R2 组合监控类型 ──

/** R2 压测单条结果 */
export interface PortfolioStressResult {
  scenario: string;
  label: string;
  portfolioPnl: number;
  portfolioPnlPct: number;
  topHit?: { stockCode: string; stockName: string; pnlPct: number };
  note: string;
}

export interface PortfolioStressBundle {
  m10?: PortfolioStressResult;
  m20?: PortfolioStressResult;
  blackSwan?: PortfolioStressResult;
}

export interface PortfolioPositionRow {
  stockCode: string;
  stockName: string;
  totalShares: number;
  avgCost: number;
  currentPrice?: number;
  marketValue?: number;
  unrealizedPnl?: number;
  unrealizedPnlPct?: number;
  totalRealizedPnl: number;
  sectorName?: string;
}

export interface PortfolioDashboard {
  isHistorical: boolean;
  asOfDate?: string;
  totalMarketValue: number;
  totalPnl: number;
  totalPnlPct: number;
  cashPct: number;
  maxDrawdownPct: number;
  beta?: number;
  sharpe30d?: number;
  correlationAvg?: number;
  topConcentrationPct: number;
  sectorExposure: Record<string, number>;
  concentrationWarning?: string;
  riskLevel: string;
  diversificationScore: number;
  stressTest: PortfolioStressBundle;
  positions: PortfolioPositionRow[];
  snapshotAt: number;
}

export interface PortfolioCorrelationCell {
  codeA: string;
  codeB: string;
  correlation: number;
}

export interface PositionLimitsCheck {
  ok: boolean;
  reason?: string;
  maxSingleStockPct: number;
  maxTotalPositions: number;
  maxSectorExposurePct: number;
  newPositionValue: number;
}
const DRY_RUN_TTL_MS = 60_000;

interface StockAnalysisState {
  searchKeyword: string;
  searchResults: StockSearchResult[];

  analysisId: string | null;
  workflowId: string | null;
  stockCode: string;
  stockName: string;
  analysisDate: string;
  status: AnalysisStatus;
  // P2-3.3: 报告输出语言 — "zh"（默认中文）或 "en"（英文）
  reportLanguage: "zh" | "en";

  quote: StockQuote | null;
  quoteError: string | null;
  quoteLoading: boolean;
  klineData: KLine[];
  klineError: string | null;
  klineLoading: boolean;
  analystReports: Record<string, string>;
  debateRounds: Array<{ round: number; bull: string; bear: string }>;
  riskAssessments: Record<string, string>;
  // 决策后处理阶段新增字段（修复 #7）
  valueAssessments: Record<string, string>;
  ruleCheckResults: Record<string, string>;
  dataQualitySummary: string;
  rawData: Record<string, string>;
  decision: StockDecision | null;
  /** 方案 D 双向并存: LLM 决策原始 JSON（trader 节点输出） */
  llmDecisionJson: string | null;
  /** 方案 D 双向并存: 公式 vs LLM 一致性分数 0-100 */
  decisionAgreementScore: number | null;
  /** 决策仪表盘报告（借鉴 daily_stock_analysis 推送格式，7 段式结构） */
  dashboardReport: DashboardReport | null;
  /** 决策仪表盘 Markdown 文本（用于复制/推送） */
  dashboardMd: string | null;
  /** V55: 跟踪哪些 AgentNode 触发了 strict_mode 兜底（LLM 输出无法解析为合法 JSON） */
  untrustedNodes: Record<string, true>;
  /**
   * 决策输入诊断报告：纯前端从 workflow results / blackboard snapshot 提取的
   * portfolio-mgr 16 个上游节点数据符合度。不持久化，只在 workflow-completed
   * 和 loadAnalysis 时填充，供 DecisionBanner 展示给用户检查决策数据是否齐全。
   */
  decisionInputsReport: DecisionInputsReport;
  error: string | null;
  errorCode: string | null;
  failedNodes: string[];
  failedNodeErrors: Record<string, string>;
  /** 数据源降级/空数据警告（不阻断流程） */
  dataWarnings: string[];

  history: AnalysisSummary[];

  currentStage: number;
  progressMessage: string;
  progressPct: number;

  llmStatus: "live" | "placeholder" | "unknown";
  chatIndicatorDismissed: boolean;

  // Phase 1: K-line period persistence cross-mount
  klinePeriod: string;
  setKlinePeriod: (period: string) => void;

  // R3-A: K-line adjustment type (复权)
  klineAdj: "auto" | "none" | "forward" | "backward";
  setKlineAdj: (adj: "auto" | "none" | "forward" | "backward") => void;

  // Phase 1: Auto-refresh toggle
  autoRefresh: boolean;
  setAutoRefresh: (enabled: boolean) => void;

  // Phase 4: K-line indicator line toggles
  klineIndicators: { ma5: boolean; ma10: boolean; ma20: boolean };
  toggleIndicator: (key: "ma5" | "ma10" | "ma20") => void;

  // Phase 4: Sidebar panel collapse state (persisted to localStorage)
  sidebarCollapsed: Record<string, boolean>;
  toggleSidebarPanel: (key: string) => void;

  watchlistVersion: number;
  bumpWatchlistVersion: () => void;

  // Phase 8: Decision Timeline
  timeline: TimelineNode[];
  pushTimelineNode: (node: TimelineNode) => void;
  updateTimelineNode: (id: string, patch: Partial<TimelineNode>) => void;
  clearTimeline: () => void;
  // Phase 8: Right panel highlight (0.4s 闪烁)
  highlightedPanel: string | null;
  setHighlightedPanel: (key: string | null) => void;

  // 荐股 ↔ 分析师交叉验证：每只股票在最近一次工作流完成时
  // 缓存的分析师共识，用于 RecommendationPanel 中提示"推荐与共识是否一致"。
  stockCodeConsensus: Record<string, StockConsensus>;
  setStockCodeConsensus: (stockCode: string, consensus: StockConsensus) => void;

  // P0-1: 证据质量驱动权重（可选，无则走旧版简单共识）
  evidenceReport: Record<string, import("@/lib/stock-analysis-utils").EvidenceWeightReport>;
  setEvidenceReport: (stockCode: string, report: import("@/lib/stock-analysis-utils").EvidenceWeightReport) => void;

  // Phase 9: Time-travel snapshot metadata
  // - `asOfDate`: 当前 analysis 的 as_of_date（live 时为 null）
  // - `mode`: 模式标签（"live" / "replay" / "backtest_sweep"）
  // - `violations`: 3 阶段 LLM 未来引用检测发现的违规列表
  asOfDate: string | null;
  mode: "live" | "replay" | "backtest_sweep";
  violations: Array<{ nodeId: string; snippet: string; ruleHit: string }>;

  // Phase 10: Experiment mode (What-If integrated)
  decisionMode: "view" | "experiment" | "execute";
  setDecisionMode: (mode: "view" | "experiment" | "execute") => void;
  experiments: ExperimentRecord[];
  pushExperiment: (record: ExperimentRecord) => void;
  clearExperiments: () => void;
  setAsOfDate: (date: string | null) => void;
  setMode: (mode: "live" | "replay" | "backtest_sweep") => void;
  setViolations: (
    v: Array<{ nodeId: string; snippet: string; ruleHit: string }>,
  ) => void;

  // Actions
  searchStock: (keyword: string, skipDebounce?: boolean) => Promise<void>;
  getStockQuote: (code: string) => Promise<void>;
  getStockKline: (
    code: string,
    period: string,
    limit: number,
    adj?: "auto" | "none" | "forward" | "backward",
  ) => Promise<void>;
  startAnalysis: (
    stockCode: string,
    options?: { parentAnalysisId?: string; language?: string },
  ) => Promise<void>;
  rerunDecision: (analysisId: string) => Promise<void>;
  cancelAnalysis: () => Promise<void>;
  getDryRun: () => Promise<boolean>;
  fetchHistory: (limit?: number, offset?: number) => Promise<void>;
  loadAnalysis: (analysisId: string) => Promise<void>;
  reset: () => void;
  dismissChatIndicator: () => void;
  setReportLanguage: (lang: "zh" | "en") => void;

  // R1 复盘→进化
  evolutionDashboard: EvolutionDriftDashboard | null;
  evolutionRecalculating: boolean;
  evolutionLastError: string | null;
  /** Phase 3: 双视角一致性分数趋势 */
  agreementScoreHistory:
    | Array<
      {
        exitAt: number;
        agreementScore: number;
        stockCode: string;
        stockName: string;
        returnPct: number;
        wasCorrect: number;
      }
    >
    | null;
  agreementScoreHistoryLoading: boolean;
  fetchAgreementScoreHistory: (limit?: number) => Promise<void>;
  fetchEvolutionDashboard: (asOfDate?: string | null) => Promise<void>;
  recalcEvolutionNow: (asOfDate?: string | null) => Promise<void>;
  loadRecoStrategyWeights: () => Promise<Record<string, number>>;

  // R2 组合监控
  portfolioDashboard: PortfolioDashboard | null;
  portfolioCorrelations: PortfolioCorrelationCell[];
  portfolioCorrelationsError: string | null;
  portfolioRefreshing: boolean;
  portfolioLastError: string | null;
  fetchPortfolioDashboard: (asOfDate?: string | null) => Promise<void>;
  refreshPortfolioMetrics: (asOfDate?: string | null) => Promise<void>;
  fetchPortfolioCorrelations: (asOfDate?: string | null) => Promise<void>;
  checkPositionLimits: (
    stockCode: string,
    proposedShares: number,
    proposedPrice: number,
  ) => Promise<PositionLimitsCheck | null>;

  // R3-B: 财报披露事件 — K 线叠加图标用
  earningsEvents: EarningsEvent[];
  earningsLoading: boolean;
  earningsError: string | null;
  showEarningsOnChart: boolean;
  setShowEarningsOnChart: (show: boolean) => void;
  fetchEarningsEvents: (stockCode: string) => Promise<void>;

  // P2-6: RealtimeMonitor T+0 自动重跑配置
  t0Config: {
    enabled: boolean;
    changePctThreshold: number | null;
    turnoverRateThreshold: number | null;
    minIntervalMinutes: number;
  };
  t0Loading: boolean;
  setT0Config: (
    cfg: Partial<{
      enabled: boolean;
      changePctThreshold: number | null;
      turnoverRateThreshold: number | null;
      minIntervalMinutes: number;
    }>,
  ) => Promise<void>;
  fetchT0Config: () => Promise<void>;

  // 模块级缓存迁入 store（防止 reset() 后泄漏） #P0-3
  _lastEarningsFetch: { stockCode: string; ts: number } | null;
  _dryRunCache: { value: boolean; ts: number } | null;

  _unlisten: UnlistenFn | null;
  setupEventListener: () => Promise<void>;
  _searchTimer: ReturnType<typeof setTimeout> | null;
  /** #21 工作流错误自动重试计数（瞬态错误最多重试一次） */
  _workflowErrorRetries: number;
}

export interface ExperimentRecord {
  id: string;
  step: number;
  params: Record<string, number | string>;
  configOverrides: Record<string, number>;
  decisionBefore: Partial<StockDecision>;
  decisionAfter: Partial<StockDecision>;
  accepted: boolean;
  createdAt: number;
}

const initialState = {
  searchKeyword: "",
  searchResults: [],
  analysisId: null,
  workflowId: null,
  stockCode: "",
  stockName: "",
  analysisDate: "",
  status: "idle" as AnalysisStatus,
  reportLanguage: "zh" as "zh" | "en",
  quote: null,
  quoteError: null,
  quoteLoading: false,
  klineData: [],
  klineError: null,
  klineLoading: false,
  analystReports: {},
  debateRounds: [],
  riskAssessments: {},
  valueAssessments: {},
  ruleCheckResults: {},
  dataQualitySummary: "",
  rawData: {},
  decision: null,
  llmDecisionJson: null,
  decisionAgreementScore: null,
  dashboardReport: null,
  dashboardMd: null,
  // V55: 跟踪哪些 AgentNode 触发了 strict_mode 兜底（LLM 输出无法解析为合法 JSON）
  // 后端会在 NodeOutput.output.__untrusted=true 标记，前端提取后存到 untrustedNodes
  // 用于显示红色"数据异常"警告横幅，避免 50/50 兜底被当成有效信号。
  untrustedNodes: {} as Record<string, true>,
  decisionInputsReport: [] as DecisionInputsReport,
  error: null,
  errorCode: null,
  failedNodes: [],
  failedNodeErrors: {},
  dataWarnings: [],
  history: [],
  currentStage: 0,
  progressMessage: "",
  progressPct: 0,
  llmStatus: "unknown" as const,
  chatIndicatorDismissed: false,
  klinePeriod: "6m",
  klineAdj: "auto" as const,
  autoRefresh: false,
  klineIndicators: { ma5: true, ma10: true, ma20: true },
  sidebarCollapsed: {},
  watchlistVersion: 0,
  timeline: [],
  highlightedPanel: null,
  stockCodeConsensus: {},
  evidenceReport: {},
  asOfDate: null,
  mode: "live" as const,
  violations: [],
  evolutionDashboard: null,
  evolutionRecalculating: false,
  evolutionLastError: null,
  agreementScoreHistory: null,
  agreementScoreHistoryLoading: false,
  portfolioDashboard: null,
  portfolioCorrelations: [],
  portfolioCorrelationsError: null,
  portfolioRefreshing: false,
  portfolioLastError: null,
  earningsEvents: [],
  earningsLoading: false,
  earningsError: null,
  showEarningsOnChart: true,
  t0Config: {
    enabled: false,
    changePctThreshold: 3.0,
    turnoverRateThreshold: 8.0,
    minIntervalMinutes: 30,
  },
  t0Loading: false,
  decisionMode: "view" as "view" | "experiment" | "execute",
  experiments: [],
  _lastEarningsFetch: null,
  _dryRunCache: null,
  _workflowErrorRetries: 0,
};

export const useStockAnalysisStore = create<StockAnalysisState>((set, get) => ({
  ...initialState,
  _unlisten: null,
  _searchTimer: null,

  searchStock: async (keyword: string, skipDebounce?: boolean) => {
    set({ searchKeyword: keyword });
    if (keyword.length < 2) {
      set({ searchResults: [] });
      return;
    }
    const { _searchTimer } = get();
    if (_searchTimer) { clearTimeout(_searchTimer); }
    if (skipDebounce) {
      // Enter/点击搜索直接执行，跳过防抖
      try {
        const results = await invoke<StockSearchResult[]>("search_stock", { keyword });
        set({ searchResults: results });
      } catch {
        set({ searchResults: [] });
      }
      return;
    }
    const timer = setTimeout(async () => {
      try {
        const results = await invoke<StockSearchResult[]>("search_stock", { keyword });
        set({ searchResults: results });
      } catch {
        set({ searchResults: [] });
      }
    }, 300);
    set({ _searchTimer: timer });
  },

  getStockQuote: async (code: string) => {
    // #11 请求去重：抢占最新序列号，慢响应(过期响应)将被丢弃
    latestQuoteReqId += 1;
    const myReqId = latestQuoteReqId;
    latestQuoteCode = code;
    set({ quoteLoading: true, quoteError: null });
    try {
      // 时间旅行：从 timeAnchorStore 读 as_of_date，透传给后端（仅 replay/backtest_sweep 模式）
      const asOfDate = (() => {
        const state = useTimeAnchorStore.getState();
        return state.mode === "replay" || state.mode === "backtest_sweep" ? state.asOfDate : null;
      })();
      const quote = await invoke<StockQuote>("get_stock_quote", { stockCode: code, asOfDate });
      // #11 去重：若已有更新的请求(或已切到别的股票)，丢弃本次过期响应
      if (myReqId !== latestQuoteReqId || latestQuoteCode !== code) {
        return;
      }
      // 后端在 as-of 模式(回放历史分析)下,K线合成 quote 时
      // name 字段会 fallback 为 stock_code(见 astock-data/src/lib.rs quote_from_klines),
      // 此时应该保留 store 里已存在的 stockName(一般是 loadAnalysis 时从
      // 历史记录写入的中文名,如 "华如科技"),而不是被 "301302" 覆盖。
      const fallbackName = get().stockName && get().stockName !== code
        ? get().stockName
        : "";
      const resolvedName = (quote.name && quote.name !== code)
        ? quote.name
        : (fallbackName || code);
      set({ quote, stockCode: code, stockName: resolvedName, quoteLoading: false });
      // R3-B: 财报事件缓存 10 分钟，避免每次报价刷新都拉取
      const now = Date.now();
      const lastFetch = get()._lastEarningsFetch;
      if (
        !lastFetch || lastFetch.stockCode !== code
        || (now - lastFetch.ts) > EARNINGS_CACHE_TTL_MS
      ) {
        get().fetchEarningsEvents(code);
        set({ _lastEarningsFetch: { stockCode: code, ts: now } });
      }
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      console.error("[StockAnalysis] Failed to get stock quote:", e);
      // #11 去重：仅当本请求仍是最新时才写入错误，避免旧错误覆盖新数据
      if (myReqId !== latestQuoteReqId || latestQuoteCode !== code) {
        return;
      }
      set({ quoteError: msg, quoteLoading: false });
    }
  },

  getStockKline: async (
    code: string,
    period: string,
    limit: number,
    adj?: "auto" | "none" | "forward" | "backward",
  ) => {
    set({ klineLoading: true, klineError: null });
    try {
      // 时间旅行：K 线按 as_of_date 截断（仅 replay/backtest_sweep 模式）
      const asOfDate = (() => {
        const state = useTimeAnchorStore.getState();
        return state.mode === "replay" || state.mode === "backtest_sweep" ? state.asOfDate : null;
      })();
      const klineData = await invoke<KLine[]>("get_stock_kline", {
        stockCode: code,
        period,
        limit,
        asOfDate,
        adj: adj ?? get().klineAdj,
      });
      set({ klineData, klineLoading: false });
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      console.error("[StockAnalysis] Failed to get kline:", e);
      set({ klineError: msg, klineLoading: false });
    }
  },

  /** 读取 analysis_dry_run 模板变量 (60s store 内缓存) */
  getDryRun: async () => {
    const now = Date.now();
    const cached = get()._dryRunCache;
    if (cached && now - cached.ts < DRY_RUN_TTL_MS) {
      return cached.value;
    }
    try {
      const tmpl = await invoke<Record<string, unknown>>("get_workflow_template", { id: "stock-analysis" });
      const vars = (tmpl?.variables ?? []) as Record<string, unknown>[];
      const v = vars.find((x: Record<string, unknown>) => x.name === "analysis_dry_run");
      const value = !!v?.value;
      set({ _dryRunCache: { value, ts: now } });
      return value;
    } catch {
      return false;
    }
  },

  startAnalysis: async (stockCode: string, options?: { parentAnalysisId?: string; language?: string }) => {
    const { status } = get();
    if (status === "loading" || status === "running") {
      console.warn("[StockAnalysis] Analysis already in progress, ignoring duplicate start");
      return;
    }

    const { _unlisten } = get();
    if (_unlisten) { _unlisten(); }

    set({
      status: "loading",
      error: null,
      errorCode: null,
      failedNodes: [],
      failedNodeErrors: {},
      dataWarnings: [],
      currentStage: 0,
      workflowId: null,
      progressMessage: i18n.t("stockAnalysis.progress.fetchingData"),
      progressPct: 0,
      chatIndicatorDismissed: false,
      analystReports: {},
      debateRounds: [],
      riskAssessments: {},
      valueAssessments: {},
      ruleCheckResults: {},
      dataQualitySummary: "",
      rawData: {},
      decision: null,
      llmDecisionJson: null,
      decisionAgreementScore: null,
      dashboardReport: null,
      dashboardMd: null,
      untrustedNodes: {},
      _unlisten: null,
      timeline: [],
      // 清理跨分析轮次缓存，防止上轮数据污染
      stockCodeConsensus: {},
      decisionInputsReport: [],
    });

    // 先拉取工作流模板（getDryRun），再注册事件监听
    // 顺序不可颠倒：getDryRun 可能失败，此时不应注册监听器
    try {
      const dryRun = await get().getDryRun();

      await get().setupEventListener();

      // 数据源健康检查（非阻塞，仅打日志）
      // P1 修复(2026-07-25): 加入 browser_eastmoney，让用户看到反爬 fallback 通道的状态
      const VENDORS = ["eastmoney", "browser_eastmoney", "sina", "tencent", "akshare"];
      for (const v of VENDORS) {
        invoke("check_vendor_health", { vendor: v }).catch((e) => {
          const msg = e instanceof Error ? e.message : typeof e === "string" ? e : JSON.stringify(e);
          console.warn(`[StockAnalysis] Vendor ${v} health check failed: ${msg}`);
        });
      }

      // 时间旅行模式：从 useTimeAnchorStore 读 as_of_date，透传给后端
      // 只在 replay / backtest_sweep 模式传日期，live 模式传 null 以避免 persist 残留
      const anchorMode = useTimeAnchorStore.getState().mode;
      const rawAsOfDate = useTimeAnchorStore.getState().asOfDate;
      const asOfDate = anchorMode === "replay" || anchorMode === "backtest_sweep" ? rawAsOfDate : null;
      set({
        asOfDate,
        mode: anchorMode === "backtest_sweep" ? "backtest_sweep" : anchorMode === "replay" ? "replay" : "live",
      });
      // 版本化分析：透传原始 analysisId 作为 parent，后端新建独立行保留历史版本。
      // 不传则是首次分析（parent_analysis_id = NULL）。
      const runArgs: Record<string, unknown> = {
        stockCode,
        dryRun,
        asOfDate,
      };
      if (options?.parentAnalysisId) {
        runArgs.parentAnalysisId = options.parentAnalysisId;
      }
      // P2-3.3: 报告语言切换 — 传入 language 让后端追加语言指示到 Agent prompt
      // 优先使用 options.language（显式传入），否则使用 store 中的 reportLanguage
      const effectiveLanguage = options?.language ?? (get().reportLanguage === "en" ? "en" : undefined);
      if (effectiveLanguage) {
        runArgs.language = effectiveLanguage;
      }
      const result = await invoke<Record<string, unknown>>(
        "run_stock_workflow",
        runArgs,
      );

      // P0-4 修复: 检查数据质量预检跳过
      // 后端 run_stock_workflow 返回的 JSON 字段是 camelCase
      // (analysisId / stockCode / stockName),不是 snake_case。
      // 早期实现读 result.analysis_id / result.stock_code 等 snake_case 键
      // 会得到 undefined,导致前端 stockCode 永远是 ""。
      if (result.status === "skipped") {
        const reason = (result.reason as string) || i18n.t("stockAnalysis.workflow.insufficientDataQuality");
        set({
          status: "error",
          error: i18n.t("stockAnalysis.workflow.skipAnalysisError", { reason }),
          errorCode: "DATA_QUALITY_INSUFFICIENT",
          analysisId: result.analysisId as string,
          stockCode: result.stockCode as string || stockCode,
          stockName: result.stockName as string || "",
        });
        return;
      }

      const analysisId = result.analysisId as string;
      const wfId = result.workflowId as string;
      const sc = result.stockCode as string;
      const sn = result.stockName as string;

      set({
        analysisId,
        workflowId: wfId,
        stockCode: sc,
        stockName: sn,
        status: "running",
        progressMessage: i18n.t("stockAnalysis.progress.started"),
        progressPct: 5,
      });

      // 异步拉取报价和 K 线
      // 如果用户已通过 StockSearchBar 选中了同一股票，数据已就绪，跳过重复请求
      const preloadedQuote = get().quote;
      if (!preloadedQuote || preloadedQuote.code !== sc) {
        get().getStockQuote(sc);
      }
      const preloadedKline = get().klineData;
      if (preloadedKline.length === 0) {
        get().getStockKline(sc, "daily", 120);
      }
    } catch (e) {
      console.error("[StockAnalysis] Failed to start workflow:", e);
      // 清理可能已注册的监听器（getDryRun 之后 setupEventListener 可能已经成功）
      const { _unlisten } = get();
      if (_unlisten) { _unlisten(); }
      set({
        status: "error",
        error: typeof e === "string" ? e : (e as Error)?.message ?? i18n.t("stockAnalysis.workflow.startFailed"),
        _unlisten: null,
        workflowId: null,
        progressPct: 0,
      });
    }
  },

  cancelAnalysis: async () => {
    const { workflowId, _unlisten } = get();
    if (workflowId) {
      await invoke("cancel_stock_workflow", { workflowId });
    }
    if (_unlisten) { _unlisten(); }
    // 使用 "cancelled" 状态保留已收集的部分数据（面板仍显示，带"已取消"标记）
    set({ status: "cancelled" as AnalysisStatus, _unlisten: null, currentStage: 0, progressPct: 0, workflowId: null });
  },

  fetchHistory: async (limit = 20, offset = 0) => {
    const history = await invoke<AnalysisSummary[]>("list_stock_analyses", { limit, offset });
    set({ history });
  },

  loadAnalysis: async (analysisId: string) => {
    // ── 清除旧分析状态，避免切换分析时残留旧数据 ──
    // 当从一条历史分析切换到另一条时，如果新记录 blackboardSnapshot 为 null，
    // 旧的分析师报告/辩论回合/风险评估等不会自动清除，导致 UI 显示的是上一个股票的数据。
    set({
      analystReports: {},
      debateRounds: [],
      riskAssessments: {},
      valueAssessments: {},
      ruleCheckResults: {},
      dataQualitySummary: "",
      rawData: {},
      decision: null,
      untrustedNodes: {},
      timeline: [],
      violations: [],
      error: null,
      failedNodes: [],
      failedNodeErrors: {},
      dataWarnings: [],
      llmDecisionJson: null,
      decisionAgreementScore: null,
      dashboardReport: null,
      dashboardMd: null,
      decisionInputsReport: [],
    });

    const record = await invoke<
      AnalysisSummary & {
        decisionJson: string | null;
        llmDecisionJson: string | null;
        blackboardSnapshot: string | null;
      }
    >("get_stock_analysis", { analysisId });

    // [DQ] 顶层诊断：无任何条件守护，必须打印 — 用于确认 loadAnalysis 路径是否触发
    console.log("[DQ] loadAnalysis enter", {
      analysisId,
      stockCode: record.stockCode,
      analysisKind: record.analysisKind,
      hasSnapshot: !!record.blackboardSnapshot,
      snapshotLen: record.blackboardSnapshot?.length ?? 0,
      hasDecisionJson: !!record.decisionJson,
      hasLlmDecisionJson: !!record.llmDecisionJson,
    });

    // 历史数据兼容：旧版在 as-of 模式写入 stock_analyses 时,stock_name 取自
    // quote.name,但 K线合成 quote 时 name 退化为 stock_code(见
    // astock-data/src/lib.rs quote_from_klines),导致历史 stock_name = stock_code。
    // 股票名称是静态的,这里用 search_stock 实时查一次 vendor 拿真实名称覆盖退化值。
    let resolvedName = record.stockName;
    if (!resolvedName || resolvedName === record.stockCode) {
      try {
        const hits = await invoke<Array<{ code: string; name: string; market: string }>>(
          "search_stock",
          { keyword: record.stockCode },
        );
        const exact = hits.find((h) => h.code === record.stockCode);
        if (exact?.name) {
          resolvedName = exact.name;
          console.log("[loadAnalysis] 已用 search_stock 覆盖退化的 stock_name:", {
            code: record.stockCode,
            oldName: record.stockName,
            newName: resolvedName,
          });
        }
      } catch (e) {
        console.warn("[loadAnalysis] search_stock 失败,保留原 stock_name:", e);
      }
    }

    set({ analysisId: record.id, stockCode: record.stockCode, stockName: resolvedName, status: "completed" });

    // 如果是 replay 分析且有 asOfDate，同步设置全局时间锚点，
    // 确保后续 getStockQuote / getStockKline 拉取的是分析时刻的数据而非当前实时数据
    console.log("[loadAnalysis] record:", { analysisKind: record.analysisKind, asOfDate: record.asOfDate });
    // 只对 replay 分析切换时间锚点，live 分析保持当前模式
    if (record.analysisKind === "replay" && record.asOfDate) {
      useTimeAnchorStore.getState().enterReplay(record.asOfDate);
    } else {
      useTimeAnchorStore.getState().enterLive(false);
    }
    console.log("[loadAnalysis] timeAnchor after:", {
      mode: useTimeAnchorStore.getState().mode,
      asOfDate: useTimeAnchorStore.getState().asOfDate,
    });
    // [DQ] 关键诊断：标记即将进入 decisionJson 处理块
    console.log("[DQ] loadAnalysis before decisionJson block", {
      hasDecisionJson: !!record.decisionJson,
      decisionJsonLen: record.decisionJson?.length ?? 0,
      hasLlmDecisionJson: !!record.llmDecisionJson,
      hasSnapshot: !!record.blackboardSnapshot,
    });
    if (record.decisionJson) {
      try {
        // 宽松解析：decisionJson 也可能被 ```json 代码块包裹（与 llmDecisionJson 同源）。
        const raw = parseJsonLoose(record.decisionJson) ?? JSON.parse(record.decisionJson);
        // normalizeDecision 可能返回 null：raw 是全零空壳（LLM 输出过短/解析残缺）。
        // null 不写入 store，让上层走 !decision 分支并在 UI 渲染"决策缺失"占位，
        // 避免 DecisionBanner 拿到全零对象静默不渲染。
        const normalized = normalizeDecision(raw);
        if (normalized) {
          set({ decision: normalized });
          // V64 修复: decisionJson 含 confidence 等字段但无 action 时，
          // normalizeDecision 返回 WAIT，但 llmDecisionJson 可能包含 verdict/report
          // 可推导真实 action。此处复用 llmDecisionJson 兜底逻辑进行修补，
          // 而非等待全零空壳分支（后者因 hasConfidence=true 永远不会触发）。
          if (
            (normalized.action === "WAIT" || !normalized.action)
            && record.llmDecisionJson
          ) {
            const llmRaw = parseJsonLoose(record.llmDecisionJson);
            if (llmRaw?.reasoning) {
              const derived = parseAction(String(llmRaw.reasoning));
              if (derived !== "WAIT") {
                normalized.action = derived;
                set({ decision: normalized });
              }
            }
          }
        } else {
          console.warn(
            "[StockAnalysis] loadAnalysis decisionJson 全零空壳，跳过 set:",
            { analysisId: record.id, keys: Object.keys(raw) },
          );
          // 兜底：formula 决策为空壳，但 llmDecisionJson 有真实决策时，
          // 用 LLM 决策填充 store，避免 UI 永久显示"决策缺失"（方案 D 双向并存，
          // 公式侧缺失时以 LLM 侧为准渲染）。
          if (record.llmDecisionJson) {
            const llmRaw = parseJsonLoose(record.llmDecisionJson);
            const llmDecision = llmRaw ? normalizeDecision(llmRaw) : null;
            if (llmDecision && llmRaw) {
              // 二次兜底：trader 节点按设计只输出价格目标（trader.md 明确定义 schema
              // 仅含 currentPrice/targetPrice/stopLoss/timeHorizon/expectedHoldingDays/
              // confidence/reasoning），不含 action/positionPct。公式侧 portfolio-mgr
              // 又返回空壳时，action 会被 normalizeDecision 退化为默认 WAIT（观望），
              // 导致看空股被误显为"观望"。trader.md 第 101 行保证 reasoning 以
              // `方向:看多|看空|中性` 开头，这里复用既有 parseAction 的 label 映射
              // （"看空"→SELL/"看多"→BUY/"中性"→HOLD）从 reasoning 推导 action，
              // 让横幅方向正确。positionPct 仍由公式侧计算，此处无数据则保持 0（未知）。
              if (
                (llmDecision.action === "WAIT" || !llmDecision.action)
                && llmRaw.reasoning
              ) {
                const derived = parseAction(String(llmRaw.reasoning));
                if (derived !== "WAIT") {
                  llmDecision.action = derived;
                }
              }
              console.warn(
                "[StockAnalysis] loadAnalysis decisionJson 空壳，回退使用 llmDecisionJson 填充 decision",
                { analysisId: record.id, derivedAction: llmDecision.action },
              );
              set({ decision: llmDecision });
            }
          }
        }
      } catch (e) {
        console.error("[StockAnalysis] Failed to parse decision JSON:", e);
      }
    }
    // 方案 D 双向并存: 解析 LLM 决策 JSON 并计算一致性分数
    let llmDecisionJson: string | null = null;
    let decisionAgreementScore: number | null = null;
    if (record.llmDecisionJson) {
      llmDecisionJson = record.llmDecisionJson;
      // 调试: 确认 record.llmDecisionJson 的实际值和前 200 字符
      console.log("[loadAnalysis] llmDecisionJson exists, first 200 chars:", record.llmDecisionJson.slice(0, 200));
      console.log("[loadAnalysis] decisionJson exists:", !!record.decisionJson);
      // V40 修复:
      // 1. 后端 compute_decision_agreement 已在 workflow 完成时计算并嵌入
      //    decision_json.formulaLlmAgreement，优先使用该值。
      // 2. 旧记录无此字段时，前端自行计算（从 action 而非 stance 读取）。
      try {
        // 优先取后端预计算的一致性分数
        // 宽松解析：decisionJson 同样可能被 ```json 代码块包裹
        const djParsed = record.decisionJson ? parseJsonLoose(record.decisionJson) : null;
        console.log(
          "[loadAnalysis] djParsed.formulaLlmAgreement:",
          djParsed?.formulaLlmAgreement ?? "(未设置，将使用前端手动计算降级)",
        );
        if (djParsed?.formulaLlmAgreement != null) {
          decisionAgreementScore = Math.round(Number(djParsed.formulaLlmAgreement));
        } else {
          // V41 修复: 用 extractLlmField 解析 llmDecisionJson，兼容 AgentNode 包装格式
          // 旧代码直接用 JSON.parse(record.llmDecisionJson) 取 lj.action，
          // 但旧记录存储的是 {role, content: '{...}', node_id} 格式，lj.action 为 undefined。
          const ljAction = extractLlmField(llmDecisionJson, "action") as string | null;
          const ljStance = extractLlmField(llmDecisionJson, "stance") as string | null;
          const ljPositionPct = extractLlmField(llmDecisionJson, "positionPct") as number | null;
          const ljConfidence = extractLlmField(llmDecisionJson, "confidence") as number | null;
          console.log(
            "[loadAnalysis] llm fields - action:",
            ljAction,
            "positionPct:",
            ljPositionPct,
            "confidence:",
            ljConfidence,
          );
          // 后端未预计算时，前端自己算（兼容旧记录）
          // V45 修复: action 一致性评分精细化, 与后端 compute_decision_agreement 保持一致
          const norm = (s: string) => s.trim().toLowerCase().replace(/[\s/_\u3000]+/g, "");
          // action 一致性 (50分)
          const fa = djParsed?.action ? norm(String(djParsed.action)) : null;
          const laRaw = ljAction ?? ljStance;
          const la = laRaw ? norm(laRaw) : null;
          const isBuy = (s: string) => s.includes("买") || s.includes("增持");
          const isSell = (s: string) => s.includes("卖") || s.includes("减持");
          const isHold = (s: string) => s === "持有";
          const isWatch = (s: string) => s === "观望";
          const isUncertain = (s: string) => s.includes("不确定") || s.includes("未知");
          let actionScore = 25;
          if (fa && la) {
            if (fa === la) { actionScore = 50; }
            else if (isBuy(fa) && isBuy(la)) { actionScore = 35; }
            else if (isSell(fa) && isSell(la)) { actionScore = 35; }
            else if ((isHold(fa) && isWatch(la)) || (isHold(la) && isWatch(fa))) { actionScore = 15; }
            else if ((isHold(fa) || isWatch(fa)) && isUncertain(la)) { actionScore = 5; }
            else if ((isHold(la) || isWatch(la)) && isUncertain(fa)) { actionScore = 5; }
            else if (isWatch(fa) && isUncertain(la) || isWatch(la) && isUncertain(fa)) { actionScore = 10; }
            else { actionScore = 0; }
          }
          // positionPct 一致性 (30分)
          const fp = typeof djParsed?.positionPct === "number" ? djParsed.positionPct : null;
          const lp = ljPositionPct;
          let posScore = 15;
          if (fp !== null && lp !== null) {
            const diff = Math.abs(fp - lp);
            posScore = diff <= 5 ? 30 : diff <= 15 ? 20 : diff <= 30 ? 10 : 0;
          }
          // confidence 一致性 (20分)
          const fc = typeof djParsed?.confidence === "number" ? djParsed.confidence : null;
          const lc = ljConfidence;
          let confScore = 10;
          if (fc !== null && lc !== null) {
            const diff = Math.abs(fc - lc);
            confScore = diff <= 0.1 ? 20 : diff <= 0.2 ? 15 : diff <= 0.4 ? 8 : 0;
          }
          decisionAgreementScore = Math.round(actionScore + posScore + confScore);
        }
      } catch (e) {
        console.warn("[StockAnalysis] Failed to compute agreement score:", e);
      }
    }
    set({ llmDecisionJson, decisionAgreementScore });
    // [DQ] 关键诊断：标记即将进入 blackboardSnapshot 处理块
    console.log("[DQ] loadAnalysis before snapshot block", {
      hasSnapshot: !!record.blackboardSnapshot,
      snapshotLen: record.blackboardSnapshot?.length ?? 0,
    });
    if (record.blackboardSnapshot) {
      try {
        if (import.meta.env.DEV) {
          console.debug("[DQ] loadAnalysis enter snapshot branch", {
            snapshotLen: record.blackboardSnapshot.length,
          });
        }
        // 后端 axagent_stock_analysis::blackboard::build_blackboard_snapshot 会把
        // 节点 ID 重写为带前缀的 key(见 blackboard.rs:25-51):
        //   a-*        → report.{nodeId}
        //   trader     → report.investment-plan
        //   value-*    → value.*
        //   rule-check → rule_check.*
        //   data-quality → data_quality_summary
        //   raw-data   → raw.*
        // 其它节点保留原 nodeId。
        // 这里用本地解析把 snapshot 还原成结构化字段。
        const snap: Record<string, string> = JSON.parse(record.blackboardSnapshot);
        if (import.meta.env.DEV) {
          const dqValue = snap["data_quality_summary"];
          console.debug("[DQ] loadAnalysis snapshot", {
            snapshotLen: record.blackboardSnapshot.length,
            snapKeys: Object.keys(snap),
            hasDQKey: "data_quality_summary" in snap,
            dqValueType: dqValue !== undefined ? typeof dqValue : "(absent)",
            dqValueIsObj: dqValue !== null && typeof dqValue === "object",
            dqValueLen: typeof dqValue === "string" ? dqValue.length : null,
            dqValuePreview: typeof dqValue === "string" ? dqValue.slice(0, 200) : null,
          });
        }
        const reports: Record<string, string> = {};
        const debates: Array<{ round: number; bull: string; bear: string }> = [];
        const risks: Record<string, string> = {};
        const values: Record<string, string> = {};
        const ruleChecks: Record<string, string> = {};
        const raws: Record<string, string> = {};
        let dataQuality = "";
        for (const [key, value] of Object.entries(snap)) {
          if (key.startsWith("report.")) {
            // 统一与 live 模式 (handleAnalystReport / parseWorkflowResults) 保持一致：
            // 分析师节点 (a-*) 存入 analystReports 时去掉 a- 前缀，
            // grid 的 `const expertId = nodeId.slice(2)` 才能正确命中。
            // trader 节点后端映射为 report.investment-plan，已是非 a- 前缀，保留不变。
            const rawKey = key.slice(7);
            const normKey = rawKey.startsWith("a-") ? rawKey.slice(2) : rawKey;
            reports[normKey] = String(value);
          } else if (key.startsWith("debate.bull.round_")) {
            const round = parseInt(key.slice("debate.bull.round_".length));
            const bearKey = `debate.bear.round_${round}`;
            debates.push({
              round,
              bull: extractContent(value),
              bear: extractContent(snap[bearKey]),
            });
          } else if (key.startsWith("risk.")) {
            // 后端 blackboard.rs 对 risk-* / agg-risk / research-mgr 走 is_structured 分支,
            // snapshot value 是 AgentResult 包装({content, model, role, node_id, params}),
            // 真正的 LLM 输出在 content 字段里。extractContent 会优先取 content 字段并清理。
            // 与 live 模式 routeNodeOutput / parseWorkflowResults 行为一致。
            risks[key.slice(5)] = extractContent(value);
          } else if (key.startsWith("value.")) {
            // value.assessment 同理：AgentResult 包装,真正的价值评估 JSON 在 content 字段里。
            // extractContent 取 content 后,ValueAssessmentPanel.tryParseValueReport 才能正确解析。
            // key 去掉 "value." 前缀,直接作为 values 的键(测试期望 values["assessment"] 存在)。
            // 同时存 "value-investor" 别名以兼容 live 模式 parseWorkflowResults 的 stepId 命名。
            const vk = key.slice(6);
            const valueContent = extractContent(value);
            values[vk] = valueContent;
            if (vk === "assessment") {
              values["value-investor"] = valueContent;
            }
          } else if (key.startsWith("rule_check.")) {
            ruleChecks[key.slice("rule_check.".length)] = String(value);
          } else if (key === "data_quality_summary") {
            // V41 修复: 旧版 snapshot 可能是 CodeNode 包装对象
            //   {status, language, result: {grade, score, ...}, input_params, node_id, params}
            // 直接 String(对象) 会得到 "[object Object]"（15 字符），导致 DecisionBanner
            // 解析失败触发"数据质量诊断未渲染"降级面板。
            // 兼容三种格式：
            //   1. CodeNode 包装对象 → 提取 .result 字段并 JSON.stringify
            //   2. 已序列化的 JSON 字符串（新版 blackboard.rs 写入）→ 保留原样
            //   3. 其他对象 → JSON.stringify 避免 "[object Object]"
            let v: unknown = value;
            if (v && typeof v === "object" && !Array.isArray(v)) {
              const obj = v as Record<string, unknown>;
              if (obj.result != null) {
                v = obj.result;
              }
            }
            dataQuality = typeof v === "string" ? v : JSON.stringify(v ?? "");
          } else if (key.startsWith("raw.")) {
            raws[key.slice(4)] = String(value);
          }
          // ── 辩论子节点：bull-r1/bear-r1 等 → 构建 debate rounds ──
          if (/^bull-r\d+$/.test(key)) {
            const round = parseInt(key.slice("bull-r".length));
            const bearKey = `bear-r${round}`;
            if (!debates.find((d) => d.round === round)) {
              // 后端 blackboard.rs 对 bull-r* 走 is_structured 分支,
              // value 是 AgentResult 包装,extractContent 取 content 字段。
              const bullContent = extractContent(value);
              const bearContent = extractContent(snap[bearKey]);
              // 允许单边数据:如果一方为空可能是 LLM 失败/超时/存储异常,
              // 仍展示已有内容而非静默丢弃(同 live 模式 parseWorkflowResults 行为)
              debates.push({
                round,
                bull: bullContent,
                bear: bearContent,
              });
            }
          }
          // ── 汇总节点 agg-risk：result 是数组,每个子元素是独立 AgentResult 包装
          //   ({content, model, role, node_id, ...}),对应 risk-agg / risk-con / risk-neu。
          //   这里把它们展开成 3 个独立 entry,与 live 模式 parseWorkflowResults 行为一致
          //   (live 模式只存 4 个子节点 risk-agg/risk-con/risk-neu/research-mgr,不存 agg-risk)。
          //   子节点的 content 可能是 "json\n{...}" 格式,extractContent + tryBeautifyJson 会处理。
          if (key === "agg-risk") {
            const result = value && typeof value === "object"
              ? (value as Record<string, unknown>).result
              : null;
            if (Array.isArray(result)) {
              for (const sub of result) {
                if (sub && typeof sub === "object") {
                  const subNodeId = (sub as Record<string, unknown>).node_id;
                  if (typeof subNodeId === "string" && subNodeId.startsWith("risk-")) {
                    risks[subNodeId] = extractContent(sub);
                  }
                }
              }
            }
            // 不再保留 agg-risk 本身,避免重复渲染
            continue;
          }
          // ── 风险子节点：所有 risk-* 前缀节点（排除 aggregator 原始输出 risk-aggregated） ──
          // live 模式 routeNodeOutput(nodeId.startsWith("risk-")) 会捕获所有 risk-* 节点，
          // 这里必须匹配同样的集合，否则 risk-level 在回放时丢失。
          // risk-aggregated 是 agg-risk AggregatorNode 的 output_var，原始 JSON 含 result 数组
          // 与子节点数据重复，排除避免污染雷达图（已被 line 988 的 agg-risk 展开替代）。
          if (key.startsWith("risk-") && key !== "risk-aggregated") {
            risks[key] = extractContent(value);
          }
          // ── 投资组合经理输出 ──
          if (key === "research-mgr" && !risks["research-mgr"]) {
            risks["research-mgr"] = extractContent(value);
          }
          // ── 原始数据聚合（兼容旧版 raw-data 未映射到 raw. 的情况）──
          if (key === "raw-data" && !raws["combined"]) {
            raws["combined"] = String(value);
          }
        }
        // Debug: 输出 snapshot 中辩论/风险键及其内容长度，帮助诊断回放数据缺失
        if (import.meta.env.DEV) {
          const debateKeys = Object.keys(snap).filter(k => /^bull-r\d+$/.test(k) || /^bear-r\d+$/.test(k));
          const riskKeys = Object.keys(snap).filter(k => k.startsWith("risk-") || k === "research-mgr");
          console.debug(
            `[StockAnalysis] loadAnalysis debateKeys=${JSON.stringify(debateKeys)} riskKeys=${
              JSON.stringify(riskKeys)
            }`,
          );
        }
        // 后端 snapshot 由 HashMap 序列化,键的迭代顺序是 hash 顺序而非插入顺序,
        // bull-r1/bull-r2/bull-r3 三个键在 JSON 字符串里可能是 3/1/2 这种乱序。
        // 这里强制按 round 数字升序排序,保证前端 DebatePanel 按 1→2→3 顺序渲染。
        debates.sort((a, b) => a.round - b.round);
        // 决策输入诊断：把 snap 的 blackboard 键名归一化为 nodeId,
        // 再传给 buildDecisionInputsReport 提取 16 个 portfolio-mgr 上游节点的数据符合度
        const normalizedSnap: Record<string, unknown> = {};
        for (const [key, value] of Object.entries(snap)) {
          let nodeId = key;
          if (key === "report.investment-plan") {
            nodeId = "trader";
          } else if (key === "data_quality_summary") {
            nodeId = "data-quality";
          } else if (key.startsWith("report.")) {
            nodeId = key.slice(7);
          }
          // snap 的 value 是 string（已序列化的 JSON），尝试 parse 成对象让诊断函数能取字段
          if (typeof value === "string") {
            try {
              normalizedSnap[nodeId] = JSON.parse(value);
            } catch {
              normalizedSnap[nodeId] = value;
            }
          } else {
            normalizedSnap[nodeId] = value;
          }
        }
        const decisionInputsReport = buildDecisionInputsReport(normalizedSnap, {});
        if (import.meta.env.DEV) {
          console.debug("[DQ] loadAnalysis final", {
            dataQualityLen: dataQuality.length,
            dataQualityPreview: dataQuality.slice(0, 200),
            reportsCount: Object.keys(reports).length,
            debatesCount: debates.length,
            risksCount: Object.keys(risks).length,
          });
        }
        set({
          analystReports: reports,
          debateRounds: debates,
          riskAssessments: risks,
          valueAssessments: values,
          ruleCheckResults: ruleChecks,
          dataQualitySummary: dataQuality,
          rawData: raws,
          dataWarnings: [],
          decisionInputsReport,
        });

        // 历史分析回放：也缓存一次共识，让 RecommendationPanel 能用
        if (record.stockCode && Object.keys(reports).length > 0) {
          get().setStockCodeConsensus(
            record.stockCode,
            computeStockConsensus(reports, Date.now(), get().decision?.timeHorizon),
          );
        }
      } catch (e) {
        console.error("[StockAnalysis] Failed to restore blackboard snapshot:", e);
      }
    } else {
      // [DQ] 明确告知：snapshot 为空导致 dataQualitySummary 永远不会被填充
      console.warn(
        "[DQ] loadAnalysis blackboardSnapshot 为 null/空，跳过 dataQualitySummary 填充",
        { analysisId, analysisKind: record.analysisKind },
      );
      // V66 修复(2026-07-29): 设置 stale_record 标记的占位 JSON，让 UI 能识别并提示用户重跑。
      // 旧版静默降级导致用户不知道为什么看不到数据质量诊断。
      set({
        dataQualitySummary: JSON.stringify({
          grade: "N/A",
          score: 0,
          stale_record: true,
          summary: i18n.t("stockAnalysis.dataQuality.staleRecordSummary"),
        }),
      });
    }
  },

  rerunDecision: async (analysisId: string) => {
    try {
      set({ status: "loading", progressMessage: i18n.t("stockAnalysis.rerunDecision"), dataWarnings: [] });
      const result = await invoke<{
        analysis_id: string;
        decision: Record<string, unknown>;
        llm_decision_json: string | null;
        dashboardReport?: DashboardReport | null;
        dashboardMd?: string | null;
      }>(
        "rerun_decision",
        { analysisId },
      );
      // 从返回的 decision 中提取关键字段
      const d = result.decision;
      // 必须走 parseAction/parseRiskLevel 映射（后端 Rhai 输出中文"增持"/"中风险"）
      // 直接 String() + as 断言会绕过中文→英文枚举映射，导致 UI 显示"不确定"
      const action = parseAction(d.action);
      const riskLevel = parseRiskLevel(d.riskLevel);
      const decision: StockDecision = {
        action,
        positionPct: Number(d.positionPct ?? 0),
        confidence: Number(d.confidence ?? 0),
        decisionConfidence: d.decisionConfidence != null ? Number(d.decisionConfidence) : null,
        signalStrength: d.signalStrength != null ? Number(d.signalStrength) : null,
        riskLevel,
        stopLoss: Number(d.stopLossPct ?? 0),
        targetPrice: null,
        reasoning: String(d.reasoning ?? ""),
        timeHorizon: String(d.timeHorizon ?? "mid"),
        expectedHoldingDays: Number(d.expectedHoldingDays ?? 0),
        targetTimeframe: String(d.targetTimeframe ?? "1m"),
      };
      // 恢复 LLM 决策（trader 原始输出，rerun 不重跑 LLM 节点，从 DB 读回旧值）
      const llmDecisionJson = result.llm_decision_json ?? null;
      // 重算公式 vs LLM 一致性分数（新公式决策 vs 旧 LLM 决策）
      let decisionAgreementScore: number | null = null;
      if (llmDecisionJson) {
        try {
          const ljAction = extractLlmField(llmDecisionJson, "action") as string | null;
          const ljStance = extractLlmField(llmDecisionJson, "stance") as string | null;
          const ljPositionPct = extractLlmField(llmDecisionJson, "positionPct") as number | null;
          const ljConfidence = extractLlmField(llmDecisionJson, "confidence") as number | null;
          const norm = (s: string) => s.trim().toLowerCase().replace(/[\s/_\u3000]+/g, "");
          // action 一致性 (50分)
          const fa = d.action ? norm(String(d.action)) : null;
          const laRaw = ljAction ?? ljStance;
          const la = laRaw ? norm(laRaw) : null;
          const isBuy = (s: string) => s.includes("买") || s.includes("增持");
          const isSell = (s: string) => s.includes("卖") || s.includes("减持");
          const isHold = (s: string) => s === "持有";
          const isWatch = (s: string) => s === "观望";
          const isUncertain = (s: string) => s.includes("不确定") || s.includes("未知");
          let actionScore = 25;
          if (fa && la) {
            if (fa === la) { actionScore = 50; }
            else if (isBuy(fa) && isBuy(la)) { actionScore = 35; }
            else if (isSell(fa) && isSell(la)) { actionScore = 35; }
            else if ((isHold(fa) && isWatch(la)) || (isHold(la) && isWatch(fa))) { actionScore = 15; }
            else if ((isHold(fa) || isWatch(fa)) && isUncertain(la)) { actionScore = 5; }
            else if ((isHold(la) || isWatch(la)) && isUncertain(fa)) { actionScore = 5; }
            else if (isWatch(fa) && isUncertain(la) || isWatch(la) && isUncertain(fa)) { actionScore = 10; }
            else { actionScore = 0; }
          }
          // positionPct 一致性 (30分)
          const fp = typeof d.positionPct === "number" ? d.positionPct : null;
          const lp = ljPositionPct;
          let posScore = 15;
          if (fp !== null && lp !== null) {
            const diff = Math.abs(fp - lp);
            posScore = diff <= 5 ? 30 : diff <= 15 ? 20 : diff <= 30 ? 10 : 0;
          }
          // confidence 一致性 (20分)
          const fc = typeof d.confidence === "number" ? d.confidence : null;
          const lc = ljConfidence;
          let confScore = 10;
          if (fc !== null && lc !== null) {
            const diff = Math.abs(fc - lc);
            confScore = diff <= 0.1 ? 20 : diff <= 0.2 ? 15 : diff <= 0.4 ? 8 : 0;
          }
          decisionAgreementScore = Math.round(actionScore + posScore + confScore);
        } catch (e) {
          console.warn("一致性计算失败:", e);
        }
      }
      set({
        decision,
        llmDecisionJson,
        decisionAgreementScore,
        status: "completed",
        error: null,
        dataWarnings: [],
        dashboardReport: result.dashboardReport ?? null,
        dashboardMd: result.dashboardMd ?? null,
      });
    } catch (e) {
      console.error("[StockAnalysis] rerunDecision failed:", e);
      set({ status: "error", error: String(e) });
    }
  },

  dismissChatIndicator: () => {
    set({ chatIndicatorDismissed: true });
  },

  setReportLanguage: (lang: "zh" | "en") => {
    set({ reportLanguage: lang });
  },

  // R1 复盘→进化
  fetchEvolutionDashboard: async (asOfDate?: string | null) => {
    try {
      const data = await invoke<EvolutionDriftDashboard>("get_evolution_drift_dashboard", {
        asOfDate: asOfDate ?? null,
      });
      set({ evolutionDashboard: data, evolutionLastError: null });
    } catch (e) {
      set({ evolutionLastError: e instanceof Error ? e.message : String(e) });
      console.error("[EvolutionDrift] fetch dashboard failed:", e);
    }
  },

  recalcEvolutionNow: async (asOfDate?: string | null) => {
    set({ evolutionRecalculating: true, evolutionLastError: null });
    try {
      await invoke<{ written: number; currentWeights: Array<[string, string, number]> }>(
        "manual_recalc_strategy_weights",
        { asOfDate: asOfDate ?? null },
      );
      // 重算后立即拉一次新 dashboard
      await get().fetchEvolutionDashboard(asOfDate ?? null);
    } catch (e) {
      set({ evolutionLastError: e instanceof Error ? e.message : String(e) });
      console.error("[EvolutionDrift] manual recalc failed:", e);
    } finally {
      set({ evolutionRecalculating: false });
    }
  },

  loadRecoStrategyWeights: async () => {
    try {
      const data = await invoke<Record<string, number>>("get_reco_strategy_weights");
      return data;
    } catch (e) {
      console.warn("[EvolutionDrift] load reco strategy weights failed:", e);
      return {};
    }
  },

  fetchAgreementScoreHistory: async (limit = 50) => {
    set({ agreementScoreHistoryLoading: true });
    try {
      const data = await invoke<
        Array<
          {
            exitAt: number;
            agreementScore: number;
            stockCode: string;
            stockName: string;
            returnPct: number;
            wasCorrect: number;
          }
        >
      >(
        "get_agreement_score_history",
        { limit },
      );
      set({ agreementScoreHistory: data, agreementScoreHistoryLoading: false });
    } catch (e) {
      console.warn("[AgreementScore] 获取一致性趋势失败:", e);
      set({ agreementScoreHistoryLoading: false });
    }
  },

  // R2 组合监控
  fetchPortfolioDashboard: async (asOfDate?: string | null) => {
    try {
      const data = await invoke<PortfolioDashboard>("get_portfolio_dashboard", {
        asOfDate: asOfDate ?? null,
      });
      set({ portfolioDashboard: data, portfolioLastError: null });
    } catch (e) {
      set({ portfolioLastError: e instanceof Error ? e.message : String(e) });
      console.error("[PortfolioMonitor] fetch dashboard failed:", e);
    }
  },

  refreshPortfolioMetrics: async (asOfDate?: string | null) => {
    set({ portfolioRefreshing: true, portfolioLastError: null });
    try {
      await invoke<{
        metricsId: string;
        positionsSnapshotted: number;
        correlationPairsWritten: number;
        asOfDate: string | null;
      }>("refresh_portfolio_metrics", { asOfDate: asOfDate ?? null });
      await Promise.all([
        get().fetchPortfolioDashboard(asOfDate ?? null),
        get().fetchPortfolioCorrelations(asOfDate ?? null),
      ]);
    } catch (e) {
      set({ portfolioLastError: e instanceof Error ? e.message : String(e) });
      console.error("[PortfolioMonitor] refresh failed:", e);
    } finally {
      set({ portfolioRefreshing: false });
    }
  },

  fetchPortfolioCorrelations: async (asOfDate?: string | null) => {
    try {
      const data = await invoke<PortfolioCorrelationCell[]>(
        "get_portfolio_correlations",
        { asOfDate: asOfDate ?? null },
      );
      set({ portfolioCorrelations: data, portfolioCorrelationsError: null });
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      console.error("[PortfolioMonitor] fetch correlations failed:", e);
      set({ portfolioCorrelationsError: msg });
    }
  },

  checkPositionLimits: async (
    stockCode: string,
    proposedShares: number,
    proposedPrice: number,
  ) => {
    try {
      return await invoke<PositionLimitsCheck>("check_position_limits", {
        stockCode,
        proposedShares,
        proposedPrice,
      });
    } catch (e) {
      console.error("[PortfolioMonitor] check position limits failed:", e);
      return null;
    }
  },

  bumpWatchlistVersion: () => {
    set((s) => ({ watchlistVersion: s.watchlistVersion + 1 }));
  },

  pushTimelineNode: (node) => {
    set((s) => {
      // 同 id 视为 update（去重），避免重复推送
      const idx = s.timeline.findIndex((n) => n.id === node.id);
      if (idx >= 0) {
        const next = s.timeline.slice();
        next[idx] = { ...next[idx], ...node };
        return { timeline: next };
      }
      return { timeline: [...s.timeline, node] };
    });
  },

  updateTimelineNode: (id, patch) => {
    set((s) => {
      const idx = s.timeline.findIndex((n) => n.id === id);
      if (idx < 0) { return {}; }
      const next = s.timeline.slice();
      next[idx] = { ...next[idx], ...patch };
      return { timeline: next };
    });
  },

  clearTimeline: () => {
    set({ timeline: [] });
  },

  setHighlightedPanel: (key) => {
    set({ highlightedPanel: key });
  },

  setStockCodeConsensus: (stockCode, consensus) => {
    if (!stockCode) { return; }
    set((s) => ({
      stockCodeConsensus: { ...s.stockCodeConsensus, [stockCode]: consensus },
    }));
  },

  setEvidenceReport: (stockCode, report) => {
    if (!stockCode) { return; }
    set((s) => ({
      evidenceReport: { ...s.evidenceReport, [stockCode]: report },
    }));
  },

  setKlinePeriod: (period: string) => {
    set({ klinePeriod: period });
  },

  setKlineAdj: (adj) => {
    set({ klineAdj: adj });
  },

  setAutoRefresh: (enabled: boolean) => {
    set({ autoRefresh: enabled });
  },

  toggleIndicator: (key) => {
    set((s) => ({
      klineIndicators: { ...s.klineIndicators, [key]: !s.klineIndicators[key] },
    }));
  },

  toggleSidebarPanel: (key) => {
    set((s) => ({
      sidebarCollapsed: { ...s.sidebarCollapsed, [key]: !s.sidebarCollapsed[key] },
    }));
    // Persist to localStorage（延迟写入，避免高频 toggle 阻塞主线程）
    if (typeof window !== "undefined") {
      // 使用 requestIdleCallback 或 setTimeout 延迟写入
      const schedule = window.requestIdleCallback || ((cb: IdleRequestCallback) => setTimeout(cb, 200));
      schedule(() => {
        try {
          window.localStorage.setItem("ax_sidebar_collapsed", JSON.stringify(get().sidebarCollapsed));
        } catch { /* noop */ }
      });
    }
  },

  reset: () => {
    const { _unlisten, _searchTimer } = get();
    if (_unlisten) {
      _unlisten();
    }
    if (_searchTimer) {
      clearTimeout(_searchTimer);
    }
    set({
      ...initialState,
      _searchTimer: null,
      _unlisten: null,
      _lastEarningsFetch: null,
      _dryRunCache: null,
      llmStatus: "unknown" as const,
    });
  },

  setAsOfDate: (date) => set({ asOfDate: date }),
  setMode: (mode) => set({ mode }),
  setViolations: (violations) => set({ violations }),
  setShowEarningsOnChart: (show) => set({ showEarningsOnChart: show }),

  // Phase 10: Experiment mode actions
  setDecisionMode: (mode) => set({ decisionMode: mode }),
  pushExperiment: (record) => set((s) => ({ experiments: [...s.experiments, record] })),
  clearExperiments: () => set({ experiments: [] }),

  /**
   * R3-B: 拉取财报披露事件列表,在 K 线图上叠加图标。
   * 拉取失败时清空列表(避免显示陈旧数据),但不影响 K 线本身渲染。
   */
  fetchEarningsEvents: async (stockCode: string) => {
    if (!stockCode) {
      set({ earningsEvents: [] });
      return;
    }
    // 切换股票:清旧数据再拉新数据,避免上一只股票的事件短暂闪烁在 UI 上
    set({ earningsLoading: true });
    try {
      const events = await invoke<EarningsEvent[]>("get_earnings_calendar", { stockCode });
      set({ earningsEvents: Array.isArray(events) ? events : [], earningsLoading: false, earningsError: null });
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      console.error("[StockAnalysis] Failed to fetch earnings events:", e);
      set({ earningsEvents: [], earningsLoading: false, earningsError: msg });
    }
  },

  /**
   * P2-6: 拉取 T+0 自动重跑配置
   */
  fetchT0Config: async () => {
    try {
      const cfg = await invoke<{
        enabled: boolean;
        changePctThreshold: number | null;
        turnoverRateThreshold: number | null;
        minIntervalMinutes: number;
      }>("get_t0_config");
      set({
        t0Config: {
          enabled: !!cfg.enabled,
          changePctThreshold: cfg.changePctThreshold,
          turnoverRateThreshold: cfg.turnoverRateThreshold,
          minIntervalMinutes: cfg.minIntervalMinutes,
        },
      });
    } catch (e) {
      console.warn("[StockAnalysis] Failed to fetch t0 config:", e);
    }
  },

  /**
   * P2-6: 更新 T+0 配置 (partial merge + 立即同步到后端)
   */
  setT0Config: async (patch) => {
    const cur = get().t0Config;
    const next = { ...cur, ...patch };
    set({ t0Config: next, t0Loading: true });
    try {
      await invoke("set_t0_config", { config: next });
    } catch (e) {
      console.error("[StockAnalysis] Failed to set t0 config:", e);
    } finally {
      set({ t0Loading: false });
    }
  },

  setupEventListener: async () => {
    const existing = get()._unlisten;
    if (existing) { return; }

    // Restore sidebar collapse state from localStorage (one-time)
    if (typeof window !== "undefined") {
      try {
        const saved = window.localStorage.getItem("ax_sidebar_collapsed");
        if (saved) { set({ sidebarCollapsed: JSON.parse(saved) }); }
      } catch { /* noop */ }
    }

    // 使用数组收集所有已注册的 unlisten 函数，即使中途失败也能统一清理
    const unlisteners: Array<() => void> = [];
    const unlistenAll = () => {
      for (const u of unlisteners) {
        try {
          u();
        } catch { /* noop */ }
      }
    };

    // Bug 14 修复: 把 _unlisten = unlistenAll 提前到第一个 try 之前。
    // 旧实现 4 个 listen 都在 try/catch 里,任意一个抛异常时:
    //   - catch 命中后,执行流跳走,line 1333 的 set _unlisten 永远不会执行
    //   - 已成功注册的 listen (push 到 unlisteners[]) 永久失去引用 → 泄漏
    // 现在 _unlisten 在最开始就指向 unlistenAll,即使后续 listen 失败,
    // 下次 setupEventListener 进入时 existing 仍是非空值,不会再注册新监听,
    // 而且 cancelAnalysis / reset / 下次 startAnalysis 调 _unlisten() 时
    // 已注册的部分也会被清理。
    set({ _unlisten: unlistenAll });

    // ── 共享辅助函数（在 workflow-step-done 内使用） ──

    /** 合并辩论轮次（去重 bull/bear 重复逻辑） */
    function updateDebateRound(
      nodeId: string,
      side: "bull" | "bear",
      text: string,
    ) {
      const round = nodeId === `${side}-researcher` ? 1 : parseInt(nodeId.slice(`${side}-r`.length), 10);
      const debates = [...get().debateRounds];
      const idx = debates.findIndex((d) => d.round === round);
      if (idx >= 0) {
        debates[idx] = { ...debates[idx], [side]: text };
      } else {
        const entry: { round: number; bull: string; bear: string } = { round, bull: "", bear: "" };
        entry[side] = text;
        debates.push(entry);
      }
      debates.sort((a, b) => a.round - b.round);
      set({ debateRounds: debates });
    }

    /** 推送 timeline 节点（失败或完成共用，靠 status 区分） */
    function pushNodeTimeline(
      nodeId: string,
      status: "done" | "failed",
      summary: string,
    ) {
      const phase = inferTimelinePhase(nodeId);
      if (!phase) { return; }
      get().pushTimelineNode({
        id: nodeId,
        phase,
        agentId: nodeId,
        agentName: agentDisplayName(nodeId),
        title: agentDisplayName(nodeId),
        summary,
        confidence: status === "done" ? 0.5 : 0,
        status,
        evidenceRefs: inferEvidenceRefs(nodeId),
        startedAt: Date.now(),
        finishedAt: Date.now(),
      });
    }

    /** 解析分析师节点（a-* 开头，排除辩论） */
    function handleAnalystReport(nodeId: string, text: string) {
      if (nodeId.startsWith("a-") && !nodeId.includes("bull") && !nodeId.includes("bear")) {
        const expertId = nodeId.slice(2);
        // Bug #P0 修复: 如果节点完成但提取的内容为空（如只有工具标签被清理后为空），
        // 设置降级内容，避免 UI 卡片一直显示"等待中"
        const safeText = text && text.trim().length > 0
          ? text
          : JSON.stringify({
            report: i18n.t("stockAnalysis.analystReport.noAnalysisData"),
            verdict: {
              verdict: i18n.t("stockAnalysis.analystReport.verdictInsufficient"),
              confidence: 0,
              bull_score: 0,
              bear_score: 0,
              position_pct: 0,
            },
            __untrusted: true,
            __empty_fallback: true,
          });
        set({ analystReports: { ...get().analystReports, [expertId]: reconstructVerdictTag(safeText) } });
        return true;
      }
      return false;
    }

    /** 按节点类型路由输出到对应 store 字段 */
    // V41 修复: 新增 rawOutput 参数，让 CodeNode 节点（data-quality / rule-check 等）
    // 能从包装对象 {status, language, result, ...} 中提取真正的 result 字段，
    // 避免 store 存的是包装对象的 JSON 字符串（深嵌套一层，导致 DecisionBanner 解析失败）。
    function routeNodeOutput(nodeId: string, text: string, rawOutput?: unknown): void {
      const s = get();
      if (handleAnalystReport(nodeId, text)) { return; }

      if (nodeId === "bull-researcher" || (nodeId.startsWith("bull-r") && nodeId !== "bull-researcher")) {
        updateDebateRound(nodeId, "bull", text);
      } else if (nodeId === "bear-researcher" || (nodeId.startsWith("bear-r") && nodeId !== "bear-researcher")) {
        updateDebateRound(nodeId, "bear", text);
      } else if (nodeId.startsWith("risk-") || nodeId === "research-mgr") {
        set({ riskAssessments: { ...s.riskAssessments, [nodeId]: text } });
      } else if (nodeId === "trader") {
        set({ analystReports: { ...s.analystReports, "investment-plan": text } });
      } else if (nodeId === "portfolio-mgr") {
        const parsed = tryParseDecision(text);
        // Bug #P1-7: 决策解析失败时不构造假 HOLD 决策。
        // 保持 decision=null 让 workflow-completed 用三层回退解析。
        if (parsed) {
          set({ decision: parsed });
        } else {
          console.warn(
            "[StockAnalysis] workflow-step-done: portfolio-mgr decision parse failed, deferring to workflow-completed",
          );
        }
      } else if (nodeId === "value-investor") {
        set({ valueAssessments: { ...s.valueAssessments, [nodeId]: text } });
      } else if (nodeId === "data-quality") {
        // V41 修复: data-quality 是 CodeNode + Rhai，原始 output 形如
        //   {status, language, result: {grade, score, diagnostics, ...}, input_params, node_id, params}
        // extractContent 收到的 raw 会 JSON.stringify 整个包装对象，让 text 变成包装对象的 JSON。
        // 真正的诊断报告在 raw.result 字段中，DecisionBanner 用 JSON.parse(text) 解析时找不到顶层
        // grade 字段（grade 在嵌套的 .result 里）→ 返回 null → 触发"数据质量诊断未渲染"降级面板。
        // 这里优先从 rawOutput 提取 result，让 store 存的是纯诊断报告 JSON。
        let content = text;
        const raw = (rawOutput ?? null) as Record<string, unknown> | null;
        if (raw && typeof raw === "object" && raw.result != null) {
          const r = raw.result;
          content = typeof r === "string" ? r : JSON.stringify(r);
        }
        if (import.meta.env.DEV) {
          console.debug("[DQ] routeNodeOutput data-quality", {
            textLen: text.length,
            rawOutputType: rawOutput ? typeof rawOutput : "(null)",
            rawHasResult: raw ? "result" in raw : false,
            resultType: raw && raw.result ? typeof raw.result : null,
            resultIsObj: raw && raw.result && typeof raw.result === "object",
            contentLen: content.length,
            contentPreview: content.slice(0, 300),
            setTo: content.slice(0, 200),
          });
        }
        set({ dataQualitySummary: content });
      } else if (nodeId === "raw-data") {
        set({ rawData: { ...s.rawData, [nodeId]: text } });
      } else if (nodeId === "rule-check") {
        // rule-check 是 AgentNode（LLM 调用），输出是 AgentResult 包装 {role, content, ...}，
        // extractContent 已经从 .content 字段取出 LLM 输出的 JSON 字符串，text 已是正确内容。
        set({ ruleCheckResults: { ...s.ruleCheckResults, [nodeId]: text } });
      }
    }

    // 手动 try-catch 包装每个 listen，一个失败不影响其他的
    try {
      const u1 = await listen<{
        workflowId: string;
        nodeId: string;
        status: string;
        totalNodes: number;
        completedNodes: number;
        output?: unknown;
        error?: string;
      }>("workflow-step-done", (event) => {
        const { nodeId, status, totalNodes, completedNodes, output, error } = event.payload;

        // Handler 1: 进度 & 阶段 & 失败节点
        const stage = inferStage(nodeId);
        if (stage >= 0) { set({ currentStage: stage }); }
        const pct = totalNodes > 0
          ? Math.round((completedNodes / totalNodes) * 100)
          : get().progressPct;
        set({
          progressPct: Math.max(pct, get().progressPct),
          progressMessage: status === "completed"
            ? i18n.t("stockAnalysis.progress.stepDone", { name: nodeId })
            : status === "failed"
            ? i18n.t("stockAnalysis.progress.stepRetrying", { name: nodeId })
            : i18n.t("stockAnalysis.progress.stepRunning", { name: nodeId }),
          failedNodes: status === "failed"
            ? [...get().failedNodes, nodeId]
            : get().failedNodes,
          failedNodeErrors: status === "failed" && error
            ? { ...get().failedNodeErrors, [nodeId]: error }
            : get().failedNodeErrors,
        });

        // Handler 2: 数据源降级检测
        if (
          status === "completed"
          && (nodeId === "t-news-data" || nodeId === "t-sentiment-data" || nodeId === "t-catalyst-data")
        ) {
          const outputValue = event.payload.output;
          const isEmpty = outputValue == null
            || (typeof outputValue === "string" && (outputValue === "[]" || outputValue.trim() === ""))
            || (Array.isArray(outputValue) && outputValue.length === 0);
          if (isEmpty) {
            const label = nodeId === "t-news-data"
              ? i18n.t("stockAnalysis.dataWarning.news")
              : nodeId === "t-sentiment-data"
              ? i18n.t("stockAnalysis.dataWarning.sentiment")
              : i18n.t("stockAnalysis.dataWarning.announcement");
            // M16 守卫：dataWarnings 可能为 undefined（旧快照路径），兜底为空数组避免 .includes 崩溃
            const warnings = get().dataWarnings ?? [];
            const msg = i18n.t("stockAnalysis.dataWarning.emptyData", { label });
            if (!warnings.includes(msg)) {
              set({ dataWarnings: [...warnings, msg] });
            }
          }
        }

        // Handler 3: 失败节点 timeline 推送
        if (status === "failed" && error) {
          pushNodeTimeline(nodeId, "failed", error);
        }

        // Handler 4: 完成节点 → timeline + 未来引用检测 + 输出路由
        if (status === "completed" && output != null) {
          const text = extractContent(output);
          const s = get();

          pushNodeTimeline(nodeId, "done", text.slice(0, 200));

          // 未来引用检测（仅 as-of 模式）
          const asOf = s.asOfDate;
          if (asOf) {
            const newViolations = detectFutureReferencesForNode(nodeId, text, asOf);
            if (newViolations.length > 0) {
              set({ violations: [...s.violations, ...newViolations] });
            }
          }

          // DQ 诊断：记录 data-quality 节点的原始输出结构
          if (nodeId === "data-quality" && import.meta.env.DEV) {
            const outputObj = output && typeof output === "object";
            console.debug("[DQ] workflow-step-done", {
              status,
              outputType: output ? typeof output : "(null)",
              outputKeys: outputObj ? Object.keys(output as object) : null,
              hasResult: outputObj ? "result" in (output as Record<string, unknown>) : null,
              resultType: outputObj && ((output as Record<string, unknown>).result)
                ? typeof (output as Record<string, unknown>).result
                : null,
              textLen: text.length,
              textPreview: text.slice(0, 300),
            });
          }

          routeNodeOutput(nodeId, text, output);
        }
      });
      unlisteners.push(u1);
    } catch (e) {
      console.error("[StockAnalysis] Failed to listen workflow-step-done:", e);
    }

    try {
      const u2 = await listen<{
        workflowId: string;
        results: Record<string, unknown>;
        output?: unknown;
        // 后端 core.rs 在 emit workflow-completed 时附加 dashboardReport/dashboardMd，
        // 让 dashboard tab 在正常完成路径立即填充，不再依赖 rerunDecision。
        dashboardReport?: DashboardReport | null;
        dashboardMd?: string | null;
      }>("workflow-completed", (event) => {
        const { results, output, dashboardReport: wfDashboardReport, dashboardMd: wfDashboardMd } = event.payload;

        // 优先从 portfolio-mgr 节点结果中提取决策（与分析页一致）
        let decision: StockDecision | null = null;
        const pmRaw = results["portfolio-mgr"];
        if (pmRaw) {
          decision = extractDecision(pmRaw);
          // P0 防御: extractDecision 走 normalizeDecision 的复杂 source 选择逻辑,
          // 可能在边缘情况下匹配到错误的 source 路径。增加一条更直接的路径:
          // 从 pmRaw.result (CodeNode 容器的原始 Rhai 输出) 直接提取。
          // 仅在 extractDecision 返回 null 或置信度为 0 时补跑, 避免覆盖正确值。
          if (
            (!decision || decision.confidence === 0)
            && pmRaw && typeof pmRaw === "object"
          ) {
            const pmObj = pmRaw as Record<string, unknown>;
            if (pmObj.result && typeof pmObj.result === "object" && !Array.isArray(pmObj.result)) {
              const direct = normalizeDecision(pmObj.result as Record<string, unknown>);
              if (direct && (direct.confidence > 0 || !decision)) {
                if (decision) {
                  console.warn(
                    "[StockAnalysis] workflow-completed extractDecision 返回置信度 0, direct result path 覆盖:",
                    { oldConf: decision.confidence, newConf: direct.confidence, newAction: direct.action },
                  );
                }
                decision = direct;
              }
            }
          }
        }
        console.log("[workflow-completed] portfolio-mgr raw:", {
          pmRaw: pmRaw ? JSON.stringify(pmRaw).slice(0, 2000) : "(undefined)",
          pmRawKeys: pmRaw && typeof pmRaw === "object" ? Object.keys(pmRaw as object) : null,
          hasResult: pmRaw && typeof pmRaw === "object" ? !!(pmRaw as Record<string, unknown>).result : null,
          decisionConfidence: decision?.confidence,
          decisionAction: decision?.action,
        });
        // P0 防御日志: 当 event 提取的置信度为 0 但 pmRaw 有 result 时, dump 完整结构
        if (decision?.confidence === 0 && pmRaw && typeof pmRaw === "object") {
          const pmObj = pmRaw as Record<string, unknown>;
          if (pmObj.result && typeof pmObj.result === "object") {
            console.warn(
              "[StockAnalysis] workflow-completed 置信度为 0 但 pmRaw.result 存在, dump 结构:",
              {
                resultKeys: Object.keys(pmObj.result as object),
                resultPreview: JSON.stringify(pmObj.result).slice(0, 1000),
              },
            );
          }
        }

        // 回退：从 output 中提取 decision
        if (!decision && output !== undefined) {
          decision = extractDecision(output);
        }

        // 回退：从 parseWorkflowResults 中获取
        const parsed = parseWorkflowResults(results);
        if (!decision) {
          decision = parsed.decision;
        }

        // 修复 #5: 三层回退全失败时记录警告，避免静默丢失决策
        if (!decision) {
          const pmDump = pmRaw
            ? {
              keys: Object.keys(pmRaw as object),
              hasResult: !!(pmRaw as Record<string, unknown>).result,
              hasOutput: !!(pmRaw as Record<string, unknown>).output,
              hasSource: !!((pmRaw as Record<string, unknown>).source),
              type: typeof pmRaw,
            }
            : null;
          const outputDump = output !== undefined
            ? { type: typeof output, keys: typeof output === "object" ? Object.keys(output as object) : null }
            : null;
          console.warn(
            "[StockAnalysis] workflow-completed 三层回退均未能解析决策",
            { hasPortfolioMgr: !!pmRaw, pmDump, hasOutput: output !== undefined, outputDump },
          );
          if (pmRaw && typeof pmRaw === "object") {
            const pmObj = pmRaw as Record<string, unknown>;
            console.log("[StockAnalysis] pmRaw 完整内容:", JSON.stringify(pmRaw, null, 2).slice(0, 2000));
            // 额外输出 output 和 source 字段（{node_id, output, source, status} 格式诊断）
            if (pmObj.output) {
              console.log("[StockAnalysis] pmRaw.output:", JSON.stringify(pmObj.output, null, 2).slice(0, 1000));
            }
            if (pmObj.source) {
              console.log("[StockAnalysis] pmRaw.source:", pmObj.source);
            }
          }
        }

        // 增量合并 workflow-step-done 已填充的数据，避免覆盖实时进度
        const s = get();
        // 辩论轮次合并: streaming 数据 (workflow-step-done) 与批量结果 (workflow-completed)
        // 可能因事件到达顺序不一致而不同步。merge: 保留两边的数据,解析轮次补充流轮次的空缺。
        const mergedDebateRounds = [...s.debateRounds];
        for (const pr of parsed.debateRounds) {
          const existing = mergedDebateRounds.find((mr) => mr.round === pr.round);
          if (existing) {
            // 不覆盖已有的流式数据(更完整),仅补充空隙
            if (!existing.bull && pr.bull) { existing.bull = pr.bull; }
            if (!existing.bear && pr.bear) { existing.bear = pr.bear; }
          } else {
            mergedDebateRounds.push(pr);
          }
        }

        // V40 修复: 从 worklow 结果中提取 LLM 决策 JSON（trader 节点），
        // 避免 llmDecisionJson 在实时分析完成后一直为 null，
        // 导致 DecisionComparisonPanel 显示"LLM视角不可用"。
        let llmDecisionJson: string | null = null;
        let decisionAgreementScore: number | null = null;
        const traderRaw = results["trader"];
        // V41 修复: 兼容 results["trader"] 的三种可能格式：
        //   1. AgentNode 包装: {role:"trader", content:"{...}"} → 取 .content
        //   2. 纯 JSON 字符串: "{\"action\":\"买入\",...}"      → 直接用
        //   3. 纯 JSON 对象: {action:"买入",...}              → JSON.stringify
        if (traderRaw) {
          if (typeof traderRaw === "string") {
            // 格式2: 纯 JSON 字符串
            llmDecisionJson = traderRaw;
          } else if (typeof traderRaw === "object") {
            const traderObj = traderRaw as Record<string, unknown>;
            const content = traderObj.content;
            if (typeof content === "string" && content.length > 0) {
              // 格式1: AgentNode 包装，content 是内层 JSON 字符串
              llmDecisionJson = content;
            } else if (content && typeof content === "object") {
              llmDecisionJson = JSON.stringify(content);
            } else {
              // 格式3 (兜底): traderRaw 本身就是决策对象
              llmDecisionJson = JSON.stringify(traderRaw);
            }
          }
        }
        // 调试日志: 验证 results["trader"] 的实际格式
        console.log(
          "[workflow-completed] traderRaw type:",
          typeof traderRaw,
          traderRaw ? Object.keys(traderRaw as object).join(",") : "null/undefined",
        );
        console.log("[workflow-completed] llmDecisionJson:", llmDecisionJson);
        // 清理 LLM 输出中的 markdown 代码围栏（```json ... ```），部分 model 会在 JSON 外包裹这些标记
        if (llmDecisionJson) {
          llmDecisionJson = llmDecisionJson
            .replace(/^`{3}(?:json)?\s*/m, "") // 去掉开头的 ```json 或 ```
            .replace(/`{3}\s*$/m, ""); // 去掉结尾的 ```
        }
        // 用同一份 llmDecisionJson 计算一致性分数
        if (llmDecisionJson && decision) {
          try {
            const fj = decision as unknown as Record<string, unknown>;
            const lj = JSON.parse(llmDecisionJson);
            const norm = (s: string) => s.trim().toLowerCase().replace(/[\s/_\u3000]+/g, "");
            const fa = fj.action ? norm(String(fj.action)) : null;
            const la = (lj.action ?? lj.stance) ? norm(String(lj.action ?? lj.stance)) : null;
            let actionScore = 25;
            if (fa && la) {
              const isBuy = (s: string) => s.includes("买") || s.includes("增持");
              const isSell = (s: string) => s.includes("卖") || s.includes("减持");
              const isHold = (s: string) => s === "持有";
              const isWatch = (s: string) => s === "观望";
              const isUncertain = (s: string) => s.includes("不确定") || s.includes("未知");
              if (fa === la) { actionScore = 50; }
              else if (isBuy(fa) && isBuy(la)) { actionScore = 35; }
              else if (isSell(fa) && isSell(la)) { actionScore = 35; }
              else if ((isHold(fa) && isWatch(la)) || (isHold(la) && isWatch(fa))) { actionScore = 15; }
              else if ((isHold(fa) || isWatch(fa)) && isUncertain(la)) { actionScore = 5; }
              else if ((isHold(la) || isWatch(la)) && isUncertain(fa)) { actionScore = 5; }
              else if (isWatch(fa) && isUncertain(la) || isWatch(la) && isUncertain(fa)) { actionScore = 10; }
              else { actionScore = 0; }
            }
            const fp = typeof fj.positionPct === "number" ? fj.positionPct : null;
            const lp = typeof lj.positionPct === "number" ? lj.positionPct : null;
            let posScore = 15;
            if (fp !== null && lp !== null) {
              const diff = Math.abs(fp - lp);
              posScore = diff <= 5 ? 30 : diff <= 15 ? 20 : diff <= 30 ? 10 : 0;
            }
            const fc = typeof fj.confidence === "number" ? fj.confidence : null;
            const lc = typeof lj.confidence === "number" ? lj.confidence : null;
            let confScore = 10;
            if (fc !== null && lc !== null) {
              const diff = Math.abs(fc - lc);
              confScore = diff <= 0.1 ? 20 : diff <= 0.2 ? 15 : diff <= 0.4 ? 8 : 0;
            }
            decisionAgreementScore = Math.round(actionScore + posScore + confScore);
          } catch (e) {
            console.warn("[StockAnalysis] Failed to compute LLM agreement:", e);
          }
        }

        // 第 3 步: 数据质量诊断日志
        if (import.meta.env.DEV) {
          const dqRaw = results["data-quality"];
          const dqKeys = dqRaw && typeof dqRaw === "object" ? Object.keys(dqRaw as object) : null;
          console.debug("[DQ] workflow-completed", {
            hasDataQualityKey: "data-quality" in results,
            resultKeys: Object.keys(results),
            dqRawType: dqRaw ? typeof dqRaw : "(absent)",
            dqRawKeys: dqKeys,
            dqHasResult: dqKeys ? dqKeys.includes("result") : null,
            dqResultType: dqKeys && dqRaw ? typeof (dqRaw as Record<string, unknown>).result : null,
            parsedDQLen: parsed.dataQualitySummary.length,
            parsedDQPreview: parsed.dataQualitySummary.slice(0, 200),
            streamingDQ: s.dataQualitySummary ? s.dataQualitySummary.slice(0, 200) : "(empty)",
          });
        }

        set({
          analystReports: { ...s.analystReports, ...parsed.analystReports },
          debateRounds: mergedDebateRounds,
          riskAssessments: { ...s.riskAssessments, ...parsed.riskAssessments },
          valueAssessments: { ...s.valueAssessments, ...parsed.valueAssessments },
          ruleCheckResults: { ...s.ruleCheckResults, ...parsed.ruleCheckResults },
          dataQualitySummary: parsed.dataQualitySummary || s.dataQualitySummary,
          rawData: { ...s.rawData, ...parsed.rawData },
          // V55: 合并 strict_mode 兜底节点标记（用于红色"数据异常"警告横幅）
          untrustedNodes: { ...s.untrustedNodes, ...parsed.untrustedNodes },
          // 决策输入诊断：从 workflow results 提取 portfolio-mgr 上游 16 个节点的数据符合度
          // 不持久化，纯前端展示，让用户检查决策数据是否齐全
          decisionInputsReport: buildDecisionInputsReport(results, parsed.untrustedNodes),
          decision,
          llmDecisionJson,
          decisionAgreementScore,
          status: "completed",
          progressMessage: i18n.t("stockAnalysis.progress.completed"),
          progressPct: 100,
          currentStage: 4,
          // 后端在 workflow-completed 事件中携带 dashboardReport/dashboardMd，
          // 正常分析完成路径也立即填充 dashboard，无需用户手动点"重跑决策"。
          // 后端为 null（dashboard 构建失败）时回退到 store 现有值，避免覆盖。
          dashboardReport: wfDashboardReport ?? s.dashboardReport,
          dashboardMd: wfDashboardMd ?? s.dashboardMd,
          // #21 工作流成功完成，重置自动重试计数，允许下次失败再次重试
          _workflowErrorRetries: 0,
        });

        // ── P0 防御: DB 决策权威覆盖 ──
        // 历史反复出现的问题: 工作流执行完成后 UI 显示置信度 0%, 但重新以历史记录打开
        // 该分析置信度正常(如 19%)。代码审查表明两条路径都应提取到相同的正确值,
        // 但可能存在极难重现的 IPC 序列化/竞态条件导致事件 payload 的决策提取异常。
        // 后端在 emit workflow-completed 之前已完成 DB 写入, 这里异步查一次 DB,
        // 如果 DB 决策置信度高于事件提取值, 用 DB 值覆盖, 确保 UI 显示与历史记录一致。
        if (get().analysisId) {
          (async () => {
            try {
              const persisted = await invoke<{
                decisionJson: string | null;
              }>("get_stock_analysis", { analysisId: get().analysisId });
              if (persisted?.decisionJson) {
                const dbRaw = parseJsonLoose(persisted.decisionJson);
                if (dbRaw) {
                  const dbDecision = normalizeDecision(dbRaw);
                  const eventConf = get().decision?.confidence ?? 0;
                  const dbConf = dbDecision?.confidence ?? 0;
                  if (dbDecision && dbConf > eventConf + 3) {
                    console.warn(
                      "[StockAnalysis] workflow-completed DB 决策置信度高于事件提取值, 使用 DB 值覆盖:",
                      { eventConf, dbConf, eventDecision: get().decision?.action, dbDecision: dbDecision.action },
                    );
                    set({ decision: dbDecision });
                  } else if (dbDecision && eventConf > dbConf + 3) {
                    // 反向差异也记录, 帮助诊断
                    console.warn(
                      "[StockAnalysis] workflow-completed 事件置信度高于 DB:",
                      { eventConf, dbConf, analysisId: get().analysisId },
                    );
                  }
                }
              }
            } catch (e) {
              console.warn("[StockAnalysis] workflow-completed DB 决策覆盖检查失败:", e);
            }
          })();
        }

        // 荐股 ↔ 分析师交叉验证：把本次的分析师投票结果缓存到 stockCodeConsensus
        // RecommendationPanel 会读取这个缓存来提示用户"推荐与共识是否一致"。
        const stockCode = get().stockCode;
        if (stockCode && parsed.analystReports && Object.keys(parsed.analystReports).length > 0) {
          const horizon = get().decision?.timeHorizon;
          // 先算旧版共识作为 fallback
          const consensus = computeStockConsensus(parsed.analystReports, undefined, horizon);
          get().setStockCodeConsensus(stockCode, consensus);
          // 异步尝试证据驱动共识（不阻塞，失败不影响旧版结果）
          (async () => {
            try {
              const { computeEvidenceDrivenConsensus } = await import("@/lib/stock-analysis-utils");
              const evidenceResult = await computeEvidenceDrivenConsensus(
                parsed.analystReports,
                undefined as unknown as import("@/lib/stock-analysis-utils").MarketRegimeInfo, // marketRegime will be fetched inside
                horizon,
                null,
              );
              if (evidenceResult.evidenceReport) {
                get().setEvidenceReport(stockCode, evidenceResult.evidenceReport);
                // 如果证据驱动共识与旧版不同，用新版覆盖
                get().setStockCodeConsensus(stockCode, evidenceResult);
              }
            } catch { /* 静默失败，旧版结果兜底 */ }
          })();
        }
      });
      unlisteners.push(u2);
    } catch (e) {
      console.error("[StockAnalysis] Failed to listen workflow-completed:", e);
    }

    try {
      const u3 = await listen<{
        workflowId: string;
        error: string;
        errorCode?: string;
        results?: Record<string, unknown>;
        output?: unknown;
      }>("workflow-error", (event) => {
        const msg = event.payload.error;
        const { results, errorCode, output } = event.payload;

        // 即使失败也尝试解析已有的部分结果
        if (results) {
          const parsed = parseWorkflowResults(results);
          if (import.meta.env.DEV) {
            console.debug("[DQ] workflow-error", {
              error: msg,
              errorCode,
              hasDataQualityKey: "data-quality" in results,
              resultKeys: Object.keys(results),
              parsedDQLen: parsed.dataQualitySummary.length,
              parsedDQPreview: parsed.dataQualitySummary.slice(0, 200),
            });
          }
          set({
            analystReports: parsed.analystReports,
            debateRounds: parsed.debateRounds,
            riskAssessments: parsed.riskAssessments,
            valueAssessments: parsed.valueAssessments,
            ruleCheckResults: parsed.ruleCheckResults,
            dataQualitySummary: parsed.dataQualitySummary,
            rawData: parsed.rawData,
            decision: parsed.decision,
            // V55: 即使工作流失败也要保留 strict_mode 兜底标记
            untrustedNodes: { ...get().untrustedNodes, ...parsed.untrustedNodes },
          });
        } else if (output) {
          // 兜底：从 output 中解析决策（少数异常路径无 results 但含 output）
          const parsed = extractDecision(output);
          if (parsed) { set({ decision: parsed }); }
        }

        // 修复 #9: 优先用结构化 errorCode，回退到 msg.includes("LLM") 字符串判断
        const effectiveErrorCode = errorCode ?? (msg.includes("LLM") ? "LLM_FALLBACK" : "GENERIC_ERROR");
        const isLlmError = effectiveErrorCode.startsWith("LLM_");
        const cur = get();
        set({
          error: msg,
          errorCode: effectiveErrorCode,
          // 修复 #4: LLM 错误时工作流已终止，status 应为 "completed" 而非 "running"，
          // llmStatus="placeholder" 已表达降级语义，progressPct 保持实际进度不虚报 100%
          status: isLlmError ? "completed" : "error",
          llmStatus: isLlmError ? "placeholder" : cur.llmStatus,
          progressMessage: isLlmError
            ? i18n.t("stockAnalysis.progress.llmFallback")
            : msg,
          progressPct: cur.progressPct,
          currentStage: cur.currentStage,
        });

        // #21 自动重试：仅瞬态错误(超时/网络/服务暂不可用等)且未超过上限时，
        // 延迟重跑同一股票的工作流一次。LLM 降级错误不重试(已走 placeholder 降级)。
        // 重试前再次校验状态仍为同一股票的 error，避免覆盖用户后续操作或重复触发。
        if (!isLlmError && isRetryableWorkflowError(effectiveErrorCode, msg)) {
          const retryState = get();
          if (retryState._workflowErrorRetries < MAX_WORKFLOW_ERROR_RETRIES) {
            set({ _workflowErrorRetries: retryState._workflowErrorRetries + 1 });
            const retryCode = retryState.stockCode;
            setTimeout(() => {
              const s = get();
              if (s.status === "error" && s.stockCode === retryCode) {
                s.startAnalysis(retryCode);
              }
            }, 2000);
          }
        }
      });
      unlisteners.push(u3);
    } catch (e) {
      console.error("[StockAnalysis] Failed to listen workflow-error:", e);
    }

    try {
      const u4 = await listen<{
        stockCode: string;
        reason: string;
        currentPrice: number;
        changePct: number;
        turnoverRate: number;
        timestamp: number;
        // P1-1: 后端 callback 已触发重跑时携带 backendTriggered=true，
        // 前端据此跳过 startAnalysis（避免后端+前端重复重跑），仅做 toast 提示。
        backendTriggered?: boolean;
      }>("stock-monitor-t0-rerun-requested", (event) => {
        const { stockCode, reason, backendTriggered } = event.payload;
        // 防抖: 当前正在跑 workflow 就不重入
        const cur = get();
        if (cur.status === "running" || cur.status === "loading") {
          console.warn(`[t0] skip ${stockCode}: workflow 已在运行中`);
          return;
        }
        console.info(
          `[t0] 收到 T+0 重跑请求: stock=${stockCode} reason=${reason} backendTriggered=${backendTriggered ?? false}`,
        );
        // P1-1: 后端已通过 t0_callback 触发重跑，前端不再调 startAnalysis，
        //       只做 toast 提示（避免后端+前端双跑造成资源浪费和版本冲突）
        if (backendTriggered) {
          console.info(`[t0] 后端已重跑，前端跳过 startAnalysis`);
          return;
        }
        // 直接调 store 内的 startAnalysis (它会拉 quote/kline 再触发 workflow)
        get().startAnalysis(stockCode);
      });
      unlisteners.push(u4);
    } catch (e) {
      console.error("[StockAnalysis] Failed to listen stock-monitor-t0-rerun-requested:", e);
    }
    // _unlisten 已在函数顶部 set 过(line 1034),这里不再重复 set。
  },
}));

// ── 节点 ID 分类配置表 ──
// 单一事实来源：inferStage / inferTimelinePhase / agentDisplayName / inferEvidenceRefs 均从此表派生。
// 新增节点类型只需在此添加一行，无需同时改三个函数。
interface NodeClassEntry {
  /** 匹配模式 — startsWith 字符串前缀或精确匹配字符串 */
  match: string | string[];
  /** 工作流管线阶段（0-4） */
  stage: number;
  /** 决策时间线阶段（null = 不进 timeline） */
  phase: TimelinePhase | null;
  /** 证据引用（侧栏面板导航） */
  evidence: Array<{ tabKey: "market" | "analyze" | "execute"; panelKey: string }>;
  /** 是否精确匹配（默认 false = startsWith） */
  exact?: boolean;
}

const NODE_CLASS_TABLE: NodeClassEntry[] = [
  // 阶段 0: 触发器（数据准备）
  { match: "trigger", stage: 0, phase: null, evidence: [], exact: true },

  // 阶段 1: 数据采集 & 分析师分析
  { match: "t-", stage: 1, phase: "scan", evidence: [{ tabKey: "market", panelKey: "concepts" }] },
  { match: "a-", stage: 1, phase: "diagnose", evidence: [{ tabKey: "analyze", panelKey: "analysts" }] },
  { match: "p-analysts", stage: 1, phase: null, evidence: [], exact: true },

  // 阶段 2: 多空辩论
  { match: "debate-bull-bear", stage: 2, phase: null, evidence: [], exact: true },
  {
    match: ["bull-researcher", "bear-researcher"],
    stage: 2,
    phase: "debate",
    evidence: [{ tabKey: "analyze", panelKey: "debate" }],
    exact: true,
  },
  { match: "bull-r", stage: 2, phase: "debate", evidence: [{ tabKey: "analyze", panelKey: "debate" }] },
  { match: "bear-r", stage: 2, phase: "debate", evidence: [{ tabKey: "analyze", panelKey: "debate" }] },

  // 阶段 3: 风险评估
  {
    match: "value-investor",
    stage: 3,
    phase: "decide",
    evidence: [{ tabKey: "analyze", panelKey: "value" }],
    exact: true,
  },
  { match: "risk-", stage: 3, phase: "decide", evidence: [{ tabKey: "analyze", panelKey: "risk" }] },
  {
    match: ["research-mgr", "p-risk-assess"],
    stage: 3,
    phase: "decide",
    evidence: [{ tabKey: "analyze", panelKey: "risk" }],
    exact: true,
  },
  { match: "data-quality", stage: 3, phase: null, evidence: [], exact: true },
  { match: "raw-data", stage: 3, phase: null, evidence: [], exact: true },

  // 阶段 4: 决策 & 后处理
  { match: "trader", stage: 4, phase: "decide", evidence: [{ tabKey: "execute", panelKey: "trade" }], exact: true },
  {
    match: "portfolio-mgr",
    stage: 4,
    phase: "decide",
    evidence: [{ tabKey: "analyze", panelKey: "decision" }],
    exact: true,
  },
  {
    match: "rule-check",
    stage: 4,
    phase: "decide",
    evidence: [{ tabKey: "analyze", panelKey: "decision" }],
    exact: true,
  },
  {
    match: ["agg-risk", "cls-risk-level", "v-validate", "notify-result"],
    stage: 4,
    phase: null,
    evidence: [],
    exact: true,
  },
];

function matchNodeClass(nodeId: string): NodeClassEntry | undefined {
  return NODE_CLASS_TABLE.find((entry) => {
    const patterns = Array.isArray(entry.match) ? entry.match : [entry.match];
    return patterns.some((p) => entry.exact ? nodeId === p : nodeId.startsWith(p));
  });
}

/** 从节点 ID 推断当前管线阶段 */
function inferStage(nodeId: string): number {
  return matchNodeClass(nodeId)?.stage ?? -1;
}

// 暴露给单元测试使用
export { inferStage, NODE_CLASS_TABLE };

/** 从节点 ID 推断时间线 4 阶段之一；非业务节点返回 null 不进 timeline */
function inferTimelinePhase(nodeId: string): TimelinePhase | null {
  return matchNodeClass(nodeId)?.phase ?? null;
}

/** Agent 显示名：去前缀 + 首字母大写。a-tech-analyst → Tech Analyst */
function agentDisplayName(nodeId: string): string {
  const stripped = nodeId
    .replace(/^a-/, "")
    .replace(/^t-/, "")
    .replace(/^bull-/, "")
    .replace(/^bear-/, "")
    .replace(/-/g, " ");
  return stripped.replace(/\b\w/g, (c) => c.toUpperCase());
}

/** 节点 → 证据引用：根据 nodeId 推断其结果会落在哪个侧栏 sheet panel */
function inferEvidenceRefs(
  nodeId: string,
): Array<{ tabKey: "market" | "analyze" | "execute"; panelKey: string }> {
  return matchNodeClass(nodeId)?.evidence ?? [];
}

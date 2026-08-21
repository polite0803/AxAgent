// i18n-exempt: 股票分析类型定义（含后端数据值枚举，如风险等级/催化剂方向中文标识），类型定义非 UI 文案。
// ── 运行时工具函数（实现已在 @/lib/stock-analysis-utils.ts） ──
// 向后兼容的 re-export，新代码应直接从 @/lib/stock-analysis-utils 导入
export {
  classifySentiment,
  computeEvidenceDrivenConsensus,
  computeEvidenceWeights,
  computeStockConsensus,
  type Consensus,
  getActionColor,
  getActionTagStyle,
  getActionTKey,
  getRiskColor,
  getRiskTKey,
  getSignalColor,
  parseAction,
  parseRiskLevel,
  type Sentiment,
  STOCK_ACTION_LABELS,
  STOCK_RISK_LABELS,
  StockAction,
  type StockConsensus,
  StockRiskLevel,
} from "@/lib/stock-analysis-utils";

// 证据质量驱动权重的类型定义
export type {
  AnalystInput,
  AnalystWeight,
  EvidenceConsensus,
  EvidenceWeightReport,
  EvidenceWeightRequest,
  HoldGateResult,
  MarketRegimeInfo,
} from "@/lib/stock-analysis-utils";

// StockActionType / StockRiskLevelType 由 @/lib/stock-analysis-utils 导出
import type { StockActionType, StockRiskLevelType } from "@/lib/stock-analysis-utils";
export type { StockActionType, StockRiskLevelType };

// ── 纯类型定义 ──

/**
 * 反思反馈提交结果（对应后端 submit_reflection_feedback 命令返回值）。
 *
 * 接入 FeedbackOrchestrator + ExperiencePipeline 双轨：
 * - Pipeline：反馈 → Experience → 经验池
 * - Orchestrator：计数 + 阈值触发 RLTraining / SkillEvolution
 */
export interface ReflectionFeedbackResult {
  analysisId: string;
  rating: number;
  /** Orchestrator 触发的动作类型 */
  action: "none" | "trigger_rl_training" | "trigger_skill_evolution" | "trigger_pool_size_check";
  orchestratorStats: {
    totalFeedback: number;
    negativeCount: number;
    positiveCount: number;
  };
}

export interface StockQuote {
  code: string;
  name: string;
  price: number;
  /** 昨收价,涨跌额 = price - preClose(中国股市惯例) */
  preClose: number;
  open: number;
  high: number;
  low: number;
  volume: number;
  amount: number;
  changePct: number;
  turnoverRate: number;
  pe: number | null;
  pb: number | null;
  totalMv: number | null;
  /** 流通市值（元）；后端 circulating_mv，serde rename_all camelCase */
  circulatingMv: number | null;
  limitUp: number | null;
  limitDown: number | null;
  isSt: boolean;
  timestamp: string;
}

export interface KLine {
  date: string;
  open: number;
  high: number;
  low: number;
  close: number;
  volume: number;
  amount: number;
  turnoverRate: number | null;
  /** 累计复权因子 (R3-A)；None 表示未应用复权。后端 adj_factor */
  adjFactor: number | null;
}

/** R3-B 财报披露事件 — 后端 `get_earnings_calendar` 返回结构 */
export type EarningsEventType =
  | "preliminary"
  | "express"
  | "formal"
  | "shareholders_meeting"
  | "other";

export interface EarningsEvent {
  stockCode: string;
  stockName: string;
  eventDate: string;
  eventType: EarningsEventType | string;
  period: string | null;
  detail: string | null;
  source: string | null;
  createdAt: number;
}

export interface StockSearchResult {
  code: string;
  name: string;
  market: string;
}

export interface AnalysisConfig {
  maxDebateRounds: number;
  klinePeriod: string;
  klineLimit: number;
  newsLimit: number;
}

export interface StockDecision {
  action: StockActionType;
  positionPct: number;
  targetPrice: number | null;
  stopLoss: number | null;
  reasoning: string;
  riskLevel: StockRiskLevelType;
  confidence: number;
  /** 决策方向置信度 (0-100) — 无论买卖方向都体现"多确信"。解决看空决策 confidence 偏低被误读为"不确信" */
  decisionConfidence?: number | null;
  /** 信号强度 (0-100) — 偏离中性的程度，0=完全中性，100=极端强信号 */
  signalStrength?: number | null;
  /** V66: 因子权重坍缩状态 — true 时 positionPct 被强制为 0、decisionConfidence 被减半 */
  weightsCollapsed?: boolean;
  /** V66: 坍缩原因代码: "none" | "dqi_collapsed" | "multi_untrusted" | "low_weight_ratio" */
  collapseReason?: string;
  /** V66: 因子权重占比 (total_weight/max_weight × 100)，用于 Tooltip 展示 */
  weightRatio?: number;
  /** V66: 不可信上游节点数量 */
  untrustedCount?: number;
  /** 时间维度: "ultra_short" | "short" | "mid" | "long" */
  timeHorizon?: string | null;
  /** 期望持有天数（交易日） */
  expectedHoldingDays?: number | null;
  /** 目标价预期实现时间框架: "1d" | "1w" | "1m" | "3m" | "6m" */
  targetTimeframe?: string | null;
  /** 后端检测到的 trader 输出自相矛盾（action 与 targetPrice 方向冲突）*/
  isContradictory?: boolean;
  /** V40 修复: quality-gate 判定 D/F 时，该决策来自 quality-fallback 降级路径 */
  isFallback?: boolean;
  /** V50: 双视角一致性调制后的置信度（受 agreement_factor 影响） */
  adjustedConfidence?: number;
  /** V50: 双视角一致性分维度诊断 */
  agreementBreakdown?: AgreementBreakdown;
}

// ── 决策仪表盘报告（借鉴 daily_stock_analysis 推送格式）──

/** 风险警报条目 */
export interface RiskAlert {
  description: string;
  severity: "低" | "中" | "高" | string;
  source?: string | null;
}

/** 催化因素条目 */
export interface Catalyst {
  description: string;
  direction: "利好" | "利空" | string;
  timeline?: string | null;
  confidenceScore?: number | null;
}

/** 操作检查清单条目 */
export interface ChecklistItem {
  description: string;
  checked: boolean;
  category: "入场" | "加仓" | "减仓" | "止损" | "止盈" | string;
}

/** 决策仪表盘报告（单只股票，7 段式结构） */
export interface DashboardReport {
  stockCode: string;
  stockName: string;
  analysisDate: string;
  generatedAt: string;
  coreConclusion: string;
  action: string;
  score: number;
  trend: string;
  confidence: number;
  buyPointLow?: number | null;
  buyPointHigh?: number | null;
  targetPrice?: number | null;
  stopLoss?: number | null;
  positionPct: number;
  riskAlerts: RiskAlert[];
  catalysts: Catalyst[];
  checklist: ChecklistItem[];
  latestNews?: string | null;
  earningsExpectation?: string | null;
  llmModel?: string | null;
  integrityPassed: boolean;
}

/** 指数行情 */
export interface IndexQuote {
  name: string;
  price: number;
  changePct: number;
}

/** 大盘复盘报告 */
export interface MarketReviewReport {
  reviewDate: string;
  generatedAt: string;
  indices: IndexQuote[];
  advancers?: number | null;
  decliners?: number | null;
  limitUp?: number | null;
  limitDown?: number | null;
  sectorLeaders: string[];
  sectorLaggards: string[];
  llmModel?: string | null;
}

/** 股票摘要（仪表盘汇总） */
export interface StockSummary {
  stockCode: string;
  stockName: string;
  action: string;
  score: number;
  trend: string;
  confidence: number;
}

/** 聚合仪表盘（多只股票汇总） */
export interface DashboardDigest {
  digestDate: string;
  generatedAt: string;
  totalCount: number;
  buyCount: number;
  watchCount: number;
  sellCount: number;
  summaries: StockSummary[];
  marketReview?: MarketReviewReport | null;
}

/** V65: 双视角一致性 6 维度诊断结果 */
export interface AgreementBreakdown {
  total: number;
  actionOk: boolean;
  actionNote: string;
  formulaAction: string;
  llmAction: string;
  /** V65: action 维度原始分 (满分 30) */
  actionScore?: number;
  /** V65: positionPct 维度原始分 (满分 20) */
  positionScore?: number;
  positionGap: number | null;
  /** V65: confidence 维度原始分 (满分 15) */
  confidenceScore?: number;
  confidenceGap: number | null;
  /** V65: riskLevel 维度原始分 (满分 15) */
  riskLevelScore?: number;
  /** V65: 公式 riskLevel 原始值 */
  formulaRiskLevel?: string;
  /** V65: LLM riskLevel 原始值 */
  llmRiskLevel?: string;
  /** V65: data_gaps 维度原始分 (满分 10) */
  dataGapsScore?: number;
  /** V65: data_gaps Jaccard 相似度 (0-1) */
  dataGapsSimilarity?: number | null;
  /** V65: evidence_cited 维度原始分 (满分 10) */
  evidenceScore?: number;
  /** V65: LLM 引用上游论据数量 */
  evidenceCount?: number;
  conflictType: string;
  /** 向后兼容: f7 自指污染标记 */
  f7WeightPct?: number | null;
  f7FreePosterior?: number | null;
  f7FreeAction?: string | null;
  f7FreeActionScore?: number | null;
}

/** 单个分析师的数据质量诊断条目（对应 data-quality.rhai 的 diagnostics[field]） */
export interface DataQualityDiagItem {
  /** 中文角色名，如"技术面分析师" */
  name: string;
  /** 该分析师预期消费的数据来源（静态描述） */
  expected_data: string;
  /** 实际 confidence 值；-1 表示字段缺失/节点失败 */
  confidence: number;
  /** "missing" | "low" | "normal" */
  status: "missing" | "low" | "normal";
  /** 缺失或低置信的具体原因（正常时为空字符串） */
  gap_reason: string;
}

/** data-quality 节点输出的结构化诊断报告（data-quality.rhai 输出 JSON） */
export interface DataQualityReport {
  grade: "A" | "B" | "C" | "D" | "F";
  score: number;
  /** P1-B3: 报告质量分（0-100），基于字数+关键词覆盖+占位符检测 */
  report_quality_score?: number;
  /** P1-B3: 工具可信度分（0-100），基于 avg_conf + gap/good_count */
  tool_credibility_score?: number;
  /** V58: 因子完整度百分比（0-100），10 个因子数据存在性评估 */
  factor_completeness_pct?: number;
  /** V58: 缺失因子中文名列表（如 ["技术面评分", "共识评分", ...]） */
  missing_factors?: string[];
  gap_count: number;
  good_count: number;
  avg_confidence: number;
  total_analysts: number;
  /** 各分析师详细诊断，键为缩写（mk/sent/news/...） */
  diagnostics: Record<string, DataQualityDiagItem>;
  /** 缺失分析师中文名列表 */
  missing_analysts: string[];
  /** 低置信度分析师中文名列表 */
  low_confidence_analysts: string[];
  /** P1-B3: 数据质量问题列表（字数不足/占位符/低置信等） */
  warnings?: string[];
  /** P2-2(2026-08-09): 分析师方向冲突标记（看多 vs 看空各有 ≥2 个有效分析师） */
  direction_conflict?: boolean;
  /** P2-2: 看多方向分析师数 */
  bull_dir_count?: number;
  /** P2-2: 看空方向分析师数 */
  bear_dir_count?: number;
  /** 人类可读的总结文本 */
  summary: string;
}

export interface AnalysisSummary {
  id: string;
  stockCode: string;
  stockName: string;
  analysisDate: string;
  status: string;
  decisionAction: string | null;
  /** 决策仓位百分比（0-100），列表场景由 decisionJson 解析或后端直返 */
  decisionPositionPct: number | null;
  /** 决策 JSON 字符串（含 action/positionPct/confidence 等），列表场景用于渲染决策 Tag */
  decisionJson: string | null;
  createdAt: number;
  updatedAt: number;
  /** "live" | "replay" */
  analysisKind: string;
  /** as_of_date YYYY-MM-DD（仅 replay 模式非空） */
  asOfDate: string | null;
  /** 版本化分析：指向原始分析记录 ID，null 表示首次分析 */
  parentAnalysisId: string | null;
}

export interface AnalysisEvent {
  type:
    | "started"
    | "dataLoaded"
    | "analystProgress"
    | "analystReport"
    | "debateRound"
    | "riskAssessment"
    | "investmentPlan"
    | "decision"
    | "error";
  payload: Record<string, unknown>;
}

export type AnalysisStatus = "idle" | "loading" | "running" | "paused" | "completed" | "error" | "cancelled";

// ── 回测类型 ──

export interface BacktestResult {
  stockCode: string;
  analysisDate: string;
  decisionAction: string;
  decisionConfidence: number;
  entryPrice: number | null;
  exitPrice: number;
  holdingDays: number;
  returnPct: number;
  wasCorrect: boolean;
  maxDrawdownPct: number;
}

export interface BacktestStats {
  totalAnalyses: number;
  accuracyPct: number;
  avgReturnPct: number;
  avgMaxDrawdownPct: number;
  avgConfidence: number;
  alphaPct: number | null;
}

// ── 荐股策略回测 ──

export interface StrategyStats {
  strategyId: string;
  style: string;
  period: string;
  totalSignals: number;
  winCount: number;
  lossCount: number;
  winRatePct: number;
  avgReturnPct: number;
  totalReturnPct: number;
  avgMaxDrawdownPct: number;
  maxConsecutiveLosses: number;
  sharpeRatio: number | null;
  profitFactor: number | null;
}

export interface GroupBacktestResult {
  label: string;
  stockCount: number;
  strategies: Record<string, StrategyStats>;
}

export interface BacktestComparisonResponse {
  positive: GroupBacktestResult;
  negative: GroupBacktestResult;
  positiveStocks: string[];
  negativeStocks: string[];
  skipped: string[];
}

// ── 荐股信号 ──

export interface StrategySignalResult {
  strategyId: string;
  stockCode: string;
  stockName: string;
  signalDate: string;
  entryPrice: number;
  exitPrice: number;
  holdingDays: number;
  returnPct: number;
  wasProfitable: boolean;
  maxDrawdownPct: number;
}

// ── 历史分析摘要 ──

/** 个股最近一次分析的摘要，用于荐股 panel 展示 */
export interface LatestAnalysisSummary {
  analysisId: string;
  analysisDate: string;
  decisionAction: string;
  decisionPositionPct: number | null;
  confidence: number | null;
  status: string;
  outcome: string | null;
  decisionTimeHorizon?: string | null;
  decisionExpectedHoldingDays?: number | null;
}

// ── 荐股结果 (Bug 10 抽离) ──
// 与后端 crates/stock-analysis/src/recommender/types.rs::RecoPick 一一对应,
// 字段顺序、类型、可选性保持一致(camelCase 由 serde rename_all 转换)。

export type StyleKey = "trend" | "value" | "capital" | "reversion" | "watchlist" | "serenity";
export type PeriodKey = "ultra_short" | "short" | "mid" | "long";

/** 荐股单条 pick — 完整字段版,直接对应后端 schema */
export interface RecoPick {
  stockCode: string;
  stockName: string;
  /** 行业/板块(后端 Option + skip_serializing_if=None) */
  sector?: string | null;
  /** 主风格 — 后端 serde(rename_all="lowercase"),必填 */
  style: StyleKey;
  /** 持有周期 — 后端 serde(rename_all="lowercase"),必填 */
  period: PeriodKey;
  /** 当前价 */
  price: number;
  /** 入场下沿 */
  entryLow: number;
  /** 入场上沿 */
  entryHigh: number;
  /** 止损 */
  stopLoss: number;
  /** 目标位 */
  targetPrice: number;
  /** 建议仓位(%) */
  positionPct: number;
  /** 持有天数(后端 u32) */
  holdingDays: number;
  /** 置信度 0-100(后端 u8) */
  confidence: number;
  /** 命中理由(可能为空数组) */
  reasons: string[];
  /** 风险提示(可能为空数组) */
  riskNotes: string[];
  /** 风格拆分后的副策略 tag,如 ["trend","capital"];空数组时后端跳过序列化 */
  secondaryStyles?: StyleKey[];
  /** true = 系统初筛 / 数据稀疏兜底(无技术信号),false = 主策略真实命中 */
  synthetic?: boolean;
}

/** 荐股接口响应 — 完整字段版 */
export interface RecoResponse {
  period: PeriodKey;
  /** 按风格分组的 picks,每组 ≤ 10。后端 HashMap<Style, Vec<RecoPick>> */
  picks: Partial<Record<StyleKey, RecoPick[]>>;
  /** 被 vendor 缺失禁用的风格(live 模式下由 vendor 状态决定) */
  disabledStyles: StyleKey[];
  /** as-of 模式下被降级(≠ 缺失)的风格(spec §8)。live 模式恒为空数组。 */
  degradedStyles?: StyleKey[];
  /** degradedStyles 中各风格的具体降级原因,key=styleKey, value=本地化文本 */
  degradedReasons?: Record<string, string>;
  /** 生成时间戳(毫秒) */
  generatedAt: number;
  /** 过滤前的 seed pool 大小(hot + industry 龙头去重后) */
  rawSeedPoolSize: number;
  /** 时间旅行模式截止日 YYYY-MM-DD;live 时 undefined */
  asOfDate?: string;
  /** 模式标签: live / replay / backtest_sweep — 后端 spec §8 注入,必填 */
  mode: string;
  /** 数据获取错误详情(picks 为空时的具体原因)。后端填充,前端据此显示具体错误文本而非泛化的"连接失败" */
  errorDetail?: string;
}

// ── 决策时间线类型 ──

/** 时间线 4 阶段：扫描 → 诊断 → 辩论 → 决策 */
export type TimelinePhase = "scan" | "diagnose" | "debate" | "decide";

/** 节点状态：pending(未开始)/ running(进行中)/ done(完成)/ failed(失败) */
export type TimelineNodeStatus = "pending" | "running" | "done" | "failed";

/** 节点证据引用：点击 EvidenceChip 时跳转到对应侧栏面板 */
export interface EvidenceRef {
  tabKey: "market" | "analyze" | "execute";
  panelKey: string;
  anchor?: string;
  snippet?: string;
}

/** 单个时间线节点 */
export interface TimelineNode {
  id: string;
  phase: TimelinePhase;
  agentId: string;
  agentName: string;
  title: string;
  summary: string;
  confidence: number;
  status: TimelineNodeStatus;
  evidenceRefs: EvidenceRef[];
  children?: TimelineNode[];
  startedAt?: number;
  finishedAt?: number;
}

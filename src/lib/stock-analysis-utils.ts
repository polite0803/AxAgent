/**
 * stock-analysis 运行时工具函数
 *
 * 与 @/types/stock-analysis 中的纯类型定义分离，避免类型文件混合实现。
 * 保留 @/types/stock-analysis 作为向后兼容的 re-export 入口。
 */

// ── 证据质量驱动权重 (P0-1) ──

import { invoke } from "@/lib/invoke";

/** 市场环境信息（与后端 EvidenceWeightRequest 对应） */
export interface MarketRegimeInfo {
  regime: string;
  confidence: number;
  volatility: string;
  description: string;
  volatilityPct?: number | null;
  consecutiveUp: number;
  consecutiveDown: number;
}

/** 分析师输入 */
export interface AnalystInput {
  analystId: string;
  reportText?: string | null;
  stance?: string | null;
  bullScore?: number | null;
  bearScore?: number | null;
  positionPct?: number | null;
}

/** 证据权重计算请求 */
export interface EvidenceWeightRequest {
  marketRegime: MarketRegimeInfo;
  timeHorizon: string;
  analysts: AnalystInput[];
  historicalWeights?: Record<string, number> | null;
}

/** 分析师权重详情 */
export interface AnalystWeight {
  analystId: string;
  domain: string;
  horizonWeight: number;
  regimeModifier: number;
  historyModifier: number;
  finalWeight: number;
  stanceDirection: string;
  stanceConfidence: number;
}

/** 共识结果 */
export interface EvidenceConsensus {
  bullishScore: number;
  bearishScore: number;
  neutralScore: number;
  totalWeight: number;
  netScore: number;
  consensus: string;
  confidence: number;
}

/** HOLD 门控结果 */
export interface HoldGateResult {
  holdAllowed: boolean;
  reason: string;
  technicalHasTrend: boolean;
  moneyflowHasDirection: boolean;
  fundamentalHasCatalyst: boolean;
  suggestedAction: string;
}

/** 完整证据权重报告 */
export interface EvidenceWeightReport {
  marketRegime: MarketRegimeInfo;
  timeHorizon: string;
  analystWeights: AnalystWeight[];
  consensus: EvidenceConsensus;
  holdGate: HoldGateResult;
  recommendedAction: string;
  recommendedPositionPct: number;
  overallConfidence: number;
}

/** 调用后端证据质量驱动权重计算 */
export async function computeEvidenceWeights(
  request: EvidenceWeightRequest,
): Promise<EvidenceWeightReport> {
  return invoke<EvidenceWeightReport>("compute_evidence_weights", { request });
}

// ── 枚举常量 ──

/** 股票操作动作枚举 — 内部统一用英文标识，展示时通过 i18n 翻译 */
export const StockAction = {
  BUY: "BUY",
  INCREASE: "INCREASE",
  HOLD: "HOLD",
  REDUCE: "REDUCE",
  SELL: "SELL",
  WAIT: "WAIT",
  UNCERTAIN: "UNCERTAIN",
} as const;

export type StockActionType = (typeof StockAction)[keyof typeof StockAction];

/** 中文标签映射（给 LLM 输出兼容 + 解析用） */
export const STOCK_ACTION_LABELS: Record<string, StockActionType> = {
  "买入": StockAction.BUY,
  "增持": StockAction.INCREASE,
  "持有": StockAction.HOLD,
  "减持": StockAction.REDUCE,
  "卖出": StockAction.SELL,
  "不确定": StockAction.UNCERTAIN,
  "无法判断": StockAction.UNCERTAIN,
  "观望": StockAction.WAIT,
  "等待": StockAction.WAIT,
  "减仓": StockAction.REDUCE,
  "加仓": StockAction.INCREASE,
  "看多": StockAction.BUY,
  "看空": StockAction.SELL,
  "中性": StockAction.HOLD,
  "买入\n": StockAction.BUY,
  "买入 ": StockAction.BUY,
  "持有 ": StockAction.HOLD,
};

/** 股票风险等级枚举 */
export const StockRiskLevel = {
  LOW: "LOW",
  MID: "MID",
  HIGH: "HIGH",
  EXTREME: "EXTREME",
} as const;

export type StockRiskLevelType = (typeof StockRiskLevel)[keyof typeof StockRiskLevel];

/** 中文风险等级标签映射 */
export const STOCK_RISK_LABELS: Record<string, StockRiskLevelType> = {
  "低风险": StockRiskLevel.LOW,
  "中风险": StockRiskLevel.MID,
  "高风险": StockRiskLevel.HIGH,
  "极高": StockRiskLevel.EXTREME,
  "低": StockRiskLevel.LOW,
  "中": StockRiskLevel.MID,
  "高": StockRiskLevel.HIGH,
};

// ── 解析函数 ──

/** 解析股票操作动作（兼容英文/中文/大小写/前后空格） */
export function parseAction(raw: unknown): StockActionType {
  if (typeof raw === "string") {
    const clean = raw.trim().toUpperCase();
    if (clean in StockAction) { return clean as StockActionType; }
    for (const [label, action] of Object.entries(STOCK_ACTION_LABELS)) {
      if (raw.includes(label)) { return action; }
    }
  }
  return StockAction.WAIT;
}

/** 解析股票风险等级（兼容英文/中文/大小写） */
export function parseRiskLevel(raw: unknown): StockRiskLevelType {
  if (typeof raw !== "string" && typeof raw !== "number") { return StockRiskLevel.MID; }
  const clean = String(raw).trim().toUpperCase();
  if (clean in StockRiskLevel) { return clean as StockRiskLevelType; }
  if (["低", "低风险", "L"].includes(clean)) { return StockRiskLevel.LOW; }
  if (["中", "中风险", "M"].includes(clean)) { return StockRiskLevel.MID; }
  if (["高", "高风险", "H"].includes(clean)) { return StockRiskLevel.HIGH; }
  if (["极高", "极高风险", "E"].includes(clean)) { return StockRiskLevel.EXTREME; }
  for (const [label, level] of Object.entries(STOCK_RISK_LABELS)) {
    if (String(raw).includes(label)) { return level; }
  }
  return StockRiskLevel.MID;
}

// ── 颜色 & i18n 键 ──

export function getActionColor(action: string): "red" | "green" | "orange" | "blue" | "default" {
  switch (action) {
    case StockAction.BUY:
    case StockAction.INCREASE:
      return "red";
    case StockAction.SELL:
    case StockAction.REDUCE:
      return "green";
    case StockAction.HOLD:
      return "blue";
    case StockAction.UNCERTAIN:
      return "default";
    case StockAction.WAIT:
      return "default";
    default:
      return "default";
  }
}

export function getActionTKey(action: string): string {
  const normalized = parseAction(action); // 中文"卖出"→"SELL"，统一后再查表
  switch (normalized) {
    case StockAction.BUY:
      return "stockAnalysis.actionBuy";
    case StockAction.INCREASE:
      return "stockAnalysis.actionIncrease";
    case StockAction.HOLD:
      return "stockAnalysis.actionHold";
    case StockAction.REDUCE:
      return "stockAnalysis.actionReduce";
    case StockAction.SELL:
      return "stockAnalysis.actionSell";
    case StockAction.WAIT:
      return "stockAnalysis.actionWait";
    default:
      return "stockAnalysis.actionUncertain";
  }
}

export function getRiskColor(level: string): string {
  switch (level) {
    case StockRiskLevel.LOW:
      return "var(--sa-green)";
    case StockRiskLevel.MID:
      return "var(--sa-amber)";
    case StockRiskLevel.HIGH:
      return "var(--sa-red)";
    case StockRiskLevel.EXTREME:
      return "var(--sa-extreme)";
    default:
      return "var(--muted)";
  }
}

export function getRiskTKey(level: string): string {
  const normalized = parseRiskLevel(level); // "高风险"→"HIGH"，统一后再查表
  const map: Record<string, string> = {
    [StockRiskLevel.HIGH]: "stockAnalysis.riskHigh",
    [StockRiskLevel.MID]: "stockAnalysis.riskMid",
    [StockRiskLevel.LOW]: "stockAnalysis.riskLow",
    [StockRiskLevel.EXTREME]: "stockAnalysis.riskExtreme",
  };
  return map[normalized] ?? "stockAnalysis.riskMid";
}

export function getSignalColor(signal: string): "green" | "red" | "blue" {
  const s = String(signal ?? "").trim().toLowerCase();
  if (s.includes("买") || s.includes("多") || s.includes("涨") || s.includes("牛")) { return "green"; }
  if (s.includes("卖") || s.includes("空") || s.includes("跌") || s.includes("熊")) { return "red"; }
  return "blue";
}

/** 尝试解析报告字符串为 JSON 对象 */
function tryParseJson(text: string): Record<string, unknown> | null {
  try {
    const trimmed = text.trim();
    if (trimmed.startsWith("{") || trimmed.startsWith("[")) {
      try {
        return JSON.parse(trimmed) as Record<string, unknown>;
      } catch { /* try below */ }
    }
    const m = trimmed.match(/```(?:json)?\s*([\s\S]*?)\s*```/);
    if (m) {
      try {
        return JSON.parse(m[1].trim()) as Record<string, unknown>;
      } catch { /* try below */ }
    }
    const fb = trimmed.indexOf("{");
    const lb = trimmed.lastIndexOf("}");
    if (fb !== -1 && lb !== -1 && lb > fb) {
      try {
        return JSON.parse(trimmed.slice(fb, lb + 1)) as Record<string, unknown>;
      } catch { /* ignore */ }
    }
  } catch { /* not json */ }
  return null;
}

// ── 情感分析 ──

/**
 * 分析师报告情感分类（先解析 JSON 提取结构化字段，再回退子串匹配）
 *
 * 支持:
 * - JSON 中 stance / action / bull_score / positionPct / recommendation 等字段
 * - 纯文本关键词匹配（买入/卖出/持有/看多/看空 等维度）
 */
export function classifySentiment(report: string): "bullish" | "bearish" | "neutral" {
  // 0) 优先解析 <!-- VERDICT: {...} --> 格式（分析师自由文本 + 末尾 verdict 标签）
  const verdictIdx = report.indexOf("<!-- VERDICT:");
  if (verdictIdx !== -1) {
    try {
      const jsonStr = report.slice(verdictIdx + "<!-- VERDICT:".length);
      const jsonEnd = jsonStr.indexOf("-->");
      if (jsonEnd !== -1) {
        const meta = JSON.parse(jsonStr.slice(0, jsonEnd).trim());
        const stance = String(meta.verdict ?? meta.stance ?? "").trim().toLowerCase();
        if (stance) {
          if (/看多|买入|增持|做多|看涨|多头|利好|上涨|乐观|上行|流入|bull|buy|overweight/i.test(stance)) {
            return "bullish";
          }
          if (/看空|卖出|减持|做空|看跌|空头|利空|下跌|悲观|下行|流出|bear|sell|underweight/i.test(stance)) {
            return "bearish";
          }
          if (/中性|观望|持有|震荡|hold|neutral/i.test(stance)) {
            return "neutral";
          }
        }
        // 用 bull_score / bear_score 判断
        const bull = Number(meta.bull_score ?? -1);
        const bear = Number(meta.bear_score ?? -1);
        if (bull >= 0 && bear >= 0) {
          if (bull > bear) { return "bullish"; }
          if (bear > bull) { return "bearish"; }
          return "neutral";
        }
      }
    } catch { /* ignore */ }
  }

  // 1) 尝试从 JSON 结构化字段提取
  const json = tryParseJson(report);
  if (json) {
    // 1a) stance 字段 — 中文方向词，覆盖各 agent 不同的命名空间
    const stanceRaw = json["stance"] ?? json["view"] ?? json["sentiment"] ?? json["verdict"];
    const stance = String(stanceRaw ?? "").trim();
    if (stance) {
      const lower = stance.toLowerCase();
      // 多头方向词
      if (
        stance.includes("买入") || stance.includes("增持") || stance.includes("看多")
        || stance.includes("做多") || stance.includes("看涨") || stance.includes("多头")
        || stance.includes("利好") || stance.includes("上涨") || stance.includes("乐观")
        || stance.includes("上行") || stance.includes("流入") || stance.includes("扫货")
        || stance.includes("强于") || stance.includes("超配") || stance.includes("加仓")
        || lower.includes("bull") || lower.includes("buy") || lower.includes("overweight")
      ) {
        return "bullish";
      }
      // 空头方向词
      if (
        stance.includes("卖出") || stance.includes("减持") || stance.includes("看空")
        || stance.includes("做空") || stance.includes("看跌") || stance.includes("空头")
        || stance.includes("利空") || stance.includes("下跌") || stance.includes("悲观")
        || stance.includes("下行") || stance.includes("流出") || stance.includes("出货")
        || stance.includes("弱于") || stance.includes("低配") || stance.includes("减仓")
        || lower.includes("bear") || lower.includes("sell") || lower.includes("underweight")
      ) {
        return "bearish";
      }
      // 中性方向词
      if (
        stance.includes("观望") || stance.includes("中性") || stance.includes("平衡")
        || stance.includes("震荡") || stance.includes("同步") || stance.includes("保守")
        || stance.includes("放缓") || stance.includes("持有") || stance.includes("hold")
        || lower.includes("neutral") || lower.includes("hold")
      ) {
        return "neutral";
      }
    }
    // 1b) action 字段（BUY/INCREASE/SELL/REDUCE/HOLD 或中文 买入/增持/...）
    const action = String(json["action"] ?? "").trim();
    if (action) {
      const a = action.toUpperCase();
      if (a === "BUY" || a === "INCREASE") { return "bullish"; }
      if (a === "SELL" || a === "REDUCE") { return "bearish"; }
      if (a === "HOLD") { return "neutral"; }
      if (action.includes("买入") || action.includes("增持")) { return "bullish"; }
      if (action.includes("卖出") || action.includes("减持")) { return "bearish"; }
      if (action.includes("持有") || action.includes("观望")) { return "neutral"; }
    }
    // 1c) bull_score / bear_score 数字打分（0-10，分开打分）
    const bullScoreRaw = json["bull_score"] ?? json["bullScore"];
    const bearScoreRaw = json["bear_score"] ?? json["bearScore"];
    if (bullScoreRaw != null || bearScoreRaw != null) {
      const bullScore = Number(bullScoreRaw ?? 0);
      const bearScore = Number(bearScoreRaw ?? 0);
      if (Number.isFinite(bullScore) && Number.isFinite(bearScore)) {
        const diff = bullScore - bearScore;
        if (diff > 0) { return "bullish"; }
        if (diff < 0) { return "bearish"; }
      }
    }
    // 1d) positionPct 仓位 — trader / debator 输出，0-100
    const posPctRaw = json["positionPct"] ?? json["position_pct"];
    if (posPctRaw != null) {
      const posPct = Number(posPctRaw);
      if (Number.isFinite(posPct)) {
        if (posPct >= 6) { return "bullish"; }
        if (posPct < 0) { return "bearish"; }
        return "neutral";
      }
    }
    // 1e) recommendation / rating 字段
    const rec = String(json["recommendation"] ?? json["rating"] ?? "").trim().toLowerCase();
    if (rec) {
      if (rec.includes("buy") || rec.includes("买入") || rec.includes("增持") || rec.includes("看涨")) {
        return "bullish";
      }
      if (rec.includes("sell") || rec.includes("卖出") || rec.includes("减持") || rec.includes("看跌")) {
        return "bearish";
      }
      if (rec.includes("hold") || rec.includes("持有") || rec.includes("中性") || rec.includes("观望")) {
        return "neutral";
      }
    }
  }

  // 2) 纯文本回退：子串级情感匹配
  const text = report.toLowerCase();
  let bullScore = 0;
  let bearScore = 0;

  const bullPatterns = [
    "买入",
    "增持",
    "看多",
    "做多",
    "看涨",
    "多头",
    "利好",
    "上涨",
    "乐观",
    "上行",
    "流入",
    "扫货",
    "强于",
    "超配",
    "加仓",
    "bull",
    "buy",
    "overweight",
    "增长",
    "改善",
    "盈利",
    "回升",
    "反弹",
    "突破",
    "看好",
  ];
  const bearPatterns = [
    "卖出",
    "减持",
    "看空",
    "做空",
    "看跌",
    "空头",
    "利空",
    "下跌",
    "悲观",
    "下行",
    "流出",
    "出货",
    "弱于",
    "低配",
    "减仓",
    "bear",
    "sell",
    "underweight",
    "下滑",
    "恶化",
    "亏损",
    "回落",
    "跌破",
  ];

  for (const p of bullPatterns) {
    const idx = text.indexOf(p);
    if (idx !== -1) {
      // 检查否定前缀
      const before = text.slice(Math.max(0, idx - 3), idx);
      if (!["无", "没有", "不会", "不存", "无需"].some((neg) => before.includes(neg))) {
        bullScore++;
      }
    }
  }
  for (const p of bearPatterns) {
    const idx = text.indexOf(p);
    if (idx !== -1) {
      const before = text.slice(Math.max(0, idx - 3), idx);
      if (!["无", "没有", "不会", "不存", "无需"].some((neg) => before.includes(neg))) {
        bearScore++;
      }
    }
  }

  if (bullScore > bearScore) { return "bullish"; }
  if (bearScore > bullScore) { return "bearish"; }
  return "neutral";
}

// ── 共识聚合 ──

export type Sentiment = "bullish" | "bearish" | "neutral";
export type Consensus = Sentiment | "divided";

export interface StockConsensus {
  consensus: Consensus;
  bullish: number;
  bearish: number;
  neutral: number;
  total: number;
  /** 时间戳（毫秒） */
  updatedAt: number;
}

/** 分析师按 ID 后缀的领域权重：value=价值/长线, technical=技术/短线, sentiment=情绪, macro=宏观 */
const ANALYST_TIME_HORIZON_WEIGHT: Record<string, Record<string, number>> = {
  // 中线决策：基本面与技术面均衡，各分析师权重接近
  mid: {
    "a-fundamentals": 1.2,
    "fundamental": 1.2,
    "value-investor": 1.2,
    "a-macro": 1.1,
    "macro": 1.1,
    "a-sector": 1.1,
    "research-mgr": 1.1,
    "a-market": 1.0,
    "a-technical": 1.0,
    "a-sentiment": 1.0,
    "sentiment": 1.0,
    "a-news": 1.0,
    "a-hot-money": 0.9,
    "capital": 0.9,
    default: 1.0,
  },
  // 长线决策：价值投资者和分析师权重最高，技术面被削弱
  long: {
    "fundamental": 1.5,
    "a-fundamentals": 1.5,
    "value-investor": 2.0,
    "a-macro": 1.3,
    "macro": 1.3,
    "a-sector": 1.2,
    "research-mgr": 1.5,
    "a-news": 0.7,
    "sentiment": 0.7,
    "a-sentiment": 0.7,
    "a-hot-money": 0.5,
    "capital": 0.5,
    "a-technical": 0.6,
    "a-market": 0.6,
    default: 1.0,
  },
  // 短线决策：技术面、资金面、情绪权重最高
  short: {
    "a-market": 1.5,
    "a-technical": 1.5,
    "a-hot-money": 1.5,
    "capital": 1.5,
    "a-sentiment": 1.3,
    "sentiment": 1.3,
    "a-news": 1.2,
    "value-investor": 0.5,
    "a-fundamentals": 0.6,
    "fundamental": 0.6,
    "a-macro": 0.7,
    default: 1.0,
  },
  // 超短线：资金面、情绪权重最高，基本面几乎不考虑
  ultra_short: {
    "a-hot-money": 2.0,
    "capital": 2.0,
    "a-sentiment": 1.5,
    "sentiment": 1.5,
    "a-news": 1.5,
    "a-market": 1.3,
    "value-investor": 0.3,
    "a-fundamentals": 0.3,
    "fundamental": 0.3,
    "a-macro": 0.3,
    "research-mgr": 0.5,
    default: 1.0,
  },
};

/** 根据分析师 ID 获取时间维度权重 */
function getAnalystWeight(analystId: string, timeHorizon?: string | null): number {
  const weights = ANALYST_TIME_HORIZON_WEIGHT[timeHorizon || "mid"] || ANALYST_TIME_HORIZON_WEIGHT.mid;
  // 精确匹配
  if (weights[analystId] != null) { return weights[analystId]; }
  // 后缀模糊匹配
  for (const [suffix, w] of Object.entries(weights)) {
    if (suffix === "default") { continue; }
    if (analystId.includes(suffix)) { return w; }
  }
  return weights.default ?? 1.0;
}

/**
 * 聚合分析师报告/投票，推最终共识。
 * 支持 timeHorizon 参数：不同时间维度下分析师权重不同
 * （长线重基本面、短线重技术面动量、超短线重资金流）。
 */
export function computeStockConsensus(
  reports: Record<string, string>,
  updatedAt?: number,
  timeHorizon?: string | null,
): StockConsensus {
  let bullish = 0;
  let bearish = 0;
  let neutral = 0;
  for (const [analystId, text] of Object.entries(reports)) {
    const s = classifySentiment(text);
    const w = getAnalystWeight(analystId, timeHorizon);
    if (s === "bullish") { bullish += w; }
    else if (s === "bearish") { bearish += w; }
    else { neutral += w; }
  }
  const total = bullish + bearish + neutral;
  let consensus: Consensus;
  if (total === 0) {
    consensus = "neutral";
  } else {
    const net = bullish - bearish;
    // 加权后阈值：加权 net > 加权 total × 0.3 → bullish（等效 N/3）
    const threshold = total * 0.3;
    if (net > threshold) { consensus = "bullish"; }
    else if (net < -threshold) { consensus = "bearish"; }
    else if (bullish > 0 && bearish > 0) { consensus = "divided"; }
    else { consensus = "neutral"; }
  }
  return {
    consensus,
    bullish: Math.round(bullish * 10) / 10,
    bearish: Math.round(bearish * 10) / 10,
    neutral: Math.round(neutral * 10) / 10,
    total: Math.round(total * 10) / 10,
    updatedAt: updatedAt ?? Date.now(),
  };
}

/**
 * 证据质量驱动的共识计算（P0-1）。
 *
 * 替代简单的阈值投票，结合市场环境(regime)、时间维度、分析师历史表现
 * 动态分配权重，并检查 HOLD 门控条件。
 *
 * @param reports 分析师报告字典
 * @param marketRegime 市场环境信息（从 loadMarketRegime 获取）
 * @param timeHorizon 投资周期
 * @param historicalWeights 可选的历史表现权重
 * @param updatedAt 可选的时间戳
 */
export async function computeEvidenceDrivenConsensus(
  reports: Record<string, string>,
  marketRegime: MarketRegimeInfo,
  timeHorizon?: string | null,
  historicalWeights?: Record<string, number> | null,
  updatedAt?: number,
): Promise<StockConsensus & { evidenceReport?: EvidenceWeightReport }> {
  // marketRegime 缺失时提供默认值（避免后端校验失败）
  const defaultRegime: MarketRegimeInfo = marketRegime ?? {
    regime: "neutral",
    confidence: 0.5,
    volatility: "medium",
    description: "未知市场环境（默认值）",
  };

  try {
    // 构建分析师输入
    const analysts: AnalystInput[] = Object.entries(reports).map(([analystId, text]) => {
      const json = tryParseJson(text);
      return {
        analystId,
        reportText: text,
        stance: json ? (String(json["stance"] ?? json["view"] ?? json["verdict"] ?? "") || null) : null,
        bullScore: json ? (json["bull_score"] as number ?? json["bullScore"] as number ?? null) : null,
        bearScore: json ? (json["bear_score"] as number ?? json["bearScore"] as number ?? null) : null,
        positionPct: json ? (json["positionPct"] as number ?? json["position_pct"] as number ?? null) : null,
      };
    });

    const request: EvidenceWeightRequest = {
      marketRegime: defaultRegime,
      timeHorizon: timeHorizon ?? "mid",
      analysts,
      historicalWeights: historicalWeights ?? null,
    };

    const evidenceReport = await computeEvidenceWeights(request);

    // 将后端结果映射为前端 StockConsensus 格式
    const consensus = evidenceReport.consensus;
    const consensusMap: Record<string, Consensus> = {
      bullish: "bullish",
      bearish: "bearish",
      divided: "divided",
      neutral: "neutral",
    };

    return {
      consensus: consensusMap[consensus.consensus] ?? "neutral",
      bullish: consensus.bullishScore,
      bearish: consensus.bearishScore,
      neutral: consensus.neutralScore,
      total: consensus.totalWeight,
      updatedAt: updatedAt ?? Date.now(),
      evidenceReport,
    };
  } catch (err) {
    // fallback: 如果后端不可用，回退到前端旧版计算
    console.warn("[computeEvidenceDrivenConsensus] 后端计算失败，回退到前端简单共识:", err);
    const result = computeStockConsensus(reports, updatedAt, timeHorizon);
    return { ...result, evidenceReport: undefined };
  }
}

// ── 分析师名称映射（已迁移至 i18n: stockAnalysis.workflow.analyst.*）──

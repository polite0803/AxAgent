// 决策输入诊断：纯前端工具，把 portfolio-mgr 实际消费的 16 个上游节点输出
// 统一解析成"数据符合度"诊断条目，供 DecisionBanner 展示。
//
// 设计原则：
//   - 不持久化、不入库，只在 workflow-completed / loadAnalysis 时由 store 提取
//   - 只覆盖 portfolio-mgr input_mapping 中绑定的节点（即真正影响决策的数据）
//   - 不重复 data-quality.rhai 的职责（那个只看 10 个分析师的 LLM 输出质量）
//
// 节点来源：seed_stock_analysis.rs 中 portfolio-mgr 的 input_mapping + 显式边
// 共 16 个（raw-data 仅调度边，不消费数据，已排除）

/**
 * 诊断状态：missing=节点输出缺失；low=置信度低；untrusted=LLM 兜底；normal=正常
 */
export type DiagnosisStatus = "missing" | "low" | "untrusted" | "normal";

/**
 * 单个决策输入节点的诊断条目
 */
export interface DecisionInputDiagItem {
  /** 节点 ID（与 blackboard 键名一致） */
  nodeId: string;
  /** 中文角色名 */
  role: string;
  /** 因子分组：f1 技术面 / f2 共识 / ... / f11 PACE / 修正因子 / 哨兵 */
  factor: string;
  /** 因子权重（来自 portfolio-mgr.rhai 的 default 权重，用于提示影响程度） */
  weight: number | null;
  /** 实际 confidence（0-100），null 表示字段缺失或节点失败 */
  confidence: number | null;
  /** 节点的方向/立场（如 verdict/stance/risk_level 等），无则空字符串 */
  stance: string;
  /** 诊断状态 */
  status: DiagnosisStatus;
  /** 状态说明（缺失原因 / 低置信提示 / 兜底标记等） */
  note: string;
}

/**
 * 决策输入诊断报告：按因子分组的条目数组
 */
export type DecisionInputsReport = DecisionInputDiagItem[];

// ── 内部常量：portfolio-mgr 上游节点清单（按因子分组）──
// 权重值与 portfolio-mgr.rhai 中的 default 权重保持一致
interface NodeSpec {
  nodeId: string;
  role: string;
  factor: string;
  weight: number | null;
}

const NODE_SPECS: readonly NodeSpec[] = [
  { nodeId: "t-scoring", role: "技术评分", factor: "f1 技术面", weight: 0.15 },
  { nodeId: "debate-convergence", role: "辩论收敛", factor: "f2 共识", weight: 0.25 },
  { nodeId: "a-catalyst", role: "催化剂分析师", factor: "f3 催化剂", weight: 0.20 },
  { nodeId: "t-catalyst-data", role: "公告数据", factor: "f3 催化剂", weight: null },
  { nodeId: "cls-risk-level", role: "LLM 风险分类", factor: "f4 风险", weight: 0.15 },
  { nodeId: "t-risk", role: "算法风险", factor: "f4 风险", weight: null },
  { nodeId: "t-valuation", role: "估值", factor: "f5 估值", weight: 0.15 },
  { nodeId: "data-quality", role: "数据质量", factor: "f6 数据质量", weight: 0.15 },
  { nodeId: "trader", role: "交易员", factor: "f7 trader", weight: 0.15 },
  { nodeId: "t-hotmoney-data", role: "资金面", factor: "f9 资金面", weight: 0.08 },
  { nodeId: "t-lockup-data", role: "解禁数据", factor: "f10 筹码面", weight: 0.08 },
  { nodeId: "t-dragon-tiger-data", role: "龙虎榜", factor: "f10 筹码面", weight: null },
  { nodeId: "pace-calc", role: "PACE 情绪", factor: "f11 PACE", weight: 0.08 },
  { nodeId: "regime-weights", role: "市场状态权重", factor: "修正因子", weight: null },
  { nodeId: "risk-convergence", role: "风险收敛", factor: "修正因子", weight: null },
  { nodeId: "research-mgr", role: "研究经理", factor: "哨兵", weight: null },
] as const;

// ── 内部辅助：从节点输出对象中提取字段 ──

/** 安全取对象字段，支持点路径（如 "result.totalScore"） */
function getPath(obj: unknown, path: string): unknown {
  if (!obj || typeof obj !== "object") { return undefined; }
  let cur: unknown = obj;
  for (const seg of path.split(".")) {
    if (cur && typeof cur === "object") {
      cur = (cur as Record<string, unknown>)[seg];
    } else {
      return undefined;
    }
  }
  return cur;
}

/** 从 AgentNode 包装对象中提取 content 字段（兼容直接对象和 CodeNode 包装） */
function extractContentField(raw: unknown): unknown {
  if (!raw || typeof raw !== "object") { return raw; }
  const r = raw as Record<string, unknown>;
  // CodeNode 包装：{status, result, params, ...} → 取 result
  if (r.result != null && typeof r.result === "object") { return r.result; }
  // AgentNode 包装：{content, model, role, ...} → content 可能是 JSON 字符串
  if (typeof r.content === "string") {
    try {
      return JSON.parse(r.content);
    } catch {
      return r.content;
    }
  }
  if (r.content != null) { return r.content; }
  return raw;
}

/** 从 VERDICT 标签中提取 JSON 对象（辩手/风控节点使用） */
function extractVerdictTag(text: unknown): Record<string, unknown> | null {
  if (typeof text !== "string") { return null; }
  const m = text.match(/<!--\s*VERDICT:\s*(\{[^}]*\})\s*-->/);
  if (!m) { return null; }
  try {
    const parsed = JSON.parse(m[1]);
    return typeof parsed === "object" && parsed !== null ? parsed : null;
  } catch {
    return null;
  }
}

/** 把 confidence 转成数字（null 表示无） */
function toConf(v: unknown): number | null {
  if (typeof v === "number" && Number.isFinite(v)) { return v; }
  return null;
}

/** 置信度阈值：<30 视为低置信 */
const LOW_CONF_THRESHOLD = 30;

// ── 各节点的诊断逻辑：返回 [confidence, stance, note] ──
type NodeDiagResult = { confidence: number | null; stance: string; note: string };

function diagnoseNode(nodeId: string, raw: unknown): NodeDiagResult {
  if (raw == null) {
    return { confidence: null, stance: "", note: "节点输出缺失" };
  }
  const content = extractContentField(raw);

  switch (nodeId) {
    case "t-scoring": {
      const total = getPath(content, "totalScore");
      if (typeof total === "number") {
        return { confidence: null, stance: `total=${total.toFixed(0)}`, note: "算法输出" };
      }
      return { confidence: null, stance: "", note: "totalScore 字段缺失" };
    }
    case "debate-convergence": {
      const consensus = getPath(content, "consensus_score");
      const aggConf = getPath(content, "aggregate_prediction.confidence");
      const direction = getPath(content, "aggregate_prediction.direction");
      const conf = toConf(aggConf);
      return {
        confidence: conf,
        stance: typeof direction === "string" ? String(direction) : "",
        note: typeof consensus === "number" ? `共识=${consensus}` : "consensus_score 缺失",
      };
    }
    case "a-catalyst": {
      const verdict = getPath(content, "verdict");
      const conf = toConf(getPath(verdict, "confidence") ?? getPath(content, "confidence"));
      const level = getPath(content, "catalyst_level");
      return {
        confidence: conf,
        stance: typeof level === "string" ? String(level) : "",
        note: verdict ? "VERDICT 已输出" : "verdict 字段缺失",
      };
    }
    case "t-catalyst-data": {
      const arr = Array.isArray(content) ? content : getPath(content, "result");
      return {
        confidence: null,
        stance: Array.isArray(arr) ? `${arr.length}条` : "",
        note: Array.isArray(arr) && arr.length > 0 ? "数据已获取" : "公告列表为空",
      };
    }
    case "cls-risk-level": {
      const cat = getPath(content, "category");
      return {
        confidence: null,
        stance: typeof cat === "string" ? String(cat) : "",
        note: cat ? "分类已输出" : "category 字段缺失",
      };
    }
    case "t-risk": {
      const vol = getPath(content, "stockRiskProfile.annualizedVolatilityPct");
      return {
        confidence: null,
        stance: typeof vol === "number" ? `vol=${vol.toFixed(1)}%` : "",
        note: typeof vol === "number" ? "算法输出" : "stockRiskProfile 缺失",
      };
    }
    case "t-valuation": {
      const dcf = getPath(content, "dcf.upsidePct");
      const graham = getPath(content, "graham.upsidePct");
      return {
        confidence: null,
        stance: typeof dcf === "number" ? `DCF=${dcf.toFixed(1)}%` : "",
        note: dcf != null || graham != null ? "算法输出" : "估值字段缺失",
      };
    }
    case "data-quality": {
      // snapshot 中是 data_quality_summary，已提取；live results 中可能是 CodeNode 包装
      const grade = getPath(content, "grade");
      const score = toConf(getPath(content, "score"));
      return {
        confidence: score,
        stance: typeof grade === "string" ? `${grade}级` : "",
        note: grade ? "已评级" : "grade 字段缺失",
      };
    }
    case "trader": {
      const verdict = getPath(content, "verdict");
      const conf = toConf(getPath(content, "confidence"));
      return {
        confidence: conf,
        stance: typeof verdict === "string" ? String(verdict) : "",
        note: verdict ? "VERDICT 已输出" : "verdict 字段缺失",
      };
    }
    case "t-hotmoney-data": {
      const main = getPath(content, "main_net_inflow");
      return {
        confidence: null,
        stance: typeof main === "number" ? `主力净流入=${main.toFixed(0)}` : "",
        note: main != null ? "数据已获取" : "资金流向数据缺失",
      };
    }
    case "t-lockup-data": {
      const trades = getPath(content, "shareholder_trades");
      const lockup = getPath(content, "lockup_schedule");
      return {
        confidence: null,
        stance: Array.isArray(trades) ? `${trades.length}笔增减持` : "",
        note: trades != null || lockup != null ? "数据已获取" : "筹码数据缺失",
      };
    }
    case "t-dragon-tiger-data": {
      const arr = Array.isArray(content) ? content : getPath(content, "result");
      return {
        confidence: null,
        stance: Array.isArray(arr) ? `${arr.length}条` : "",
        note: Array.isArray(arr) && arr.length > 0 ? "数据已获取" : "龙虎榜数据为空",
      };
    }
    case "pace-calc": {
      const sig = getPath(content, "pace_signal");
      const degraded = getPath(content, "pace_degraded");
      return {
        confidence: null,
        stance: typeof sig === "number" ? `signal=${sig.toFixed(2)}` : "",
        note: degraded === true ? "已降级" : sig != null ? "算法输出" : "pace_signal 缺失",
      };
    }
    case "regime-weights": {
      const w = getPath(content, "factor_weights");
      return {
        confidence: null,
        stance: typeof w === "object" && w !== null ? "已输出" : "",
        note: w ? "权重表已生成" : "factor_weights 缺失",
      };
    }
    case "risk-convergence": {
      // risk-convergence 输出可能含 VERDICT 标签或直接 JSON
      const verdict = extractVerdictTag(typeof content === "string" ? content : "");
      const level = getPath(verdict ?? content, "converged_risk_level");
      const disagreement = getPath(verdict ?? content, "disagreement_score");
      const conf = toConf(getPath(verdict ?? content, "confidence"));
      return {
        confidence: conf,
        stance: typeof level === "string" ? String(level) : "",
        note: typeof disagreement === "number" ? `分歧=${disagreement}` : "disagreement 缺失",
      };
    }
    case "research-mgr": {
      // research-mgr 输出纯文本投资计划，无机读字段，只检查是否非空
      const text = typeof content === "string" ? content : JSON.stringify(content ?? "");
      return {
        confidence: null,
        stance: "",
        note: text.trim().length > 0 ? "已输出文本" : "输出为空",
      };
    }
    default:
      return { confidence: null, stance: "", note: "未知节点" };
  }
}

/**
 * 从 workflow results / blackboard snapshot 提取决策输入诊断报告
 *
 * @param results workflow-completed 的 results 或 loadAnalysis 解析的 snap
 * @param untrustedNodes 已知的 strict_mode 兜底节点集合
 */
export function buildDecisionInputsReport(
  results: Record<string, unknown> | null | undefined,
  untrustedNodes: Record<string, true> | null | undefined,
): DecisionInputsReport {
  if (!results || typeof results !== "object") { return []; }

  const report: DecisionInputDiagItem[] = [];

  for (const spec of NODE_SPECS) {
    const raw = results[spec.nodeId];
    const isUntrusted = untrustedNodes?.[spec.nodeId] === true;
    const diag = diagnoseNode(spec.nodeId, raw);

    let status: DiagnosisStatus;
    if (raw == null) {
      status = "missing";
    } else if (isUntrusted) {
      status = "untrusted";
    } else if (diag.confidence !== null && diag.confidence < LOW_CONF_THRESHOLD) {
      status = "low";
    } else {
      status = "normal";
    }

    let note = diag.note;
    if (isUntrusted) {
      note = `strict_mode 兜底（LLM 输出无法解析）｜${note}`;
    }

    report.push({
      nodeId: spec.nodeId,
      role: spec.role,
      factor: spec.factor,
      weight: spec.weight,
      confidence: diag.confidence,
      stance: diag.stance,
      status,
      note,
    });
  }

  return report;
}

/** 统计：返回各状态的计数，便于面板顶部展示 */
export function summarizeDecisionInputs(
  report: DecisionInputsReport,
): { total: number; missing: number; low: number; untrusted: number; normal: number } {
  const acc = { total: report.length, missing: 0, low: 0, untrusted: 0, normal: 0 };
  for (const item of report) {
    acc[item.status] += 1;
  }
  return acc;
}

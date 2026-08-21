/**
 * 股票分析组件共享工具函数
 *
 * 统一导出自 @/lib/agentOutput，避免重复实现。
 * 所有组件通过 `./utils` 导入保持向后兼容。
 */
export { cleanToolCallTags, tryBeautifyJson } from "@/lib/agentOutput";
import i18n from "@/i18n";
import { cleanToolCallTags } from "@/lib/agentOutput";

function looksLikeJson(text: string): boolean {
  const trimmed = text.trim();
  return (
    (trimmed.startsWith("{") && trimmed.endsWith("}"))
    || (trimmed.startsWith("[") && trimmed.endsWith("]"))
  );
}

/**
 * 尝试从风险报告 JSON 中提取可读 Markdown 文本
 * 支持 a-risk / trader / portfolio-manager 等多种输出结构
 * 支持 strict_mode 嵌套格式：{"report":"...","verdict":{"stance":"aggressive","position_pct":50,"confidence":70}}
 *
 * 单一事实来源：RiskMatrix 与组件共用此实现，避免两处解析逻辑漂移。
 */
export function extractReadableFromRiskReport(report: string): string {
  const cleaned = cleanToolCallTags(report);
  const trimmed = cleaned.trim();

  if (!looksLikeJson(trimmed)) { return cleaned; }

  try {
    const parsed = JSON.parse(trimmed);
    if (typeof parsed !== "object" || parsed === null) { return cleaned; }

    // strict_mode 嵌套 verdict：从 parsed.verdict 提升字段到顶层
    const v = parsed.verdict && typeof parsed.verdict === "object" && !Array.isArray(parsed.verdict)
      ? { ...parsed, ...parsed.verdict }
      : parsed;

    const parts: string[] = [];

    // 1. 立场/风格（支持顶层 + verdict 嵌套）
    if (typeof v.stance === "string") {
      parts.push(i18n.t("stockAnalysis.riskReport.stance", { stance: v.stance }));
    }

    // 2. 仓位/头寸（支持 positionPct / position_pct）
    const posPct = v.positionPct ?? v.position_pct;
    if (typeof posPct === "number") {
      parts.push(i18n.t("stockAnalysis.riskReport.positionPct", { posPct }));
    }

    // 3. 信心度
    if (typeof v.confidence === "number") {
      parts.push(i18n.t("stockAnalysis.riskReport.confidence", { confidence: v.confidence }));
    }

    // 4. 风险等级（支持 riskLevel / risk_level / converged_risk_level）
    const riskLevel = v.riskLevel ?? v.risk_level ?? v.converged_risk_level;
    if (typeof riskLevel === "string") {
      parts.push(i18n.t("stockAnalysis.riskReport.riskLevel", { riskLevel }));
    }

    // 5. 摘要/分析/推理
    const textFields = ["summary", "risk_analysis", "analysis", "reasoning", "report", "content", "text", "detail"];
    for (const field of textFields) {
      if (typeof parsed[field] === "string" && parsed[field].length > 5) {
        parts.push(parsed[field]);
      }
    }

    // 6. Kelly 公式参数
    if (parsed.kelly_inputs && typeof parsed.kelly_inputs === "object") {
      const k = parsed.kelly_inputs;
      const kParts: string[] = [];
      if (typeof k.win_rate === "number") {
        kParts.push(i18n.t("stockAnalysis.riskReport.kellyWinRate", { pct: (k.win_rate * 100).toFixed(0) }));
      }
      if (typeof k.payoff_ratio === "number") {
        kParts.push(i18n.t("stockAnalysis.riskReport.kellyPayoff", { ratio: k.payoff_ratio }));
      }
      if (typeof k.raw_kelly === "number") {
        kParts.push(i18n.t("stockAnalysis.riskReport.kellyRaw", { pct: (k.raw_kelly * 100).toFixed(1) }));
      }
      if (typeof k.scale_factor === "number") {
        kParts.push(i18n.t("stockAnalysis.riskReport.kellyScale", { factor: k.scale_factor }));
      }
      if (kParts.length > 0) {
        parts.push(i18n.t("stockAnalysis.riskReport.kellyParams", { params: kParts.join("，") }));
      }
    }

    // 7. 非对称机会
    if (Array.isArray(parsed.asymmetric_opportunities) && parsed.asymmetric_opportunities.length > 0) {
      parts.push(i18n.t("stockAnalysis.riskReport.asymOpportunities"));
      for (const opp of parsed.asymmetric_opportunities) {
        if (typeof opp.opportunity === "string") {
          parts.push(`- ${opp.opportunity}`);
        }
        if (Array.isArray(opp.evidence_refs)) {
          for (const ref of opp.evidence_refs) {
            if (typeof ref === "string") { parts.push(i18n.t("stockAnalysis.riskReport.evidenceRef", { ref })); }
          }
        }
        if (typeof opp.expected_value === "string") {
          parts.push(i18n.t("stockAnalysis.riskReport.expectedValue", { value: opp.expected_value }));
        }
      }
    }

    // 8. 执行备注
    if (Array.isArray(parsed.execution_notes) && parsed.execution_notes.length > 0) {
      parts.push(i18n.t("stockAnalysis.riskReport.executionNotes"));
      for (const note of parsed.execution_notes) {
        if (typeof note === "string") { parts.push(`- ${note}`); }
      }
    } else if (typeof parsed.execution_notes === "string" && parsed.execution_notes.length > 5) {
      parts.push(i18n.t("stockAnalysis.riskReport.executionNotesInline", { notes: parsed.execution_notes }));
    }

    // 9. 风险项列表
    if (Array.isArray(parsed.risk_items) && parsed.risk_items.length > 0) {
      parts.push(i18n.t("stockAnalysis.riskReport.riskItems"));
      for (const item of parsed.risk_items) {
        if (typeof item.risk === "string") {
          const severity = typeof item.severity === "string" ? `（${item.severity}）` : "";
          parts.push(`- ${item.risk}${severity}`);
        }
        if (Array.isArray(item.evidence_refs)) {
          for (const ref of item.evidence_refs) {
            if (typeof ref === "string") { parts.push(i18n.t("stockAnalysis.riskReport.evidenceRef", { ref })); }
          }
        }
      }
    }

    // 10. 关键条件跟踪
    if (Array.isArray(parsed.key_conditions_to_track) && parsed.key_conditions_to_track.length > 0) {
      parts.push(i18n.t("stockAnalysis.riskReport.keyConditions"));
      for (const cond of parsed.key_conditions_to_track) {
        if (typeof cond === "string") { parts.push(`- ${cond}`); }
      }
    }

    // 11. 多空核心论据
    if (Array.isArray(parsed.decisive_bull_acks) && parsed.decisive_bull_acks.length > 0) {
      parts.push(i18n.t("stockAnalysis.riskReport.bullArgs"));
      for (const ack of parsed.decisive_bull_acks) {
        if (typeof ack === "string") { parts.push(`- ${ack}`); }
      }
    }
    if (Array.isArray(parsed.decisive_bear_acks) && parsed.decisive_bear_acks.length > 0) {
      parts.push(i18n.t("stockAnalysis.riskReport.bearArgs"));
      for (const ack of parsed.decisive_bear_acks) {
        if (typeof ack === "string") { parts.push(`- ${ack}`); }
      }
    }

    // 12. 止损/止盈
    if (typeof parsed.stopLossPct === "number") {
      parts.push(i18n.t("stockAnalysis.riskReport.stopLoss", { pct: parsed.stopLossPct }));
    }
    if (typeof parsed.takeProfitPct === "number") {
      parts.push(i18n.t("stockAnalysis.riskReport.takeProfit", { pct: parsed.takeProfitPct }));
    }

    if (parts.length > 0) {
      return parts.join("\n\n");
    }

    // 兜底：提取所有字符串值
    for (const [key, value] of Object.entries(parsed)) {
      if (typeof value === "string" && value.length > 10) {
        parts.push(`**${key}**: ${value}`);
      }
    }
    if (parts.length > 0) { return parts.join("\n\n"); }
  } catch {
    // 解析失败回退
  }

  return cleaned;
}

/** VERDICT 标签正则（<!-- VERDICT: {...} --> 旧格式） */
export const VERDICT_RE = /<!--\s*VERDICT\s*:\s*(\{[^}]*\})\s*-->/i;

/**
 * 尝试从文本中提取 VERDICT JSON 的指定字段
 * 支持 strict_mode 嵌套 verdict（{verdict:{...}}）与旧格式 HTML 注释两种来源。
 * 单一事实来源：风险评分共用此解析，避免 VERDICT 解析逻辑散落各处。
 */
export function parseVerdictField(text: string, field: string): number | null {
  // 1. 先尝试从 strict_mode JSON 的嵌套 verdict 中提取
  if (field === "position_pct" || field === "converged_position_pct" || field === "confidence") {
    try {
      const parsed = JSON.parse(text);
      if (parsed && typeof parsed === "object" && !Array.isArray(parsed)) {
        const verdict = parsed.verdict;
        if (verdict && typeof verdict === "object" && !Array.isArray(verdict)) {
          // field 直接匹配
          if (typeof verdict[field] === "number") { return Math.round(verdict[field]); }
          // position_pct 未找到 → 查 converged_position_pct（risk-convergence 节点）
          if (field === "position_pct" && typeof verdict.converged_position_pct === "number") {
            return Math.round(verdict.converged_position_pct);
          }
        }
      }
    } catch {
      /* 不是合法 JSON，继续下一方案 */
    }
  }

  // 2. 尝试从 <!-- VERDICT: {...} --> HTML 注释中提取（旧格式）
  const m = text.match(VERDICT_RE);
  if (!m?.[1]) { return null; }
  try {
    const v = JSON.parse(m[1]);
    if (typeof v[field] === "number") { return Math.round(v[field]); }
  } catch {
    /* 不是合法 JSON */
  }
  return null;
}

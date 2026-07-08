/**
 * 股票分析组件共享工具函数
 *
 * 统一导出自 @/lib/agentOutput，避免重复实现。
 * 所有组件通过 `./utils` 导入保持向后兼容。
 */
export { cleanToolCallTags, tryBeautifyJson } from "@/lib/agentOutput";
import { cleanToolCallTags } from "@/lib/agentOutput";

function looksLikeJson(text: string): boolean {
  const trimmed = text.trim();
  return (
    (trimmed.startsWith("{") && trimmed.endsWith("}"))
    || (trimmed.startsWith("[") && trimmed.endsWith("]"))
  );
}

/** 从风险报告的 JSON 中提取可读文本 */
export function extractReadableFromRiskReport(report: string): string {
  const cleaned = cleanToolCallTags(report);
  const trimmed = cleaned.trim();

  if (!looksLikeJson(trimmed)) { return cleaned; }

  try {
    const parsed = JSON.parse(trimmed);
    if (typeof parsed !== "object" || parsed === null) { return cleaned; }

    const parts: string[] = [];

    // 1. 立场/风格
    if (typeof parsed.stance === "string") {
      parts.push(`**立场**: ${parsed.stance}`);
    }
    // 2. 仓位/头寸
    if (typeof parsed.positionPct === "number") {
      parts.push(`**建议仓位**: ${parsed.positionPct}%`);
    }
    // 3. 信心度
    if (typeof parsed.confidence === "number") {
      parts.push(`**信心度**: ${parsed.confidence}%`);
    }
    // 4. 风险等级
    if (typeof parsed.riskLevel === "string") {
      parts.push(`**风险等级**: ${parsed.riskLevel}`);
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
      if (typeof k.win_rate === "number") { kParts.push(`胜率 ${(k.win_rate * 100).toFixed(0)}%`); }
      if (typeof k.payoff_ratio === "number") { kParts.push(`赔率 ${k.payoff_ratio}`); }
      if (typeof k.raw_kelly === "number") { kParts.push(`原始 Kelly ${(k.raw_kelly * 100).toFixed(1)}%`); }
      if (typeof k.scale_factor === "number") { kParts.push(`缩放因子 ${k.scale_factor}`); }
      if (kParts.length > 0) { parts.push(`**Kelly 参数**: ${kParts.join("，")}`); }
    }
    // 7. 非对称机会
    if (Array.isArray(parsed.asymmetric_opportunities)) {
      for (const item of parsed.asymmetric_opportunities) {
        if (typeof item === "string") { parts.push(item); }
        else if (typeof item?.desc === "string") { parts.push(item.desc); }
      }
    }
    // 8. 执行备注
    if (Array.isArray(parsed.execution_notes)) {
      for (const note of parsed.execution_notes) {
        if (typeof note === "string") { parts.push(note); }
        else if (typeof note?.content === "string") { parts.push(note.content); }
      }
    }
    // 9. 风险项列表
    if (Array.isArray(parsed.risk_items)) {
      for (const item of parsed.risk_items) {
        if (typeof item === "string") { parts.push(`- ${item}`); }
        else if (typeof item?.description === "string") { parts.push(`- ${item.description}`); }
        else if (typeof item?.risk === "string") { parts.push(`- ${item.risk}`); }
      }
    }
    // 10. 关键条件跟踪
    if (Array.isArray(parsed.key_conditions_to_track)) {
      for (const cond of parsed.key_conditions_to_track) {
        if (typeof cond === "string") { parts.push(`📌 ${cond}`); }
      }
    }
    // 11. 多空核心论据
    if (Array.isArray(parsed.decisive_bull_acks)) {
      for (const arg of parsed.decisive_bull_acks) {
        if (typeof arg === "string") { parts.push(`📈 ${arg}`); }
      }
    }
    if (Array.isArray(parsed.decisive_bear_acks)) {
      for (const arg of parsed.decisive_bear_acks) {
        if (typeof arg === "string") { parts.push(`📉 ${arg}`); }
      }
    }
    // 12. 止损/止盈
    if (typeof parsed.stopLossPct === "number") { parts.push(`**止损**: -${parsed.stopLossPct}%`); }
    if (typeof parsed.takeProfitPct === "number") { parts.push(`**止盈**: +${parsed.takeProfitPct}%`); }

    if (parts.length > 0) { return parts.join("\n\n"); }

    // 兜底：提取所有字符串值
    for (const [, value] of Object.entries(parsed)) {
      if (typeof value === "string" && value.length > 10) { parts.push(value); }
    }
    if (parts.length > 0) { return parts.join("\n\n"); }
  } catch { /* 解析失败回退 */ }

  return cleaned;
}

/**
 * 股票分析报告导出工具
 * 生成包含所有分析数据的结构化 Markdown 报告，
 * 并通过 Tauri 命令导出为 DOCX / PPTX。
 */

import { invoke } from "@/lib/invoke";
import { isTauri } from "@/lib/invoke";
import type { StockDecision } from "@/types/stock-analysis";
import type { TFunction } from "i18next";

// ── 数据契约 ──

export interface ExportData {
  stockCode: string;
  stockName: string;
  asOfDate: string | null;
  quote:
    | { price: number; change: number; changePct: number; high: number; low: number; volume: number; amount: number }
    | null;
  analystReports: Record<string, string>;
  debateRounds: Array<{ round: number; bull: string; bear: string }>;
  riskAssessments: Record<string, string>;
  valueAssessments: Record<string, string>;
  decision: StockDecision | null;
  llmDecisionJson: string | null;
  dataQualitySummary: string;
  ruleCheckResults: Record<string, string>;
  rawData: Record<string, string>;
  failedNodes: string[];
  dataWarnings: string[];
}

// ── 分析师名称映射 ──

const ANALYST_LABELS: Record<string, string> = {
  "market-analyst": "📊 技术面分析师",
  sentiment: "😊 情绪面分析师",
  news: "📰 消息面分析师",
  fundamentals: "📈 基本面分析师",
  policy: "🏛️ 政策面分析师",
  "hot-money": "💰 资金面分析师",
  lockup: "🔒 解禁观察员",
  research: "📚 券商研报分析师",
  sector: "🏭 板块题材分析师",
  catalyst: "💥 催化剂与叙事分析师",
};

// ── 辅助函数 ──

function safeJson(raw: string): string {
  try {
    return JSON.stringify(JSON.parse(raw), null, 2);
  } catch {
    return raw;
  }
}

function truncateLong(raw: string, maxLen = 3000): string {
  if (raw.length <= maxLen) { return raw; }
  return raw.slice(0, maxLen) + "\n\n> ⚠️ 内容过长已截断，完整内容请查看原分析页面";
}

/** 从 LLM 文本中提取 readable 摘要 */
function extractSummary(raw: string): string {
  try {
    const parsed = JSON.parse(raw);
    return parsed.summary || parsed.analysis || parsed.argument || parsed.reasoning || raw.slice(0, 200);
  } catch {
    return raw.replace(/```json\s*/g, "").replace(/```\s*/g, "").trim().slice(0, 200);
  }
}

/** 从 score 字段提取标签 */
function extractScoreTags(raw: string): string[] {
  const tags: string[] = [];
  try {
    const p = JSON.parse(raw);
    if (typeof p.bull_score === "number") { tags.push(`看多:${p.bull_score}`); }
    if (typeof p.bear_score === "number") { tags.push(`看空:${p.bear_score}`); }
    if (typeof p.bull_strength_score === "number") { tags.push(`多方强度:${p.bull_strength_score}`); }
    if (typeof p.bear_strength_score === "number") { tags.push(`空方强度:${p.bear_strength_score}`); }
    if (typeof p.confidence === "number") { tags.push(`置信度:${p.confidence}`); }
    if (p.stance) { tags.push(`立场:${p.stance}`); }
    if (p.action) { tags.push(`操作:${p.action}`); }
  } catch { /* ignore */ }
  return tags;
}

// ═════════════════════════════════════════════════════════════════════════
//  Markdown 报告构建
// ═════════════════════════════════════════════════════════════════════════

export function buildMarkdownReport(data: ExportData): string {
  const lines: string[] = [];
  const now = new Date().toLocaleString("zh-CN");
  const sep = "\n---\n";
  const h = (level: number, text: string) => lines.push(`${"#".repeat(level)} ${text}\n`);

  // ── 封面 ──
  h(1, `股票分析报告：${data.stockName}（${data.stockCode}）`);
  lines.push(`> **导出时间**：${now}`);
  if (data.asOfDate) { lines.push(`> **时间回溯**：${data.asOfDate}`); }
  if (data.failedNodes.length > 0) {
    lines.push(`> ⚠️ **部分节点执行失败**：${data.failedNodes.join("、")}`);
  }
  lines.push(sep);

  // ── 1. 行情概览 ──
  if (data.quote) {
    h(2, "📊 行情概览");
    const q = data.quote;
    lines.push(`| 指标 | 数值 |`);
    lines.push(`|------|------|`);
    lines.push(`| 现价 | ¥${q.price.toFixed(2)} |`);
    lines.push(`| 涨跌幅 | ${q.changePct >= 0 ? "+" : ""}${q.changePct.toFixed(2)}% |`);
    lines.push(`| 涨跌额 | ${q.change >= 0 ? "+" : ""}${q.change.toFixed(2)} |`);
    lines.push(`| 最高 | ¥${q.high.toFixed(2)} |`);
    lines.push(`| 最低 | ¥${q.low.toFixed(2)} |`);
    lines.push(`| 成交量 | ${(q.volume / 10000).toFixed(0)} 万手 |`);
    lines.push(`| 成交额 | ¥${(q.amount / 100000000).toFixed(2)} 亿 |`);
    lines.push("");
  }

  // ── 2. 最终决策 ──
  h(2, "🎯 最终决策");
  if (data.decision) {
    const d = data.decision;
    lines.push(`| 指标 | 数值 |`);
    lines.push(`|------|------|`);
    const actionLabel = (d.action as string) === "无法判断" ? `**${d.action}** ⚠️` : `**${d.action}**`;
    lines.push(`| 操作建议 | ${actionLabel} |`);
    lines.push(`| 置信度 | ${Math.round(d.confidence ?? 0)}% |`);
    lines.push(`| 建议仓位 | ${d.positionPct}% |`);
    lines.push(`| 风险等级 | ${d.riskLevel} |`);
    lines.push(`| 目标价 | ${d.targetPrice ?? "—"} |`);
    lines.push(`| 止损价 | ${d.stopLoss ?? "—"} |`);
    if (d.expectedHoldingDays) { lines.push(`| 持有天数 | ${d.expectedHoldingDays} 天 |`); }
    if (d.targetTimeframe) { lines.push(`| 目标时间框架 | ${d.targetTimeframe} |`); }
    lines.push("");
    if (d.reasoning) {
      h(3, "决策理由");
      lines.push(`> ${d.reasoning}\n`);
    }
    const dRecord = d as unknown as Record<string, unknown>;
    // 数据不足时展示缺失数据源
    const dataGaps = dRecord.data_gaps;
    if (Array.isArray(dataGaps) && dataGaps.length > 0) {
      h(3, "缺失数据源");
      for (const g of dataGaps) {
        if (g) { lines.push(`- ${g}\n`); }
      }
      lines.push("");
    }
    // 多时间维度信号
    const signals: Record<string, { action: string; confidence: number }> = {};
    for (const k of ["ultraShortTermSignal", "shortTermSignal", "midTermSignal", "longTermSignal"] as const) {
      const s = dRecord[k] as { action?: string; confidence?: number } | undefined;
      if (s?.action) { signals[k] = { action: s.action, confidence: s.confidence ?? 0 }; }
    }
    if (Object.keys(signals).length > 0) {
      h(3, "多时间维度信号");
      const labelMap: Record<string, string> = {
        ultraShortTermSignal: "超短线",
        shortTermSignal: "短线",
        midTermSignal: "中线",
        longTermSignal: "长线",
      };
      lines.push(`| 维度 | 操作 | 置信度 |`);
      lines.push(`|------|------|--------|`);
      for (const [k, v] of Object.entries(signals)) {
        lines.push(`| ${labelMap[k] || k} | ${v.action} | ${Math.round(v.confidence)}% |`);
      }
      lines.push("");
    }
  } else {
    // 决策为空 → 展示失败上下文
    lines.push(`> ⚠️ **决策未生成**：portfolio-mgr 节点未产出有效决策\n`);
    if (data.failedNodes.length > 0) {
      lines.push(`\n**相关失败节点**：${data.failedNodes.join("、")}\n`);
    }
    if (data.dataWarnings.length > 0) {
      lines.push("\n**数据警告**：\n");
      for (const w of data.dataWarnings) { lines.push(`- ${w}\n`); }
    }
    // 尝试从 rawData 提取 portfolio-mgr 原始输出
    const pmRaw = data.rawData?.["portfolio-mgr"] || data.rawData?.["portfolio-manager"];
    if (pmRaw) {
      h(3, "portfolio-mgr 原始输出");
      lines.push(`\`\`\`\n${truncateLong(pmRaw, 2000)}\n\`\`\`\n`);
    }
  }
  lines.push(sep);

  // ── 3. 分析师报告 ──
  const analystKeys = Object.keys(data.analystReports);
  if (analystKeys.length > 0) {
    h(2, `📋 分析师报告（共 ${analystKeys.length} 份）\n`);
    for (const key of analystKeys) {
      const raw = data.analystReports[key];
      if (!raw || raw.trim().length === 0) { continue; }
      const label = ANALYST_LABELS[key] || key;
      h(3, label);
      const tags = extractScoreTags(raw);
      if (tags.length > 0) { lines.push(`> ${tags.join(" · ")}\n`); }
      lines.push(truncateLong(safeJson(raw), 2000));
      lines.push("\n");
    }
    lines.push(sep);
  }

  // ── 4. 辩论记录 ──
  if (data.debateRounds.length > 0) {
    h(2, `⚖️ 多空辩论（共 ${data.debateRounds.length} 轮）\n`);
    for (const round of data.debateRounds) {
      h(3, `第 ${round.round} 轮`);
      if (round.bull) {
        lines.push(`**多方（看涨）**：\n\n${truncateLong(extractSummary(round.bull), 1500)}\n`);
      }
      if (round.bear) {
        lines.push(`**空方（看跌）**：\n\n${truncateLong(extractSummary(round.bear), 1500)}\n`);
      }
    }
    lines.push(sep);
  }

  // ── 5. 风险评估 ──
  const riskKeys = Object.keys(data.riskAssessments);
  if (riskKeys.length > 0) {
    h(2, "🛡️ 风险评估\n");
    const riskLabels: Record<string, string> = {
      "risk-agg": "激进型",
      "risk-con": "保守型",
      "risk-neu": "中性",
      "risk-aggregated": "聚合风险",
      "risk-level": "风险等级",
      "risk-convergence": "风险收敛",
      "research-mgr": "研究经理",
    };
    for (const key of riskKeys) {
      const raw = data.riskAssessments[key];
      if (!raw || raw.trim().length === 0) { continue; }
      h(3, riskLabels[key] || key);
      lines.push(truncateLong(safeJson(raw), 1500));
      lines.push("\n");
    }
    lines.push(sep);
  }

  // ── 6. 估值分析 ──
  const valKeys = Object.keys(data.valueAssessments);
  if (valKeys.length > 0) {
    h(2, "💎 估值分析（巴菲特框架）\n");
    for (const key of valKeys) {
      const raw = data.valueAssessments[key];
      if (!raw || raw.trim().length === 0) { continue; }
      h(3, key);
      lines.push(truncateLong(safeJson(raw), 2000));
      lines.push("\n");
    }
    lines.push(sep);
  }

  // ── 7. 数据质量 ──
  if (data.dataQualitySummary) {
    h(2, "🔍 数据质量");
    lines.push(`\n${data.dataQualitySummary}\n`);
    lines.push(sep);
  }

  // ── 8. 降级与警告 ──
  if (data.failedNodes.length > 0 || data.dataWarnings.length > 0) {
    h(2, "⚠️ 降级与警告");
    if (data.failedNodes.length > 0) {
      lines.push(`\n**执行失败的节点**：${data.failedNodes.join("、")}\n`);
    }
    if (data.dataWarnings.length > 0) {
      lines.push("\n**数据警告**：\n");
      for (const w of data.dataWarnings) { lines.push(`- ${w}\n`); }
    }
    lines.push(sep);
  }

  // ── 页脚 ──
  h(2, "📎 附录");
  lines.push(`- **报告生成工具**：AxInvest v2.6`);
  lines.push(`- **股票代码**：${data.stockCode}`);
  lines.push(`- **股票名称**：${data.stockName}`);
  lines.push(`- **分析师报告数**：${analystKeys.length}`);
  lines.push(`- **辩论轮次**：${data.debateRounds.length}`);
  lines.push(`- **风险评估项**：${riskKeys.length}`);
  lines.push("");

  return lines.join("");
}

// ═════════════════════════════════════════════════════════════════════════
//  导出处理
// ═════════════════════════════════════════════════════════════════════════

export type ExportFormat = "md" | "docx" | "pptx";

function downloadBlob(content: string, filename: string, mime: string) {
  const blob = new Blob([content], { type: mime });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = filename;
  a.click();
  URL.revokeObjectURL(url);
}

/** 导出分析报告 */
export async function exportAnalysisReport(
  data: ExportData,
  format: ExportFormat,
  t: TFunction,
): Promise<string> {
  const markdown = buildMarkdownReport(data);
  const baseName = `AxInvest_${data.stockCode}_${new Date().toISOString().slice(0, 10)}`;

  if (format === "md") {
    const content = markdown;
    if (isTauri()) {
      const { save } = await import("@tauri-apps/plugin-dialog");
      const { writeTextFile } = await import("@tauri-apps/plugin-fs");
      const path = await save({ defaultPath: `${baseName}.md`, filters: [{ name: "Markdown", extensions: ["md"] }] });
      if (!path) { return t("stockAnalysis.exportCancelled"); }
      await writeTextFile(path, content);
      return t("stockAnalysis.exportSaved", { path });
    }
    downloadBlob(content, `${baseName}.md`, "text/markdown;charset=utf-8");
    return t("stockAnalysis.exportDownloaded", { name: `${baseName}.md` });
  }

  if (format === "docx") {
    if (!isTauri()) { return t("stockAnalysis.docxDesktopOnly"); }
    const { save } = await import("@tauri-apps/plugin-dialog");
    const path = await save({
      defaultPath: `${baseName}.docx`,
      filters: [{ name: t("stockAnalysis.docxFilterName"), extensions: ["docx"] }],
    });
    if (!path) { return t("stockAnalysis.exportCancelled"); }
    return await invoke<string>("export_md_to_docx", {
      markdown,
      outputPath: path,
      title: `${data.stockName}（${data.stockCode}）股票分析报告`,
    });
  }

  if (format === "pptx") {
    if (!isTauri()) { return t("stockAnalysis.pptxDesktopOnly"); }
    const { save } = await import("@tauri-apps/plugin-dialog");
    const path = await save({
      defaultPath: `${baseName}.pptx`,
      filters: [{ name: t("stockAnalysis.pptxFilterName"), extensions: ["pptx"] }],
    });
    if (!path) { return t("stockAnalysis.exportCancelled"); }
    return await invoke<string>("export_md_to_pptx", {
      markdown,
      outputPath: path,
      title: `${data.stockName}（${data.stockCode}）股票分析报告`,
    });
  }

  return t("stockAnalysis.exportUnsupportedFormat", { format });
}

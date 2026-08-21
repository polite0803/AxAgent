// i18n-exempt: 业务逻辑判断字符串，非 UI 展示文本
import { getSignalColor } from "@/lib/stock-analysis-utils";
import { useSettingsStore } from "@/stores";
import { ExpandOutlined, SafetyCertificateOutlined } from "@ant-design/icons";
import { Button, Card, Collapse, Empty, Modal, Tag } from "antd";
import { setCustomComponents } from "markstream-react";
import type { RenderContext, RenderNodeFn } from "markstream-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { AnalystDataQualityModal } from "./AnalystDataQualityModal";
import { ReportMarkdown } from "./ReportMarkdown";
import { cleanToolCallTags, tryBeautifyJson } from "./utils";

// ── 自定义段落渲染：用 <div> 替代 <p> ─────────────────────────
// 解决 React DOM 嵌套警告：<div> cannot be a descendant of <p>
// markstream-react FallbackComponent 对未知内联节点类型渲染 <div>，
// 在 <p> 内渲染 <div> 违反 HTML 规范。
// 这里用官方 API 全局注册一个段落组件，用 <div> 包裹内容而非 <p>，
// 同时复用库自身的子节点渲染管线，无需修改 node_modules。
let customParagraphRegistered = false;

function registerCustomParagraph() {
  if (customParagraphRegistered) { return; }
  customParagraphRegistered = true;

  const CustomParagraph: React.FC<{
    node: Record<string, unknown> & { children?: unknown[] };
    ctx?: RenderContext;
    renderNode?: RenderNodeFn;
    indexKey?: React.Key;
  }> = ({ node, ctx, renderNode, indexKey }) => {
    if (!ctx || !renderNode || !node?.children?.length) {
      return <div dir="auto" className="paragraph-node" />;
    }

    const children = (node.children as Array<Record<string, unknown>>).map(
      (child: Record<string, unknown>, i: number) =>
        renderNode(child as Parameters<RenderNodeFn>[0], `${String(indexKey ?? "paragraph")}-${i}`, ctx),
    );

    return <div dir="auto" className="paragraph-node">{children}</div>;
  };

  setCustomComponents({ paragraph: CustomParagraph });
}

registerCustomParagraph();

interface Props {
  expertId: string;
  report: string;
}

/** 宽松解析：支持多种后端 JSON 格式 */
interface ParsedReport {
  // 标准格式
  type?: string;
  summary?: string;
  signals?: string[];
  risk_flags?: string[];
  argument?: string;
  key_points?: string[];
  confidence?: number;
  // 辩论格式
  core_arguments?: string[];
  resonance_points?: string[];
  preempted_counter_attacks?: string[];
  bull_strength_score?: number;
  bear_strength_score?: number;
  data_gaps?: string[];
  // 资金面/筹码面格式
  main_flow_state?: string;
  active_player?: string;
  dragon_tiger_signal?: string;
  limit_up_sustainability?: string;
  bull_score?: number;
  bear_score?: number;
  trigger_bull?: string;
  trigger_bear?: string;
  evidence?: Array<{ point?: string; data?: string; weight?: number }>;
  // 估值格式
  expert?: string;
  business_model?: string;
  moat_rating?: string;
  moat_reasoning?: string;
  financial_health?: string;
  intrinsic_value_range?: string | null;
  margin_of_safety?: string;
  buffett_verdict?: string;
  ideal_buy_price?: string | null;
  catalyst_detail?: string;
  catalyst_level?: string;
  narrative_completeness?: string;
  narrative_missing?: string[];
  institutional_trace?: string;
  concept_risk?: string;
  key_events?: Array<{ event?: string; source?: string; stance?: string; weight?: number }>;
  // 通用分析
  analysis?: string;
  assessment?: string;
  verdict?: string;
  reasoning?: string;
  stance?: string;
  positionPct?: number;
  action?: string;
  [key: string]: unknown;
}

/** 解析 `<!-- VERDICT: {...} -->` 格式（分析师自由文本 + 末尾 verdict 标签）*/
function tryParseVerdictFormat(report: string): ParsedReport | null {
  const trimmed = report.trim();
  const verdictIdx = trimmed.indexOf("<!-- VERDICT:");
  if (verdictIdx === -1) { return null; }
  try {
    const jsonStr = trimmed.slice(verdictIdx + "<!-- VERDICT:".length);
    const jsonEnd = jsonStr.indexOf("-->");
    if (jsonEnd === -1) { return null; }
    const meta = JSON.parse(jsonStr.slice(0, jsonEnd).trim());
    const summary = trimmed.slice(0, verdictIdx).trim();
    // 兼容两种字段命名：
    //   分析师节点: verdict / bull_score / bear_score / confidence
    //   辩论节点:   stance / strength_score / confidence（无 bear_score）
    const bull = meta.bull_score ?? meta.strength_score ?? undefined;
    const bear = meta.bear_score ?? undefined;
    // 辩论节点的 stance 映射到 verdict
    let verdict = meta.verdict ?? undefined;
    if (!verdict && meta.stance) {
      const s = String(meta.stance).toLowerCase();
      if (s.includes("bull") || s.includes("看多")) { verdict = "看多"; }
      else if (s.includes("bear") || s.includes("看空")) { verdict = "看空"; }
      else if (s.includes("neutral") || s.includes("中性")) { verdict = "中性"; }
      else { verdict = meta.stance; }
    }
    return {
      ...meta,
      verdict,
      bull_score: bull,
      bear_score: bear,
      confidence: meta.confidence ?? undefined,
      summary: summary.length > 0 ? summary : undefined,
      analysis: summary.length > 0 ? summary : undefined,
    } as ParsedReport;
  } catch {
    // JSON 解析失败也尝试提取正文
    const summary = trimmed.slice(0, verdictIdx).trim();
    if (summary.length > 20) {
      return { summary, analysis: summary } as ParsedReport;
    }
    return null;
  }
}

function tryParse(report: string): ParsedReport | null {
  try {
    const trimmed = report.trim();
    // 1) 直接是 JSON
    if (trimmed.startsWith("{") || trimmed.startsWith("[")) {
      try {
        return JSON.parse(trimmed);
      } catch { /* try below */ }
    }
    // 2) ```json ``` 代码块
    const m = trimmed.match(/```(?:json)?\s*([\s\S]*?)\s*```/);
    if (m) {
      try {
        return JSON.parse(m[1].trim());
      } catch { /* try below */ }
    }
    // 3) 复用 tryBeautifyJson 的容错提取（处理前面有解释文字、trailing comma 等）
    const beautified = tryBeautifyJson(report);
    if (beautified !== report) {
      // tryBeautifyJson 返回了不同的字符串，说明它做了提取/修复
      try {
        return JSON.parse(beautified);
      } catch { /* fallthrough */ }
    }
    // 4) 手动找第一个 { 到最后一个 }
    const fb = trimmed.indexOf("{");
    const lb = trimmed.lastIndexOf("}");
    if (fb !== -1 && lb !== -1 && lb > fb) {
      try {
        return JSON.parse(trimmed.slice(fb, lb + 1));
      } catch { /* ignore */ }
    }
  } catch { /* not JSON */ }
  return null;
}

/** 从任意 ParsedReport 中提取可读的摘要文本 */
function extractSummary(parsed: ParsedReport): string {
  // candidate 顺序：先 prose 字段（reasoning / summary / analysis ...），再 report 兜底。
  // P2 修复(2026-07-24): trader 节点的 `report` 字段是内层 JSON 字符串而非 markdown，
  // 排在第一位会导致整张卡片显示 JSON 原文。把 report 放到末尾并跳过 JSON-like 字符串。
  const candidates = [
    parsed.summary,
    parsed.argument,
    parsed.analysis,
    parsed.assessment,
    parsed.buffett_verdict,
    parsed.verdict,
    parsed.reasoning,
    parsed.business_model,
    parsed.moat_reasoning,
    parsed.financial_health,
    parsed.margin_of_safety,
    parsed.catalyst_detail,
    parsed.report,
  ];
  for (const c of candidates) {
    if (typeof c === "string" && c.length > 10) {
      // 跳过看起来像 JSON 的字符串（trader / 包装对象的 inner JSON）
      const trimmed = c.trimStart();
      if (trimmed.startsWith("{") || trimmed.startsWith("[")) { continue; }
      return c;
    }
  }
  return "";
}

/** 从 ParsedReport 中提取标签列表 */
function extractTags(parsed: ParsedReport, t: (key: string) => string): string[] {
  if (Array.isArray(parsed.signals) && parsed.signals.length > 0) {
    return parsed.signals;
  }
  const tags: string[] = [];
  if (parsed.stance) { tags.push(parsed.stance); }
  if (parsed.action) { tags.push(parsed.action); }
  if (parsed.main_flow_state) {
    tags.push(`${t("stockAnalysis.analystReport.moneyFlow")}:${parsed.main_flow_state}`);
  }
  if (parsed.dragon_tiger_signal) { tags.push(parsed.dragon_tiger_signal); }
  if (parsed.moat_rating) {
    tags.push(
      `${t("stockAnalysis.analystReport.moat")}:${parsed.moat_rating}`,
    );
  }
  if (parsed.catalyst_level) { tags.push(parsed.catalyst_level); }
  if (parsed.narrative_completeness) {
    tags.push(`${t("stockAnalysis.analystReport.narrative")}:${parsed.narrative_completeness}`);
  }
  if (parsed.institutional_trace) {
    tags.push(`${t("stockAnalysis.analystReport.capital")}:${parsed.institutional_trace}`);
  }
  if (parsed.concept_risk) {
    tags.push(`${t("stockAnalysis.analystReport.conceptRisk")}:${parsed.concept_risk}`);
  }
  if (typeof parsed.bull_score === "number" && parsed.bull_score > 0) {
    const normalized = parsed.bull_score <= 10 ? parsed.bull_score * 10 : parsed.bull_score;
    tags.push(`${t("stockAnalysis.sentimentBullish")}:${Math.round(normalized)}`);
  }
  if (typeof parsed.bear_score === "number" && parsed.bear_score > 0) {
    const normalized = parsed.bear_score <= 10 ? parsed.bear_score * 10 : parsed.bear_score;
    tags.push(`${t("stockAnalysis.sentimentBearish")}:${Math.round(normalized)}`);
  }
  return tags;
}

/** 从 ParsedReport 中提取要点列表 */
function extractKeyPoints(parsed: ParsedReport): string[] {
  if (Array.isArray(parsed.key_points) && parsed.key_points.length > 0) {
    return parsed.key_points;
  }
  if (Array.isArray(parsed.core_arguments) && parsed.core_arguments.length > 0) {
    return parsed.core_arguments;
  }
  if (Array.isArray(parsed.resonance_points) && parsed.resonance_points.length > 0) {
    return parsed.resonance_points;
  }
  if (Array.isArray(parsed.evidence) && parsed.evidence.length > 0) {
    return parsed.evidence
      .filter((e) => e && typeof e.point === "string")
      .map((e) => `${e.point}${e.data ? ` (${e.data})` : ""}`);
  }
  if (Array.isArray(parsed.data_gaps) && parsed.data_gaps.length > 0) {
    return parsed.data_gaps.slice(0, 3);
  }
  if (Array.isArray(parsed.narrative_missing) && parsed.narrative_missing.length > 0) {
    return parsed.narrative_missing;
  }
  if (Array.isArray(parsed.key_events) && parsed.key_events.length > 0) {
    return parsed.key_events
      .filter((e) => e && typeof e.event === "string")
      .map((e) => `${e.event}${e.source ? ` (${e.source})` : ""}`);
  }
  return [];
}

/** 从 ParsedReport 中提取风险标志 */
function extractRiskFlags(parsed: ParsedReport): string[] {
  if (Array.isArray(parsed.risk_flags) && parsed.risk_flags.length > 0) {
    return parsed.risk_flags;
  }
  return [];
}

/** 判断是否为"空分析"（全是 data_gaps 或空字段） */
function isEmptyAnalysis(parsed: ParsedReport): boolean {
  const summary = extractSummary(parsed);
  if (summary.length > 20) { return false; }
  const points = extractKeyPoints(parsed);
  if (points.length > 0) { return false; }
  // 如果只有 data_gaps 且没有实质内容，认为是空的
  const hasContent = Object.keys(parsed).some((k) => {
    const v = parsed[k];
    if (k === "data_gaps") { return false; }
    if (Array.isArray(v)) { return v.length > 0; }
    if (typeof v === "string") { return v.length > 5 && !v.includes("无法"); }
    if (typeof v === "number") { return v !== 0; }
    return v != null;
  });
  return !hasContent;
}

export function AnalystReportCard({ expertId, report }: Props) {
  const { t } = useTranslation();
  const themeMode = useSettingsStore((s) => s.settings.themeMode);
  const isDark = themeMode === "dark"
    || (themeMode === "system" && window.matchMedia("(prefers-color-scheme: dark)").matches);
  let name = t(`stockAnalysis.workflow.analyst.${expertId}`, expertId);
  // 安全兜底：如果 i18n key miss（比如 expertId 含 a- 前缀），剥离后重试
  if (name === expertId && expertId.startsWith("a-")) {
    const fallback = t(`stockAnalysis.workflow.analyst.${expertId.slice(2)}`, expertId.slice(2));
    if (fallback !== expertId.slice(2)) {
      name = fallback;
    }
  }
  const cleanedReport = cleanToolCallTags(report);
  // 优先解析 <!-- VERDICT: {...} --> 格式（分析师自由文本 + 末尾 verdict 标签）
  let parsed = tryParseVerdictFormat(cleanedReport);
  let displayContent: string = "";
  if (parsed) {
    displayContent = cleanedReport;
  } else {
    const beautified = tryBeautifyJson(cleanedReport);
    parsed = tryParse(beautified);
    displayContent = beautified || report || "";
  }
  const [expanded, setExpanded] = useState(false);
  const [dqOpen, setDqOpen] = useState(false);
  const hasContent = !!displayContent || !!parsed;

  const extraBtns = hasContent
    ? (
      <>
        <Button
          type="text"
          size="small"
          icon={<SafetyCertificateOutlined />}
          onClick={() => setDqOpen(true)}
          title={t("stockAnalysis.analystReport.dataQuality")}
        />
        <Button
          type="text"
          size="small"
          icon={<ExpandOutlined />}
          onClick={() => setExpanded(true)}
          title={t("stockAnalysis.expandView")}
        />
      </>
    )
    : null;

  // 有解析结果：尝试结构化渲染
  if (parsed) {
    // strict_mode 下 LLM 输出被重构成 {"report":"...","verdict":{"verdict":"看多","bull_score":7,...}}
    // verdict/bull_score/bear_score/confidence 在嵌套的 verdict 对象里，需要提取到顶层
    if (parsed.verdict && typeof parsed.verdict === "object" && !Array.isArray(parsed.verdict)) {
      const v = parsed.verdict as Record<string, unknown>;
      if (typeof v.verdict === "string") { parsed.verdict = v.verdict; }
      else if (typeof v.stance === "string") { parsed.verdict = v.stance; }
      else {
        // 尝试其他常见字段名
        for (const key of ["direction", "recommendation", "decision"] as const) {
          if (typeof v[key] === "string") {
            parsed.verdict = v[key];
            break;
          }
        }
      }
      if (parsed.bull_score == null && typeof v.bull_score === "number") { parsed.bull_score = v.bull_score as number; }
      if (parsed.bear_score == null && typeof v.bear_score === "number") { parsed.bear_score = v.bear_score as number; }
      if (parsed.confidence == null && typeof v.confidence === "number") { parsed.confidence = v.confidence as number; }
      // report 文本不在 summary 里，在 report 字段
      if (!parsed.summary && typeof parsed.report === "string") { parsed.summary = parsed.report; }
      // 提取其他可能的嵌套字段
      if (parsed.summary == null && typeof v.report === "string") { parsed.summary = v.report; }
      if (parsed.analysis == null && typeof v.analysis === "string") { parsed.analysis = v.analysis; }
      if (parsed.summary == null && typeof v.summary === "string") { parsed.summary = v.summary; }
    }
    const summary = extractSummary(parsed);
    const tags = extractTags(parsed, t);
    const points = extractKeyPoints(parsed);
    const riskFlags = extractRiskFlags(parsed);
    const empty = isEmptyAnalysis(parsed);
    const confidence = typeof parsed.confidence === "number"
      ? (parsed.confidence > 1 ? parsed.confidence : parsed.confidence * 100)
      : (typeof parsed.positionPct === "number" ? parsed.positionPct : null);

    // 计算看多/看空/中性显眼展示
    const bullScore = typeof parsed.bull_score === "number"
      ? (parsed.bull_score > 1 ? parsed.bull_score : parsed.bull_score * 100)
      : null;
    const bearScore = typeof parsed.bear_score === "number"
      ? (parsed.bear_score > 1 ? parsed.bear_score : parsed.bear_score * 100)
      : null;
    const verdictColor = parsed.verdict
      ? (String(parsed.verdict).includes("看多") || String(parsed.verdict).includes("bull")
        ? "red" // A股红=涨
        : String(parsed.verdict).includes("看空") || String(parsed.verdict).includes("bear")
        ? "green" // A股绿=跌
        : "default")
      : "default";
    return (
      <>
        <Card
          size="small"
          className="h-full flex flex-col"
          title={
            <span className="flex items-center gap-2 flex-wrap">
              {name}
              {parsed.type && <Tag style={{ marginLeft: 8 }}>{parsed.type}</Tag>}
              {empty && <Tag color="orange">{t("quant.common.empty")}</Tag>}
            </span>
          }
          extra={extraBtns}
          styles={{ body: { flex: 1, maxHeight: 400, overflow: "auto" } }}
        >
          {/* 看多/看空/置信度 显眼展示 */}
          {(bullScore != null || bearScore != null || (typeof parsed.verdict === "string")) && (
            <div className="flex items-center gap-3 mb-2 flex-wrap">
              {typeof parsed.verdict === "string" && (
                <Tag color={verdictColor} style={{ fontSize: 13, padding: "2px 10px", fontWeight: 600 }}>
                  {t(
                    `stockAnalysis.analystVerdict.${
                      parsed.verdict.includes("看多") || parsed.verdict.toLowerCase().includes("bull")
                        ? "bullish"
                        : parsed.verdict.includes("看空") || parsed.verdict.toLowerCase().includes("bear")
                        ? "bearish"
                        : "neutral"
                    }`,
                  )}
                </Tag>
              )}
              {bullScore != null && (
                <span className="text-xs" style={{ color: "#f5222d", fontWeight: 600 }}>
                  {t("stockAnalysis.sentimentBullish")} {Math.round(bullScore)}
                </span>
              )}
              {bearScore != null && (
                <span className="text-xs" style={{ color: "#52c41a", fontWeight: 600 }}>
                  {t("stockAnalysis.sentimentBearish")} {Math.round(bearScore)}
                </span>
              )}
              {confidence != null && (
                <span className="text-xs" style={{ color: "var(--muted)" }}>
                  {t("stockAnalysis.confidence")} {confidence.toFixed(0)}%
                </span>
              )}
            </div>
          )}

          {summary && (
            <div className="sa-markdown-content" style={{ marginTop: 4 }}>
              <ReportMarkdown content={summary} isDark={isDark} />
            </div>
          )}
          {points.length > 0 && (
            <ul className="text-xs list-disc pl-4 mb-1 mt-2" style={{ color: "var(--muted)" }}>
              {points.map((p, i) => <li key={i}>{p}</li>)}
            </ul>
          )}
          {tags.length > 0 && (
            <div className="flex gap-1 flex-wrap mt-2">
              {tags.map((s, i) => (
                <Tag key={i} color={getSignalColor(s)}>
                  {s}
                </Tag>
              ))}
            </div>
          )}
          {riskFlags.length > 0 && (
            <div className="flex gap-1 flex-wrap mt-2">
              {riskFlags.map((r, i) => <Tag key={i} color="orange">{r}</Tag>)}
            </div>
          )}
          {!parsed.verdict && bullScore == null && bearScore == null && !summary && points.length === 0 && (
            <div className="text-xs" style={{ color: "var(--muted)" }}>
              {t("stockAnalysis.analystReport.completedNoStructure")}
            </div>
          )}
        </Card>
        <Modal
          title={name}
          open={expanded}
          onCancel={() => setExpanded(false)}
          footer={null}
          width="80vw"
          style={{ top: 20 }}
          styles={{ body: { maxHeight: "80vh", overflow: "auto" } }}
        >
          <div className="sa-markdown-content">
            {summary && <ReportMarkdown content={summary} isDark={isDark} />}
            {points.length > 0 && (
              <ul className="list-disc pl-6 my-2">
                {points.map((p, i) => <li key={i}>{p}</li>)}
              </ul>
            )}
            <Collapse
              defaultActiveKey={[]}
              items={[{
                key: "raw",
                label: t("stockAnalysis.analystReport.viewRawData"),
                children: (
                  <pre
                    style={{
                      background: isDark ? "#1f1f1f" : "#f5f5f5",
                      padding: 12,
                      borderRadius: 6,
                      fontSize: 12,
                      overflow: "auto",
                      maxHeight: "50vh",
                      margin: 0,
                    }}
                  >
                    <code>{displayContent}</code>
                  </pre>
                ),
              }]}
              style={{ marginTop: 12 }}
            />
          </div>
        </Modal>
        <AnalystDataQualityModal
          name={name}
          expertId={expertId}
          parsed={(() => {
            // 创建规范化的 parsed 对象，将分数归一化到 0-100 范围
            const normalized: Record<string, unknown> = { ...parsed };
            if (typeof normalized.bull_score === "number") {
              normalized.bull_score = normalized.bull_score > 1
                ? normalized.bull_score
                : (normalized.bull_score as number) * 100;
            }
            if (typeof normalized.bear_score === "number") {
              normalized.bear_score = normalized.bear_score > 1
                ? normalized.bear_score
                : (normalized.bear_score as number) * 100;
            }
            if (typeof normalized.confidence === "number") {
              normalized.confidence = normalized.confidence > 1
                ? normalized.confidence
                : (normalized.confidence as number) * 100;
            }
            // 确保 summary 字段存在（优先用 summary，其次用 report/analysis）
            if (!normalized.summary && typeof normalized.report === "string") {
              normalized.summary = normalized.report;
            }
            if (!normalized.summary && typeof normalized.analysis === "string") {
              normalized.summary = normalized.analysis;
            }
            return normalized as ParsedReport;
          })()}
          report={displayContent}
          open={dqOpen}
          onClose={() => setDqOpen(false)}
        />
      </>
    );
  }

  // 无解析结果：尝试从原始文本中"模糊"提取关键信息
  const fuzzy = ((): { points: string[]; tags: string[]; summary: string; empty: boolean } => {
    const text = displayContent || "";
    const points: string[] = [];
    const tags: string[] = [];
    let summary = "";

    // 提取 evidence[].point
    const evMatch = text.match(/"evidence"\s*:\s*\[([\s\S]*?)\]/);
    if (evMatch) {
      const items = evMatch[1].match(/"point"\s*:\s*"([^"]*)"/g);
      if (items) {
        items.forEach((item) => {
          const m = item.match(/"point"\s*:\s*"([^"]*)"/);
          if (m && m[1].length > 3) { points.push(m[1]); }
        });
      }
    }

    // 提取 core_arguments[]
    const coreMatch = text.match(/"core_arguments"\s*:\s*\[([\s\S]*?)\]/);
    if (coreMatch) {
      const raw = coreMatch[1];
      // 匹配 "string" 或 { "point": "..." }
      const strItems = raw.match(/"([^"\n]{5,})"/g);
      if (strItems) {
        strItems.forEach((item) => {
          const clean = item.replace(/^"|"$/g, "");
          if (clean.length > 5 && !points.includes(clean)) { points.push(clean); }
        });
      }
      const objItems = raw.match(/"point"\s*:\s*"([^"]*)"/g);
      if (objItems) {
        objItems.forEach((item) => {
          const m = item.match(/"point"\s*:\s*"([^"]*)"/);
          if (m && m[1].length > 3 && !points.includes(m[1])) { points.push(m[1]); }
        });
      }
    }

    // 提取 key_points[]
    const kpMatch = text.match(/"key_points"\s*:\s*\[([\s\S]*?)\]/);
    if (kpMatch) {
      const items = kpMatch[1].match(/"([^"\n]{5,})"/g);
      if (items) {
        items.forEach((item) => {
          const clean = item.replace(/^"|"$/g, "");
          if (clean.length > 5 && !points.includes(clean)) { points.push(clean); }
        });
      }
    }

    // 提取 data_gaps（作为数据不足提示）
    const gapMatch = text.match(/"data_gaps"\s*:\s*\[([\s\S]*?)\]/);
    if (gapMatch && points.length === 0) {
      const items = gapMatch[1].match(/"([^"\n]{5,})"/g);
      if (items && items.length > 0) {
        const first = items[0].replace(/^"|"$/g, "");
        if (first.length > 5) { summary = `${t("stockAnalysis.analystReport.dataLimited")}:${first}`; }
      }
    }

    // 提取 bull_score / bear_score（归一化到百分制）
    const bullMatch = text.match(/"bull_score"\s*:\s*(\d+)/);
    const bearMatch = text.match(/"bear_score"\s*:\s*(\d+)/);
    if (bullMatch) {
      const raw = parseInt(bullMatch[1], 10);
      const normalized = raw <= 10 ? raw * 10 : raw;
      tags.push(`${t("stockAnalysis.sentimentBullish")}:${Math.round(normalized)}`);
    }
    if (bearMatch) {
      const raw = parseInt(bearMatch[1], 10);
      const normalized = raw <= 10 ? raw * 10 : raw;
      tags.push(`${t("stockAnalysis.sentimentBearish")}:${Math.round(normalized)}`);
    }

    // 提取 stance / action
    const stanceMatch = text.match(/"stance"\s*:\s*"([^"]*)"/);
    if (stanceMatch) { tags.push(stanceMatch[1]); }
    const actionMatch = text.match(/"action"\s*:\s*"([^"]*)"/);
    if (actionMatch) { tags.push(actionMatch[1]); }

    // 提取 summary
    const summaryMatch = text.match(/"summary"\s*:\s*"([^"]*)"/);
    if (summaryMatch && summaryMatch[1].length > 10) {
      summary = summaryMatch[1];
    }
    // 提取 argument
    if (!summary) {
      const argMatch = text.match(/"argument"\s*:\s*"([^"]*)"/);
      if (argMatch && argMatch[1].length > 10) { summary = argMatch[1]; }
    }
    // 提取 analysis
    if (!summary) {
      const anaMatch = text.match(/"analysis"\s*:\s*"([^"]*)"/);
      if (anaMatch && anaMatch[1].length > 10) { summary = anaMatch[1]; }
    }

    // 纯文本 fallback：内容不像 JSON 但有实质文字，提取前 200 字作为摘要
    if (!summary && points.length === 0 && tags.length === 0) {
      const cleaned = text
        .replace(/由于上游工具调用返回了.*?的错误[，。]/g, "")
        .replace(/根据系统指令.*?[，。]/g, "")
        .replace(/我的职责是.*?[，。]/g, "")
        .replace(/在上游数据缺失.*?[，。]/g, "")
        .replace(/我无法获取.*?[，。]/g, "")
        .replace(/我必须诚实反映.*?[，。]/g, "")
        .replace(/以下是基于当前可用上下文.*?[，。]/g, "")
        .replace(/请注意，由于缺乏.*?[，。]/g, "")
        .replace(/我需要调用.*?[，。]/g, "")
        .replace(/让我先调用.*?[，。]/g, "")
        .replace(/首先我需要.*?[。]/g, "")
        .replace(/用户要求.*?进行.*?[。]/g, "")
        .replace(/用户需要.*?[。]/g, "")
        .replace(/^\s*[-*]\s+/gm, "") // markdown list
        .replace(/\s+/g, " ")
        .trim();
      if (cleaned.length > 30) {
        summary = cleaned.slice(0, 200) + (cleaned.length > 200 ? "..." : "");
      } else if (!text.trim().startsWith("{") && !text.trim().startsWith("[") && text.length > 30) {
        summary = text.slice(0, 200) + "...";
      }
    }

    const empty = points.length === 0 && !summary && tags.length === 0;
    return { points, tags, summary, empty };
  })();

  return (
    <>
      <Card
        size="small"
        className="h-full flex flex-col"
        title={
          <span>
            {name}
            {fuzzy.empty && <Tag color="orange" style={{ marginLeft: 8 }}>{t("quant.common.empty")}</Tag>}
          </span>
        }
        extra={extraBtns}
        styles={{ body: { flex: 1, maxHeight: 400, overflow: "auto" } }}
      >
        {fuzzy.summary && (
          <div className="sa-markdown-content">
            <ReportMarkdown content={fuzzy.summary} isDark={isDark} />
          </div>
        )}
        {fuzzy.points.length > 0 && (
          <ul className="text-xs list-disc pl-4 mb-1 mt-1" style={{ color: "var(--muted)" }}>
            {fuzzy.points.map((p, i) => <li key={i}>{p}</li>)}
          </ul>
        )}
        {fuzzy.tags.length > 0 && (
          <div className="flex gap-1 flex-wrap mt-1">
            {fuzzy.tags.map((s, i) => (
              <Tag key={i} color={getSignalColor(s)}>
                {s}
              </Tag>
            ))}
          </div>
        )}
        {fuzzy.empty && displayContent && (
          <div className="text-xs" style={{ color: "var(--muted)" }}>
            {t("quant.common.empty")}
          </div>
        )}
        {!displayContent && (
          <Empty
            image={Empty.PRESENTED_IMAGE_SIMPLE}
            description={
              <span style={{ color: "var(--muted)", fontSize: 12 }}>
                {t("stockAnalysis.noReport")}
              </span>
            }
          />
        )}
      </Card>
      <Modal
        title={name}
        open={expanded}
        onCancel={() => setExpanded(false)}
        footer={null}
        width="80vw"
        style={{ top: 20 }}
        styles={{ body: { maxHeight: "80vh", overflow: "auto" } }}
      >
        <div className="sa-markdown-content">
          {fuzzy.summary && <ReportMarkdown content={fuzzy.summary} isDark={isDark} />}
          {fuzzy.points.length > 0 && (
            <ul className="list-disc pl-6 my-2">
              {fuzzy.points.map((p, i) => <li key={i}>{p}</li>)}
            </ul>
          )}
          <Collapse
            defaultActiveKey={[]}
            items={[{
              key: "raw",
              label: t("stockAnalysis.analystReport.viewRawData"),
              children: (
                <pre
                  style={{
                    background: isDark ? "#1f1f1f" : "#f5f5f5",
                    padding: 12,
                    borderRadius: 6,
                    fontSize: 12,
                    overflow: "auto",
                    maxHeight: "50vh",
                    margin: 0,
                  }}
                >
                  <code>{displayContent}</code>
                </pre>
              ),
            }]}
            style={{ marginTop: 12 }}
          />
        </div>
      </Modal>
      <AnalystDataQualityModal
        name={name}
        expertId={expertId}
        parsed={(() => {
          // 从 displayContent 中提取基础字段用于数据质量检测
          const text = displayContent || "";
          const fallback: Record<string, unknown> = {};

          // 尝试 VERDICT tag
          const verdictIdx = text.indexOf("<!-- VERDICT:");
          if (verdictIdx !== -1) {
            try {
              const jsonStr = text.slice(verdictIdx + "<!-- VERDICT:".length);
              const jsonEnd = jsonStr.indexOf("-->");
              if (jsonEnd !== -1) {
                const meta = JSON.parse(jsonStr.slice(0, jsonEnd).trim());
                // 提取关键字段
                if (typeof meta.verdict === "string") { fallback.verdict = meta.verdict; }
                if (typeof meta.stance === "string") { fallback.verdict = meta.stance; }
                if (typeof meta.bull_score === "number") {
                  fallback.bull_score = meta.bull_score > 1 ? meta.bull_score : meta.bull_score * 100;
                }
                if (typeof meta.bear_score === "number") {
                  fallback.bear_score = meta.bear_score > 1 ? meta.bear_score : meta.bear_score * 100;
                }
                if (typeof meta.confidence === "number") {
                  fallback.confidence = meta.confidence > 1 ? meta.confidence : meta.confidence * 100;
                }
              }
            } catch { /* ignore */ }
          }

          // 从文本中正则提取 bull_score/bear_score
          if (fallback.bull_score === undefined) {
            const bullMatch = text.match(/"bull_score"\s*:\s*(\d+(?:\.\d+)?)/);
            if (bullMatch) {
              const raw = parseFloat(bullMatch[1]);
              fallback.bull_score = raw > 1 ? raw : raw * 100;
            }
          }
          if (fallback.bear_score === undefined) {
            const bearMatch = text.match(/"bear_score"\s*:\s*(\d+(?:\.\d+)?)/);
            if (bearMatch) {
              const raw = parseFloat(bearMatch[1]);
              fallback.bear_score = raw > 1 ? raw : raw * 100;
            }
          }

          // 从 fuzzy 结果中获取 summary
          if (fuzzy.summary) { fallback.summary = fuzzy.summary; }
          if (fuzzy.points.length > 0) { fallback.key_points = fuzzy.points; }

          // 如果提取到了任何字段，返回 fallback
          if (Object.keys(fallback).length > 0) {
            return fallback as unknown as ParsedReport;
          }
          return null;
        })()}
        report={displayContent}
        open={dqOpen}
        onClose={() => setDqOpen(false)}
      />
    </>
  );
}

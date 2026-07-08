/**
 * CompactValueAssessment — ValueAssessmentPanel 在 chat 中的紧凑版本
 * 输入:已从 LLM 输出提取出的 report 字符串(可能是 JSON 或纯文本)
 * 输出:1-2 行关键信息,verdict + 理想买入价
 */
import { useMemo } from "react";

interface CompactValueAssessmentProps {
  data: { report: string } | string;
}

interface ParsedValue {
  buffett_verdict?: string;
  ideal_buy_price?: string;
  intrinsic_value_range?: string;
  margin_of_safety?: string;
  moat_rating?: string;
}

function tryParse(report: string): ParsedValue | null {
  if (!report) { return null; }
  try {
    // 先尝试从 markdown ```json 块提取
    const m = report.match(/```(?:json)?\s*([\s\S]+?)```/);
    const candidate = m ? m[1].trim() : report.trim();
    const parsed = JSON.parse(candidate);
    if (parsed && typeof parsed === "object") { return parsed as ParsedValue; }
  } catch { /* fall through */ }
  return null;
}

export function CompactValueAssessment({ data }: CompactValueAssessmentProps) {
  const report = typeof data === "string" ? data : data?.report ?? "";
  const parsed = useMemo(() => tryParse(report), [report]);

  if (!parsed) {
    return (
      <div className="text-[12px] italic" style={{ color: "var(--muted)" }}>
        {report.slice(0, 120) || "—"}
      </div>
    );
  }

  const verdict = parsed.buffett_verdict?.slice(0, 80)
    ?? parsed.intrinsic_value_range?.slice(0, 80)
    ?? "";
  const ideal = parsed.ideal_buy_price;
  const moat = parsed.moat_rating;

  return (
    <div className="space-y-1 text-[12px]">
      <div className="flex items-baseline gap-2 flex-wrap">
        {moat && (
          <span
            className="px-1.5 py-0.5 rounded text-[10px] font-medium"
            style={{ background: "var(--accent-bg, #ede9fe)", color: "var(--accent, #7c3aed)" }}
          >
            护城河: {moat}
          </span>
        )}
        {ideal && <span style={{ color: "var(--sa-green, #16a34a)" }}>理想买入 ¥{ideal}</span>}
      </div>
      {verdict && (
        <div className="text-[11px] leading-snug" style={{ color: "var(--color-text-secondary)" }}>
          {verdict}
          {parsed.buffett_verdict && parsed.buffett_verdict.length > 80 && "…"}
        </div>
      )}
      {parsed.margin_of_safety && (
        <div className="text-[10px]" style={{ color: "var(--muted)" }}>
          {parsed.margin_of_safety}
        </div>
      )}
    </div>
  );
}

// i18n-exempt: 业务逻辑/API 描述/日志字符串，非 UI 展示文本
/**
 * CompactRiskSummary — RiskMatrix 在 chat 中的紧凑版本
 * 输入:风险评估文本(键为节点 ID,值为报告)
 * 输出:风险分条形对比 + 平均/最高分
 * V55: 检测 store.untrustedNodes，在顶部显示红色"数据异常"警告横幅
 *      避免 LLM 失败的 50/50 兜底被当成有效信号。
 */
import { useStockAnalysisStore } from "@/stores/feature/stockAnalysisStore";
import { useMemo } from "react";
import { useTranslation } from "react-i18next";

function computeRiskScore(text: string): number {
  // 统一提取报告文本，忽略 VERDICT 标签和 JSON 语法噪音
  const clean = text
    .replace(/<!--\s*VERDICT\s*:\s*\{[^}]*\}\s*-->/gi, "")
    .replace(/```json[\s\S]*?```/g, "")
    .replace(/[{}[\]"\\,:\s]+/g, " ")
    .replace(/\s+/g, " ")
    .trim();
  const high = [
    "高风险",
    "重大风险",
    "严重",
    "危机",
    "暴跌",
    "崩盘",
    "预警",
    "危险",
    "不确定",
    "大幅下",
    "极度",
  ];
  const mid = ["风险", "谨慎", "关注", "波动", "压力", "挑战", "不确定性", "潜在", "下行", "回落"];
  let score = 40;
  for (const w of high) { score += (clean.match(new RegExp(w, "g")) || []).length * 8; }
  for (const w of mid) { score += (clean.match(new RegExp(w, "g")) || []).length * 3; }
  if (clean.length > 500) { score += 5; }
  if (clean.length > 1000) { score += 5; }
  if (clean.length > 2000) { score += 5; }
  return Math.min(100, Math.max(5, score));
}

interface CompactRiskSummaryProps {
  data: Record<string, string> | unknown;
}

function normalizeMap(data: CompactRiskSummaryProps["data"]): Record<string, string> {
  if (data && typeof data === "object" && !Array.isArray(data)) {
    const out: Record<string, string> = {};
    for (const [k, v] of Object.entries(data as Record<string, unknown>)) {
      if (typeof v === "string") { out[k] = v; }
    }
    return out;
  }
  return {};
}

const LABEL_KEY_MAP: Record<string, string> = {
  "risk-agg": "stockAnalysis.workflow.riskAggregation",
  "risk-con": "stockAnalysis.workflow.riskConservative",
  "risk-neu": "stockAnalysis.workflow.riskNeutral",
  "research-mgr": "stockAnalysis.risk.researchManager",
  "comprehensive": "stockAnalysis.risk.comprehensive",
  "risk-aggregated": "stockAnalysis.workflow.riskAggregation",
  "risk-level": "stockAnalysis.workflow.riskLevel",
  "risk-convergence": "stockAnalysis.workflow.riskConvergence",
};

function useGetLabel() {
  const { t } = useTranslation();
  return (key: string): string => {
    const i18nKey = LABEL_KEY_MAP[key];
    return i18nKey ? t(i18nKey) : key;
  };
}

export function CompactRiskSummary({ data }: CompactRiskSummaryProps) {
  const { t } = useTranslation();
  const getLabel = useGetLabel();
  // V55: 读取 strict_mode 兜底节点标记，用于在顶部显示红色警告横幅
  const untrustedNodes = useStockAnalysisStore((s) => s.untrustedNodes);
  const assessments = useMemo(() => normalizeMap(data), [data]);
  // 筛选出风险评估相关的不可信节点（risk-*/research-mgr）
  const untrustedRiskNodes = useMemo(() => {
    return Object.keys(untrustedNodes).filter((k) => k.startsWith("risk-") || k === "research-mgr");
  }, [untrustedNodes]);
  const entries = useMemo(() => {
    return Object.entries(assessments).map(([type, text]) => ({
      type,
      label: getLabel(type),
      score: computeRiskScore(text),
    })).sort((a, b) => b.score - a.score);
  }, [assessments, getLabel]);

  if (entries.length === 0) {
    return (
      <div className="text-[12px] italic" style={{ color: "var(--muted)" }}>
        {t("stockAnalysis.noRiskData")}
      </div>
    );
  }

  const avg = Math.round(entries.reduce((a, e) => a + e.score, 0) / entries.length);
  const max = entries[0];

  return (
    <div className="space-y-1 text-[12px]">
      {/* V55: strict_mode 兜底警告横幅 */}
      {untrustedRiskNodes.length > 0 && (
        <div
          className="rounded px-2 py-1 text-[11px] font-medium"
          style={{
            background: "var(--sa-red-bg, #fee2e2)",
            color: "var(--sa-red, #dc2626)",
            border: "1px solid var(--sa-red, #dc2626)",
          }}
        >
          ⚠️ {t("stockAnalysis.dataAlert", { nodes: untrustedRiskNodes.join(", ") })}
        </div>
      )}
      <div className="flex items-baseline gap-2 flex-wrap">
        <span
          className="px-1.5 py-0.5 rounded text-[10px] font-medium"
          style={{
            background: avg > 70
              ? "var(--sa-red-bg, #fee2e2)"
              : avg > 40
              ? "var(--sa-amber-bg, #fef3c7)"
              : "var(--sa-green-bg, #dcfce7)",
            color: avg > 70
              ? "var(--sa-red, #dc2626)"
              : avg > 40
              ? "var(--sa-amber, #d97706)"
              : "var(--sa-green, #16a34a)",
          }}
        >
          {t("stockAnalysis.avgRiskScore", { score: avg })}
        </span>
        <span style={{ color: "var(--muted)" }}>
          {t("stockAnalysis.highestRisk", { label: max.label, score: max.score })}
        </span>
      </div>
      <div className="space-y-0.5">
        {entries.slice(0, 4).map((e) => (
          <div key={e.type} className="flex items-center gap-1.5">
            <span
              className="text-[10px] shrink-0"
              style={{ color: "var(--muted)", minWidth: 48 }}
            >
              {e.label}
            </span>
            <div
              className="flex-1 rounded h-1.5 overflow-hidden"
              style={{ background: "var(--muted-bg, #e5e7eb)" }}
            >
              <div
                style={{
                  width: `${e.score}%`,
                  height: "100%",
                  background: e.score > 70
                    ? "var(--sa-red, #dc2626)"
                    : e.score > 40
                    ? "var(--sa-amber, #d97706)"
                    : "var(--sa-green, #16a34a)",
                }}
              />
            </div>
            <span
              className="text-[10px] font-mono shrink-0"
              style={{
                color: e.score > 70
                  ? "var(--sa-red, #dc2626)"
                  : e.score > 40
                  ? "var(--sa-amber, #d97706)"
                  : "var(--sa-green, #16a34a)",
                minWidth: 24,
                textAlign: "right",
              }}
            >
              {e.score}
            </span>
          </div>
        ))}
      </div>
    </div>
  );
}

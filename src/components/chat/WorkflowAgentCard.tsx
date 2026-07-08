import { cleanToolCallTags, extractReadableFromRiskReport } from "@/components/stock-analysis/utils";
import { getWorkflowNodeLabel } from "@/utils/workflowNodeLabel";
import { Card, Progress, Tag } from "antd";
import { TrendingUp } from "lucide-react";
import { useMemo } from "react";
import { useTranslation } from "react-i18next";

/* eslint-disable react-refresh/only-export-components */

/** 从任意 Agent JSON 输出中提取一段可读摘要 */
function extractAgentBrief(report: string, maxLen = 180): string {
  const cleaned = cleanToolCallTags(report).trim();
  if (!cleaned) { return ""; }

  // 尝试解析 JSON（支持：纯JSON、```json...```、```...```、以及前后带文字的混合格式）
  let parsed: Record<string, unknown> | null = null;
  try {
    const trimmed = cleaned.trim();
    if (trimmed.startsWith("{")) {
      parsed = JSON.parse(trimmed);
    } else {
      // 匹配 ```json ... ```
      const m = trimmed.match(/```(?:json)?\s*([\s\S]*?)\s*```/);
      if (m) { parsed = JSON.parse(m[1]); }
    }
  } catch {
    // 混合格式：找第一个 { 到最后一个 }
    const firstBrace = cleaned.indexOf("{");
    const lastBrace = cleaned.lastIndexOf("}");
    if (firstBrace !== -1 && lastBrace !== -1 && lastBrace > firstBrace) {
      const candidate = cleaned.slice(firstBrace, lastBrace + 1);
      try {
        parsed = JSON.parse(candidate);
      } catch {
        // 修复常见错误后重试
        try {
          const fixed = candidate.replace(/,\s*}/g, "}").replace(/,\s*\]/g, "]");
          parsed = JSON.parse(fixed);
        } catch { /* fallthrough */ }
      }
    }
  }

  if (parsed) {
    // 按优先级提取字段（分析师 + 辩论 + 估值 + 决策）
    const candidates = [
      // 分析师标准字段
      parsed.summary,
      parsed.argument,
      parsed.analysis,
      parsed.assessment,
      // 催化剂分析师字段
      parsed.catalyst_type,
      // 辩论字段
      parsed.our_claim,
      parsed.their_weakness,
      // 估值字段
      parsed.buffett_verdict,
      parsed.verdict,
      parsed.reasoning,
      parsed.business_model,
      parsed.moat_reasoning,
      parsed.financial_health,
      parsed.margin_of_safety,
      // 资金面/筹码面
      parsed.stance,
      parsed.main_flow_state,
      parsed.dragon_tiger_signal,
      // 决策
      parsed.action,
    ];
    for (const c of candidates) {
      if (typeof c === "string" && c.length > 5) {
        return c.length > maxLen ? c.slice(0, maxLen) + "..." : c;
      }
    }
    // 提取 evidence / key_points / core_arguments / resonance_points 的前几项
    for (
      const key of [
        "evidence",
        "key_points",
        "core_arguments",
        "resonance_points",
        "preempted_counter_attacks",
        "key_events",
      ]
    ) {
      const arr = parsed[key];
      if (Array.isArray(arr) && arr.length > 0) {
        const first = arr[0];
        const text = typeof first === "string"
          ? first
          : (first && typeof first === "object" && "point" in first)
          ? String((first as Record<string, unknown>).point ?? "")
          : (first && typeof first === "object" && "claim" in first)
          ? String((first as Record<string, unknown>).claim ?? "")
          : (first && typeof first === "object" && "event" in first)
          ? String((first as Record<string, unknown>).event ?? "")
          : "";
        if (text.length > 5) {
          return text.length > maxLen ? text.slice(0, maxLen) + "..." : text;
        }
      }
    }
    // 如果有 bull_score / bear_score，构造标签文本
    const bScore = parsed.bull_strength_score ?? parsed.bull_score;
    const beScore = parsed.bear_strength_score ?? parsed.bear_score;
    if (typeof bScore === "number" || typeof beScore === "number") {
      const parts: string[] = [];
      if (typeof bScore === "number") { parts.push(`看多:${bScore}`); }
      if (typeof beScore === "number") { parts.push(`看空:${beScore}`); }
      return parts.join("，");
    }
    // 如果有 data_gaps 且没有实质内容，提示数据不足
    const gaps = parsed.data_gaps;
    if (Array.isArray(gaps) && gaps.length > 0) {
      const firstGap = String(gaps[0]);
      return firstGap.length > maxLen
        ? `数据不足: ${firstGap.slice(0, maxLen)}...`
        : `数据不足: ${firstGap}`;
    }
  }

  // 回退：直接截断文本（过滤掉常见解释性前缀）
  let plain = cleaned.replace(/\n/g, " ").trim();
  // 移除"由于上游工具调用..."这类前缀
  const noisePrefixes = [
    /由于上游工具调用返回了.*?的错误[，。]/,
    /根据系统指令.*?[，。]/,
    /我的职责是.*?[，。]/,
    /在上游数据缺失.*?[，。]/,
    /我无法获取.*?[，。]/,
    /我必须诚实反映.*?[，。]/,
    /以下是基于当前可用上下文.*?[，。]/,
    /请注意，由于缺乏.*?[，。]/,
  ];
  for (const re of noisePrefixes) {
    plain = plain.replace(re, "");
  }
  plain = plain.replace(/\s+/g, " ").trim();
  return plain.length > maxLen ? plain.slice(0, maxLen) + "..." : plain;
}

export interface WorkflowCardData {
  type: "progress" | "analyst" | "decision" | "aggregate" | "debate" | "risk";
  phase?: string;
  completed?: number;
  total?: number;
  analystName?: string;
  analystReport?: string;
  action?: string;
  positionPct?: number;
  targetPrice?: number;
  stopLoss?: number;
  reasoning?: string;
  riskLevel?: string;
  confidence?: number;
  round?: number;
  bull?: string;
  bear?: string;
  riskKey?: string;
  riskContent?: string;
  analysts?: Array<
    { nodeId: string; name: string; report?: string; status: "pending" | "running" | "done" | "failed" }
  >;
  debates?: Array<{ round: number; bull?: string; bear?: string; status: "pending" | "running" | "done" | "failed" }>;
  risks?: Array<{ key: string; content?: string; status: "pending" | "running" | "done" | "failed" }>;
  dataSources?: Array<{
    nodeId: string;
    toolName: string;
    label: string;
    status: "pending" | "fetching" | "success" | "failed";
    error?: string;
    summary?: string;
  }>;
  decision?: WorkflowCardData;
  status?: "running" | "done" | "error";
  error?: string;
  failedSteps?: Array<{ nodeId: string; error?: string }>;
}

export function parseWorkflowCard(content: string): WorkflowCardData | null {
  const match = content.match(/^<!-- workflow-(progress|analyst|decision|aggregate|debate|risk):(.*?) -->/);
  if (!match) { return null; }
  try {
    return { type: match[1] as WorkflowCardData["type"], ...JSON.parse(match[2]) };
  } catch {
    return null;
  }
}

export function makeWorkflowContent(type: string, data: Record<string, unknown>, fallback: string): string {
  return `<!-- workflow-${type}:${JSON.stringify(data)} -->${fallback}`;
}

export function WorkflowAgentCard({ data }: { data: WorkflowCardData }) {
  const { t } = useTranslation();

  const phaseLabel = useMemo(() => {
    if (!data.phase) { return t("stockAnalysis.workflow.initializing"); }
    return t(`stockAnalysis.workflow.phase.${data.phase}`, data.phase);
  }, [data.phase, t]);

  const analystLabel = useMemo(() => {
    if (!data.analystName) { return t("stockAnalysis.workflow.analystFallback"); }
    return t(`stockAnalysis.workflow.analyst.${data.analystName}`, data.analystName);
  }, [data.analystName, t]);

  const getAnalystDisplayName = (name: string) => {
    return t(`stockAnalysis.workflow.analyst.${name}`, name);
  };

  const getToolLabel = (toolName: string) => {
    return t(`stockAnalysis.toolLabel.${toolName}`, toolName);
  };

  if (data.type === "progress") {
    const pct = data.total && data.total > 0
      ? Math.round(((data.completed ?? 0) / data.total) * 100)
      : 0;
    return (
      <div className="workflow-card" style={{ padding: "12px 16px" }}>
        <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 8 }}>
          <TrendingUp size={16} style={{ color: "var(--accent)" }} />
          <span style={{ fontSize: 13, fontWeight: 600 }}>
            {t("stockAnalysis.workflow.title")}
          </span>
          <Tag color="processing" style={{ fontSize: 11 }}>{t("stockAnalysis.workflow.inProgress")}</Tag>
        </div>
        <Progress percent={pct} size="small" status="active" />
        <div style={{ fontSize: 12, color: "var(--muted)", marginTop: 4 }}>
          {t("stockAnalysis.workflow.currentPhase")}: {phaseLabel} ({data.completed ?? 0}/{data.total ?? "?"})
        </div>
      </div>
    );
  }

  if (data.type === "analyst") {
    const brief = data.analystReport
      ? extractAgentBrief(data.analystReport, 200)
      : t("stockAnalysis.workflow.analystComplete");
    return (
      <div className="workflow-card" style={{ padding: "10px 14px" }}>
        <div style={{ fontSize: 13, fontWeight: 600, marginBottom: 4 }}>📊 {analystLabel}</div>
        <div style={{ fontSize: 12, color: "var(--muted)", lineHeight: 1.6 }}>{brief}</div>
      </div>
    );
  }

  if (data.type === "debate") {
    const round = data.round ?? 1;
    const bullBrief = data.bull ? extractAgentBrief(data.bull, 200) : t("stockAnalysis.workflow.pending");
    const bearBrief = data.bear ? extractAgentBrief(data.bear, 200) : t("stockAnalysis.workflow.pending");
    return (
      <div className="workflow-card" style={{ padding: "10px 14px" }}>
        <div style={{ fontSize: 13, fontWeight: 600, marginBottom: 8 }}>
          🎯 {t("stockAnalysis.workflow.debateRound")} {round}
        </div>
        <div style={{ display: "flex", gap: 8, marginBottom: 8 }}>
          <div
            style={{
              flex: 1,
              background: "var(--sa-red-glass)",
              padding: 8,
              borderRadius: 6,
              border: "1px solid var(--sa-red-soft)",
            }}
          >
            <div style={{ fontSize: 12, fontWeight: 600, marginBottom: 4, color: "var(--sa-red)" }}>
              🐂 {t("stockAnalysis.workflow.bullCase")}
            </div>
            <div style={{ fontSize: 11, color: "var(--muted)", lineHeight: 1.5 }}>{bullBrief}</div>
          </div>
          <div
            style={{
              flex: 1,
              background: "var(--sa-green-glass)",
              padding: 8,
              borderRadius: 6,
              border: "1px solid var(--sa-green-soft)",
            }}
          >
            <div style={{ fontSize: 12, fontWeight: 600, marginBottom: 4, color: "var(--sa-green)" }}>
              🐻 {t("stockAnalysis.workflow.bearCase")}
            </div>
            <div style={{ fontSize: 11, color: "var(--muted)", lineHeight: 1.5 }}>{bearBrief}</div>
          </div>
        </div>
      </div>
    );
  }

  if (data.type === "risk") {
    const riskKey = data.riskKey ?? "risk";
    const RISK_LABEL_MAP: Record<string, string> = {
      "risk-agg": "stockAnalysis.workflow.riskAggregation",
      "risk-con": "stockAnalysis.workflow.riskConservative",
      "risk-neu": "stockAnalysis.workflow.riskNeutral",
      "research-mgr": "stockAnalysis.risk.researchManager",
      "risk-level": "stockAnalysis.workflow.riskLevel",
      "risk-aggregated": "stockAnalysis.workflow.riskAggregation",
      "risk-convergence": "stockAnalysis.workflow.riskConvergence",
    };
    const i18nKey = RISK_LABEL_MAP[riskKey];
    const riskName = i18nKey
      ? t(i18nKey)
      : (riskKey.startsWith("risk-") ? riskKey.slice(5) : riskKey);
    const riskBrief = data.riskContent
      ? extractReadableFromRiskReport(data.riskContent)
      : "";
    return (
      <div className="workflow-card" style={{ padding: "10px 14px" }}>
        <div style={{ fontSize: 13, fontWeight: 600, marginBottom: 4 }}>
          ⚠️ {t("stockAnalysis.workflow.riskAssessment")}: {riskName}
        </div>
        {riskBrief && <div style={{ fontSize: 12, color: "var(--muted)", lineHeight: 1.6 }}>{riskBrief}</div>}
      </div>
    );
  }

  if (data.type === "decision") {
    const isBull = data.action === t("stockAnalysis.actionBuy") || data.action === t("stockAnalysis.actionIncrease");
    const isBear = data.action === t("stockAnalysis.actionSell") || data.action === t("stockAnalysis.actionReduce");
    return (
      <Card
        size="small"
        style={{ borderColor: isBull ? "var(--sa-red)" : isBear ? "var(--sa-green)" : undefined }}
        title={
          <span>
            {isBull ? "🟢" : isBear ? "🔴" : "🟡"} {t("stockAnalysis.workflow.decisionTitle")}：{data.action}
          </span>
        }
      >
        <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "4px 16px", fontSize: 12 }}>
          <span>
            {t("stockAnalysis.workflow.positionPct")}: <b>{data.positionPct ?? "N/A"}%</b>
          </span>
          <span>
            {t("stockAnalysis.workflow.targetPrice")}: <b>¥{data.targetPrice ?? "N/A"}</b>
          </span>
          <span>
            {t("stockAnalysis.workflow.stopLoss")}: <b>¥{data.stopLoss ?? "N/A"}</b>
          </span>
          <span>
            {t("stockAnalysis.workflow.confidence")}: <b>{data.confidence ?? "N/A"}%</b>
          </span>
          <span style={{ gridColumn: "1 / -1" }}>
            {t("stockAnalysis.workflow.riskLevel")}:{" "}
            <Tag
              color={data.riskLevel === t("stockAnalysis.risk.high")
                ? "red"
                : data.riskLevel === t("stockAnalysis.risk.medium")
                ? "orange"
                : "green"}
            >
              {data.riskLevel ?? "N/A"}
            </Tag>
          </span>
        </div>
        {data.reasoning && (
          <div style={{ fontSize: 12, color: "var(--muted)", marginTop: 6, lineHeight: 1.5 }}>
            {data.reasoning}
          </div>
        )}
      </Card>
    );
  }

  if (data.type === "aggregate") {
    const analysts = data.analysts || [];
    const dataSources = data.dataSources || [];
    const pct = data.total && data.total > 0
      ? Math.round(((data.completed ?? 0) / data.total) * 100)
      : 0;

    const successCount = dataSources.filter((d) => d.status === "success").length;
    const failedCount = dataSources.filter((d) => d.status === "failed").length;

    return (
      <div className="workflow-card" style={{ padding: "12px 16px" }}>
        <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 12 }}>
          <TrendingUp size={16} style={{ color: "var(--accent)" }} />
          <span style={{ fontSize: 13, fontWeight: 600 }}>
            {t("stockAnalysis.workflow.title")}
          </span>
          {data.status === "running" && (
            <Tag color="processing" style={{ fontSize: 11 }}>{t("stockAnalysis.workflow.inProgress")}</Tag>
          )}
          {data.status === "done" && (
            <Tag color="success" style={{ fontSize: 11 }}>{t("stockAnalysis.workflow.phase.done")}</Tag>
          )}
          {data.status === "error" && (
            <Tag color="error" style={{ fontSize: 11 }}>{t("stockAnalysis.workflow.phase.error")}</Tag>
          )}
        </div>

        <>
          <Progress
            percent={pct}
            size="small"
            status={data.status === "done" ? "success" : "active"}
            style={{ marginBottom: 12 }}
          />
          <div style={{ fontSize: 12, color: "var(--muted)", marginBottom: 12 }}>
            {t("stockAnalysis.workflow.currentPhase")}: {phaseLabel} ({data.completed ?? 0}/{data.total ?? "?"})
          </div>

          {dataSources.length > 0 && (
            <div style={{ marginBottom: 16 }}>
              <div
                style={{
                  fontSize: 12,
                  fontWeight: 600,
                  marginBottom: 8,
                  color: "var(--text)",
                  display: "flex",
                  alignItems: "center",
                  gap: 6,
                }}
              >
                🔗 {t("stockAnalysis.dataSource")}
                {successCount > 0 && (
                  <Tag color="success" style={{ fontSize: 10, margin: 0 }}>
                    {successCount} {t("stockAnalysis.success")}
                  </Tag>
                )}
                {failedCount > 0 && (
                  <Tag color="error" style={{ fontSize: 10, margin: 0 }}>
                    {failedCount} {t("stockAnalysis.failure")}
                  </Tag>
                )}
              </div>
              <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
                {dataSources.map((ds) => (
                  <div
                    key={ds.nodeId}
                    style={{
                      padding: "6px 10px",
                      background: ds.status === "failed"
                        ? "var(--error-glass, rgba(255,77,79,0.06))"
                        : ds.status === "success"
                        ? "var(--accent-glass)"
                        : "var(--bg-glass)",
                      borderRadius: 6,
                      border: ds.status === "failed"
                        ? "1px solid var(--error-soft, rgba(255,77,79,0.2))"
                        : ds.status === "success"
                        ? "1px solid var(--accent-soft)"
                        : "1px solid var(--border)",
                    }}
                  >
                    <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                      <span style={{ fontSize: 12, fontWeight: 500 }}>
                        {ds.status === "success"
                          ? "✅"
                          : ds.status === "failed"
                          ? "❌"
                          : ds.status === "fetching"
                          ? "🔄"
                          : "⏳"} {ds.label || getToolLabel(ds.toolName)}
                      </span>
                      <Tag
                        color={ds.status === "success"
                          ? "success"
                          : ds.status === "failed"
                          ? "error"
                          : ds.status === "fetching"
                          ? "processing"
                          : "default"}
                        style={{ fontSize: 10 }}
                      >
                        {ds.status === "success"
                          ? t("stockAnalysis.dataFetched")
                          : ds.status === "failed"
                          ? t("stockAnalysis.failure")
                          : ds.status === "fetching"
                          ? t("stockAnalysis.dataFetching")
                          : t("stockAnalysis.dataWaiting")}
                      </Tag>
                    </div>
                    {ds.status === "success" && ds.summary && (
                      <div style={{ fontSize: 11, color: "var(--muted)", marginTop: 2, lineHeight: 1.4 }}>
                        {ds.summary}
                      </div>
                    )}
                    {ds.status === "failed" && ds.error && (
                      <div style={{ fontSize: 11, color: "var(--error, #ff4d4f)", marginTop: 2, lineHeight: 1.4 }}>
                        {ds.error}
                      </div>
                    )}
                  </div>
                ))}
              </div>
            </div>
          )}

          {analysts.length > 0 && (
            <div style={{ marginBottom: 16 }}>
              <div style={{ fontSize: 12, fontWeight: 600, marginBottom: 8, color: "var(--text)" }}>
                📊 {t("stockAnalysis.workflow.analystReports")}
              </div>
              <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
                {analysts.map((a) => (
                  <div
                    key={a.nodeId}
                    style={{
                      padding: "8px 10px",
                      background: a.status === "done" ? "var(--accent-glass)" : "var(--bg-glass)",
                      borderRadius: 6,
                      border: a.status === "done" ? "1px solid var(--accent-soft)" : "1px solid var(--border)",
                    }}
                  >
                    <div
                      style={{
                        display: "flex",
                        justifyContent: "space-between",
                        alignItems: "center",
                        marginBottom: 4,
                      }}
                    >
                      <span style={{ fontSize: 12, fontWeight: 600 }}>
                        {a.status === "done" ? "✅" : a.status === "running" ? "⚙️" : "⏳"}{" "}
                        {getAnalystDisplayName(a.name)}
                      </span>
                      <Tag
                        color={a.status === "done" ? "success" : a.status === "running" ? "processing" : "default"}
                        style={{ fontSize: 10 }}
                      >
                        {a.status === "done"
                          ? t("stockAnalysis.workflow.completed")
                          : a.status === "running"
                          ? t("stockAnalysis.workflow.running")
                          : t("stockAnalysis.workflow.pending")}
                      </Tag>
                    </div>
                    {a.status === "done" && a.report && (
                      <div style={{ fontSize: 11, color: "var(--muted)", lineHeight: 1.6, whiteSpace: "pre-wrap" }}>
                        {extractAgentBrief(a.report, 500)}
                      </div>
                    )}
                  </div>
                ))}
              </div>
            </div>
          )}

          {data.debates && data.debates.length > 0 && (
            <div style={{ marginBottom: 16 }}>
              <div style={{ fontSize: 12, fontWeight: 600, marginBottom: 8, color: "var(--text)" }}>
                🎯 {t("stockAnalysis.workflow.bullBearDebate")}
              </div>
              <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
                {data.debates.map((d) => (
                  <div
                    key={`debate-${d.round}`}
                    style={{
                      padding: "8px 10px",
                      background: d.status === "done" ? "var(--accent-glass)" : "var(--bg-glass)",
                      borderRadius: 6,
                      border: d.status === "done" ? "1px solid var(--accent-soft)" : "1px solid var(--border)",
                    }}
                  >
                    <div style={{ fontSize: 12, fontWeight: 600, marginBottom: 6 }}>
                      {d.status === "done" ? "✅" : d.status === "running" ? "⚙️" : "⏳"}{" "}
                      {t("stockAnalysis.workflow.debateRound")} {d.round}
                    </div>
                    {d.status === "done" && d.bull && d.bear && (
                      <div style={{ display: "flex", gap: 8 }}>
                        <div
                          style={{
                            flex: 1,
                            background: "var(--sa-red-glass)",
                            padding: 8,
                            borderRadius: 6,
                            border: "1px solid var(--sa-red-soft)",
                          }}
                        >
                          <div style={{ fontSize: 11, fontWeight: 600, marginBottom: 4, color: "var(--sa-red)" }}>
                            🐂 {t("stockAnalysis.workflow.bullCase")}
                          </div>
                          <div style={{ fontSize: 10, color: "var(--muted)", lineHeight: 1.5 }}>
                            {extractAgentBrief(d.bull ?? "", 200)}
                          </div>
                        </div>
                        <div
                          style={{
                            flex: 1,
                            background: "var(--sa-green-glass)",
                            padding: 8,
                            borderRadius: 6,
                            border: "1px solid var(--sa-green-soft)",
                          }}
                        >
                          <div style={{ fontSize: 11, fontWeight: 600, marginBottom: 4, color: "var(--sa-green)" }}>
                            🐻 {t("stockAnalysis.workflow.bearCase")}
                          </div>
                          <div style={{ fontSize: 10, color: "var(--muted)", lineHeight: 1.5 }}>
                            {extractAgentBrief(d.bear ?? "", 200)}
                          </div>
                        </div>
                      </div>
                    )}
                  </div>
                ))}
              </div>
            </div>
          )}

          {data.risks && data.risks.length > 0 && (
            <div style={{ marginBottom: 16 }}>
              <div style={{ fontSize: 12, fontWeight: 600, marginBottom: 8, color: "var(--text)" }}>
                ⚠️ {t("stockAnalysis.workflow.riskAssessment")}
              </div>
              <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
                {data.risks.map((r) => (
                  <div
                    key={`risk-${r.key}`}
                    style={{
                      padding: "8px 10px",
                      background: r.status === "done" ? "var(--accent-glass)" : "var(--bg-glass)",
                      borderRadius: 6,
                      border: r.status === "done" ? "1px solid var(--accent-soft)" : "1px solid var(--border)",
                    }}
                  >
                    <div
                      style={{
                        display: "flex",
                        justifyContent: "space-between",
                        alignItems: "center",
                        marginBottom: 4,
                      }}
                    >
                      <span style={{ fontSize: 12, fontWeight: 600 }}>
                        {r.status === "done" ? "✅" : r.status === "running" ? "⚙️" : "⏳"}{" "}
                        {r.key.startsWith("risk-") ? r.key.slice(5) : r.key}
                      </span>
                      <Tag
                        color={r.status === "done" ? "success" : r.status === "running" ? "processing" : "default"}
                        style={{ fontSize: 10 }}
                      >
                        {r.status === "done"
                          ? t("stockAnalysis.workflow.completed")
                          : r.status === "running"
                          ? t("stockAnalysis.workflow.running")
                          : t("stockAnalysis.workflow.pending")}
                      </Tag>
                    </div>
                    {r.status === "done" && r.content && (
                      <div style={{ fontSize: 11, color: "var(--muted)", lineHeight: 1.6, whiteSpace: "pre-wrap" }}>
                        {extractAgentBrief(r.content, 400)}
                      </div>
                    )}
                  </div>
                ))}
              </div>
            </div>
          )}

          {data.decision && <WorkflowAgentCard data={data.decision} />}
        </>

        {(data.status === "error" || data.failedSteps) && (
          <div
            style={{
              marginTop: 12,
              padding: 12,
              background: "var(--error-glass, rgba(255,77,79,0.06))",
              borderRadius: 6,
              border: "1px solid var(--error-soft, rgba(255,77,79,0.2))",
            }}
          >
            <div style={{ fontSize: 12, fontWeight: 600, color: "var(--error, #ff4d4f)", marginBottom: 8 }}>
              ❌ {data.error || t("stockAnalysis.workflow.startFailed")}
            </div>
            {data.failedSteps && data.failedSteps.length > 0 && (
              <div style={{ fontSize: 11 }}>
                <div style={{ fontWeight: 600, marginBottom: 4 }}>
                  {t("stockAnalysis.workflow.failedStepsWithCount", { count: data.failedSteps.length })}
                </div>
                {data.failedSteps.map((fs) => (
                  <details key={fs.nodeId} style={{ marginBottom: 4 }}>
                    <summary
                      style={{
                        cursor: "pointer",
                        color: "var(--error, #ff4d4f)",
                        padding: "2px 0",
                      }}
                    >
                      ❌ {getWorkflowNodeLabel(fs.nodeId, t)}
                    </summary>
                    {fs.error && (
                      <pre
                        style={{
                          margin: "4px 0 0 0",
                          padding: 8,
                          background: "var(--bg-glass)",
                          borderRadius: 4,
                          fontSize: 10,
                          whiteSpace: "pre-wrap",
                          lineHeight: 1.5,
                          color: "var(--muted)",
                          maxHeight: 160,
                          overflow: "auto",
                        }}
                      >
                        {fs.error}
                      </pre>
                    )}
                  </details>
                ))}
              </div>
            )}
          </div>
        )}
      </div>
    );
  }

  return null;
}

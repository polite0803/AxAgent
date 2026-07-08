import { Tooltip } from "@/components/layout/Tooltip";
import { useRightPanel } from "@/hooks/useRightPanel";
import { useStockAnalysisStore } from "@/stores/feature/stockAnalysisStore";
import type { EvidenceRef, TimelineNode } from "@/types/stock-analysis";
import { ChevronDown, ChevronRight, Send } from "lucide-react";
import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router-dom";

interface TimelineNodeCardProps {
  node: TimelineNode;
}

/** 紫底 + agent 名首字母徽章(按 plan 8.3 要求) */
function NodeBadge({ name }: { name: string }) {
  const initial = name.trim().charAt(0).toUpperCase() || "?";
  return (
    <div
      className="flex items-center justify-center rounded-full text-sm font-bold text-white shrink-0"
      style={{
        width: 22,
        height: 22,
        background: "var(--accent, #7c3aed)",
        flexShrink: 0,
      }}
      title={name}
    >
      {initial}
    </div>
  );
}

/** 证据 chip:点击 → useRightPanel.navigateTo */
function EvidenceChip({ evidence: ev }: { evidence: EvidenceRef }) {
  const { t } = useTranslation();
  const { navigateTo } = useRightPanel();
  return (
    <button
      type="button"
      className="inline-flex items-center gap-1 text-sm px-1.5 py-0.5 rounded cursor-pointer border"
      style={{
        background: "var(--surface)",
        borderColor: "var(--border)",
        color: "var(--color-text)",
      }}
      onClick={() => navigateTo(ev.tabKey, ev.panelKey, ev.anchor)}
      title={`${ev.tabKey} / ${ev.panelKey}`}
    >
      🔗 {t(`stockAnalysis.evidence.${ev.panelKey}`)}
    </button>
  );
}

/**
 * 把 summary 中的违规片段包裹为 <mark>，用于 LLM 未来引用高亮。
 * 替代引入 mark.js（避免 5KB 依赖 + DOM Mutation 风险）。
 */
function highlightViolations(
  text: string,
  snippets: string[],
  markClass: string,
): React.ReactNode {
  if (!text || snippets.length === 0) { return text; }
  // 按片段长度倒序，避免短片段先匹配覆盖长片段
  const sorted = [...new Set(snippets)]
    .filter((s) => s && s.length > 0)
    .sort((a, b) => b.length - a.length);
  if (sorted.length === 0) { return text; }
  const escaped = sorted.map((s) => s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&"));
  const re = new RegExp(`(${escaped.join("|")})`, "g");
  const parts = text.split(re);
  return parts.map((part, i) => {
    if (sorted.includes(part)) {
      return (
        <mark
          key={i}
          className={markClass}
          style={{
            background: "rgba(239, 68, 68, 0.18)",
            color: "var(--sa-red, #ef4444)",
            padding: "0 2px",
            borderRadius: 3,
            fontWeight: 600,
          }}
        >
          {part}
        </mark>
      );
    }
    return <span key={i}>{part}</span>;
  });
}

/** 节点卡片:紧凑态 1 行 / 展开态 3-5 行 */
export function TimelineNodeCard({ node }: TimelineNodeCardProps) {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const [expanded, setExpanded] = useState(false);

  const allViolations = useStockAnalysisStore((s) => s.violations);
  const nodeViolations = useMemo(
    () => allViolations.filter((v) => v.nodeId === node.id),
    [allViolations, node.id],
  );
  const violationCount = nodeViolations.length;
  const markClass = t("timeTravel.violations.markClass", { defaultValue: "ax-violation-mark" });
  const summaryNodes = useMemo(
    () =>
      highlightViolations(
        node.summary,
        nodeViolations.map((v) => v.snippet),
        markClass,
      ),
    [node.summary, nodeViolations, markClass],
  );

  const statusColor = node.status === "done"
    ? "var(--sa-green)"
    : node.status === "failed"
    ? "var(--sa-red)"
    : node.status === "running"
    ? "var(--accent)"
    : "var(--muted)";

  const statusLabel = node.status === "done"
    ? "✓"
    : node.status === "failed"
    ? "✕"
    : node.status === "running"
    ? "⟳"
    : "·";

  return (
    <div
      className="rounded border text-sm relative"
      style={{
        background: "var(--surface)",
        borderColor: node.status === "failed" || violationCount > 0 ? "var(--sa-red)" : "var(--border)",
        borderLeft: `3px solid ${statusColor}`,
      }}
    >
      <button
        type="button"
        className="flex items-center gap-2 w-full text-left p-1.5"
        onClick={() => setExpanded(!expanded)}
        style={{ color: "var(--color-text)" }}
      >
        {expanded
          ? <ChevronDown size={11} style={{ flexShrink: 0 }} />
          : <ChevronRight size={11} style={{ flexShrink: 0 }} />}
        <NodeBadge name={node.agentName} />
        <span className="font-medium truncate flex-1" title={node.title}>
          {node.title}
        </span>
        {violationCount > 0 && (
          <Tooltip
            title={t("timeTravel.violations.tooltip")}
            data-testid="violation-chip-tooltip"
          >
            <span
              data-testid="violation-chip"
              aria-label={t("timeTravel.violations.chipAria", { n: violationCount })}
              style={{
                display: "inline-flex",
                alignItems: "center",
                padding: "1px 6px",
                borderRadius: 8,
                fontSize: 12,
                fontWeight: 700,
                background: "rgba(239, 68, 68, 0.12)",
                color: "var(--sa-red, #ef4444)",
                border: "1px solid rgba(239, 68, 68, 0.35)",
                flexShrink: 0,
              }}
            >
              {t("timeTravel.violations.chip", { n: violationCount })}
            </span>
          </Tooltip>
        )}
        <span style={{ color: statusColor, fontWeight: 600, flexShrink: 0 }}>
          {statusLabel}
        </span>
      </button>

      {expanded && (
        <div className="px-2 pb-2 space-y-1.5">
          {node.summary && (
            <div
              className="text-sm leading-relaxed"
              style={{ color: "var(--color-text-secondary)" }}
            >
              {summaryNodes}
            </div>
          )}

          {nodeViolations.length > 0 && (
            <ul
              data-testid="violation-snippets"
              style={{
                margin: 0,
                paddingLeft: 16,
                fontSize: 12,
                color: "var(--sa-red, #ef4444)",
              }}
            >
              {nodeViolations.map((v, i) => (
                <li key={i}>
                  <code
                    style={{
                      background: "rgba(239,68,68,0.08)",
                      padding: "0 4px",
                      borderRadius: 3,
                    }}
                  >
                    {v.snippet}
                  </code>{" "}
                  <span style={{ opacity: 0.7 }}>({v.ruleHit})</span>
                </li>
              ))}
            </ul>
          )}

          {node.confidence > 0 && (
            <div className="flex items-center gap-1.5 text-sm">
              <span style={{ color: "var(--muted)" }}>{t("stockAnalysis.confidence")}</span>
              <div
                className="flex-1 rounded-full overflow-hidden"
                style={{ height: 4, background: "var(--border)" }}
              >
                <div
                  style={{
                    width: `${Math.round(node.confidence * 100)}%`,
                    height: "100%",
                    background: statusColor,
                  }}
                />
              </div>
              <span className="font-mono">{Math.round(node.confidence * 100)}%</span>
            </div>
          )}

          {node.evidenceRefs.length > 0 && (
            <div className="flex flex-wrap gap-1">
              {node.evidenceRefs.map((ev, i) => <EvidenceChip key={i} evidence={ev} />)}
            </div>
          )}

          <div className="flex gap-1 pt-0.5">
            <button
              type="button"
              className="inline-flex items-center gap-1 text-sm px-1.5 py-0.5 rounded border"
              style={{
                background: "transparent",
                borderColor: "var(--border)",
                color: "var(--color-text-secondary)",
              }}
              title={t("stockAnalysis.timeline.sendToChat")}
              onClick={() => {
                // 简化：跳转到对话页并附 query
                navigate(`/?refTimeline=${encodeURIComponent(node.id)}`);
              }}
            >
              <Send size={12} />
              {t("stockAnalysis.timeline.sendToChat")}
            </button>
          </div>
        </div>
      )}
    </div>
  );
}

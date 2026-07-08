/**
 * ExperimentTrail — 实验轨迹底部组件
 *
 * 展示实验历史线：Original → Experiment #N → Execute
 * 每次 Accept 后追加一个新的实验节点。
 */

import { useStockAnalysisStore } from "@/stores/feature/stockAnalysisStore";
import { useTranslation } from "react-i18next";

export function ExperimentTrail() {
  const { t } = useTranslation();
  const experiments = useStockAnalysisStore((s) => s.experiments);
  const decision = useStockAnalysisStore((s) => s.decision);

  if (experiments.length === 0) { return null; }

  const steps = [
    {
      label: t("stockAnalysis.experimentTrail.originalAnalysis"),
      sub: decision
        ? `${decision.action} / ${decision.confidence}% / ${decision.positionPct}%`
        : "—",
      active: experiments.length === 0,
      color: "var(--color-background-secondary)",
    },
    ...experiments.map((e, i) => ({
      label: t("stockAnalysis.experimentTrail.experiment", { n: e.step }),
      sub: `${e.decisionAfter.action ?? "—"} / ${e.decisionAfter.confidence ?? "—"}% / ${
        e.decisionAfter.positionPct ?? "—"
      }%`,
      active: i === experiments.length - 1,
      color: "var(--color-background-info)",
      detail: Object.entries(e.params)
        .filter(([, v]) => typeof v === "number")
        .map(([k, v]) => `${k}=${v}`)
        .join(", "),
    })),
    {
      label: t("stockAnalysis.experimentTrail.execute"),
      sub: t("stockAnalysis.experimentTrail.acceptOrSkip"),
      active: false,
      color: "var(--color-border-tertiary)",
      dashed: true,
    },
  ];

  return (
    <div style={{ marginTop: 16, borderTop: "0.5px solid var(--color-border-tertiary)", paddingTop: 12 }}>
      <div style={{ fontSize: 12, fontWeight: 500, marginBottom: 10 }}>{t("stockAnalysis.experimentTrail.trail")}</div>
      <div style={{ display: "flex", gap: 0, fontSize: 11 }}>
        {steps.map((step, i) => (
          <div key={i} style={{ display: "flex", alignItems: "center", gap: 0, flex: 1 }}>
            {/* Circle */}
            <div
              style={{
                width: 24,
                height: 24,
                borderRadius: "50%",
                background: step.active ? step.color : "var(--color-border-tertiary)",
                color: step.active ? "white" : "var(--color-text-tertiary)",
                display: "flex",
                alignItems: "center",
                justifyContent: "center",
                fontSize: 10,
                fontWeight: 500,
                flexShrink: 0,
              }}
            >
              {i + 1}
            </div>
            {/* Connector */}
            {i < steps.length - 1 && (
              <div
                style={{
                  flex: 1,
                  height: 1.5,
                  borderTop: ("dashed" in step && step.dashed)
                    ? "1.5px dashed var(--color-border-tertiary)"
                    : `1.5px solid ${step.color}`,
                  margin: "0 4px",
                }}
              />
            )}
            {/* Label */}
            <div style={{ marginLeft: 4, flex: 1 }}>
              <div
                style={{
                  fontWeight: 500,
                  fontSize: 11,
                  color: step.active ? "var(--color-text-info)" : "var(--color-text-primary)",
                }}
              >
                {step.label}
              </div>
              <div style={{ fontSize: 10, color: "var(--color-text-secondary)" }}>
                {step.sub}
              </div>
              {"detail" in step && step.detail && (
                <div style={{ fontSize: 9, color: "var(--color-text-tertiary)", marginTop: 1 }}>
                  {step.detail}
                </div>
              )}
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}

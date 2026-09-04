// SPDX-License-Identifier: AGPL-3.0-only

import { invoke } from "@/lib/invoke";
import { useAppConfigStore } from "@/stores/feature/appConfigStore";
import type { AwarenessSummary } from "@/types";
import { Activity, BrainCircuit, Gauge, Zap } from "lucide-react";
import React, { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

/** Horizontal metric bar for a [0,1] awareness quantity. */
const MetricBar: React.FC<{ label: string; value: number; color: string }> = ({
  label,
  value,
  color,
}) => (
  <div className="flex items-center gap-2">
    <span className="text-[10px] text-muted-foreground w-20 shrink-0">{label}</span>
    <div className="flex-1 h-1.5 rounded-full bg-zinc-200 dark:bg-zinc-800 overflow-hidden">
      <div
        className={`h-full rounded-full ${color}`}
        style={{ width: `${Math.round(Math.min(1, Math.max(0, value)) * 100)}%` }}
      />
    </div>
    <span className="text-[10px] font-mono text-muted-foreground w-8 text-right">
      {value.toFixed(2)}
    </span>
  </div>
);

/** Format a SignalSource enum value for display ("causal_insight" → "CausalInsight"). */
const formatSource = (s: string): string =>
  s
    .split("_")
    .map((p) => p.charAt(0).toUpperCase() + p.slice(1))
    .join("");

/** AwarenessPanel — renders the backend awareness state (A2/A3):
 *  three data-chain-closed state quantities, confidence calibration
 *  summary, and the saliency arbiter's last broadcast. Read-only,
 *  zero side effects. */
export const AwarenessPanel: React.FC = () => {
  const { t } = useTranslation();
  const proactiveEnabled = useAppConfigStore((s) => s.features.proactiveMode);

  const [expanded, setExpanded] = useState(false);
  const [summary, setSummary] = useState<AwarenessSummary | null>(null);
  const [loadFailed, setLoadFailed] = useState(false);

  useEffect(() => {
    if (!expanded || !proactiveEnabled) {
      return;
    }
    const load = async () => {
      try {
        const data = await invoke<AwarenessSummary>(
          "proactive_awareness_summary",
        );
        setSummary(data);
        setLoadFailed(false);
      } catch {
        setLoadFailed(true);
      }
    };
    void load();
    const interval = setInterval(() => void load(), 60_000);
    return () => clearInterval(interval);
  }, [expanded, proactiveEnabled]);

  if (!proactiveEnabled) {
    return null;
  }

  const latest = summary?.frames.length ? summary.frames[summary.frames.length - 1] : null;
  const calibration = summary?.calibration ?? null;
  const broadcast = summary?.lastBroadcast ?? null;

  if (!expanded) {
    return (
      <div className="border-b border-border/50 px-3 py-2">
        <button
          onClick={() => setExpanded(true)}
          className="flex items-center gap-1.5 text-xs text-muted-foreground hover:text-foreground transition-colors"
        >
          <BrainCircuit
            size={14}
            className={latest ? "text-violet-500" : ""}
          />
          {t("awareness.title")}
        </button>
      </div>
    );
  }

  return (
    <div className="border-b border-border/50">
      <div className="flex items-center justify-between px-3 py-2">
        <span className="text-xs font-medium text-foreground/80">
          {t("awareness.title")}
        </span>
        <button
          onClick={() => setExpanded(false)}
          className="text-muted-foreground hover:text-foreground transition-colors"
        >
          <svg
            className="size-3.5"
            fill="none"
            viewBox="0 0 24 24"
            stroke="currentColor"
            strokeWidth={2}
          >
            <path strokeLinecap="round" strokeLinejoin="round" d="M6 18L18 6M6 6l12 12" />
          </svg>
        </button>
      </div>

      <div className="px-3 pb-3">
        {!summary && (
          <div className="text-xs text-muted-foreground/60 pb-1">
            {t("awareness.noData")}
          </div>
        )}

        {latest && (
          <div className="space-y-1.5">
            <MetricBar
              label={t("awareness.arousal")}
              value={latest.arousal}
              color="bg-orange-500"
            />
            <MetricBar
              label={t("awareness.cognitiveLoad")}
              value={latest.cognitiveLoad}
              color="bg-blue-500"
            />
            <MetricBar
              label={t("awareness.selfEfficacy")}
              value={latest.selfEfficacy}
              color="bg-green-500"
            />
            {latest.dominantSource && (
              <div className="flex items-center gap-1 pt-1 text-xs text-violet-600 dark:text-violet-400">
                <Zap size={12} />
                <span>
                  {t("awareness.dominantFocus")}: {formatSource(latest.dominantSource)}
                </span>
              </div>
            )}
          </div>
        )}

        {calibration && (
          <div className="mt-3 rounded-lg border border-border/40 p-2">
            <div className="flex items-center gap-1 text-xs font-medium text-foreground/70 mb-1">
              <Gauge size={12} />
              {t("awareness.calibration")}
            </div>
            <div className="text-[10px] text-muted-foreground font-mono">
              {t("awareness.bias")}: {calibration.avgBias >= 0 ? "+" : ""}
              {calibration.avgBias.toFixed(2)}
            </div>
            <div className="text-[10px] text-muted-foreground mt-0.5">
              {t("awareness.overconfident")} {(calibration.overconfidentRate * 100).toFixed(0)}% ·{" "}
              {t("awareness.calibrated")} {(calibration.calibratedRate * 100).toFixed(0)}% ·{" "}
              {t("awareness.underconfident")} {(calibration.underconfidentRate * 100).toFixed(0)}%
            </div>
          </div>
        )}

        {broadcast && broadcast.winners.length > 0 && (
          <div className="mt-3 rounded-lg border border-border/40 p-2">
            <div className="flex items-center gap-1 text-xs font-medium text-foreground/70 mb-1">
              <Activity size={12} />
              {t("awareness.lastBroadcast")}
            </div>
            {broadcast.winners.map((w) => (
              <div
                key={w.signal.originId}
                className="flex items-center justify-between text-[10px] text-muted-foreground font-mono"
              >
                <span className="truncate">
                  {formatSource(w.signal.source)} · {w.signal.originId}
                </span>
                <span>{w.effective.toFixed(2)}</span>
              </div>
            ))}
          </div>
        )}

        {loadFailed && (
          <div className="mt-2 flex items-center gap-1 text-[10px] text-red-400">
            <span className="size-1.5 rounded-full bg-red-400" />
            {t("awareness.loadFailed")}
          </div>
        )}
      </div>
    </div>
  );
};

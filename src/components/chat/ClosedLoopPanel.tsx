import { invoke } from "@/lib/invoke";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

interface ClosedLoopStatus {
  is_running: boolean;
  nudge_count: number;
  insight_count: number;
  trajectory_stats: {
    total_trajectories?: number;
    total_steps?: number;
    avg_quality?: number;
    success_rate?: number;
  } | null;
  pattern_stats: {
    total_patterns?: number;
    high_value_patterns?: number;
  } | null;
}

export default function ClosedLoopPanel() {
  const { t } = useTranslation();
  const [expanded, setExpanded] = useState(false);
  const [status, setStatus] = useState<ClosedLoopStatus | null>(null);

  useEffect(() => {
    if (!expanded) { return; }
    const fetch = async () => {
      try {
        const s = await invoke<ClosedLoopStatus>("closed_loop_status");
        setStatus(s);
      } catch (e) {
        console.warn("[closedLoop] Failed to fetch status:", e);
      }
    };
    fetch();
    const interval = setInterval(fetch, 30000);
    return () => clearInterval(interval);
  }, [expanded]);

  if (!expanded) {
    return (
      <div className="border-b border-border/50 px-3 py-2">
        <button
          onClick={() => setExpanded(true)}
          className="flex items-center gap-1.5 text-xs text-muted-foreground hover:text-foreground transition-colors"
        >
          <svg className="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"
            />
          </svg>
          {t("chat.closedLoop")}
        </button>
      </div>
    );
  }

  const trajCount = status?.trajectory_stats?.total_trajectories ?? 0;
  const successRate = status?.trajectory_stats?.success_rate ?? 0;
  const patternCount = status?.pattern_stats?.total_patterns ?? 0;

  return (
    <div className="border-b border-border/50 px-3 py-2 space-y-2">
      <div className="flex items-center justify-between">
        <span className="text-xs font-medium text-foreground/80">{t("chat.closedLoopLearning")}</span>
        <button
          onClick={() => setExpanded(false)}
          className="text-muted-foreground hover:text-foreground transition-colors"
        >
          <svg className="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
            <path strokeLinecap="round" strokeLinejoin="round" d="M6 18L18 6M6 6l12 12" />
          </svg>
        </button>
      </div>

      {/* Status indicators */}
      <div className="grid grid-cols-2 gap-1.5">
        <div className="text-xs p-1.5 rounded bg-muted/30">
          <div className="text-[10px] text-muted-foreground">{t("chat.trajectories")}</div>
          <div className="text-sm font-medium">{trajCount}</div>
        </div>
        <div className="text-xs p-1.5 rounded bg-muted/30">
          <div className="text-[10px] text-muted-foreground">{t("chat.successRate")}</div>
          <div className="text-sm font-medium">{(successRate * 100).toFixed(0)}%</div>
        </div>
        <div className="text-xs p-1.5 rounded bg-muted/30">
          <div className="text-[10px] text-muted-foreground">{t("chat.insights")}</div>
          <div className="text-sm font-medium">{status?.insight_count ?? 0}</div>
        </div>
        <div className="text-xs p-1.5 rounded bg-muted/30">
          <div className="text-[10px] text-muted-foreground">{t("chat.patterns")}</div>
          <div className="text-sm font-medium">{patternCount}</div>
        </div>
      </div>

      {/* Pipeline status */}
      <div className="text-[10px] text-muted-foreground space-y-0.5">
        <div className="flex items-center gap-1">
          <span
            className={`inline-block w-1.5 h-1.5 rounded-full ${
              status?.is_running ? "bg-green-500" : "bg-muted-foreground/30"
            }`}
          />
          ClosedLoop: {status?.is_running ? t("chat.closedLoopRunning") : t("chat.closedLoopStopped")}
        </div>
        <div>{t("chat.nudgesPending")}: {status?.nudge_count ?? 0}</div>
        <div className="text-muted-foreground/60 mt-1">
          {t("chat.pipeline")}
        </div>
      </div>
    </div>
  );
}

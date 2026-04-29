import { invoke } from "@/lib/invoke";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

interface RLConfig {
  config: {
    gamma: number;
    lambda: number;
    reward_scale: number;
    entropy_coefficient: number;
    value_coefficient: number;
    use_td_lambda: boolean;
  };
  weights: {
    task_completion: number;
    tool_efficiency: number;
    reasoning_quality: number;
    error_recovery: number;
    user_feedback: number;
    pattern_match: number;
  };
}

interface TrajectoryStats {
  total: number;
  success_rate: number;
  avg_quality: number;
}

export default function RLPanel() {
  const { t } = useTranslation();
  const [expanded, setExpanded] = useState(false);
  const [config, setConfig] = useState<RLConfig | null>(null);
  const [stats, setStats] = useState<TrajectoryStats | null>(null);

  useEffect(() => {
    if (!expanded) { return; }
    const fetch = async () => {
      try {
        const c = await invoke<RLConfig>("rl_config", {});
        setConfig(c);
      } catch (e) {
        console.warn("[rl] config fetch failed:", e);
      }
      try {
        const s = await invoke<TrajectoryStats>("trajectory_stats", {});
        setStats(s);
      } catch (e) {
        console.warn("[rl] stats fetch failed:", e);
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
              d="M3.75 3v11.25A2.25 2.25 0 006 16.5h2.25M3.75 3h-1.5m1.5 0h16.5m0 0H18m0 0v11.25A2.25 2.25 0 0018 16.5h-2.25m-7.5 0h7.5m-7.5 0v-3.75m7.5 3.75v-3.75"
            />
          </svg>
          {t("chat.rlEngine")}
        </button>
      </div>
    );
  }

  return (
    <div className="border-b border-border/50 px-3 py-2 space-y-2">
      <div className="flex items-center justify-between">
        <span className="text-xs font-medium text-foreground/80">{t("chat.rlEngine")}</span>
        <button
          onClick={() => setExpanded(false)}
          className="text-muted-foreground hover:text-foreground transition-colors"
        >
          <svg className="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
            <path strokeLinecap="round" strokeLinejoin="round" d="M6 18L18 6M6 6l12 12" />
          </svg>
        </button>
      </div>

      {/* Stats grid */}
      {stats && (
        <div className="grid grid-cols-3 gap-1.5">
          <div className="text-center p-1 rounded bg-muted/30">
            <div className="text-[10px] text-muted-foreground">{t("chat.trajectories")}</div>
            <div className="text-xs font-medium">{stats.total}</div>
          </div>
          <div className="text-center p-1 rounded bg-muted/30">
            <div className="text-[10px] text-muted-foreground">{t("chat.success")}</div>
            <div className="text-xs font-medium">{Math.round((stats.success_rate ?? 0) * 100)}%</div>
          </div>
          <div className="text-center p-1 rounded bg-muted/30">
            <div className="text-[10px] text-muted-foreground">{t("chat.quality")}</div>
            <div className="text-xs font-medium">{(stats.avg_quality ?? 0).toFixed(2)}</div>
          </div>
        </div>
      )}

      {/* Config */}
      {config && (
        <div className="space-y-1">
          <div className="text-[10px] font-medium text-muted-foreground/70 uppercase tracking-wider">
            {t("chat.config")}
          </div>
          <div className="grid grid-cols-2 gap-x-3 gap-y-0.5 text-[11px]">
            <span className="text-muted-foreground">{t("chat.gamma")}</span>
            <span className="text-foreground/80 text-right">{config.config.gamma}</span>
            <span className="text-muted-foreground">{t("chat.lambda")}</span>
            <span className="text-foreground/80 text-right">{config.config.lambda}</span>
            <span className="text-muted-foreground">{t("chat.rewardScale")}</span>
            <span className="text-foreground/80 text-right">{config.config.reward_scale}</span>
            <span className="text-muted-foreground">{t("chat.entropyCoeff")}</span>
            <span className="text-foreground/80 text-right">{config.config.entropy_coefficient}</span>
          </div>

          <div className="text-[10px] font-medium text-muted-foreground/70 uppercase tracking-wider mt-1">
            {t("chat.rewardWeights.title")}
          </div>
          <div className="space-y-0.5">
            {Object.entries(config.weights).map(([k, v]) => (
              <div key={k} className="flex items-center gap-1.5 text-[11px]">
                <span className="text-muted-foreground truncate flex-1">{t(`chat.rewardWeights.${k}`)}</span>
                <div className="w-16 h-1.5 bg-muted/40 rounded-full overflow-hidden">
                  <div className="h-full bg-blue-500/60 rounded-full" style={{ width: `${v * 100}%` }} />
                </div>
                <span className="text-foreground/60 w-6 text-right">{v.toFixed(2)}</span>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Export button */}
      <button
        onClick={async () => {
          try {
            const data = await invoke<unknown[]>("rl_export_training_data", { minQuality: 0.3, limit: 20 });
            const blob = new Blob([JSON.stringify(data, null, 2)], { type: "application/json" });
            const url = URL.createObjectURL(blob);
            const a = document.createElement("a");
            a.href = url;
            a.download = "rl_training_data.json";
            a.click();
            URL.revokeObjectURL(url);
          } catch (e) {
            console.warn("[rl] export failed:", e);
          }
        }}
        className="w-full text-[11px] py-1 rounded bg-muted/30 hover:bg-muted/50 text-muted-foreground hover:text-foreground transition-colors"
      >
        {t("chat.exportTrainingData")}
      </button>
    </div>
  );
}

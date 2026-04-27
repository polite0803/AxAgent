import { invoke } from "@/lib/invoke";
import { Sparkles } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

interface EvolutionStats {
  generation: number;
  best_fitness: number;
  avg_fitness: number;
  fitness_history: number[];
  converged: boolean;
}

export default function EvolutionPanel() {
  const { t } = useTranslation();
  const [expanded, setExpanded] = useState(false);
  const [status, setStatus] = useState<{ is_running: boolean; stats: EvolutionStats | null } | null>(null);

  const fetchData = useCallback(async () => {
    try {
      const s = await invoke<{ is_running: boolean; stats: EvolutionStats }>("skill_evolution_status", {});
      setStatus(s);
    } catch (e) { /* ignore */ }
  }, []);

  useEffect(() => {
    if (!expanded) { return; }
    fetchData();
    const interval = setInterval(fetchData, 15000);
    return () => clearInterval(interval);
  }, [expanded, fetchData]);

  if (!expanded) {
    return (
      <div className="border-b border-border/50 px-3 py-2">
        <button
          onClick={() => setExpanded(true)}
          className="flex items-center gap-1.5 text-xs text-muted-foreground hover:text-foreground transition-colors"
        >
          <Sparkles size={14} />
          {t("chat.evolution")}
        </button>
      </div>
    );
  }

  const stats = status?.stats;
  const fitnessHistory = stats?.fitness_history ?? [];

  return (
    <div className="border-b border-border/50 px-3 py-2 space-y-2">
      <div className="flex items-center justify-between">
        <span className="text-xs font-medium text-foreground/80">{t("chat.skillEvolution")}</span>
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
      <div className="grid grid-cols-3 gap-1.5">
        <div className="text-center p-1 rounded bg-muted/30">
          <div className="text-[10px] text-muted-foreground">{t("chat.generation")}</div>
          <div className="text-xs font-medium">{stats?.generation ?? 0}</div>
        </div>
        <div className="text-center p-1 rounded bg-muted/30">
          <div className="text-[10px] text-muted-foreground">{t("chat.bestFitness")}</div>
          <div className="text-xs font-medium">{(stats?.best_fitness ?? 0).toFixed(3)}</div>
        </div>
        <div className="text-center p-1 rounded bg-muted/30">
          <div className="text-[10px] text-muted-foreground">{t("chat.status")}</div>
          <div className={`text-xs font-medium ${stats?.converged ? "text-green-500" : "text-amber-500"}`}>
            {stats?.converged ? t("chat.converged") : status?.is_running ? t("chat.running") : t("chat.idle")}
          </div>
        </div>
      </div>

      {/* Fitness history sparkline */}
      {fitnessHistory.length > 1 && (
        <div>
          <div className="text-[10px] font-medium text-muted-foreground/70 uppercase tracking-wider mb-1">
            {t("chat.fitnessCurve")}
          </div>
          <div className="h-12 flex items-end gap-px">
            {fitnessHistory.slice(-20).map((f, i) => {
              const max = Math.max(...fitnessHistory.slice(-20));
              const min = Math.min(...fitnessHistory.slice(-20));
              const range = max - min || 1;
              const height = ((f - min) / range) * 100;
              return (
                <div
                  key={i}
                  className="flex-1 bg-blue-500/50 rounded-t-sm min-h-0.5"
                  style={{ height: `${Math.max(height, 5)}%` }}
                  title={`Gen ${i}: ${f.toFixed(3)}`}
                />
              );
            })}
          </div>
        </div>
      )}
    </div>
  );
}

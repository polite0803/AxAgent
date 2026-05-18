import { useNudgeStore } from "@/stores";
import { X } from "lucide-react";
import { useEffect } from "react";
import { useTranslation } from "react-i18next";
import { ClosedLoopPanel } from "./ClosedLoopPanel";
import { EvolutionPanel } from "./EvolutionPanel";
import { InsightPanel } from "./InsightPanel";
import { NudgePanel } from "./NudgePanel";
import { PatternPanel } from "./PatternPanel";
import { RLPanel } from "./RLPanel";
import { TracerPanel } from "./TracerPanel";

interface EvolutionSidebarProps {
  onClose?: () => void;
}

export function EvolutionSidebar({ onClose }: EvolutionSidebarProps) {
  const { t } = useTranslation();
  const insightCount = useNudgeStore((s) => s.insights.length);
  const pendingNudges = useNudgeStore((s) => s.pendingNudges);
  const closedLoopNudges = useNudgeStore((s) => s.closedLoopNudges);
  const fetchInsights = useNudgeStore((s) => s.fetchInsights);
  const fetchClosedLoopNudges = useNudgeStore((s) => s.fetchClosedLoopNudges);

  const nudgeCount = pendingNudges.length
    + closedLoopNudges.filter((n) => !n.acknowledged).length;

  useEffect(() => {
    fetchInsights();
    fetchClosedLoopNudges();
    const interval = setInterval(() => {
      fetchInsights();
      fetchClosedLoopNudges();
    }, 60_000);
    return () => clearInterval(interval);
  }, [fetchInsights, fetchClosedLoopNudges]);

  return (
    <div className="flex flex-col h-full overflow-y-auto">
      <div className="flex items-center justify-between px-3 py-2 border-b border-border/50">
        <span className="text-xs font-semibold text-foreground/60 uppercase tracking-wider">
          {t("chat.selfEvolution")}
        </span>
        <div className="flex items-center gap-2">
          {insightCount > 0 && (
            <span className="text-[10px] text-muted-foreground/60">
              {insightCount} {t("chat.insights").toLowerCase()}
            </span>
          )}
          {nudgeCount > 0 && (
            <span className="text-[10px] text-orange-500/80">
              {nudgeCount} {t("nudge.learningSuggestions").toLowerCase()}
            </span>
          )}
          {onClose && (
            <button
              onClick={onClose}
              className="p-0.5 rounded hover:bg-muted/50 text-muted-foreground hover:text-foreground transition-colors"
            >
              <X size={14} />
            </button>
          )}
        </div>
      </div>
      <EvolutionPanel />
      <RLPanel />
      <ClosedLoopPanel />
      <PatternPanel />
      <InsightPanel />
      <NudgePanel />
      <TracerPanel />
    </div>
  );
}

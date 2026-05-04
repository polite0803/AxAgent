import { X } from "lucide-react";
import { useTranslation } from "react-i18next";
import ClosedLoopPanel from "./ClosedLoopPanel";
import EvolutionPanel from "./EvolutionPanel";
import InsightPanel from "./InsightPanel";
import NudgePanel from "./NudgePanel";
import PatternPanel from "./PatternPanel";
import RLPanel from "./RLPanel";
import TracerPanel from "./TracerPanel";

interface EvolutionSidebarProps {
  onClose: () => void;
}

export function EvolutionSidebar({ onClose }: EvolutionSidebarProps) {
  const { t } = useTranslation();

  return (
    <div className="flex flex-col h-full overflow-y-auto">
      <div className="flex items-center justify-between px-3 py-2 border-b border-border/50">
        <span className="text-xs font-semibold text-foreground/60 uppercase tracking-wider">
          {t("chat.selfEvolution")}
        </span>
        <button
          onClick={onClose}
          className="p-0.5 rounded hover:bg-muted/50 text-muted-foreground hover:text-foreground transition-colors"
        >
          <X size={14} />
        </button>
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

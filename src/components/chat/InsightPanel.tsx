import { useNudgeStore } from "@/stores";
import type { InsightCategory } from "@/types/nudge";
import { Lightbulb } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

const CATEGORY_COLORS: Record<InsightCategory, string> = {
  pattern: "bg-blue-500/10 text-blue-500",
  preference: "bg-purple-500/10 text-purple-500",
  improvement: "bg-green-500/10 text-green-500",
  warning: "bg-amber-500/10 text-amber-500",
};

const CATEGORY_LABELS: Record<InsightCategory, string> = {
  pattern: "Pattern",
  preference: "Pref",
  improvement: "Improve",
  warning: "Warn",
};

export default function InsightPanel() {
  const { t } = useTranslation();
  const [expanded, setExpanded] = useState(false);
  const [flushContent, setFlushContent] = useState("");
  const [flushTarget, setFlushTarget] = useState<"memory" | "user">("memory");
  const insights = useNudgeStore((s) => s.insights);
  const fetchInsights = useNudgeStore((s) => s.fetchInsights);
  const memoryFlush = useNudgeStore((s) => s.memoryFlush);

  useEffect(() => {
    if (expanded) {
      fetchInsights();
    }
  }, [expanded, fetchInsights]);

  const handleFlush = useCallback(async () => {
    if (!flushContent.trim()) { return; }
    await memoryFlush(flushContent.trim(), flushTarget);
    setFlushContent("");
    await fetchInsights();
  }, [flushContent, flushTarget, memoryFlush, fetchInsights]);

  if (!expanded) {
    return (
      <div className="border-b border-border/50 px-3 py-2">
        <button
          onClick={() => setExpanded(true)}
          className="flex items-center gap-1.5 text-xs text-muted-foreground hover:text-foreground transition-colors"
        >
          <Lightbulb size={14} />
          {t("chat.insightsMemory")} ({insights.length})
        </button>
      </div>
    );
  }

  return (
    <div className="border-b border-border/50 px-3 py-2 space-y-2">
      <div className="flex items-center justify-between">
        <span className="text-xs font-medium text-foreground/80">{t("chat.learningInsights")}</span>
        <button
          onClick={() => {
            setExpanded(false);
            setFlushContent("");
          }}
          className="text-muted-foreground hover:text-foreground transition-colors"
        >
          <svg className="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
            <path strokeLinecap="round" strokeLinejoin="round" d="M6 18L18 6M6 6l12 12" />
          </svg>
        </button>
      </div>

      {/* Insights list */}
      {insights.length > 0
        ? (
          <div className="max-h-32 overflow-y-auto space-y-1">
            {insights.slice(0, 8).map((insight) => (
              <div key={insight.id} className="text-xs p-1.5 rounded bg-muted/30">
                <div className="flex items-center gap-1 mb-0.5">
                  <span
                    className={`inline-block px-1 rounded text-[10px] font-medium ${CATEGORY_COLORS[insight.category]}`}
                  >
                    {CATEGORY_LABELS[insight.category]}
                  </span>
                  <span className="text-foreground/80 truncate">{insight.title}</span>
                  <span className="text-[10px] text-muted-foreground/60 ml-auto">
                    {Math.round(insight.confidence * 100)}%
                  </span>
                </div>
                <div className="text-[11px] text-foreground/60 line-clamp-1">{insight.description}</div>
              </div>
            ))}
          </div>
        )
        : <div className="text-xs text-muted-foreground/60">{t("chat.noInsights")}</div>}

      {/* Memory flush input */}
      <div className="space-y-1">
        <div className="text-[10px] text-muted-foreground">{t("chat.flushToMemory")}</div>
        <div className="flex items-center gap-1.5">
          <select
            value={flushTarget}
            onChange={(e) => setFlushTarget(e.target.value as "memory" | "user")}
            className="text-[10px] bg-muted/30 rounded px-1 py-0.5 border-none outline-none"
          >
            <option value="memory">System</option>
            <option value="user">User</option>
          </select>
          <input
            type="text"
            value={flushContent}
            onChange={(e) => setFlushContent(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter" && !e.shiftKey) {
                e.preventDefault();
                handleFlush();
              }
            }}
            placeholder="e.g. User prefers dark mode..."
            className="flex-1 bg-transparent text-xs outline-none placeholder:text-muted-foreground/60"
          />
          <button
            onClick={handleFlush}
            disabled={!flushContent.trim()}
            className="text-xs px-1.5 py-0.5 rounded bg-primary/10 text-primary hover:bg-primary/20 disabled:opacity-40 transition-colors"
          >
            Save
          </button>
        </div>
      </div>
    </div>
  );
}

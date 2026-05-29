import { useProactiveStore } from "@/stores/feature/proactiveStore";
import type { InsightCategory, LearningInsight } from "@/types";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

const categoryIcons: Record<InsightCategory, string> = {
  pattern: "🔄",
  preference: "⚙️",
  improvement: "💡",
  warning: "⚠️",
};

const categoryColors: Record<InsightCategory, string> = {
  pattern: "border-blue-500/30 bg-blue-500/10",
  preference: "border-purple-500/30 bg-purple-500/10",
  improvement: "border-green-500/30 bg-green-500/10",
  warning: "border-amber-500/30 bg-amber-500/10",
};

function InsightItem({ insight }: { insight: LearningInsight }) {
  const { t } = useTranslation();
  const [expanded, setExpanded] = useState(false);

  return (
    <div
      role="button"
      tabIndex={0}
      className={`p-3 rounded-lg border cursor-pointer transition-all hover:shadow-sm ${
        categoryColors[insight.category]
      }`}
      onClick={() => setExpanded(!expanded)}
      onKeyDown={(e) => {
        if (e.key === "Enter" || e.key === " ") { setExpanded(!expanded); }
      }}
    >
      <div className="flex items-start gap-2">
        <span className="text-base leading-none mt-0.5">
          {categoryIcons[insight.category]}
        </span>
        <div className="flex-1 min-w-0">
          <div className="flex items-center justify-between gap-2">
            <span className="text-sm font-medium truncate">
              {insight.title}
            </span>
            <span className="text-xs text-muted-foreground whitespace-nowrap">
              {Math.round(insight.confidence * 100)}%
            </span>
          </div>
          {!expanded && (
            <p className="text-xs text-muted-foreground truncate mt-0.5">
              {insight.description}
            </p>
          )}
        </div>
      </div>

      {expanded && (
        <div className="mt-2 ml-6 space-y-2" role="presentation">
          <p className="text-xs text-muted-foreground">
            {insight.description}
          </p>
          {insight.evidence.length > 0 && (
            <div className="space-y-1">
              <span className="text-xs font-medium">
                {t("proactive.insight.evidence")}
              </span>
              <ul className="text-xs text-muted-foreground list-disc list-inside">
                {insight.evidence.map((e, i) => <li key={i}>{e}</li>)}
              </ul>
            </div>
          )}
          {insight.suggestedAction && (
            <div className="text-xs">
              <span className="font-medium">
                {t("proactive.insight.suggestedAction")}
              </span>
              <span className="text-muted-foreground ml-1">
                {insight.suggestedAction}
              </span>
            </div>
          )}
        </div>
      )}
    </div>
  );
}

const ALL_CATEGORIES: InsightCategory[] = [
  "pattern",
  "preference",
  "improvement",
  "warning",
];

export function InsightPanel() {
  const { t } = useTranslation();
  const { insights, fetchInsights } = useProactiveStore();
  const [activeCategory, setActiveCategory] = useState<InsightCategory | "all">(
    "all",
  );

  useEffect(() => {
    fetchInsights();
  }, [fetchInsights]);

  const filtered = activeCategory === "all"
    ? insights
    : insights.filter((i) => i.category === activeCategory);

  return (
    <div className="flex flex-col gap-3 p-4">
      <div className="flex items-center justify-between">
        <h3 className="text-sm font-semibold">
          {t("proactive.insight.title")}
        </h3>
        <button
          onClick={() => fetchInsights()}
          className="text-xs text-muted-foreground hover:text-foreground transition-colors"
        >
          {t("proactive.insight.refresh")}
        </button>
      </div>

      <div className="flex gap-1.5 flex-wrap">
        <button
          onClick={() => setActiveCategory("all")}
          className={`px-2 py-0.5 text-xs rounded-full transition-colors ${
            activeCategory === "all"
              ? "bg-primary text-primary-foreground"
              : "bg-muted hover:bg-muted/80"
          }`}
        >
          {t("proactive.insight.all")}
        </button>
        {ALL_CATEGORIES.map((cat) => (
          <button
            key={cat}
            onClick={() => setActiveCategory(cat)}
            className={`px-2 py-0.5 text-xs rounded-full transition-colors ${
              activeCategory === cat
                ? "bg-primary text-primary-foreground"
                : "bg-muted hover:bg-muted/80"
            }`}
          >
            {categoryIcons[cat]} {t(`proactive.insight.category.${cat}`)}
          </button>
        ))}
      </div>

      {filtered.length === 0
        ? (
          <p className="text-xs text-muted-foreground text-center py-4">
            {t("proactive.insight.empty")}
          </p>
        )
        : (
          <div className="flex flex-col gap-2">
            {filtered.map((insight) => <InsightItem key={insight.id} insight={insight} />)}
          </div>
        )}
    </div>
  );
}

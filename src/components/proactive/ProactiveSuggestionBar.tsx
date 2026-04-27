import { useTranslation } from "react-i18next";
import { useProactiveStore } from "@/stores/feature/proactiveStore";
import SuggestionCard from "./SuggestionCard";

export default function ProactiveSuggestionBar() {
  const { t } = useTranslation();
  const { suggestions, isEnabled, setEnabled, isLoading } = useProactiveStore();

  if (!isEnabled || suggestions.length === 0) {
    return null;
  }

  return (
    <div className="border-b border-border bg-gradient-to-r from-primary/5 via-primary/10 to-primary/5">
      <div className="px-4 py-3">
        <div className="flex items-center justify-between mb-3">
          <div className="flex items-center gap-2">
            <div className="w-2 h-2 rounded-full bg-primary animate-pulse" />
            <span className="text-sm font-medium">{t("proactive.suggestions")}</span>
            <span className="text-xs text-muted-foreground">
              ({suggestions.length})
            </span>
          </div>

          <button
            onClick={() => setEnabled(false)}
            className="p-1 text-muted-foreground hover:text-foreground transition-colors"
            title={t("proactive.dismissAll")}
          >
            <svg
              className="w-4 h-4"
              fill="none"
              viewBox="0 0 24 24"
              stroke="currentColor"
              strokeWidth={2}
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                d="M6 18L18 6M6 6l12 12"
              />
            </svg>
          </button>
        </div>

        <div className="flex gap-3 overflow-x-auto pb-1">
          {isLoading ? (
            <div className="flex items-center gap-2 text-sm text-muted-foreground">
              <div className="w-4 h-4 border-2 border-primary border-t-transparent rounded-full animate-spin" />
              {t("proactive.loading")}
            </div>
          ) : (
            suggestions.slice(0, 5).map((suggestion) => (
              <SuggestionCard
                key={suggestion.id}
                suggestion={suggestion}
                compact
              />
            ))
          )}
        </div>

        {suggestions.length > 5 && (
          <div className="mt-2 text-xs text-muted-foreground text-center">
            {t("proactive.moreSuggestions", { count: suggestions.length - 5 })}
          </div>
        )}
      </div>
    </div>
  );
}

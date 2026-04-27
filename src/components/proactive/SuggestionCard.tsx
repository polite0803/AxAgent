import { useState, type ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { useProactiveStore } from "@/stores/feature/proactiveStore";
import type { ProactiveSuggestion } from "@/types/proactive";

interface SuggestionCardProps {
  suggestion: ProactiveSuggestion;
  compact?: boolean;
}

export default function SuggestionCard({ suggestion, compact = false }: SuggestionCardProps) {
  const { t } = useTranslation();
  const { acceptSuggestion, dismissSuggestion, snoozeSuggestion } = useProactiveStore();
  const [isExpanded, setIsExpanded] = useState(false);
  const [isLoading, setIsLoading] = useState(false);

  const priorityColors = {
    low: "border-muted-foreground/30 bg-muted/30",
    medium: "border-yellow-500/30 bg-yellow-500/10",
    high: "border-orange-500/30 bg-orange-500/10",
    critical: "border-red-500/30 bg-red-500/10",
  };

  const suggestionTypeIcons: Record<string, ReactNode> = {
    Completion: (
      <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
        <path strokeLinecap="round" strokeLinejoin="round" d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z" />
      </svg>
    ),
    Refactor: (
      <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
        <path strokeLinecap="round" strokeLinejoin="round" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
      </svg>
    ),
    Documentation: (
      <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
        <path strokeLinecap="round" strokeLinejoin="round" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
      </svg>
    ),
    Test: (
      <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
        <path strokeLinecap="round" strokeLinejoin="round" d="M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2m-6 9l2 2 4-4" />
      </svg>
    ),
    Optimization: (
      <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
        <path strokeLinecap="round" strokeLinejoin="round" d="M13 10V3L4 14h7v7l9-11h-7z" />
      </svg>
    ),
    Learning: (
      <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
        <path strokeLinecap="round" strokeLinejoin="round" d="M12 6.253v13m0-13C10.832 5.477 9.246 5 7.5 5S4.168 5.477 3 6.253v13C4.168 18.477 5.754 18 7.5 18s3.332.477 4.5 1.253m0-13C13.168 5.477 14.754 5 16.5 5c1.747 0 3.332.477 4.5 1.253v13C19.832 18.477 18.247 18 16.5 18c-1.746 0-3.332.477-4.5 1.253" />
      </svg>
    ),
  };

  const handleAccept = async () => {
    setIsLoading(true);
    await acceptSuggestion(suggestion.id);
    setIsLoading(false);
  };

  const handleDismiss = async () => {
    setIsLoading(true);
    await dismissSuggestion(suggestion.id);
    setIsLoading(false);
  };

  const handleSnooze = async () => {
    setIsLoading(true);
    await snoozeSuggestion(suggestion.id, 15);
    setIsLoading(false);
  };

  if (compact) {
    return (
      <div
        className={`shrink-0 w-64 p-3 rounded-lg border ${priorityColors[suggestion.priority]} transition-all hover:shadow-md cursor-pointer`}
        onClick={() => setIsExpanded(!isExpanded)}
      >
        <div className="flex items-start gap-2">
          <div className="text-primary mt-0.5">
            {suggestionTypeIcons[suggestion.suggestion_type] || suggestionTypeIcons.Completion}
          </div>
          <div className="flex-1 min-w-0">
            <div className="text-sm font-medium truncate">{suggestion.title}</div>
            {!isExpanded && (
              <div className="text-xs text-muted-foreground truncate mt-0.5">
                {suggestion.description}
              </div>
            )}
          </div>
        </div>

        {isExpanded && (
          <div className="mt-3 space-y-2" onClick={(e) => e.stopPropagation()}>
            <div className="text-xs text-muted-foreground">
              {suggestion.description}
            </div>
            <div className="flex gap-2">
              <button
                onClick={handleAccept}
                disabled={isLoading}
                className="flex-1 px-2 py-1 text-xs bg-primary text-primary-foreground rounded hover:bg-primary/90 disabled:opacity-50 transition-colors"
              >
                {t("proactive.accept")}
              </button>
              <button
                onClick={handleDismiss}
                disabled={isLoading}
                className="px-2 py-1 text-xs bg-muted rounded hover:bg-muted/80 disabled:opacity-50 transition-colors"
              >
                {t("proactive.dismiss")}
              </button>
            </div>
          </div>
        )}
      </div>
    );
  }

  return (
    <div className={`p-4 rounded-lg border ${priorityColors[suggestion.priority]}`}>
      <div className="flex items-start gap-3">
        <div className="text-primary mt-0.5">
          {suggestionTypeIcons[suggestion.suggestion_type] || suggestionTypeIcons.Completion}
        </div>
        <div className="flex-1 min-w-0">
          <div className="flex items-center justify-between gap-2">
            <h4 className="text-sm font-medium">{suggestion.title}</h4>
            <span className="text-xs px-1.5 py-0.5 bg-primary/10 text-primary rounded">
              {suggestion.priority}
            </span>
          </div>
          <p className="text-xs text-muted-foreground mt-1">{suggestion.description}</p>
          <div className="flex items-center gap-2 mt-3">
            <button
              onClick={handleAccept}
              disabled={isLoading}
              className="px-3 py-1.5 text-xs bg-primary text-primary-foreground rounded hover:bg-primary/90 disabled:opacity-50 transition-colors"
            >
              {t("proactive.accept")}
            </button>
            <button
              onClick={handleSnooze}
              disabled={isLoading}
              className="px-3 py-1.5 text-xs bg-muted rounded hover:bg-muted/80 disabled:opacity-50 transition-colors"
            >
              {t("proactive.snooze")}
            </button>
            <button
              onClick={handleDismiss}
              disabled={isLoading}
              className="px-3 py-1.5 text-xs text-muted-foreground hover:text-foreground disabled:opacity-50 transition-colors"
            >
              {t("proactive.dismiss")}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}

// SPDX-License-Identifier: AGPL-3.0-only

import { useAppConfigStore } from "@/stores/feature/appConfigStore";
import { useProactiveStore } from "@/stores/feature/proactiveStore";
import type { ProactiveSuggestion } from "@/types";
import { Bell, Check, Clock, GitBranch, X } from "lucide-react";
import React, { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

/** Priority color dot (backend sends "low" | "medium" | "high" | "critical") */
const priorityDot: Record<string, string> = {
  critical: "bg-red-500",
  high: "bg-orange-500",
  medium: "bg-blue-500",
  low: "bg-zinc-400",
};

/** Construct minimal ContextFeatures for the refresh call.
 *  Field names must be camelCase; enums lowercase (serde `rename_all = "lowercase"`). */
function buildContextFeatures(): Record<string, unknown> {
  const now = new Date();
  return {
    recentActions: [],
    timeOfDay: now.getHours(),
    dayOfWeek: now.toLocaleDateString("en-US", { weekday: "long" }),
    userActivityLevel: "medium",
    detectedErrors: [],
    detectedPatterns: [],
  };
}

/** Single suggestion card. CausalInsight suggestions show the
 *  explainable from → to causal edge derived from trajectory observation. */
const SuggestionCard: React.FC<{
  suggestion: ProactiveSuggestion;
  onAccept: (id: string) => void;
  onDismiss: (id: string) => void;
  onSnooze: (id: string) => void;
}> = ({ suggestion, onAccept, onDismiss, onSnooze }) => {
  const { t } = useTranslation();
  const isCausal = suggestion.action?.type === "CausalInsight";
  const fromEntity = typeof suggestion.action?.fromEntity === "string"
    ? suggestion.action.fromEntity
    : "";
  const toEntity = typeof suggestion.action?.toEntity === "string"
    ? suggestion.action.toEntity
    : "";

  return (
    <div
      className={`rounded-lg border-l-4 p-3 mb-2 transition-all ${
        isCausal
          ? "border-teal-400 bg-teal-50 dark:bg-teal-950/30"
          : "border-zinc-300 bg-zinc-50 dark:bg-zinc-900/30"
      } ${suggestion.accepted ? "opacity-60" : ""}`}
    >
      <div className="flex items-start gap-2">
        <div
          className={`size-2 rounded-full mt-1.5 shrink-0 ${priorityDot[suggestion.priority] || priorityDot.low}`}
        />
        <div className="flex-1 min-w-0">
          {isCausal && (
            <div className="flex items-center gap-1 text-xs font-medium text-teal-600 dark:text-teal-400 mb-1">
              <GitBranch size={12} />
              <span>{t("suggestion.causalInsight")}</span>
            </div>
          )}
          {isCausal && fromEntity && toEntity && (
            <div className="text-xs text-zinc-500 dark:text-zinc-400 font-mono mb-1 truncate">
              {fromEntity} → {toEntity}
            </div>
          )}
          <p className="text-sm text-zinc-800 dark:text-zinc-200 leading-snug">
            {suggestion.title}
          </p>
          {suggestion.description && (
            <p className="text-xs text-zinc-500 dark:text-zinc-400 mt-1">
              {suggestion.description}
            </p>
          )}
        </div>
        {!suggestion.accepted && (
          <div className="flex items-center gap-1 shrink-0">
            <button
              onClick={() => onAccept(suggestion.id)}
              className="p-1 rounded hover:bg-green-100 dark:hover:bg-green-900/30 text-green-600 dark:text-green-400"
              title={t("suggestion.accept")}
            >
              <Check size={14} />
            </button>
            <button
              onClick={() => onSnooze(suggestion.id)}
              className="p-1 rounded hover:bg-blue-100 dark:hover:bg-blue-900/30 text-blue-500 dark:text-blue-400"
              title={t("suggestion.snooze")}
            >
              <Clock size={14} />
            </button>
            <button
              onClick={() => onDismiss(suggestion.id)}
              className="p-1 rounded hover:bg-zinc-200 dark:hover:bg-zinc-700 text-zinc-400"
              title={t("suggestion.dismiss")}
            >
              <X size={14} />
            </button>
          </div>
        )}
        {suggestion.accepted && (
          <Check
            size={14}
            className="shrink-0 text-green-500"
            aria-label={t("suggestion.accept")}
          />
        )}
      </div>
    </div>
  );
};

/** SuggestionPanel — renders proactive suggestions from the backend
 *  suggestion engine, including causal-insight suggestions derived from
 *  the trajectory causal edge layer. */
export const SuggestionPanel: React.FC = () => {
  const { t } = useTranslation();
  const proactiveEnabled = useAppConfigStore(
    (s) => s.features.proactiveMode,
  );
  const suggestions = useProactiveStore((s) => s.suggestions);
  const storeError = useProactiveStore((s) => s.error);
  const refreshSuggestions = useProactiveStore((s) => s.refreshSuggestions);
  const acceptSuggestion = useProactiveStore((s) => s.acceptSuggestion);
  const dismissSuggestion = useProactiveStore((s) => s.dismissSuggestion);
  const snoozeSuggestion = useProactiveStore((s) => s.snoozeSuggestion);

  const [expanded, setExpanded] = useState(false);

  useEffect(() => {
    if (!expanded || !proactiveEnabled) {
      return;
    }
    const load = () => {
      void refreshSuggestions(buildContextFeatures());
    };
    load();
    const interval = setInterval(load, 60_000);
    return () => clearInterval(interval);
  }, [expanded, proactiveEnabled, refreshSuggestions]);

  const handleAccept = useCallback(
    (id: string) => {
      void acceptSuggestion(id);
    },
    [acceptSuggestion],
  );
  const handleDismiss = useCallback(
    (id: string) => {
      void dismissSuggestion(id);
    },
    [dismissSuggestion],
  );
  const handleSnooze = useCallback(
    (id: string) => {
      void snoozeSuggestion(id, 30);
    },
    [snoozeSuggestion],
  );

  if (!proactiveEnabled) {
    return null;
  }

  if (!expanded) {
    return (
      <div className="border-b border-border/50 px-3 py-2">
        <button
          onClick={() => setExpanded(true)}
          className="flex items-center gap-1.5 text-xs text-muted-foreground hover:text-foreground transition-colors"
        >
          <Bell
            size={14}
            className={suggestions.length > 0 ? "text-teal-500" : ""}
          />
          {t("suggestion.title")} ({suggestions.length})
          {storeError && (
            <span
              className="size-1.5 rounded-full bg-red-400"
              title={t("chat.error")}
            />
          )}
        </button>
      </div>
    );
  }

  return (
    <div className="border-b border-border/50">
      <div className="flex items-center justify-between px-3 py-2">
        <span className="text-xs font-medium text-foreground/80">
          {t("suggestion.title")}
        </span>
        <div className="flex items-center gap-1">
          {storeError && (
            <span
              className="size-1.5 rounded-full bg-red-400"
              title={t("chat.error")}
            />
          )}
          {suggestions.length > 0 && (
            <span className="bg-teal-100 dark:bg-teal-900/40 text-teal-600 dark:text-teal-400 rounded-full px-1.5 py-0.5 text-[10px] font-bold">
              {suggestions.length}
            </span>
          )}
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
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                d="M6 18L18 6M6 6l12 12"
              />
            </svg>
          </button>
        </div>
      </div>

      <div className="px-3 pb-3 max-h-64 overflow-y-auto">
        {suggestions.length === 0 && (
          <div className="text-xs text-muted-foreground/60 pb-1">
            {t("suggestion.noSuggestions")}
          </div>
        )}

        {suggestions.map((s) => (
          <SuggestionCard
            key={s.id}
            suggestion={s}
            onAccept={handleAccept}
            onDismiss={handleDismiss}
            onSnooze={handleSnooze}
          />
        ))}
      </div>
    </div>
  );
};

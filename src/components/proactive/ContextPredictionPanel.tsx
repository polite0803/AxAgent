import { useDebounce } from "@/hooks/useDebounce";
import { useProactiveStore } from "@/stores/feature/proactiveStore";
import type { ContextPrediction, PredictedIntent } from "@/types";
import { type ReactNode, useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";

const SEARCH_ICON: ReactNode = (
  <svg
    className="size-5"
    fill="none"
    viewBox="0 0 24 24"
    stroke="currentColor"
    strokeWidth={2}
  >
    <path
      strokeLinecap="round"
      strokeLinejoin="round"
      d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"
    />
  </svg>
);

const INTENT_ICONS: Record<PredictedIntent["type"], ReactNode> = {
  CodeCompletion: (
    <svg
      className="size-5"
      fill="none"
      viewBox="0 0 24 24"
      stroke="currentColor"
      strokeWidth={2}
    >
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        d="M10 20l4-16m4 4l4 4-4 4M6 16l-4-4 4-4"
      />
    </svg>
  ),
  Documentation: (
    <svg
      className="size-5"
      fill="none"
      viewBox="0 0 24 24"
      stroke="currentColor"
      strokeWidth={2}
    >
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"
      />
    </svg>
  ),
  Search: SEARCH_ICON,
  Refactoring: (
    <svg
      className="size-5"
      fill="none"
      viewBox="0 0 24 24"
      stroke="currentColor"
      strokeWidth={2}
    >
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"
      />
    </svg>
  ),
  Debug: (
    <svg
      className="size-5"
      fill="none"
      viewBox="0 0 24 24"
      stroke="currentColor"
      strokeWidth={2}
    >
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
      />
    </svg>
  ),
  TestGeneration: (
    <svg
      className="size-5"
      fill="none"
      viewBox="0 0 24 24"
      stroke="currentColor"
      strokeWidth={2}
    >
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        d="M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2m-6 9l2 2 4-4"
      />
    </svg>
  ),
  Unknown: (
    <svg
      className="size-5"
      fill="none"
      viewBox="0 0 24 24"
      stroke="currentColor"
      strokeWidth={2}
    >
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        d="M8.228 9c.549-1.165 2.03-2 3.772-2 2.21 0 4 1.343 4 3 0 1.4-1.278 2.575-3.006 2.907-.542.104-.994.54-.994 1.093m0 3h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
      />
    </svg>
  ),
};

function getIntentDescription(
  intent: PredictedIntent,
  t: (key: string) => string,
): string {
  switch (intent.type) {
    case "CodeCompletion":
      return `${t("proactive.language")}: ${intent.language}`;
    case "Documentation":
      return intent.topic;
    case "Search":
      return intent.query_type;
    case "Refactoring":
      return intent.target;
    case "Debug":
      return intent.error;
    case "TestGeneration":
      return intent.target;
    case "Unknown":
      return t("proactive.unknown");
  }
}

function getConfidenceColor(confidence: number): string {
  if (Number.isNaN(confidence) || confidence == null) {
    return "text-muted-foreground";
  }
  if (confidence >= 0.8) {
    return "text-green-500";
  }
  if (confidence >= 0.5) {
    return "text-yellow-500";
  }
  return "text-red-500";
}

function formatConfidence(confidence: number): string {
  if (Number.isNaN(confidence) || confidence == null) {
    return "\u2014";
  }
  return `${Math.round(confidence * 100)}%`;
}

function getPredictionKey(
  prediction: ContextPrediction,
  index: number,
): string {
  return `${prediction.created_at}-${prediction.predicted_intent.type}-${index}`;
}

interface ContextPredictionPanelProps {
  context: Record<string, unknown>;
  onApplyPrediction?: (prediction: ContextPrediction) => void;
}

const DEBOUNCE_MS = 800;

export function ContextPredictionPanel({
  context,
  onApplyPrediction,
}: ContextPredictionPanelProps) {
  const { t } = useTranslation();

  const predictions = useProactiveStore((s) => s.predictions);
  const fetchPredictions = useProactiveStore((s) => s.fetchPredictions);
  const isLoading = useProactiveStore((s) => s.isLoading);
  const error = useProactiveStore((s) => s.error);

  const [retrying, setRetrying] = useState(false);
  const [appliedKey, setAppliedKey] = useState<string | null>(null);
  const mountedRef = useRef(true);

  const contextKey = useMemo(() => JSON.stringify(context), [context]);
  const debouncedContextKey = useDebounce(contextKey, DEBOUNCE_MS);

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
    };
  }, []);

  useEffect(() => {
    if (!context || Object.keys(context).length === 0) {
      return;
    }
    if (!debouncedContextKey) {
      return;
    }

    fetchPredictions(context);
  }, [debouncedContextKey]);

  const handleRetry = useCallback(async () => {
    if (!context || Object.keys(context).length === 0) {
      return;
    }
    setRetrying(true);
    try {
      await fetchPredictions(context);
    } finally {
      if (mountedRef.current) {
        setRetrying(false);
      }
    }
  }, [context, fetchPredictions]);

  const handleApplyPrediction = useCallback(
    async (prediction: ContextPrediction) => {
      const key = getPredictionKey(prediction, 0);
      if (onApplyPrediction) {
        onApplyPrediction(prediction);
      } else {
        const text = `${prediction.predicted_intent.type}: ${prediction.reasoning}`;
        try {
          await navigator.clipboard.writeText(text);
        } catch {
          // clipboard unavailable — silently ignore
        }
      }
      setAppliedKey(key);
      const timer = setTimeout(() => {
        if (mountedRef.current) {
          setAppliedKey(null);
        }
      }, 2000);
      return () => clearTimeout(timer);
    },
    [onApplyPrediction],
  );

  const isRetrying = retrying || (isLoading && error !== null);

  return (
    <div className="bg-card border rounded-lg">
      <div className="px-4 py-3 border-b">
        <h3 className="font-medium flex items-center gap-2">
          <svg
            className="size-4 text-primary"
            fill="none"
            viewBox="0 0 24 24"
            stroke="currentColor"
            strokeWidth={2}
          >
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              d="M9.663 17h4.673M12 3v1m6.364 1.636l-.707.707M21 12h-1M4 12H3m3.343-5.657l-.707-.707m2.828 9.9a5 5 0 117.072 0l-.548.547A3.374 3.374 0 0014 18.469V19a2 2 0 11-4 0v-.531c0-.895-.356-1.754-.988-2.386l-.548-.547z"
            />
          </svg>
          {t("proactive.contextPrediction")}
        </h3>
      </div>

      <div className="p-4">
        {isLoading && !retrying
          ? (
            <div className="flex items-center justify-center py-8">
              <div className="size-6 border-2 border-primary border-t-transparent rounded-full animate-spin" />
            </div>
          )
          : error && !isRetrying
          ? (
            <div className="text-sm text-destructive space-y-2">
              <p>{error}</p>
              <button
                onClick={handleRetry}
                disabled={isRetrying}
                className="px-3 py-1 text-xs rounded bg-primary/10 text-primary hover:bg-primary/20 transition-colors disabled:opacity-50"
              >
                {t("proactive.retry")}
              </button>
            </div>
          )
          : isRetrying
          ? (
            <div className="flex items-center justify-center py-8">
              <div className="size-6 border-2 border-primary border-t-transparent rounded-full animate-spin" />
            </div>
          )
          : predictions.length === 0
          ? (
            <div className="text-sm text-muted-foreground text-center py-4">
              {t("proactive.noPredictions")}
            </div>
          )
          : (
            <div className="space-y-3">
              {predictions.map((prediction, index) => {
                const key = getPredictionKey(prediction, index);
                const isApplied = appliedKey === key;

                return (
                  <button
                    key={key}
                    type="button"
                    onClick={() => handleApplyPrediction(prediction)}
                    className={`w-full p-3 rounded-lg text-left transition-colors ${
                      isApplied
                        ? "bg-primary/10 ring-1 ring-primary"
                        : "bg-muted/50 hover:bg-muted cursor-pointer"
                    }`}
                  >
                    <div className="flex items-start gap-3">
                      <div className="text-primary mt-0.5 shrink-0">
                        {INTENT_ICONS[prediction.predicted_intent.type]
                          ?? INTENT_ICONS.Unknown}
                      </div>
                      <div className="flex-1 min-w-0">
                        <div className="flex items-center justify-between">
                          <span className="text-sm font-medium">
                            {prediction.predicted_intent.type}
                          </span>
                          <span
                            className={`text-xs font-medium ${getConfidenceColor(prediction.confidence)}`}
                          >
                            {formatConfidence(prediction.confidence)}
                          </span>
                        </div>
                        <p className="text-xs text-muted-foreground mt-1">
                          {getIntentDescription(prediction.predicted_intent, t)}
                        </p>
                        <p className="text-xs text-muted-foreground mt-2 italic">
                          {prediction.reasoning}
                        </p>
                      </div>
                    </div>

                    {prediction.suggested_actions.length > 0 && (
                      <div className="mt-3 pt-3 border-t">
                        <p className="text-xs text-muted-foreground mb-2">
                          {t("proactive.suggestedActions")}:
                        </p>
                        <div className="flex flex-wrap gap-1">
                          {prediction.suggested_actions.map(
                            (action, actionIndex) => (
                              <span
                                key={`${key}-action-${actionIndex}`}
                                className="px-2 py-0.5 text-xs bg-primary/10 text-primary rounded"
                              >
                                {action.title}
                              </span>
                            ),
                          )}
                        </div>
                      </div>
                    )}

                    {isApplied && (
                      <div className="mt-2 text-xs text-primary font-medium">
                        {onApplyPrediction
                          ? t("proactive.applied")
                          : t("proactive.copied")}
                      </div>
                    )}
                  </button>
                );
              })}
            </div>
          )}
      </div>
    </div>
  );
}

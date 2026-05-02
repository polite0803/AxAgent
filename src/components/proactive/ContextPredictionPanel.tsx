import { useProactiveStore } from "@/stores/feature/proactiveStore";
import type { ContextPrediction } from "@/types/proactive";
import { useEffect } from "react";
import { useTranslation } from "react-i18next";

interface ContextPredictionPanelProps {
  context: Record<string, unknown>;
}

export default function ContextPredictionPanel({ context }: ContextPredictionPanelProps) {
  const { t } = useTranslation();
  const { predictions, fetchPredictions, isLoading, error } = useProactiveStore();

  useEffect(() => {
    if (context && Object.keys(context).length > 0) {
      fetchPredictions(context);
    }
  }, [context, fetchPredictions]);

  const getIntentIcon = (intent: ContextPrediction["predicted_intent"]) => {
    switch (intent.type) {
      case "CodeCompletion":
        return (
          <svg className="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
            <path strokeLinecap="round" strokeLinejoin="round" d="M10 20l4-16m4 4l4 4-4 4M6 16l-4-4 4-4" />
          </svg>
        );
      case "Documentation":
        return (
          <svg className="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"
            />
          </svg>
        );
      case "Search":
        return (
          <svg className="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
            <path strokeLinecap="round" strokeLinejoin="round" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
          </svg>
        );
      case "Refactoring":
        return (
          <svg className="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"
            />
          </svg>
        );
      case "Debug":
        return (
          <svg className="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
            <path strokeLinecap="round" strokeLinejoin="round" d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
          </svg>
        );
      case "TestGeneration":
        return (
          <svg className="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              d="M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2m-6 9l2 2 4-4"
            />
          </svg>
        );
      default:
        return (
          <svg className="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              d="M8.228 9c.549-1.165 2.03-2 3.772-2 2.21 0 4 1.343 4 3 0 1.4-1.278 2.575-3.006 2.907-.542.104-.994.54-.994 1.093m0 3h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
            />
          </svg>
        );
    }
  };

  const getIntentDescription = (intent: ContextPrediction["predicted_intent"]) => {
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
      default:
        return t("proactive.unknown");
    }
  };

  const getConfidenceColor = (confidence: number) => {
    if (confidence >= 0.8) { return "text-green-500"; }
    if (confidence >= 0.5) { return "text-yellow-500"; }
    return "text-red-500";
  };

  return (
    <div className="bg-card border rounded-lg">
      <div className="px-4 py-3 border-b">
        <h3 className="font-medium flex items-center gap-2">
          <svg className="w-4 h-4 text-primary" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
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
        {isLoading
          ? (
            <div className="flex items-center justify-center py-8">
              <div className="w-6 h-6 border-2 border-primary border-t-transparent rounded-full animate-spin" />
            </div>
          )
          : error
          ? <div className="text-sm text-destructive">{error}</div>
          : predictions.length === 0
          ? (
            <div className="text-sm text-muted-foreground text-center py-4">
              {t("proactive.noPredictions")}
            </div>
          )
          : (
            <div className="space-y-3">
              {predictions.map((prediction, index) => (
                <div
                  key={index}
                  className="p-3 rounded-lg bg-muted/50 hover:bg-muted transition-colors"
                >
                  <div className="flex items-start gap-3">
                    <div className="text-primary mt-0.5">
                      {getIntentIcon(prediction.predicted_intent)}
                    </div>
                    <div className="flex-1 min-w-0">
                      <div className="flex items-center justify-between">
                        <span className="text-sm font-medium">
                          {prediction.predicted_intent.type}
                        </span>
                        <span className={`text-xs font-medium ${getConfidenceColor(prediction.confidence)}`}>
                          {Math.round(prediction.confidence * 100)}%
                        </span>
                      </div>
                      <p className="text-xs text-muted-foreground mt-1">
                        {getIntentDescription(prediction.predicted_intent)}
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
                        {prediction.suggested_actions.map((action, actionIndex) => (
                          <span
                            key={actionIndex}
                            className="px-2 py-0.5 text-xs bg-primary/10 text-primary rounded"
                          >
                            {action.title}
                          </span>
                        ))}
                      </div>
                    </div>
                  )}
                </div>
              ))}
            </div>
          )}
      </div>
    </div>
  );
}

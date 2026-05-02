import type { PrefetchResult, PrefetchType } from "@/types/proactive";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

interface PrefetchIndicatorProps {
  results?: PrefetchResult[];
  totalEstimatedTime?: number;
  isActive?: boolean;
}

export default function PrefetchIndicator({
  results = [],
  totalEstimatedTime = 0,
  isActive = false,
}: PrefetchIndicatorProps) {
  const { t } = useTranslation();
  const [visible, setVisible] = useState(false);
  const [mounted, setMounted] = useState(false);

  useEffect(() => {
    if (isActive || results.length > 0) {
      setVisible(true);
      setMounted(true);
    } else if (results.length === 0) {
      setVisible(false);
      const timer = setTimeout(() => setMounted(false), 300);
      return () => clearTimeout(timer);
    }
  }, [isActive, results.length]);

  if (!mounted) { return null; }

  const readyCount = results.filter((r) => r.ready).length;
  const totalCount = results.length;
  const progress = totalCount > 0 ? (readyCount / totalCount) * 100 : 0;

  const getTypeIcon = (type: PrefetchType) => {
    switch (type) {
      case "codeCompletion":
        return (
          <svg className="w-3 h-3" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
            <path strokeLinecap="round" strokeLinejoin="round" d="M10 20l4-16m4 4l4 4-4 4M6 16l-4-4 4-4" />
          </svg>
        );
      case "searchResults":
        return (
          <svg className="w-3 h-3" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
            <path strokeLinecap="round" strokeLinejoin="round" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
          </svg>
        );
      case "documentation":
        return (
          <svg className="w-3 h-3" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"
            />
          </svg>
        );
      case "contextAnalysis":
        return (
          <svg className="w-3 h-3" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              d="M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2"
            />
          </svg>
        );
      case "toolCache":
        return (
          <svg className="w-3 h-3" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              d="M4 7v10c0 2.21 3.582 4 8 4s8-1.79 8-4V7M4 7c0 2.21 3.582 4 8 4s8-1.79 8-4M4 7c0-2.21 3.582-4 8-4s8 1.79 8 4"
            />
          </svg>
        );
    }
  };

  return (
    <div
      className={`fixed bottom-4 right-4 z-50 transition-all duration-300 ${
        visible ? "opacity-100 translate-y-0" : "opacity-0 translate-y-2"
      }`}
    >
      <div className="bg-card border rounded-lg shadow-lg w-72">
        <div className="px-3 py-2 border-b flex items-center justify-between">
          <div className="flex items-center gap-2">
            <div className={`w-2 h-2 rounded-full ${isActive ? "bg-yellow-500 animate-pulse" : "bg-green-500"}`} />
            <span className="text-sm font-medium">
              {isActive ? t("proactive.prefetching") : t("proactive.prefetchReady")}
            </span>
          </div>
          <span className="text-xs text-muted-foreground">
            {readyCount}/{totalCount}
          </span>
        </div>

        <div className="p-3">
          <div className="h-1.5 bg-muted rounded-full overflow-hidden mb-3">
            <div
              className="h-full bg-primary transition-all duration-300"
              style={{ width: `${progress}%` }}
            />
          </div>

          {results.length > 0 && (
            <div className="space-y-2">
              {results.slice(0, 4).map((result, index) => (
                <div key={index} className="flex items-center gap-2">
                  <div className={`${result.ready ? "text-green-500" : "text-muted-foreground"}`}>
                    {result.ready
                      ? (
                        <svg className="w-3 h-3" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
                          <path strokeLinecap="round" strokeLinejoin="round" d="M5 13l4 4L19 7" />
                        </svg>
                      )
                      : (
                        getTypeIcon(result.prefetch_type)
                      )}
                  </div>
                  <span className="text-xs text-muted-foreground flex-1 truncate">
                    {result.prefetch_type}
                  </span>
                  <span className="text-xs text-muted-foreground">
                    {result.estimated_prepare_time_ms}ms
                  </span>
                </div>
              ))}
              {results.length > 4 && (
                <div className="text-xs text-muted-foreground text-center">
                  +{results.length - 4} more
                </div>
              )}
            </div>
          )}

          {totalEstimatedTime > 0 && (
            <div className="mt-3 pt-2 border-t flex items-center justify-between text-xs text-muted-foreground">
              <span>{t("proactive.estimatedTime")}</span>
              <span>{totalEstimatedTime}ms</span>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

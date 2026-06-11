// SPDX-License-Identifier: AGPL-3.0-only

import { invoke, logIpcError } from "@/lib/invoke";
import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";

interface TrajectoryPattern {
  id: string;
  name: string;
  description: string;
  pattern_type: string;
  frequency: number;
  success_rate: number;
  average_quality: number;
}

type FilterMode = "all" | "success" | "medium" | "failure";

export function PatternPanel() {
  const { t } = useTranslation();
  const [expanded, setExpanded] = useState(false);
  const [patterns, setPatterns] = useState<TrajectoryPattern[]>([]);
  const [filter, setFilter] = useState<FilterMode>("all");
  const [error, setError] = useState(false);
  const mountedRef = useRef(true);

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
    };
  }, []);

  useEffect(() => {
    if (!expanded) {
      return;
    }
    const fetch = async () => {
      try {
        const p = await invoke<TrajectoryPattern[]>("pattern_list", {});
        if (mountedRef.current) {
          setPatterns(p);
          setError(false);
        }
      } catch (e) {
        logIpcError("Failed to fetch patterns")(e);
        if (mountedRef.current) {
          setError(true);
        }
      }
    };
    fetch();
    const interval = setInterval(fetch, 60000);
    return () => clearInterval(interval);
  }, [expanded]);

  const filtered = patterns.filter((p) => {
    if (filter === "success") {
      return p.success_rate >= 0.6;
    }
    if (filter === "medium") {
      return p.success_rate >= 0.4 && p.success_rate < 0.6;
    }
    if (filter === "failure") {
      return p.success_rate < 0.4;
    }
    return true;
  });

  if (!expanded) {
    return (
      <div className="border-b border-border/50 px-3 py-2">
        <button
          onClick={() => setExpanded(true)}
          className="flex items-center gap-1.5 text-xs text-muted-foreground hover:text-foreground transition-colors"
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
              d="M3.75 6A2.25 2.25 0 016 3.75h2.25A2.25 2.25 0 0110.5 6v2.25a2.25 2.25 0 01-2.25 2.25H6a2.25 2.25 0 01-2.25-2.25V6zM3.75 15.75A2.25 2.25 0 016 13.5h2.25a2.25 2.25 0 012.25 2.25V18a2.25 2.25 0 01-2.25 2.25H6A2.25 2.25 0 013.75 18v-2.25zM13.5 6a2.25 2.25 0 012.25-2.25H18A2.25 2.25 0 0120.25 6v2.25A2.25 2.25 0 0118 10.5h-2.25a2.25 2.25 0 01-2.25-2.25V6zM13.5 15.75a2.25 2.25 0 012.25-2.25H18a2.25 2.25 0 012.25 2.25V18A2.25 2.25 0 0118 20.25h-2.25A2.25 2.25 0 0113.5 18v-2.25z"
            />
          </svg>
          {t("chat.patterns")} ({patterns.length})
          {error && (
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
    <div className="border-b border-border/50 px-3 py-2 space-y-2">
      <div className="flex items-center justify-between">
        <span className="text-xs font-medium text-foreground/80">
          {t("chat.learnedPatterns")}
        </span>
        <div className="flex items-center gap-1">
          <select
            value={filter}
            onChange={(e) => setFilter(e.target.value as FilterMode)}
            className="text-[10px] bg-muted/30 rounded px-1 py-0.5 border-none outline-none"
          >
            <option value="all">All</option>
            <option value="success">Success</option>
            <option value="medium">Medium</option>
            <option value="failure">Failure</option>
          </select>
          {error && (
            <span
              className="size-1.5 rounded-full bg-red-400"
              title={t("chat.error")}
            />
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

      {filtered.length > 0
        ? (
          <div className="max-h-40 overflow-y-auto space-y-1">
            {filtered.slice(0, 10).map((p) => (
              <div key={p.id} className="text-xs p-1.5 rounded bg-muted/30">
                <div className="flex items-center gap-1 mb-0.5">
                  <span
                    className={`inline-block px-1 rounded text-[10px] font-medium ${
                      p.success_rate >= 0.6
                        ? "bg-green-500/10 text-green-500"
                        : p.success_rate < 0.4
                        ? "bg-red-500/10 text-red-500"
                        : "bg-amber-500/10 text-amber-500"
                    }`}
                  >
                    {p.pattern_type}
                  </span>
                  <span className="text-foreground/80 truncate">{p.name}</span>
                  <span className="text-[10px] text-muted-foreground/60 ml-auto">
                    {Math.round(p.success_rate * 100)}% x{p.frequency}
                  </span>
                </div>
                <div className="text-[11px] text-foreground/60 line-clamp-1">
                  {p.description}
                </div>
              </div>
            ))}
          </div>
        )
        : (
          <div className="text-xs text-muted-foreground/60">
            {error && patterns.length === 0
              ? t("chat.loadError")
              : patterns.length === 0
              ? t("chat.noPatternsYet")
              : t("chat.noPatternsFilter")}
          </div>
        )}
    </div>
  );
}

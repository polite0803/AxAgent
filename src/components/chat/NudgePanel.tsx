import { useConversationStore, useNudgeStore } from "@/stores";
import type { Nudge, PeriodicNudge } from "@/types";
import { Bell, Check, Clock, Lightbulb, X } from "lucide-react";
import React, { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";

const urgencyColor: Record<string, string> = {
  high: "border-orange-400 bg-orange-50 dark:bg-orange-950/30",
  medium: "border-blue-400 bg-blue-50 dark:bg-blue-950/30",
  low: "border-zinc-300 bg-zinc-50 dark:bg-zinc-900/30",
};

const urgencyDot: Record<string, string> = {
  high: "bg-orange-500",
  medium: "bg-blue-500",
  low: "bg-zinc-400",
};

const nudgeTypeIcon: Record<string, string> = {
  memory_consolidation: "🧠",
  skill_creation: "⚡",
  pattern_learn: "🔄",
  review_reminder: "📋",
};

/** Single nudge card */
const NudgeCard: React.FC<{
  nudge: Nudge;
  onDismiss: (id: string) => void;
  onExecute: (id: string) => void;
  onSnooze: (id: string, until: number) => void;
}> = ({ nudge, onDismiss, onExecute, onSnooze }) => {
  const { t } = useTranslation();
  const urgency = nudge.urgency;

  return (
    <div
      className={`rounded-lg border-l-4 p-3 mb-2 transition-all ${urgencyColor[urgency] || urgencyColor.low}`}
    >
      <div className="flex items-start gap-2">
        <div
          className={`size-2 rounded-full mt-1.5 shrink-0 ${urgencyDot[urgency] || urgencyDot.low}`}
        />
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-1 text-xs font-medium text-zinc-500 dark:text-zinc-400 mb-1">
            <Lightbulb size={12} />
            <span>{nudge.entityName}</span>
          </div>
          <p className="text-sm text-zinc-800 dark:text-zinc-200 leading-snug">
            {nudge.reason}
          </p>
          {nudge.suggestedAction && (
            <p className="text-xs text-zinc-500 dark:text-zinc-400 mt-1 italic">
              {nudge.suggestedAction}
            </p>
          )}
        </div>
        <div className="flex items-center gap-1 shrink-0">
          {nudge.suggestedAction && (
            <button
              onClick={() => onExecute(nudge.id)}
              className="p-1 rounded hover:bg-green-100 dark:hover:bg-green-900/30 text-green-600 dark:text-green-400"
              title={t("nudge.execute")}
            >
              <Check size={14} />
            </button>
          )}
          <button
            // eslint-disable-next-line react-doctor/rendering-hydration-mismatch-time
            onClick={() => onSnooze(nudge.id, Date.now() + 30 * 60 * 1000)}
            className="p-1 rounded hover:bg-blue-100 dark:hover:bg-blue-900/30 text-blue-500 dark:text-blue-400"
            title={t("nudge.snooze30")}
          >
            <Clock size={14} />
          </button>
          <button
            onClick={() => onDismiss(nudge.id)}
            className="p-1 rounded hover:bg-zinc-200 dark:hover:bg-zinc-700 text-zinc-400"
            title={t("nudge.dismiss")}
          >
            <X size={14} />
          </button>
        </div>
      </div>
    </div>
  );
};

/** Closed-loop periodic nudge card */
const ClosedLoopNudgeCard: React.FC<{
  nudge: PeriodicNudge;
  onAcknowledge: (id: string) => void;
}> = ({ nudge, onAcknowledge }) => {
  const icon = nudgeTypeIcon[nudge.nudgeType] || "💡";

  return (
    <div className="rounded-lg border border-dashed border-zinc-300 dark:border-zinc-600 p-3 mb-2 bg-zinc-50/50 dark:bg-zinc-900/20">
      <div className="flex items-start gap-2">
        <span className="text-base">{icon}</span>
        <div className="flex-1 min-w-0">
          <div className="text-xs font-medium text-zinc-500 dark:text-zinc-400 mb-0.5">
            {nudge.title}
          </div>
          <p className="text-sm text-zinc-700 dark:text-zinc-300 leading-snug">
            {nudge.description}
          </p>
        </div>
        {!nudge.acknowledged && (
          <button
            onClick={() => onAcknowledge(nudge.id)}
            className="p-1 rounded hover:bg-zinc-200 dark:hover:bg-zinc-700 text-zinc-400 shrink-0"
          >
            <X size={14} />
          </button>
        )}
      </div>
    </div>
  );
};

/** NudgePanel — displays self-evolution learning suggestions */
export const NudgePanel: React.FC = () => {
  const { t } = useTranslation();
  const activeConversationId = useConversationStore(
    (s) => s.activeConversationId,
  );
  const pendingNudges = useNudgeStore((s) => s.pendingNudges);
  const closedLoopNudges = useNudgeStore((s) => s.closedLoopNudges);
  const stats = useNudgeStore((s) => s.stats);
  const fetchPendingNudges = useNudgeStore((s) => s.fetchPendingNudges);
  const fetchClosedLoopNudges = useNudgeStore((s) => s.fetchClosedLoopNudges);
  const fetchStats = useNudgeStore((s) => s.fetchStats);
  const dismissNudge = useNudgeStore((s) => s.dismissNudge);
  const snoozeNudge = useNudgeStore((s) => s.snoozeNudge);
  const executeNudge = useNudgeStore((s) => s.executeNudge);
  const acknowledgeClosedLoopNudge = useNudgeStore(
    (s) => s.acknowledgeClosedLoopNudge,
  );

  const [expanded, setExpanded] = useState(false);
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
    const load = async () => {
      try {
        if (activeConversationId) {
          await fetchPendingNudges(activeConversationId);
        }
        await fetchClosedLoopNudges();
        await fetchStats();
        if (mountedRef.current) {
          setError(false);
        }
      } catch {
        if (mountedRef.current) {
          setError(true);
        }
      }
    };
    load();
    const interval = setInterval(load, 60_000);
    return () => clearInterval(interval);
  }, [
    expanded,
    activeConversationId,
    fetchPendingNudges,
    fetchClosedLoopNudges,
    fetchStats,
  ]);

  const unacknowledgedClosedLoop = closedLoopNudges.filter(
    (n) => !n.acknowledged,
  );
  const totalItems = pendingNudges.length + unacknowledgedClosedLoop.length;

  if (!expanded) {
    return (
      <div className="border-b border-border/50 px-3 py-2">
        <button
          onClick={() => setExpanded(true)}
          className="flex items-center gap-1.5 text-xs text-muted-foreground hover:text-foreground transition-colors"
        >
          <Bell size={14} className={totalItems > 0 ? "text-orange-500" : ""} />
          {t("nudge.learningSuggestions")} ({totalItems})
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
    <div className="border-b border-border/50">
      <div className="flex items-center justify-between px-3 py-2">
        <span className="text-xs font-medium text-foreground/80">
          {t("nudge.learningSuggestions")}
        </span>
        <div className="flex items-center gap-1">
          {error && (
            <span
              className="size-1.5 rounded-full bg-red-400"
              title={t("chat.error")}
            />
          )}
          {totalItems > 0 && (
            <span className="bg-orange-100 dark:bg-orange-900/40 text-orange-600 dark:text-orange-400 rounded-full px-1.5 py-0.5 text-[10px] font-bold">
              {totalItems}
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
        {totalItems === 0 && (
          <div className="text-xs text-muted-foreground/60 pb-1">
            {t("nudge.noSuggestions")}
          </div>
        )}

        {pendingNudges.map((n) => (
          <NudgeCard
            key={n.id}
            nudge={n}
            onDismiss={dismissNudge}
            onExecute={executeNudge}
            onSnooze={snoozeNudge}
          />
        ))}

        {unacknowledgedClosedLoop.map((n) => (
          <ClosedLoopNudgeCard
            key={n.id}
            nudge={n}
            onAcknowledge={acknowledgeClosedLoopNudge}
          />
        ))}

        {stats && stats.totalNudges > 0 && (
          <div className="text-[10px] text-zinc-400 dark:text-zinc-500 mt-2 text-right">
            {t("nudge.acceptanceRate")}: {(stats.acceptanceRate * 100).toFixed(0)}% (
            {stats.addedToMemoryCount}/{stats.presentedCount})
          </div>
        )}
      </div>
    </div>
  );
};

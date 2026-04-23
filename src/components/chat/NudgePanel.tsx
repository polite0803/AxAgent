import React, { useEffect, useState } from 'react';
import { useNudgeStore, useConversationStore } from '@/stores';
import { Lightbulb, X, Clock, Check, Bell } from 'lucide-react';
import type { Nudge, PeriodicNudge } from '@/types/nudge';
import { useTranslation } from 'react-i18next';

const urgencyColor: Record<string, string> = {
  high: 'border-orange-400 bg-orange-50 dark:bg-orange-950/30',
  medium: 'border-blue-400 bg-blue-50 dark:bg-blue-950/30',
  low: 'border-gray-300 bg-gray-50 dark:bg-gray-900/30',
};

const urgencyDot: Record<string, string> = {
  high: 'bg-orange-500',
  medium: 'bg-blue-500',
  low: 'bg-gray-400',
};

const nudgeTypeIcon: Record<string, string> = {
  memory_consolidation: '🧠',
  skill_creation: '⚡',
  pattern_learn: '🔄',
  review_reminder: '📋',
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
        <div className={`w-2 h-2 rounded-full mt-1.5 shrink-0 ${urgencyDot[urgency] || urgencyDot.low}`} />
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-1 text-xs font-medium text-gray-500 dark:text-gray-400 mb-1">
            <Lightbulb size={12} />
            <span>{nudge.entityName}</span>
          </div>
          <p className="text-sm text-gray-800 dark:text-gray-200 leading-snug">
            {nudge.reason}
          </p>
          {nudge.suggestedAction && (
            <p className="text-xs text-gray-500 dark:text-gray-400 mt-1 italic">
              {nudge.suggestedAction}
            </p>
          )}
        </div>
        <div className="flex items-center gap-1 shrink-0">
          {nudge.suggestedAction && (
            <button
              onClick={() => onExecute(nudge.id)}
              className="p-1 rounded hover:bg-green-100 dark:hover:bg-green-900/30 text-green-600 dark:text-green-400"
              title={t('nudge.execute', 'Execute')}
            >
              <Check size={14} />
            </button>
          )}
          <button
            onClick={() => onSnooze(nudge.id, Date.now() + 30 * 60 * 1000)}
            className="p-1 rounded hover:bg-blue-100 dark:hover:bg-blue-900/30 text-blue-500 dark:text-blue-400"
            title={t('nudge.snooze30', 'Snooze 30min')}
          >
            <Clock size={14} />
          </button>
          <button
            onClick={() => onDismiss(nudge.id)}
            className="p-1 rounded hover:bg-gray-200 dark:hover:bg-gray-700 text-gray-400"
            title={t('nudge.dismiss', 'Dismiss')}
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
  const icon = nudgeTypeIcon[nudge.nudgeType] || '💡';

  return (
    <div className="rounded-lg border border-dashed border-gray-300 dark:border-gray-600 p-3 mb-2 bg-gray-50/50 dark:bg-gray-900/20">
      <div className="flex items-start gap-2">
        <span className="text-base">{icon}</span>
        <div className="flex-1 min-w-0">
          <div className="text-xs font-medium text-gray-500 dark:text-gray-400 mb-0.5">
            {nudge.title}
          </div>
          <p className="text-sm text-gray-700 dark:text-gray-300 leading-snug">
            {nudge.description}
          </p>
        </div>
        {!nudge.acknowledged && (
          <button
            onClick={() => onAcknowledge(nudge.id)}
            className="p-1 rounded hover:bg-gray-200 dark:hover:bg-gray-700 text-gray-400 shrink-0"
          >
            <X size={14} />
          </button>
        )}
      </div>
    </div>
  );
};

/** NudgePanel — displays self-evolution learning suggestions */
const NudgePanel: React.FC = () => {
  const { t } = useTranslation();
  const activeConversationId = useConversationStore((s) => s.activeConversationId);
  const pendingNudges = useNudgeStore((s) => s.pendingNudges);
  const closedLoopNudges = useNudgeStore((s) => s.closedLoopNudges);
  const stats = useNudgeStore((s) => s.stats);
  const fetchPendingNudges = useNudgeStore((s) => s.fetchPendingNudges);
  const fetchClosedLoopNudges = useNudgeStore((s) => s.fetchClosedLoopNudges);
  const fetchStats = useNudgeStore((s) => s.fetchStats);
  const dismissNudge = useNudgeStore((s) => s.dismissNudge);
  const snoozeNudge = useNudgeStore((s) => s.snoozeNudge);
  const executeNudge = useNudgeStore((s) => s.executeNudge);
  const acknowledgeClosedLoopNudge = useNudgeStore((s) => s.acknowledgeClosedLoopNudge);

  const [expanded, setExpanded] = useState(true);

  // Fetch nudges periodically
  useEffect(() => {
    const load = () => {
      if (activeConversationId) {
        fetchPendingNudges(activeConversationId);
      }
      fetchClosedLoopNudges();
      fetchStats();
    };
    load();
    const interval = setInterval(load, 60_000); // refresh every minute
    return () => clearInterval(interval);
  }, [activeConversationId, fetchPendingNudges, fetchClosedLoopNudges, fetchStats]);

  const unacknowledgedClosedLoop = closedLoopNudges.filter((n) => !n.acknowledged);
  const totalItems = pendingNudges.length + unacknowledgedClosedLoop.length;

  if (totalItems === 0) return null;

  return (
    <div className="border-t border-gray-200 dark:border-gray-700">
      {/* Header */}
      <button
        onClick={() => setExpanded(!expanded)}
        className="w-full flex items-center gap-2 px-3 py-2 text-xs font-medium text-gray-500 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-800 transition-colors"
      >
        <Bell size={14} className={totalItems > 0 ? 'text-orange-500' : ''} />
        <span>{t('nudge.learningSuggestions', 'Learning Suggestions')}</span>
        {totalItems > 0 && (
          <span className="ml-auto bg-orange-100 dark:bg-orange-900/40 text-orange-600 dark:text-orange-400 rounded-full px-1.5 py-0.5 text-[10px] font-bold">
            {totalItems}
          </span>
        )}
        <span className="ml-1 text-[10px]">{expanded ? '▲' : '▼'}</span>
      </button>

      {/* Content */}
      {expanded && (
        <div className="px-3 pb-3 max-h-64 overflow-y-auto">
          {/* Session nudges */}
          {pendingNudges.map((n) => (
            <NudgeCard
              key={n.id}
              nudge={n}
              onDismiss={dismissNudge}
              onExecute={executeNudge}
              onSnooze={snoozeNudge}
            />
          ))}

          {/* Closed-loop periodic nudges */}
          {unacknowledgedClosedLoop.map((n) => (
            <ClosedLoopNudgeCard
              key={n.id}
              nudge={n}
              onAcknowledge={acknowledgeClosedLoopNudge}
            />
          ))}

          {/* Stats summary */}
          {stats && stats.totalNudges > 0 && (
            <div className="text-[10px] text-gray-400 dark:text-gray-500 mt-2 text-right">
              {t('nudge.acceptanceRate', 'Acceptance')}: {(stats.acceptanceRate * 100).toFixed(0)}% ({stats.addedToMemoryCount}/{stats.presentedCount})
            </div>
          )}
        </div>
      )}
    </div>
  );
};

export default NudgePanel;

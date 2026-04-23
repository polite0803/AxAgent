import React, { useEffect, useState } from 'react';
import { invoke } from '@/lib/invoke';
import { useAgentStore, useConversationStore, useStreamStore } from '@/stores';
import { useTranslation } from 'react-i18next';
import { Pause, Play, Activity, Clock, Shield, Wrench } from 'lucide-react';

interface RuntimeStats {
  conversationId: string;
  running: boolean;
  paused: boolean;
  activeSessions: number;
  pendingPermissions: number;
  pendingAskUser: number;
  activeToolCalls: number;
}

const AgentStatsPanel: React.FC = () => {
  const { t } = useTranslation();
  const [stats, setStats] = useState<RuntimeStats | null>(null);
  const [elapsed, setElapsed] = useState(0);
  const activeConversationId = useConversationStore((s) => s.activeConversationId);
  const streaming = useStreamStore((s) => s.streaming);
  const queryStats = useAgentStore((s) => s.queryStats);
  const streamingMessageId = useStreamStore((s) => s.streamingMessageId);
  const pauseAgent = useAgentStore((s) => s.pauseAgent);
  const resumeAgent = useAgentStore((s) => s.resumeAgent);
  const isPaused = useAgentStore((s) => s.isAgentPaused);

  // Poll runtime stats while agent is running
  useEffect(() => {
    if (!streaming || !activeConversationId) {
      setStats(null);
      setElapsed(0);
      return;
    }

    const startTime = Date.now();
    setElapsed(0);

    const interval = setInterval(async () => {
      try {
        const s = await invoke<RuntimeStats>('agent_runtime_stats', {
          conversationId: activeConversationId,
        });
        setStats(s);
        setElapsed(Math.floor((Date.now() - startTime) / 1000));
      } catch {
        // ignore
      }
    }, 2000);

    // Initial fetch
    invoke<RuntimeStats>('agent_runtime_stats', {
      conversationId: activeConversationId,
    }).then(setStats).catch(() => {});

    return () => clearInterval(interval);
  }, [streaming, activeConversationId]);

  if (!streaming || !stats) return null;

  const currentQueryStats = streamingMessageId ? queryStats[streamingMessageId] : null;
  const paused = isPaused(activeConversationId!);

  const formatElapsed = (secs: number) => {
    const m = Math.floor(secs / 60);
    const s = secs % 60;
    return m > 0 ? `${m}m ${s}s` : `${s}s`;
  };

  const formatCost = (cost?: number) => {
    if (cost === undefined || cost === null) return '--';
    if (cost < 1.0) return '<$1.0';
    return `$${cost.toFixed(3)}`;
  };

  return (
    <div className="flex items-center gap-3 px-3 py-1.5 bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 rounded-lg text-xs text-blue-700 dark:text-blue-300">
      {/* Status indicator */}
      <div className="flex items-center gap-1">
        {paused ? (
          <Pause size={12} className="text-orange-500" />
        ) : (
          <Activity size={12} className="animate-pulse text-blue-500" />
        )}
        <span className="font-medium">{paused ? t('chat.agentStats.paused') : t('chat.agentStats.running')}</span>
      </div>

      {/* Elapsed time */}
      <div className="flex items-center gap-1">
        <Clock size={12} />
        <span>{formatElapsed(elapsed)}</span>
      </div>

      {/* Token usage */}
      {currentQueryStats && (
        <div className="flex items-center gap-1">
          <span>
            {(currentQueryStats.inputTokens || 0) + (currentQueryStats.outputTokens || 0)} {t('chat.agentStats.tokens')}
          </span>
          <span className="text-blue-500/70">({formatCost(currentQueryStats.costUsd)})</span>
        </div>
      )}

      {/* Pending permissions */}
      {stats.pendingPermissions > 0 && (
        <div className="flex items-center gap-1 text-orange-600 dark:text-orange-400">
          <Shield size={12} />
          <span>{stats.pendingPermissions} {t('chat.agentStats.pending')}</span>
        </div>
      )}

      {/* Active tool calls */}
      {stats.activeToolCalls > 0 && (
        <div className="flex items-center gap-1">
          <Wrench size={12} />
          <span>{stats.activeToolCalls} {t('chat.agentStats.tool')}</span>
        </div>
      )}

      {/* Sessions */}
      <div className="text-blue-500/50">
        {stats.activeSessions} {t('chat.agentStats.session')}
      </div>

      {/* Pause/Resume button */}
      <button
        onClick={() => paused ? resumeAgent(activeConversationId!) : pauseAgent(activeConversationId!)}
        className="ml-auto flex items-center gap-1 px-2 py-0.5 rounded border border-blue-300 dark:border-blue-700 hover:bg-blue-100 dark:hover:bg-blue-800/30 transition-colors"
      >
        {paused ? <Play size={12} /> : <Pause size={12} />}
        <span>{paused ? t('chat.agentStats.resume') : t('chat.agentStats.pause')}</span>
      </button>
    </div>
  );
};

export default AgentStatsPanel;

import { invoke } from "@/lib/invoke";
import { useAgentStore, useConversationStore, useStreamStore } from "@/stores";
import { Activity, Clock, HelpCircle, Pause, Play, Shield, Wrench } from "lucide-react";
import React, { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { DreamStatusIndicator } from "./DreamStatusIndicator";

interface RuntimeStats {
  conversationId: string;
  paused: boolean;
  activeSessions: number;
  pendingPermissions: number;
  pendingAskUser: number;
  activeToolCalls: number;
}

export const AgentStatsPanel: React.FC = () => {
  const { t } = useTranslation();
  const [stats, setStats] = useState<RuntimeStats | null>(null);
  const [elapsed, setElapsed] = useState(0);
  const activeConversationId = useConversationStore(
    (s) => s.activeConversationId,
  );
  const activeStreams = useStreamStore((s) => s.activeStreams);
  const streaming = activeConversationId
    ? activeConversationId in activeStreams
    : false;
  const streamingMessageId = useStreamStore((s) => s.streamingMessageId);
  const pauseAgent = useAgentStore((s) => s.pauseAgent);
  const resumeAgent = useAgentStore((s) => s.resumeAgent);
  const isPaused = useAgentStore((s) => s.isAgentPaused);
  const currentQueryStats = useAgentStore((s) =>
    streamingMessageId ? (s.queryStats[streamingMessageId] ?? null) : null
  );

  const startTimeRef = useRef(0);
  const pausedDurationRef = useRef(0);
  const pauseStartRef = useRef(0);

  useEffect(() => {
    if (!streaming || !activeConversationId) {
      setStats(null);
      setElapsed(0);
      startTimeRef.current = 0;
      pausedDurationRef.current = 0;
      pauseStartRef.current = 0;
      return;
    }

    let cancelled = false;
    startTimeRef.current = Date.now();
    pausedDurationRef.current = 0;
    pauseStartRef.current = 0;
    setElapsed(0);

    const interval = setInterval(async () => {
      if (cancelled) {
        return;
      }
      try {
        const s = await invoke<RuntimeStats>("agent_runtime_stats", {
          conversationId: activeConversationId,
        });
        if (cancelled) {
          return;
        }
        setStats(s);

        if (s.paused && pauseStartRef.current === 0) {
          pauseStartRef.current = Date.now();
        } else if (!s.paused && pauseStartRef.current > 0) {
          pausedDurationRef.current += Date.now() - pauseStartRef.current;
          pauseStartRef.current = 0;
        }

        if (!s.paused) {
          setElapsed(
            Math.floor(
              (Date.now() - startTimeRef.current - pausedDurationRef.current)
                / 1000,
            ),
          );
        }
      } catch (e) {
        console.warn("[IPC] agent_runtime_stats poll error:", e);
      }
    }, 2000);

    invoke<RuntimeStats>("agent_runtime_stats", {
      conversationId: activeConversationId,
    })
      .then((s) => {
        if (!cancelled) {
          setStats(s);
        }
      })
      .catch((e: unknown) => {
        console.warn("[IPC]", e);
      });

    return () => {
      cancelled = true;
      clearInterval(interval);
    };
  }, [streaming, activeConversationId]);

  const paused = activeConversationId ? isPaused(activeConversationId) : false;

  const formatElapsed = (secs: number) => {
    const m = Math.floor(secs / 60);
    const s = secs % 60;
    return m > 0 ? `${m}m ${s}s` : `${s}s`;
  };

  const formatCost = (cost?: number) => {
    if (cost === undefined || cost === null) {
      return "--";
    }
    if (cost < 1.0) {
      return "<$1.0";
    }
    return `$${cost.toFixed(3)}`;
  };

  const handlePauseResume = () => {
    if (!activeConversationId) {
      return;
    }
    if (paused) {
      resumeAgent(activeConversationId);
    } else {
      pauseAgent(activeConversationId);
    }
  };

  return (
    <div
      data-testid="agent-stats-panel"
      className="flex items-center gap-3 px-3 py-1.5 bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 rounded-lg text-xs text-blue-700 dark:text-blue-300"
    >
      {streaming && !stats
        ? (
          <div className="flex items-center gap-2">
            <Activity size={12} className="animate-pulse text-blue-500" />
            <span>{t("chat.agentStats.loading")}</span>
            <div className="flex-1" />
          </div>
        )
        : streaming && stats
        ? (
          <>
            <div className="flex items-center gap-1">
              {paused
                ? <Pause size={12} className="text-orange-500" />
                : <Activity size={12} className="animate-pulse text-blue-500" />}
              <span className="font-medium">
                {paused
                  ? t("chat.agentStats.paused")
                  : t("chat.agentStats.running")}
              </span>
            </div>

            <div className="flex items-center gap-1">
              <Clock size={12} />
              <span>{formatElapsed(elapsed)}</span>
            </div>

            {currentQueryStats && (
              <div className="flex items-center gap-1">
                <span>
                  {(currentQueryStats.inputTokens || 0)
                    + (currentQueryStats.outputTokens || 0)} {t("chat.agentStats.tokens")}
                </span>
                <span className="text-blue-500/70">
                  ({formatCost(currentQueryStats.costUsd)})
                </span>
              </div>
            )}

            {stats.pendingPermissions > 0 && (
              <div className="flex items-center gap-1 text-orange-600 dark:text-orange-400">
                <Shield size={12} />
                <span>
                  {stats.pendingPermissions} {t("chat.agentStats.pending")}
                </span>
              </div>
            )}

            {stats.pendingAskUser > 0 && (
              <div className="flex items-center gap-1 text-orange-600 dark:text-orange-400">
                <HelpCircle size={12} />
                <span>
                  {stats.pendingAskUser} {t("chat.agentStats.askUser")}
                </span>
              </div>
            )}

            {stats.activeToolCalls > 0 && (
              <div className="flex items-center gap-1">
                <Wrench size={12} />
                <span>
                  {stats.activeToolCalls} {t("chat.agentStats.tool")}
                </span>
              </div>
            )}

            <div className="text-blue-500/50">
              {stats.activeSessions} {t("chat.agentStats.session")}
            </div>

            <button
              onClick={handlePauseResume}
              className="ml-auto flex items-center gap-1 px-2 py-0.5 rounded border border-blue-300 dark:border-blue-700 hover:bg-blue-100 dark:hover:bg-blue-800/30 transition-colors"
            >
              {paused ? <Play size={12} /> : <Pause size={12} />}
              <span>
                {paused
                  ? t("chat.agentStats.resume")
                  : t("chat.agentStats.pause")}
              </span>
            </button>
          </>
        )
        : <div className="flex-1" />}

      <DreamStatusIndicator />
    </div>
  );
};

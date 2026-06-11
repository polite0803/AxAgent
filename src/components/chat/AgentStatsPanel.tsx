// SPDX-License-Identifier: AGPL-3.0-only

import { invoke, logIpcError } from "@/lib/invoke";
import { useAgentStore, useConversationStore, useStreamStore } from "@/stores";
import { Activity, AlertTriangle, Clock, HelpCircle, IterationCw, Pause, Play, Shield, Wrench } from "lucide-react";
import React, { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { DreamStatusIndicator } from "./DreamStatusIndicator";

interface ToolExecRecord {
  toolName: string;
  startedAt: number;
  completedAt: number | null;
  isError: boolean;
  outputSummary?: string | null;
  inputSummary?: string | null;
}

interface ExecutionProgress {
  running: boolean;
  phase: string;
  currentIteration: number;
  maxIterations: number;
  currentTool: string | null;
  currentToolStartedAt: number | null;
  executedToolCount: number;
  failedToolCount: number;
  recentTools: ToolExecRecord[];
  lastError: string | null;
  statusMessage: string;
}

interface RuntimeStats {
  conversationId: string;
  running: boolean;
  paused: boolean;
  activeSessions: number;
  pendingPermissions: number;
  pendingAskUser: number;
  activeToolCalls: number;
  executionProgress: ExecutionProgress | null;
}

export const AgentStatsPanel: React.FC = () => {
  const { t } = useTranslation();
  const [stats, setStats] = useState<RuntimeStats | null>(null);
  const [elapsed, setElapsed] = useState(0);
  const [toolElapsed, setToolElapsed] = useState(0);
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
  const prog = stats?.executionProgress ?? null;

  const startTimeRef = useRef(0);
  const pausedDurationRef = useRef(0);
  const pauseStartRef = useRef(0);

  useEffect(() => {
    if (!streaming || !activeConversationId) {
      // eslint-disable-next-line react-hooks/set-state-in-effect
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
        logIpcError("agent_runtime_stats")(e);
      }
    }, 2000);

    // Track per-tool elapsed time
    const toolInterval = setInterval(() => {
      setStats((prev) => {
        const p = prev?.executionProgress;
        if (p?.currentTool && p.currentToolStartedAt) {
          setToolElapsed(
            Math.floor((Date.now() - p.currentToolStartedAt) / 1000),
          );
        } else {
          setToolElapsed(0);
        }
        return prev;
      });
    }, 1000);

    invoke<RuntimeStats>("agent_runtime_stats", {
      conversationId: activeConversationId,
    })
      .then((s) => {
        if (!cancelled) {
          setStats(s);
        }
      })
      .catch(logIpcError("agent_session_stats"));

    return () => {
      cancelled = true;
      clearInterval(interval);
      clearInterval(toolInterval);
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

            {prog?.currentTool && (
              <div className="flex items-center gap-1 font-medium">
                <Wrench size={12} className="animate-spin" />
                <span title={prog.statusMessage}>
                  {prog.currentTool}
                </span>
                {toolElapsed > 0 && (
                  <span className="text-blue-500/70">
                    ({formatElapsed(toolElapsed)})
                  </span>
                )}
              </div>
            )}

            {prog && prog.currentIteration > 0 && (
              <div className="flex items-center gap-1">
                <IterationCw size={12} />
                <span title={t("chat.agentStats.iteration")}>
                  {prog.currentIteration}/{prog.maxIterations}
                </span>
              </div>
            )}

            {prog && prog.executedToolCount > 0 && (
              <div className="flex items-center gap-1">
                <span
                  title={prog.recentTools
                    .slice(-3)
                    .map((rt) => `${rt.toolName} ${rt.isError ? "✗" : "✓"}`)
                    .join(" | ")}
                >
                  {prog.executedToolCount} {t("chat.agentStats.tool")}
                  {prog.failedToolCount > 0
                    ? ` (${prog.failedToolCount} ${t("chat.agentStats.failed")})`
                    : ""}
                </span>
              </div>
            )}

            {prog?.lastError && (
              <div className="flex items-center gap-1 text-red-500" title={prog.lastError}>
                <AlertTriangle size={12} />
                <span className="truncate max-w-[120px]">{prog.lastError}</span>
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

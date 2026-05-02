import { invoke } from "@/lib/invoke";
import { Clock, Cpu, GitBranch, Hash, MemoryStick, Wifi, WifiOff } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

export interface StatusBarInfo {
  gitBranch?: string;
  gitStatus?: string;
  sessionDuration?: number;
  tokenCount?: number;
  inputTokens?: number;
  outputTokens?: number;
  cpuUsage?: number;
  memoryUsage?: number;
  networkStatus?: "connected" | "disconnected";
  activeSessions?: number;
}

export interface StatusBarWidgetProps {
  sessionId?: string;
  refreshInterval?: number;
  showGit?: boolean;
  showTimer?: boolean;
  showTokens?: boolean;
  showSystem?: boolean;
  style?: React.CSSProperties;
  className?: string;
}

function formatDuration(seconds: number): string {
  const hours = Math.floor(seconds / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  const secs = seconds % 60;

  if (hours > 0) {
    return `${hours}:${minutes.toString().padStart(2, "0")}:${secs.toString().padStart(2, "0")}`;
  }
  return `${minutes}:${secs.toString().padStart(2, "0")}`;
}

function formatTokens(tokens: number): string {
  if (tokens >= 1000000) {
    return `${(tokens / 1000000).toFixed(1)}M`;
  }
  if (tokens >= 1000) {
    return `${(tokens / 1000).toFixed(1)}K`;
  }
  return tokens.toString();
}

export function StatusBarWidget({
  sessionId,
  refreshInterval = 1000,
  showGit = true,
  showTimer = true,
  showTokens = true,
  showSystem = true,
  style,
  className,
}: StatusBarWidgetProps) {
  const { t } = useTranslation();
  const [status, setStatus] = useState<StatusBarInfo>({});
  const [elapsedTime, setElapsedTime] = useState(0);
  const [startTime] = useState(Date.now());

  useEffect(() => {
    if (!showTimer) { return; }

    const interval = setInterval(() => {
      setElapsedTime(Math.floor((Date.now() - startTime) / 1000));
    }, refreshInterval);

    return () => clearInterval(interval);
  }, [showTimer, startTime, refreshInterval]);

  useEffect(() => {
    const fetchStatus = async () => {
      try {
        const [gitBranch, sessionStatus, systemInfo] = await Promise.all([
          showGit ? invoke<string | null>("git_get_branch").catch(() => null) : Promise.resolve(null),
          sessionId
            ? invoke<StatusBarInfo>("session_get_status", { sessionId }).catch(() => ({}))
            : Promise.resolve({}),
          showSystem ? invoke<StatusBarInfo>("system_get_info").catch(() => ({})) : Promise.resolve({}),
        ]);

        setStatus((prev) => ({
          ...prev,
          gitBranch: gitBranch || undefined,
          ...sessionStatus,
          ...systemInfo,
        }));
      } catch (e) {
        console.error("Failed to fetch status:", e);
      }
    };

    fetchStatus();
    const interval = setInterval(fetchStatus, refreshInterval * 5);
    return () => clearInterval(interval);
  }, [sessionId, showGit, showSystem, refreshInterval]);

  const statusItems: React.ReactNode[] = [];

  if (showGit && status.gitBranch) {
    statusItems.push(
      <StatusBarItem
        key="git"
        icon={<GitBranch size={12} />}
        label={status.gitBranch}
        color="#a6e3a1"
      />,
    );
  }

  if (showTimer) {
    statusItems.push(
      <StatusBarItem
        key="timer"
        icon={<Clock size={12} />}
        label={formatDuration(elapsedTime)}
        color="#f9e2af"
      />,
    );
  }

  if (showTokens) {
    if (status.tokenCount !== undefined) {
      statusItems.push(
        <StatusBarItem
          key="tokens"
          icon={<Hash size={12} />}
          label={formatTokens(status.tokenCount)}
          color="#89b4fa"
        />,
      );
    } else if (status.inputTokens !== undefined || status.outputTokens !== undefined) {
      const inTokens = status.inputTokens || 0;
      const outTokens = status.outputTokens || 0;
      statusItems.push(
        <StatusBarItem
          key="tokens"
          icon={<Hash size={12} />}
          label={`${formatTokens(inTokens)} / ${formatTokens(outTokens)}`}
          color="#89b4fa"
        />,
      );
    }
  }

  if (showSystem) {
    if (status.cpuUsage !== undefined) {
      statusItems.push(
        <StatusBarItem
          key="cpu"
          icon={<Cpu size={12} />}
          label={`${status.cpuUsage.toFixed(0)}%`}
          color={status.cpuUsage > 80 ? "#f38ba8" : "#94e2d5"}
        />,
      );
    }

    if (status.memoryUsage !== undefined) {
      statusItems.push(
        <StatusBarItem
          key="memory"
          icon={<MemoryStick size={12} />}
          label={`${status.memoryUsage.toFixed(0)}%`}
          color={status.memoryUsage > 80 ? "#f38ba8" : "#94e2d5"}
        />,
      );
    }

    if (status.networkStatus) {
      statusItems.push(
        <StatusBarItem
          key="network"
          icon={status.networkStatus === "connected" ? <Wifi size={12} /> : <WifiOff size={12} />}
          label={status.networkStatus === "connected" ? t("terminal.online") : t("terminal.offline")}
          color={status.networkStatus === "connected" ? "#a6e3a1" : "#f38ba8"}
        />,
      );
    }
  }

  if (status.activeSessions !== undefined && status.activeSessions > 0) {
    statusItems.push(
      <StatusBarItem
        key="sessions"
        icon={<span style={{ fontSize: 10, fontWeight: "bold" }}>#</span>}
        label={`${status.activeSessions}`}
        color="#f5c2e7"
      />,
    );
  }

  if (statusItems.length === 0) {
    return null;
  }

  return (
    <div
      className={className}
      style={{
        display: "flex",
        alignItems: "center",
        gap: 16,
        padding: "4px 12px",
        background: "#181825",
        borderTop: "1px solid #333",
        fontSize: 11,
        fontFamily: "'JetBrains Mono', monospace",
        ...style,
      }}
    >
      {statusItems}
    </div>
  );
}

interface StatusBarItemProps {
  icon: React.ReactNode;
  label: string;
  color?: string;
  tooltip?: string;
}

export function StatusBarItem({ icon, label, color, tooltip }: StatusBarItemProps) {
  return (
    <div
      style={{
        display: "flex",
        alignItems: "center",
        gap: 4,
        color: color || "#cdd6f4",
        cursor: tooltip ? "help" : "default",
      }}
      title={tooltip}
    >
      {icon}
      <span>{label}</span>
    </div>
  );
}

export interface GitStatusInfo {
  branch: string;
  ahead: number;
  behind: number;
  dirty: boolean;
  staged: number;
  conflicted: number;
}

export function useGitStatus() {
  const [gitStatus, setGitStatus] = useState<GitStatusInfo | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const fetchGitStatus = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const status = await invoke<GitStatusInfo>("git_status");
      setGitStatus(status);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to get git status");
      setGitStatus(null);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchGitStatus();
  }, [fetchGitStatus]);

  return { gitStatus, loading, error, refetch: fetchGitStatus };
}

export function useSessionTimer() {
  const [elapsed, setElapsed] = useState(0);
  const [startTime, setStartTime] = useState<number | null>(null);
  const [isRunning, setIsRunning] = useState(false);

  const start = useCallback(() => {
    if (!isRunning) {
      setStartTime(Date.now() - elapsed * 1000);
      setIsRunning(true);
    }
  }, [isRunning, elapsed]);

  const pause = useCallback(() => {
    setIsRunning(false);
  }, []);

  const reset = useCallback(() => {
    setElapsed(0);
    setStartTime(null);
    setIsRunning(false);
  }, []);

  useEffect(() => {
    if (!isRunning || !startTime) { return; }

    const interval = setInterval(() => {
      setElapsed(Math.floor((Date.now() - startTime) / 1000));
    }, 1000);

    return () => clearInterval(interval);
  }, [isRunning, startTime]);

  return { elapsed, isRunning, start, pause, reset, formatted: formatDuration(elapsed) };
}

import { type PtySessionInfo, useTerminalStore } from "@/stores/feature/terminalStore";
import { Badge, Button, Empty, Select, Tooltip } from "antd";
import { AlertTriangle, CheckCircle, Maximize2, Minimize2, Plus, RefreshCw, Terminal, Trash2, X } from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";

interface IntegratedTerminalProps {
  defaultCwd?: string;
  defaultShell?: string;
  height?: number;
  onOutput?: (sessionId: string, data: string) => void;
  onError?: (sessionId: string, errors: any[]) => void;
}

export function IntegratedTerminal({
  defaultCwd,
  defaultShell,
  height = 400,
  onOutput,
  onError,
}: IntegratedTerminalProps) {
  const {
    sessions,
    activeSessionId,
    outputBuffers,
    analysis,
    loading,
    error,
    createSession,
    killSession,
    removeSession,
    setActiveSession,
    writeToSession,
    resizeSession,
    clearOutput,
    analyzeOutput,
    clearError,
  } = useTerminalStore();

  const terminalRef = useRef<HTMLDivElement>(null);
  const xtermRef = useRef<any>(null);
  const fitAddonRef = useRef<any>(null);
  const [isMaximized, setIsMaximized] = useState(false);
  const [terminalReady, setTerminalReady] = useState(false);

  const activeSession = sessions.find((s) => s.id === activeSessionId);
  const activeOutput = activeSessionId ? outputBuffers[activeSessionId] ?? [] : [];
  const activeAnalysis = activeSessionId ? analysis[activeSessionId] : undefined;

  const initTerminal = useCallback(async () => {
    if (!terminalRef.current) { return; }

    try {
      const { Terminal: XTerm } = await import("@xterm/xterm");
      const { FitAddon } = await import("@xterm/addon-fit");
      const { WebLinksAddon } = await import("@xterm/addon-web-links");

      await import("@xterm/xterm/css/xterm.css");

      const xterm = new XTerm({
        cursorBlink: true,
        fontSize: 14,
        fontFamily: "'JetBrains Mono', 'Fira Code', 'Cascadia Code', monospace",
        theme: {
          background: "#1e1e2e",
          foreground: "#cdd6f4",
          cursor: "#f5e0dc",
          selectionBackground: "#585b7066",
          black: "#45475a",
          red: "#f38ba8",
          green: "#a6e3a1",
          yellow: "#f9e2af",
          blue: "#89b4fa",
          magenta: "#f5c2e7",
          cyan: "#94e2d5",
          white: "#bac2de",
        },
      });

      const fitAddon = new FitAddon();
      xterm.loadAddon(fitAddon);
      xterm.loadAddon(new WebLinksAddon());

      xterm.open(terminalRef.current);
      fitAddon.fit();

      xtermRef.current = xterm;
      fitAddonRef.current = fitAddon;
      setTerminalReady(true);

      xterm.onData((data: string) => {
        if (activeSessionId) {
          writeToSession(activeSessionId, data);
        }
      });

      xterm.onResize(({ cols, rows }: { cols: number; rows: number }) => {
        if (activeSessionId) {
          resizeSession(activeSessionId, rows, cols);
        }
      });
    } catch (e) {
      console.error("Failed to initialize xterm:", e);
    }
  }, [activeSessionId, resizeSession, writeToSession]);

  useEffect(() => {
    initTerminal();

    return () => {
      if (xtermRef.current) {
        xtermRef.current.dispose();
        xtermRef.current = null;
        fitAddonRef.current = null;
        setTerminalReady(false);
      }
    };
  }, []);

  useEffect(() => {
    if (!terminalReady || !xtermRef.current) { return; }

    const xterm = xtermRef.current;
    const lastLine = activeOutput[activeOutput.length - 1] ?? "";
    if (lastLine) {
      xterm.write(lastLine + "\r\n");
      if (onOutput && activeSessionId) {
        onOutput(activeSessionId, lastLine);
      }
    }
  }, [activeOutput, terminalReady, activeSessionId, onOutput]);

  useEffect(() => {
    if (activeAnalysis?.has_errors && onError && activeSessionId) {
      onError(activeSessionId, activeAnalysis.errors);
    }
  }, [activeAnalysis, activeSessionId, onError]);

  useEffect(() => {
    const handleResize = () => {
      if (fitAddonRef.current) {
        fitAddonRef.current.fit();
      }
    };

    window.addEventListener("resize", handleResize);
    const observer = new ResizeObserver(handleResize);
    if (terminalRef.current) {
      observer.observe(terminalRef.current);
    }

    return () => {
      window.removeEventListener("resize", handleResize);
      observer.disconnect();
    };
  }, []);

  const handleCreateSession = async () => {
    try {
      await createSession({
        shell: defaultShell,
        cwd: defaultCwd,
      });
    } catch (e) {
      console.error("Failed to create terminal session:", e);
    }
  };

  const handleKillSession = async () => {
    if (!activeSessionId) { return; }
    await killSession(activeSessionId);
  };

  const handleRemoveSession = async () => {
    if (!activeSessionId) { return; }
    await removeSession(activeSessionId);
  };

  const handleAnalyze = async () => {
    if (!activeSessionId) { return; }
    try {
      await analyzeOutput(activeSessionId);
    } catch (e) {
      console.error("Failed to analyze output:", e);
    }
  };

  const handleClear = () => {
    if (!activeSessionId) { return; }
    clearOutput(activeSessionId);
    if (xtermRef.current) {
      xtermRef.current.clear();
    }
  };

  const toggleMaximize = () => {
    setIsMaximized(!isMaximized);
    setTimeout(() => {
      if (fitAddonRef.current) {
        fitAddonRef.current.fit();
      }
    }, 100);
  };

  const containerHeight = isMaximized ? "calc(100vh - 48px)" : height;

  return (
    <div
      style={{
        display: "flex",
        flexDirection: "column",
        height: containerHeight,
        border: "1px solid #333",
        borderRadius: 8,
        overflow: "hidden",
        background: "#1e1e2e",
      }}
    >
      <div
        style={{
          display: "flex",
          alignItems: "center",
          padding: "4px 12px",
          background: "#181825",
          borderBottom: "1px solid #333",
          gap: 8,
          flexShrink: 0,
        }}
      >
        <Terminal size={16} color="#89b4fa" />
        <span style={{ color: "#cdd6f4", fontSize: 13, fontWeight: 500 }}>
          Terminal
        </span>

        {sessions.length > 0 && (
          <Select
            value={activeSessionId ?? undefined}
            onChange={setActiveSession}
            size="small"
            style={{ minWidth: 120, flex: 1 }}
            options={sessions.map((s) => ({
              value: s.id,
              label: (
                <span style={{ display: "flex", alignItems: "center", gap: 6 }}>
                  <SessionStatusBadge status={s.status} />
                  {s.id.slice(0, 8)}
                </span>
              ),
            }))}
          />
        )}

        <div style={{ display: "flex", gap: 4, marginLeft: "auto" }}>
          <Tooltip title="New Terminal">
            <Button
              size="small"
              type="text"
              icon={<Plus size={14} />}
              onClick={handleCreateSession}
              loading={loading}
              style={{ color: "#a6e3a1" }}
            />
          </Tooltip>

          {activeSessionId && (
            <>
              <Tooltip title="Analyze Output">
                <Button
                  size="small"
                  type="text"
                  icon={<AlertTriangle size={14} />}
                  onClick={handleAnalyze}
                  style={{ color: "#f9e2af" }}
                />
              </Tooltip>
              <Tooltip title="Clear">
                <Button
                  size="small"
                  type="text"
                  icon={<RefreshCw size={14} />}
                  onClick={handleClear}
                  style={{ color: "#89b4fa" }}
                />
              </Tooltip>
              <Tooltip title="Kill Process">
                <Button
                  size="small"
                  type="text"
                  icon={<X size={14} />}
                  onClick={handleKillSession}
                  style={{ color: "#f38ba8" }}
                />
              </Tooltip>
              <Tooltip title="Close Session">
                <Button
                  size="small"
                  type="text"
                  icon={<Trash2 size={14} />}
                  onClick={handleRemoveSession}
                  style={{ color: "#f38ba8" }}
                />
              </Tooltip>
            </>
          )}

          <Tooltip title={isMaximized ? "Restore" : "Maximize"}>
            <Button
              size="small"
              type="text"
              icon={isMaximized ? <Minimize2 size={14} /> : <Maximize2 size={14} />}
              onClick={toggleMaximize}
              style={{ color: "#cdd6f4" }}
            />
          </Tooltip>
        </div>
      </div>

      {error && (
        <div
          style={{
            padding: "4px 12px",
            background: "#f38ba822",
            color: "#f38ba8",
            fontSize: 12,
            display: "flex",
            alignItems: "center",
            gap: 8,
          }}
        >
          <AlertTriangle size={12} />
          {error}
          <Button
            size="small"
            type="text"
            onClick={clearError}
            style={{ color: "#f38ba8", marginLeft: "auto", padding: "0 4px" }}
          >
            Dismiss
          </Button>
        </div>
      )}

      {activeAnalysis?.has_errors && (
        <div
          style={{
            padding: "4px 12px",
            background: "#f9e2af22",
            color: "#f9e2af",
            fontSize: 12,
            display: "flex",
            alignItems: "center",
            gap: 8,
          }}
        >
          <AlertTriangle size={12} />
          {activeAnalysis.summary}
        </div>
      )}

      <div style={{ flex: 1, position: "relative" }}>
        {sessions.length === 0
          ? (
            <div
              style={{
                display: "flex",
                flexDirection: "column",
                alignItems: "center",
                justifyContent: "center",
                height: "100%",
                gap: 12,
              }}
            >
              <Empty
                image={Empty.PRESENTED_IMAGE_SIMPLE}
                description={
                  <span style={{ color: "#6c7086" }}>
                    No terminal sessions
                  </span>
                }
              />
              <Button
                size="small"
                icon={<Plus size={14} />}
                onClick={handleCreateSession}
                loading={loading}
                style={{ background: "#313244", borderColor: "#45475a", color: "#cdd6f4" }}
              >
                New Terminal
              </Button>
            </div>
          )
          : (
            <div
              ref={terminalRef}
              style={{
                width: "100%",
                height: "100%",
                padding: "4px 8px",
              }}
            />
          )}
      </div>

      {activeSession && (
        <div
          style={{
            display: "flex",
            alignItems: "center",
            padding: "2px 12px",
            background: "#181825",
            borderTop: "1px solid #333",
            gap: 12,
            fontSize: 11,
            color: "#6c7086",
            flexShrink: 0,
          }}
        >
          <SessionStatusBadge status={activeSession.status} />
          <span>
            {activeSession.rows}×{activeSession.cols}
          </span>
          {activeSession.cwd && (
            <span style={{ overflow: "hidden", textOverflow: "ellipsis" }}>
              {activeSession.cwd}
            </span>
          )}
          {activeAnalysis && (
            <span style={{ marginLeft: "auto" }}>
              {activeAnalysis.has_errors
                ? (
                  <span style={{ color: "#f38ba8" }}>
                    <AlertTriangle size={10} style={{ marginRight: 4 }} />
                    {activeAnalysis.errors.length} error(s)
                  </span>
                )
                : (
                  <span style={{ color: "#a6e3a1" }}>
                    <CheckCircle size={10} style={{ marginRight: 4 }} />
                    OK
                  </span>
                )}
            </span>
          )}
        </div>
      )}
    </div>
  );
}

function SessionStatusBadge({ status }: { status: PtySessionInfo["status"] }) {
  const colorMap: Record<string, string> = {
    starting: "#f9e2af",
    running: "#a6e3a1",
    exited: "#6c7086",
    error: "#f38ba8",
  };

  return (
    <Badge
      color={colorMap[status] ?? "#6c7086"}
      style={{ marginRight: 4 }}
    />
  );
}

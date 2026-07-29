// SPDX-License-Identifier: AGPL-3.0-only

import { Tooltip } from "@/components/layout/Tooltip";
import { isTauri, listen, logIpcError, type UnlistenFn } from "@/lib/invoke";
import { type PtySessionInfo, useTerminalStore } from "@/stores/feature/terminalStore";
import { Badge, Button, Empty, Select } from "antd";
import { AlertTriangle, CheckCircle, Maximize2, Minimize2, Plus, RefreshCw, Terminal, Trash2, X } from "lucide-react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";

interface IntegratedTerminalProps {
  defaultCwd?: string;
  defaultShell?: string;
  height?: number;
  onOutput?: (sessionId: string, data: string) => void;
  onError?: (sessionId: string, errors: unknown[]) => void;
}

export function IntegratedTerminal({
  defaultCwd,
  defaultShell,
  onOutput,
  onError,
}: IntegratedTerminalProps) {
  const { t } = useTranslation();
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
    appendOutput,
  } = useTerminalStore();

  const terminalRef = useRef<HTMLDivElement>(null);
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const xtermRef = useRef<any>(null);
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const fitAddonRef = useRef<any>(null);
  const [isMaximized, setIsMaximized] = useState(false);
  const terminalReadyRef = useRef(false);

  // 使用 ref 避免 onData/onResize 回调捕获过期状态
  const activeSessionIdRef = useRef(activeSessionId);
  const writeToSessionRef = useRef(writeToSession);
  const resizeSessionRef = useRef(resizeSession);

  useEffect(() => {
    activeSessionIdRef.current = activeSessionId;
    writeToSessionRef.current = writeToSession;
    resizeSessionRef.current = resizeSession;
  }, [activeSessionId, writeToSession, resizeSession]);

  const activeSession = sessions.find((s) => s.id === activeSessionId);
  const activeOutput = useMemo(
    () => (activeSessionId ? (outputBuffers[activeSessionId] ?? []) : []),
    [activeSessionId, outputBuffers],
  );
  const activeAnalysis = useMemo(
    () => (activeSessionId ? analysis[activeSessionId] : undefined),
    [activeSessionId, analysis],
  );

  const initTerminal = useCallback(async () => {
    if (!terminalRef.current) {
      return;
    }

    try {
      const [{ Terminal: XTerm }, { FitAddon }, { WebLinksAddon }] = await Promise.all([
        import("@xterm/xterm"),
        import("@xterm/addon-fit"),
        import("@xterm/addon-web-links"),
      ]);

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
      terminalReadyRef.current = true;

      // 写入初始化前的待处理输出
      const lastLine = activeOutput[activeOutput.length - 1] ?? "";
      if (lastLine) {
        xterm.write(lastLine + "\r\n");
        if (onOutput && activeSessionId) {
          onOutput(activeSessionId, lastLine);
        }
      }

      // 通过 ref 获取最新值，避免闭包过期
      xterm.onData((data: string) => {
        const sessionId = activeSessionIdRef.current;
        if (sessionId) {
          writeToSessionRef.current(sessionId, data);
        }
      });

      xterm.onResize(({ cols, rows }: { cols: number; rows: number }) => {
        const sessionId = activeSessionIdRef.current;
        if (sessionId) {
          resizeSessionRef.current(sessionId, rows, cols);
        }
      });
    } catch (e) {
      logIpcError("initialize xterm")(e);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    initTerminal();

    // P0-1: 监听后端 PTY 输出事件，实时写入 xterm
    // 在浏览器 mock 模式下 listen() 会回退到内存事件总线，确保 e2e 可用
    let unlistenOutput: UnlistenFn | null = null;
    let unlistenExit: UnlistenFn | null = null;
    (async () => {
      try {
        unlistenOutput = await listen<{
          sessionId: string;
          data: string;
          timestamp: number;
        }>("pty_output", (event) => {
          const payload = event.payload;
          if (!payload) { return; }
          const { sessionId, data } = payload;
          // 更新 store 缓冲区
          appendOutput(sessionId, data);
          // 直接写入 xterm，避免走 activeOutput useMemo 再触发 useEffect 的二次写入
          if (
            xtermRef.current
            && terminalReadyRef.current
            && sessionId === activeSessionIdRef.current
          ) {
            xtermRef.current.write(data);
          }
        });
      } catch (e) {
        logIpcError("listen pty_output")(e);
      }

      try {
        unlistenExit = await listen<{
          sessionId: string;
          exitCode: number | null;
          timestamp: number;
        }>("pty_exit", (event) => {
          const payload = event.payload;
          if (!payload) { return; }
          const sessionId = payload.sessionId;
          // 更新会话状态为 exited
          useTerminalStore.setState((state) => ({
            sessions: state.sessions.map((s) => s.id === sessionId ? { ...s, status: "exited" as const } : s),
          }));
        });
      } catch (e) {
        logIpcError("listen pty_exit")(e);
      }
    })();

    return () => {
      // P0-2: 清理 xterm 资源
      if (xtermRef.current) {
        xtermRef.current.dispose();
        xtermRef.current = null;
        fitAddonRef.current = null;
        terminalReadyRef.current = false;
      }
      // P0-2: 取消事件订阅，避免重复写入已 dispose 的 xterm
      unlistenOutput?.();
      unlistenExit?.();
      // P0-2: 清理本组件创建的所有后端 PTY 会话，防止资源泄漏
      // 仅在 Tauri 真机环境下执行后端清理
      const snapshot = useTerminalStore.getState().sessions;
      if (isTauri() && snapshot.length > 0) {
        for (const s of snapshot) {
          // removeSession 会同步清空 store 并调用后端 pty_remove_session
          // 用 catch 吞掉错误避免一个失败影响其他清理
          useTerminalStore
            .getState()
            .removeSession(s.id)
            .catch((e) => logIpcError(`cleanup PTY session ${s.id}`)(e));
        }
      }
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    if (!terminalReadyRef.current || !xtermRef.current) {
      return;
    }

    const xterm = xtermRef.current;
    // 切换会话时重写整个缓冲区到 xterm（PTY 输出事件只对当前 activeSessionId 实时写入，
    // 切换到其他会话时需要把该会话已有的缓冲区内容回写到 xterm 显示）
    xterm.reset();
    if (activeOutput.length > 0) {
      xterm.write(activeOutput.join(""));
      if (onOutput && activeSessionId) {
        onOutput(activeSessionId, activeOutput[activeOutput.length - 1]);
      }
    }
  }, [activeOutput, activeSessionId, onOutput]);

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
    if (loading) {
      return;
    }
    try {
      await createSession({
        shell: defaultShell,
        cwd: defaultCwd,
      });
    } catch (e) {
      logIpcError("create terminal session")(e);
    }
  };

  const handleKillSession = async () => {
    if (!activeSessionId) {
      return;
    }
    await killSession(activeSessionId);
  };

  const handleRemoveSession = async () => {
    if (!activeSessionId) {
      return;
    }
    await removeSession(activeSessionId);
  };

  const handleAnalyze = async () => {
    if (!activeSessionId) {
      return;
    }
    try {
      await analyzeOutput(activeSessionId);
    } catch (e) {
      logIpcError("analyze terminal output")(e);
    }
  };

  const handleClear = () => {
    if (!activeSessionId) {
      return;
    }
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

  return (
    <div
      className={isMaximized ? "term-maximized" : undefined}
      style={{
        display: "flex",
        flexDirection: "column",
        flex: 1,
        width: "100%",
        border: "1px solid #333",
        borderRadius: isMaximized ? 0 : 8,
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
          <Tooltip title={t("terminal.newTerminal")}>
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
              <Tooltip title={t("terminal.analyzeOutput")}>
                <Button
                  size="small"
                  type="text"
                  icon={<AlertTriangle size={14} />}
                  onClick={handleAnalyze}
                  style={{ color: "#f9e2af" }}
                />
              </Tooltip>
              <Tooltip title={t("terminal.clear")}>
                <Button
                  size="small"
                  type="text"
                  icon={<RefreshCw size={14} />}
                  onClick={handleClear}
                  style={{ color: "#89b4fa" }}
                />
              </Tooltip>
              <Tooltip title={t("terminal.killProcess")}>
                <Button
                  size="small"
                  type="text"
                  icon={<X size={14} />}
                  onClick={handleKillSession}
                  style={{ color: "#f38ba8" }}
                />
              </Tooltip>
              <Tooltip title={t("terminal.closeSession")}>
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

          <Tooltip title={isMaximized ? t("terminal.restore") : t("terminal.maximize")}>
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
            {t("common.dismiss")}
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
                description={<span style={{ color: "#6c7086" }}>{t("terminal.noSessions")}</span>}
              />
              <Button
                size="small"
                icon={<Plus size={14} />}
                onClick={handleCreateSession}
                loading={loading}
                style={{
                  background: "#313244",
                  borderColor: "#45475a",
                  color: "#cdd6f4",
                }}
              >
                {t("terminal.newTerminal")}
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
            fontSize: 12,
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
                    {t("terminal.errorsCount", { count: activeAnalysis.errors.length })}
                  </span>
                )
                : (
                  <span style={{ color: "#a6e3a1" }}>
                    <CheckCircle size={10} style={{ marginRight: 4 }} />
                    {t("terminal.ok")}
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

  return <Badge color={colorMap[status] ?? "#6c7086"} style={{ marginRight: 4 }} />;
}

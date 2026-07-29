// SPDX-License-Identifier: AGPL-3.0-only
// Phase 4: WorkflowLogPanel — 工作流执行日志面板

import type { ExecutionLogEntry } from "@/types";
import { Button, Empty } from "antd";
import { useCallback, useEffect, useRef } from "react";
import { useTranslation } from "react-i18next";

interface WorkflowLogPanelProps {
  logs: ExecutionLogEntry[];
  onClear?: () => void;
  onExport?: () => void;
  maxHeight?: number;
}

const levelColors: Record<ExecutionLogEntry["level"], string> = {
  info: "#52c41a",
  warn: "#faad14",
  error: "#ff4d4f",
};

const levelLabels: Record<ExecutionLogEntry["level"], string> = {
  info: "INFO",
  warn: "WARN",
  error: "ERROR",
};

export function WorkflowLogPanel({ logs, onClear, onExport, maxHeight = 300 }: WorkflowLogPanelProps) {
  const { t } = useTranslation();
  const containerRef = useRef<HTMLDivElement>(null);

  const scrollToBottom = useCallback(() => {
    if (containerRef.current) {
      containerRef.current.scrollTop = containerRef.current.scrollHeight;
    }
  }, []);

  useEffect(() => {
    scrollToBottom();
  }, [logs, scrollToBottom]);

  if (logs.length === 0) {
    return <Empty description={t("workflow.log.empty")} image={Empty.PRESENTED_IMAGE_SIMPLE} />;
  }

  return (
    <div>
      {(onClear || onExport) && (
        <div style={{ marginBottom: 8, display: "flex", justifyContent: "flex-end", gap: 8 }}>
          {onClear && (
            <Button size="small" onClick={onClear}>
              {t("workflow.log.clear")}
            </Button>
          )}
          {onExport && (
            <Button size="small" onClick={onExport}>
              {t("workflow.log.export")}
            </Button>
          )}
        </div>
      )}
      <div
        ref={containerRef}
        style={{
          maxHeight,
          overflowY: "auto",
          backgroundColor: "#1e1e1e",
          borderRadius: 6,
          padding: 8,
          fontFamily: "var(--font-mono, 'JetBrains Mono', ui-monospace, monospace)",
          fontSize: 12,
          lineHeight: 1.6,
        }}
      >
        {logs.map((log, i) => (
          <div key={`${log.timestamp}-${i}`} style={{ color: "#d4d4d4", whiteSpace: "pre-wrap" }}>
            <span style={{ color: "#6a9955" }}>
              {new Date(log.timestamp).toLocaleTimeString()}
            </span>{" "}
            <span style={{ color: levelColors[log.level], fontWeight: 600 }}>
              [{levelLabels[log.level]}]
            </span>{" "}
            <span style={{ color: "#569cd6" }}>[{log.nodeName}]</span> <span>{log.message}</span>
          </div>
        ))}
        <div ref={containerRef} />
      </div>
    </div>
  );
}

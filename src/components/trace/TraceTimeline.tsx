// SPDX-License-Identifier: AGPL-3.0-only
/* eslint-disable react-hooks/set-state-in-effect */

import { useTracerStore } from "@/stores/devtools/tracerStore";
import type { Span } from "@/types";
import { Popover, Spin, Typography } from "antd";
import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

const { Text } = Typography;

// ── Types ──

export interface TraceStep {
  id: string;
  name: string;
  type: "thinking" | "tool_call" | "permission" | "error";
  description?: string;
  durationMs: number;
  tokenUsage?: number;
  startMs: number;
}

interface TraceTimelineProps {
  traceId: string;
}

// ── Colors by type ──

const TYPE_COLORS: Record<string, string> = {
  thinking: "#1890ff",
  tool_call: "#fa8c16",
  permission: "#fadb14",
  error: "#f5222d",
};

const TYPE_LABELS: Record<string, string> = {
  thinking: "思考",
  tool_call: "工具调用",
  permission: "等待权限",
  error: "错误",
};

// ── Helpers ──

function mapSpanTypeToStepType(spanType: string, status: string): TraceStep["type"] {
  if (status === "error") { return "error"; }
  switch (spanType) {
    case "agent":
    case "reasoning":
    case "reflection":
      return "thinking";
    case "tool":
    case "llm_call":
    case "task":
    case "sub_task":
      return "tool_call";
    default:
      return "thinking";
  }
}

function spansToSteps(spans: Span[]): TraceStep[] {
  if (spans.length === 0) { return []; }

  const baseTime = new Date(spans[0].start_time).getTime();

  return spans.map((span) => {
    const startMs = new Date(span.start_time).getTime() - baseTime;
    const durationMs = span.duration_ms ?? 0;
    const tokenUsage = (span.attributes as Record<string, unknown> | null)?.total_tokens as number | undefined;
    const description = span.errors.length > 0
      ? span.errors.map((e) => e.message).join("; ")
      : undefined;

    return {
      id: span.id,
      name: span.name,
      type: mapSpanTypeToStepType(span.span_type, span.status),
      description,
      durationMs,
      tokenUsage,
      startMs,
    };
  });
}

// ── Component ──

export function TraceTimeline({ traceId }: TraceTimelineProps) {
  const { t } = useTranslation();
  const loadTrace = useTracerStore((s) => s.loadTrace);
  const selectedTrace = useTracerStore((s) => s.selectedTrace);
  const isLoading = useTracerStore((s) => s.isLoading);
  const error = useTracerStore((s) => s.error);
  const [loaded, setLoaded] = useState(false);

  useEffect(() => {
    if (traceId) {
      setLoaded(false);
      loadTrace(traceId).then(() => setLoaded(true));
    }
  }, [traceId, loadTrace]);

  const steps = useMemo(() => {
    if (!selectedTrace?.trace?.spans) { return []; }
    return spansToSteps(selectedTrace.trace.spans);
  }, [selectedTrace]);

  // ── Loading / Error / Empty states ──

  if (!loaded || isLoading) {
    return (
      <div style={{ textAlign: "center", padding: 32 }}>
        <Spin />
        <Text type="secondary" style={{ display: "block", marginTop: 8 }}>
          {t("trace.loading", "加载执行轨迹...")}
        </Text>
      </div>
    );
  }

  if (error) {
    return (
      <div style={{ textAlign: "center", padding: 32 }}>
        <Text type="danger">{t("trace.error", "加载失败")}: {error}</Text>
      </div>
    );
  }

  if (steps.length === 0) {
    return (
      <div style={{ textAlign: "center", padding: 32 }}>
        <Text type="secondary">{t("trace.empty", "未找到执行轨迹数据")}</Text>
      </div>
    );
  }

  // ── Calculations ──

  const totalDuration = steps.length > 0
    ? steps[steps.length - 1].startMs + steps[steps.length - 1].durationMs - steps[0].startMs
    : 1;
  const totalTokens = steps.reduce((sum, s) => sum + (s.tokenUsage ?? 0), 0);
  const errorCount = steps.filter((s) => s.type === "error").length;

  return (
    <div>
      {/* Summary bar */}
      <div
        style={{
          display: "flex",
          gap: 32,
          padding: "8px 16px",
          background: "var(--ant-color-bg-container, #fff)",
          borderRadius: 8,
          border: "1px solid var(--ant-color-border-secondary, #f0f0f0)",
          marginBottom: 16,
        }}
      >
        <div>
          <Text type="secondary" style={{ fontSize: 12 }}>{t("trace.totalSteps", "总步骤")}</Text>
          <div>
            <Text strong>{steps.length}</Text>
          </div>
        </div>
        <div>
          <Text type="secondary" style={{ fontSize: 12 }}>{t("trace.totalDuration", "总耗时")}</Text>
          <div>
            <Text strong>{(totalDuration / 1000).toFixed(1)}s</Text>
          </div>
        </div>
        <div>
          <Text type="secondary" style={{ fontSize: 12 }}>{t("trace.totalTokens", "总 Token")}</Text>
          <div>
            <Text strong>{totalTokens.toLocaleString()}</Text>
          </div>
        </div>
        {errorCount > 0 && (
          <div>
            <Text type="secondary" style={{ fontSize: 12 }}>{t("trace.errors", "错误")}</Text>
            <div>
              <Text strong type="danger">{errorCount}</Text>
            </div>
          </div>
        )}
      </div>

      {/* Horizontal timeline */}
      <div
        style={{
          position: "relative",
          display: "flex",
          alignItems: "flex-start",
          gap: 4,
          padding: 12,
          background: "var(--ant-color-bg-container, #fff)",
          borderRadius: 8,
          border: "1px solid var(--ant-color-border-secondary, #f0f0f0)",
          overflow: "auto",
          minHeight: 80,
        }}
      >
        {steps.map((step) => {
          const widthPct = Math.max(2, (step.durationMs / totalDuration) * 100);
          const color = TYPE_COLORS[step.type] ?? "#888";

          return (
            <Popover
              key={step.id}
              title={
                <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                  <span
                    style={{
                      display: "inline-block",
                      width: 8,
                      height: 8,
                      borderRadius: "50%",
                      background: color,
                    }}
                  />
                  <Text strong>{step.name}</Text>
                  <Text type="secondary" style={{ fontSize: 11 }}>
                    {TYPE_LABELS[step.type] ?? step.type}
                  </Text>
                </div>
              }
              content={
                <div style={{ maxWidth: 300 }}>
                  {step.description && (
                    <Text style={{ fontSize: 12, display: "block", marginBottom: 4 }}>
                      {step.description}
                    </Text>
                  )}
                  <Text type="secondary" style={{ fontSize: 11 }}>
                    耗时: {(step.durationMs / 1000).toFixed(2)}s
                  </Text>
                  {step.tokenUsage !== undefined && (
                    <Text type="secondary" style={{ fontSize: 11, display: "block" }}>
                      Token: {step.tokenUsage}
                    </Text>
                  )}
                </div>
              }
            >
              <div
                style={{
                  width: `${widthPct}%`,
                  minWidth: 24,
                  height: 40,
                  background: color,
                  borderRadius: 4,
                  cursor: "pointer",
                  display: "flex",
                  alignItems: "center",
                  justifyContent: "center",
                  opacity: step.type === "error" ? 0.7 : 0.85,
                  transition: "opacity 0.2s",
                }}
                title={step.name}
              >
                <Text
                  style={{
                    color: "#fff",
                    fontSize: 10,
                    fontWeight: 500,
                    overflow: "hidden",
                    textOverflow: "ellipsis",
                    whiteSpace: "nowrap",
                    padding: "0 4px",
                  }}
                >
                  {step.name.length > 8 ? step.name.slice(0, 8) + "..." : step.name}
                </Text>
              </div>
            </Popover>
          );
        })}
      </div>

      {/* Legend */}
      <div style={{ display: "flex", gap: 16, marginTop: 12, flexWrap: "wrap" }}>
        {Object.entries(TYPE_COLORS).map(([type, color]) => (
          <div key={type} style={{ display: "flex", alignItems: "center", gap: 6 }}>
            <span
              style={{
                display: "inline-block",
                width: 12,
                height: 12,
                borderRadius: 2,
                background: color,
              }}
            />
            <Text style={{ fontSize: 12 }}>{TYPE_LABELS[type] ?? type}</Text>
          </div>
        ))}
      </div>
    </div>
  );
}

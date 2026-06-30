// SPDX-License-Identifier: AGPL-3.0-only

import { Popover, Typography } from "antd";
import { useMemo } from "react";
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

// ── Mock data ──

function buildMockSteps(): TraceStep[] {
  const steps: TraceStep[] = [
    {
      id: "s1",
      name: "理解用户意图",
      type: "thinking",
      description: "解析用户输入，识别意图和关键实体",
      durationMs: 1200,
      tokenUsage: 450,
      startMs: 0,
    },
    {
      id: "s2",
      name: "制定执行计划",
      type: "thinking",
      description: "生成分步执行计划，评估工具需求",
      durationMs: 800,
      tokenUsage: 320,
      startMs: 1200,
    },
    {
      id: "s3",
      name: "search_file",
      type: "tool_call",
      description: "搜索相关文档文件",
      durationMs: 1500,
      tokenUsage: 120,
      startMs: 2000,
    },
    {
      id: "s4",
      name: "等待用户授权",
      type: "permission",
      description: "需要用户确认文件删除操作",
      durationMs: 5000,
      startMs: 3500,
    },
    {
      id: "s5",
      name: "read_file",
      type: "tool_call",
      description: "读取目标文件内容",
      durationMs: 600,
      tokenUsage: 80,
      startMs: 8500,
    },
    {
      id: "s6",
      name: "分析文件内容",
      type: "thinking",
      description: "使用 LLM 分析文件并生成摘要",
      durationMs: 2200,
      tokenUsage: 680,
      startMs: 9100,
    },
    {
      id: "s7",
      name: "write_file",
      type: "tool_call",
      description: "将分析结果写入新文件",
      durationMs: 400,
      tokenUsage: 50,
      startMs: 11300,
    },
    {
      id: "s8",
      name: "write_file 失败",
      type: "error",
      description: "权限不足，无法写入目标路径",
      durationMs: 200,
      startMs: 11700,
    },
    {
      id: "s9",
      name: "重试 write_file",
      type: "tool_call",
      description: "切换到有权限的路径重新写入",
      durationMs: 350,
      tokenUsage: 45,
      startMs: 11900,
    },
  ];
  // Assign cumulative startMs for proportional display
  return steps;
}

// ── Component ──

export function TraceTimeline({ traceId: _traceId }: TraceTimelineProps) {
  const { t } = useTranslation();
  const steps = useMemo(() => buildMockSteps(), []);

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

// SPDX-License-Identifier: AGPL-3.0-only

// DT-P0-2: 与后端 telemetry/src/span.rs SpanType 对齐,补充 workflow/workflow_node
export type SpanType =
  | "agent"
  | "tool"
  | "llm_call"
  | "task"
  | "sub_task"
  | "reflection"
  | "reasoning"
  | "workflow"
  | "workflow_node";

export type SpanStatus = "ok" | "error" | "cancelled";

export interface SpanEvent {
  name: string;
  timestamp: string;
  attributes: Record<string, unknown>;
}

export interface SpanError {
  errorType: string;
  message: string;
  stackTrace?: string;
  timestamp: string;
}

export interface Span {
  id: string;
  traceId: string;
  parentSpanId?: string;
  name: string;
  spanType: SpanType;
  serviceName?: string;
  startTime: string;
  endTime?: string;
  durationMs?: number;
  status: SpanStatus;
  attributes: Record<string, unknown>;
  events: SpanEvent[];
  inputs?: unknown;
  outputs?: unknown;
  errors: SpanError[];
}

export interface TraceMetadata {
  userId: string;
  sessionId: string;
  agentVersion: string;
  model: string;
  totalTokens: number;
  totalCostUsd: number;
  totalDurationMs: number;
}

export interface TraceExport {
  traceId: string;
  spans: Span[];
  metadata: TraceMetadata;
  exportedAt: string;
}

export interface TraceSummary {
  traceId: string;
  sessionId: string;
  startedAt: string;
  endedAt?: string;
  durationMs?: number;
  spanCount: number;
  errorCount: number;
  totalTokens: number;
  totalCostUsd: number;
}

export interface TraceFilter {
  sessionId?: string;
  traceId?: string;
  fromDate?: string;
  toDate?: string;
  minDurationMs?: number;
  maxDurationMs?: number;
  hasErrors?: boolean;
  limit?: number;
  offset?: number;
}

export interface CostMetrics {
  totalTokens: number;
  inputTokens: number;
  outputTokens: number;
  cacheCreationTokens: number;
  cacheReadTokens: number;
  totalCostUsd: number;
  model: string;
}

export interface TraceMetrics {
  totalDurationMs: number;
  ttftMs?: number;
  cost: CostMetrics;
  spansCount: number;
  errorsCount: number;
}

export interface SpanMetrics {
  spanId: string;
  name: string;
  spanType: string;
  durationMs: number;
  startTime: string;
  endTime?: string;
  status: string;
  attributes: Record<string, unknown>;
  errorCount: number;
}

export interface AggregatedMetrics {
  totalTraces: number;
  totalSpans: number;
  totalErrors: number;
  avgDurationMs: number;
  avgTokens: number;
  avgCostUsd: number;
  tracesByType: Record<string, number>;
  errorsByType: Record<string, number>;
}

export interface SpanTreeNode extends Span {
  children: SpanTreeNode[];
}

export interface TraceListItem {
  traceId: string;
  sessionId: string;
  startedAt: string;
  durationMs?: number;
  spanCount: number;
  errorCount: number;
  totalCostUsd: number;
  status: "completed" | "in_progress" | "error";
}

export interface TraceDetail {
  trace: TraceExport;
  summary: TraceSummary;
  metrics: TraceMetrics;
  tree: SpanTreeNode[];
}

export interface TimelineItem {
  spanId: string;
  name: string;
  startTime: string;
  durationMs?: number;
  depth: number;
  spanType: SpanType;
  status: SpanStatus;
}

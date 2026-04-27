export type SpanType =
  | "agent"
  | "tool"
  | "llm_call"
  | "task"
  | "sub_task"
  | "reflection"
  | "reasoning";

export type SpanStatus = "ok" | "error" | "cancelled";

export interface SpanEvent {
  name: string;
  timestamp: string;
  attributes: Record<string, unknown>;
}

export interface SpanError {
  error_type: string;
  message: string;
  stack_trace?: string;
  timestamp: string;
}

export interface Span {
  id: string;
  trace_id: string;
  parent_span_id?: string;
  name: string;
  span_type: SpanType;
  service_name?: string;
  start_time: string;
  end_time?: string;
  duration_ms?: number;
  status: SpanStatus;
  attributes: Record<string, unknown>;
  events: SpanEvent[];
  inputs?: unknown;
  outputs?: unknown;
  errors: SpanError[];
}

export interface TraceMetadata {
  user_id: string;
  session_id: string;
  agent_version: string;
  model: string;
  total_tokens: number;
  total_cost_usd: number;
  total_duration_ms: number;
}

export interface TraceExport {
  trace_id: string;
  spans: Span[];
  metadata: TraceMetadata;
  exported_at: string;
}

export interface TraceSummary {
  trace_id: string;
  session_id: string;
  started_at: string;
  ended_at?: string;
  duration_ms?: number;
  span_count: number;
  error_count: number;
  total_tokens: number;
  total_cost_usd: number;
}

export interface TraceFilter {
  session_id?: string;
  trace_id?: string;
  from_date?: string;
  to_date?: string;
  min_duration_ms?: number;
  max_duration_ms?: number;
  has_errors?: boolean;
  limit?: number;
  offset?: number;
}

export interface CostMetrics {
  total_tokens: number;
  input_tokens: number;
  output_tokens: number;
  cache_creation_tokens: number;
  cache_read_tokens: number;
  total_cost_usd: number;
  model: string;
}

export interface TraceMetrics {
  total_duration_ms: number;
  ttft_ms?: number;
  cost: CostMetrics;
  spans_count: number;
  errors_count: number;
}

export interface SpanMetrics {
  span_id: string;
  name: string;
  span_type: string;
  duration_ms: number;
  start_time: string;
  end_time?: string;
  status: string;
  attributes: Record<string, unknown>;
  error_count: number;
}

export interface AggregatedMetrics {
  total_traces: number;
  total_spans: number;
  total_errors: number;
  avg_duration_ms: number;
  avg_tokens: number;
  avg_cost_usd: number;
  traces_by_type: Record<string, number>;
  errors_by_type: Record<string, number>;
}

export interface SpanTreeNode extends Span {
  children: SpanTreeNode[];
}

export interface TraceListItem {
  trace_id: string;
  session_id: string;
  started_at: string;
  duration_ms?: number;
  span_count: number;
  error_count: number;
  total_cost_usd: number;
  status: "completed" | "in_progress" | "error";
}

export interface TraceDetail {
  trace: TraceExport;
  summary: TraceSummary;
  metrics: TraceMetrics;
  tree: SpanTreeNode[];
}

export interface TimelineItem {
  span_id: string;
  name: string;
  start_time: string;
  duration_ms?: number;
  depth: number;
  span_type: SpanType;
  status: SpanStatus;
}

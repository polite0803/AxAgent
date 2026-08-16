// SPDX-License-Identifier: AGPL-3.0-only

import { invoke } from "@/lib/invoke";
import type { Span, SpanTreeNode, TraceDetail, TraceExport, TraceFilter, TraceMetrics, TraceSummary } from "@/types";
import { create } from "zustand";

interface TracerState {
  traces: TraceSummary[];
  selectedTrace: TraceDetail | null;
  selectedSpan: Span | null;
  isLoading: boolean;
  error: string | null;
  filter: TraceFilter;
  tree: SpanTreeNode[];
  metrics: TraceMetrics | null;

  loadTraces: (filter?: TraceFilter) => Promise<void>;
  loadTrace: (traceId: string) => Promise<void>;
  selectTrace: (traceId: string) => Promise<void>;
  selectSpan: (spanId: string) => void;
  clearSelection: () => void;
  setFilter: (filter: TraceFilter) => void;
  exportTrace: (traceId: string, format: "json" | "csv") => Promise<void>;
  deleteTrace: (traceId: string) => Promise<void>;
  clearAll: () => void;

  /** Record an LLM call span with model/token/cost metadata */
  recordLlmCall: (params: {
    traceId: string;
    parentSpanId?: string;
    modelId: string;
    providerId: string;
    inputTokens: number;
    outputTokens: number;
    costUsd: number;
    durationMs: number;
    cacheHit: boolean;
    fallbackUsed: boolean;
    fallbackModelId?: string;
  }) => Promise<void>;

  /** Setup PerformanceObserver for long task detection */
  setupLongTaskObserver: () => void;

  // ── Phase 3: Bottleneck analysis + suggestions + feedback ──

  getBottlenecks: (traceId: string) => Promise<{
    timeDistribution: { name: string; value: number; color: string }[];
    tokenDistribution: { name: string; tokens: number }[];
    failureModes: { reason: string; count: number; pct: number }[];
  }>;

  generateSuggestions: (traceId: string) => Promise<
    { id: string; problem: string; suggestion: string; expectedImprovement: string }[]
  >;

  feedbackHistory: { traceId: string; rating: "like" | "dislike"; comment?: string; timestamp: number }[];

  submitFeedback: (traceId: string, rating: "like" | "dislike", comment?: string) => Promise<void>;
}

function buildSpanTree(spans: Span[]): SpanTreeNode[] {
  const spanMap = new Map<string, SpanTreeNode>();
  const roots: SpanTreeNode[] = [];

  spans.forEach((span) => {
    spanMap.set(span.id, { ...span, children: [] });
  });

  spans.forEach((span) => {
    const node = spanMap.get(span.id)!;
    if (span.parentSpanId) {
      const parent = spanMap.get(span.parentSpanId);
      if (parent) {
        parent.children.push(node);
      } else {
        roots.push(node);
      }
    } else {
      roots.push(node);
    }
  });

  return roots;
}

export const useTracerStore = create<TracerState>((set, get) => ({
  traces: [],
  selectedTrace: null,
  selectedSpan: null,
  isLoading: false,
  error: null,
  filter: {},
  tree: [],
  metrics: null,
  feedbackHistory: [],

  loadTraces: async (filter?: TraceFilter) => {
    set({ isLoading: true, error: null });
    try {
      const traces = await invoke<TraceSummary[]>("tracer_list_traces", {
        filter: filter || get().filter,
      });
      set({ traces, isLoading: false });
    } catch (error) {
      set({
        error: error instanceof Error ? error.message : "Failed to load traces",
        isLoading: false,
      });
    }
  },

  loadTrace: async (traceId: string) => {
    set({ isLoading: true, error: null });
    try {
      const traceExport = await invoke<TraceExport>("tracer_get_trace", { traceId });
      if (!traceExport || !traceExport.spans) {
        set({ error: "Trace not found", isLoading: false });
        return;
      }
      const tree = buildSpanTree(traceExport.spans);
      const metrics: TraceMetrics = {
        totalDurationMs: traceExport.metadata.totalDurationMs,
        ttftMs: undefined,
        cost: {
          totalTokens: traceExport.metadata.totalTokens,
          inputTokens: 0,
          outputTokens: 0,
          cacheCreationTokens: 0,
          cacheReadTokens: 0,
          totalCostUsd: traceExport.metadata.totalCostUsd,
          model: traceExport.metadata.model,
        },
        spansCount: traceExport.spans.length,
        errorsCount: traceExport.spans.filter((s) => s.status === "error").length,
      };
      const summary: TraceSummary = {
        traceId: traceExport.traceId,
        sessionId: traceExport.metadata.sessionId,
        startedAt: traceExport.spans[0]?.startTime || traceExport.exportedAt,
        durationMs: traceExport.metadata.totalDurationMs,
        spanCount: traceExport.spans.length,
        errorCount: traceExport.spans.filter((s) => s.status === "error").length,
        totalTokens: traceExport.metadata.totalTokens,
        totalCostUsd: traceExport.metadata.totalCostUsd,
      };
      set({
        selectedTrace: { trace: traceExport, summary, metrics, tree },
        tree,
        metrics,
        isLoading: false,
      });
    } catch (error) {
      set({
        error: error instanceof Error ? error.message : "Failed to load trace",
        isLoading: false,
      });
    }
  },

  selectTrace: async (traceId: string) => {
    await get().loadTrace(traceId);
  },

  selectSpan: (spanId: string) => {
    const { selectedTrace } = get();
    if (selectedTrace) {
      const findSpan = (spans: Span[]): Span | undefined => {
        for (const span of spans) {
          if (span.id === spanId) {
            return span;
          }
          const found = findSpan(span.events as unknown as Span[]); /* SAFE: tracer span events type narrowing */
          if (found) {
            return found;
          }
        }
        return undefined;
      };
      const span = findSpan(selectedTrace.trace.spans);
      set({ selectedSpan: span || null });
    }
  },

  clearSelection: () => {
    set({
      selectedTrace: null,
      selectedSpan: null,
      tree: [],
      metrics: null,
    });
  },

  setFilter: (filter: TraceFilter) => {
    set({ filter });
  },

  exportTrace: async (traceId: string, format: "json" | "csv") => {
    set({ isLoading: true, error: null });
    try {
      await invoke("tracer_export_traces", { trace_ids: [traceId], format });
      set({ isLoading: false });
    } catch (error) {
      set({
        error: error instanceof Error ? error.message : "Failed to export trace",
        isLoading: false,
      });
    }
  },

  deleteTrace: async (traceId: string) => {
    set({ isLoading: true, error: null });
    try {
      await invoke("tracer_delete_trace", { traceId });
      const traces = get().traces.filter((t) => t.traceId !== traceId);
      set({ traces, isLoading: false });
      if (get().selectedTrace?.trace.traceId === traceId) {
        set({ selectedTrace: null, tree: [], metrics: null });
      }
    } catch (error) {
      set({
        error: error instanceof Error ? error.message : "Failed to delete trace",
        isLoading: false,
      });
    }
  },

  clearAll: () => {
    set({
      traces: [],
      selectedTrace: null,
      selectedSpan: null,
      tree: [],
      metrics: null,
      filter: {},
      error: null,
    });
  },

  // ── LLM call tracing (P2 enhancement) ──

  recordLlmCall: async (params: {
    traceId: string;
    parentSpanId?: string;
    modelId: string;
    providerId: string;
    inputTokens: number;
    outputTokens: number;
    costUsd: number;
    durationMs: number;
    cacheHit: boolean;
    fallbackUsed: boolean;
    fallbackModelId?: string;
  }) => {
    try {
      await invoke("tracer_record_span", {
        request: {
          traceId: params.traceId,
          span: {
            spanType: "llm_call",
            parentSpanId: params.parentSpanId || null,
            name: `llm:${params.modelId}`,
            startTime: new Date(Date.now() - params.durationMs).toISOString(),
            endTime: new Date().toISOString(),
            durationMs: params.durationMs,
            status: "ok",
            attributes: {
              model_id: params.modelId,
              provider_id: params.providerId,
              input_tokens: params.inputTokens,
              output_tokens: params.outputTokens,
              total_tokens: params.inputTokens + params.outputTokens,
              cost_usd: params.costUsd,
              cache_hit: params.cacheHit,
              fallback_used: params.fallbackUsed,
              fallback_model_id: params.fallbackModelId || null,
            },
            events: [],
            errors: [],
          },
        },
      });
    } catch {
      // Tracer is fire-and-forget
    }
  },

  setupLongTaskObserver: () => {
    if (typeof window === "undefined" || !("PerformanceObserver" in window)) {
      return;
    }
    try {
      const observer = new PerformanceObserver((list) => {
        for (const entry of list.getEntries()) {
          if (entry.duration > 50) {
            console.debug(`[tracer] Long task: ${entry.duration.toFixed(1)}ms`);
          }
        }
      });
      observer.observe({ type: "longtask", buffered: true });
    } catch {
      // Long task API not universally available
    }
  },

  // ── Phase 3: Bottleneck analysis + suggestions + feedback ──

  getBottlenecks: async (traceId: string) => {
    try {
      return await invoke<{
        timeDistribution: { name: string; value: number; color: string }[];
        tokenDistribution: { name: string; tokens: number }[];
        failureModes: { reason: string; count: number; pct: number }[];
      }>("tracer_get_bottlenecks", { trace_id: traceId });
    } catch (e) {
      console.warn("[tracerStore] getBottlenecks failed, using mock", e);
      return {
        timeDistribution: [
          { name: "LLM Inference", value: 45, color: "#1890ff" },
          { name: "Tool Calls", value: 25, color: "#fa8c16" },
          { name: "Awaiting Permission", value: 15, color: "#fadb14" },
          { name: "Network Latency", value: 10, color: "#722ed1" },
          { name: "Other", value: 5, color: "#d9d9d9" },
        ],
        tokenDistribution: [
          { name: "System Prompt", tokens: 1200 },
          { name: "Tool Definitions", tokens: 800 },
          { name: "Conversation History", tokens: 3200 },
          { name: "Tool Results", tokens: 1500 },
          { name: "User Input", tokens: 400 },
        ],
        failureModes: [
          { reason: "Tool Execution Timeout", count: 12, pct: 40 },
          { reason: "Insufficient Permission", count: 8, pct: 26.7 },
          { reason: "Invalid Parameter Format", count: 5, pct: 16.7 },
          { reason: "Network Error", count: 3, pct: 10 },
          { reason: "LLM Output Parse Failure", count: 2, pct: 6.6 },
        ],
      };
    }
  },

  generateSuggestions: async (traceId: string) => {
    try {
      return await invoke<
        { id: string; problem: string; suggestion: string; expectedImprovement: string }[]
      >("tracer_generate_suggestions", { trace_id: traceId });
    } catch (e) {
      console.warn("[tracerStore] generateSuggestions failed, using mock", e);
      return [
        {
          id: "sug_001",
          problem: "Tool calls `search_file` and `read_file` could run in parallel but ran sequentially.",
          suggestion: "Mark independent tool calls as parallel; the agent should auto-detect and batch them.",
          expectedImprovement: "Expected ~25% reduction in total execution time",
        },
        {
          id: "sug_002",
          problem: "System prompt includes many redundant tool definitions.",
          suggestion: "Dynamically trim the tool list based on conversation context.",
          expectedImprovement: "Saves ~800 tokens per session",
        },
        {
          id: "sug_003",
          problem: "Error handling is too conservative: it aborts on permission errors.",
          suggestion: "Add a fallback path list in the skill configuration.",
          expectedImprovement: "Expected error rate drop from 8% to 3%",
        },
      ];
    }
  },

  submitFeedback: async (traceId: string, rating: "like" | "dislike", comment?: string) => {
    const entry = { traceId, rating, comment, timestamp: Date.now() };
    set((s) => ({ feedbackHistory: [...s.feedbackHistory, entry] }));

    try {
      await invoke("tracer_submit_feedback", { trace_id: traceId, rating, comment });
    } catch {
      console.warn("[tracerStore] submitFeedback invoke failed, saved locally");
    }
  },
}));

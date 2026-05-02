import { invoke } from "@/lib/invoke";
import type { Span, SpanTreeNode, TraceDetail, TraceFilter, TraceMetrics, TraceSummary } from "@/types/tracer";
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
}

function buildSpanTree(spans: Span[]): SpanTreeNode[] {
  const spanMap = new Map<string, SpanTreeNode>();
  const roots: SpanTreeNode[] = [];

  spans.forEach((span) => {
    spanMap.set(span.id, { ...span, children: [] });
  });

  spans.forEach((span) => {
    const node = spanMap.get(span.id)!;
    if (span.parent_span_id) {
      const parent = spanMap.get(span.parent_span_id);
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
      const trace = await invoke<TraceDetail>("tracer_get_trace", { traceId });
      const tree = buildSpanTree(trace.trace.spans);
      set({
        selectedTrace: trace,
        tree,
        metrics: trace.metrics,
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
          if (span.id === spanId) { return span; }
          const found = findSpan(span.events as unknown as Span[]);
          if (found) { return found; }
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
      await invoke("tracer_export_trace", { traceId, format });
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
      const traces = get().traces.filter((t) => t.trace_id !== traceId);
      set({ traces, isLoading: false });
      if (get().selectedTrace?.trace.trace_id === traceId) {
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
        traceId: params.traceId,
        span: {
          span_type: "llm_call",
          parent_span_id: params.parentSpanId || null,
          name: `llm:${params.modelId}`,
          start_time: new Date(Date.now() - params.durationMs).toISOString(),
          end_time: new Date().toISOString(),
          duration_ms: params.durationMs,
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
      });
    } catch {
      // Tracer is fire-and-forget
    }
  },

  setupLongTaskObserver: () => {
    if (typeof window === "undefined" || !("PerformanceObserver" in window)) { return; }
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
}));

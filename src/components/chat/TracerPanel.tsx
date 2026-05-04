import { invoke } from "@/lib/invoke";
import { Bug, Clock, Trash2 } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

interface TraceSummary {
  trace_id: string;
  started_at?: string;
  ended_at?: string;
  duration_ms: number;
  span_count: number;
  error_count: number;
}

export default function TracerPanel() {
  const { t } = useTranslation();
  const [expanded, setExpanded] = useState(false);
  const [traces, setTraces] = useState<TraceSummary[]>([]);

  const fetchTraces = useCallback(async () => {
    try {
      const list = await invoke<TraceSummary[]>("tracer_list_traces", {
        limit: 10,
        offset: 0,
      });
      setTraces(list);
    } catch {
      // ignore
    }
  }, []);

  useEffect(() => {
    if (!expanded) { return; }
    fetchTraces();
    const interval = setInterval(fetchTraces, 30_000);
    return () => clearInterval(interval);
  }, [expanded, fetchTraces]);

  const handleDelete = async (traceId: string) => {
    try {
      await invoke("tracer_delete_trace", { traceId });
      fetchTraces();
    } catch {
      // ignore
    }
  };

  if (!expanded) {
    return (
      <div className="border-b border-border/50 px-3 py-2">
        <button
          onClick={() => setExpanded(true)}
          className="flex items-center gap-1.5 text-xs text-muted-foreground hover:text-foreground transition-colors"
        >
          <Bug size={14} />
          {t("chat.tracer")} ({traces.length})
        </button>
      </div>
    );
  }

  const totalErrors = traces.reduce((sum, t) => sum + t.error_count, 0);
  const totalSpans = traces.reduce((sum, t) => sum + t.span_count, 0);

  return (
    <div className="border-b border-border/50 px-3 py-2 space-y-2">
      <div className="flex items-center justify-between">
        <span className="text-xs font-medium text-foreground/80">{t("chat.tracerTitle")}</span>
        <button
          onClick={() => setExpanded(false)}
          className="text-muted-foreground hover:text-foreground transition-colors"
        >
          <svg className="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
            <path strokeLinecap="round" strokeLinejoin="round" d="M6 18L18 6M6 6l12 12" />
          </svg>
        </button>
      </div>

      {/* Summary stats */}
      <div className="grid grid-cols-2 gap-1.5">
        <div className="text-center p-1 rounded bg-muted/30">
          <div className="text-[10px] text-muted-foreground">{t("chat.traces")}</div>
          <div className="text-xs font-medium">{traces.length}</div>
        </div>
        <div className="text-center p-1 rounded bg-muted/30">
          <div className="text-[10px] text-muted-foreground">{t("chat.spans")}</div>
          <div className="text-xs font-medium">{totalSpans}</div>
        </div>
        <div className="text-center p-1 rounded bg-muted/30">
          <div className="text-[10px] text-muted-foreground">{t("chat.errors")}</div>
          <div className={`text-xs font-medium ${totalErrors > 0 ? "text-red-500" : ""}`}>
            {totalErrors}
          </div>
        </div>
        <div className="text-center p-1 rounded bg-muted/30">
          <div className="text-[10px] text-muted-foreground">{t("chat.status")}</div>
          <div className="text-xs font-medium text-green-500">
            {t("chat.active")}
          </div>
        </div>
      </div>

      {/* Trace list */}
      {traces.length > 0
        ? (
          <div className="max-h-48 overflow-y-auto space-y-1">
            {traces.map((trace) => (
              <div
                key={trace.trace_id}
                className="text-xs p-1.5 rounded bg-muted/30 flex items-center gap-2"
              >
                <span
                  className={`w-1.5 h-1.5 rounded-full shrink-0 ${
                    trace.error_count > 0 ? "bg-red-500" : "bg-green-500"
                  }`}
                />
                <div className="flex-1 min-w-0">
                  <div className="text-foreground/80 truncate font-mono text-[10px]">
                    {trace.trace_id.slice(0, 8)}
                  </div>
                  <div className="text-[10px] text-muted-foreground/60 flex items-center gap-1">
                    <Clock size={10} />
                    {trace.duration_ms > 0 ? `${trace.duration_ms}ms` : "--"}
                    <span>·</span>
                    {trace.span_count} spans
                    {trace.error_count > 0 && (
                      <>
                        <span>·</span>
                        <span className="text-red-500">{trace.error_count} err</span>
                      </>
                    )}
                  </div>
                </div>
                <button
                  onClick={() => handleDelete(trace.trace_id)}
                  className="p-0.5 rounded hover:bg-muted/50 text-muted-foreground/40 hover:text-red-500 transition-colors"
                  title="Delete trace"
                >
                  <Trash2 size={12} />
                </button>
              </div>
            ))}
          </div>
        )
        : (
          <div className="text-xs text-muted-foreground/60">
            {t("chat.noTraces")}
          </div>
        )}
    </div>
  );
}

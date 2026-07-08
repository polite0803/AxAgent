import { invoke } from "@/lib/invoke";
import type { ExecutionStatusResponse, ExecutionSummary } from "@/types";
import { Badge, Button, Card, Empty, Spin, Tag, Timeline } from "antd";
import { CheckCircle, Clock, Loader, Play, RefreshCw, XCircle } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

const STATUS_COLOR: Record<string, string> = {
  completed: "green",
  running: "blue",
  pending: "default",
  ready: "cyan",
  failed: "red",
  skipped: "orange",
  paused: "gold",
};

const STATUS_ICON: Record<string, React.ReactNode> = {
  completed: <CheckCircle size={14} />,
  running: <Loader size={14} className="animate-spin" />,
  pending: <Clock size={14} />,
  ready: <Play size={14} />,
  failed: <XCircle size={14} />,
  skipped: <Clock size={14} />,
  paused: <Clock size={14} />,
};

export function ExecutionReplayPanel() {
  const { t } = useTranslation();
  const [executions, setExecutions] = useState<ExecutionSummary[]>([]);
  const [loading, setLoading] = useState(false);
  const [fetchError, setFetchError] = useState(false);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [status, setStatus] = useState<ExecutionStatusResponse | null>(null);
  const [loadingStatus, setLoadingStatus] = useState(false);

  const load = async () => {
    setLoading(true);
    setFetchError(false);
    try {
      const list = await invoke<ExecutionSummary[]>("list_workflow_executions", {
        workflowId: "stock-analysis",
      });
      setExecutions(list ?? []);
    } catch {
      setFetchError(true);
    }
    setLoading(false);
  };

  useEffect(() => {
    let cancelled = false;
    Promise.resolve().then(() => {
      if (cancelled) { return; }
      setLoading(true);
      setFetchError(false);
      return invoke<ExecutionSummary[]>("list_workflow_executions", {
        workflowId: "stock-analysis",
      });
    })
      .then((list) => {
        if (cancelled || !list) { return; }
        setExecutions(list ?? []);
      })
      .catch(() => {
        if (!cancelled) { setFetchError(true); }
      })
      .finally(() => {
        if (!cancelled) { setLoading(false); }
      });
    return () => {
      cancelled = true;
    };
  }, []);

  const loadStatus = useCallback(async (executionId: string) => {
    setLoadingStatus(true);
    try {
      const s = await invoke<ExecutionStatusResponse>("get_workflow_execution_status", {
        executionId,
      });
      setStatus(s);
    } catch {
      /* */
    }
    setLoadingStatus(false);
  }, []);

  const handleSelect = async (id: string) => {
    setSelectedId(id);
    await loadStatus(id);
  };

  const formatMs = (ms: number) => {
    if (ms < 1000) { return `${ms}ms`; }
    return `${(ms / 1000).toFixed(1)}s`;
  };

  const formatDate = (ts: number) => {
    return new Date(ts).toLocaleString("zh-CN", {
      month: "2-digit",
      day: "2-digit",
      hour: "2-digit",
      minute: "2-digit",
    });
  };

  return (
    <Card
      size="small"
      title={t("workEngine.executionHistory")}
      styles={{ body: { padding: "4px 8px" } }}
      extra={
        <Button size="small" loading={loading} icon={<RefreshCw size={12} />} onClick={load}>
          {t("stockAnalysis.settings.panels.refresh")}
        </Button>
      }
    >
      {loading
        ? <Spin size="small" />
        : fetchError
        ? (
          <Empty
            image={Empty.PRESENTED_IMAGE_SIMPLE}
            description={t("stockAnalysis.settings.vendor.connectionFailed")}
          />
        )
        : executions.length === 0
        ? <Empty image={Empty.PRESENTED_IMAGE_SIMPLE} description={t("workEngine.noExecutionHistory")} />
        : (
          <div className="flex flex-col gap-1">
            <div className="flex flex-col gap-0.5 max-h-32 overflow-y-auto">
              {executions.slice(0, 20).map((e) => (
                <button
                  key={e.id}
                  type="button"
                  onClick={() => handleSelect(e.id)}
                  className={`flex items-center gap-1.5 px-2 py-1 rounded text-xs text-left transition-colors ${
                    selectedId === e.id ? "bg-blue-50 dark:bg-blue-900/20" : "hover:bg-gray-50 dark:hover:bg-gray-800"
                  }`}
                >
                  <Tag
                    color={STATUS_COLOR[e.status] ?? "default"}
                    className="text-xs m-0 leading-none shrink-0"
                    style={{ fontSize: 10, padding: "0 4px", lineHeight: "16px" }}
                  >
                    {e.status}
                  </Tag>
                  <span className="flex-1 truncate text-gray-500 font-mono" style={{ fontSize: 10 }}>
                    {e.id.slice(0, 8)}
                  </span>
                  <span className="text-gray-400 shrink-0" style={{ fontSize: 10 }}>
                    {formatDate(e.created_at)}
                  </span>
                  {e.total_time_ms != null && (
                    <span className="text-gray-400 shrink-0" style={{ fontSize: 10 }}>
                      {formatMs(e.total_time_ms)}
                    </span>
                  )}
                </button>
              ))}
            </div>

            {selectedId && (
              <div className="border-t pt-1 mt-1">
                {loadingStatus
                  ? <Spin size="small" />
                  : status
                  ? (
                    <Timeline
                      items={[
                        ...(status.node_records ?? []).map((r) => ({
                          color: STATUS_COLOR[r.status] ?? "gray",
                          dot: STATUS_ICON[r.status] ?? <Clock size={14} />,
                          children: (
                            <div className="flex flex-col gap-0.5" style={{ fontSize: 11 }}>
                              <div className="flex items-center gap-1">
                                <Badge
                                  color={STATUS_COLOR[r.status] as string}
                                  className="shrink-0"
                                />
                                <span className="font-medium truncate">{r.node_name ?? r.node_id}</span>
                                <Tag className="text-xs m-0 leading-none" style={{ fontSize: 10, padding: "0 3px" }}>
                                  {r.node_type}
                                </Tag>
                              </div>
                              {r.execution_time_ms != null && (
                                <span className="text-gray-400">{formatMs(r.execution_time_ms)}</span>
                              )}
                              {r.error && <span className="text-red-400 truncate text-xs">{r.error}</span>}
                            </div>
                          ),
                        })),
                        {
                          color: status.status === "completed" ? "green" : status.status === "failed" ? "red" : "blue",
                          dot: status.status === "completed"
                            ? <CheckCircle size={14} />
                            : status.status === "failed"
                            ? <XCircle size={14} />
                            : <Loader size={14} className="animate-spin" />,
                          children: (
                            <span className="font-medium" style={{ fontSize: 11 }}>
                              {t("workEngine.status")}: {status.status}
                              {" · "}
                              {status.node_count} {t("workEngine.nodes")}
                              {" · "}
                              {status.total_time_ms != null ? formatMs(status.total_time_ms) : "-"}
                            </span>
                          ),
                        },
                      ]}
                      style={{ marginTop: 4 }}
                    />
                  )
                  : <Empty image={Empty.PRESENTED_IMAGE_SIMPLE} description={t("workEngine.noExecutionHistory")} />}
              </div>
            )}
          </div>
        )}
    </Card>
  );
}

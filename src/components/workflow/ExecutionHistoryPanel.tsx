import { invoke } from "@/lib/invoke";
import { theme } from "antd";
import { AlertTriangle, CheckCircle, Clock, XCircle } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

interface ExecutionRecord {
  execution_id: string;
  workflow_id: string;
  status: string;
  started_at: string;
  completed_at: string | null;
  total_time_ms: number;
  node_count: number;
  error: string | null;
}

interface Props {
  workflowId?: string;
  onSelectExecution?: (executionId: string) => void;
}

export function ExecutionHistoryPanel({ workflowId, onSelectExecution }: Props) {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const [records, setRecords] = useState<ExecutionRecord[]>([]);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    if (!workflowId) { return; }
    setLoading(true);
    invoke<ExecutionRecord[]>("list_workflow_executions", { workflowId })
      .then(setRecords)
      .catch(() => setRecords([]))
      .finally(() => setLoading(false));
  }, [workflowId]);

  const statusIcon = (status: string) => {
    switch (status) {
      case "completed":
        return <CheckCircle size={14} color={token.colorSuccess} />;
      case "failed":
        return <XCircle size={14} color={token.colorError} />;
      case "running":
        return <Clock size={14} color={token.colorPrimary} />;
      default:
        return <AlertTriangle size={14} color={token.colorWarning} />;
    }
  };

  if (records.length === 0) {
    return (
      <div style={{ padding: 16, color: token.colorTextQuaternary, fontSize: 12, textAlign: "center" }}>
        {loading ? t("common.loading") : t("workflow.noExecutionHistory")}
      </div>
    );
  }

  return (
    <div style={{ padding: "0 4px" }}>
      {records.map((r) => (
        <div
          key={r.execution_id}
          onClick={() => onSelectExecution?.(r.execution_id)}
          style={{
            padding: "8px 10px",
            marginBottom: 4,
            borderRadius: 6,
            cursor: "pointer",
            border: `1px solid ${token.colorBorderSecondary}`,
            fontSize: 12,
          }}
        >
          <div style={{ display: "flex", alignItems: "center", gap: 6, marginBottom: 3 }}>
            {statusIcon(r.status)}
            <span style={{ fontWeight: 500 }}>{r.status}</span>
            <span style={{ marginLeft: "auto", color: token.colorTextQuaternary, fontSize: 11 }}>
              {(r.total_time_ms / 1000).toFixed(1)}s
            </span>
          </div>
          <div style={{ color: token.colorTextSecondary, fontSize: 11 }}>
            {new Date(r.started_at).toLocaleString()}
            {r.error && <div style={{ color: token.colorError, marginTop: 2 }}>{r.error.slice(0, 80)}</div>}
          </div>
        </div>
      ))}
    </div>
  );
}

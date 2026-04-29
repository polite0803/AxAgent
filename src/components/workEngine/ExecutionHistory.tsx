import { Empty, Spin, Table, Tag, Typography } from "antd";
import React, { useEffect } from "react";
import { useTranslation } from "react-i18next";
import { useWorkEngineStore } from "../../stores/feature/workEngineStore";

const { Text } = Typography;

interface ExecutionHistoryProps {
  workflowId: string;
}

const STATUS_COLORS: Record<string, string> = {
  pending: "default",
  running: "processing",
  paused: "warning",
  completed: "success",
  failed: "error",
  cancelled: "default",
};

export const ExecutionHistory: React.FC<ExecutionHistoryProps> = ({ workflowId }) => {
  const { t } = useTranslation();
  const STATUS_LABELS: Record<string, string> = {
    pending: t("workEngine.statusPending"),
    running: t("workEngine.statusRunning"),
    paused: t("workEngine.statusPaused"),
    completed: t("workEngine.statusCompleted"),
    failed: t("workEngine.statusFailed"),
    cancelled: t("workEngine.statusCancelled"),
  };
  const { executionHistory: history, loading, loadHistory } = useWorkEngineStore();

  useEffect(() => {
    loadHistory(workflowId);
  }, [workflowId, loadHistory]);

  const columns = [
    {
      title: t("workEngine.executionId"),
      dataIndex: "id",
      key: "id",
      width: 120,
      render: (id: string) => <Text copyable={{ text: id }}>{id.slice(0, 8)}</Text>,
    },
    {
      title: t("workEngine.status"),
      dataIndex: "status",
      key: "status",
      width: 90,
      render: (status: string) => (
        <Tag color={STATUS_COLORS[status] || "default"}>
          {STATUS_LABELS[status] || status}
        </Tag>
      ),
    },
    {
      title: t("workEngine.startTime"),
      dataIndex: "started_at",
      key: "started_at",
      width: 180,
      render: (t: string) => t ? new Date(t).toLocaleString() : "-",
    },
    {
      title: t("workEngine.duration"),
      dataIndex: "total_time_ms",
      key: "total_time_ms",
      width: 80,
      render: (ms: number) => ms ? `${ms}ms` : "-",
    },
    {
      title: t("workEngine.nodeProgress"),
      key: "progress",
      width: 100,
      render: (_: unknown, record: Record<string, unknown>) => {
        const completed = typeof record.completed_nodes === "number" ? record.completed_nodes : 0;
        const total = typeof record.total_nodes === "number"
          ? record.total_nodes
          : (typeof record.node_count === "number" ? record.node_count : 0);
        return <Text>{completed}/{total}</Text>;
      },
    },
  ];

  if (loading) { return <Spin />; }
  if (!history || history.length === 0) { return <Empty description={t("workEngine.noExecutionHistory")} />; }

  return (
    <Table
      dataSource={history as unknown as Record<string, unknown>[]}
      columns={columns}
      rowKey="id"
      size="small"
      pagination={{ pageSize: 10 }}
    />
  );
};

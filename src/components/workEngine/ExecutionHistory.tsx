import React, { useEffect } from 'react';
import { Table, Tag, Typography, Empty, Spin } from 'antd';
import { useWorkEngineStore } from '../../stores/feature/workEngineStore';

const { Text } = Typography;

interface ExecutionHistoryProps {
  workflowId: string;
}

const STATUS_COLORS: Record<string, string> = {
  pending: 'default',
  running: 'processing',
  paused: 'warning',
  completed: 'success',
  failed: 'error',
  cancelled: 'default',
};

const STATUS_LABELS: Record<string, string> = {
  pending: '等待中',
  running: '运行中',
  paused: '已暂停',
  completed: '已完成',
  failed: '已失败',
  cancelled: '已取消',
};

export const ExecutionHistory: React.FC<ExecutionHistoryProps> = ({ workflowId }) => {
  const { executionHistory: history, loading, loadHistory } = useWorkEngineStore();

  useEffect(() => {
    loadHistory(workflowId);
  }, [workflowId, loadHistory]);

  const columns = [
    {
      title: '执行ID',
      dataIndex: 'id',
      key: 'id',
      width: 120,
      render: (id: string) => <Text copyable={{ text: id }}>{id.slice(0, 8)}</Text>,
    },
    {
      title: '状态',
      dataIndex: 'status',
      key: 'status',
      width: 90,
      render: (status: string) => (
        <Tag color={STATUS_COLORS[status] || 'default'}>
          {STATUS_LABELS[status] || status}
        </Tag>
      ),
    },
    {
      title: '开始时间',
      dataIndex: 'started_at',
      key: 'started_at',
      width: 180,
      render: (t: string) => t ? new Date(t).toLocaleString() : '-',
    },
    {
      title: '耗时',
      dataIndex: 'total_time_ms',
      key: 'total_time_ms',
      width: 80,
      render: (ms: number) => ms ? `${ms}ms` : '-',
    },
    {
      title: '节点进度',
      key: 'progress',
      width: 100,
      render: (_: unknown, record: Record<string, unknown>) => {
        const completed = typeof record.completed_nodes === 'number' ? record.completed_nodes : 0;
        const total = typeof record.total_nodes === 'number' ? record.total_nodes : (typeof record.node_count === 'number' ? record.node_count : 0);
        return <Text>{completed}/{total}</Text>;
      },
    },
  ];

  if (loading) return <Spin />;
  if (!history || history.length === 0) return <Empty description="暂无执行记录" />;

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

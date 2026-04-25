import React, { useEffect, useState } from 'react';
import { Button, Space, Tag, Tooltip, Popconfirm } from 'antd';
import {
  PlayCircleOutlined, PauseCircleOutlined, ReloadOutlined,
  StopOutlined, HistoryOutlined,
} from '@ant-design/icons';
import { useWorkEngineStore } from '../../stores/feature/workEngineStore';

interface ExecutionControlBarProps {
  workflowId: string;
}

export const ExecutionControlBar: React.FC<ExecutionControlBarProps> = ({ workflowId }) => {
  const {
    executionId, status, startExecution, pause, resume, cancel, loadHistory, setupEventListeners,
  } = useWorkEngineStore();
  const [starting, setStarting] = useState(false);

  useEffect(() => {
    const cleanup = setupEventListeners();
    return () => {
      cleanup.then((fn) => fn());
    };
  }, [setupEventListeners]);

  const isRunning = status?.status === 'running';
  const isPaused = status?.status === 'paused';
  const isCompleted = status?.status === 'completed' || status?.status === 'failed' || status?.status === 'cancelled';

  const handleStart = async () => {
    setStarting(true);
    try {
      await startExecution(workflowId, {});
    } finally {
      setStarting(false);
    }
  };

  return (
    <Space>
      {!executionId || isCompleted ? (
        <Tooltip title="启动执行">
          <Button
            type="primary"
            icon={<PlayCircleOutlined />}
            loading={starting}
            onClick={handleStart}
            size="small"
          >
            启动
          </Button>
        </Tooltip>
      ) : null}

      {isRunning && (
        <Tooltip title="暂停执行">
          <Button icon={<PauseCircleOutlined />} onClick={pause} size="small">暂停</Button>
        </Tooltip>
      )}

      {isPaused && (
        <Tooltip title="恢复执行">
          <Button type="primary" icon={<ReloadOutlined />} onClick={resume} size="small">恢复</Button>
        </Tooltip>
      )}

      {(isRunning || isPaused) && (
        <Popconfirm title="确定取消执行？" onConfirm={cancel}>
          <Tooltip title="取消执行">
            <Button danger icon={<StopOutlined />} size="small">取消</Button>
          </Tooltip>
        </Popconfirm>
      )}

      {status && (
        <Tag color={
          isRunning ? 'processing' :
          isPaused ? 'warning' :
          status.status === 'completed' ? 'success' :
          status.status === 'failed' ? 'error' : 'default'
        }>
          {status.status === 'running' ? '运行中' :
           status.status === 'paused' ? '已暂停' :
           status.status === 'completed' ? '已完成' :
           status.status === 'failed' ? '已失败' :
           status.status === 'cancelled' ? '已取消' : status.status}
        </Tag>
      )}

      {status && status.total_time_ms > 0 && (
        <span style={{ fontSize: 12, color: '#999' }}>{status.total_time_ms}ms</span>
      )}

      <Tooltip title="执行历史">
        <Button icon={<HistoryOutlined />} onClick={() => loadHistory(workflowId)} size="small" />
      </Tooltip>
    </Space>
  );
};

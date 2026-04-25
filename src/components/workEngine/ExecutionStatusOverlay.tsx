import React from 'react';
import { Badge, Tooltip } from 'antd';
import { useWorkEngineStore } from '../../stores/feature/workEngineStore';

interface ExecutionStatusOverlayProps {
  nodeIds: string[];
}

const STATUS_STYLES: Record<string, { color: string; status: 'default' | 'processing' | 'success' | 'error' | 'warning' }> = {
  pending: { color: '#d9d9d9', status: 'default' },
  running: { color: '#1890ff', status: 'processing' },
  completed: { color: '#52c41a', status: 'success' },
  failed: { color: '#ff4d4f', status: 'error' },
  skipped: { color: '#faad14', status: 'warning' },
};

export const ExecutionStatusOverlay: React.FC<ExecutionStatusOverlayProps> = ({ nodeIds }) => {
  const { nodeStatuses } = useWorkEngineStore();

  return (
    <div style={{ position: 'absolute', top: 0, right: 0, zIndex: 10 }}>
      {nodeIds.map((nodeId) => {
        const status = nodeStatuses[nodeId] || 'pending';
        const style = STATUS_STYLES[status] || STATUS_STYLES.pending;
        return (
          <Tooltip key={nodeId} title={`节点 ${nodeId}: ${status}`}>
            <Badge status={style.status} />
          </Tooltip>
        );
      })}
    </div>
  );
};

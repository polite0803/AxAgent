import React from 'react';
import { Input, Divider } from 'antd';
import type { WorkflowNode, EndNode } from '../../types';
import { BasePropertyPanel } from './BasePropertyPanel';

interface EndPropertyPanelProps {
  node: WorkflowNode;
  onUpdate: (updates: Partial<WorkflowNode>) => void;
  onDelete: () => void;
}

export const EndPropertyPanel: React.FC<EndPropertyPanelProps> = ({ node, onUpdate, onDelete }) => {
  const endNode = node as EndNode;
  const config = endNode.config || {};

  const handleConfigChange = (key: string, value: unknown) => {
    onUpdate({ config: { ...config, [key]: value } });
  };

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
      <div>
        <label style={{ display: 'block', color: '#999', fontSize: 11, marginBottom: 4 }}>输出变量</label>
        <Input
          value={config.output_var || ''}
          onChange={(e) => handleConfigChange('output_var', e.target.value)}
          size="small"
          placeholder="workflow_output"
        />
        <div style={{ fontSize: 10, color: '#666', marginTop: 4 }}>
          工作流的最终输出变量
        </div>
      </div>

      <Divider style={{ margin: '8px 0', borderColor: '#333' }} />

      <div style={{ borderTop: '1px solid #333', paddingTop: 12, marginTop: 4 }}>
        <BasePropertyPanel node={node} onUpdate={onUpdate} onDelete={onDelete} />
      </div>
    </div>
  );
};

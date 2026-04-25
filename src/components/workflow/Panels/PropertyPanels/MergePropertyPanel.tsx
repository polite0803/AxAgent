import React from 'react';
import { Select, Divider } from 'antd';
import type { WorkflowNode, MergeNode } from '../../types';
import { BasePropertyPanel } from './BasePropertyPanel';

interface MergePropertyPanelProps {
  node: WorkflowNode;
  onUpdate: (updates: Partial<WorkflowNode>) => void;
  onDelete: () => void;
}

export const MergePropertyPanel: React.FC<MergePropertyPanelProps> = ({ node, onUpdate, onDelete }) => {
  const mergeNode = node as MergeNode;
  const config = mergeNode.config || {
    merge_type: 'all',
    inputs: [],
  };

  const handleConfigChange = (key: string, value: unknown) => {
    onUpdate({ config: { ...config, [key]: value } });
  };

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
      <div>
        <label style={{ display: 'block', color: '#999', fontSize: 11, marginBottom: 4 }}>合并类型</label>
        <Select
          value={config.merge_type}
          onChange={(value) => handleConfigChange('merge_type', value)}
          size="small"
          style={{ width: '100%' }}
          options={[
            { value: 'all', label: '全部 - 等待所有输入' },
            { value: 'first', label: '首个 - 使用第一个完成的输入' },
            { value: 'last', label: '最后 - 使用最后一个完成的输入' },
          ]}
        />
      </div>

      <div>
        <label style={{ display: 'block', color: '#999', fontSize: 11, marginBottom: 4 }}>
          输入数量 ({config.inputs?.length || 0})
        </label>
        <div style={{ color: '#666', fontSize: 11 }}>
          从画布上连接节点到此节点的输入端口
        </div>
      </div>

      <Divider style={{ margin: '8px 0', borderColor: '#333' }} />

      <div style={{ borderTop: '1px solid #333', paddingTop: 12, marginTop: 4 }}>
        <BasePropertyPanel node={node} onUpdate={onUpdate} onDelete={onDelete} />
      </div>
    </div>
  );
};

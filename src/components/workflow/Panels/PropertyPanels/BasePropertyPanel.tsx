import React from 'react';
import { Input, Switch, InputNumber, Select, Divider } from 'antd';
import type { WorkflowNode } from '../../types';

interface BasePropertyPanelProps {
  node: WorkflowNode;
  onUpdate: (updates: Partial<WorkflowNode>) => void;
  onDelete: () => void;
}

export const BasePropertyPanel: React.FC<BasePropertyPanelProps> = ({ node, onUpdate }) => {
  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
      <div>
        <label style={{ display: 'block', color: '#999', fontSize: 11, marginBottom: 4 }}>标题</label>
        <Input
          value={node.title}
          onChange={(e) => onUpdate({ title: e.target.value })}
          size="small"
        />
      </div>

      <div>
        <label style={{ display: 'block', color: '#999', fontSize: 11, marginBottom: 4 }}>描述</label>
        <Input.TextArea
          value={node.description || ''}
          onChange={(e) => onUpdate({ description: e.target.value })}
          rows={2}
          size="small"
        />
      </div>

      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
        <label style={{ color: '#999', fontSize: 11 }}>启用</label>
        <Switch
          size="small"
          checked={node.enabled}
          onChange={(checked) => onUpdate({ enabled: checked })}
        />
      </div>

      <Divider style={{ margin: '8px 0', borderColor: '#333' }} />

      <div>
        <label style={{ display: 'block', color: '#999', fontSize: 11, marginBottom: 4 }}>重试策略</label>
        <Switch
          size="small"
          checked={node.retry.enabled}
          onChange={(enabled) => onUpdate({ retry: { ...node.retry, enabled } })}
        />
        {node.retry.enabled && (
          <div style={{ marginTop: 8, display: 'flex', flexDirection: 'column', gap: 8 }}>
            <div>
              <label style={{ color: '#666', fontSize: 10 }}>最大重试次数</label>
              <InputNumber
                value={node.retry.max_retries}
                onChange={(value) => onUpdate({ retry: { ...node.retry, max_retries: value || 3 } })}
                min={1}
                max={10}
                size="small"
                style={{ width: '100%' }}
              />
            </div>
            <div>
              <label style={{ color: '#666', fontSize: 10 }}>退避策略</label>
              <Select
                value={node.retry.backoff_type}
                onChange={(backoff_type) => onUpdate({ retry: { ...node.retry, backoff_type } })}
                size="small"
                style={{ width: '100%' }}
                options={[
                  { value: 'Linear', label: '线性' },
                  { value: 'Exponential', label: '指数' },
                  { value: 'Fixed', label: '固定' },
                ]}
              />
            </div>
            <div>
              <label style={{ color: '#666', fontSize: 10 }}>基础延迟 (ms)</label>
              <InputNumber
                value={node.retry.base_delay_ms}
                onChange={(value) => onUpdate({ retry: { ...node.retry, base_delay_ms: value || 1000 } })}
                min={100}
                max={60000}
                size="small"
                style={{ width: '100%' }}
              />
            </div>
          </div>
        )}
      </div>

      <Divider style={{ margin: '8px 0', borderColor: '#333' }} />

      <div>
        <label style={{ display: 'block', color: '#999', fontSize: 11, marginBottom: 4 }}>超时 (秒)</label>
        <InputNumber
          value={node.timeout}
          onChange={(value) => onUpdate({ timeout: value ?? undefined })}
          min={1}
          placeholder="不设置"
          size="small"
          style={{ width: '100%' }}
        />
      </div>
    </div>
  );
};

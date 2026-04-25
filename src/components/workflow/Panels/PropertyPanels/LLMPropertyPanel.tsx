import React from 'react';
import { Input, Select, InputNumber } from 'antd';
import type { WorkflowNode, LLMNode } from '../../types';
import { BasePropertyPanel } from './BasePropertyPanel';

interface LLMPropertyPanelProps {
  node: WorkflowNode;
  onUpdate: (updates: Partial<WorkflowNode>) => void;
  onDelete: () => void;
}

export const LLMPropertyPanel: React.FC<LLMPropertyPanelProps> = ({ node, onUpdate, onDelete }) => {
  const llmNode = node as LLMNode;
  const config = llmNode.config || {
    model: '',
    prompt: '',
    temperature: 0.7,
    max_tokens: 2048,
  };

  const handleConfigChange = (key: string, value: unknown) => {
    onUpdate({ config: { ...config, [key]: value } });
  };

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
      <div>
        <label style={{ display: 'block', color: '#999', fontSize: 11, marginBottom: 4 }}>模型</label>
        <Select
          value={config.model || 'gpt-4'}
          onChange={(value) => handleConfigChange('model', value)}
          size="small"
          style={{ width: '100%' }}
          showSearch
          options={[
            { value: 'gpt-4', label: 'GPT-4' },
            { value: 'gpt-4-turbo', label: 'GPT-4 Turbo' },
            { value: 'gpt-4-32k', label: 'GPT-4 32K' },
            { value: 'gpt-3.5-turbo', label: 'GPT-3.5 Turbo' },
            { value: 'gpt-3.5-turbo-16k', label: 'GPT-3.5 Turbo 16K' },
            { value: 'claude-3-opus', label: 'Claude 3 Opus' },
            { value: 'claude-3-sonnet', label: 'Claude 3 Sonnet' },
            { value: 'claude-3-haiku', label: 'Claude 3 Haiku' },
            { value: 'gemini-pro', label: 'Gemini Pro' },
            { value: 'gemini-ultra', label: 'Gemini Ultra' },
            { value: 'llama-3-70b', label: 'Llama 3 70B' },
            { value: 'llama-3-8b', label: 'Llama 3 8B' },
            { value: 'mistral-7b', label: 'Mistral 7B' },
            { value: 'mixtral-8x7b', label: 'Mixtral 8x7B' },
            { value: 'deepseek-chat', label: 'DeepSeek Chat' },
            { value: 'qwen-72b', label: 'Qwen 72B' },
          ]}
        />
      </div>

      <div>
        <label style={{ display: 'block', color: '#999', fontSize: 11, marginBottom: 4 }}>提示词</label>
        <Input.TextArea
          value={config.prompt || ''}
          onChange={(e) => handleConfigChange('prompt', e.target.value)}
          rows={5}
          size="small"
          placeholder="输入你的提示词..."
        />
      </div>

      <div style={{ display: 'flex', gap: 8 }}>
        <div style={{ flex: 1 }}>
          <label style={{ display: 'block', color: '#999', fontSize: 11, marginBottom: 4 }}>温度</label>
          <InputNumber
            value={config.temperature ?? 0.7}
            onChange={(value) => handleConfigChange('temperature', value)}
            min={0}
            max={2}
            step={0.1}
            size="small"
            style={{ width: '100%' }}
          />
          <div style={{ fontSize: 9, color: '#666', marginTop: 2 }}>
            0: 确定性 ↑ | 2: 随机性 ↑
          </div>
        </div>
        <div style={{ flex: 1 }}>
          <label style={{ display: 'block', color: '#999', fontSize: 11, marginBottom: 4 }}>最大 Token</label>
          <InputNumber
            value={config.max_tokens ?? 2048}
            onChange={(value) => handleConfigChange('max_tokens', value)}
            min={100}
            max={128000}
            step={100}
            size="small"
            style={{ width: '100%' }}
          />
        </div>
      </div>

      <div style={{ borderTop: '1px solid #333', paddingTop: 12, marginTop: 4 }}>
        <BasePropertyPanel node={node} onUpdate={onUpdate} onDelete={onDelete} />
      </div>
    </div>
  );
};

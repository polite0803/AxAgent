import React from 'react';
import { Input, Select, InputNumber, Divider } from 'antd';
import type { WorkflowNode, AgentNode, AgentRole, OutputMode } from '../../types';
import { BasePropertyPanel } from './BasePropertyPanel';

interface AgentPropertyPanelProps {
  node: WorkflowNode;
  onUpdate: (updates: Partial<WorkflowNode>) => void;
  onDelete: () => void;
}

export const AgentPropertyPanel: React.FC<AgentPropertyPanelProps> = ({ node, onUpdate, onDelete }) => {
  const agentNode = node as AgentNode;
  const config = agentNode.config || {
    role: 'developer' as AgentRole,
    system_prompt: '',
    context_sources: [],
    output_var: '',
    tools: [],
    output_mode: 'text' as OutputMode,
  };

  const handleConfigChange = (key: string, value: unknown) => {
    onUpdate({ config: { ...config, [key]: value } });
  };

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
      <div>
        <label style={{ display: 'block', color: '#999', fontSize: 11, marginBottom: 4 }}>Agent 角色</label>
        <Select
          value={config.role}
          onChange={(value) => handleConfigChange('role', value)}
          size="small"
          style={{ width: '100%' }}
          options={[
            { value: 'researcher', label: '🔍 研究员' },
            { value: 'planner', label: '📋 规划师' },
            { value: 'developer', label: '💻 开发者' },
            { value: 'reviewer', label: '👀 审核员' },
            { value: 'synthesizer', label: '🔬 综合师' },
            { value: 'executor', label: '⚙️ 执行者' },
          ]}
        />
      </div>

      <div>
        <label style={{ display: 'block', color: '#999', fontSize: 11, marginBottom: 4 }}>系统提示词</label>
        <Input.TextArea
          value={config.system_prompt || ''}
          onChange={(e) => handleConfigChange('system_prompt', e.target.value)}
          rows={4}
          size="small"
          placeholder="定义 Agent 的行为和能力..."
        />
      </div>

      <div>
        <label style={{ display: 'block', color: '#999', fontSize: 11, marginBottom: 4 }}>模型</label>
        <Select
          value={config.model || 'gpt-4'}
          onChange={(value) => handleConfigChange('model', value)}
          size="small"
          style={{ width: '100%' }}
          options={[
            { value: 'gpt-4', label: 'GPT-4' },
            { value: 'gpt-4-turbo', label: 'GPT-4 Turbo' },
            { value: 'gpt-3.5-turbo', label: 'GPT-3.5 Turbo' },
            { value: 'claude-3-opus', label: 'Claude 3 Opus' },
            { value: 'claude-3-sonnet', label: 'Claude 3 Sonnet' },
            { value: 'claude-3-haiku', label: 'Claude 3 Haiku' },
            { value: 'gemini-pro', label: 'Gemini Pro' },
          ]}
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

      <div>
        <label style={{ display: 'block', color: '#999', fontSize: 11, marginBottom: 4 }}>输出模式</label>
        <Select
          value={config.output_mode}
          onChange={(value) => handleConfigChange('output_mode', value)}
          size="small"
          style={{ width: '100%' }}
          options={[
            { value: 'text', label: '📝 文本' },
            { value: 'json', label: '{} JSON' },
            { value: 'artifact', label: '🎨 工件' },
          ]}
        />
      </div>

      <div>
        <label style={{ display: 'block', color: '#999', fontSize: 11, marginBottom: 4 }}>输出变量</label>
        <Input
          value={config.output_var || ''}
          onChange={(e) => handleConfigChange('output_var', e.target.value)}
          size="small"
          placeholder="agent_output"
        />
      </div>

      <Divider style={{ margin: '8px 0', borderColor: '#333' }} />

      <div>
        <label style={{ display: 'block', color: '#999', fontSize: 11, marginBottom: 4 }}>
          工具 ({config.tools?.length || 0})
        </label>
        <Select
          mode="multiple"
          value={config.tools || []}
          onChange={(value) => handleConfigChange('tools', value)}
          size="small"
          style={{ width: '100%' }}
          placeholder="选择工具..."
          options={[
            { value: 'web_search', label: '🌐 网页搜索' },
            { value: 'web_fetch', label: '📄 网页抓取' },
            { value: 'file_read', label: '📖 文件读取' },
            { value: 'file_write', label: '📝 文件写入' },
            { value: 'code_interpreter', label: '💻 代码执行' },
            { value: 'image_generation', label: '🎨 图像生成' },
          ]}
        />
      </div>

      <div>
        <label style={{ display: 'block', color: '#999', fontSize: 11, marginBottom: 4 }}>
          上下文源 ({config.context_sources?.length || 0})
        </label>
        <Select
          mode="multiple"
          value={config.context_sources || []}
          onChange={(value) => handleConfigChange('context_sources', value)}
          size="small"
          style={{ width: '100%' }}
          placeholder="选择上下文源..."
          options={[
            { value: 'conversation_history', label: '💬 对话历史' },
            { value: 'knowledge_base', label: '📚 知识库' },
            { value: 'document', label: '📄 文档' },
            { value: 'database', label: '🗄️ 数据库' },
          ]}
        />
      </div>

      <div style={{ borderTop: '1px solid #333', paddingTop: 12, marginTop: 4 }}>
        <BasePropertyPanel node={node} onUpdate={onUpdate} onDelete={onDelete} />
      </div>
    </div>
  );
};

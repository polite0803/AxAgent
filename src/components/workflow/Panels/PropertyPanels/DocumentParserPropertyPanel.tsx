import React from 'react';
import { Input, Select, Divider } from 'antd';
import type { WorkflowNode, DocumentParserNode } from '../../types';
import { BasePropertyPanel } from './BasePropertyPanel';

interface DocumentParserPropertyPanelProps {
  node: WorkflowNode;
  onUpdate: (updates: Partial<WorkflowNode>) => void;
  onDelete: () => void;
}

const PARSER_TYPE_OPTIONS = [
  { value: 'pdf', label: '📄 PDF' },
  { value: 'markdown', label: '📝 Markdown' },
  { value: 'html', label: '🌐 HTML' },
  { value: 'json', label: '{} JSON' },
  { value: 'xml', label: '📋 XML' },
  { value: 'csv', label: '📊 CSV' },
  { value: 'text', label: '📃 纯文本' },
];

export const DocumentParserPropertyPanel: React.FC<DocumentParserPropertyPanelProps> = ({ node, onUpdate, onDelete }) => {
  const documentParserNode = node as DocumentParserNode;
  const config = documentParserNode.config || {
    input_var: '',
    parser_type: 'text',
    output_var: '',
  };

  const handleConfigChange = (key: string, value: unknown) => {
    onUpdate({ config: { ...config, [key]: value } });
  };

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
      <div>
        <label style={{ display: 'block', color: '#999', fontSize: 11, marginBottom: 4 }}>输入变量</label>
        <Input
          value={config.input_var || ''}
          onChange={(e) => handleConfigChange('input_var', e.target.value)}
          size="small"
          placeholder="document"
        />
      </div>

      <div>
        <label style={{ display: 'block', color: '#999', fontSize: 11, marginBottom: 4 }}>解析类型</label>
        <Select
          value={config.parser_type}
          onChange={(value) => handleConfigChange('parser_type', value)}
          size="small"
          style={{ width: '100%' }}
          options={PARSER_TYPE_OPTIONS}
        />
      </div>

      <div>
        <label style={{ display: 'block', color: '#999', fontSize: 11, marginBottom: 4 }}>输出变量</label>
        <Input
          value={config.output_var || ''}
          onChange={(e) => handleConfigChange('output_var', e.target.value)}
          size="small"
          placeholder="parsed_content"
        />
      </div>

      <Divider style={{ margin: '8px 0', borderColor: '#333' }} />

      <div style={{ borderTop: '1px solid #333', paddingTop: 12, marginTop: 4 }}>
        <BasePropertyPanel node={node} onUpdate={onUpdate} onDelete={onDelete} />
      </div>
    </div>
  );
};

import React from 'react';
import { Descriptions, Tag, Typography } from 'antd';
import type { ToolDependency } from '../../types';

const { Text, Paragraph } = Typography;

interface ToolGenerationPreviewProps {
  dependency: ToolDependency;
}

export const ToolGenerationPreview: React.FC<ToolGenerationPreviewProps> = ({ dependency }) => {
  return (
    <div style={{ padding: '12px 0' }}>
      <Descriptions
        size="small"
        column={1}
        bordered
        items={[
          {
            key: 'name',
            label: '工具名称',
            children: <Text code>generated_{dependency.name.replace(/[^a-zA-Z0-9]/g, '_')}</Text>,
          },
          {
            key: 'original',
            label: '原始名称',
            children: dependency.name,
          },
          {
            key: 'type',
            label: '实现方式',
            children: <Tag color="blue">Prompt 模板</Tag>,
          },
          {
            key: 'description',
            label: '说明',
            children: (
              <Paragraph type="secondary" style={{ fontSize: 12, marginBottom: 0 }}>
                将通过 Developer Agent 分析工具需求，生成 Prompt 模板作为工具实现。
                模板中包含 {'{{input}}'} 占位符，运行时替换为实际输入后调用 LLM 执行。
              </Paragraph>
            ),
          },
        ]}
      />
    </div>
  );
};

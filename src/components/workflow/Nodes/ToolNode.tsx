import React, { memo } from 'react';
import { Handle, Position, type NodeProps } from 'reactflow';
import { Tag } from 'antd';

interface ToolNodeData {
  id: string;
  type: string;
  title: string;
  description?: string;
  color: string;
  nodeType: string;
  enabled: boolean;
  toolName?: string;
  inputMapping?: Record<string, string>;
  outputVar?: string;
}

const ToolNodeComponent: React.FC<NodeProps<ToolNodeData>> = ({ data, selected }) => {
  const color = '#52c41a';
  const toolName = data.toolName || '未选择工具';
  const inputMapping = data.inputMapping || {};
  const outputVar = data.outputVar;

  const inputCount = Object.keys(inputMapping).length;

  return (
    <div
      style={{
        minWidth: 180,
        maxWidth: 220,
        opacity: data.enabled ? 1 : 0.5,
        filter: data.enabled ? 'none' : 'grayscale(100%)',
      }}
    >
      <div
        style={{
          background: '#1e1e1e',
          border: `2px solid ${selected ? '#1890ff' : color}`,
          borderRadius: 8,
          overflow: 'hidden',
          boxShadow: selected ? `0 0 0 2px ${color}40` : 'none',
          transition: 'all 0.2s',
        }}
      >
        <div
          style={{
            padding: '8px 12px',
            borderBottom: `1px solid ${color}30`,
            display: 'flex',
            alignItems: 'center',
            gap: 8,
            background: `${color}15`,
          }}
        >
          <span style={{ fontSize: 14 }}>🔧</span>
          <span
            style={{
              fontSize: 11,
              color: color,
              fontWeight: 600,
            }}
          >
            工具
          </span>
        </div>

        <div style={{ padding: '10px 12px' }}>
          <div
            style={{
              fontSize: 13,
              color: '#fff',
              fontWeight: 500,
              marginBottom: 6,
              overflow: 'hidden',
              textOverflow: 'ellipsis',
              whiteSpace: 'nowrap',
            }}
          >
            {data.title}
          </div>

          <div
            style={{
              fontSize: 11,
              color: color,
              marginBottom: 6,
              padding: '4px 6px',
              background: `${color}15`,
              borderRadius: 4,
              overflow: 'hidden',
              textOverflow: 'ellipsis',
              whiteSpace: 'nowrap',
              fontWeight: 500,
            }}
          >
            {toolName}
          </div>

          <div style={{ display: 'flex', flexWrap: 'wrap', gap: 4 }}>
            {inputCount > 0 && (
              <Tag
                style={{
                  margin: 0,
                  fontSize: 9,
                  padding: '0 4px',
                  background: '#252525',
                  border: '1px solid #444',
                  color: '#aaa',
                }}
              >
                📥 {inputCount} 输入
              </Tag>
            )}

            {outputVar && (
              <Tag
                style={{
                  margin: 0,
                  fontSize: 9,
                  padding: '0 4px',
                  background: '#1890ff20',
                  border: '1px solid #1890ff50',
                  color: '#1890ff',
                }}
              >
                📤 {outputVar}
              </Tag>
            )}
          </div>
        </div>
      </div>

      <Handle
        type="target"
        position={Position.Top}
        style={{
          background: color,
          border: 'none',
          width: 8,
          height: 8,
        }}
      />

      <Handle
        type="source"
        position={Position.Bottom}
        style={{
          background: color,
          border: 'none',
          width: 8,
          height: 8,
        }}
      />
    </div>
  );
};

export const ToolNode = memo(ToolNodeComponent);

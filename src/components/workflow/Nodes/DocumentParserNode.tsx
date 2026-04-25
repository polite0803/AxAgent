import React, { memo } from 'react';
import { Handle, Position, type NodeProps } from 'reactflow';
import { Tag } from 'antd';

interface DocumentParserNodeData {
  id: string;
  type: string;
  title: string;
  description?: string;
  color: string;
  nodeType: string;
  enabled: boolean;
  inputVar?: string;
  parserType?: string;
  outputVar?: string;
}

const DocumentParserNodeComponent: React.FC<NodeProps<DocumentParserNodeData>> = ({ data, selected }) => {
  const color = '#eb2f96';
  const inputVar = data.inputVar || '未设置';
  const parserType = data.parserType || '未选择';
  const outputVar = data.outputVar;

  const getParserTypeIcon = (type: string): string => {
    const icons: Record<string, string> = {
      pdf: '📄',
      markdown: '📝',
      html: '🌐',
      json: '{}',
      xml: '📋',
      csv: '📊',
      text: '📃',
    };
    return icons[type.toLowerCase()] || '📄';
  };

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
          <span style={{ fontSize: 14 }}>📄</span>
          <span
            style={{
              fontSize: 11,
              color: color,
              fontWeight: 600,
            }}
          >
            文档解析
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

          <div style={{ display: 'flex', flexDirection: 'column', gap: 4, marginBottom: 6 }}>
            <div
              style={{
                fontSize: 10,
                color: '#888',
                padding: '3px 6px',
                background: '#252525',
                borderRadius: 4,
                overflow: 'hidden',
                textOverflow: 'ellipsis',
                whiteSpace: 'nowrap',
              }}
            >
              📥 {inputVar}
            </div>
            <div
              style={{
                fontSize: 10,
                color: color,
                padding: '3px 6px',
                background: `${color}15`,
                borderRadius: 4,
                fontWeight: 500,
              }}
            >
              {getParserTypeIcon(parserType)} {parserType}
            </div>
          </div>

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

export const DocumentParserNode = memo(DocumentParserNodeComponent);

import React, { memo } from 'react';
import { Handle, Position, type NodeProps } from 'reactflow';
import { Tag } from 'antd';

interface LoopNodeData {
  id: string;
  type: string;
  title: string;
  description?: string;
  color: string;
  nodeType: string;
  enabled: boolean;
  loopType?: 'forEach' | 'while' | 'doWhile' | 'until';
  maxIterations?: number;
  continueOnError?: boolean;
  bodySteps?: string[];
  itemsVar?: string;
}

const LoopNodeComponent: React.FC<NodeProps<LoopNodeData>> = ({ data, selected }) => {
  const color = '#fa8c16';
  const loopType = data.loopType || 'forEach';
  const maxIterations = data.maxIterations || 100;
  const bodySteps = data.bodySteps || [];

  const getLoopTypeIcon = (type: string): string => {
    switch (type) {
      case 'forEach': return '🔁';
      case 'while': return '⏳';
      case 'doWhile': return '↻';
      case 'until': return '🔚';
      default: return '🔁';
    }
  };

  const getLoopTypeLabel = (type: string): string => {
    const labels: Record<string, string> = {
      forEach: '遍历',
      while: '当...时',
      doWhile: '执行...直到',
      until: '直到...',
    };
    return labels[type] || type;
  };

  return (
    <div
      style={{
        minWidth: 200,
        maxWidth: 240,
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
          <span style={{ fontSize: 14 }}>{getLoopTypeIcon(loopType)}</span>
          <span
            style={{
              fontSize: 11,
              color: color,
              fontWeight: 600,
            }}
          >
            循环 · {getLoopTypeLabel(loopType)}
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

          {data.itemsVar && (
            <div
              style={{
                fontSize: 10,
                color: '#888',
                marginBottom: 6,
                padding: '4px 6px',
                background: '#252525',
                borderRadius: 4,
                overflow: 'hidden',
                textOverflow: 'ellipsis',
                whiteSpace: 'nowrap',
              }}
            >
              📋 {data.itemsVar}
            </div>
          )}

          <div style={{ display: 'flex', flexWrap: 'wrap', gap: 4 }}>
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
              最多 {maxIterations} 次
            </Tag>

            {data.continueOnError && (
              <Tag
                style={{
                  margin: 0,
                  fontSize: 9,
                  padding: '0 4px',
                  background: '#fa8c1620',
                  border: '1px solid #fa8c1650',
                  color: '#fa8c16',
                }}
              >
                容错
              </Tag>
            )}

            {bodySteps.length > 0 && (
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
                📝 {bodySteps.length} 步
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
        id="loop-end"
        style={{
          background: color,
          border: 'none',
          width: 8,
          height: 8,
        }}
      />

      <Handle
        type="source"
        position={Position.Right}
        id="loop-body"
        style={{
          background: '#52c41a',
          border: 'none',
          width: 6,
          height: 6,
          top: '50%',
        }}
      />

      <div
        style={{
          position: 'absolute',
          left: -10,
          top: '50%',
          transform: 'translateY(-50%)',
          width: 0,
          height: 0,
          borderTop: '6px solid transparent',
          borderBottom: '6px solid transparent',
          borderRight: `8px solid ${color}`,
        }}
      />
    </div>
  );
};

export const LoopNode = memo(LoopNodeComponent);

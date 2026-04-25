import React, { memo } from 'react';
import { Handle, Position, type NodeProps } from 'reactflow';
import { Tag } from 'antd';

interface ConditionNodeData {
  id: string;
  type: string;
  title: string;
  description?: string;
  color: string;
  nodeType: string;
  enabled: boolean;
  conditions?: Array<{
    var_path: string;
    operator: string;
    value: unknown;
  }>;
  logicalOp?: 'and' | 'or';
}

const ConditionNodeComponent: React.FC<NodeProps<ConditionNodeData>> = ({ data, selected }) => {
  const color = '#fa8c16';
  const conditions = data.conditions || [];
  const logicalOp = data.logicalOp || 'and';

  const getOperatorLabel = (op: string): string => {
    const labels: Record<string, string> = {
      eq: '=',
      ne: '≠',
      gt: '>',
      lt: '<',
      gte: '≥',
      lte: '≤',
      contains: '包含',
      notContains: '不包含',
      startsWith: '开头',
      endsWith: '结尾',
      regexMatch: '正则',
      isEmpty: '为空',
      isNotEmpty: '不为空',
    };
    return labels[op] || op;
  };

  const formatValue = (value: unknown): string => {
    if (value === null || value === undefined) return '';
    if (typeof value === 'string') return value.length > 10 ? `${value.slice(0, 10)}...` : value;
    if (typeof value === 'number') return String(value);
    return JSON.stringify(value).slice(0, 10);
  };

  return (
    <div
      style={{
        minWidth: 200,
        maxWidth: 260,
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
          <span style={{ fontSize: 14 }}>🔀</span>
          <span
            style={{
              fontSize: 11,
              color: color,
              fontWeight: 600,
            }}
          >
            条件分支
          </span>
          <Tag
            style={{
              margin: 0,
              fontSize: 9,
              padding: '0 4px',
              background: `${color}30`,
              border: 'none',
              color: '#fff',
            }}
          >
            {logicalOp.toUpperCase()}
          </Tag>
        </div>

        <div style={{ padding: '10px 12px' }}>
          <div
            style={{
              fontSize: 13,
              color: '#fff',
              fontWeight: 500,
              marginBottom: 8,
              overflow: 'hidden',
              textOverflow: 'ellipsis',
              whiteSpace: 'nowrap',
            }}
          >
            {data.title}
          </div>

          {conditions.length > 0 ? (
            <div style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
              {conditions.slice(0, 3).map((condition, index) => (
                <div
                  key={index}
                  style={{
                    fontSize: 10,
                    color: '#aaa',
                    padding: '4px 6px',
                    background: '#252525',
                    borderRadius: 4,
                    display: 'flex',
                    alignItems: 'center',
                    gap: 4,
                    overflow: 'hidden',
                  }}
                >
                  <span
                    style={{
                      color: '#888',
                      overflow: 'hidden',
                      textOverflow: 'ellipsis',
                      whiteSpace: 'nowrap',
                      flex: 1,
                    }}
                  >
                    {condition.var_path || '未设置'}
                  </span>
                  <span style={{ color: color, fontWeight: 500 }}>
                    {getOperatorLabel(condition.operator)}
                  </span>
                  <span
                    style={{
                      color: '#52c41a',
                      overflow: 'hidden',
                      textOverflow: 'ellipsis',
                      whiteSpace: 'nowrap',
                      maxWidth: 60,
                    }}
                  >
                    {formatValue(condition.value)}
                  </span>
                </div>
              ))}
              {conditions.length > 3 && (
                <div
                  style={{
                    fontSize: 9,
                    color: '#666',
                    textAlign: 'center',
                  }}
                >
                  +{conditions.length - 3} 更多条件
                </div>
              )}
            </div>
          ) : (
            <div
              style={{
                fontSize: 10,
                color: '#666',
                textAlign: 'center',
                padding: 8,
                background: '#252525',
                borderRadius: 4,
              }}
            >
              点击编辑条件
            </div>
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

      <div
        style={{
          position: 'absolute',
          right: -10,
          top: '50%',
          transform: 'translateY(-50%)',
          width: 0,
          height: 0,
          borderTop: '6px solid transparent',
          borderBottom: '6px solid transparent',
          borderLeft: `8px solid ${color}`,
        }}
      />

      <div style={{ display: 'flex', justifyContent: 'space-around', marginTop: 4 }}>
        <Tag color="green" style={{ margin: 0, fontSize: 9 }}>
          ✅ 真
        </Tag>
        <Tag color="red" style={{ margin: 0, fontSize: 9 }}>
          ❌ 假
        </Tag>
      </div>
    </div>
  );
};

export const ConditionNode = memo(ConditionNodeComponent);

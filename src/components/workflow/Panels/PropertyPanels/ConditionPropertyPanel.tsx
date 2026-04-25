import React from 'react';
import { Input, Select, Button, Divider } from 'antd';
import { Plus, Trash2 } from 'lucide-react';
import type { WorkflowNode, ConditionNode, Condition, CompareOperator, LogicalOperator } from '../../types';
import { BasePropertyPanel } from './BasePropertyPanel';

interface ConditionPropertyPanelProps {
  node: WorkflowNode;
  onUpdate: (updates: Partial<WorkflowNode>) => void;
  onDelete: () => void;
}

const OPERATOR_OPTIONS: { value: CompareOperator; label: string }[] = [
  { value: 'eq', label: '=' },
  { value: 'ne', label: '≠' },
  { value: 'gt', label: '>' },
  { value: 'lt', label: '<' },
  { value: 'gte', label: '≥' },
  { value: 'lte', label: '≤' },
  { value: 'contains', label: '包含' },
  { value: 'notContains', label: '不包含' },
  { value: 'startsWith', label: '开头是' },
  { value: 'endsWith', label: '结尾是' },
  { value: 'regexMatch', label: '正则匹配' },
  { value: 'isEmpty', label: '为空' },
  { value: 'isNotEmpty', label: '不为空' },
];

export const ConditionPropertyPanel: React.FC<ConditionPropertyPanelProps> = ({ node, onUpdate, onDelete }) => {
  const conditionNode = node as ConditionNode;
  const config = conditionNode.config || {
    conditions: [],
    logical_op: 'and' as LogicalOperator,
  };

  const handleAddCondition = () => {
    const newCondition: Condition = {
      var_path: '',
      operator: 'eq',
      value: '',
    };
    onUpdate({
      config: {
        ...config,
        conditions: [...config.conditions, newCondition],
      },
    });
  };

  const handleUpdateCondition = (index: number, updates: Partial<Condition>) => {
    const newConditions = [...config.conditions];
    newConditions[index] = { ...newConditions[index], ...updates };
    onUpdate({
      config: {
        ...config,
        conditions: newConditions,
      },
    });
  };

  const handleDeleteCondition = (index: number) => {
    const newConditions = config.conditions.filter((_, i) => i !== index);
    onUpdate({
      config: {
        ...config,
        conditions: newConditions,
      },
    });
  };

  const handleLogicalOpChange = (logical_op: LogicalOperator) => {
    onUpdate({
      config: {
        ...config,
        logical_op,
      },
    });
  };

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
      <div>
        <label style={{ display: 'block', color: '#999', fontSize: 11, marginBottom: 4 }}>逻辑操作</label>
        <Select
          value={config.logical_op}
          onChange={handleLogicalOpChange}
          size="small"
          style={{ width: '100%' }}
          options={[
            { value: 'and', label: 'AND - 所有条件都为真' },
            { value: 'or', label: 'OR - 任一条件为真' },
          ]}
        />
      </div>

      <div>
        <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: 8 }}>
          <label style={{ color: '#999', fontSize: 11 }}>
            条件 ({config.conditions.length})
          </label>
          <Button
            type="dashed"
            size="small"
            icon={<Plus size={12} />}
            onClick={handleAddCondition}
          >
            添加条件
          </Button>
        </div>

        <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
          {config.conditions.map((condition, index) => (
            <div
              key={index}
              style={{
                padding: 8,
                background: '#1e1e1e',
                borderRadius: 6,
                border: '1px solid #333',
              }}
            >
              <div style={{ marginBottom: 8 }}>
                <Input
                  value={condition.var_path}
                  onChange={(e) => handleUpdateCondition(index, { var_path: e.target.value })}
                  size="small"
                  placeholder="变量路径 (如: input.status)"
                />
              </div>

              <div style={{ display: 'flex', gap: 4, marginBottom: condition.operator === 'isEmpty' || condition.operator === 'isNotEmpty' ? 0 : 8 }}>
                <Select
                  value={condition.operator}
                  onChange={(value) => handleUpdateCondition(index, { operator: value })}
                  size="small"
                  style={{ flex: 1 }}
                  options={OPERATOR_OPTIONS}
                />

                {condition.operator !== 'isEmpty' && condition.operator !== 'isNotEmpty' && (
                  <Input
                    value={String(condition.value || '')}
                    onChange={(e) => handleUpdateCondition(index, { value: e.target.value })}
                    size="small"
                    placeholder="值"
                    style={{ flex: 1 }}
                  />
                )}

                <Button
                  type="text"
                  danger
                  size="small"
                  icon={<Trash2 size={12} />}
                  onClick={() => handleDeleteCondition(index)}
                />
              </div>
            </div>
          ))}

          {config.conditions.length === 0 && (
            <div style={{ color: '#666', fontSize: 11, textAlign: 'center', padding: 16 }}>
              点击"添加条件"创建第一个条件
            </div>
          )}
        </div>
      </div>

      <Divider style={{ margin: '8px 0', borderColor: '#333' }} />

      <div style={{ borderTop: '1px solid #333', paddingTop: 12, marginTop: 4 }}>
        <BasePropertyPanel node={node} onUpdate={onUpdate} onDelete={onDelete} />
      </div>
    </div>
  );
};

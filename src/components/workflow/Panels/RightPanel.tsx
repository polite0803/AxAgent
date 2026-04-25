import React from 'react';
import { Tabs, Input, Select, Button, Divider } from 'antd';
import { Trash2 } from 'lucide-react';
import { useWorkflowEditorStore } from '@/stores';
import { NODE_TYPE_MAP, type WorkflowNode, type WorkflowEdge } from '../types';
import {
  TriggerPropertyPanel,
  AgentPropertyPanel,
  LLMPropertyPanel,
  ConditionPropertyPanel,
  ParallelPropertyPanel,
  LoopPropertyPanel,
  MergePropertyPanel,
  DelayPropertyPanel,
  AtomicSkillPropertyPanel,
  ToolPropertyPanel,
  CodePropertyPanel,
  SubWorkflowPropertyPanel,
  DocumentParserPropertyPanel,
  VectorRetrievePropertyPanel,
  EndPropertyPanel,
} from './PropertyPanels';

interface RightPanelProps {
  selectedNode: WorkflowNode | null;
  selectedEdge: WorkflowEdge | null;
}

export const RightPanel: React.FC<RightPanelProps> = React.memo(({ selectedNode, selectedEdge }) => {
  const deleteNode = useWorkflowEditorStore((state) => state.deleteNode);
  const deleteEdge = useWorkflowEditorStore((state) => state.deleteEdge);
  const updateNode = useWorkflowEditorStore((state) => state.updateNode);
  const currentTemplate = useWorkflowEditorStore((state) => state.currentTemplate);

  const handleUpdateNode = (updates: Partial<WorkflowNode>) => {
    if (selectedNode) {
      updateNode(selectedNode.id, updates);
    }
  };

  const handleDeleteNode = () => {
    if (selectedNode) {
      deleteNode(selectedNode.id);
    }
  };

  const renderNodeProperties = () => {
    if (!selectedNode) {
      return (
        <div style={{ padding: 20, textAlign: 'center', color: '#666' }}>
          选择一个节点以编辑属性
        </div>
      );
    }

    const nodeTypeInfo = NODE_TYPE_MAP[selectedNode.type] || { label: selectedNode.type, color: '#999' };

    const renderPropertyPanel = () => {
      switch (selectedNode.type) {
        case 'trigger':
          return (
            <TriggerPropertyPanel
              node={selectedNode}
              onUpdate={handleUpdateNode}
              onDelete={handleDeleteNode}
            />
          );
        case 'agent':
          return (
            <AgentPropertyPanel
              node={selectedNode}
              onUpdate={handleUpdateNode}
              onDelete={handleDeleteNode}
            />
          );
        case 'llm':
          return (
            <LLMPropertyPanel
              node={selectedNode}
              onUpdate={handleUpdateNode}
              onDelete={handleDeleteNode}
            />
          );
        case 'condition':
          return (
            <ConditionPropertyPanel
              node={selectedNode}
              onUpdate={handleUpdateNode}
              onDelete={handleDeleteNode}
            />
          );
        case 'parallel':
          return (
            <ParallelPropertyPanel
              node={selectedNode}
              onUpdate={handleUpdateNode}
              onDelete={handleDeleteNode}
            />
          );
        case 'loop':
          return (
            <LoopPropertyPanel
              node={selectedNode}
              onUpdate={handleUpdateNode}
              onDelete={handleDeleteNode}
            />
          );
        case 'merge':
          return (
            <MergePropertyPanel
              node={selectedNode}
              onUpdate={handleUpdateNode}
              onDelete={handleDeleteNode}
            />
          );
        case 'delay':
          return (
            <DelayPropertyPanel
              node={selectedNode}
              onUpdate={handleUpdateNode}
              onDelete={handleDeleteNode}
            />
          );
        case 'atomicSkill':
          return (
            <AtomicSkillPropertyPanel
              node={selectedNode}
              onUpdate={handleUpdateNode}
              onDelete={handleDeleteNode}
            />
          );
        case 'tool':
          return (
            <ToolPropertyPanel
              node={selectedNode}
              onUpdate={handleUpdateNode}
              onDelete={handleDeleteNode}
            />
          );
        case 'code':
          return (
            <CodePropertyPanel
              node={selectedNode}
              onUpdate={handleUpdateNode}
              onDelete={handleDeleteNode}
            />
          );
        case 'subWorkflow':
          return (
            <SubWorkflowPropertyPanel
              node={selectedNode}
              onUpdate={handleUpdateNode}
              onDelete={handleDeleteNode}
            />
          );
        case 'documentParser':
          return (
            <DocumentParserPropertyPanel
              node={selectedNode}
              onUpdate={handleUpdateNode}
              onDelete={handleDeleteNode}
            />
          );
        case 'vectorRetrieve':
          return (
            <VectorRetrievePropertyPanel
              node={selectedNode}
              onUpdate={handleUpdateNode}
              onDelete={handleDeleteNode}
            />
          );
        case 'end':
          return (
            <EndPropertyPanel
              node={selectedNode}
              onUpdate={handleUpdateNode}
              onDelete={handleDeleteNode}
            />
          );
        default:
          return (
            <div style={{ color: '#666', textAlign: 'center', padding: 20 }}>
              暂不支持此节点类型的编辑
            </div>
          );
      }
    };

    return (
      <div style={{ padding: 12 }}>
        <div style={{ marginBottom: 12 }}>
          <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
            <span style={{ color: '#999', fontSize: 11, textTransform: 'uppercase' }}>节点类型</span>
            <span
              style={{
                background: `${nodeTypeInfo.color}20`,
                color: nodeTypeInfo.color,
                padding: '2px 8px',
                borderRadius: 4,
                fontSize: 11,
                fontWeight: 500,
              }}
            >
              {nodeTypeInfo.label}
            </span>
          </div>
        </div>

        <Divider style={{ margin: '8px 0', borderColor: '#333' }} />

        {renderPropertyPanel()}
      </div>
    );
  };

  const renderEdgeProperties = () => {
    if (!selectedEdge) {
      return (
        <div style={{ padding: 20, textAlign: 'center', color: '#666' }}>
          选择一条边以编辑属性
        </div>
      );
    }

    return (
      <div style={{ padding: 12 }}>
        <div style={{ marginBottom: 12 }}>
          <label style={{ display: 'block', color: '#999', fontSize: 11, marginBottom: 4 }}>边类型</label>
          <Select
            value={selectedEdge.edge_type}
            onChange={(edge_type) =>
              useWorkflowEditorStore.getState().updateEdge(selectedEdge.id, { edge_type })
            }
            size="small"
            style={{ width: '100%' }}
            options={[
              { value: 'direct', label: '直接' },
              { value: 'conditionTrue', label: '条件-真' },
              { value: 'conditionFalse', label: '条件-假' },
              { value: 'loopBack', label: '循环回边' },
              { value: 'parallelBranch', label: '并行分支' },
              { value: 'merge', label: '合并' },
              { value: 'error', label: '错误' },
            ]}
          />
        </div>

        <div style={{ marginBottom: 12 }}>
          <label style={{ display: 'block', color: '#999', fontSize: 11, marginBottom: 4 }}>标签</label>
          <Input
            value={selectedEdge.label || ''}
            onChange={(e) =>
              useWorkflowEditorStore.getState().updateEdge(selectedEdge.id, { label: e.target.value })
            }
            size="small"
            placeholder="边的标签（可选）"
          />
        </div>

        <Divider style={{ margin: '8px 0', borderColor: '#333' }} />

        <Button
          type="text"
          danger
          icon={<Trash2 size={14} />}
          onClick={() => deleteEdge(selectedEdge.id)}
          style={{ width: '100%', justifyContent: 'flex-start' }}
        >
          删除边
        </Button>
      </div>
    );
  };

  const renderTemplateSettings = () => {
    if (!currentTemplate) return null;

    return (
      <div style={{ padding: 12 }}>
        <div style={{ marginBottom: 12 }}>
          <label style={{ display: 'block', color: '#999', fontSize: 11, marginBottom: 4 }}>名称</label>
          <Input
            value={currentTemplate.name}
            size="small"
            onChange={(e) =>
              useWorkflowEditorStore.getState().updateTemplateMetadata({ name: e.target.value })
            }
          />
        </div>

        <div style={{ marginBottom: 12 }}>
          <label style={{ display: 'block', color: '#999', fontSize: 11, marginBottom: 4 }}>描述</label>
          <Input.TextArea
            value={currentTemplate.description || ''}
            rows={3}
            size="small"
            onChange={(e) =>
              useWorkflowEditorStore.getState().updateTemplateMetadata({ description: e.target.value })
            }
          />
        </div>

        <div style={{ marginBottom: 12 }}>
          <label style={{ display: 'block', color: '#999', fontSize: 11, marginBottom: 4 }}>图标</label>
          <Select
            value={currentTemplate.icon}
            size="small"
            style={{ width: '100%' }}
            onChange={(icon) => useWorkflowEditorStore.getState().updateTemplateMetadata({ icon })}
            options={[
              { value: 'Bot', label: '🤖 Bot' },
              { value: 'Code', label: '💻 Code' },
              { value: 'FileText', label: '📄 Document' },
              { value: 'GitBranch', label: '🔀 Git Branch' },
              { value: 'Zap', label: '⚡ Zap' },
              { value: 'Layers', label: '📚 Layers' },
            ]}
          />
        </div>

        <Divider style={{ margin: '8px 0', borderColor: '#333' }} />

        <div>
          <label style={{ display: 'block', color: '#999', fontSize: 11, marginBottom: 4 }}>标签</label>
          <Select
            mode="tags"
            value={currentTemplate.tags || []}
            size="small"
            style={{ width: '100%' }}
            onChange={(tags) => useWorkflowEditorStore.getState().updateTemplateMetadata({ tags })}
            placeholder="添加标签..."
            options={[
              { value: 'ai', label: 'AI' },
              { value: 'automation', label: '自动化' },
              { value: 'workflow', label: '工作流' },
              { value: 'agent', label: 'Agent' },
              { value: 'chatbot', label: '聊天机器人' },
              { value: 'data-processing', label: '数据处理' },
            ]}
          />
        </div>
      </div>
    );
  };

  return (
    <div
      style={{
        width: 320,
        background: '#252525',
        borderLeft: '1px solid #333',
        display: 'flex',
        flexDirection: 'column',
        overflow: 'hidden',
      }}
    >
      <Tabs
        defaultActiveKey="properties"
        size="small"
        style={{ height: '100%' }}
        items={[
          {
            key: 'properties',
            label: '属性',
            children: selectedNode
              ? renderNodeProperties()
              : selectedEdge
              ? renderEdgeProperties()
              : (
                <div style={{ padding: 20, textAlign: 'center', color: '#666' }}>
                  选择节点或边以编辑属性
                </div>
              ),
          },
          {
            key: 'settings',
            label: '设置',
            children: renderTemplateSettings(),
          },
        ]}
      />
    </div>
  );
});

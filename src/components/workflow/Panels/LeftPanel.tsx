import React, { useState } from 'react';
import { Tabs, Input, Tag } from 'antd';
import { Search, FileText } from 'lucide-react';
import { useWorkflowEditorStore } from '@/stores';
import { NODE_CATEGORIES, NODE_TYPE_MAP } from '../types';

export const LeftPanel: React.FC = () => {
  const [search, setSearch] = useState('');
  const { templates, loadTemplate } = useWorkflowEditorStore();

  const handleDragStart = (event: React.DragEvent, nodeType: string, nodeLabel: string) => {
    event.dataTransfer.setData('application/reactflow', JSON.stringify({ type: nodeType, label: nodeLabel }));
    event.dataTransfer.effectAllowed = 'move';
  };

  const filteredNodeTypes = Object.entries(NODE_TYPE_MAP).filter(([_, info]) =>
    info.label.toLowerCase().includes(search.toLowerCase())
  );

  const groupedNodeTypes = NODE_CATEGORIES.map((category) => ({
    ...category,
    items: filteredNodeTypes.filter(([_, info]) => info.category === category.id),
  })).filter((category) => category.items.length > 0);

  const handleTemplateClick = (templateId: string) => {
    loadTemplate(templateId);
  };

  return (
    <div
      style={{
        width: 280,
        background: '#252525',
        borderRight: '1px solid #333',
        display: 'flex',
        flexDirection: 'column',
        overflow: 'hidden',
      }}
    >
      <Tabs
        defaultActiveKey="nodes"
        size="small"
        style={{ height: '100%' }}
        items={[
          {
            key: 'nodes',
            label: '节点',
            children: (
              <div style={{ display: 'flex', flexDirection: 'column', height: '100%' }}>
                <Input
                  prefix={<Search size={14} style={{ color: '#666' }} />}
                  placeholder="搜索节点..."
                  value={search}
                  onChange={(e) => setSearch(e.target.value)}
                  style={{ margin: '8px', width: 'auto' }}
                  size="small"
                />

                <div style={{ flex: 1, overflow: 'auto', padding: '0 8px' }}>
                  {groupedNodeTypes.map((category) => (
                    <div key={category.id} style={{ marginBottom: 12 }}>
                      <div
                        style={{
                          fontSize: 11,
                          color: '#666',
                          textTransform: 'uppercase',
                          marginBottom: 6,
                          paddingLeft: 4,
                        }}
                      >
                        {category.label}
                      </div>
                      <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 6 }}>
                        {category.items.map(([type, info]) => (
                          <div
                            key={type}
                            draggable
                            onDragStart={(e) => handleDragStart(e, type, info.label)}
                            style={{
                              padding: '8px 6px',
                              background: '#1a1a1a',
                              border: `1px solid ${info.color}40`,
                              borderRadius: 6,
                              cursor: 'grab',
                              textAlign: 'center',
                              fontSize: 11,
                              color: '#ccc',
                              transition: 'all 0.2s',
                            }}
                            onMouseEnter={(e) => {
                              e.currentTarget.style.borderColor = info.color;
                              e.currentTarget.style.background = `${info.color}10`;
                            }}
                            onMouseLeave={(e) => {
                              e.currentTarget.style.borderColor = `${info.color}40`;
                              e.currentTarget.style.background = '#1a1a1a';
                            }}
                          >
                            <div style={{ fontSize: 16, marginBottom: 4 }}>{getNodeIcon(type)}</div>
                            <div style={{ whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>
                              {info.label}
                            </div>
                          </div>
                        ))}
                      </div>
                    </div>
                  ))}
                </div>
              </div>
            ),
          },
          {
            key: 'templates',
            label: '模板',
            children: (
              <div style={{ padding: '8px' }}>
                <Input
                  prefix={<Search size={14} style={{ color: '#666' }} />}
                  placeholder="搜索模板..."
                  style={{ marginBottom: 8 }}
                  size="small"
                />
                <div style={{ overflow: 'auto', maxHeight: 'calc(100vh - 200px)' }}>
                  {templates.map((template) => (
                    <div
                      key={template.id}
                      onClick={() => handleTemplateClick(template.id)}
                      style={{
                        padding: 10,
                        marginBottom: 6,
                        background: '#1a1a1a',
                        borderRadius: 6,
                        cursor: 'pointer',
                        border: '1px solid transparent',
                      }}
                      onMouseEnter={(e) => {
                        e.currentTarget.style.borderColor = '#1890ff40';
                      }}
                      onMouseLeave={(e) => {
                        e.currentTarget.style.borderColor = 'transparent';
                      }}
                    >
                      <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                        <FileText size={14} style={{ color: '#1890ff' }} />
                        <span style={{ color: '#ccc', fontSize: 12 }}>{template.name}</span>
                        {template.is_preset && (
                          <Tag color="blue" style={{ fontSize: 10, margin: 0 }}>
                            预设
                          </Tag>
                        )}
                      </div>
                      {template.description && (
                        <div
                          style={{
                            color: '#666',
                            fontSize: 11,
                            marginTop: 4,
                            marginLeft: 22,
                            overflow: 'hidden',
                            textOverflow: 'ellipsis',
                            whiteSpace: 'nowrap',
                          }}
                        >
                          {template.description}
                        </div>
                      )}
                    </div>
                  ))}
                </div>
              </div>
            ),
          },
        ]}
      />
    </div>
  );
};

function getNodeIcon(type: string): string {
  const icons: Record<string, string> = {
    trigger: '⚡',
    agent: '🤖',
    llm: '🧠',
    condition: '❓',
    parallel: '�>||',
    loop: '🔄',
    merge: '⊕',
    delay: '⏱',
    atomicSkill: '⚛️',
    tool: '🔧',
    code: '💻',
    subWorkflow: '📦',
    documentParser: '📄',
    vectorRetrieve: '🔍',
    end: '🏁',
  };
  return icons[type] || '📦';
}

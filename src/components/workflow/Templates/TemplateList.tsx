import React, { useState } from 'react';
import { Card, Input, Select, Button, Tag, Empty, Spin, Dropdown, Modal, message } from 'antd';
import { Search, Plus, MoreVertical, Copy, Trash2, Edit2, Eye, History } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { useWorkflowEditorStore } from '@/stores';
import type { WorkflowTemplateResponse } from '../types';
import { VersionHistoryModal } from './VersionHistoryModal';

/** Maps preset template (kebab-case) IDs to their i18n key suffixes (camelCase). */
const PRESET_I18N_KEY: Record<string, string> = {
  'code-review': 'codeReview',
  'bug-fix': 'bugFix',
  'doc-gen': 'docGen',
  'test-gen': 'testGen',
  'refactor': 'refactor',
  'explore': 'explore',
  'performance': 'performance',
  'security': 'security',
  'migration': 'migration',
  'api-design': 'apiDesign',
  'debug-env': 'debugEnv',
  'feature': 'feature',
  'knowledge-extract': 'knowledgeExtract',
  'knowledge-to-code': 'knowledgeToCode',
};

interface TemplateListProps {
  onSelectTemplate: (template: WorkflowTemplateResponse) => void;
  onCreateNew: () => void;
  onEditTemplate?: (template: WorkflowTemplateResponse) => void;
}

const TAG_COLORS: Record<string, string> = {
  ai: 'blue',
  automation: 'green',
  workflow: 'cyan',
  agent: 'purple',
  chatbot: 'magenta',
  'data-processing': 'orange',
  code: 'geekblue',
  review: 'lime',
  quality: 'green',
  debug: 'red',
  fix: 'volcano',
  troubleshoot: 'orange',
  docs: 'purple',
  api: 'blue',
  readme: 'cyan',
  testing: 'green',
  tdd: 'lime',
  coverage: 'geekblue',
};

export const TemplateList: React.FC<TemplateListProps> = ({
  onSelectTemplate,
  onCreateNew,
  onEditTemplate,
}) => {
  const { t } = useTranslation();
  const { templates, isLoading, loadTemplates, deleteTemplate, duplicateTemplate } = useWorkflowEditorStore();
  const [searchText, setSearchText] = useState('');
  const [filterTag, setFilterTag] = useState<string | undefined>(undefined);
  const [filterPreset, setFilterPreset] = useState<boolean | undefined>(undefined);
  const [deleteModalVisible, setDeleteModalVisible] = useState(false);
  const [templateToDelete, setTemplateToDelete] = useState<WorkflowTemplateResponse | null>(null);
  const [versionHistoryVisible, setVersionHistoryVisible] = useState(false);
  const [templateForVersionHistory, setTemplateForVersionHistory] = useState<WorkflowTemplateResponse | null>(null);

  React.useEffect(() => {
    loadTemplates();
  }, [loadTemplates]);

  const allTags = React.useMemo(() => {
    const tagSet = new Set<string>();
    templates.forEach((t) => {
      t.tags?.forEach((tag) => tagSet.add(tag));
    });
    return Array.from(tagSet).sort();
  }, [templates]);

  const filteredTemplates = React.useMemo(() => {
    return templates.filter((template) => {
      const matchesSearch =
        !searchText ||
        template.name.toLowerCase().includes(searchText.toLowerCase()) ||
        template.description?.toLowerCase().includes(searchText.toLowerCase());
      const matchesTag = !filterTag || template.tags?.includes(filterTag);
      const matchesPreset = filterPreset === undefined || template.is_preset === filterPreset;
      return matchesSearch && matchesTag && matchesPreset;
    });
  }, [templates, searchText, filterTag, filterPreset]);

  const handleDelete = async () => {
    if (!templateToDelete) return;
    try {
      await deleteTemplate(templateToDelete.id);
      message.success('模板已删除');
      setDeleteModalVisible(false);
      setTemplateToDelete(null);
    } catch (error) {
      message.error('删除失败');
    }
  };

  const handleDuplicate = async (template: WorkflowTemplateResponse) => {
    try {
      await duplicateTemplate(template.id);
      message.success('模板已复制');
    } catch (error) {
      message.error('复制失败');
    }
  };

  const renderTemplateCard = (template: WorkflowTemplateResponse) => {
    const menuItems = [
      {
        key: 'view',
        icon: <Eye size={14} />,
        label: '查看',
        onClick: () => onSelectTemplate(template),
      },
    ];

    if (template.is_editable) {
      menuItems.push(
        {
          key: 'edit',
          icon: <Edit2 size={14} />,
          label: '编辑',
          onClick: () => onEditTemplate?.(template),
        },
        {
          key: 'versionHistory',
          icon: <History size={14} />,
          label: '版本历史',
          onClick: () => {
            setTemplateForVersionHistory(template);
            setVersionHistoryVisible(true);
          },
        },
        {
          key: 'duplicate',
          icon: <Copy size={14} />,
          label: '复制',
          onClick: () => handleDuplicate(template),
        },
        {
          key: 'delete',
          icon: <Trash2 size={14} style={{ color: '#ff4d4f' }} />,
          label: '删除',
          onClick: () => {
            setTemplateToDelete(template);
            setDeleteModalVisible(true);
          },
        }
      );
    }

    return (
      <Card
        key={template.id}
        size="small"
        hoverable
        onClick={() => onSelectTemplate(template)}
        style={{
          background: '#1e1e1e',
          border: '1px solid #333',
          cursor: 'pointer',
          transition: 'all 0.2s',
        }}
        styles={{
          body: { padding: 12 },
        }}
      >
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start' }}>
          <div style={{ flex: 1 }}>
            <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 6 }}>
              <span style={{ fontSize: 16 }}>{template.icon || '📋'}</span>
              <span style={{ fontWeight: 500, color: '#fff', fontSize: 14 }}>
                {template.is_preset && PRESET_I18N_KEY[template.id]
                  ? t(`chat.workflow.${PRESET_I18N_KEY[template.id]}.name`, template.name)
                  : template.name}
              </span>
              {template.is_preset && (
                <Tag color="gold" style={{ marginLeft: 4, fontSize: 10 }}>
                  预设
                </Tag>
              )}
              {!template.is_editable && (
                <Tag color="default" style={{ fontSize: 10 }}>
                  只读
                </Tag>
              )}
            </div>
            <div
              style={{
                color: '#888',
                fontSize: 12,
                marginBottom: 8,
                overflow: 'hidden',
                textOverflow: 'ellipsis',
                whiteSpace: 'nowrap',
              }}
            >
              {template.is_preset && PRESET_I18N_KEY[template.id]
                ? t(`chat.workflow.${PRESET_I18N_KEY[template.id]}.description`, template.description || '')
                : template.description || '暂无描述'}
            </div>
            <div style={{ display: 'flex', gap: 4, flexWrap: 'wrap' }}>
              {template.tags?.slice(0, 4).map((tag) => (
                <Tag
                  key={tag}
                  color={TAG_COLORS[tag] || 'default'}
                  style={{ fontSize: 10, margin: 0 }}
                >
                  {tag}
                </Tag>
              ))}
              {template.tags && template.tags.length > 4 && (
                <Tag style={{ fontSize: 10, margin: 0 }}>+{template.tags.length - 4}</Tag>
              )}
            </div>
          </div>
          <Dropdown
            menu={{ items: menuItems }}
            trigger={['click']}
            placement="bottomRight"
          >
            <Button
              type="text"
              size="small"
              icon={<MoreVertical size={14} />}
              onClick={(e) => e.stopPropagation()}
              style={{ color: '#666' }}
            />
          </Dropdown>
        </div>
      </Card>
    );
  };

  if (isLoading) {
    return (
      <div style={{ display: 'flex', justifyContent: 'center', alignItems: 'center', height: 200 }}>
        <Spin size="large" />
      </div>
    );
  }

  return (
    <div style={{ padding: 16 }}>
      <div style={{ marginBottom: 16 }}>
        <div style={{ display: 'flex', gap: 8, marginBottom: 12 }}>
          <Input
            placeholder="搜索模板..."
            prefix={<Search size={14} color="#666" />}
            value={searchText}
            onChange={(e) => setSearchText(e.target.value)}
            size="small"
            style={{ flex: 1 }}
            allowClear
          />
          <Select
            placeholder="标签"
            value={filterTag}
            onChange={setFilterTag}
            allowClear
            size="small"
            style={{ width: 100 }}
            options={allTags.map((tag) => ({ value: tag, label: tag }))}
          />
          <Select
            placeholder="类型"
            value={filterPreset}
            onChange={setFilterPreset}
            allowClear
            size="small"
            style={{ width: 100 }}
            options={[
              { value: true, label: '预设' },
              { value: false, label: '自定义' },
            ]}
          />
        </div>
        <Button
          type="primary"
          icon={<Plus size={14} />}
          onClick={onCreateNew}
          style={{ width: '100%' }}
          size="small"
        >
          新建模板
        </Button>
      </div>

      <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fill, minmax(280px, 1fr))', gap: 12 }}>
        {filteredTemplates.map(renderTemplateCard)}
      </div>

      {filteredTemplates.length === 0 && !isLoading && (
        <Empty
          description={searchText || filterTag ? '未找到匹配的模板' : '暂无模板'}
          style={{ marginTop: 48 }}
        />
      )}

      <Modal
        title="确认删除"
        open={deleteModalVisible}
        onOk={handleDelete}
        onCancel={() => {
          setDeleteModalVisible(false);
          setTemplateToDelete(null);
        }}
        okText="删除"
        okButtonProps={{ danger: true }}
      >
        <p>确定要删除模板 "{templateToDelete?.name}" 吗？</p>
        <p style={{ color: '#ff4d4f', fontSize: 12 }}>此操作不可恢复</p>
      </Modal>

      <VersionHistoryModal
        visible={versionHistoryVisible}
        template={templateForVersionHistory}
        onClose={() => {
          setVersionHistoryVisible(false);
          setTemplateForVersionHistory(null);
        }}
        onLoadVersion={onSelectTemplate}
      />
    </div>
  );
};

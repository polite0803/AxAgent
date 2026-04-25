import React, { useState, useCallback } from 'react';
import { Input, Button, Space, Tooltip } from 'antd';
import { Save, ArrowLeft, Eye, Share2, Bot } from 'lucide-react';

interface EditorHeaderProps {
  templateName: string;
  isDirty: boolean;
  isSaving: boolean;
  onSave: () => void;
  onClose?: () => void;
}

export const EditorHeader: React.FC<EditorHeaderProps> = ({
  templateName,
  isDirty,
  isSaving,
  onSave,
  onClose,
}) => {
  const [isEditing, setIsEditing] = useState(false);
  const [name, setName] = useState(templateName);

  const handleSave = useCallback(() => {
    onSave();
  }, [onSave]);

  return (
    <div
      style={{
        height: 56,
        background: '#252525',
        borderBottom: '1px solid #333',
        display: 'flex',
        alignItems: 'center',
        padding: '0 16px',
        gap: 12,
      }}
    >
      {onClose && (
        <Button
          type="text"
          icon={<ArrowLeft size={18} />}
          onClick={onClose}
          style={{ color: '#999' }}
        />
      )}

      <Bot size={20} style={{ color: '#1890ff' }} />

      {isEditing ? (
        <Input
          value={name}
          onChange={(e) => setName(e.target.value)}
          onBlur={() => setIsEditing(false)}
          onPressEnter={() => setIsEditing(false)}
          autoFocus
          style={{ width: 200 }}
        />
      ) : (
        <span
          onClick={() => setIsEditing(true)}
          style={{ color: '#fff', cursor: 'pointer', fontSize: 14 }}
        >
          {name}
          {isDirty && <span style={{ color: '#faad14', marginLeft: 4 }}>*</span>}
        </span>
      )}

      <div style={{ flex: 1 }} />

      <Space>
        <Tooltip title="预览">
          <Button type="text" icon={<Eye size={18} />} style={{ color: '#999' }} />
        </Tooltip>

        <Tooltip title="发布">
          <Button type="text" icon={<Share2 size={18} />} style={{ color: '#999' }} />
        </Tooltip>

        <Button
          type="primary"
          icon={<Save size={16} />}
          loading={isSaving}
          onClick={handleSave}
          style={{ display: 'flex', alignItems: 'center', gap: 6 }}
        >
          {isSaving ? '保存中...' : '保存'}
        </Button>
      </Space>
    </div>
  );
};

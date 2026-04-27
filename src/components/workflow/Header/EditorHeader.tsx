import { Button, Input, Space, Tooltip, message } from "antd";
import { ArrowLeft, Bot, Bug, Eye, Save, Share2, Sparkles, Download } from "lucide-react";
import React, { useCallback, useEffect, useState } from "react";

interface EditorHeaderProps {
  templateName: string;
  isDirty: boolean;
  isSaving: boolean;
  onSave: () => void;
  onNameChange?: (name: string) => void;
  onClose?: () => void;
  onToggleAIPanel?: () => void;
  onToggleDebugPanel?: () => void;
  onOpenImportExport?: () => void;
  aiPanelVisible?: boolean;
  debugPanelVisible?: boolean;
}

export const EditorHeader: React.FC<EditorHeaderProps> = ({
  templateName,
  isDirty,
  isSaving,
  onSave,
  onNameChange,
  onClose,
  onToggleAIPanel,
  onToggleDebugPanel,
  onOpenImportExport,
  aiPanelVisible = false,
  debugPanelVisible = false,
}) => {
  const [isEditing, setIsEditing] = useState(false);
  const [name, setName] = useState(templateName);

  useEffect(() => {
    if (!isEditing) {
      setName(templateName);
    }
  }, [templateName, isEditing]);

  const handleNameChange = useCallback((newName: string) => {
    setName(newName);
  }, []);

  const handleNameBlur = useCallback(() => {
    setIsEditing(false);
    if (onNameChange && name !== templateName) {
      onNameChange(name);
    }
  }, [name, templateName, onNameChange]);

  const handleNameKeyDown = useCallback((e: React.KeyboardEvent) => {
    if (e.key === "Enter") {
      setIsEditing(false);
      if (onNameChange && name !== templateName) {
        onNameChange(name);
      }
    }
  }, [name, templateName, onNameChange]);

  const handleSave = useCallback(() => {
    onSave();
  }, [onSave]);

  const handlePreview = useCallback(() => {
    message.info("预览功能开发中");
  }, []);

  const handlePublish = useCallback(() => {
    message.info("发布功能开发中");
  }, []);

  return (
    <div
      style={{
        height: 56,
        background: "#252525",
        borderBottom: "1px solid #333",
        display: "flex",
        alignItems: "center",
        padding: "0 16px",
        gap: 12,
      }}
    >
      {onClose && (
        <Button
          type="text"
          icon={<ArrowLeft size={18} />}
          onClick={onClose}
          style={{ color: "#999" }}
        />
      )}

      <Bot size={20} style={{ color: "#1890ff" }} />

      {isEditing
        ? (
          <Input
            value={name}
            onChange={(e) => handleNameChange(e.target.value)}
            onBlur={handleNameBlur}
            onKeyDown={handleNameKeyDown}
            autoFocus
            style={{ width: 200 }}
          />
        )
        : (
          <span
            onClick={() => setIsEditing(true)}
            style={{ color: "#fff", cursor: "pointer", fontSize: 14 }}
          >
            {name}
            {isDirty && <span style={{ color: "#faad14", marginLeft: 4 }}>*</span>}
          </span>
        )}

      <div style={{ flex: 1 }} />

      <Space>
        {onToggleAIPanel && (
          <Tooltip title="AI 辅助">
            <Button
              type="text"
              icon={<Sparkles size={18} />}
              onClick={onToggleAIPanel}
              style={{ color: aiPanelVisible ? "#1890ff" : "#999" }}
            />
          </Tooltip>
        )}

        {onToggleDebugPanel && (
          <Tooltip title="调试面板">
            <Button
              type="text"
              icon={<Bug size={18} />}
              onClick={onToggleDebugPanel}
              style={{ color: debugPanelVisible ? "#1890ff" : "#999" }}
            />
          </Tooltip>
        )}

        {onOpenImportExport && (
          <Tooltip title="导入/导出">
            <Button
              type="text"
              icon={<Download size={18} />}
              onClick={onOpenImportExport}
              style={{ color: "#999" }}
            />
          </Tooltip>
        )}

        <Tooltip title="预览">
          <Button type="text" icon={<Eye size={18} />} onClick={handlePreview} style={{ color: "#999" }} />
        </Tooltip>

        <Tooltip title="发布">
          <Button type="text" icon={<Share2 size={18} />} onClick={handlePublish} style={{ color: "#999" }} />
        </Tooltip>

        <Button
          type="primary"
          icon={<Save size={16} />}
          loading={isSaving}
          onClick={handleSave}
          style={{ display: "flex", alignItems: "center", gap: 6 }}
        >
          {isSaving ? "保存中..." : "保存"}
        </Button>
      </Space>
    </div>
  );
};

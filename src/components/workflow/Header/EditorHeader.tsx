import { Button, Input, message, Space, Tooltip } from "antd";
import { ArrowLeft, Bot, Bug, Download, Eye, Save, Share2, Sparkles } from "lucide-react";
import React, { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

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
  const { t } = useTranslation();

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
    message.info(t("workflow.previewInDevelopment"));
  }, [t]);

  const handlePublish = useCallback(() => {
    message.info(t("workflow.publishInDevelopment"));
  }, [t]);

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
          <Tooltip title={t("workflow.aiAssistant")}>
            <Button
              type="text"
              data-testid="workflow-ai-panel-btn"
              icon={<Sparkles size={18} />}
              onClick={onToggleAIPanel}
              style={{ color: aiPanelVisible ? "#1890ff" : "#999" }}
            />
          </Tooltip>
        )}

        {onToggleDebugPanel && (
          <Tooltip title={t("workflow.debugPanel")}>
            <Button
              type="text"
              icon={<Bug size={18} />}
              onClick={onToggleDebugPanel}
              style={{ color: debugPanelVisible ? "#1890ff" : "#999" }}
            />
          </Tooltip>
        )}

        {onOpenImportExport && (
          <Tooltip title={t("workflow.importExport")}>
            <Button
              type="text"
              data-testid="workflow-import-export-btn"
              icon={<Download size={18} />}
              onClick={onOpenImportExport}
              style={{ color: "#999" }}
            />
          </Tooltip>
        )}

        <Tooltip title={t("workflow.preview")}>
          <Button type="text" icon={<Eye size={18} />} onClick={handlePreview} style={{ color: "#999" }} />
        </Tooltip>

        <Tooltip title={t("workflow.publish")}>
          <Button type="text" icon={<Share2 size={18} />} onClick={handlePublish} style={{ color: "#999" }} />
        </Tooltip>

        <Button
          type="primary"
          icon={<Save size={16} />}
          loading={isSaving}
          onClick={handleSave}
          style={{ display: "flex", alignItems: "center", gap: 6 }}
        >
          {isSaving ? t("workflow.saving") : t("workflow.save")}
        </Button>
      </Space>
    </div>
  );
};

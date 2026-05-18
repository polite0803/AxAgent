import { Button, Input, Popover, Space, theme, Tooltip } from "antd";
import { ArrowLeft, Bot, Bug, Download, Eye, Keyboard, Redo2, Save, Share2, Sparkles, Undo2 } from "lucide-react";
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
  onUndo?: () => void;
  onRedo?: () => void;
  canUndo?: boolean;
  canRedo?: boolean;
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
  onUndo,
  onRedo,
  canUndo = false,
  canRedo = false,
  aiPanelVisible = false,
  debugPanelVisible = false,
}) => {
  const [isEditing, setIsEditing] = useState(false);
  const [name, setName] = useState(templateName);
  const { t } = useTranslation();
  const { token } = theme.useToken();

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

  return (
    <div
      style={{
        height: 56,
        background: token.colorBgElevated,
        borderBottom: `1px solid ${token.colorBorderSecondary}`,
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
          style={{ color: token.colorTextSecondary }}
        />
      )}

      <Bot size={20} style={{ color: "#1890ff" }} />

      {isEditing
        ? (
          <Input
            id="editor-header-input-73"
            value={name}
            onChange={(e) => handleNameChange(e.target.value)}
            onBlur={handleNameBlur}
            onKeyDown={handleNameKeyDown}
            style={{ width: 200 }}
          />
        )
        : (
          <span
            role="button"
            tabIndex={0}
            onClick={() => setIsEditing(true)}
            onKeyDown={(e) => {
              if (e.key === "Enter" || e.key === " ") { setIsEditing(true); }
            }}
            style={{ color: token.colorText, cursor: "pointer", fontSize: 14 }}
          >
            {name}
            {isDirty && <span style={{ color: token.colorWarning, marginLeft: 4 }}>*</span>}
          </span>
        )}

      <div style={{ flex: 1 }} />

      <Space>
        {onUndo && (
          <Tooltip title={`${t("workflow.undo")} (Ctrl+Z)`}>
            <Button
              type="text"
              icon={<Undo2 size={18} />}
              onClick={onUndo}
              disabled={!canUndo}
              style={{ color: canUndo ? token.colorTextSecondary : token.colorTextQuaternary }}
            />
          </Tooltip>
        )}

        {onRedo && (
          <Tooltip title={`${t("workflow.redo")} (Ctrl+Shift+Z)`}>
            <Button
              type="text"
              icon={<Redo2 size={18} />}
              onClick={onRedo}
              disabled={!canRedo}
              style={{ color: canRedo ? token.colorTextSecondary : token.colorTextQuaternary }}
            />
          </Tooltip>
        )}

        <Popover
          content={
            <div style={{ minWidth: 220 }}>
              <div style={{ fontSize: 13, fontWeight: 600, color: token.colorText, marginBottom: 8 }}>
                {t("workflow.shortcuts.title")}
              </div>
              {[
                { keys: "Ctrl+Z", label: t("workflow.shortcuts.undo") },
                { keys: "Ctrl+Shift+Z / Ctrl+Y", label: t("workflow.shortcuts.redo") },
                { keys: "Ctrl+C", label: t("workflow.shortcuts.copy") },
                { keys: "Ctrl+V", label: t("workflow.shortcuts.paste") },
                { keys: "Delete / Backspace", label: t("workflow.shortcuts.delete") },
                { keys: "Ctrl+S", label: t("workflow.shortcuts.save") },
              ].map((item) => (
                <div
                  key={item.keys}
                  style={{ display: "flex", justifyContent: "space-between", alignItems: "center", padding: "4px 0" }}
                >
                  <span style={{ fontSize: 12, color: token.colorTextSecondary }}>{item.label}</span>
                  <kbd
                    style={{
                      fontSize: 12,
                      padding: "1px 6px",
                      background: token.colorBgContainer,
                      border: `1px solid ${token.colorBorderSecondary}`,
                      borderRadius: 3,
                      color: token.colorTextSecondary,
                    }}
                  >
                    {item.keys}
                  </kbd>
                </div>
              ))}
            </div>
          }
          trigger="click"
          placement="bottomRight"
        >
          <Button
            type="text"
            icon={<Keyboard size={18} />}
            style={{ color: token.colorTextSecondary }}
          />
        </Popover>

        {onToggleAIPanel && (
          <Tooltip title={t("workflow.aiAssistant")}>
            <Button
              type="text"
              data-testid="workflow-ai-panel-btn"
              icon={<Sparkles size={18} />}
              onClick={onToggleAIPanel}
              style={{ color: aiPanelVisible ? "#1890ff" : token.colorTextSecondary }}
            />
          </Tooltip>
        )}

        {onToggleDebugPanel && (
          <Tooltip title={t("workflow.debugPanel")}>
            <Button
              type="text"
              icon={<Bug size={18} />}
              onClick={onToggleDebugPanel}
              style={{ color: debugPanelVisible ? "#1890ff" : token.colorTextSecondary }}
            />
          </Tooltip>
        )}

        {onOpenImportExport && (
          <Tooltip title={t("workflow.importExport.title")}>
            <Button
              type="text"
              data-testid="workflow-import-export-btn"
              icon={<Download size={18} />}
              onClick={onOpenImportExport}
              style={{ color: token.colorTextSecondary }}
            />
          </Tooltip>
        )}

        <Tooltip title={t("workflow.preview")}>
          <Button type="text" icon={<Eye size={18} />} disabled style={{ color: token.colorTextSecondary }} />
        </Tooltip>

        <Tooltip title={t("workflow.publish")}>
          <Button type="text" icon={<Share2 size={18} />} disabled style={{ color: token.colorTextSecondary }} />
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

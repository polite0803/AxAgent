import { Button, Input, Space, theme, Tooltip } from "antd";
import { Play, Save, Settings } from "lucide-react";
import React, { memo, useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

interface CanvasTitleBarProps {
  workflowName: string;
  isDirty: boolean;
  isSaving: boolean;
  onNameChange: (name: string) => void;
  onSave: () => void;
  onRun: () => void;
  onSettings: () => void;
}

/**
 * 画布顶部名称条。
 *
 * 始终固定显示当前工作流名称（可编辑），
 * 右侧显示面包屑导航：Workspace > Workflow > {name}，
 * 名称条下方为画布工具栏（保存/运行/设置按钮组）。
 */
export const CanvasTitleBar: React.FC<CanvasTitleBarProps> = memo(
  ({
    workflowName,
    isDirty,
    isSaving,
    onNameChange,
    onSave,
    onRun,
    onSettings,
  }) => {
    const { t } = useTranslation();
    const { token } = theme.useToken();
    const [isEditing, setIsEditing] = useState(false);
    const [localName, setLocalName] = useState(workflowName);

    useEffect(() => {
      if (!isEditing) {
        setLocalName(workflowName);
      }
    }, [workflowName, isEditing]);

    const handleBlur = useCallback(() => {
      setIsEditing(false);
      if (localName !== workflowName && localName.trim()) {
        onNameChange(localName.trim());
      } else {
        setLocalName(workflowName);
      }
    }, [localName, workflowName, onNameChange]);

    const handleKeyDown = useCallback(
      (e: React.KeyboardEvent) => {
        if (e.key === "Enter") {
          (e.target as HTMLElement).blur();
        }
        if (e.key === "Escape") {
          setLocalName(workflowName);
          setIsEditing(false);
        }
      },
      [workflowName],
    );

    return (
      <div
        style={{
          background: token.colorBgElevated,
          borderBottom: `1px solid ${token.colorBorderSecondary}`,
          flexShrink: 0,
        }}
      >
        {/* 名称行：面包屑 + 可编辑名称 + 工具栏 */}
        <div
          style={{
            display: "flex",
            alignItems: "center",
            padding: "8px 16px",
            gap: 12,
          }}
        >
          {/* 面包屑导航 */}
          <span
            style={{
              fontSize: 11,
              color: token.colorTextTertiary,
              whiteSpace: "nowrap",
            }}
          >
            <span style={{ color: token.colorTextQuaternary }}>
              Workspace
            </span>
            <span style={{ margin: "0 4px", color: token.colorTextQuaternary }}>
              &gt;
            </span>
            <span style={{ color: token.colorTextQuaternary }}>Workflow</span>
            <span style={{ margin: "0 4px", color: token.colorTextQuaternary }}>
              &gt;
            </span>
          </span>

          {/* 可编辑工作流名称 */}
          {isEditing
            ? (
              <Input
                id="canvas-title-input"
                value={localName}
                onChange={(e) => setLocalName(e.target.value)}
                onBlur={handleBlur}
                onKeyDown={handleKeyDown}
                autoFocus
                size="small"
                style={{
                  width: 280,
                  fontSize: 14,
                  fontWeight: 600,
                }}
              />
            )
            : (
              <span
                onClick={() => setIsEditing(true)}
                style={{
                  fontSize: 14,
                  fontWeight: 600,
                  color: token.colorText,
                  cursor: "text",
                  padding: "2px 4px",
                  borderRadius: 4,
                  border: `1px solid transparent`,
                  transition: "border-color 0.15s",
                  maxWidth: 400,
                  overflow: "hidden",
                  textOverflow: "ellipsis",
                  whiteSpace: "nowrap",
                }}
                onMouseEnter={(e) => {
                  e.currentTarget.style.borderColor = token.colorBorderSecondary;
                }}
                onMouseLeave={(e) => {
                  e.currentTarget.style.borderColor = "transparent";
                }}
                title={t("workflow.canvasTitle.clickToRename", {
                  defaultValue: "Click to rename",
                })}
              >
                {localName}
                {isDirty && (
                  <span
                    style={{
                      marginLeft: 4,
                      fontSize: 10,
                      color: token.colorWarning,
                      fontWeight: 400,
                    }}
                  >
                    ●
                  </span>
                )}
              </span>
            )}

          {/* 右侧填充 */}
          <div style={{ flex: 1 }} />

          {/* 工具栏按钮组 */}
          <Space size={4}>
            <Tooltip
              title={isSaving
                ? t("workflow.saving")
                : t("workflow.save")}
            >
              <Button
                type="text"
                size="small"
                icon={<Save size={14} />}
                onClick={onSave}
                loading={isSaving}
                style={{ color: token.colorTextSecondary }}
              >
                {t("workflow.save")}
              </Button>
            </Tooltip>

            <Tooltip title={t("workflow.run")}>
              <Button
                type="primary"
                size="small"
                icon={<Play size={14} />}
                onClick={onRun}
              >
                {t("workflow.run")}
              </Button>
            </Tooltip>

            <Tooltip title={t("workflow.settings")}>
              <Button
                type="text"
                size="small"
                icon={<Settings size={14} />}
                onClick={onSettings}
                style={{ color: token.colorTextSecondary }}
              />
            </Tooltip>
          </Space>
        </div>
      </div>
    );
  },
);

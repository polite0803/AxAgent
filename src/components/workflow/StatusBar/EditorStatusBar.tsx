import { Tooltip } from "@/components/layout/Tooltip";
import { Tag, theme } from "antd";
import { AlertCircle, AlertTriangle, CheckCircle, Circle, Maximize2 } from "lucide-react";
import React from "react";
import { useTranslation } from "react-i18next";
import type { ValidationResult } from "../types";

interface StatusBarProps {
  nodeCount: number;
  edgeCount: number;
  validationResult: ValidationResult | null;
  isDirty: boolean;
  zoom: number;
  onFitView: () => void;
  onResetZoom: () => void;
}

export const StatusBar: React.FC<StatusBarProps> = ({
  nodeCount,
  edgeCount,
  validationResult,
  isDirty,
  zoom,
  onFitView,
  onResetZoom,
}) => {
  const { t } = useTranslation();
  const { token } = theme.useToken();

  const getValidationIcon = () => {
    if (!validationResult) {
      return null;
    }

    if (validationResult.errors.length > 0) {
      return (
        <Tooltip
          title={t("workflow.statusBar.errors", {
            count: validationResult.errors.length,
          })}
        >
          <AlertCircle size={14} style={{ color: token.colorError }} />
        </Tooltip>
      );
    }

    if (validationResult.warnings.length > 0) {
      return (
        <Tooltip
          title={t("workflow.statusBar.warnings", {
            count: validationResult.warnings.length,
          })}
        >
          <AlertTriangle size={14} style={{ color: token.colorWarning }} />
        </Tooltip>
      );
    }

    return (
      <Tooltip title={t("workflow.statusBar.valid")}>
        <CheckCircle size={14} style={{ color: token.colorSuccess }} />
      </Tooltip>
    );
  };

  return (
    <div
      style={{
        height: 28,
        background: token.colorBgContainer,
        borderTop: `1px solid ${token.colorBorderSecondary}`,
        display: "flex",
        alignItems: "center",
        padding: "0 12px",
        gap: 16,
        fontSize: 12,
        color: token.colorTextTertiary,
      }}
    >
      <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
        <Circle
          size={10}
          fill={isDirty ? token.colorWarning : token.colorSuccess}
          color={isDirty ? token.colorWarning : token.colorSuccess}
        />
        <span>
          {isDirty
            ? t("workflow.statusBar.unsaved")
            : t("workflow.statusBar.saved")}
        </span>
      </div>

      <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
        <span>{t("workflow.statusBar.nodes", { count: nodeCount })}</span>
      </div>

      <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
        <span>{t("workflow.statusBar.edges", { count: edgeCount })}</span>
      </div>

      {validationResult && (
        <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
          {getValidationIcon()}
          <span>
            {t("workflow.statusBar.error", {
              count: validationResult.errors.length,
            })}
            , {t("workflow.statusBar.warning", {
              count: validationResult.warnings.length,
            })}
          </span>
        </div>
      )}

      <div style={{ flex: 1 }} />

      <Tooltip title={t("workflow.statusBar.resetZoom")}>
        <span
          role="button"
          tabIndex={0}
          onClick={onResetZoom}
          onKeyDown={(e) => {
            if (e.key === "Enter" || e.key === " ") {
              onResetZoom();
            }
          }}
          style={{
            cursor: "pointer",
            color: token.colorTextSecondary,
            userSelect: "none",
          }}
        >
          {Math.round(zoom * 100)}%
        </span>
      </Tooltip>

      <Tooltip title={t("workflow.statusBar.fitView")}>
        <Maximize2
          size={14}
          onClick={onFitView}
          style={{ cursor: "pointer", color: token.colorTextSecondary }}
        />
      </Tooltip>

      <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
        <Tag color="purple" style={{ margin: 0, fontSize: 12 }}>
          {t("workflow.statusBar.dagEditor")}
        </Tag>
      </div>
    </div>
  );
};

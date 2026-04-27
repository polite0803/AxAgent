import { Tag, Tooltip } from "antd";
import { AlertCircle, AlertTriangle, CheckCircle, Circle } from "lucide-react";
import React from "react";
import type { ValidationResult } from "../types";

interface StatusBarProps {
  nodeCount: number;
  edgeCount: number;
  validationResult: ValidationResult | null;
  isDirty: boolean;
}

export const StatusBar: React.FC<StatusBarProps> = ({
  nodeCount,
  edgeCount,
  validationResult,
  isDirty,
}) => {
  const getValidationIcon = () => {
    if (!validationResult) { return null; }

    if (validationResult.errors.length > 0) {
      return (
        <Tooltip title={`${validationResult.errors.length} 个错误`}>
          <AlertCircle size={14} style={{ color: "#ff4d4f" }} />
        </Tooltip>
      );
    }

    if (validationResult.warnings.length > 0) {
      return (
        <Tooltip title={`${validationResult.warnings.length} 个警告`}>
          <AlertTriangle size={14} style={{ color: "#faad14" }} />
        </Tooltip>
      );
    }

    return (
      <Tooltip title="工作流有效">
        <CheckCircle size={14} style={{ color: "#52c41a" }} />
      </Tooltip>
    );
  };

  return (
    <div
      style={{
        height: 28,
        background: "#1a1a1a",
        borderTop: "1px solid #333",
        display: "flex",
        alignItems: "center",
        padding: "0 12px",
        gap: 16,
        fontSize: 11,
        color: "#666",
      }}
    >
      <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
        <Circle size={10} fill={isDirty ? "#faad14" : "#52c41a"} color={isDirty ? "#faad14" : "#52c41a"} />
        <span>{isDirty ? "未保存" : "已保存"}</span>
      </div>

      <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
        <span>节点: {nodeCount}</span>
      </div>

      <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
        <span>边: {edgeCount}</span>
      </div>

      {validationResult && (
        <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
          {getValidationIcon()}
          <span>
            {validationResult.errors.length} 错误, {validationResult.warnings.length} 警告
          </span>
        </div>
      )}

      <div style={{ flex: 1 }} />

      <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
        <Tag color="purple" style={{ margin: 0, fontSize: 10 }}>
          DAG 编辑器
        </Tag>
      </div>
    </div>
  );
};

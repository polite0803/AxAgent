import { theme } from "antd";
import React, { memo } from "react";
import { useTranslation } from "react-i18next";
import { Handle, type NodeProps, Position } from "reactflow";

const NODE_COLOR = "#1890ff";

interface FileOperationNodeData {
  id: string;
  type: string;
  title: string;
  color: string;
  nodeType: string;
  enabled: boolean;
}

const FileOperationNodeComponent: React.FC<NodeProps<FileOperationNodeData>> = ({ data, selected }) => {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  return (
    <div
      style={{
        minWidth: 160,
        maxWidth: 220,
        opacity: data.enabled ? 1 : 0.5,
        filter: data.enabled ? "none" : "grayscale(100%)",
      }}
    >
      <div
        style={{
          background: token.colorBgElevated,
          border: `2px solid ${selected ? token.colorPrimary : NODE_COLOR}`,
          borderRadius: 8,
          overflow: "hidden",
          boxShadow: selected ? `0 0 0 2px ${NODE_COLOR}40` : "none",
          transition: "box-shadow 0.2s, transform 0.2s",
        }}
      >
        <div
          style={{
            padding: "8px 12px",
            borderBottom: `1px solid ${NODE_COLOR}30`,
            display: "flex",
            alignItems: "center",
            gap: 8,
            background: NODE_COLOR + "15",
          }}
        >
          <span style={{ fontSize: 14 }}>📁</span>
          <span style={{ fontSize: 12, color: NODE_COLOR, fontWeight: 600 }}>
            {t("workflow.nodeTypes.fileOperation")}
          </span>
        </div>
        <div style={{ padding: "10px 12px" }}>
          <div
            style={{
              fontSize: 13,
              color: token.colorText,
              fontWeight: 500,
              marginBottom: 4,
              overflow: "hidden",
              textOverflow: "ellipsis",
              whiteSpace: "nowrap",
            }}
          >
            {data.title}
          </div>
        </div>
      </div>
      <Handle
        type="target"
        position={Position.Top}
        style={{ background: NODE_COLOR, border: "none", width: 8, height: 8 }}
      />
      <Handle
        type="source"
        position={Position.Bottom}
        style={{ background: NODE_COLOR, border: "none", width: 8, height: 8 }}
      />
    </div>
  );
};

export const FileOperationNode = memo(FileOperationNodeComponent);

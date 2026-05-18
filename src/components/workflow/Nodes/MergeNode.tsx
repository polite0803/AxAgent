import { Tag } from "antd";
import React, { memo } from "react";
import { useTranslation } from "react-i18next";
import { Handle, type NodeProps, Position } from "reactflow";

interface MergeNodeData {
  id: string;
  type: string;
  title: string;
  description?: string;
  color: string;
  nodeType: string;
  enabled: boolean;
  mergeType?: string;
  inputs?: string[];
}

const MergeNodeComponent: React.FC<NodeProps<MergeNodeData>> = ({
  data,
  selected,
}) => {
  const { t } = useTranslation();
  const color = "#fa8c16";
  const mergeType = data.mergeType || "all";
  const inputs = data.inputs || [];

  const getMergeTypeLabel = (type: string): string => {
    const labels: Record<string, string> = {
      all: t("workflow.mergeNode.all"),
      first: t("workflow.mergeNode.first"),
      last: t("workflow.mergeNode.last"),
    };
    return labels[type] || type;
  };

  return (
    <div
      style={{
        minWidth: 160,
        maxWidth: 200,
        opacity: data.enabled ? 1 : 0.5,
        filter: data.enabled ? "none" : "grayscale(100%)",
      }}
    >
      <div
        style={{
          background: "#1e1e1e",
          border: `2px solid ${selected ? "#1890ff" : color}`,
          borderRadius: 8,
          overflow: "hidden",
          boxShadow: selected ? `0 0 0 2px ${color}40` : "none",
          transition: "box-shadow 0.2s, transform 0.2s",
        }}
      >
        <div
          style={{
            padding: "8px 12px",
            borderBottom: `1px solid ${color}30`,
            display: "flex",
            alignItems: "center",
            gap: 8,
            background: `${color}15`,
          }}
        >
          <span style={{ fontSize: 14 }}>🔗</span>
          <span
            style={{
              fontSize: 12,
              color: color,
              fontWeight: 600,
            }}
          >
            {t("workflow.mergeNode.title")}
          </span>
          <Tag
            style={{
              margin: 0,
              fontSize: 9,
              padding: "0 4px",
              background: `${color}30`,
              border: "none",
              color: "#fff",
            }}
          >
            {getMergeTypeLabel(mergeType)}
          </Tag>
        </div>

        <div style={{ padding: "10px 12px" }}>
          <div
            style={{
              fontSize: 13,
              color: "#fff",
              fontWeight: 500,
              marginBottom: 6,
              overflow: "hidden",
              textOverflow: "ellipsis",
              whiteSpace: "nowrap",
            }}
          >
            {data.title}
          </div>

          <Tag
            style={{
              margin: 0,
              fontSize: 12,
              padding: "0 6px",
              background: "#252525",
              border: "1px solid #444",
              color: "#aaa",
            }}
          >
            {t("workflow.mergeNode.inputCount", { count: inputs.length })}
          </Tag>
        </div>
      </div>

      <Handle
        type="target"
        position={Position.Top}
        style={{
          background: color,
          border: "none",
          width: 8,
          height: 8,
        }}
      />

      <Handle
        type="source"
        position={Position.Bottom}
        style={{
          background: color,
          border: "none",
          width: 8,
          height: 8,
        }}
      />

      <div
        style={{
          position: "absolute",
          left: -10,
          top: "50%",
          transform: "translateY(-50%)",
          width: 0,
          height: 0,
          borderTop: "6px solid transparent",
          borderBottom: "6px solid transparent",
          borderRight: `8px solid ${color}`,
        }}
      />
    </div>
  );
};

export const MergeNode = memo(MergeNodeComponent);

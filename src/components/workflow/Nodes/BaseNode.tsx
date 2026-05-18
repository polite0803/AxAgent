import React, { memo } from "react";
import { useTranslation } from "react-i18next";
import { Handle, type NodeProps, Position } from "reactflow";
import { NODE_TYPE_MAP } from "../types";

export interface BaseNodeData {
  id: string;
  type: string;
  title: string;
  description?: string;
  color: string;
  nodeType: string;
  enabled: boolean;
  validationState?: "error" | "warning";
}

const BaseNodeComponent: React.FC<NodeProps<BaseNodeData>> = ({ data, selected }) => {
  const { t } = useTranslation();
  const typeInfo = NODE_TYPE_MAP[data.nodeType] || { labelKey: "", color: "#999" };

  const getBorderColor = () => {
    if (data.validationState === "error") { return "#ff4d4f"; }
    if (data.validationState === "warning") { return "#faad14"; }
    if (selected) { return "#1890ff"; }
    return data.color;
  };

  const borderColor = getBorderColor();

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
          background: "#252525",
          border: `2px solid ${borderColor}`,
          borderRadius: 8,
          padding: 0,
          boxShadow: selected ? `0 0 0 2px ${borderColor}40` : "none",
          transition: "box-shadow 0.2s, transform 0.2s",
        }}
      >
        <div
          style={{
            padding: "8px 12px",
            borderBottom: `1px solid ${data.color}40`,
            display: "flex",
            alignItems: "center",
            gap: 8,
          }}
        >
          <span style={{ fontSize: 16 }}>{getNodeIcon(data.nodeType)}</span>
          <span
            style={{
              fontSize: 12,
              color: data.color,
              fontWeight: 500,
            }}
          >
            {typeInfo.labelKey ? t(typeInfo.labelKey) : data.nodeType}
          </span>
        </div>

        <div style={{ padding: "8px 12px" }}>
          <div
            style={{
              fontSize: 13,
              color: "#fff",
              fontWeight: 500,
              marginBottom: data.description ? 4 : 0,
              overflow: "hidden",
              textOverflow: "ellipsis",
              whiteSpace: "nowrap",
            }}
          >
            {data.title}
          </div>
          {data.description && (
            <div
              style={{
                fontSize: 10,
                color: "#666",
                overflow: "hidden",
                textOverflow: "ellipsis",
                whiteSpace: "nowrap",
              }}
            >
              {data.description}
            </div>
          )}
        </div>
      </div>

      <Handle
        type="target"
        position={Position.Top}
        style={{
          background: data.color,
          border: "none",
          width: 8,
          height: 8,
        }}
      />

      <Handle
        type="source"
        position={Position.Bottom}
        style={{
          background: data.color,
          border: "none",
          width: 8,
          height: 8,
        }}
      />

      {["condition", "parallel", "merge"].includes(data.nodeType) && (
        <>
          <Handle
            type="target"
            position={Position.Left}
            id="left-handle"
            style={{
              background: data.color,
              border: "none",
              width: 6,
              height: 6,
              top: "50%",
            }}
          />
          <Handle
            type="source"
            position={Position.Right}
            id="right-handle"
            style={{
              background: data.color,
              border: "none",
              width: 6,
              height: 6,
              top: "50%",
            }}
          />
        </>
      )}
    </div>
  );
};

function getNodeIcon(type: string): string {
  const icons: Record<string, string> = {
    trigger: "⚡",
    agent: "🤖",
    llm: "🧠",
    condition: "❓",
    parallel: "⏩",
    loop: "🔄",
    merge: "🔗",
    delay: "⏱",
    tool: "🔧",
    code: "💻",
    subWorkflow: "📦",
    documentParser: "📄",
    vectorRetrieve: "🔍",
    end: "🏁",
  };
  return icons[type] || "📦";
}

export const BaseNode = memo(BaseNodeComponent);

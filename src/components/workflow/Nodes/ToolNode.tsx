import { Tag } from "antd";
import React, { memo } from "react";
import { useTranslation } from "react-i18next";
import { Handle, type NodeProps, Position } from "reactflow";

interface ToolNodeData {
  id: string;
  type: string;
  title: string;
  description?: string;
  color: string;
  nodeType: string;
  enabled: boolean;
  toolName?: string;
  inputMapping?: Record<string, string>;
  outputVar?: string;
}

const ToolNodeComponent: React.FC<NodeProps<ToolNodeData>> = ({ data, selected }) => {
  const { t } = useTranslation();
  const color = "#52c41a";
  const toolName = data.toolName || t("workflow.toolNode.notSelected");
  const inputMapping = data.inputMapping || {};
  const outputVar = data.outputVar;

  const inputCount = Object.keys(inputMapping).length;

  return (
    <div
      style={{
        minWidth: 180,
        maxWidth: 220,
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
          <span style={{ fontSize: 14 }}>🔧</span>
          <span
            style={{
              fontSize: 11,
              color: color,
              fontWeight: 600,
            }}
          >
            {t("workflow.toolNode.title")}
          </span>
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

          <div
            style={{
              fontSize: 11,
              color: color,
              marginBottom: 6,
              padding: "4px 6px",
              background: `${color}15`,
              borderRadius: 4,
              overflow: "hidden",
              textOverflow: "ellipsis",
              whiteSpace: "nowrap",
              fontWeight: 500,
            }}
          >
            {toolName}
          </div>

          <div style={{ display: "flex", flexWrap: "wrap", gap: 4 }}>
            {inputCount > 0 && (
              <Tag
                style={{
                  margin: 0,
                  fontSize: 9,
                  padding: "0 4px",
                  background: "#252525",
                  border: "1px solid #444",
                  color: "#aaa",
                }}
              >
                📥 {t("workflow.toolNode.inputCount", { count: inputCount })}
              </Tag>
            )}

            {outputVar && (
              <Tag
                style={{
                  margin: 0,
                  fontSize: 9,
                  padding: "0 4px",
                  background: "#1890ff20",
                  border: "1px solid #1890ff50",
                  color: "#1890ff",
                }}
              >
                📤 {outputVar}
              </Tag>
            )}
          </div>
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
    </div>
  );
};

export const ToolNode = memo(ToolNodeComponent);

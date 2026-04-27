import { Tag } from "antd";
import React, { memo } from "react";
import { Handle, type NodeProps, Position } from "reactflow";

export interface ValidationNodeData {
  id: string;
  type: string;
  title: string;
  description?: string;
  color: string;
  nodeType: string;
  enabled: boolean;
  assertions: Assertion[];
  onFail: "stop" | "retry" | "continue";
  maxRetries: number;
}

export interface Assertion {
  type: "equals" | "contains" | "matches" | "exists" | "custom";
  expected?: string;
  actual?: string;
  expression?: string;
}

const ValidationNodeComponent: React.FC<NodeProps<ValidationNodeData>> = ({ data, selected }) => {
  const color = "#722ed1";
  const assertions = data.assertions || [];
  const onFail = data.onFail || "stop";
  const maxRetries = data.maxRetries || 0;

  const getOnFailLabel = (): string => {
    switch (onFail) {
      case "stop":
        return "停止";
      case "retry":
        return `重试 (最多${maxRetries}次)`;
      case "continue":
        return "继续";
      default:
        return onFail;
    }
  };

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
          background: "#1e1e1e",
          border: `2px solid ${selected ? "#1890ff" : color}`,
          borderRadius: 8,
          overflow: "hidden",
          boxShadow: selected ? `0 0 0 2px ${color}40` : "none",
          transition: "all 0.2s",
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
          <span style={{ fontSize: 14 }}>✓</span>
          <span
            style={{
              fontSize: 11,
              color: color,
              fontWeight: 600,
            }}
          >
            验证
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

          <Tag
            style={{
              margin: 0,
              fontSize: 10,
              padding: "4px 8px",
              background: `${color}20`,
              border: `1px solid ${color}50`,
              color: color,
              fontWeight: 500,
            }}
          >
            {assertions.length} 个断言
          </Tag>

          <div
            style={{
              marginTop: 6,
              fontSize: 10,
              color: "#888",
            }}
          >
            失败策略: {getOnFailLabel()}
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
          background: "#52c41a",
          border: "none",
          width: 8,
          height: 8,
        }}
      />

      <Handle
        type="source"
        position={Position.Bottom}
        id="fail"
        style={{
          left: "30%",
          background: "#ff4d4f",
          border: "none",
          width: 8,
          height: 8,
        }}
      />
    </div>
  );
};

export const ValidationNode = memo(ValidationNodeComponent);

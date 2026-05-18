import { Tag } from "antd";
import React, { memo } from "react";
import { useTranslation } from "react-i18next";
import { Handle, type NodeProps, Position } from "reactflow";

interface DelayNodeData {
  id: string;
  type: string;
  title: string;
  description?: string;
  color: string;
  nodeType: string;
  enabled: boolean;
  delayType?: string;
  seconds?: number;
  until?: string;
}

const DelayNodeComponent: React.FC<NodeProps<DelayNodeData>> = ({
  data,
  selected,
}) => {
  const { t } = useTranslation();
  const color = "#fa8c16";
  const delayType = data.delayType || "seconds";
  const seconds = data.seconds || 5;

  const formatDelay = (): string => {
    if (delayType === "seconds") {
      if (seconds >= 60) {
        const minutes = Math.floor(seconds / 60);
        const remainingSeconds = seconds % 60;
        return remainingSeconds > 0
          ? `${minutes}${t("workflow.delayNode.minute")}${remainingSeconds}${t("workflow.delayNode.second")}`
          : `${minutes}${t("workflow.delayNode.minutes")}`;
      }
      return `${seconds}${t("workflow.delayNode.second")}`;
    }
    if (delayType === "until" && data.until) {
      return `${t("workflow.delayNode.until")} ${data.until}`;
    }
    return `${seconds}${t("workflow.delayNode.second")}`;
  };

  return (
    <div
      style={{
        minWidth: 140,
        maxWidth: 180,
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
          <span style={{ fontSize: 14 }}>⏱️</span>
          <span
            style={{
              fontSize: 12,
              color: color,
              fontWeight: 600,
            }}
          >
            {t("workflow.delayNode.title")}
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
              fontSize: 12,
              padding: "4px 8px",
              background: `${color}20`,
              border: `1px solid ${color}50`,
              color: color,
              fontWeight: 500,
            }}
          >
            ⏳ {formatDelay()}
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
    </div>
  );
};

export const DelayNode = memo(DelayNodeComponent);

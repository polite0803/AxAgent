import { Tag, theme } from "antd";
import React, { memo } from "react";
import { useTranslation } from "react-i18next";
import { Handle, type NodeProps, Position } from "reactflow";

const PINK_BASE = "#eb2f96";

interface StorageNodeData {
  id: string;
  type: string;
  title: string;
  color: string;
  nodeType: string;
  enabled: boolean;
  config?: {
    backend?: string;
    operation?: string;
    collection?: string;
  };
}

const StorageNodeComponent: React.FC<NodeProps<StorageNodeData>> = ({
  data,
  selected,
}) => {
  const { t } = useTranslation();
  const { token } = theme.useToken();

  const backend = data.config?.backend || "sqlite";
  const operation = data.config?.operation || "insert";

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
          border: `2px solid ${selected ? token.colorPrimary : PINK_BASE}`,
          borderRadius: 8,
          overflow: "hidden",
          boxShadow: selected ? `0 0 0 2px ${PINK_BASE}40` : "none",
          transition: "box-shadow 0.2s, transform 0.2s",
        }}
      >
        <div
          style={{
            padding: "8px 12px",
            borderBottom: `1px solid ${PINK_BASE}30`,
            display: "flex",
            alignItems: "center",
            gap: 8,
            background: `${PINK_BASE}15`,
          }}
        >
          <span style={{ fontSize: 14 }}>💾</span>
          <span style={{ fontSize: 12, color: PINK_BASE, fontWeight: 600 }}>
            {t("workflow.nodeTypes.storage")}
          </span>
        </div>
        <div style={{ padding: "8px 12px" }}>
          <div
            style={{
              fontSize: 13,
              color: token.colorText,
              fontWeight: 500,
              marginBottom: 6,
              overflow: "hidden",
              textOverflow: "ellipsis",
              whiteSpace: "nowrap",
            }}
          >
            {data.title}
          </div>
          <div style={{ display: "flex", gap: 4, flexWrap: "wrap" }}>
            <Tag
              style={{
                margin: 0,
                fontSize: 9,
                padding: "0 4px",
                background: `${PINK_BASE}15`,
                border: `1px solid ${PINK_BASE}40`,
                color: PINK_BASE,
              }}
            >
              {backend}
            </Tag>
            <Tag
              style={{
                margin: 0,
                fontSize: 9,
                padding: "0 4px",
                background: "transparent",
                border: `1px solid ${token.colorBorderSecondary}`,
                color: token.colorTextTertiary,
              }}
            >
              {operation}
            </Tag>
          </div>
        </div>
      </div>
      <Handle
        type="target"
        position={Position.Top}
        style={{ background: PINK_BASE, border: "none", width: 8, height: 8 }}
      />
      <Handle
        type="source"
        position={Position.Bottom}
        style={{ background: PINK_BASE, border: "none", width: 8, height: 8 }}
      />
    </div>
  );
};

export const StorageNode = memo(StorageNodeComponent);

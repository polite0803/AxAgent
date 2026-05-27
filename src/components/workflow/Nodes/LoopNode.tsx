import { Tag, theme } from "antd";
import React, { memo } from "react";
import { useTranslation } from "react-i18next";
import { Handle, type NodeProps, Position } from "reactflow";

const ORANGE_BASE = "#fa8c16";
const ORANGE_VAR = `var(--orange, ${ORANGE_BASE})`;

interface LoopNodeData {
  id: string;
  type: string;
  title: string;
  description?: string;
  color: string;
  nodeType: string;
  enabled: boolean;
  loopType?: "count" | "condition" | "forEach";
  maxIterations?: number;
  loopCondition?: string;
  collectionVar?: string;
}

const LoopNodeComponent: React.FC<NodeProps<LoopNodeData>> = ({
  data,
  selected,
}) => {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const color = ORANGE_VAR;
  const loopType = data.loopType || "count";

  const getLoopDescription = (): string => {
    switch (loopType) {
      case "count":
        return data.maxIterations
          ? `${data.maxIterations}x`
          : t("workflow.loopNode.notConfigured");
      case "condition":
        return data.loopCondition || t("workflow.loopNode.notConfigured");
      case "forEach":
        return data.collectionVar
          ? `∈ ${data.collectionVar}`
          : t("workflow.loopNode.notConfigured");
      default:
        return t("workflow.loopNode.notConfigured");
    }
  };

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
          background: token.colorBgElevated,
          border: `2px solid ${selected ? token.colorPrimary : color}`,
          borderRadius: 8,
          overflow: "hidden",
          boxShadow: selected ? `0 0 0 2px ${color}40` : "none",
          transition: "box-shadow 0.2s, transform 0.2s",
        }}
      >
        <div
          style={{
            padding: "8px 12px",
            borderBottom: `1px solid ${ORANGE_BASE}30`,
            display: "flex",
            alignItems: "center",
            gap: 8,
            background: `${ORANGE_BASE}15`,
          }}
        >
          <span style={{ fontSize: 14 }}>🔁</span>
          <span
            style={{
              fontSize: 12,
              color: color,
              fontWeight: 600,
            }}
          >
            {t("workflow.loopNode.title")}
          </span>
        </div>

        <div style={{ padding: "10px 12px" }}>
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

          <div style={{ display: "flex", flexWrap: "wrap", gap: 4 }}>
            <Tag
              style={{
                margin: 0,
                fontSize: 9,
                padding: "0 4px",
                background: `${ORANGE_BASE}20`,
                border: `1px solid ${ORANGE_BASE}50`,
                color: ORANGE_VAR,
              }}
            >
              {loopType.toUpperCase()}
            </Tag>
            <Tag
              style={{
                margin: 0,
                fontSize: 9,
                padding: "0 4px",
                background: token.colorBgContainer,
                border: `1px solid ${token.colorBorderSecondary}`,
                color: token.colorTextQuaternary,
              }}
            >
              {getLoopDescription()}
            </Tag>
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
        id="loop"
        style={{
          background: token.colorSuccess,
          border: "none",
          width: 8,
          height: 8,
          left: "30%",
        }}
      />

      <Handle
        type="source"
        position={Position.Bottom}
        id="done"
        style={{
          background: token.colorTextTertiary,
          border: "none",
          width: 8,
          height: 8,
          left: "70%",
        }}
      />

      <div
        style={{
          position: "absolute",
          bottom: -18,
          left: "25%",
          transform: "translateX(-50%)",
          fontSize: 9,
          color: token.colorSuccess,
          fontWeight: 600,
        }}
      >
        ↻
      </div>
      <div
        style={{
          position: "absolute",
          bottom: -18,
          left: "75%",
          transform: "translateX(-50%)",
          fontSize: 9,
          color: token.colorTextTertiary,
          fontWeight: 600,
        }}
      >
        →
      </div>
    </div>
  );
};

export const LoopNode = memo(LoopNodeComponent);

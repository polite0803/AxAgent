import { Tag, theme } from "antd";
import React, { memo } from "react";
import { useTranslation } from "react-i18next";
import { Handle, type NodeProps, Position } from "reactflow";
import type { MergeStrategy } from "../types/workflow.types";

const ORANGE_BASE = "#fa8c16";
const ORANGE_VAR = `var(--orange, ${ORANGE_BASE})`;

interface ParallelNodeData {
  id: string;
  type: string;
  title: string;
  description?: string;
  color: string;
  nodeType: string;
  enabled: boolean;
  branches?: number;
  waitStrategy?: "all" | "any" | "race";
  aggregation?: MergeStrategy;
  autoInputFromParent?: boolean;
}

const ParallelNodeComponent: React.FC<NodeProps<ParallelNodeData>> = ({
  data,
  selected,
}) => {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const color = ORANGE_VAR;
  const branches = data.branches || 2;
  const waitStrategy = data.waitStrategy || "all";
  const autoInputFromParent = data.autoInputFromParent !== false;

  const getWaitStrategyLabel = (strategy: string): string => {
    switch (strategy) {
      case "all":
        return t("workflow.parallelNode.waitAll");
      case "any":
        return t("workflow.parallelNode.waitAny");
      case "race":
        return t("workflow.parallelNode.race");
      default:
        return strategy;
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
          <span style={{ fontSize: 14 }}>⚡</span>
          <span
            style={{
              fontSize: 12,
              color: color,
              fontWeight: 600,
            }}
          >
            {t("workflow.parallelNode.title")}
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
              {branches} {t("workflow.parallelNode.branches")}
            </Tag>
            <Tag
              style={{
                margin: 0,
                fontSize: 9,
                padding: "0 4px",
                background: token.colorBgContainer,
                border: `1px solid ${token.colorBorderSecondary}`,
                color: token.colorTextTertiary,
              }}
            >
              {getWaitStrategyLabel(waitStrategy)}
            </Tag>
          </div>

          {autoInputFromParent && (
            <div style={{ marginTop: 6, fontSize: 9, color: token.colorTextTertiary }}>
              {t("workflow.parallelNode.autoInput")}
            </div>
          )}
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

export const ParallelNode = memo(ParallelNodeComponent);

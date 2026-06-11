// SPDX-License-Identifier: AGPL-3.0-only
// @ts-nocheck

import { Handle, type NodeProps, Position } from "@xyflow/react";
import { Tag, theme } from "antd";
import React, { memo } from "react";
import { useTranslation } from "react-i18next";

const ORANGE_BASE = "#fa8c16";
const ORANGE_VAR = `var(--orange, ${ORANGE_BASE})`;

interface ConditionNodeData extends Record<string, unknown> {
  id: string;
  type: string;
  title: string;
  description?: string;
  color: string;
  nodeType: string;
  enabled: boolean;
  conditions?: Array<{
    field: string;
    operator: string;
    value: string;
  }>;
  logicOperator?: "and" | "or";
}

const ConditionNodeComponent: React.FC<NodeProps> = ({
  data,
  selected,
}) => {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const color = ORANGE_VAR;
  const conditions = data.conditions || [];
  const logicOperator = data.logicOperator || "and";

  return (
    <div
      style={{
        minWidth: 200,
        maxWidth: 260,
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
          <span style={{ fontSize: 14 }}>🔀</span>
          <span
            style={{
              fontSize: 12,
              color: color,
              fontWeight: 600,
            }}
          >
            {t("workflow.conditionNode.title")}
          </span>
          {conditions.length > 0 && (
            <Tag
              style={{
                margin: 0,
                fontSize: 9,
                padding: "0 4px",
                background: `${ORANGE_BASE}30`,
                border: "none",
                color: token.colorText,
              }}
            >
              {conditions.length}
            </Tag>
          )}
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

          {conditions.length > 0
            ? (
              <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
                {conditions.slice(0, 3).map((condition, index) => (
                  <div
                    key={index}
                    style={{
                      fontSize: 10,
                      color: token.colorTextQuaternary,
                      padding: "2px 4px",
                      background: token.colorBgContainer,
                      borderRadius: 3,
                      overflow: "hidden",
                      textOverflow: "ellipsis",
                      whiteSpace: "nowrap",
                    }}
                  >
                    {condition.field} {condition.operator} {condition.value}
                  </div>
                ))}
                {conditions.length > 3 && (
                  <div
                    style={{
                      fontSize: 10,
                      color: token.colorTextTertiary,
                    }}
                  >
                    +{conditions.length - 3} {t("workflow.conditionNode.moreConditions")}
                  </div>
                )}
                <div
                  style={{
                    fontSize: 9,
                    color: token.colorTextTertiary,
                    marginTop: 2,
                  }}
                >
                  {logicOperator.toUpperCase()}
                </div>
              </div>
            )
            : (
              <div
                style={{
                  fontSize: 12,
                  color: token.colorTextQuaternary,
                }}
              >
                {t("workflow.conditionNode.notSet")}
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
        id="true"
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
        id="false"
        style={{
          background: token.colorError,
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
        ✓
      </div>
      <div
        style={{
          position: "absolute",
          bottom: -18,
          left: "75%",
          transform: "translateX(-50%)",
          fontSize: 9,
          color: token.colorError,
          fontWeight: 600,
        }}
      >
        ✗
      </div>
    </div>
  );
};

export const ConditionNode = memo(ConditionNodeComponent);

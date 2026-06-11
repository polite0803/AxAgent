// SPDX-License-Identifier: AGPL-3.0-only
// @ts-nocheck

import { Handle, type NodeProps, Position } from "@xyflow/react";
import { Tag, theme } from "antd";
import React, { memo } from "react";
import { useTranslation } from "react-i18next";

const PURPLE_BASE = "#722ed1";
const PURPLE_VAR = `var(--purple, ${PURPLE_BASE})`;

interface ValidationNodeData extends Record<string, unknown> {
  id: string;
  type: string;
  title: string;
  description?: string;
  color: string;
  nodeType: string;
  enabled: boolean;
  validationType?: "schema" | "rules" | "custom";
  rules?: Array<{
    field: string;
    rule: string;
    message?: string;
  }>;
  failAction?: "error" | "warning" | "skip";
}

const ValidationNodeComponent: React.FC<NodeProps> = ({
  data,
  selected,
}) => {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const color = PURPLE_VAR;
  const validationType = data.validationType || "rules";
  const rules = data.rules || [];
  const failAction = data.failAction || "error";

  const getFailActionColor = (action: string): string => {
    switch (action) {
      case "error":
        return token.colorError;
      case "warning":
        return token.colorWarning;
      case "skip":
        return token.colorSuccess;
      default:
        return token.colorTextTertiary;
    }
  };

  return (
    <div
      style={{
        minWidth: 200,
        maxWidth: 240,
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
            borderBottom: `1px solid ${PURPLE_BASE}30`,
            display: "flex",
            alignItems: "center",
            gap: 8,
            background: `${PURPLE_BASE}15`,
          }}
        >
          <span style={{ fontSize: 14 }}>✅</span>
          <span
            style={{
              fontSize: 12,
              color: color,
              fontWeight: 600,
            }}
          >
            {t("workflow.validationNode.title")}
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
                background: `${PURPLE_BASE}20`,
                border: `1px solid ${PURPLE_BASE}50`,
                color: PURPLE_VAR,
              }}
            >
              {validationType.toUpperCase()}
            </Tag>

            {rules.length > 0 && (
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
                {rules.length} {t("workflow.validationNode.rules")}
              </Tag>
            )}

            <Tag
              style={{
                margin: 0,
                fontSize: 9,
                padding: "0 4px",
                background: `${getFailActionColor(failAction)}20`,
                border: `1px solid ${getFailActionColor(failAction)}50`,
                color: getFailActionColor(failAction),
              }}
            >
              {failAction.toUpperCase()}
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
        id="valid"
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
        id="invalid"
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

export const ValidationNode = memo(ValidationNodeComponent);

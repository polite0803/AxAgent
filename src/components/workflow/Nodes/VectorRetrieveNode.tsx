// SPDX-License-Identifier: AGPL-3.0-only

import { Handle, type NodeProps, Position } from "@xyflow/react";
import { Tag, theme } from "antd";
import React, { memo } from "react";
import { useTranslation } from "react-i18next";

const MAGENTA_BASE = "#eb2f96";
const MAGENTA_VAR = `var(--magenta, ${MAGENTA_BASE})`;

interface VectorRetrieveNodeData {
  id: string;
  type: string;
  title: string;
  description?: string;
  color: string;
  nodeType: string;
  enabled: boolean;
  knowledgeBaseId?: string;
  knowledgeBaseName?: string;
  topK?: number;
  similarityThreshold?: number;
  outputVar?: string;
}

const VectorRetrieveNodeComponent: React.FC<NodeProps> = ({ data: _data, selected }) => {
  const data = _data as unknown as VectorRetrieveNodeData;
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const color = MAGENTA_VAR;
  const knowledgeBaseName = data.knowledgeBaseName;
  const topK = data.topK || 5;
  const similarityThreshold = data.similarityThreshold || 0.7;

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
            borderBottom: `1px solid ${MAGENTA_BASE}30`,
            display: "flex",
            alignItems: "center",
            gap: 8,
            background: `${MAGENTA_BASE}15`,
          }}
        >
          <span style={{ fontSize: 14 }}>🔍</span>
          <span
            style={{
              fontSize: 12,
              color: color,
              fontWeight: 600,
            }}
          >
            {t("workflow.vectorRetrieveNode.title")}
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

          {knowledgeBaseName && (
            <div
              style={{
                fontSize: 12,
                color: color,
                marginBottom: 6,
                padding: "4px 6px",
                background: `${MAGENTA_BASE}15`,
                borderRadius: 4,
                overflow: "hidden",
                textOverflow: "ellipsis",
                whiteSpace: "nowrap",
                fontWeight: 500,
              }}
            >
              📚 {knowledgeBaseName}
            </div>
          )}

          <div style={{ display: "flex", flexWrap: "wrap", gap: 4 }}>
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
              Top {topK}
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
              ≥{(similarityThreshold * 100).toFixed(0)}%
            </Tag>

            {data.outputVar && (
              <Tag
                style={{
                  margin: 0,
                  fontSize: 9,
                  padding: "0 4px",
                  background: `${token.colorPrimary}20`,
                  border: `1px solid ${token.colorPrimary}50`,
                  color: token.colorPrimary,
                }}
              >
                📤 {data.outputVar}
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

export const VectorRetrieveNode = memo(VectorRetrieveNodeComponent);

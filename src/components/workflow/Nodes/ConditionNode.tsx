// SPDX-License-Identifier: AGPL-3.0-only

import { Handle, type NodeProps, Position } from "@xyflow/react";
import { theme } from "antd";
import React, { memo } from "react";

const ORANGE_BASE = "#fa8c16";

interface ConditionNodeData {
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

const ConditionNodeComponent: React.FC<NodeProps> = ({ data: _data, selected }) => {
  const data = _data as unknown as ConditionNodeData;
  const { token } = theme.useToken();
  const color = ORANGE_BASE;
  const conditions = data.conditions || [];

  return (
    <div
      style={{
        minWidth: 120,
        maxWidth: 180,
        opacity: data.enabled ? 1 : 0.5,
        filter: data.enabled ? "none" : "grayscale(100%)",
      }}
    >
      <div
        title={data.description || data.title}
        style={{
          background: token.colorBgContainer,
          border: `1.5px solid ${selected ? token.colorPrimary : color}`,
          borderRadius: 8,
          overflow: "hidden",
          boxShadow: selected
            ? `0 0 0 1.5px ${color}40`
            : "0 1px 3px rgba(0,0,0,0.08)",
          transition: "box-shadow 0.15s",
        }}
      >
        {/* n8n 风格：单行 — 图标色块 + 标题 + 条件数 */}
        <div
          style={{
            padding: "6px 10px",
            display: "flex",
            alignItems: "center",
            gap: 6,
          }}
        >
          <div
            style={{
              width: 22,
              height: 22,
              borderRadius: 4,
              background: `${color}18`,
              border: `1px solid ${color}30`,
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
              fontSize: 12,
              flexShrink: 0,
              lineHeight: 1,
            }}
          >
            🔀
          </div>
          <span
            style={{
              fontSize: 11,
              color: token.colorText,
              fontWeight: 500,
              flex: 1,
              overflow: "hidden",
              textOverflow: "ellipsis",
              whiteSpace: "nowrap",
              lineHeight: "22px",
            }}
          >
            {data.title}
          </span>
          {conditions.length > 0 && (
            <span
              style={{
                fontSize: 9,
                lineHeight: "14px",
                padding: "0 4px",
                borderRadius: 3,
                background: `${color}15`,
                border: `1px solid ${color}30`,
                color: color,
                fontWeight: 600,
                flexShrink: 0,
              }}
            >
              {conditions.length}
            </span>
          )}
        </div>
      </div>

      <Handle
        type="target"
        position={Position.Top}
        style={{
          background: color,
          border: "none",
          width: 7,
          height: 7,
        }}
      />

      <Handle
        type="source"
        position={Position.Bottom}
        id="true"
        style={{
          background: token.colorSuccess,
          border: "none",
          width: 7,
          height: 7,
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
          width: 7,
          height: 7,
          left: "70%",
        }}
      />

      {/* True/False 标签 */}
      <div
        style={{
          position: "absolute",
          bottom: -14,
          left: "25%",
          transform: "translateX(-50%)",
          fontSize: 8,
          color: token.colorSuccess,
          fontWeight: 600,
        }}
      >
        T
      </div>
      <div
        style={{
          position: "absolute",
          bottom: -14,
          left: "75%",
          transform: "translateX(-50%)",
          fontSize: 8,
          color: token.colorError,
          fontWeight: 600,
        }}
      >
        F
      </div>
    </div>
  );
};

export const ConditionNode = memo(ConditionNodeComponent);

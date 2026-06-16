// SPDX-License-Identifier: AGPL-3.0-only

import { Handle, type NodeProps, Position } from "@xyflow/react";
import { theme } from "antd";
import React, { memo } from "react";

const NODE_COLOR = "#fa8c16";

interface NotificationNodeData {
  id: string;
  type: string;
  title: string;
  color: string;
  nodeType: string;
  enabled: boolean;
}

const NotificationNodeComponent: React.FC<NodeProps> = ({ data: _data, selected }) => {
  const data = _data as unknown as NotificationNodeData;
  const { token } = theme.useToken();

  const borderColor = selected ? token.colorPrimary : NODE_COLOR;

  return (
    <div
      style={{
        minWidth: 120,
        maxWidth: 200,
        opacity: data.enabled ? 1 : 0.5,
        filter: data.enabled ? "none" : "grayscale(100%)",
      }}
    >
      <div
        className="workflow-node-card"
        title={data.title}
        style={{
          background: token.colorBgContainer,
          border: `1.5px solid ${borderColor}`,
          borderRadius: 8,
          padding: 0,
          boxShadow: selected
            ? `0 0 0 1.5px ${borderColor}40`
            : "0 1px 3px rgba(0,0,0,0.08)",
          transition: "box-shadow 0.15s",
        }}
      >
        {/* n8n 风格：单行 — 图标色块 + 标题 */}
        <div
          style={{
            padding: "6px 10px",
            display: "flex",
            alignItems: "center",
            gap: 6,
          }}
        >
          {/* 图标色块 */}
          <div
            style={{
              width: 22,
              height: 22,
              borderRadius: 4,
              background: `${NODE_COLOR}18`,
              border: `1px solid ${NODE_COLOR}30`,
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
              fontSize: 12,
              flexShrink: 0,
              lineHeight: 1,
            }}
          >
            🔔
          </div>

          {/* 标题 */}
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
        </div>
      </div>

      <Handle
        type="target"
        position={Position.Top}
        style={{ background: NODE_COLOR, border: "none", width: 7, height: 7 }}
      />
      <Handle
        type="source"
        position={Position.Bottom}
        style={{ background: NODE_COLOR, border: "none", width: 7, height: 7 }}
      />
    </div>
  );
};

export const NotificationNode = memo(NotificationNodeComponent);

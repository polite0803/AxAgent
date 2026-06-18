// SPDX-License-Identifier: AGPL-3.0-only

import { getHandlePosition, getNodeSize, PORT_SIZE } from "@/lib/workflowLayout";
import { Handle, type NodeProps, Position } from "@xyflow/react";
import { theme, Tooltip } from "antd";
import React, { memo } from "react";

const NODE_COLOR = "#722ed1";

interface WorkflowRefNodeData {
  id: string;
  type: string;
  title: string;
  color: string;
  nodeType: string;
  enabled: boolean;
  target_workflow_id?: string;
}

const WorkflowRefNodeComponent: React.FC<NodeProps> = ({ data: _data, selected }) => {
  const data = _data as unknown as WorkflowRefNodeData;
  const { token } = theme.useToken();

  const borderColor = selected ? token.colorPrimary : NODE_COLOR;
  const collapsedSize = getNodeSize("workflowRef");

  return (
    <div
      style={{
        width: collapsedSize.width,
        height: collapsedSize.height,
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
            🔗
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

          {!data.target_workflow_id && (
            <Tooltip title="No workflow referenced">
              <span style={{ fontSize: 10, color: token.colorError, lineHeight: 1 }}>⚠</span>
            </Tooltip>
          )}
        </div>
      </div>

      <Handle
        type="target"
        position={Position.Top}
        style={{
          background: NODE_COLOR,
          border: "none",
          width: PORT_SIZE,
          height: PORT_SIZE,
          ...getHandlePosition(collapsedSize.width, collapsedSize.height, "top"),
        }}
      />
      <Handle
        type="source"
        position={Position.Bottom}
        style={{
          background: NODE_COLOR,
          border: "none",
          width: PORT_SIZE,
          height: PORT_SIZE,
          ...getHandlePosition(collapsedSize.width, collapsedSize.height, "bottom"),
        }}
      />
    </div>
  );
};

export const WorkflowRefNode = memo(WorkflowRefNodeComponent);

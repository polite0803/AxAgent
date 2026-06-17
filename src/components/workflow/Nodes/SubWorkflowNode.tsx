// SPDX-License-Identifier: AGPL-3.0-only

import { getHandlePosition, getNodeSize, PORT_SIZE } from "@/lib/workflowLayout";
import { useWorkflowEditorStore } from "@/stores";
import { Handle, type NodeProps, Position } from "@xyflow/react";
import { theme, Tooltip } from "antd";
import React, { memo, useCallback } from "react";

const NODE_COLOR = "#eb2f96";

interface SubWorkflowNodeData {
  id: string;
  type: string;
  title: string;
  color: string;
  nodeType: string;
  enabled: boolean;
  subWorkflowId?: string;
  subWorkflowName?: string;
  target_workflow_id?: string;
  nodeWidth?: number;
  nodeHeight?: number;
}

const SubWorkflowNodeComponent: React.FC<NodeProps> = ({ data: _data, selected }) => {
  const data = _data as unknown as SubWorkflowNodeData;
  const { token } = theme.useToken();

  const workflowId = data.subWorkflowId || data.target_workflow_id;

  const expandedData = useWorkflowEditorStore((s) => s.expandedSubWorkflows[data.id]);
  const toggleExpand = useCallback(() => {
    useWorkflowEditorStore.getState().toggleExpandSubWorkflow(data.id, workflowId);
  }, [data.id, workflowId]);

  const isExpanded = !!expandedData && !expandedData.isLoading;
  const isLoading = !!expandedData?.isLoading;
  const childCount = isExpanded ? expandedData?.nodes?.length || 0 : 0;
  const childEdgeCount = isExpanded ? expandedData?.edges?.length || 0 : 0;

  const borderColor = selected ? token.colorPrimary : NODE_COLOR;

  if (isExpanded) {
    return (
      <div
        style={{
          width: data.nodeWidth ?? 400,
          height: data.nodeHeight ?? 200,
          minWidth: 200,
          minHeight: 80,
          background: `${NODE_COLOR}06`,
          border: `1.5px dashed ${borderColor}50`,
          borderRadius: 8,
          padding: 8,
          opacity: data.enabled ? 1 : 0.5,
          position: "relative",
          transition: "opacity 0.15s, border-color 0.15s",
        }}
      >
        {/* 紧凑标题栏 */}
        <div
          className="workflow-container-drag-handle"
          style={{
            display: "flex",
            alignItems: "center",
            gap: 5,
            background: token.colorBgElevated,
            border: `1px solid ${NODE_COLOR}20`,
            borderRadius: 4,
            padding: "3px 6px",
            position: "absolute",
            top: 6,
            left: 8,
            zIndex: 10,
            cursor: "grab",
          }}
        >
          {/* 图标色块 */}
          <div
            style={{
              width: 18,
              height: 18,
              borderRadius: 3,
              background: `${NODE_COLOR}18`,
              border: `1px solid ${NODE_COLOR}30`,
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
              fontSize: 10,
              flexShrink: 0,
              lineHeight: 1,
            }}
          >
            🔄
          </div>
          <span style={{ fontSize: 11, color: NODE_COLOR, fontWeight: 600, lineHeight: "18px" }}>
            {data.title}
          </span>
          <span
            style={{
              fontSize: 9,
              color: NODE_COLOR,
              background: `${NODE_COLOR}12`,
              border: `1px solid ${NODE_COLOR}30`,
              padding: "0 4px",
              borderRadius: 2,
              lineHeight: "16px",
              fontWeight: 600,
            }}
          >
            🔓 {childCount} nodes · {childEdgeCount} edges
          </span>
        </div>

        <Tooltip title="Collapse">
          <span
            className="react-flow__nodrag"
            onClick={(e) => {
              e.stopPropagation();
              toggleExpand();
            }}
            style={{
              position: "absolute",
              top: 6,
              right: 8,
              cursor: "pointer",
              fontSize: 10,
              lineHeight: 1,
              padding: "3px 5px",
              borderRadius: 3,
              background: token.colorBgElevated,
              border: `1px solid ${NODE_COLOR}20`,
              zIndex: 10,
              opacity: 0.6,
              transition: "opacity 0.15s",
              userSelect: "none",
            }}
            onMouseEnter={(e) => {
              (e.currentTarget as HTMLElement).style.opacity = "1";
            }}
            onMouseLeave={(e) => {
              (e.currentTarget as HTMLElement).style.opacity = "0.6";
            }}
          >
            ▼
          </span>
        </Tooltip>

        <Handle
          type="target"
          position={Position.Top}
          style={{
            background: NODE_COLOR,
            border: "none",
            width: PORT_SIZE,
            height: PORT_SIZE,
            ...getHandlePosition(data.nodeWidth ?? 400, data.nodeHeight ?? 200, "top"),
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
            ...getHandlePosition(data.nodeWidth ?? 400, data.nodeHeight ?? 200, "bottom"),
          }}
        />
      </div>
    );
  }

  // Collapsed: n8n compact style
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
        {/* n8n 风格：单行 — 图标色块 + 标题 + 展开按钮 */}
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
            🔄
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

          {/* 展开按钮 */}
          {workflowId && (
            <Tooltip title="Expand">
              <span
                className="react-flow__nodrag"
                onClick={(e) => {
                  e.stopPropagation();
                  toggleExpand();
                }}
                style={{
                  cursor: "pointer",
                  fontSize: 10,
                  lineHeight: 1,
                  padding: "2px 4px",
                  borderRadius: 3,
                  opacity: isLoading ? 0.5 : 0.6,
                  transition: "opacity 0.15s",
                  userSelect: "none",
                }}
                onMouseEnter={(e) => {
                  (e.currentTarget as HTMLElement).style.opacity = "1";
                }}
                onMouseLeave={(e) => {
                  (e.currentTarget as HTMLElement).style.opacity = "0.6";
                }}
              >
                {isLoading ? "⏳" : "▶"}
              </span>
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

export const SubWorkflowNode = memo(SubWorkflowNodeComponent);

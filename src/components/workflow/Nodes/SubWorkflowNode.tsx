// SPDX-License-Identifier: AGPL-3.0-only

import { getHandlePosition, getNodeSize, PORT_SIZE } from "@/lib/workflowLayout";
import { useWorkflowEditorStore } from "@/stores";
import { LoadingOutlined, UnlockOutlined } from "@ant-design/icons";
import { Handle, type NodeProps, Position } from "@xyflow/react";
import { theme, Tooltip } from "antd";
import React, { memo, useCallback } from "react";
import { useTranslation } from "react-i18next";
import { NodeCard } from "./NodeCard";
import { nodeIconFor } from "./nodeIcons";

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
  const data = _data as unknown as SubWorkflowNodeData; // SAFE: ReactFlow NodeProps.data is untyped; runtime data matches expected component data
  const { t } = useTranslation();
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
              flexShrink: 0,
              lineHeight: 1,
            }}
          >
            {nodeIconFor("subWorkflow")}
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
              display: "inline-flex",
              alignItems: "center",
              gap: 3,
            }}
          >
            <UnlockOutlined style={{ fontSize: 9 }} />
            {childCount} nodes · {childEdgeCount} edges
          </span>
        </div>

        <Tooltip title={t("subWorkflowNode.collapse")}>
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
          id="in"
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
          id="out"
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
    <NodeCard
      nodeType="subWorkflow"
      title={data.title}
      accent={NODE_COLOR}
      selected={selected}
      enabled={data.enabled}
      icon={nodeIconFor("subWorkflow")}
      wrapperStyle={{ width: collapsedSize.width, height: collapsedSize.height }}
      rightSlot={workflowId
        ? (
          <Tooltip title={t("subWorkflowNode.expand")}>
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
              {isLoading ? <LoadingOutlined /> : "▶"}
            </span>
          </Tooltip>
        )
        : undefined}
    >
      <Handle
        type="target"
        id="in"
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
        id="out"
        position={Position.Bottom}
        style={{
          background: NODE_COLOR,
          border: "none",
          width: PORT_SIZE,
          height: PORT_SIZE,
          ...getHandlePosition(collapsedSize.width, collapsedSize.height, "bottom"),
        }}
      />
    </NodeCard>
  );
};

export const SubWorkflowNode = memo(SubWorkflowNodeComponent);

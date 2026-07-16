// SPDX-License-Identifier: AGPL-3.0-only

import { getHandlePosition, getNodeSize, PORT_SIZE } from "@/lib/workflowLayout";
import { Handle, type NodeProps, Position } from "@xyflow/react";
import { theme, Tooltip } from "antd";
import React, { memo } from "react";
import { useTranslation } from "react-i18next";
import { NodeCard } from "./NodeCard";

interface WorkflowRefNodeData {
  id: string;
  type: string;
  title: string;
  color: string;
  nodeType: string;
  enabled: boolean;
  description?: string;
  target_workflow_id?: string;
}

const WorkflowRefNodeComponent: React.FC<NodeProps> = ({ data: _data, selected }) => {
  const data = _data as unknown as WorkflowRefNodeData; // SAFE: ReactFlow NodeProps.data is untyped; runtime data matches expected component data
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const collapsedSize = getNodeSize("workflowRef");

  return (
    <NodeCard
      nodeType={data.nodeType}
      title={data.title}
      accent={data.color}
      selected={selected}
      enabled={data.enabled}
      description={data.description}
      wrapperStyle={{ width: collapsedSize.width, height: collapsedSize.height }}
      rightSlot={!data.target_workflow_id
        ? (
          <Tooltip title={t("workflowRefNode.noWorkflow")}>
            <span style={{ fontSize: 10, color: token.colorError, lineHeight: 1 }}>⚠</span>
          </Tooltip>
        )
        : undefined}
    >
      <Handle
        type="target"
        position={Position.Top}
        style={{
          background: data.color,
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
          background: data.color,
          border: "none",
          width: PORT_SIZE,
          height: PORT_SIZE,
          ...getHandlePosition(collapsedSize.width, collapsedSize.height, "bottom"),
        }}
      />
    </NodeCard>
  );
};

export const WorkflowRefNode = memo(WorkflowRefNodeComponent);

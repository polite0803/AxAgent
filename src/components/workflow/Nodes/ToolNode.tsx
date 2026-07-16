// SPDX-License-Identifier: AGPL-3.0-only

import { getHandlePosition, getNodeSize, PORT_SIZE } from "@/lib/workflowLayout";
import { Handle, type NodeProps, Position } from "@xyflow/react";
import React, { memo } from "react";
import { NodeCard } from "./NodeCard";

interface ToolNodeData {
  id: string;
  type: string;
  title: string;
  color: string;
  nodeType: string;
  enabled: boolean;
  description?: string;
}

const ToolNodeComponent: React.FC<NodeProps> = ({ data: _data, selected }) => {
  const data = _data as unknown as ToolNodeData; // SAFE: ReactFlow NodeProps.data is untyped; runtime data matches expected component data
  const nodeSize = getNodeSize(data.type);
  return (
    <NodeCard
      nodeType={data.nodeType}
      title={data.title}
      accent={data.color}
      selected={selected}
      enabled={data.enabled}
      description={data.description}
    >
      <Handle
        type="target"
        position={Position.Top}
        style={{
          background: data.color,
          border: "none",
          width: PORT_SIZE,
          height: PORT_SIZE,
          ...getHandlePosition(nodeSize.width, nodeSize.height, "top"),
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
          ...getHandlePosition(nodeSize.width, nodeSize.height, "bottom"),
        }}
      />
    </NodeCard>
  );
};

export const ToolNode = memo(ToolNodeComponent);

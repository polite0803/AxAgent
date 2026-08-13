// SPDX-License-Identifier: AGPL-3.0-only

import { Handle, type NodeProps, Position } from "@xyflow/react";
import React, { memo } from "react";
import { NodeCard } from "./NodeCard";

interface ApprovalNodeData {
  id: string;
  type: string;
  title: string;
  color: string;
  nodeType: string;
  enabled: boolean;
  description?: string;
}

const ApprovalNodeComponent: React.FC<NodeProps> = ({ data: _data, selected }) => {
  const data = _data as unknown as ApprovalNodeData; // SAFE: ReactFlow NodeProps.data is untyped; runtime data matches expected component data
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
        id="in"
        position={Position.Top}
        style={{ background: data.color, border: "none", width: 7, height: 7 }}
      />
      <Handle
        type="source"
        id="out"
        position={Position.Bottom}
        style={{ background: data.color, border: "none", width: 7, height: 7 }}
      />
    </NodeCard>
  );
};

export const ApprovalNode = memo(ApprovalNodeComponent);

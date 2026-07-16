// SPDX-License-Identifier: AGPL-3.0-only

import { Handle, type NodeProps, Position } from "@xyflow/react";
import { theme } from "antd";
import React, { memo } from "react";
import { NodeCard } from "./NodeCard";

interface ValidationNodeData {
  id: string;
  type: string;
  title: string;
  color: string;
  nodeType: string;
  enabled: boolean;
  description?: string;
}

const ValidationNodeComponent: React.FC<NodeProps> = ({ data: _data, selected }) => {
  const data = _data as unknown as ValidationNodeData; // SAFE: ReactFlow NodeProps.data is untyped; runtime data matches expected component data
  const { token } = theme.useToken();
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
        style={{ background: data.color, border: "none", width: 7, height: 7 }}
      />
      <Handle
        type="source"
        position={Position.Bottom}
        id="valid"
        style={{ background: token.colorSuccess, border: "none", width: 7, height: 7, left: "30%" }}
      />
      <Handle
        type="source"
        position={Position.Bottom}
        id="invalid"
        style={{ background: token.colorError, border: "none", width: 7, height: 7, left: "70%" }}
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
    </NodeCard>
  );
};

export const ValidationNode = memo(ValidationNodeComponent);

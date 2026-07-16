// SPDX-License-Identifier: AGPL-3.0-only

import { Handle, type NodeProps, Position } from "@xyflow/react";
import { theme } from "antd";
import React, { memo } from "react";
import { NodeCard } from "./NodeCard";
import { nodeIconFor } from "./nodeIcons";

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
  const data = _data as unknown as ConditionNodeData; // SAFE: ReactFlow NodeProps.data is untyped; runtime data matches expected component data
  const { token } = theme.useToken();
  const color = ORANGE_BASE;
  const conditions = data.conditions || [];

  return (
    <NodeCard
      nodeType="condition"
      title={data.title}
      accent={color}
      selected={selected}
      enabled={data.enabled}
      description={data.description}
      icon={nodeIconFor("condition")}
      rightSlot={conditions.length > 0
        ? (
          <span
            style={{
              fontSize: 9,
              lineHeight: "14px",
              padding: "0 4px",
              borderRadius: 3,
              background: `${color}15`,
              border: `1px solid ${color}30`,
              color,
              fontWeight: 600,
              flexShrink: 0,
            }}
          >
            {conditions.length}
          </span>
        )
        : undefined}
    >
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
    </NodeCard>
  );
};

export const ConditionNode = memo(ConditionNodeComponent);

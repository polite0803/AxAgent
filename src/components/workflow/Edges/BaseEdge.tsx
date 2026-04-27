import React from "react";
import { EdgeLabelRenderer, type EdgeProps, getBezierPath } from "reactflow";

interface BaseEdgeData {
  edgeType: string;
}

const BaseEdgeComponent: React.FC<EdgeProps<BaseEdgeData>> = ({
  id,
  sourceX,
  sourceY,
  targetX,
  targetY,
  sourcePosition,
  targetPosition,
  data,
  selected,
  label,
}) => {
  const [edgePath, labelX, labelY] = getBezierPath({
    sourceX,
    sourceY,
    sourcePosition,
    targetX,
    targetY,
    targetPosition,
  });

  const edgeColor = selected ? "#1890ff" : "#555";
  const isAnimated = data?.edgeType === "loopBack";

  return (
    <>
      <path
        id={id}
        className="react-flow__edge-path"
        d={edgePath}
        stroke={edgeColor}
        strokeWidth={selected ? 2 : 1.5}
        fill="none"
        style={{
          strokeDasharray: data?.edgeType === "error" ? "5,5" : undefined,
        }}
        markerEnd={`url(#arrow-${data?.edgeType || "default"})`}
      />
      {isAnimated && (
        <path
          d={edgePath}
          stroke={edgeColor}
          strokeWidth={2}
          fill="none"
          strokeDasharray="5,5"
          style={{
            animation: "dash 0.5s linear infinite",
          }}
        >
          <animate attributeName="stroke-dashoffset" from="0" to="10" dur="0.5s" repeatCount="indefinite" />
        </path>
      )}
      {label && (
        <EdgeLabelRenderer>
          <div
            style={{
              position: "absolute",
              transform: `translate(-50%, -50%) translate(${labelX}px,${labelY}px)`,
              fontSize: 10,
              color: "#999",
              background: "#1a1a1a",
              padding: "2px 6px",
              borderRadius: 4,
              border: "1px solid #333",
              pointerEvents: "all",
            }}
          >
            {label}
          </div>
        </EdgeLabelRenderer>
      )}
      <defs>
        <marker
          id="arrow-default"
          viewBox="0 0 10 10"
          refX="8"
          refY="5"
          markerWidth="6"
          markerHeight="6"
          orient="auto-start-reverse"
        >
          <path d="M 0 0 L 10 5 L 0 10 z" fill={edgeColor} />
        </marker>
        <marker
          id="arrow-direct"
          viewBox="0 0 10 10"
          refX="8"
          refY="5"
          markerWidth="6"
          markerHeight="6"
          orient="auto-start-reverse"
        >
          <path d="M 0 0 L 10 5 L 0 10 z" fill={edgeColor} />
        </marker>
        <marker
          id="arrow-conditionTrue"
          viewBox="0 0 10 10"
          refX="8"
          refY="5"
          markerWidth="6"
          markerHeight="6"
          orient="auto-start-reverse"
        >
          <path d="M 0 0 L 10 5 L 0 10 z" fill="#52c41a" />
        </marker>
        <marker
          id="arrow-conditionFalse"
          viewBox="0 0 10 10"
          refX="8"
          refY="5"
          markerWidth="6"
          markerHeight="6"
          orient="auto-start-reverse"
        >
          <path d="M 0 0 L 10 5 L 0 10 z" fill="#ff4d4f" />
        </marker>
        <marker
          id="arrow-loopBack"
          viewBox="0 0 10 10"
          refX="8"
          refY="5"
          markerWidth="6"
          markerHeight="6"
          orient="auto-start-reverse"
        >
          <path d="M 0 0 L 10 5 L 0 10 z" fill="#fa8c16" />
        </marker>
        <marker
          id="arrow-error"
          viewBox="0 0 10 10"
          refX="8"
          refY="5"
          markerWidth="6"
          markerHeight="6"
          orient="auto-start-reverse"
        >
          <path d="M 0 0 L 10 5 L 0 10 z" fill="#ff4d4f" />
        </marker>
        <marker
          id="arrow-parallelBranch"
          viewBox="0 0 10 10"
          refX="8"
          refY="5"
          markerWidth="6"
          markerHeight="6"
          orient="auto-start-reverse"
        >
          <path d="M 0 0 L 10 5 L 0 10 z" fill="#722ed1" />
        </marker>
        <marker
          id="arrow-merge"
          viewBox="0 0 10 10"
          refX="8"
          refY="5"
          markerWidth="6"
          markerHeight="6"
          orient="auto-start-reverse"
        >
          <path d="M 0 0 L 10 5 L 0 10 z" fill="#1890ff" />
        </marker>
      </defs>
    </>
  );
};

export { BaseEdgeComponent as BaseEdge };

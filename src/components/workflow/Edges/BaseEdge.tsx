import { useWorkEngineStore } from "@/stores/feature/workEngineStore";
import { theme } from "antd";
import React from "react";
import { EdgeLabelRenderer, type EdgeProps, getBezierPath } from "reactflow";

const ORANGE_BASE = "#fa8c16";
const PURPLE_BASE = "#722ed1";

interface BaseEdgeData {
  edgeType: string;
}

const BaseEdgeComponent: React.FC<EdgeProps<BaseEdgeData>> = ({
  id,
  source,
  target,
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
  const { token } = theme.useToken();

  const nodeStatuses = useWorkEngineStore((s) => s.nodeStatuses);
  const isDebugRunning = useWorkEngineStore((s) => s.isDebugRunning);

  const sourceRunning = nodeStatuses[source] === "running" || nodeStatuses[source] === "completed";
  const targetActive = nodeStatuses[target!] === "running" || nodeStatuses[target!] === "completed";
  const showFlowAnimation = isDebugRunning && (sourceRunning || targetActive);

  const [edgePath, labelX, labelY] = getBezierPath({
    sourceX,
    sourceY,
    sourcePosition,
    targetX,
    targetY,
    targetPosition,
  });

  const edgeColor = selected ? token.colorPrimary : token.colorBorderSecondary;
  const isAnimated = data?.edgeType === "loopBack";

  const getMarkerColor = (edgeType?: string): string => {
    switch (edgeType) {
      case "conditionTrue":
        return token.colorSuccess;
      case "conditionFalse":
      case "error":
        return token.colorError;
      case "loopBack":
        return `var(--orange, ${ORANGE_BASE})`;
      case "parallelBranch":
        return `var(--purple, ${PURPLE_BASE})`;
      case "merge":
        return token.colorPrimary;
      default:
        return edgeColor;
    }
  };

  const getEdgeStroke = () => {
    if (showFlowAnimation) {
      if (data?.edgeType === "conditionTrue") { return token.colorSuccess; }
      if (data?.edgeType === "conditionFalse") { return token.colorError; }
      return token.colorPrimary;
    }
    return edgeColor;
  };

  return (
    <>
      <path
        id={id}
        className="react-flow__edge-path"
        d={edgePath}
        stroke={getEdgeStroke()}
        strokeWidth={selected || showFlowAnimation ? 2 : 1.5}
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
        />
      )}
      {showFlowAnimation && !isAnimated && (
        <path
          d={edgePath}
          stroke={getEdgeStroke()}
          strokeWidth={2}
          fill="none"
          strokeDasharray="8,4"
          opacity={0.6}
          style={{
            animation: "dash 0.6s linear infinite",
          }}
        />
      )}
      {label && (
        <EdgeLabelRenderer>
          <div
            style={{
              position: "absolute",
              transform: `translate(-50%, -50%) translate(${labelX}px,${labelY}px)`,
              fontSize: 12,
              color: token.colorTextTertiary,
              background: token.colorBgElevated,
              padding: "2px 6px",
              borderRadius: 4,
              border: `1px solid ${token.colorBorderSecondary}`,
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
          <path d="M 0 0 L 10 5 L 0 10 z" fill={getMarkerColor("default")} />
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
          <path d="M 0 0 L 10 5 L 0 10 z" fill={getMarkerColor("direct")} />
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
          <path d="M 0 0 L 10 5 L 0 10 z" fill={getMarkerColor("conditionTrue")} />
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
          <path d="M 0 0 L 10 5 L 0 10 z" fill={getMarkerColor("conditionFalse")} />
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
          <path d="M 0 0 L 10 5 L 0 10 z" fill={getMarkerColor("loopBack")} />
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
          <path d="M 0 0 L 10 5 L 0 10 z" fill={getMarkerColor("error")} />
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
          <path d="M 0 0 L 10 5 L 0 10 z" fill={getMarkerColor("parallelBranch")} />
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
          <path d="M 0 0 L 10 5 L 0 10 z" fill={getMarkerColor("merge")} />
        </marker>
      </defs>
    </>
  );
};

export { BaseEdgeComponent as BaseEdge };

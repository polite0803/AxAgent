// SPDX-License-Identifier: AGPL-3.0-only

import { useWorkEngineStore } from "@/stores/feature/workEngineStore";
import { EdgeLabelRenderer, type EdgeProps, getSmoothStepPath } from "@xyflow/react";
import { theme } from "antd";
import React from "react";

const ORANGE_BASE = "#fa8c16";
const PURPLE_BASE = "#722ed1";

/**
 * 解析 sourceHandle 中的 port 信息，返回水平偏移量。
 * 格式：`"port-0"`（左）、`"port-1"`（中）、`"port-2"`（右）。
 * 用于 parallel 子节点出口区分，减少边交叉。
 */
function sourceOffsetFromHandle(sourceHandle?: string | null, sourceNodeW?: number): number {
  if (!sourceHandle || !sourceHandle.startsWith("port-")) { return 0; }
  const idx = parseInt(sourceHandle.replace("port-", ""), 10);
  if (isNaN(idx)) { return 0; }
  const w = sourceNodeW || 200;
  // -1/3w, 0, +1/3w
  if (idx === 0) { return -w / 3; }
  if (idx === 2) { return w / 3; }
  return 0;
}

const BaseEdgeComponent: React.FC<EdgeProps> = ({
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
  sourceHandleId,
}) => {
  const { token } = theme.useToken();

  const nodeStatuses = useWorkEngineStore((s) => s.nodeStatuses);
  const isDebugRunning = useWorkEngineStore((s) => s.isDebugRunning);

  const sourceRunning = nodeStatuses[source] === "running" || nodeStatuses[source] === "completed";
  const targetActive = nodeStatuses[target!] === "running" || nodeStatuses[target!] === "completed";
  const showFlowAnimation = isDebugRunning && (sourceRunning || targetActive);

  // 正交路由：使用 SmoothStep 替代 Bezier
  // 对 parallel 子节点做 port 偏移，使边出口分散
  const offsetX = sourceOffsetFromHandle(sourceHandleId);
  const [edgePath, labelX, labelY] = getSmoothStepPath({
    sourceX: sourceX + offsetX,
    sourceY,
    sourcePosition,
    targetX,
    targetY,
    targetPosition,
    borderRadius: 8,
  });

  const edgeColor = selected ? token.colorPrimary : token.colorBorderSecondary;
  const isAnimated = data?.edgeType === "loopBack";
  const isGrouping = data?.edgeType === "grouping";

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
      case "grouping":
        return "none";
      default:
        return edgeColor;
    }
  };

  const getEdgeStroke = () => {
    if (isGrouping) { return token.colorTextQuaternary; }
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
        strokeWidth={selected ? 2 : (isGrouping ? 1 : 1.5)}
        fill="none"
        strokeDasharray={isGrouping ? "4,4" : data?.edgeType === "error" ? "5,5" : undefined}
        markerEnd={isGrouping ? undefined : `url(#arrow-${data?.edgeType || "default"})`}
      />
      {!isGrouping && isAnimated && (
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
      {!isGrouping && showFlowAnimation && !isAnimated && (
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

// SPDX-License-Identifier: AGPL-3.0-only

import { useWorkEngineStore } from "@/stores/feature/workEngineStore";

import { EdgeLabelRenderer, type EdgeProps } from "@xyflow/react";
import { theme } from "antd";
import React from "react";

/**
 * 解析 sourceHandle 中的 port 信息，返回水平偏移量。
 * 格式：`"port-N"`（N=0~5，对应 1/7 ~ 6/7 宽度位置）。
 * 用于 parallel 子节点出口区分，减少边交叉。
 * 偏移量从节点中心计算：centerFrac × w − w/2
 */
function sourceOffsetFromHandle(sourceHandle?: string | null, sourceNodeW?: number): number {
  if (!sourceHandle || !sourceHandle.startsWith("port-")) { return 0; }
  const idx = parseInt(sourceHandle.replace("port-", ""), 10);
  if (isNaN(idx)) { return 0; }
  const w = sourceNodeW || 200;
  const N = 6;
  const clampedIdx = Math.min(Math.max(idx, 0), N - 1);
  const centerFrac = (clampedIdx + 1) / (N + 1);
  return centerFrac * w - w / 2;
}

/**
 * 解析 targetHandle 中的 port 信息，返回水平偏移量。
 * 与 sourceOffsetFromHandle 对称，用于 parallel 子节点入口分散边交叉。
 */
function targetOffsetFromHandle(targetHandle?: string | null, targetNodeW?: number): number {
  if (!targetHandle || !targetHandle.startsWith("port-")) { return 0; }
  const idx = parseInt(targetHandle.replace("port-", ""), 10);
  if (isNaN(idx)) { return 0; }
  const w = targetNodeW || 200;
  const N = 6;
  const clampedIdx = Math.min(Math.max(idx, 0), N - 1);
  const centerFrac = (clampedIdx + 1) / (N + 1);
  return centerFrac * w - w / 2;
}

function getSmoothStepPath(
  params: {
    sourceX: number;
    sourceY: number;
    sourcePosition: "top" | "bottom" | "left" | "right";
    targetX: number;
    targetY: number;
    targetPosition: "top" | "bottom" | "left" | "right";
    borderRadius?: number;
  },
): [string, number, number] {
  const { sourceX, sourceY, sourcePosition, targetX, targetY, targetPosition } = params;

  const dx = targetX - sourceX;
  const dy = targetY - sourceY;
  const absDx = Math.abs(dx);
  const absDy = Math.abs(dy);

  // Calculate control points for cubic bezier curve
  // Default offset for control points
  const offset = Math.max(50, Math.min(absDx, absDy) * 0.5);

  let cp1x: number, cp1y: number;
  let cp2x: number, cp2y: number;

  // Determine control points based on source and target positions
  if (sourcePosition === "bottom" && targetPosition === "top") {
    // Forward vertical connection
    cp1x = sourceX;
    cp1y = sourceY + offset;
    cp2x = targetX;
    cp2y = targetY - offset;
  } else if (sourcePosition === "top" && targetPosition === "bottom") {
    // Backward vertical connection
    cp1x = sourceX;
    cp1y = sourceY - offset;
    cp2x = targetX;
    cp2y = targetY + offset;
  } else if (sourcePosition === "right" && targetPosition === "left") {
    // Forward horizontal connection
    cp1x = sourceX + offset;
    cp1y = sourceY;
    cp2x = targetX - offset;
    cp2y = targetY;
  } else if (sourcePosition === "left" && targetPosition === "right") {
    // Backward horizontal connection
    cp1x = sourceX - offset;
    cp1y = sourceY;
    cp2x = targetX + offset;
    cp2y = targetY;
  } else if (sourcePosition === "bottom" && targetPosition === "right") {
    // Top to right
    cp1x = sourceX;
    cp1y = sourceY + offset;
    cp2x = targetX - offset;
    cp2y = targetY;
  } else if (sourcePosition === "bottom" && targetPosition === "left") {
    // Top to left
    cp1x = sourceX;
    cp1y = sourceY + offset;
    cp2x = targetX + offset;
    cp2y = targetY;
  } else if (sourcePosition === "top" && targetPosition === "right") {
    // Bottom to right
    cp1x = sourceX;
    cp1y = sourceY - offset;
    cp2x = targetX - offset;
    cp2y = targetY;
  } else if (sourcePosition === "top" && targetPosition === "left") {
    // Bottom to left
    cp1x = sourceX;
    cp1y = sourceY - offset;
    cp2x = targetX + offset;
    cp2y = targetY;
  } else if (sourcePosition === "right" && targetPosition === "top") {
    // Left to top
    cp1x = sourceX + offset;
    cp1y = sourceY;
    cp2x = targetX;
    cp2y = targetY + offset;
  } else if (sourcePosition === "right" && targetPosition === "bottom") {
    // Left to bottom
    cp1x = sourceX + offset;
    cp1y = sourceY;
    cp2x = targetX;
    cp2y = targetY - offset;
  } else if (sourcePosition === "left" && targetPosition === "top") {
    // Right to top
    cp1x = sourceX - offset;
    cp1y = sourceY;
    cp2x = targetX;
    cp2y = targetY + offset;
  } else if (sourcePosition === "left" && targetPosition === "bottom") {
    // Right to bottom
    cp1x = sourceX - offset;
    cp1y = sourceY;
    cp2x = targetX;
    cp2y = targetY - offset;
  } else {
    // Fallback: simple curve
    cp1x = sourceX + (targetX - sourceX) * 0.3;
    cp1y = sourceY;
    cp2x = targetX - (targetX - sourceX) * 0.3;
    cp2y = targetY;
  }

  // Calculate midpoint for label positioning
  const midX = (sourceX + 3 * cp1x + 3 * cp2x + targetX) / 8;
  const midY = (sourceY + 3 * cp1y + 3 * cp2y + targetY) / 8;

  return [`M ${sourceX} ${sourceY} C ${cp1x} ${cp1y}, ${cp2x} ${cp2y}, ${targetX} ${targetY}`, midX, midY];
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
  targetHandleId,
}) => {
  const { token } = theme.useToken();

  const isDebugRunning = useWorkEngineStore((s) => s.isDebugRunning);
  const sourceStatus = useWorkEngineStore((s) => s.nodeStatuses[source]);
  const targetStatus = useWorkEngineStore((s) => s.nodeStatuses[target!]);
  const showFlowAnimation = isDebugRunning
    && (sourceStatus === "running" || sourceStatus === "completed"
      || targetStatus === "running" || targetStatus === "completed");

  // 正交路由：使用 SmoothStep 替代 Bezier
  // 对 parallel 子节点做 port 偏移，使边出口/入口分散
  const offsetX = sourceOffsetFromHandle(sourceHandleId);
  const offsetTargetX = targetOffsetFromHandle(targetHandleId);
  const [edgePath, labelX, labelY] = getSmoothStepPath({
    sourceX: sourceX + offsetX,
    sourceY,
    sourcePosition,
    targetX: targetX + offsetTargetX,
    targetY,
    targetPosition,
    borderRadius: 8,
  });

  const edgeColor = selected ? token.colorPrimary : token.colorBorderSecondary;
  const isAnimated = data?.edgeType === "loopBack";
  const isGrouping = data?.edgeType === "grouping";

  const getEdgeStroke = () => {
    if (isGrouping) { return token.colorTextQuaternary; }
    if (showFlowAnimation) {
      if (data?.edgeType === "conditionTrue") { return token.colorSuccess; }
      if (data?.edgeType === "conditionFalse") { return token.colorError; }
      return token.colorPrimary;
    }
    return edgeColor;
  };

  const stroke = getEdgeStroke();
  const selectedGlow = selected
    ? `drop-shadow(0 0 4px ${stroke})`
    : undefined;

  return (
    <>
      <path
        id={id}
        className="react-flow__edge-path"
        d={edgePath}
        stroke={stroke}
        strokeWidth={selected ? 2.5 : (isGrouping ? 1 : 1.5)}
        fill="none"
        strokeDasharray={isGrouping ? "4,4" : data?.edgeType === "error" ? "5,5" : undefined}
        markerEnd={isGrouping ? undefined : `url(#arrow-${data?.edgeType || "default"})`}
        style={{ filter: selectedGlow, transition: "stroke-width 0.15s, filter 0.15s" }}
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
    </>
  );
};

export { BaseEdgeComponent as BaseEdge };

// SPDX-License-Identifier: AGPL-3.0-only

import { useWorkEngineStore } from "@/stores/feature/workEngineStore";

import { EdgeLabelRenderer, type EdgeProps } from "@xyflow/react";
import { theme } from "antd";
import React from "react";

const MIN_CTRL = 40;

const MAX_CTRL = 120;
const BEND_AMOUNT = 60;

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

  const isForward = sourcePosition === "bottom" && targetPosition === "top";

  let cp1x: number;
  let cp1y = sourceY;
  let cp2x: number;
  let cp2y = targetY;

  if (isForward) {
    const baseCtrl = Math.max(absDx * 0.3, MIN_CTRL);
    const ctrl = Math.min(baseCtrl, MAX_CTRL);

    cp1x = sourceX + ctrl;
    cp2x = targetX - ctrl;

    if (absDy > 60) {
      const bendFactor = Math.min(absDy / 200, 1);
      const bend = BEND_AMOUNT * bendFactor;
      cp1y = sourceY + bend;
      cp2y = targetY - bend;
    } else if (dx < 0) {
      cp1y = sourceY + BEND_AMOUNT;
      cp2y = targetY - BEND_AMOUNT;
    }
  } else {
    const baseCtrl = Math.max(absDx * 0.3, MIN_CTRL);
    const ctrl = Math.min(baseCtrl, MAX_CTRL);

    cp1x = sourceX - ctrl;
    cp2x = targetX + ctrl;

    if (absDy > 60) {
      const bendFactor = Math.min(absDy / 200, 1);
      const bend = BEND_AMOUNT * bendFactor;
      cp1y = sourceY - bend;
      cp2y = targetY + bend;
    } else if (dx > 0) {
      cp1y = sourceY - BEND_AMOUNT;
      cp2y = targetY + BEND_AMOUNT;
    }
  }

  // 三次贝塞尔曲线 B(t) 在 t=0.5 的点：B(0.5) = (P0 + 3·P1 + 3·P2 + P3) / 8
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

  const nodeStatuses = useWorkEngineStore((s) => s.nodeStatuses);
  const isDebugRunning = useWorkEngineStore((s) => s.isDebugRunning);

  const sourceRunning = nodeStatuses[source] === "running" || nodeStatuses[source] === "completed";
  const targetActive = nodeStatuses[target!] === "running" || nodeStatuses[target!] === "completed";
  const showFlowAnimation = isDebugRunning && (sourceRunning || targetActive);

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

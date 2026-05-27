import { theme } from "antd";
import React, { useCallback, useEffect, useRef, useState } from "react";
import { type Node, useReactFlow } from "reactflow";

interface AlignmentLine {
  position: number;
  orientation: "horizontal" | "vertical";
  start: number;
  end: number;
}

interface AlignmentGuidesProps {
  nodes: Node[];
  children?: React.ReactNode;
}

const SNAP_THRESHOLD = 8;

export const AlignmentGuides: React.FC<AlignmentGuidesProps> = ({
  nodes,
  children,
}) => {
  const { screenToFlowPosition, flowToScreenPosition } = useReactFlow();
  const [lines, setLines] = useState<AlignmentLine[]>([]);
  const { token } = theme.useToken();
  const draggedNodeRef = useRef<string | null>(null);
  const containerRef = useRef<HTMLDivElement>(null);

  const calculateAlignmentLines = useCallback(
    (draggingNodeId: string, position: { x: number; y: number }) => {
      const draggingNode = nodes.find((n) => n.id === draggingNodeId);
      if (!draggingNode) {
        return;
      }

      const newLines: AlignmentLine[] = [];
      const draggingBounds = {
        left: position.x,
        right: position.x + (draggingNode.width || 160),
        top: position.y,
        bottom: position.y + (draggingNode.height || 60),
        centerX: position.x + (draggingNode.width || 160) / 2,
        centerY: position.y + (draggingNode.height || 60) / 2,
      };

      nodes.forEach((node) => {
        if (node.id === draggingNodeId) {
          return;
        }

        const nodeBounds = {
          left: node.position.x,
          right: node.position.x + (node.width || 160),
          top: node.position.y,
          bottom: node.position.y + (node.height || 60),
          centerX: node.position.x + (node.width || 160) / 2,
          centerY: node.position.y + (node.height || 60) / 2,
        };

        if (Math.abs(draggingBounds.left - nodeBounds.left) < SNAP_THRESHOLD) {
          const screenStart = flowToScreenPosition({ x: nodeBounds.top, y: 0 });
          const screenEnd = flowToScreenPosition({
            x: nodeBounds.bottom,
            y: 0,
          });
          newLines.push({
            position: screenStart.x,
            orientation: "vertical",
            start: screenStart.y,
            end: screenEnd.y,
          });
        }

        if (
          Math.abs(draggingBounds.right - nodeBounds.right) < SNAP_THRESHOLD
        ) {
          const screenStart = flowToScreenPosition({ x: nodeBounds.top, y: 0 });
          const screenEnd = flowToScreenPosition({
            x: nodeBounds.bottom,
            y: 0,
          });
          newLines.push({
            position: screenStart.x,
            orientation: "vertical",
            start: screenStart.y,
            end: screenEnd.y,
          });
        }

        if (
          Math.abs(draggingBounds.centerX - nodeBounds.centerX) < SNAP_THRESHOLD
        ) {
          const screenStart = flowToScreenPosition({ x: nodeBounds.top, y: 0 });
          const screenEnd = flowToScreenPosition({
            x: nodeBounds.bottom,
            y: 0,
          });
          newLines.push({
            position: screenStart.x,
            orientation: "vertical",
            start: screenStart.y,
            end: screenEnd.y,
          });
        }

        if (Math.abs(draggingBounds.top - nodeBounds.top) < SNAP_THRESHOLD) {
          const screenStart = flowToScreenPosition({ x: 0, y: nodeBounds.top });
          const screenEnd = flowToScreenPosition({ x: nodeBounds.right, y: 0 });
          newLines.push({
            position: screenStart.y,
            orientation: "horizontal",
            start: screenStart.x,
            end: screenEnd.x,
          });
        }

        if (
          Math.abs(draggingBounds.bottom - nodeBounds.bottom) < SNAP_THRESHOLD
        ) {
          const screenStart = flowToScreenPosition({
            x: 0,
            y: nodeBounds.bottom,
          });
          const screenEnd = flowToScreenPosition({ x: nodeBounds.right, y: 0 });
          newLines.push({
            position: screenStart.y,
            orientation: "horizontal",
            start: screenStart.x,
            end: screenEnd.x,
          });
        }

        if (
          Math.abs(draggingBounds.centerY - nodeBounds.centerY) < SNAP_THRESHOLD
        ) {
          const screenStart = flowToScreenPosition({
            x: 0,
            y: nodeBounds.centerY,
          });
          const screenEnd = flowToScreenPosition({ x: nodeBounds.right, y: 0 });
          newLines.push({
            position: screenStart.y,
            orientation: "horizontal",
            start: screenStart.x,
            end: screenEnd.x,
          });
        }
      });

      setLines(newLines);
    },
    [nodes, flowToScreenPosition],
  );

  useEffect(() => {
    const handleMouseMove = (e: MouseEvent) => {
      if (!draggedNodeRef.current) {
        return;
      }

      const bounds = containerRef.current?.getBoundingClientRect();
      if (!bounds) {
        return;
      }

      const position = screenToFlowPosition({
        x: e.clientX - bounds.left,
        y: e.clientY - bounds.top,
      });

      calculateAlignmentLines(draggedNodeRef.current, position);
    };

    const handleMouseUp = () => {
      draggedNodeRef.current = null;
      setLines([]);
    };

    if (draggedNodeRef.current) {
      document.addEventListener("mousemove", handleMouseMove);
      document.addEventListener("mouseup", handleMouseUp);
    }

    return () => {
      document.removeEventListener("mousemove", handleMouseMove);
      document.removeEventListener("mouseup", handleMouseUp);
    };
  }, [screenToFlowPosition, calculateAlignmentLines]);

  useEffect(() => {
    const handleNodeDragStart = (_: MouseEvent, node: Node) => {
      draggedNodeRef.current = node.id;
    };

    const handlePaneClick = () => {
      setLines([]);
      draggedNodeRef.current = null;
    };

    const container = containerRef.current;
    if (container) {
      container.addEventListener(
        "nodeDragStart",
        handleNodeDragStart as EventListener,
      );
      container.addEventListener("pane-click", handlePaneClick);
    }

    return () => {
      if (container) {
        container.removeEventListener(
          "nodeDragStart",
          handleNodeDragStart as EventListener,
        );
        container.removeEventListener("pane-click", handlePaneClick);
      }
    };
  }, [nodes]);

  return (
    <div
      ref={containerRef}
      style={{ position: "relative", width: "100%", height: "100%" }}
    >
      {children}

      <svg
        style={{
          position: "absolute",
          top: 0,
          left: 0,
          width: "100%",
          height: "100%",
          pointerEvents: "none",
          zIndex: 1000,
        }}
      >
        <defs>
          <pattern
            id="gridPattern"
            width="16"
            height="16"
            patternUnits="userSpaceOnUse"
          >
            <circle cx="1" cy="1" r="0.5" fill={token.colorBorderSecondary} />
          </pattern>
        </defs>

        {/* static SVG alignment guides computed on the fly, safe to use index as key */}
        {lines.map((line, index) =>
          line.orientation === "vertical"
            ? (
              <line
                key={`v-${index}`}
                x1={line.position}
                y1={line.start}
                x2={line.position}
                y2={line.end}
                stroke={token.colorPrimary}
                strokeWidth={1}
                strokeDasharray="4,4"
                opacity={0.8}
              />
            )
            : (
              <line
                key={`h-${index}`}
                x1={line.start}
                y1={line.position}
                x2={line.end}
                y2={line.position}
                stroke={token.colorPrimary}
                strokeWidth={1}
                strokeDasharray="4,4"
                opacity={0.8}
              />
            )
        )}
      </svg>
    </div>
  );
};

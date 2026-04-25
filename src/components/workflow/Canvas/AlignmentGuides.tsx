import React, { useState, useCallback, useEffect, useRef } from 'react';
import { type Node, useReactFlow } from 'reactflow';

interface AlignmentLine {
  position: number;
  orientation: 'horizontal' | 'vertical';
  start: number;
  end: number;
}

interface AlignmentGuidesProps {
  nodes: Node[];
  children?: React.ReactNode;
}

const SNAP_THRESHOLD = 8;

export const AlignmentGuides: React.FC<AlignmentGuidesProps> = ({ nodes, children }) => {
  const { screenToFlowPosition, flowToScreenPosition } = useReactFlow();
  const [lines, setLines] = useState<AlignmentLine[]>([]);
  const [draggedNode, setDraggedNode] = useState<string | null>(null);
  const containerRef = useRef<HTMLDivElement>(null);

  const calculateAlignmentLines = useCallback(
    (draggingNodeId: string, position: { x: number; y: number }) => {
      const draggingNode = nodes.find((n) => n.id === draggingNodeId);
      if (!draggingNode) return;

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
        if (node.id === draggingNodeId) return;

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
          const screenEnd = flowToScreenPosition({ x: nodeBounds.bottom, y: 0 });
          newLines.push({
            position: screenStart.x,
            orientation: 'vertical',
            start: screenStart.y,
            end: screenEnd.y,
          });
        }

        if (Math.abs(draggingBounds.right - nodeBounds.right) < SNAP_THRESHOLD) {
          const screenStart = flowToScreenPosition({ x: nodeBounds.top, y: 0 });
          const screenEnd = flowToScreenPosition({ x: nodeBounds.bottom, y: 0 });
          newLines.push({
            position: screenStart.x,
            orientation: 'vertical',
            start: screenStart.y,
            end: screenEnd.y,
          });
        }

        if (Math.abs(draggingBounds.centerX - nodeBounds.centerX) < SNAP_THRESHOLD) {
          const screenStart = flowToScreenPosition({ x: nodeBounds.top, y: 0 });
          const screenEnd = flowToScreenPosition({ x: nodeBounds.bottom, y: 0 });
          newLines.push({
            position: screenStart.x,
            orientation: 'vertical',
            start: screenStart.y,
            end: screenEnd.y,
          });
        }

        if (Math.abs(draggingBounds.top - nodeBounds.top) < SNAP_THRESHOLD) {
          const screenStart = flowToScreenPosition({ x: 0, y: nodeBounds.top });
          const screenEnd = flowToScreenPosition({ x: nodeBounds.right, y: 0 });
          newLines.push({
            position: screenStart.y,
            orientation: 'horizontal',
            start: screenStart.x,
            end: screenEnd.x,
          });
        }

        if (Math.abs(draggingBounds.bottom - nodeBounds.bottom) < SNAP_THRESHOLD) {
          const screenStart = flowToScreenPosition({ x: 0, y: nodeBounds.bottom });
          const screenEnd = flowToScreenPosition({ x: nodeBounds.right, y: 0 });
          newLines.push({
            position: screenStart.y,
            orientation: 'horizontal',
            start: screenStart.x,
            end: screenEnd.x,
          });
        }

        if (Math.abs(draggingBounds.centerY - nodeBounds.centerY) < SNAP_THRESHOLD) {
          const screenStart = flowToScreenPosition({ x: 0, y: nodeBounds.centerY });
          const screenEnd = flowToScreenPosition({ x: nodeBounds.right, y: 0 });
          newLines.push({
            position: screenStart.y,
            orientation: 'horizontal',
            start: screenStart.x,
            end: screenEnd.x,
          });
        }
      });

      setLines(newLines);
    },
    [nodes, flowToScreenPosition]
  );

  useEffect(() => {
    const handleMouseMove = (e: MouseEvent) => {
      if (!draggedNode) return;

      const bounds = containerRef.current?.getBoundingClientRect();
      if (!bounds) return;

      const position = screenToFlowPosition({
        x: e.clientX - bounds.left,
        y: e.clientY - bounds.top,
      });

      calculateAlignmentLines(draggedNode, position);
    };

    const handleMouseUp = () => {
      setDraggedNode(null);
      setLines([]);
    };

    if (draggedNode) {
      document.addEventListener('mousemove', handleMouseMove);
      document.addEventListener('mouseup', handleMouseUp);
    }

    return () => {
      document.removeEventListener('mousemove', handleMouseMove);
      document.removeEventListener('mouseup', handleMouseUp);
    };
  }, [draggedNode, screenToFlowPosition, calculateAlignmentLines]);

  useEffect(() => {
    const handleNodeDragStart = (_: MouseEvent, node: Node) => {
      setDraggedNode(node.id);
    };

    const handlePaneClick = () => {
      setLines([]);
      setDraggedNode(null);
    };

    const container = containerRef.current;
    if (container) {
      container.addEventListener('nodeDragStart', handleNodeDragStart as any);
      container.addEventListener('pane-click', handlePaneClick);
    }

    return () => {
      if (container) {
        container.removeEventListener('nodeDragStart', handleNodeDragStart as any);
        container.removeEventListener('pane-click', handlePaneClick);
      }
    };
  }, [nodes]);

  return (
    <div ref={containerRef} style={{ position: 'relative', width: '100%', height: '100%' }}>
      {children}

      <svg
        style={{
          position: 'absolute',
          top: 0,
          left: 0,
          width: '100%',
          height: '100%',
          pointerEvents: 'none',
          zIndex: 1000,
        }}
      >
        <defs>
          <pattern id="gridPattern" width="16" height="16" patternUnits="userSpaceOnUse">
            <circle cx="1" cy="1" r="0.5" fill="#333" />
          </pattern>
        </defs>

        {lines.map((line, index) =>
          line.orientation === 'vertical' ? (
            <line
              key={`v-${index}`}
              x1={line.position}
              y1={line.start}
              x2={line.position}
              y2={line.end}
              stroke="#1890ff"
              strokeWidth={1}
              strokeDasharray="4,4"
              opacity={0.8}
            />
          ) : (
            <line
              key={`h-${index}`}
              x1={line.start}
              y1={line.position}
              x2={line.end}
              y2={line.position}
              stroke="#1890ff"
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

export default AlignmentGuides;

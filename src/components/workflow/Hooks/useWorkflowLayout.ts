// SPDX-License-Identifier: AGPL-3.0-only

import { autoLayoutWorkflow, type NodePositionLike, toAbsolutePosition } from "@/lib/workflowLayout";
import { useCallback, useEffect, useRef } from "react";
import type { WorkflowEdge, WorkflowNode } from "../types";

/**
 * 封装 autoLayout 自动布局与 parentRefs 协调的调度逻辑。
 *
 * 并发安全：
 * 1. autoLayoutTimerRef 确保只有一个布局 timer 在飞，避免多次叠加
 * 2. hasAutoLaidOutRef 标记是否已做过布局，防止每次渲染都重新布局
 * 3. pendingParentSync 检查：当 parentRefs 还在同步时延后布局，
 *    因为 autoLayout 依赖正确的 parentId 来计算容器内子节点位置
 * 4. skipPositionWriteRef 防止 autoLayout 回调被 onNodesChange 再次写回
 *
 * 使用方：WorkflowEditor.tsx
 */
export interface LayoutController {
  /** 标记初始化布局是否已完成 */
  hasAutoLaidOutRef: React.MutableRefObject<boolean>;
  /** 程序布局后跳过位置写入的标志 */
  skipPositionWriteRef: React.MutableRefObject<boolean>;
  /** 执行 autoLayout（由 useEffect 调度） */
  runAutoLayout: (params: LayoutParams) => void;
}

export interface LayoutParams {
  nodes: WorkflowNode[];
  edges: WorkflowEdge[];
  parentRefs: Record<string, string>;
  computedFlowNodes: import("@xyflow/react").Node[];
  computedFlowEdges: import("@xyflow/react").Edge[];
  setRNodes: (nodes: import("@xyflow/react").Node[]) => void;
  setREdges: (edges: import("@xyflow/react").Edge[]) => void;
  updateNode: (nodeId: string, updates: Partial<WorkflowNode>) => void;
}

export function useWorkflowLayout(): LayoutController {
  const hasAutoLaidOutRef = useRef(false);
  const autoLayoutTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const skipPositionWriteRef = useRef(false);

  useEffect(() => {
    return () => {
      if (autoLayoutTimerRef.current) {
        clearTimeout(autoLayoutTimerRef.current);
        autoLayoutTimerRef.current = null;
      }
    };
  }, []);

  const runAutoLayout = useCallback((params: LayoutParams) => {
    const {
      parentRefs,
      computedFlowNodes,
      computedFlowEdges,
      setRNodes,
      setREdges,
      updateNode,
    } = params;

    if (autoLayoutTimerRef.current) {
      clearTimeout(autoLayoutTimerRef.current);
      autoLayoutTimerRef.current = null;
    }

    autoLayoutTimerRef.current = setTimeout(() => {
      const { nodes: layouted, edges: layoutedE } = autoLayoutWorkflow(
        computedFlowNodes,
        computedFlowEdges,
        parentRefs,
      );

      // 拓扑排序：父节点排在子节点前面（React Flow parent/child 模式要求）
      const sortedLayouted = [...layouted].sort((a, b) => {
        const aPid = parentRefs[a.id];
        const bPid = parentRefs[b.id];
        if (aPid === b.id) { return 1; }
        if (bPid === a.id) { return -1; }
        return 0;
      });

      skipPositionWriteRef.current = true;
      setRNodes(sortedLayouted);
      setREdges(layoutedE);
      requestAnimationFrame(() => {
        skipPositionWriteRef.current = false;
      });

      for (const ln of layouted) {
        const pid = parentRefs[ln.id];
        if (pid) {
          const parentLn = layouted.find((n) => n.id === pid);
          if (parentLn) {
            const absPos = toAbsolutePosition(
              ln.id,
              ln.position,
              parentRefs,
              layouted.map((n) => ({ id: n.id, position: n.position })) as NodePositionLike[],
            );
            updateNode(ln.id, { position: absPos } as Partial<WorkflowNode>);
            continue;
          }
        }
        updateNode(ln.id, { position: ln.position } as Partial<WorkflowNode>);
      }
    }, 100);
  }, []);

  return {
    hasAutoLaidOutRef,
    skipPositionWriteRef,
    runAutoLayout,
  };
}

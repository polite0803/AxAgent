// SPDX-License-Identifier: AGPL-3.0-only

import { type NodePositionLike, toAbsolutePosition } from "@/lib/workflowLayout";
import { useWorkflowEditorStore } from "@/stores";
import { useCallback, useEffect, useRef } from "react";
import type { WorkflowNode } from "../types";

/**
 * 封装拖拽节点位置更新的 RAF 批处理与竞态保护。
 *
 * 解决的并发安全问题：
 * 1. 拖拽过程中 React Flow 内部高频 position 变更（每帧多次）与 Zustand
 *    store 写入的竞态 — 使用 RAF 合并一帧内所有位置更新
 * 2. 拖拽位置写入与 autoLayout 程序布局的冲突 — 通过 `skipPositionWriteRef`
 *    标志避免 autoLayout 回调被 onNodesChange 再次写回
 * 3. dragStop 后短暂抑制 useEffect 全量重建 — `suppressRebuildRef`
 *    防止 reactFlowInstance.setNodes 结果被覆盖
 *
 * 使用方：WorkflowEditor.tsx 的 handleNodesChange / handleNodeDragStop
 */
export interface DragPositionController {
  /** 拖拽中标志 — 在 onNodeDragStart/Stop 切换 */
  isDraggingRef: React.MutableRefObject<boolean>;
  /** 程序布局后的位置写入抑制标志 */
  skipPositionWriteRef: React.MutableRefObject<boolean>;
  /** dragStop 后短暂全量重建抑制 */
  suppressRebuildRef: React.MutableRefObject<boolean>;
  /** 队列化位置更新 — 供 onNodesChange(position) 调用 */
  queuePositionUpdate: (nodeId: string, relativePos: { x: number; y: number }) => void;
  /** flush 所有待写位置更新 — dragStop 时调用 */
  flushPendingPositions: () => void;
  /** 清空待写队列（cancel 场景） */
  clearPending: () => void;
}

export function useWorkflowDragPosition(): DragPositionController {
  const isDraggingRef = useRef(false);
  const skipPositionWriteRef = useRef(false);
  const suppressRebuildRef = useRef(false);

  // 本帧内待写的绝对位置 { nodeId → {x, y} }
  const pendingPositionsRef = useRef<Map<string, { x: number; y: number }>>(new Map());
  // RAF 句柄
  const rafIdRef = useRef<number | null>(null);

  // FE-I1 修复：hook 自身卸载时清理待写 RAF 与位置队列，
  // 防止卸载后仍有回调写入已卸载 store。
  useEffect(() => {
    return () => {
      if (rafIdRef.current !== null) {
        cancelAnimationFrame(rafIdRef.current);
        rafIdRef.current = null;
      }
      pendingPositionsRef.current.clear();
    };
  }, []);

  const flush = useCallback(() => {
    rafIdRef.current = null;
    if (isDraggingRef.current) {
      // 仍在拖拽则延后
      return;
    }
    const pending = pendingPositionsRef.current;
    pendingPositionsRef.current = new Map();
    pending.forEach((pos, nodeId) => {
      useWorkflowEditorStore.getState().updateNode(nodeId, { position: pos } as Partial<WorkflowNode>);
    });
  }, []);

  const queuePositionUpdate = useCallback(
    (nodeId: string, relativePos: { x: number; y: number }) => {
      if (isDraggingRef.current || skipPositionWriteRef.current) {
        // 拖拽中或正在程序化布局 — 不入队
        return;
      }
      const state = useWorkflowEditorStore.getState();
      const storePos = toAbsolutePosition(
        nodeId,
        relativePos,
        state.parentRefs,
        state.nodes as NodePositionLike[],
      );
      pendingPositionsRef.current.set(nodeId, storePos);
      if (rafIdRef.current === null) {
        rafIdRef.current = requestAnimationFrame(() => flush());
      }
    },
    [flush],
  );

  const flushPendingPositions = useCallback(() => {
    // 同步 flush — dragStop 时确保最后一帧的位置也写入
    if (rafIdRef.current !== null) {
      cancelAnimationFrame(rafIdRef.current);
      rafIdRef.current = null;
    }
    const pending = pendingPositionsRef.current;
    pendingPositionsRef.current = new Map();
    pending.forEach((pos, nodeId) => {
      useWorkflowEditorStore.getState().updateNode(nodeId, { position: pos } as Partial<WorkflowNode>);
    });
  }, []);

  const clearPending = useCallback(() => {
    if (rafIdRef.current !== null) {
      cancelAnimationFrame(rafIdRef.current);
      rafIdRef.current = null;
    }
    pendingPositionsRef.current.clear();
  }, []);

  return {
    isDraggingRef,
    skipPositionWriteRef,
    suppressRebuildRef,
    queuePositionUpdate,
    flushPendingPositions,
    clearPending,
  };
}

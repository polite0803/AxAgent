// SPDX-License-Identifier: AGPL-3.0-only

import { useWorkflowEditorStore } from "@/stores";
import { message } from "antd";
import { useEffect, useRef } from "react";
import { useTranslation } from "react-i18next";
import { NODE_TYPE_MAP, type WorkflowNode } from "../types";

interface UseKeyboardShortcutsParams {
  undo: () => void;
  redo: () => void;
  canUndo: () => boolean;
  canRedo: () => boolean;
  selectedNodeId: string | null;
  deleteNode: (id: string) => void;
  nodes: WorkflowNode[];
  addNode: (node: WorkflowNode) => void;
  setSelectedNode: (id: string | null) => void;
  setParentRef: (childId: string, parentId: string | null) => void;
  updateNode: (nodeId: string, updates: Partial<WorkflowNode>) => void;
  clipboardRef: React.MutableRefObject<WorkflowNode[]>;
  handleSaveRef: React.MutableRefObject<() => void>;
  setSearchVisible: (visible: boolean) => void;
}

export function useKeyboardShortcuts(params: UseKeyboardShortcutsParams) {
  const {
    undo,
    redo,
    canUndo,
    canRedo,
    selectedNodeId,
    deleteNode,
    nodes,
    addNode,
    setSelectedNode,
    setParentRef,
    updateNode,
    clipboardRef,
    handleSaveRef,
    setSearchVisible,
  } = params;

  const { t } = useTranslation();

  const keyRef = useRef({
    undo,
    redo,
    canUndo,
    canRedo,
    selectedNodeId,
    deleteNode,
    nodes,
    addNode,
    setSelectedNode,
    setParentRef,
    updateNode,
    clipboardRef,
  });
  // eslint-disable-next-line react-hooks/refs
  keyRef.current = {
    undo,
    redo,
    canUndo,
    canRedo,
    selectedNodeId,
    deleteNode,
    nodes,
    addNode,
    setSelectedNode,
    setParentRef,
    updateNode,
    clipboardRef,
  };

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      const r = keyRef.current;
      const isCtrlOrCmd = e.ctrlKey || e.metaKey;
      const isEditing = (e.target as HTMLElement).tagName === "INPUT"
        || (e.target as HTMLElement).tagName === "TEXTAREA"
        || (e.target as HTMLElement).isContentEditable;

      if (isCtrlOrCmd && e.key === "s") {
        e.preventDefault();
        handleSaveRef.current();
        return;
      }
      if (isCtrlOrCmd && e.key === "z" && !e.shiftKey) {
        e.preventDefault();
        if (r.canUndo()) { r.undo(); }
        else { message.info(t("workflow.noUndoAvailable")); }
        return;
      }
      if ((isCtrlOrCmd && e.key === "z" && e.shiftKey) || (isCtrlOrCmd && e.key === "y")) {
        e.preventDefault();
        if (r.canRedo()) { r.redo(); }
        else { message.info(t("workflow.noRedoAvailable")); }
        return;
      }
      if ((e.key === "Delete" || e.key === "Backspace") && r.selectedNodeId) {
        if (isEditing) { return; }
        e.preventDefault();
        r.deleteNode(r.selectedNodeId);
        r.setSelectedNode(null);
        return;
      }
      if (isCtrlOrCmd && e.key === "c" && r.selectedNodeId) {
        const store = useWorkflowEditorStore.getState();
        const nodeToCopy = r.nodes.find((n) => n.id === r.selectedNodeId);
        if (nodeToCopy) {
          const allNodes: WorkflowNode[] = [nodeToCopy];
          const allEdges: WorkflowNode[] = [];
          if (NODE_TYPE_MAP[nodeToCopy.type]?.isContainer) {
            for (const [cid, pid] of Object.entries(store.parentRefs)) {
              if (pid === nodeToCopy.id) {
                const child = store.nodes.find((n) => n.id === cid);
                if (child) { allNodes.push(child); }
              }
            }
            const childIds = new Set(allNodes.map((n) => n.id));
            for (const e of store.edges) {
              if (childIds.has(e.source) && childIds.has(e.target)) {
                allEdges.push(e as unknown as WorkflowNode);
              }
            }
          }
          r.clipboardRef.current = allNodes;
          (r.clipboardRef as unknown as React.MutableRefObject<WorkflowNode[] & { _edges?: WorkflowNode[] }>).current
            ._edges = allEdges as unknown as WorkflowNode[];
          message.success(t("workflow.nodeCopied"));
        }
        return;
      }
      if (isCtrlOrCmd && e.key === "v" && !isEditing) {
        if (r.clipboardRef.current.length === 0) { return; }
        const offset = { x: 50, y: 50 };
        const idMap = new Map<string, string>();
        r.clipboardRef.current.forEach((node) => {
          const newId = `node-${crypto.randomUUID()}`;
          idMap.set(node.id, newId);
          r.addNode({
            ...node,
            id: newId,
            position: { x: node.position.x + offset.x, y: node.position.y + offset.y },
          });
          const originalParentId = useWorkflowEditorStore.getState().parentRefs[node.id];
          if (originalParentId) {
            const newParentId = idMap.get(originalParentId);
            if (newParentId) {
              r.setParentRef(newId, newParentId);
            }
          }
        });
        const clipboardEdges =
          (r.clipboardRef as unknown as React.MutableRefObject<WorkflowNode[] & { _edges?: WorkflowNode[] }>).current
            ._edges;
        if (clipboardEdges && clipboardEdges.length > 0) {
          const store = useWorkflowEditorStore.getState();
          for (const edge of clipboardEdges as unknown as import("../types").WorkflowEdge[]) {
            const newSource = idMap.get(edge.source) ?? edge.source;
            const newTarget = idMap.get(edge.target) ?? edge.target;
            store.addEdge({
              ...edge,
              id: `edge-${crypto.randomUUID()}`,
              source: newSource,
              target: newTarget,
            });
          }
        }
        message.success(t("workflow.nodesPasted", { count: r.clipboardRef.current.length }));
        return;
      }
      if (isCtrlOrCmd && e.key === "f" && !isEditing) {
        e.preventDefault();
        setSearchVisible(true);
        return;
      }
      if (isCtrlOrCmd && e.shiftKey && e.key === "C" && !isEditing) {
        e.preventDefault();
        const containers = r.nodes.filter((n) => NODE_TYPE_MAP[n.type]?.isContainer);
        if (containers.length > 0) {
          useWorkflowEditorStore.getState().collapseAllContainers();
          message.success(t("workflow.containersCollapsed", { defaultValue: "All containers collapsed" }));
        }
        return;
      }
      if (isCtrlOrCmd && e.shiftKey && e.key === "E" && !isEditing) {
        e.preventDefault();
        useWorkflowEditorStore.getState().expandAllContainers();
        message.success(t("workflow.containersExpanded", { defaultValue: "All containers expanded" }));
        return;
      }
      if (isCtrlOrCmd && e.key === "a" && !isEditing) {
        e.preventDefault();
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [t, handleSaveRef, setSearchVisible]);
}

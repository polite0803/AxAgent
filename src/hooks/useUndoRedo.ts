import { useCallback, useState } from "react";
import type { WorkflowEdge, WorkflowNode } from "@/components/workflow/types";

interface HistoryState {
  nodes: WorkflowNode[];
  edges: WorkflowEdge[];
}

interface UseUndoRedoProps {
  initialNodes: WorkflowNode[];
  initialEdges: WorkflowEdge[];
  onRestore?: (nodes: WorkflowNode[], edges: WorkflowEdge[]) => void;
}

export const useUndoRedo = ({
  initialNodes,
  initialEdges,
  onRestore,
}: UseUndoRedoProps) => {
  const [history, setHistory] = useState<HistoryState[]>([{ nodes: initialNodes, edges: initialEdges }]);
  const [historyIndex, setHistoryIndex] = useState(0);

  const pushState = useCallback((nodes: WorkflowNode[], edges: WorkflowEdge[]) => {
    setHistory((prev) => {
      const newHistory = prev.slice(0, historyIndex + 1);
      newHistory.push({ nodes: [...nodes], edges: [...edges] });
      if (newHistory.length > 50) {
        newHistory.shift();
        return newHistory;
      }
      return newHistory;
    });
    setHistoryIndex((prev) => Math.min(prev + 1, 49));
  }, [historyIndex]);

  const undo = useCallback(() => {
    if (historyIndex > 0) {
      const newIndex = historyIndex - 1;
      const state = history[newIndex];
      setHistoryIndex(newIndex);
      onRestore?.(state.nodes, state.edges);
      return true;
    }
    return false;
  }, [historyIndex, history, onRestore]);

  const redo = useCallback(() => {
    if (historyIndex < history.length - 1) {
      const newIndex = historyIndex + 1;
      const state = history[newIndex];
      setHistoryIndex(newIndex);
      onRestore?.(state.nodes, state.edges);
      return true;
    }
    return false;
  }, [historyIndex, history, onRestore]);

  const canUndo = historyIndex > 0;
  const canRedo = historyIndex < history.length - 1;

  return {
    pushState,
    undo,
    redo,
    canUndo,
    canRedo,
  };
};

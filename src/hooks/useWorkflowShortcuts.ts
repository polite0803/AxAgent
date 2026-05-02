import { useCallback, useEffect } from "react";
import { useReactFlow } from "reactflow";

interface UseWorkflowShortcutsProps {
  onDelete?: () => void;
  onUndo?: () => void;
  onRedo?: () => void;
  onSearch?: () => void;
  onCopy?: () => void;
  onPaste?: () => void;
  onSelectAll?: () => void;
}

export const useWorkflowShortcuts = ({
  onDelete,
  onUndo,
  onRedo,
  onSearch,
  onCopy,
  onPaste,
  onSelectAll,
}: UseWorkflowShortcutsProps) => {
  const { deleteElements } = useReactFlow();

  const handleKeyDown = useCallback(
    (event: KeyboardEvent) => {
      const isInputField = event.target instanceof HTMLInputElement
        || event.target instanceof HTMLTextAreaElement
        || event.target instanceof HTMLSelectElement;

      if (isInputField) {
        return;
      }

      const isMac = navigator.platform.toUpperCase().indexOf("MAC") >= 0;
      const ctrlKey = isMac ? event.metaKey : event.ctrlKey;

      if (ctrlKey && event.key === "z" && !event.shiftKey) {
        event.preventDefault();
        onUndo?.();
        return;
      }

      if (ctrlKey && (event.key === "y" || (event.key === "z" && event.shiftKey))) {
        event.preventDefault();
        onRedo?.();
        return;
      }

      if (ctrlKey && event.key === "f") {
        event.preventDefault();
        onSearch?.();
        return;
      }

      if (ctrlKey && event.key === "c") {
        event.preventDefault();
        onCopy?.();
        return;
      }

      if (ctrlKey && event.key === "v") {
        event.preventDefault();
        onPaste?.();
        return;
      }

      if (ctrlKey && event.key === "a") {
        event.preventDefault();
        onSelectAll?.();
        return;
      }

      if (event.key === "Delete" || event.key === "Backspace") {
        event.preventDefault();
        onDelete?.();
        return;
      }

      if (event.key === "Escape") {
        event.preventDefault();
        return;
      }
    },
    [onDelete, onUndo, onRedo, onSearch, onCopy, onPaste, onSelectAll, deleteElements],
  );

  useEffect(() => {
    window.addEventListener("keydown", handleKeyDown);
    return () => {
      window.removeEventListener("keydown", handleKeyDown);
    };
  }, [handleKeyDown]);
};

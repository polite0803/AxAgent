/**
 * Custom drag-and-drop state for the workflow editor.
 *
 * HTML5 native drag-and-drop (dataTransfer) does not work reliably in
 * Tauri's WebView2 — the webview intercepts drag events for file handling,
 * which causes dataTransfer.getData() to return empty strings.
 *
 * Instead, we store the dragged node info in a simple module-level variable
 * and rely on mousedown/mousemove/mouseup events for the full DnD cycle.
 */

export interface DragPayload {
  type: string;
  label: string;
}

/** Module-level drag state — no React re-renders needed for the drag source. */
let currentDrag: DragPayload | null = null;

export function setDragPayload(payload: DragPayload | null) {
  currentDrag = payload;
}

export function getDragPayload(): DragPayload | null {
  return currentDrag;
}

export function clearDragPayload() {
  currentDrag = null;
}

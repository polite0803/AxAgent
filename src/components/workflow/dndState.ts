/**
 * Custom drag-and-drop state for the workflow editor.
 *
 * HTML5 native drag-and-drop (dataTransfer) does not work reliably in
 * Tauri's WebView2 — the webview intercepts drag events for file handling,
 * which causes dataTransfer.getData() to return empty strings.
 *
 * Instead, we store the dragged node info in a keyed map and rely on
 * mousedown/mousemove/mouseup events for the full DnD cycle.
 *
 * ## 多窗口 / iframe 隔离
 *
 * 原实现使用模块级 `let currentDrag: DragPayload | null = null`，
 * 在多 webview / iframe 共享同一 JS bundle 时会跨窗口串味。
 * 改用 windowId-keyed Map，每个 windowId 持有自己的 state。
 * 旧 API（`setDragPayload` / `getDragPayload` / `clearDragPayload`）
 * 仍以单例形式提供，向后兼容既有调用方（LeftPanel、WorkflowEditor）。
 *
 * ## 测试
 *
 * `__resetDragStateForTest` 仅供单元测试调用，**严禁**在产品代码中
 * 调用，否则会污染正在进行的拖拽会话。
 */

export interface DragPayload {
  type: string;
  label: string;
}

/** 默认 windowId：旧 API 内部使用的固定 key */
const DEFAULT_WINDOW_ID = "__default__";

/** WindowId → 当前拖拽 payload；Map 保证每个窗口独立 */
const dragStates = new Map<string, DragPayload>();

export function setDragPayload(payload: DragPayload | null): void {
  setDragPayloadForWindow(DEFAULT_WINDOW_ID, payload);
}

export function getDragPayload(): DragPayload | null {
  return getDragPayloadForWindow(DEFAULT_WINDOW_ID);
}

export function clearDragPayload(): void {
  clearDragPayloadForWindow(DEFAULT_WINDOW_ID);
}

/** 显式指定 windowId 的版本：用于多窗口/iframe 隔离。 */
export function setDragPayloadForWindow(
  windowId: string,
  payload: DragPayload | null,
): void {
  if (payload === null) {
    dragStates.delete(windowId);
  } else {
    dragStates.set(windowId, payload);
  }
}

export function getDragPayloadForWindow(windowId: string): DragPayload | null {
  return dragStates.get(windowId) ?? null;
}

export function clearDragPayloadForWindow(windowId: string): void {
  dragStates.delete(windowId);
}

/** 仅供测试：清空所有 windowId 的 drag state */
export function __resetDragStateForTest(): void {
  dragStates.clear();
}

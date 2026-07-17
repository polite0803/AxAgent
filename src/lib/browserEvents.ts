// SPDX-License-Identifier: AGPL-3.0-only

/**
 * 浏览器（非 Tauri）模式下的内存事件总线。
 *
 * 仅在本地 `npm run dev` 与 Playwright e2e 中使用：让 browserMock 能像真实后端那样
 * 经由 `listen()` 向前端派发事件（如 `agent-plan-ready-for-approval`），从而可以对
 * 事件驱动的 UI 流程（计划确认闸门等）做端到端测试。Tauri 模式走原生事件通道，
 * 不经过此总线。
 */

let _browserEventTarget: EventTarget | null = null;

function getBrowserEventTarget(): EventTarget {
  if (_browserEventTarget === null) {
    _browserEventTarget = new EventTarget();
  }
  return _browserEventTarget;
}

/** 派发一个浏览器模式事件（payload 透传到监听器）。 */
export function emitBrowserEvent(event: string, payload: unknown): void {
  getBrowserEventTarget().dispatchEvent(new CustomEvent(event, { detail: payload }));
}

/** 订阅浏览器模式事件，返回取消订阅函数。 */
export function onBrowserEvent(
  event: string,
  handler: (payload: unknown) => void,
): () => void {
  const target = getBrowserEventTarget();
  const listener = (e: Event) => handler((e as CustomEvent).detail);
  target.addEventListener(event, listener);
  return () => target.removeEventListener(event, listener);
}

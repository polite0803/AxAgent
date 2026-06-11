// SPDX-License-Identifier: AGPL-3.0-only

import "@testing-library/jest-dom";

// jsdom 销毁后 react-dom scheduler 异步回调仍可能访问 window/document，
// 用 getter 防御 ReferenceError（不可删除的兜底值）
{
  const g = globalThis as Record<string, unknown>;
  const fallbackDoc = {
    createElement: () => ({}),
    documentElement: {},
    body: {},
    head: {},
  };
  let _win: unknown = g.window;
  let _doc: unknown = g.document;
  Object.defineProperty(g, "window", {
    get() {
      return _win ?? g;
    },
    set(v) {
      _win = v;
    },
    configurable: true,
    enumerable: true,
  });
  Object.defineProperty(g, "document", {
    get() {
      return _doc ?? fallbackDoc;
    },
    set(v) {
      _doc = v;
    },
    configurable: true,
    enumerable: true,
  });
}

if (typeof window !== "undefined" && !window.matchMedia) {
  Object.defineProperty(window, "matchMedia", {
    writable: true,
    value: (query: string) => ({
      matches: false,
      media: query,
      onchange: null,
      addListener: () => {},
      removeListener: () => {},
      addEventListener: () => {},
      removeEventListener: () => {},
      dispatchEvent: () => false,
    }),
  });
}

if (typeof globalThis.ResizeObserver === "undefined") {
  class ResizeObserver {
    observe() {}
    unobserve() {}
    disconnect() {}
  }

  globalThis.ResizeObserver = ResizeObserver;
}

// 用 setTimeout 替代 setImmediate — React scheduler 在 jsdom 销毁后
// 仍可能通过 setImmediate 触发回调，导致 "window is not defined"
// setTimeout 确保回调在 jsdom 生命周期内执行完毕
if (typeof globalThis.setImmediate !== "undefined") {
  const pending = new Map<number, ReturnType<typeof setTimeout>>();
  let nextId = 1;
  (globalThis as any).setImmediate = (fn: (...args: any[]) => void, ...args: any[]) => {
    const id = nextId++;
    pending.set(
      id,
      setTimeout(() => {
        pending.delete(id);
        // jsdom 销毁后 React scheduler 回调仍可能执行，window 已变为 undefined
        try {
          fn(...args);
        } catch {
          // React performWorkOnRootViaSchedulerTask — silently discard post-teardown errors
        }
      }, 0),
    );
    return id;
  };
  (globalThis as any).clearImmediate = (id: number) => {
    const timer = pending.get(id);
    if (timer) {
      clearTimeout(timer);
      pending.delete(id);
    }
  };
}

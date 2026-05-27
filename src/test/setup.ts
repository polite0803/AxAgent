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

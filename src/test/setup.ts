import "@testing-library/jest-dom";

// jsdom 环境初始化前的 window 兜底（防止 react-dom 清理阶段报 ReferenceError）
if (typeof window === "undefined") {
  // @ts-expect-error - vitest jsdom 环境兜底，提供最小 window stub
  globalThis.window = globalThis;
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

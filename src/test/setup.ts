import "@testing-library/jest-dom";

// jsdom 环境初始化/清理阶段的 window 兜底（防止 react-dom 清理时报 ReferenceError）
if (typeof window === "undefined") {
  // @ts-expect-error - vitest jsdom 环境兜底，提供最小 window stub
  globalThis.window = globalThis;
}
// 防御 jsdom 环境被销毁后 scheduler 仍尝试访问 window 的情况
if (typeof document === "undefined") {
  // @ts-expect-error - jsdom 清理后兜底
  globalThis.document = { createElement: () => ({}), documentElement: {} };
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

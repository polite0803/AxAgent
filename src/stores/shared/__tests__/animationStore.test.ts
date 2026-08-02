// SPDX-License-Identifier: AGPL-3.0-only

import { beforeEach, describe, expect, it, vi } from "vitest";

describe("animationStore", () => {
  let useAnimationStore: typeof import("../animationStore").useAnimationStore;
  let isAnimationEnabled: typeof import("../animationStore").isAnimationEnabled;
  let initAnimationPreference: typeof import("../animationStore").initAnimationPreference;

  beforeEach(async () => {
    vi.resetModules();
    localStorage.clear();
    // 默认 mock：系统未开启「减少动态效果」
    Object.defineProperty(window, "matchMedia", {
      writable: true,
      value: vi.fn().mockImplementation((query: string) => ({
        matches: false,
        media: query,
        onchange: null,
        addEventListener: vi.fn(),
        removeEventListener: vi.fn(),
        dispatchEvent: vi.fn(),
      })),
    });
    const mod = await import("../animationStore");
    useAnimationStore = mod.useAnimationStore;
    isAnimationEnabled = mod.isAnimationEnabled;
    initAnimationPreference = mod.initAnimationPreference;
  });

  describe("默认状态", () => {
    it("默认 mode 为 system", () => {
      expect(useAnimationStore.getState().mode).toBe("system");
    });

    it("系统未减少动态效果时，system 模式默认启用动画", () => {
      expect(isAnimationEnabled()).toBe(true);
    });
  });

  describe("setMode", () => {
    it("mode=on 时始终启用动画", () => {
      useAnimationStore.getState().setMode("on");
      useAnimationStore.setState({ systemPrefersReducedMotion: true });
      expect(isAnimationEnabled()).toBe(true);
    });

    it("mode=off 时始终禁用动画", () => {
      useAnimationStore.getState().setMode("off");
      useAnimationStore.setState({ systemPrefersReducedMotion: false });
      expect(isAnimationEnabled()).toBe(false);
    });

    it("mode=system 时跟随系统状态", () => {
      useAnimationStore.getState().setMode("system");
      useAnimationStore.setState({ systemPrefersReducedMotion: true });
      expect(isAnimationEnabled()).toBe(false);

      useAnimationStore.setState({ systemPrefersReducedMotion: false });
      expect(isAnimationEnabled()).toBe(true);
    });
  });

  describe("持久化", () => {
    it("setMode 后持久化到 localStorage", () => {
      useAnimationStore.getState().setMode("off");
      const persisted = JSON.parse(
        localStorage.getItem("axagent-animation-preference") ?? "{}",
      );
      expect(persisted.state.mode).toBe("off");
    });

    it("重建 store 时恢复持久化 mode", async () => {
      useAnimationStore.getState().setMode("on");
      vi.resetModules();
      const mod = await import("../animationStore");
      expect(mod.useAnimationStore.getState().mode).toBe("on");
    });
  });

  describe("initAnimationPreference", () => {
    it("系统减少动态效果开启时，system 模式自动禁用动画", () => {
      // 模拟用户切换系统设置 → change 事件
      let changeHandler: ((e: { matches: boolean }) => void) | null = null;
      Object.defineProperty(window, "matchMedia", {
        writable: true,
        value: vi.fn().mockImplementation(() => ({
          matches: false,
          addEventListener: vi.fn((_event: string, cb: () => void) => {
            changeHandler = cb;
          }),
          removeEventListener: vi.fn(),
        })),
      });
      const cleanup = initAnimationPreference();
      useAnimationStore.getState().setMode("system");

      changeHandler?.({ matches: true });
      expect(isAnimationEnabled()).toBe(false);

      changeHandler?.({ matches: false });
      expect(isAnimationEnabled()).toBe(true);

      cleanup();
    });
  });
});

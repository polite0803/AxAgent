// SPDX-License-Identifier: AGPL-3.0-only

import { create } from "zustand";
import { createJSONStorage, persist } from "zustand/middleware";

/**
 * 动画偏好 store —— 控制界面动画效果是否加载/播放。
 *
 * 三态模式：
 * - "system"（默认）：跟随系统 prefers-reduced-motion，系统开启"减少动态效果"时自动禁用动画
 * - "on"：始终启用动画
 * - "off"：始终禁用动画（消费方应退化为静态渲染，甚至 lazy 不加载动画 chunk）
 *
 * 持久化到 localStorage，为纯 UI 偏好（非会话偏好，不走后端）。
 */
export type AnimationMode = "system" | "on" | "off";

/** localStorage 持久化键名 */
export const ANIMATION_PREFERENCE_KEY = "axagent-animation-preference";

/** prefers-reduced-motion 媒体查询 */
const REDUCED_MOTION_QUERY = "(prefers-reduced-motion: reduce)";

/** 模块级单例监听器（防止多实例重复注册） */
let reducedMotionListenerRegistered = false;

interface AnimationState {
  /** 用户偏好模式 */
  mode: AnimationMode;
  /** 系统级 prefers-reduced-motion 当前状态（由监听器维护） */
  systemPrefersReducedMotion: boolean;
  /** 设置偏好模式 */
  setMode: (mode: AnimationMode) => void;
  /** 计算当前是否应启用动画（供动画门控消费） */
  isAnimationEnabled: () => boolean;
}

/** 读取系统 prefers-reduced-motion 当前值 */
function readSystemPrefersReducedMotion(): boolean {
  return typeof window !== "undefined"
    && typeof window.matchMedia === "function"
    && window.matchMedia(REDUCED_MOTION_QUERY).matches;
}

export const useAnimationStore = create<AnimationState>()(
  persist(
    (set, get) => ({
      mode: "system",
      systemPrefersReducedMotion: readSystemPrefersReducedMotion(),

      setMode: (mode) => set({ mode }),

      isAnimationEnabled: () => {
        const { mode, systemPrefersReducedMotion } = get();
        if (mode === "on") {
          return true;
        }
        if (mode === "off") {
          return false;
        }
        return !systemPrefersReducedMotion;
      },
    }),
    {
      name: ANIMATION_PREFERENCE_KEY,
      storage: createJSONStorage(() => localStorage),
      // 只持久化用户偏好；系统状态永远实时读取
      partialize: (state) => ({ mode: state.mode }),
    },
  ),
);

/** 注册系统 prefers-reduced-motion 监听（仅一次），系统设置变化实时生效 */
export function initAnimationPreference(): () => void {
  if (!reducedMotionListenerRegistered) {
    reducedMotionListenerRegistered = true;
    const mql = typeof window !== "undefined"
      ? window.matchMedia?.(REDUCED_MOTION_QUERY)
      : null;
    const handler = (event: MediaQueryListEvent) => {
      useAnimationStore.setState({ systemPrefersReducedMotion: event.matches });
    };
    mql?.addEventListener?.("change", handler);
    return () => mql?.removeEventListener?.("change", handler);
  }
  return () => {};
}

/** 非 hook 版：在任意位置读取当前是否启用动画（用于门控逻辑） */
export function isAnimationEnabled(): boolean {
  return useAnimationStore.getState().isAnimationEnabled();
}

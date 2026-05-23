import { resolveDeviceLayout, useUIStore } from "@/stores/shared/uiStore";
import { useEffect } from "react";

/** 分辨率的 CSS 媒体查询 breakpoint 常量 */
export const BREAKPOINTS = {
  mobile: 600,
  tablet: 900,
} as const;

/**
 * 监听窗口 resize 自动更新 uiStore.deviceLayout。
 * 在 AppInner 启动时调用一次即可。
 */
export function useResponsive() {
  const setDeviceLayout = useUIStore((s) => s.setDeviceLayout);

  useEffect(() => {
    // 初始检测
    setDeviceLayout(resolveDeviceLayout(window.innerWidth));

    let timer: ReturnType<typeof setTimeout> | null = null;
    const handleResize = () => {
      if (timer) { clearTimeout(timer); }
      timer = setTimeout(() => {
        setDeviceLayout(resolveDeviceLayout(window.innerWidth));
      }, 150);
    };

    window.addEventListener("resize", handleResize);
    return () => {
      window.removeEventListener("resize", handleResize);
      if (timer) { clearTimeout(timer); }
    };
  }, [setDeviceLayout]);
}

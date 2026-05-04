import type { ThemePreset } from "@/theme/shadcnTheme";
import { useEffect, useState } from "react";

const DARK_PRESETS: Set<string> = new Set(["dark-elegance", "dark-neon", "paperclip-dark"]);

export function useResolvedDarkMode(themeMode: string, themePreset?: ThemePreset): boolean {
  const [systemDark, setSystemDark] = useState(
    () => window.matchMedia("(prefers-color-scheme: dark)").matches,
  );

  useEffect(() => {
    if (themeMode !== "system") { return; }
    const mq = window.matchMedia("(prefers-color-scheme: dark)");
    const handler = (e: MediaQueryListEvent) => setSystemDark(e.matches);
    mq.addEventListener("change", handler);
    return () => mq.removeEventListener("change", handler);
  }, [themeMode]);

  // Preset takes highest priority for dark/light determination
  if (themePreset) { return DARK_PRESETS.has(themePreset); }
  if (themeMode === "dark") { return true; }
  if (themeMode === "light") { return false; }
  return systemDark;
}

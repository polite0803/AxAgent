// SPDX-License-Identifier: AGPL-3.0-only

import type { ThemePreset } from "@/theme/shadcnTheme";
import { IS_DARK_PRESET } from "@/theme/shadcnTheme";
import { useEffect, useState } from "react";

export function useResolvedDarkMode(
  themeMode: string,
  themePreset?: ThemePreset,
): boolean {
  const [systemDark, setSystemDark] = useState(
    () => window.matchMedia("(prefers-color-scheme: dark)").matches,
  );

  useEffect(() => {
    if (themeMode !== "system") {
      return;
    }
    const mq = window.matchMedia("(prefers-color-scheme: dark)");
    const handler = (e: MediaQueryListEvent) => setSystemDark(e.matches);
    mq.addEventListener("change", handler);
    return () => mq.removeEventListener("change", handler);
  }, [themeMode]);

  // Preset takes highest priority for dark/light determination
  if (themePreset) {
    return IS_DARK_PRESET[themePreset];
  }
  if (themeMode === "dark") {
    return true;
  }
  if (themeMode === "light") {
    return false;
  }
  return systemDark;
}

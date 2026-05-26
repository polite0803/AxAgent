import { theme } from "antd";
import type { ThemeConfig } from "antd";
import { useMemo } from "react";

export type ThemePreset =
  | "dark-elegance"
  | "dark-neon"
  | "light-professional"
  | "light-minimal"
  | "paperclip-dark"
  | "paperclip-light"
  | "cyberpunk-dark"
  | "cyberpunk-light"
  // 新预设（2026-05 设计改版）
  | "deep-dusk"
  | "light-dawn"
  | "oceanic-dark"
  | "forest-dark";

interface PresetColors {
  bgBase: string;
  bgElevated: string;
  bgSurface2: string;
  borderColor: string;
  borderLight: string;
  textPrimary: string;
  textSecondary: string;
  textFg2: string;
  primaryColor: string;
  successColor: string;
  errorColor: string;
  warningColor: string;
  borderRadiusBias: number;
  shadowStyle: "soft-dark" | "glow" | "soft-light" | "none" | "border-only";
}

const PRESETS: Record<ThemePreset, PresetColors> = {
  "dark-elegance": {
    bgBase: "#141414",
    bgElevated: "#1a1a1a",
    bgSurface2: "#1f1f1f",
    borderColor: "#2a2a2a",
    borderLight: "#222",
    textPrimary: "rgba(255,255,255,0.85)",
    textSecondary: "rgba(255,255,255,0.45)",
    textFg2: "rgba(255,255,255,0.65)",
    primaryColor: "#1677ff",
    successColor: "#49aa19",
    errorColor: "#dc4446",
    warningColor: "#d89614",
    borderRadiusBias: 0,
    shadowStyle: "soft-dark",
  },
  "dark-neon": {
    bgBase: "#0a0a0f",
    bgElevated: "#12121a",
    bgSurface2: "#17172a",
    borderColor: "#1e1e3a",
    borderLight: "#181830",
    textPrimary: "rgba(230,230,255,0.9)",
    textSecondary: "rgba(200,200,255,0.5)",
    textFg2: "rgba(200,200,255,0.7)",
    primaryColor: "#a855f7",
    successColor: "#4ade80",
    errorColor: "#f87171",
    warningColor: "#fbbf24",
    borderRadiusBias: -2,
    shadowStyle: "glow",
  },
  "light-professional": {
    bgBase: "#ffffff",
    bgElevated: "#fafafa",
    bgSurface2: "#f0f0f0",
    borderColor: "#e5e7eb",
    borderLight: "#f3f4f6",
    textPrimary: "rgba(0,0,0,0.88)",
    textSecondary: "rgba(0,0,0,0.45)",
    textFg2: "rgba(0,0,0,0.65)",
    primaryColor: "#1677ff",
    successColor: "#49aa19",
    errorColor: "#dc4446",
    warningColor: "#d89614",
    borderRadiusBias: 0,
    shadowStyle: "soft-light",
  },
  "light-minimal": {
    bgBase: "#f8fafc",
    bgElevated: "#ffffff",
    bgSurface2: "#f1f5f9",
    borderColor: "#f1f5f9",
    borderLight: "#f8fafc",
    textPrimary: "rgba(0,0,0,0.9)",
    textSecondary: "rgba(0,0,0,0.35)",
    textFg2: "rgba(0,0,0,0.55)",
    primaryColor: "#0891b2",
    successColor: "#16a34a",
    errorColor: "#dc2626",
    warningColor: "#d97706",
    borderRadiusBias: 4,
    shadowStyle: "none",
  },
  // 深色暮色 — 暖深灰基底 + 青绿强调（2026 设计改版默认）
  "deep-dusk": {
    bgBase: "oklch(17% 0.008 55)",
    bgElevated: "oklch(21% 0.01 55)",
    bgSurface2: "oklch(24% 0.011 55)",
    borderColor: "oklch(28% 0.01 55)",
    borderLight: "oklch(25% 0.008 55)",
    textPrimary: "rgba(255,255,255,0.92)",
    textSecondary: "rgba(255,255,255,0.65)",
    textFg2: "rgba(255,255,255,0.80)",
    primaryColor: "oklch(62% 0.16 195)",
    successColor: "oklch(62% 0.18 150)",
    errorColor: "oklch(60% 0.20 30)",
    warningColor: "oklch(68% 0.16 85)",
    borderRadiusBias: 0,
    shadowStyle: "border-only",
  },
  // 亮色晨曦 — 暖白基底 + 青绿强调
  "light-dawn": {
    bgBase: "oklch(96% 0.006 75)",
    bgElevated: "oklch(99% 0.004 75)",
    bgSurface2: "oklch(93% 0.006 75)",
    borderColor: "oklch(88% 0.005 70)",
    borderLight: "oklch(92% 0.004 70)",
    textPrimary: "oklch(20% 0.01 70)",
    textSecondary: "oklch(55% 0.006 70)",
    textFg2: "oklch(45% 0.008 70)",
    primaryColor: "oklch(54% 0.18 195)",
    successColor: "oklch(52% 0.18 150)",
    errorColor: "oklch(52% 0.20 30)",
    warningColor: "oklch(58% 0.16 85)",
    borderRadiusBias: 0,
    shadowStyle: "soft-light",
  },
  // 海洋深色 — 冷蓝灰基底 + 亮蓝强调
  "oceanic-dark": {
    bgBase: "oklch(15% 0.012 250)",
    bgElevated: "oklch(19% 0.014 250)",
    bgSurface2: "oklch(22% 0.015 250)",
    borderColor: "oklch(26% 0.012 250)",
    borderLight: "oklch(23% 0.01 250)",
    textPrimary: "rgba(255,255,255,0.92)",
    textSecondary: "rgba(255,255,255,0.65)",
    textFg2: "rgba(255,255,255,0.80)",
    primaryColor: "oklch(60% 0.18 220)",
    successColor: "oklch(60% 0.16 150)",
    errorColor: "oklch(58% 0.18 30)",
    warningColor: "oklch(65% 0.14 85)",
    borderRadiusBias: 0,
    shadowStyle: "border-only",
  },
  // 森林深色 — 绿调基底 + 翠绿强调
  "forest-dark": {
    bgBase: "oklch(16% 0.012 140)",
    bgElevated: "oklch(20% 0.014 140)",
    bgSurface2: "oklch(23% 0.015 140)",
    borderColor: "oklch(26% 0.01 140)",
    borderLight: "oklch(23% 0.008 140)",
    textPrimary: "rgba(255,255,255,0.92)",
    textSecondary: "rgba(255,255,255,0.65)",
    textFg2: "rgba(255,255,255,0.80)",
    primaryColor: "oklch(58% 0.18 150)",
    successColor: "oklch(62% 0.18 145)",
    errorColor: "oklch(58% 0.20 30)",
    warningColor: "oklch(65% 0.16 85)",
    borderRadiusBias: 0,
    shadowStyle: "border-only",
  },
  "paperclip-dark": {
    bgBase: "#18181b",
    bgElevated: "#27272a",
    bgSurface2: "#303036",
    borderColor: "#3f3f46",
    borderLight: "#2a2a30",
    textPrimary: "rgba(250,250,250,0.95)",
    textSecondary: "rgba(161,161,170,0.85)",
    textFg2: "rgba(200,200,200,0.9)",
    primaryColor: "#a1a1aa",
    successColor: "#4ade80",
    errorColor: "#f87171",
    warningColor: "#fbbf24",
    borderRadiusBias: 0,
    shadowStyle: "border-only",
  },
  "paperclip-light": {
    bgBase: "#fafafa",
    bgElevated: "#ffffff",
    bgSurface2: "#f4f4f5",
    borderColor: "#e4e4e7",
    borderLight: "#f0f0f0",
    textPrimary: "rgba(24,24,27,0.92)",
    textSecondary: "rgba(113,113,122,0.85)",
    textFg2: "rgba(63,63,70,0.85)",
    primaryColor: "#71717a",
    successColor: "#16a34a",
    errorColor: "#dc2626",
    warningColor: "#d97706",
    borderRadiusBias: 0,
    shadowStyle: "border-only",
  },
  "cyberpunk-dark": {
    bgBase: "#0a0a12",
    bgElevated: "#12121f",
    bgSurface2: "#18182a",
    borderColor: "#1e1e3a",
    borderLight: "#161630",
    textPrimary: "rgba(224,224,255,0.92)",
    textSecondary: "rgba(160,160,210,0.7)",
    textFg2: "rgba(190,190,230,0.82)",
    primaryColor: "#00f0ff",
    successColor: "#00ff88",
    errorColor: "#ff4466",
    warningColor: "#ffaa00",
    borderRadiusBias: -2,
    shadowStyle: "glow",
  },
  "cyberpunk-light": {
    bgBase: "#f0f0f8",
    bgElevated: "#ffffff",
    bgSurface2: "#e8e8f0",
    borderColor: "#c8c8e0",
    borderLight: "#d8d8e8",
    textPrimary: "rgba(10,10,30,0.92)",
    textSecondary: "rgba(80,80,140,0.7)",
    textFg2: "rgba(50,50,100,0.82)",
    primaryColor: "#7b2ff7",
    successColor: "#16a34a",
    errorColor: "#dc2626",
    warningColor: "#d97706",
    borderRadiusBias: -2,
    shadowStyle: "soft-light",
  },
};

export const IS_DARK_PRESET: Record<ThemePreset, boolean> = {
  "dark-elegance": true,
  "dark-neon": true,
  "light-professional": false,
  "light-minimal": false,
  "paperclip-dark": true,
  "paperclip-light": false,
  "cyberpunk-dark": true,
  "cyberpunk-light": false,
  "deep-dusk": true,
  "light-dawn": false,
  "oceanic-dark": true,
  "forest-dark": true,
};

function resolveShadow(preset: PresetColors): {
  boxShadow: string;
  boxShadowSecondary: string;
} {
  switch (preset.shadowStyle) {
    case "soft-dark":
      return {
        boxShadow: "0 2px 8px 0 rgba(0,0,0,0.3), 0 1px 3px -1px rgba(0,0,0,0.4)",
        boxShadowSecondary: "0 6px 16px -2px rgba(0,0,0,0.4), 0 3px 8px -4px rgba(0,0,0,0.5)",
      };
    case "glow":
      return {
        boxShadow: "0 2px 12px 0 rgba(0,240,255,0.12), 0 1px 4px -1px rgba(0,240,255,0.08)",
        boxShadowSecondary: "0 6px 24px -2px rgba(0,240,255,0.18), 0 3px 10px -4px rgba(0,240,255,0.12)",
      };
    case "soft-light":
      return {
        boxShadow: "0 1px 3px 0 rgba(0,0,0,0.08), 0 1px 2px -1px rgba(0,0,0,0.06)",
        boxShadowSecondary: "0 4px 6px -1px rgba(0,0,0,0.08), 0 2px 4px -2px rgba(0,0,0,0.06)",
      };
    case "none":
      return { boxShadow: "none", boxShadowSecondary: "none" };
    case "border-only":
      return {
        boxShadow: "0 1px 2px 0 rgba(0,0,0,0.1)",
        boxShadowSecondary: "0 1px 3px 0 rgba(0,0,0,0.06)",
      };
  }
}

/**
 * Theme config that supports user-specified preset OR custom primary_color override.
 *
 * When theme_preset is set, its base colors are used as the foundation.
 * The user can still override primary_color via DisplaySettings.
 */
export function useShadcnTheme(
  isDark: boolean,
  primaryColor: string,
  fontSize: number,
  borderRadius: number,
  fontFamily?: string,
  codeFontFamily?: string,
  themePreset?: ThemePreset,
): ThemeConfig {
  return useMemo<ThemeConfig>(() => {
    // Resolve preset: when user has set theme_preset, use its colors as base
    const preset = themePreset && PRESETS[themePreset] ? PRESETS[themePreset] : null;

    // Derive proportional radii from the base value, optionally biased by preset
    const baseRadius = preset
      ? borderRadius + preset.borderRadiusBias
      : borderRadius;
    const radiusSM = Math.max(0, Math.round(baseRadius * 0.6));
    const radiusXS = Math.max(0, Math.round(baseRadius * 0.2));
    const radiusLG = Math.max(0, Math.round(baseRadius * 1.4));

    // Preset determines dark/light algorithm; fall back to isDark flag
    const effectiveDark = preset ? IS_DARK_PRESET[themePreset!] : isDark;
    const algorithm = effectiveDark
      ? theme.darkAlgorithm
      : theme.defaultAlgorithm;

    // Resolve primary: preset default or user override
    const effectivePrimary = primaryColor !== DEFAULT_SETTINGS_PRIMARY
      ? primaryColor
      : (preset?.primaryColor ?? primaryColor);

    const shadows = preset
      ? resolveShadow(preset)
      : {
        boxShadow: "0 1px 3px 0 rgba(0,0,0,0.1), 0 1px 2px -1px rgba(0,0,0,0.1)",
        boxShadowSecondary: "0 4px 6px -1px rgba(0,0,0,0.1), 0 2px 4px -2px rgba(0,0,0,0.1)",
      };

    // Derive accent alpha string for shadow / active states
    const accentAlpha = preset
      ? preset.primaryColor.replace(/\)$/, " / 0.18)")
      : "rgba(0,168,168,0.18)";

    return {
      algorithm,
      token: {
        // ── 品牌色 ──
        colorPrimary: effectivePrimary,
        colorLink: effectivePrimary,
        colorSuccess: preset?.successColor ?? "#49aa19",
        colorError: preset?.errorColor ?? "#dc4446",
        colorWarning: preset?.warningColor ?? "#d89614",
        colorInfo: effectivePrimary,

        // ── 背景色 — 从原型 bg/surface/surface-2 映射 ──
        colorBgLayout: preset?.bgBase ?? (effectiveDark ? "#141414" : "#ffffff"),
        colorBgContainer: preset?.bgElevated ?? (effectiveDark ? "#1a1a1a" : "#ffffff"),
        colorBgElevated: preset?.bgSurface2 ?? (effectiveDark ? "#1f1f1f" : "#f5f5f5"),
        colorFillSecondary: preset?.bgSurface2 ?? (effectiveDark ? "#1f1f1f" : "#f5f5f5"),
        colorFillTertiary: preset?.bgBase ?? (effectiveDark ? "#141414" : "#ffffff"),

        // ── 文字色 — 从原型 fg/fg-2/muted 映射 ──
        colorText: preset?.textPrimary ?? (effectiveDark ? "rgba(255,255,255,0.85)" : "rgba(0,0,0,0.88)"),
        colorTextSecondary: preset?.textSecondary ?? (effectiveDark ? "rgba(255,255,255,0.45)" : "rgba(0,0,0,0.45)"),
        colorTextTertiary: preset?.textSecondary ?? (effectiveDark ? "rgba(255,255,255,0.45)" : "rgba(0,0,0,0.45)"),
        colorTextQuaternary: preset?.textFg2 ?? (effectiveDark ? "rgba(255,255,255,0.25)" : "rgba(0,0,0,0.25)"),

        // ── 边框色 — 从原型 border/border-light 映射 ──
        colorBorder: preset?.borderColor ?? (effectiveDark ? "#2a2a2a" : "#e5e7eb"),
        colorBorderSecondary: preset?.borderLight ?? (effectiveDark ? "#1f1f1f" : "#f0f0f0"),

        // ── 排版 ──
        fontSize,
        fontSizeSM: 12,
        fontWeightStrong: 500,
        ...(fontFamily ? { fontFamily } : {}),
        ...(codeFontFamily ? { fontFamilyCode: codeFontFamily } : {}),

        // ── 圆角 ──
        borderRadius: baseRadius,
        borderRadiusXS: radiusXS,
        borderRadiusSM: radiusSM,
        borderRadiusLG: radiusLG,

        // ── 间距 ──
        padding: 16,
        paddingSM: 12,
        paddingLG: 24,
        margin: 16,
        marginSM: 12,
        marginLG: 24,

        // ── 阴影 ──
        boxShadow: shadows.boxShadow,
        boxShadowSecondary: shadows.boxShadowSecondary,
        lineWidth: 1,
        lineType: "solid",
        sizeUnit: 4,
        sizeStep: 4,
        sizePopupArrow: 8,
        controlHeight: 28,
        controlHeightSM: 24,
        controlHeightLG: 32,
      },
      components: {
        Button: {
          controlHeight: 28,
          controlHeightSM: 24,
          borderRadiusSM: baseRadius,
          paddingContentHorizontal: 12,
          paddingContentHorizontalSM: 8,
          primaryShadow: "none",
          defaultShadow: "none",
          dangerShadow: "none",
        },
        Input: {
          paddingBlock: 6,
          paddingInline: 10,
          activeBorderColor: effectivePrimary,
          activeShadow: `0 0 0 2px ${accentAlpha}`,
        },
        Select: {
          optionSelectedFontWeight: 500,
        },
        Tag: {
          defaultBg: "transparent",
        },
        Tabs: {
          inkBarColor: effectivePrimary,
          itemColor: preset?.textSecondary ?? "rgba(0,0,0,0.45)",
          itemHoverColor: preset?.textPrimary ?? "rgba(0,0,0,0.88)",
          itemSelectedColor: effectivePrimary,
          horizontalItemPadding: "8px 0",
        },
        Card: {
          paddingLG: 16,
          borderRadiusLG: 10,
        },
        Divider: {
          colorSplit: preset?.borderColor ?? (effectiveDark ? "#2a2a2a" : "#e5e7eb"),
        },
        Modal: {
          borderRadiusLG: Math.min(Math.max(radiusLG, 4), 8),
        },
        Slider: {
          handleSize: 8,
          handleSizeHover: 10,
          railSize: 4,
        },
        Switch: {
          trackHeight: 22,
          trackMinWidth: 44,
          handleSize: 18,
        },
      },
    };
  }, [
    isDark,
    primaryColor,
    fontSize,
    borderRadius,
    fontFamily,
    codeFontFamily,
    themePreset,
  ]);
}

/** Default primary color from settings store — used as sentinel for "user hasn't changed it" */
const DEFAULT_SETTINGS_PRIMARY = "#17A93D";

import { theme } from "antd";
import type { ThemeConfig } from "antd";
import { useMemo } from "react";

export type ThemePreset =
  | "dark-elegance"
  | "dark-neon"
  | "light-professional"
  | "light-minimal"
  | "paperclip-dark"
  | "paperclip-light";

interface PresetColors {
  bgBase: string;
  bgElevated: string;
  borderColor: string;
  textPrimary: string;
  textSecondary: string;
  primaryColor: string;
  borderRadiusBias: number;
  shadowStyle: "soft-dark" | "glow" | "soft-light" | "none" | "border-only";
}

const PRESETS: Record<ThemePreset, PresetColors> = {
  "dark-elegance": {
    bgBase: "#141414",
    bgElevated: "#1a1a1a",
    borderColor: "#2a2a2a",
    textPrimary: "rgba(255,255,255,0.85)",
    textSecondary: "rgba(255,255,255,0.45)",
    primaryColor: "#1677ff",
    borderRadiusBias: 0,
    shadowStyle: "soft-dark",
  },
  "dark-neon": {
    bgBase: "#0a0a0f",
    bgElevated: "#12121a",
    borderColor: "#1e1e3a",
    textPrimary: "rgba(230,230,255,0.9)",
    textSecondary: "rgba(200,200,255,0.5)",
    primaryColor: "#a855f7",
    borderRadiusBias: -2,
    shadowStyle: "glow",
  },
  "light-professional": {
    bgBase: "#ffffff",
    bgElevated: "#fafafa",
    borderColor: "#e5e7eb",
    textPrimary: "rgba(0,0,0,0.88)",
    textSecondary: "rgba(0,0,0,0.45)",
    primaryColor: "#1677ff",
    borderRadiusBias: 0,
    shadowStyle: "soft-light",
  },
  "light-minimal": {
    bgBase: "#f8fafc",
    bgElevated: "#ffffff",
    borderColor: "#f1f5f9",
    textPrimary: "rgba(0,0,0,0.9)",
    textSecondary: "rgba(0,0,0,0.35)",
    primaryColor: "#0891b2",
    borderRadiusBias: 4,
    shadowStyle: "none",
  },
  "paperclip-dark": {
    bgBase: "#18181b",
    bgElevated: "#27272a",
    borderColor: "#3f3f46",
    textPrimary: "rgba(250,250,250,0.95)",
    textSecondary: "rgba(161,161,170,0.85)",
    primaryColor: "#a1a1aa",
    borderRadiusBias: 0,
    shadowStyle: "border-only",
  },
  "paperclip-light": {
    bgBase: "#fafafa",
    bgElevated: "#ffffff",
    borderColor: "#e4e4e7",
    textPrimary: "rgba(24,24,27,0.92)",
    textSecondary: "rgba(113,113,122,0.85)",
    primaryColor: "#71717a",
    borderRadiusBias: 0,
    shadowStyle: "border-only",
  },
};

const IS_DARK_PRESET: Record<ThemePreset, boolean> = {
  "dark-elegance": true,
  "dark-neon": true,
  "light-professional": false,
  "light-minimal": false,
  "paperclip-dark": true,
  "paperclip-light": false,
};

function resolveShadow(preset: PresetColors): { boxShadow: string; boxShadowSecondary: string } {
  switch (preset.shadowStyle) {
    case "soft-dark":
      return {
        boxShadow: "0 2px 8px 0 rgba(0,0,0,0.3), 0 1px 3px -1px rgba(0,0,0,0.4)",
        boxShadowSecondary: "0 6px 16px -2px rgba(0,0,0,0.4), 0 3px 8px -4px rgba(0,0,0,0.5)",
      };
    case "glow":
      return {
        boxShadow: "0 2px 12px 0 rgba(168,85,247,0.15), 0 1px 4px -1px rgba(168,85,247,0.1)",
        boxShadowSecondary: "0 6px 24px -2px rgba(168,85,247,0.2), 0 3px 10px -4px rgba(168,85,247,0.15)",
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
    const baseRadius = preset ? borderRadius + preset.borderRadiusBias : borderRadius;
    const radiusSM = Math.max(0, Math.round(baseRadius * 0.6));
    const radiusXS = Math.max(0, Math.round(baseRadius * 0.2));
    const radiusLG = Math.max(0, Math.round(baseRadius * 1.4));

    // Preset determines dark/light algorithm; fall back to isDark flag
    const effectiveDark = preset ? IS_DARK_PRESET[themePreset!] : isDark;
    const algorithm = effectiveDark ? theme.darkAlgorithm : theme.defaultAlgorithm;

    // Resolve primary: preset default or user override
    const effectivePrimary = primaryColor !== DEFAULT_SETTINGS_PRIMARY
      ? primaryColor
      : preset?.primaryColor ?? primaryColor;

    const shadows = preset ? resolveShadow(preset) : {
      boxShadow: "0 1px 3px 0 rgba(0,0,0,0.1), 0 1px 2px -1px rgba(0,0,0,0.1)",
      boxShadowSecondary: "0 4px 6px -1px rgba(0,0,0,0.1), 0 2px 4px -2px rgba(0,0,0,0.1)",
    };

    return {
      algorithm,
      token: {
        colorPrimary: effectivePrimary,
        colorLink: effectivePrimary,
        fontSize,
        fontSizeSM: 12,
        fontWeightStrong: 500,
        ...(fontFamily ? { fontFamily } : {}),
        ...(codeFontFamily ? { fontFamilyCode: codeFontFamily } : {}),

        borderRadius: baseRadius,
        borderRadiusXS: radiusXS,
        borderRadiusSM: radiusSM,
        borderRadiusLG: radiusLG,

        padding: 16,
        paddingSM: 12,
        paddingLG: 24,
        margin: 16,
        marginSM: 12,
        marginLG: 24,

        boxShadow: shadows.boxShadow,
        boxShadowSecondary: shadows.boxShadowSecondary,
      },
      components: {
        Button: {
          primaryShadow: "none",
          defaultShadow: "none",
          dangerShadow: "none",
        },
        Input: {
          activeShadow: "none",
        },
        Select: {
          optionSelectedFontWeight: 500,
        },
        Modal: {
          borderRadiusLG: Math.min(Math.max(radiusLG, 4), 8),
        },
        Slider: {
          handleSize: 8,
          handleSizeHover: 10,
          railSize: 4,
        },
      },
    };
  }, [isDark, primaryColor, fontSize, borderRadius, fontFamily, codeFontFamily, themePreset]);
}

/** Default primary color from settings store — used as sentinel for "user hasn't changed it" */
const DEFAULT_SETTINGS_PRIMARY = "#17A93D";

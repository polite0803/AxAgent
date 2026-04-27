import { Dropdown, theme, Tooltip } from "antd";
import { Check, ChevronRight, Copy, Eye, Maximize2, Minimize2, Minus, Palette, Plus, RotateCcw } from "lucide-react";
import React, { useCallback, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
// CodeBlockActionContext is no longer exported from markstream-react.
// This component is kept for reference but is not used by the new built-in code block header.
// The new markstream-react version uses built-in header actions with event callbacks.

interface CodeBlockActionContext {
  collapsed: boolean;
  toggleCollapse: () => void;
  copied: boolean;
  copy: () => void;
  expanded: boolean;
  toggleExpand: () => void;
  fontSize: number;
  defaultFontSize: number;
  decreaseFontSize: () => void;
  increaseFontSize: () => void;
  resetFontSize: () => void;
  isPreviewable: boolean;
  previewCode: () => void;
}
import { formatThemeName, SHIKI_DARK_THEMES, SHIKI_LIGHT_THEMES } from "@/constants/codeThemes";
import { useResolvedDarkMode } from "@/hooks/useResolvedDarkMode";
import { useSettingsStore } from "@/stores";
import type { MenuProps } from "antd";

interface Props {
  ctx: CodeBlockActionContext;
}

export const CodeBlockHeaderActions: React.FC<Props> = ({ ctx }) => {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const settings = useSettingsStore((s) => s.settings);
  const saveSettings = useSettingsStore((s) => s.saveSettings);
  const isDark = useResolvedDarkMode(settings.theme_mode);
  const [hoveredIdx, setHoveredIdx] = useState<number | null>(null);

  const currentTheme = isDark ? (settings.code_theme || "poimandres") : (settings.code_theme_light || "github-light");
  const themeList = isDark ? SHIKI_DARK_THEMES : SHIKI_LIGHT_THEMES;
  const settingsKey = isDark ? "code_theme" : "code_theme_light";

  const themeMenuItems: MenuProps["items"] = useMemo(() =>
    themeList.map((id) => ({
      key: id,
      label: (
        <span style={{ display: "flex", alignItems: "center", gap: 6 }}>
          {id === currentTheme && <Check size={12} style={{ color: token.colorPrimary, flexShrink: 0 }} />}
          <span style={{ marginLeft: id === currentTheme ? 0 : 18 }}>{formatThemeName(id)}</span>
        </span>
      ),
    })), [themeList, currentTheme, token.colorPrimary]);

  const handleThemeSelect = useCallback<NonNullable<MenuProps["onClick"]>>(
    ({ key }) => {
      saveSettings({ [settingsKey]: key });
    },
    [saveSettings, settingsKey],
  );

  const getBtnStyle = useCallback((idx: number, disabled?: boolean): React.CSSProperties => ({
    display: "inline-flex",
    alignItems: "center",
    justifyContent: "center",
    width: 24,
    height: 24,
    padding: 0,
    border: "none",
    borderRadius: token.borderRadiusSM,
    background: !disabled && hoveredIdx === idx ? (token.colorFillSecondary || "rgba(255,255,255,0.1)") : "transparent",
    color: !disabled && hoveredIdx === idx ? token.colorText : "inherit",
    cursor: disabled ? "not-allowed" : "pointer",
    transition: "color 0.2s, background 0.2s",
    opacity: disabled ? 0.3 : (hoveredIdx === idx ? 1 : 0.7),
  }), [hoveredIdx, token]);

  return (
    <div style={{ display: "flex", alignItems: "center", gap: 2 }}>
      {/* Collapse/Expand */}
      <Tooltip title={ctx.collapsed ? t("common.expand") : t("common.collapse")} mouseEnterDelay={0.4}>
        <button
          type="button"
          className="code-action-btn"
          style={getBtnStyle(0)}
          onClick={ctx.toggleCollapse}
          onMouseEnter={() => setHoveredIdx(0)}
          onMouseLeave={() => setHoveredIdx(null)}
        >
          <ChevronRight
            size={14}
            style={{
              transform: ctx.collapsed ? "rotate(0deg)" : "rotate(90deg)",
              transition: "transform 0.2s",
            }}
          />
        </button>
      </Tooltip>

      {/* Font Decrease */}
      <Tooltip title={t("common.decrease", { defaultValue: "Decrease" })} mouseEnterDelay={0.4}>
        <button
          type="button"
          className="code-action-btn"
          style={getBtnStyle(1, ctx.fontSize <= 10)}
          disabled={ctx.fontSize <= 10}
          onClick={ctx.decreaseFontSize}
          onMouseEnter={() => setHoveredIdx(1)}
          onMouseLeave={() => setHoveredIdx(null)}
        >
          <Minus size={14} />
        </button>
      </Tooltip>

      {/* Font Reset */}
      <Tooltip title={t("common.reset", { defaultValue: "Reset" })} mouseEnterDelay={0.4}>
        <button
          type="button"
          className="code-action-btn"
          style={getBtnStyle(2, ctx.fontSize === ctx.defaultFontSize)}
          disabled={ctx.fontSize === ctx.defaultFontSize}
          onClick={ctx.resetFontSize}
          onMouseEnter={() => setHoveredIdx(2)}
          onMouseLeave={() => setHoveredIdx(null)}
        >
          <RotateCcw size={14} />
        </button>
      </Tooltip>

      {/* Font Increase */}
      <Tooltip title={t("common.increase", { defaultValue: "Increase" })} mouseEnterDelay={0.4}>
        <button
          type="button"
          className="code-action-btn"
          style={getBtnStyle(3, ctx.fontSize >= 36)}
          disabled={ctx.fontSize >= 36}
          onClick={ctx.increaseFontSize}
          onMouseEnter={() => setHoveredIdx(3)}
          onMouseLeave={() => setHoveredIdx(null)}
        >
          <Plus size={14} />
        </button>
      </Tooltip>

      {/* Copy */}
      <Tooltip title={ctx.copied ? t("common.copied") : t("common.copy")} mouseEnterDelay={0.4}>
        <button
          type="button"
          className="code-action-btn"
          style={getBtnStyle(4)}
          onClick={() => ctx.copy()}
          onMouseEnter={() => setHoveredIdx(4)}
          onMouseLeave={() => setHoveredIdx(null)}
        >
          {ctx.copied
            ? <Check size={14} style={{ color: token.colorSuccess }} />
            : <Copy size={14} />}
        </button>
      </Tooltip>

      {/* Fullscreen */}
      <Tooltip title={ctx.expanded ? t("common.collapse") : t("settings.fullscreen")} mouseEnterDelay={0.4}>
        <button
          type="button"
          className="code-action-btn"
          style={getBtnStyle(5)}
          onClick={ctx.toggleExpand}
          onMouseEnter={() => setHoveredIdx(5)}
          onMouseLeave={() => setHoveredIdx(null)}
        >
          {ctx.expanded ? <Minimize2 size={14} /> : <Maximize2 size={14} />}
        </button>
      </Tooltip>

      {/* Theme Picker */}
      <Dropdown
        menu={{ items: themeMenuItems, onClick: handleThemeSelect, style: { maxHeight: 320, overflowY: "auto" } }}
        trigger={["click"]}
        placement="bottomRight"
      >
        <Tooltip title={t("settings.codeTheme", { defaultValue: "Code Theme" })} mouseEnterDelay={0.4}>
          <button
            type="button"
            className="code-action-btn"
            style={getBtnStyle(6)}
            onMouseEnter={() => setHoveredIdx(6)}
            onMouseLeave={() => setHoveredIdx(null)}
          >
            <Palette size={14} />
          </button>
        </Tooltip>
      </Dropdown>

      {/* Preview (only for HTML/SVG) */}
      {ctx.isPreviewable && (
        <Tooltip title={t("common.preview")} mouseEnterDelay={0.4}>
          <button
            type="button"
            className="code-action-btn"
            style={getBtnStyle(7)}
            onClick={ctx.previewCode}
            onMouseEnter={() => setHoveredIdx(7)}
            onMouseLeave={() => setHoveredIdx(null)}
          >
            <Eye size={14} />
          </button>
        </Tooltip>
      )}
    </div>
  );
};

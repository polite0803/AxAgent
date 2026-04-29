import { formatThemeName, SHIKI_DARK_THEMES, SHIKI_LIGHT_THEMES } from "@/constants/codeThemes";
import { invoke, isTauri } from "@/lib/invoke";
import { useSettingsStore } from "@/stores";
import type { ThemePreset } from "@/theme/shadcnTheme";
import { ColorPicker, Divider, Segmented, Slider, Tooltip } from "antd";
import { Monitor, Moon, Sun } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { SettingsGroup } from "./SettingsGroup";
import { SettingsSelect } from "./SettingsSelect";

const THEME_PRESETS: { key: ThemePreset; label: string; bg: string; accent: string }[] = [
  { key: "dark-elegance", label: "Dark Elegance", bg: "#141414", accent: "#1677ff" },
  { key: "dark-neon", label: "Dark Neon", bg: "#0a0a0f", accent: "#a855f7" },
  { key: "light-professional", label: "Light Pro", bg: "#ffffff", accent: "#1677ff" },
  { key: "light-minimal", label: "Light Minimal", bg: "#f8fafc", accent: "#0891b2" },
];

export function DisplaySettings() {
  const { t } = useTranslation();
  const settings = useSettingsStore((s) => s.settings);
  const saveSettings = useSettingsStore((s) => s.saveSettings);
  const [systemFonts, setSystemFonts] = useState<string[]>([]);

  useEffect(() => {
    if (!isTauri()) { return; }
    invoke<string[]>("list_system_fonts").then(setSystemFonts).catch(() => {});
  }, []);

  const rowStyle = { padding: "4px 0" };

  const lightThemeOptions = useMemo(
    () => SHIKI_LIGHT_THEMES.map((id) => ({ label: formatThemeName(id), value: id })),
    [],
  );
  const darkThemeOptions = useMemo(
    () => SHIKI_DARK_THEMES.map((id) => ({ label: formatThemeName(id), value: id })),
    [],
  );

  return (
    <div className="p-6 pb-12">
      <SettingsGroup title={t("settings.groupTheme")}>
        <div style={rowStyle} className="flex items-center justify-between">
          <span>{t("settings.theme")}</span>
          <Segmented
            data-testid="dark-mode-toggle"
            value={settings.theme_mode}
            onChange={(val) => saveSettings({ theme_mode: val as string })}
            options={[
              { label: t("settings.themeSystem"), value: "system", icon: <Monitor size={14} /> },
              { label: t("settings.themeLight"), value: "light", icon: <Sun size={14} /> },
              { label: t("settings.themeDark"), value: "dark", icon: <Moon size={14} /> },
            ]}
          />
        </div>
        <Divider style={{ margin: "4px 0" }} />
        <div style={rowStyle} className="flex items-start justify-between">
          <span style={{ paddingTop: 4 }}>{t("settings.themePreset")}</span>
          <div style={{ display: "flex", gap: 6 }}>
            {THEME_PRESETS.map((preset) => {
              const isActive = (settings.theme_preset || "dark-elegance") === preset.key;
              return (
                <Tooltip key={preset.key} title={preset.label}>
                  <div
                    onClick={() => saveSettings({ theme_preset: preset.key })}
                    style={{
                      width: 48,
                      height: 48,
                      borderRadius: 8,
                      backgroundColor: preset.bg,
                      cursor: "pointer",
                      border: isActive ? `2px solid ${preset.accent}` : "2px solid transparent",
                      display: "flex",
                      flexDirection: "column",
                      alignItems: "center",
                      justifyContent: "center",
                      gap: 4,
                      transition: "border-color 0.2s",
                      boxShadow: isActive ? `0 0 0 1px ${preset.accent}` : "none",
                    }}
                  >
                    <div
                      style={{
                        width: 20,
                        height: 4,
                        borderRadius: 2,
                        backgroundColor: preset.accent,
                      }}
                    />
                    <div style={{ display: "flex", gap: 2 }}>
                      <div style={{ width: 8, height: 8, borderRadius: 2, backgroundColor: preset.accent, opacity: 0.6 }} />
                      <div style={{ width: 8, height: 8, borderRadius: 2, backgroundColor: preset.accent, opacity: 0.3 }} />
                    </div>
                  </div>
                </Tooltip>
              );
            })}
          </div>
        </div>
        <Divider style={{ margin: "4px 0" }} />
        <div style={rowStyle} className="flex items-center justify-between">
          <span>{t("settings.primaryColor")}</span>
          <div style={{ display: "flex", gap: 8, alignItems: "center", flexWrap: "wrap" }}>
            {[
              "#17A93D",
              "#1677ff",
              "#1890ff",
              "#13c2c2",
              "#2f54eb",
              "#722ed1",
              "#eb2f96",
              "#fa541c",
              "#faad14",
              "#fadb14",
              "#a0d911",
              "#000000",
            ].map((color) => (
              <div
                key={color}
                onClick={() => saveSettings({ primary_color: color })}
                style={{
                  width: 24,
                  height: 24,
                  borderRadius: "50%",
                  backgroundColor: color,
                  cursor: "pointer",
                  border: settings.primary_color === color
                    ? "2px solid currentColor"
                    : "2px solid transparent",
                  boxShadow: settings.primary_color === color
                    ? `0 0 0 1px ${color}`
                    : "none",
                  transition: "all 0.2s",
                }}
              />
            ))}
            <ColorPicker
              value={settings.primary_color}
              onChangeComplete={(color) => saveSettings({ primary_color: color.toHexString() })}
              size="small"
            />
          </div>
        </div>
      </SettingsGroup>
      <SettingsGroup title={t("settings.groupFontRadius")}>
        <div style={{ padding: "4px 0" }}>
          <span>{t("settings.fontSize")}</span>
          <Slider
            min={12}
            max={20}
            value={settings.font_size}
            onChange={(val) => saveSettings({ font_size: val })}
            marks={{ 12: "12", 14: "14", 16: "16", 18: "18", 20: "20" }}
          />
        </div>
        <Divider style={{ margin: "4px 0" }} />
        <div style={{ padding: "4px 0" }}>
          <span>{t("settings.fontWeight")}</span>
          <Slider
            min={100}
            max={900}
            step={100}
            value={settings.font_weight}
            onChange={(val) => saveSettings({ font_weight: val })}
            marks={{ 100: "100", 300: "300", 400: "400", 500: "500", 700: "700", 900: "900" }}
          />
        </div>
        <Divider style={{ margin: "4px 0" }} />
        <div style={rowStyle} className="flex items-center justify-between">
          <span>{t("settings.fontFamily")}</span>
          <SettingsSelect
            searchable
            value={settings.font_family || ""}
            onChange={(val) => saveSettings({ font_family: val })}
            options={[
              { label: t("settings.fontDefault"), value: "" },
              ...systemFonts.map((f) => ({ label: f, value: f })),
            ]}
          />
        </div>
        <Divider style={{ margin: "4px 0" }} />
        <div style={rowStyle} className="flex items-center justify-between">
          <span>{t("settings.codeFontFamily")}</span>
          <SettingsSelect
            searchable
            value={settings.code_font_family || ""}
            onChange={(val) => saveSettings({ code_font_family: val })}
            options={[
              { label: t("settings.fontDefault"), value: "" },
              ...systemFonts.map((f) => ({ label: f, value: f })),
            ]}
          />
        </div>
        <Divider style={{ margin: "4px 0" }} />
        <div style={rowStyle} className="flex items-center justify-between">
          <span>{t("settings.codeThemeLight")}</span>
          <SettingsSelect
            searchable
            value={settings.code_theme_light || "github-light"}
            onChange={(val) => saveSettings({ code_theme_light: val })}
            options={lightThemeOptions}
          />
        </div>
        <Divider style={{ margin: "4px 0" }} />
        <div style={rowStyle} className="flex items-center justify-between">
          <span>{t("settings.codeThemeDark")}</span>
          <SettingsSelect
            searchable
            value={settings.code_theme || "poimandres"}
            onChange={(val) => saveSettings({ code_theme: val })}
            options={darkThemeOptions}
          />
        </div>
        <Divider style={{ margin: "4px 0" }} />
        <div style={{ padding: "4px 0" }}>
          <span>{t("settings.borderRadius")}</span>
          <Slider
            min={0}
            max={20}
            value={settings.border_radius}
            onChange={(val) => saveSettings({ border_radius: val })}
            marks={{ 0: "0", 4: "4", 8: "8", 12: "12", 16: "16", 20: "20" }}
          />
        </div>
      </SettingsGroup>
    </div>
  );
}

import { LANG_OPTIONS } from "@/lib/constants";
import { invoke, isTauri } from "@/lib/invoke";
import { useSettingsStore } from "@/stores";
import { open } from "@tauri-apps/plugin-dialog";
import { Button, Divider, Switch, Typography } from "antd";
import { FolderOpen, X } from "lucide-react";
import { useTranslation } from "react-i18next";
import { SettingsGroup } from "./SettingsGroup";
import { SettingsSelect } from "./SettingsSelect";

const { Text } = Typography;

export function GeneralSettings() {
  const { t, i18n } = useTranslation();
  const inTauri = isTauri();
  const settings = useSettingsStore((s) => s.settings);
  const saveSettings = useSettingsStore((s) => s.saveSettings);

  const handleLanguageChange = (language: string) => {
    i18n.changeLanguage(language);
    saveSettings({ language });
  };

  const rowStyle = { padding: "4px 0" };

  return (
    <div className="p-6 pb-12">
      {/* Language */}
      <SettingsGroup title={t("settings.groupLanguage")}>
        <div style={rowStyle} className="flex items-center justify-between">
          <span>{t("settings.language")}</span>
          <SettingsSelect
            value={i18n.language}
            onChange={handleLanguageChange}
            options={LANG_OPTIONS.map((opt) => ({
              label: (
                <span style={{ display: "inline-flex", alignItems: "center", gap: 6 }}>{opt.icon} {opt.label}</span>
              ),
              value: opt.key,
            }))}
          />
        </div>
      </SettingsGroup>

      {/* Startup */}
      <SettingsGroup title={t("settings.groupStartup")}>
        <div style={rowStyle} className="flex items-center justify-between">
          <span>{t("settings.autoStart")}</span>
          <Switch
            checked={settings.auto_start}
            onChange={async (checked) => {
              saveSettings({ auto_start: checked });
              if (inTauri && !import.meta.env.DEV) {
                try {
                  if (checked) {
                    const { enable } = await import("@tauri-apps/plugin-autostart");
                    await enable();
                  } else {
                    const { disable } = await import("@tauri-apps/plugin-autostart");
                    await disable();
                  }
                } catch (e) {
                  console.warn("Autostart toggle failed:", e);
                }
              }
            }}
          />
        </div>
        <Divider style={{ margin: "4px 0" }} />
        <div style={rowStyle} className="flex items-center justify-between">
          <span>{t("settings.showOnStart")}</span>
          <Switch
            checked={settings.show_on_start}
            onChange={(checked) => saveSettings({ show_on_start: checked })}
          />
        </div>
        <Divider style={{ margin: "4px 0" }} />
        <div style={rowStyle} className="flex items-center justify-between">
          <span>{t("desktop.alwaysOnTop")}</span>
          <Switch
            checked={settings.always_on_top ?? false}
            onChange={(checked) => {
              saveSettings({ always_on_top: checked });
              if (inTauri) {
                invoke("set_always_on_top", { enabled: checked }).catch(() => {});
              }
            }}
            disabled={!inTauri}
          />
        </div>
        <Divider style={{ margin: "4px 0" }} />
        <div style={rowStyle} className="flex items-center justify-between">
          <span>{t("desktop.startMinimized")}</span>
          <Switch
            checked={settings.start_minimized ?? false}
            onChange={(checked) => saveSettings({ start_minimized: checked })}
            disabled={!inTauri}
          />
        </div>
      </SettingsGroup>

      {/* Tray & Window */}
      <SettingsGroup title={t("settings.groupTray")}>
        <div style={rowStyle} className="flex items-center justify-between">
          <span>{t("settings.minimizeToTray")}</span>
          <Switch
            checked={settings.minimize_to_tray}
            onChange={(checked) => {
              saveSettings({ minimize_to_tray: checked });
              if (inTauri) {
                invoke("set_close_to_tray", { enabled: checked }).catch(() => {});
              }
            }}
          />
        </div>
      </SettingsGroup>

      {/* Workspace */}
      <SettingsGroup title={t("settings.groupWorkspace")}>
        <div style={rowStyle} className="flex items-center justify-between">
          <span>{t("settings.defaultWorkspaceDir")}</span>
          <div className="flex items-center gap-2">
            {settings.default_workspace_dir
              ? (
                <>
                  <Text type="secondary" ellipsis style={{ maxWidth: 200 }}>
                    {settings.default_workspace_dir}
                  </Text>
                  <Button
                    size="small"
                    icon={<X size={14} />}
                    onClick={() => saveSettings({ default_workspace_dir: null })}
                    disabled={!inTauri}
                  />
                </>
              )
              : (
                <Button
                  size="small"
                  icon={<FolderOpen size={14} />}
                  onClick={async () => {
                    if (!inTauri) { return; }
                    try {
                      const selected = await open({ directory: true, multiple: false });
                      if (selected) {
                        saveSettings({ default_workspace_dir: selected as string });
                      }
                    } catch {
                      // User cancelled or not available
                    }
                  }}
                >
                  {t("common.selectDirectory")}
                </Button>
              )}
          </div>
        </div>
      </SettingsGroup>
    </div>
  );
}

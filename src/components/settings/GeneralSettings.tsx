// SPDX-License-Identifier: AGPL-3.0-only

import { LANG_OPTIONS } from "@/lib/constants";
import { invoke, isTauri, logIpcError } from "@/lib/invoke";
import { useCurrencyStore, useSettingsStore } from "@/stores";
import { useProviderStore, useVoicePreferenceStore } from "@/stores";
import type { TtsVoice } from "@/stores/feature/voicePreferenceStore";
import { TTS_VOICES } from "@/stores/feature/voicePreferenceStore";
import { open } from "@tauri-apps/plugin-dialog";
import { Button, Divider, InputNumber, Switch, Tooltip, Typography } from "antd";
import { FolderOpen, HelpCircle, X } from "lucide-react";
import { useTranslation } from "react-i18next";
import { SettingsGroup } from "./SettingsGroup";
import { SettingsSelect } from "./SettingsSelect";

const { Text } = Typography;

export function GeneralSettings() {
  const { t, i18n } = useTranslation();
  const inTauri = isTauri();
  const settings = useSettingsStore((s) => s.settings);
  const saveSettings = useSettingsStore((s) => s.saveSettings);
  const providers = useProviderStore((s) => s.providers);
  // 货币汇率偏好：后端成本以 USD 计价，前端按此汇率换算为人民币展示
  const usdToCnyRate = useCurrencyStore((s) => s.usdToCnyRate);
  const setUsdToCnyRate = useCurrencyStore((s) => s.setUsdToCnyRate);
  const ttsVoice = useVoicePreferenceStore((s) => s.ttsVoice);
  const sttProviderId = useVoicePreferenceStore((s) => s.sttProviderId);
  const ttsProviderId = useVoicePreferenceStore((s) => s.ttsProviderId);
  const setTtsVoice = useVoicePreferenceStore((s) => s.setTtsVoice);
  const setSttProviderId = useVoicePreferenceStore((s) => s.setSttProviderId);
  const setTtsProviderId = useVoicePreferenceStore((s) => s.setTtsProviderId);

  const handleLanguageChange = (language: string) => {
    i18n.changeLanguage(language);
    saveSettings({ language });
  };

  const rowStyle = { padding: "4px 0" };

  return (
    <div className="p-6 pb-12">
      {/* Language */}
      <SettingsGroup title={t("settings.groupLanguage")}>
        <div style={rowStyle} className="flex items-center justify-between" data-search-key="general:language">
          <span>{t("settings.language")}</span>
          <SettingsSelect
            value={i18n.language}
            onChange={handleLanguageChange}
            options={LANG_OPTIONS.map((opt) => ({
              label: (
                <span
                  style={{
                    display: "inline-flex",
                    alignItems: "center",
                    gap: 6,
                  }}
                >
                  {opt.icon} {opt.label}
                </span>
              ),
              value: opt.key,
            }))}
          />
        </div>
      </SettingsGroup>

      {/* Startup */}
      <SettingsGroup title={t("settings.groupStartup")}>
        <div style={rowStyle} className="flex items-center justify-between" data-search-key="general:autoStart">
          <span>{t("settings.autoStart")}</span>
          <Switch
            id="general-settings-switch-60"
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
                  logIpcError("autostart toggle")(e);
                }
              }
            }}
          />
        </div>
        <Divider style={{ margin: "4px 0" }} />
        <div style={rowStyle} className="flex items-center justify-between" data-search-key="general:showOnStart">
          <span>{t("settings.showOnStart")}</span>
          <Switch
            id="general-settings-switch-61"
            checked={settings.show_on_start}
            onChange={(checked) => saveSettings({ show_on_start: checked })}
          />
        </div>
        <Divider style={{ margin: "4px 0" }} />
        <div style={rowStyle} className="flex items-center justify-between" data-search-key="general:alwaysOnTop">
          <span>{t("desktop.alwaysOnTop")}</span>
          <Switch
            id="general-settings-switch-62"
            checked={settings.always_on_top ?? false}
            onChange={(checked) => {
              saveSettings({ always_on_top: checked });
              if (inTauri) {
                invoke("set_always_on_top", { enabled: checked }).catch(
                  logIpcError("set_always_on_top"),
                );
              }
            }}
            disabled={!inTauri}
          />
        </div>
        <Divider style={{ margin: "4px 0" }} />
        <div style={rowStyle} className="flex items-center justify-between" data-search-key="general:startMinimized">
          <span>{t("desktop.startMinimized")}</span>
          <Switch
            id="general-settings-switch-63"
            checked={settings.start_minimized ?? false}
            onChange={(checked) => saveSettings({ start_minimized: checked })}
            disabled={!inTauri}
          />
        </div>
      </SettingsGroup>

      {/* Tray & Window */}
      <SettingsGroup title={t("settings.groupTray")}>
        <div style={rowStyle} className="flex items-center justify-between" data-search-key="general:minimizeToTray">
          <span>{t("settings.minimizeToTray")}</span>
          <Switch
            id="general-settings-switch-64"
            checked={settings.minimize_to_tray}
            onChange={(checked) => {
              saveSettings({ minimize_to_tray: checked });
              if (inTauri) {
                invoke("set_close_to_tray", { enabled: checked }).catch(
                  logIpcError("set_close_to_tray"),
                );
              }
            }}
          />
        </div>
      </SettingsGroup>

      {/* Workspace */}
      <SettingsGroup title={t("settings.groupWorkspace")}>
        <div
          style={rowStyle}
          className="flex items-center justify-between"
          data-search-key="general:defaultWorkspaceDir"
        >
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
                    if (!inTauri) {
                      return;
                    }
                    try {
                      const selected = await open({
                        directory: true,
                        multiple: false,
                      });
                      if (selected) {
                        saveSettings({
                          default_workspace_dir: selected as string,
                        });
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

      {/* Currency */}
      <SettingsGroup title={t("settings.groupCurrency")}>
        <div style={rowStyle} className="flex items-center justify-between" data-search-key="general:currencyRate">
          <span className="flex items-center gap-1">
            {t("settings.currencyRate")}
            <Tooltip title={t("settings.currencyRateHelp")}>
              <HelpCircle size={13} className="text-zinc-400 cursor-help" />
            </Tooltip>
          </span>
          <InputNumber
            value={usdToCnyRate}
            min={0.01}
            step={0.01}
            precision={4}
            style={{ width: 120 }}
            onChange={(v) => setUsdToCnyRate(Number(v) || 7.2)}
          />
        </div>
      </SettingsGroup>

      {/* Voice Settings */}
      <SettingsGroup title={t("settings.groupVoice")}>
        <div style={rowStyle} className="flex items-center justify-between">
          <span>{t("voice.voiceType")}</span>
          <SettingsSelect
            value={ttsVoice}
            onChange={(v) => setTtsVoice(v as TtsVoice)}
            options={TTS_VOICES.map((v) => ({ label: v, value: v }))}
          />
        </div>
        <Divider style={{ margin: "4px 0" }} />
        <div style={rowStyle} className="flex items-center justify-between">
          <span>{t("voice.sttProvider")}</span>
          <SettingsSelect
            value={sttProviderId}
            onChange={(v) => setSttProviderId(v)}
            options={[
              { label: t("voice.providerAuto"), value: "" },
              ...providers
                .filter((p) => p.enabled)
                .map((p) => ({ label: p.name, value: p.id })),
            ]}
          />
        </div>
        <Divider style={{ margin: "4px 0" }} />
        <div style={rowStyle} className="flex items-center justify-between">
          <span>{t("voice.ttsProvider")}</span>
          <SettingsSelect
            value={ttsProviderId}
            onChange={(v) => setTtsProviderId(v)}
            options={[
              { label: t("voice.providerAuto"), value: "" },
              ...providers
                .filter((p) => p.enabled)
                .map((p) => ({ label: p.name, value: p.id })),
            ]}
          />
        </div>
      </SettingsGroup>
    </div>
  );
}

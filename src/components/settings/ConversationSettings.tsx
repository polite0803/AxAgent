// SPDX-License-Identifier: AGPL-3.0-only

import { useSettingsStore } from "@/stores";
import { Divider, Input, Switch, theme } from "antd";
import { useTranslation } from "react-i18next";
import { SettingsGroup } from "./SettingsGroup";
import { SettingsSelect } from "./SettingsSelect";

export function ConversationSettings() {
  const { t } = useTranslation();
  const settings = useSettingsStore((s) => s.settings);
  const saveSettings = useSettingsStore((s) => s.saveSettings);
  const { token } = theme.useToken();
  const rowStyle = { padding: "4px 0" };

  return (
    <div style={{ padding: 24 }}>
      <SettingsGroup title={t("settings.defaultSystemPrompt")}>
        <div
          style={{
            fontSize: 12,
            color: token.colorTextDescription,
            marginBottom: 12,
          }}
        >
          {t("settings.defaultSystemPromptDesc")}
        </div>
        <Input.TextArea
          value={settings.defaultSystemPrompt ?? ""}
          onChange={(e) => saveSettings({ defaultSystemPrompt: e.target.value || null })}
          placeholder={t("settings.defaultSystemPromptPlaceholder")}
          autoSize={{ minRows: 3, maxRows: 10 }}
        />
      </SettingsGroup>

      <SettingsGroup title={t("settings.groupMessageStyle")}>
        <div className="flex items-center justify-between" style={rowStyle} data-search-key="conversation:bubbleStyle">
          <span>{t("settings.bubbleStyle")}</span>
          <SettingsSelect
            value={settings.bubbleStyle}
            onChange={(val) => saveSettings({ bubbleStyle: val })}
            options={[
              { label: t("settings.bubbleModern"), value: "modern" },
              { label: t("settings.bubbleCompact"), value: "compact" },
              { label: t("settings.bubbleMinimal"), value: "minimal" },
            ]}
          />
        </div>
        <Divider style={{ margin: "4px 0" }} />
        <div
          className="flex items-center justify-between"
          style={rowStyle}
          data-search-key="conversation:renderUserMarkdown"
        >
          <div>
            <div>{t("settings.renderUserMarkdown")}</div>
            <div style={{ fontSize: 12, color: token.colorTextDescription }}>
              {t("settings.renderUserMarkdownDesc")}
            </div>
          </div>
          <Switch
            id="conversation-settings-switch-43"
            checked={settings.renderUserMarkdown ?? false}
            onChange={(checked) => saveSettings({ renderUserMarkdown: checked })}
          />
        </div>
      </SettingsGroup>

      <SettingsGroup title={t("settings.multiModelDisplayMode")}>
        <div
          style={{
            fontSize: 12,
            color: token.colorTextDescription,
            marginBottom: 12,
          }}
        >
          {t("settings.multiModelDisplayModeDesc")}
        </div>
        <div
          className="flex items-center justify-between"
          style={rowStyle}
          data-search-key="conversation:multiModelDisplayMode"
        >
          <span>{t("settings.multiModelDisplayMode")}</span>
          <SettingsSelect
            value={settings.multiModelDisplayMode ?? "tabs"}
            onChange={(val) =>
              saveSettings({
                multiModelDisplayMode: val as
                  | "tabs"
                  | "side-by-side"
                  | "stacked",
              })}
            options={[
              { label: t("settings.multiModelDisplayModeTabs"), value: "tabs" },
              {
                label: t("settings.multiModelDisplayModeSideBySide"),
                value: "side-by-side",
              },
              {
                label: t("settings.multiModelDisplayModeStacked"),
                value: "stacked",
              },
            ]}
          />
        </div>
      </SettingsGroup>

      <SettingsGroup title={t("settings.chatMinimap")}>
        <div
          style={{
            fontSize: 12,
            color: token.colorTextDescription,
            marginBottom: 12,
          }}
        >
          {t("settings.chatMinimapEnabledDesc")}
        </div>
        <div
          className="flex items-center justify-between"
          style={rowStyle}
          data-search-key="conversation:chatMinimapEnabled"
        >
          <span>{t("settings.chatMinimapEnabled")}</span>
          <Switch
            id="conversation-settings-switch-44"
            checked={settings.chatMinimapEnabled ?? false}
            onChange={(checked) => saveSettings({ chatMinimapEnabled: checked })}
          />
        </div>
        {settings.chatMinimapEnabled && (
          <>
            <Divider style={{ margin: "4px 0" }} />
            <div
              className="flex items-center justify-between"
              style={rowStyle}
              data-search-key="conversation:chatMinimapStyle"
            >
              <span>{t("settings.chatMinimapStyle")}</span>
              <SettingsSelect
                value={settings.chatMinimapStyle ?? "faq"}
                onChange={(val) => saveSettings({ chatMinimapStyle: val as "faq" | "sticky" })}
                options={[
                  { label: t("settings.chatMinimapFaq"), value: "faq" },
                  { label: t("settings.chatMinimapSticky"), value: "sticky" },
                ]}
              />
            </div>
          </>
        )}
      </SettingsGroup>
    </div>
  );
}

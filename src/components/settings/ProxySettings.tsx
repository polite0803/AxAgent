// SPDX-License-Identifier: AGPL-3.0-only

import { invoke } from "@/lib/invoke";
import { useSettingsStore } from "@/stores";
import { App, Button, Input, InputNumber } from "antd";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { SettingsGroup } from "./SettingsGroup";
import { SettingsSelect } from "./SettingsSelect";

export function ProxySettings() {
  const { t } = useTranslation();
  const { message } = App.useApp();
  const settings = useSettingsStore((s) => s.settings);
  const saveSettings = useSettingsStore((s) => s.saveSettings);
  const [testing, setTesting] = useState(false);

  const handleTestProxy = async () => {
    const { proxyType: proxy_type, proxyAddress: proxy_address, proxyPort: proxy_port } = settings;

    if (!proxy_address) {
      message.warning(t("settings.proxyAddressRequired"));
      return;
    }

    setTesting(true);
    try {
      const result = await invoke<{
        ok: boolean;
        latency_ms?: number;
        error?: string;
      }>("test_proxy", {
        proxyType: proxy_type || "http",
        proxyAddress: proxy_address,
        proxyPort: proxy_port || 7890,
      });

      if (result.ok) {
        message.success(
          `${t("settings.proxyTestSuccess")}${result.latency_ms ? ` (${result.latency_ms}ms)` : ""}`,
        );
      } else {
        message.error(result.error || t("settings.proxyTestFailed"));
      }
    } catch {
      message.error(t("settings.proxyTestFailed"));
    } finally {
      setTesting(false);
    }
  };

  const rowStyle = { padding: "4px 0" };

  const isSystemProxy = settings.proxyType === "system";
  const needsAddress = !!settings.proxyType && !isSystemProxy;

  return (
    <div className="p-6 pb-12">
      <SettingsGroup title={t("settings.groupProxy")}>
        <div style={rowStyle} className="flex items-center justify-between" data-search-key="proxy:proxyType">
          <span>{t("settings.proxyType")}</span>
          <SettingsSelect
            value={settings.proxyType ?? "none"}
            onChange={(val) => saveSettings({ proxyType: val === "none" ? null : val })}
            options={[
              { label: t("settings.proxyNone"), value: "none" },
              { label: t("settings.proxySystem"), value: "system" },
              { label: t("settings.proxyHttp"), value: "http" },
              { label: t("settings.proxySocks5"), value: "socks5" },
            ]}
          />
        </div>
        <div
          style={{
            height: 1,
            margin: "4px 0",
            backgroundColor: "var(--border-color)",
          }}
        />
        <div style={rowStyle} className="flex items-center justify-between" data-search-key="proxy:proxyAddress">
          <span>{t("settings.proxyAddress")}</span>
          <Input
            id="proxy-settings-input-148"
            value={settings.proxyAddress ?? ""}
            onChange={(e) => saveSettings({ proxyAddress: e.target.value || null })}
            placeholder="127.1.0.0"
            disabled={!needsAddress}
            style={{ width: 280 }}
          />
        </div>
        <div
          style={{
            height: 1,
            margin: "4px 0",
            backgroundColor: "var(--border-color)",
          }}
        />
        <div style={rowStyle} className="flex items-center justify-between" data-search-key="proxy:proxyPort">
          <span>{t("settings.proxyPort")}</span>
          <InputNumber
            id="proxy-settings-inputnumber-149"
            value={settings.proxyPort}
            onChange={(val) => saveSettings({ proxyPort: val ?? null })}
            placeholder="7890"
            disabled={!needsAddress}
            min={1}
            max={65535}
            style={{ width: 150 }}
          />
        </div>
        <div
          style={{
            height: 1,
            margin: "4px 0",
            backgroundColor: "var(--border-color)",
          }}
        />
        <div
          style={{
            padding: "4px 0",
            display: "flex",
            justifyContent: "flex-end",
          }}
        >
          <Button
            onClick={handleTestProxy}
            disabled={!needsAddress}
            loading={testing}
          >
            {t("settings.testProxy")}
          </Button>
        </div>
      </SettingsGroup>
    </div>
  );
}

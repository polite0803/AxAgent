// SPDX-License-Identifier: AGPL-3.0-only

import { PasteButton } from "@/components/common/PasteButton";
import { usePlatformStore } from "@/stores";
import { ALL_PLATFORMS, type PlatformConfig } from "@/types";
import { App, Card, Input, Select, Space, Switch, Typography } from "antd";
import { useRef } from "react";
import { useTranslation } from "react-i18next";

const { Text } = Typography;

type PlatformFieldDef = {
  key: keyof PlatformConfig;
  label: string;
  type: "switch" | "password" | "text" | "number" | "select";
  placeholder?: string;
  options?: { value: string; label: string }[];
};

const PLATFORM_FIELDS: Record<string, PlatformFieldDef[]> = {
  telegram: [
    {
      key: "telegramEnabled",
      label: "settings.platform.enable",
      type: "switch",
    },
    {
      key: "telegramBotToken",
      label: "Bot Token",
      type: "password",
      placeholder: "settings.platform.placeholder.telegramBotToken",
    },
    {
      key: "telegramWebhookUrl",
      label: "Webhook URL (Optional)",
      type: "text",
    },
    {
      key: "telegramWebhookSecret",
      label: "Webhook Secret (Optional)",
      type: "password",
    },
  ],
  discord: [
    {
      key: "discordEnabled",
      label: "settings.platform.enable",
      type: "switch",
    },
    {
      key: "discordBotToken",
      label: "Bot Token",
      type: "password",
      placeholder: "settings.platform.placeholder.discordDevPortal",
    },
    {
      key: "discordWebhookUrl",
      label: "Webhook URL (Optional)",
      type: "text",
    },
  ],
  slack: [
    { key: "slackEnabled", label: "settings.platform.enable", type: "switch" },
    { key: "slackBotToken", label: "Bot Token", type: "password" },
    {
      key: "slackAppToken",
      label: "App Token (Socket Mode)",
      type: "password",
      placeholder: "settings.platform.placeholder.slackAppToken",
    },
    { key: "slackSigningSecret", label: "Signing Secret", type: "password" },
    { key: "slackWorkspaceId", label: "Workspace ID", type: "text" },
  ],
  whatsapp: [
    {
      key: "whatsappEnabled",
      label: "settings.platform.enable",
      type: "switch",
    },
    { key: "whatsappPhoneNumberId", label: "Phone Number ID", type: "text" },
    { key: "whatsappAccessToken", label: "Access Token", type: "password" },
    {
      key: "whatsappBusinessAccountId",
      label: "Business Account ID",
      type: "text",
    },
    {
      key: "whatsappWebhookVerifyToken",
      label: "Webhook Verify Token (Optional)",
      type: "text",
      placeholder: "settings.platform.placeholder.webhookVerify",
    },
    {
      key: "whatsappApiVersion",
      label: "API Version (Optional)",
      type: "text",
      placeholder: "settings.platform.placeholder.apiVersion",
    },
  ],
  wechat: [
    {
      key: "wechatEnabled",
      label: "settings.platform.enable",
      type: "switch",
    },
    {
      key: "wechatMode",
      label: "settings.platform.wechatMode",
      type: "select",
      options: [
        {
          value: "official_account",
          label: "settings.platform.wechatModeOfficial",
        },
        {
          value: "customer_service",
          label: "settings.platform.wechatModeCustomer",
        },
      ],
    },
    { key: "wechatAppId", label: "App ID", type: "text" },
    { key: "wechatAppSecret", label: "App Secret", type: "password" },
    { key: "wechatToken", label: "Token (Official Account)", type: "text" },
    {
      key: "wechatEncodingAesKey",
      label: "Encoding AES Key (Optional)",
      type: "password",
    },
    {
      key: "wechatOriginalId",
      label: "Original ID (Optional)",
      type: "text",
    },
  ],
  feishu: [
    {
      key: "feishuEnabled",
      label: "settings.platform.enable",
      type: "switch",
    },
    { key: "feishuAppId", label: "App ID", type: "text" },
    { key: "feishuAppSecret", label: "App Secret", type: "password" },
    {
      key: "feishuVerificationToken",
      label: "Verification Token (Optional)",
      type: "password",
    },
    {
      key: "feishuEncryptKey",
      label: "Encrypt Key (Optional)",
      type: "password",
    },
  ],
  qq: [
    { key: "qqEnabled", label: "settings.platform.enable", type: "switch" },
    { key: "qqBotAppId", label: "App ID", type: "text" },
    { key: "qqBotToken", label: "Token", type: "password" },
    { key: "qqBotSecret", label: "Secret (Optional)", type: "password" },
  ],
  dingtalk: [
    {
      key: "dingtalkEnabled",
      label: "settings.platform.enable",
      type: "switch",
    },
    { key: "dingtalkAppKey", label: "App Key", type: "text" },
    { key: "dingtalkAppSecret", label: "App Secret", type: "password" },
    {
      key: "dingtalkAgentId",
      label: "Agent ID",
      type: "text",
      placeholder: "settings.platform.placeholder.dingtalkAgent",
    },
    {
      key: "dingtalkRobotCode",
      label: "Robot Code (Optional)",
      type: "text",
    },
  ],
};

export function GatewayConfigPanel() {
  const { t } = useTranslation();
  const config = usePlatformStore((s) => s.config);
  const saveConfig = usePlatformStore((s) => s.saveConfig);
  const debounceTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const pendingRef = useRef<Partial<PlatformConfig>>({});

  const { message } = App.useApp();

  const handleChange = (key: keyof PlatformConfig, value: unknown) => {
    // Immediately update local store state for responsive UI
    usePlatformStore.setState((s) => ({
      config: { ...s.config, [key]: value },
    }));
    // Debounce backend save to avoid excessive API calls on rapid input
    (pendingRef.current as Record<string, unknown>)[key] = value;
    if (debounceTimer.current) {
      clearTimeout(debounceTimer.current);
    }
    debounceTimer.current = setTimeout(async () => {
      try {
        await saveConfig(pendingRef.current);
      } catch (e) {
        message.error(
          t("settings.platform.saveConfigFailed", { error: String(e) }),
        );
      }
      pendingRef.current = {};
    }, 300);
  };

  return (
    <div className="flex flex-col gap-3">
      {ALL_PLATFORMS.map((platform) => {
        if (platform.name === "api_server") {
          return null;
        }
        const fields = PLATFORM_FIELDS[platform.name];
        if (!fields) {
          return null;
        }

        const enabled = config[platform.enabledKey] as boolean;

        return (
          <Card
            key={platform.name}
            size="small"
            title={`${platform.icon} ${platform.label}`}
          >
            {fields.map((field) => {
              if (field.type === "switch") {
                return (
                  <div
                    key={field.key}
                    className="flex items-center justify-between py-1"
                  >
                    <span>{t(field.label)}</span>
                    <Switch
                      id="gateway-config-panel-switch-52"
                      checked={enabled}
                      onChange={(v) => handleChange(field.key, v)}
                    />
                  </div>
                );
              }
              if (!enabled) {
                return null;
              }
              return (
                <div key={field.key} className="mt-3">
                  <Text type="secondary">{t(field.label)}</Text>
                  {field.type === "select"
                    ? (
                      <Select
                        id="gateway-config-panel-select-53"
                        value={(config[field.key] as string) ?? ""}
                        onChange={(v) => handleChange(field.key, v)}
                        options={field.options?.map((o) => ({
                          ...o,
                          label: t(o.label),
                        }))}
                        style={{ width: "100%" }}
                      />
                    )
                    : field.type === "password"
                    ? (
                      <Space.Compact style={{ width: "100%" }}>
                        <Input.Password
                          id="gateway-config-panel-input-password-54"
                          value={(config[field.key] as string) ?? ""}
                          onChange={(e) => handleChange(field.key, e.target.value)}
                          placeholder={field.placeholder ? t(field.placeholder) : undefined}
                        />
                        <PasteButton onPaste={(text) => handleChange(field.key, text)} />
                      </Space.Compact>
                    )
                    : (
                      <Input
                        id="gateway-config-panel-input-55"
                        value={(config[field.key] as string) ?? ""}
                        onChange={(e) => handleChange(field.key, e.target.value)}
                        placeholder={field.placeholder ? t(field.placeholder) : undefined}
                      />
                    )}
                </div>
              );
            })}
          </Card>
        );
      })}

      <Card size="small" title={t("settings.platform.generalSettings")}>
        <div className="flex items-center justify-between py-1">
          <span>{t("settings.platform.enableApiServer")}</span>
          <Switch
            id="gateway-config-panel-switch-56"
            checked={config.apiServerEnabled}
            onChange={(v) => handleChange("apiServerEnabled", v)}
          />
        </div>
        {config.apiServerEnabled && (
          <div className="mt-3">
            <Text type="secondary">{t("settings.platform.apiServerPort")}</Text>
            <Input
              id="gateway-config-panel-input-57"
              type="number"
              value={config.apiServerPort ?? 8080}
              onChange={(e) =>
                handleChange(
                  "apiServerPort",
                  Number.parseInt(e.target.value, 10) || 8080,
                )}
              placeholder="8080"
            />
          </div>
        )}
        <div className="flex items-center justify-between py-1 mt-2">
          <span>{t("settings.platform.autoSyncMessages")}</span>
          <Switch
            id="gateway-config-panel-switch-58"
            checked={config.autoSyncMessages}
            onChange={(v) => handleChange("autoSyncMessages", v)}
          />
        </div>
        <div className="mt-3">
          <Text type="secondary">
            {t("settings.platform.maxHistoryPerSession")}
          </Text>
          <Input
            id="gateway-config-panel-input-59"
            type="number"
            value={config.maxHistoryPerSession}
            onChange={(e) =>
              handleChange(
                "maxHistoryPerSession",
                Number.parseInt(e.target.value, 10) || 100,
              )}
          />
        </div>
      </Card>
    </div>
  );
}

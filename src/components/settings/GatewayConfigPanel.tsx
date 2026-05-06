import { usePlatformStore } from "@/stores";
import { ALL_PLATFORMS, type PlatformConfig } from "@/types/platform";
import { Card, Input, Select, Switch, Typography } from "antd";
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
    { key: "telegram_enabled", label: "settings.platform.enable", type: "switch" },
    {
      key: "telegram_bot_token",
      label: "Bot Token",
      type: "password",
      placeholder: "settings.platform.placeholder.telegramBotToken",
    },
    { key: "telegram_webhook_url", label: "Webhook URL (Optional)", type: "text" },
    { key: "telegram_webhook_secret", label: "Webhook Secret (Optional)", type: "password" },
  ],
  discord: [
    { key: "discord_enabled", label: "settings.platform.enable", type: "switch" },
    {
      key: "discord_bot_token",
      label: "Bot Token",
      type: "password",
      placeholder: "settings.platform.placeholder.discordDevPortal",
    },
    { key: "discord_webhook_url", label: "Webhook URL (Optional)", type: "text" },
  ],
  slack: [
    { key: "slack_enabled", label: "settings.platform.enable", type: "switch" },
    { key: "slack_bot_token", label: "Bot Token", type: "password" },
    {
      key: "slack_app_token",
      label: "App Token (Socket Mode)",
      type: "password",
      placeholder: "settings.platform.placeholder.slackAppToken",
    },
    { key: "slack_signing_secret", label: "Signing Secret", type: "password" },
    { key: "slack_workspace_id", label: "Workspace ID", type: "text" },
  ],
  whatsapp: [
    { key: "whatsapp_enabled", label: "settings.platform.enable", type: "switch" },
    { key: "whatsapp_phone_number_id", label: "Phone Number ID", type: "text" },
    { key: "whatsapp_access_token", label: "Access Token", type: "password" },
    { key: "whatsapp_business_account_id", label: "Business Account ID", type: "text" },
    {
      key: "whatsapp_webhook_verify_token",
      label: "Webhook Verify Token (Optional)",
      type: "text",
      placeholder: "settings.platform.placeholder.webhookVerify",
    },
    {
      key: "whatsapp_api_version",
      label: "API Version (Optional)",
      type: "text",
      placeholder: "settings.platform.placeholder.apiVersion",
    },
  ],
  wechat: [
    { key: "wechat_enabled", label: "settings.platform.enable", type: "switch" },
    {
      key: "wechat_mode",
      label: "settings.platform.wechatMode",
      type: "select",
      options: [
        { value: "official_account", label: "settings.platform.wechatModeOfficial" },
        { value: "customer_service", label: "settings.platform.wechatModeCustomer" },
      ],
    },
    { key: "wechat_app_id", label: "App ID", type: "text" },
    { key: "wechat_app_secret", label: "App Secret", type: "password" },
    { key: "wechat_token", label: "Token (Official Account)", type: "text" },
    { key: "wechat_encoding_aes_key", label: "Encoding AES Key (Optional)", type: "password" },
    { key: "wechat_original_id", label: "Original ID (Optional)", type: "text" },
  ],
  feishu: [
    { key: "feishu_enabled", label: "settings.platform.enable", type: "switch" },
    { key: "feishu_app_id", label: "App ID", type: "text" },
    { key: "feishu_app_secret", label: "App Secret", type: "password" },
    { key: "feishu_verification_token", label: "Verification Token (Optional)", type: "password" },
    { key: "feishu_encrypt_key", label: "Encrypt Key (Optional)", type: "password" },
  ],
  qq: [
    { key: "qq_enabled", label: "settings.platform.enable", type: "switch" },
    { key: "qq_bot_app_id", label: "App ID", type: "text" },
    { key: "qq_bot_token", label: "Token", type: "password" },
    { key: "qq_bot_secret", label: "Secret (Optional)", type: "password" },
  ],
  dingtalk: [
    { key: "dingtalk_enabled", label: "settings.platform.enable", type: "switch" },
    { key: "dingtalk_app_key", label: "App Key", type: "text" },
    { key: "dingtalk_app_secret", label: "App Secret", type: "password" },
    {
      key: "dingtalk_agent_id",
      label: "Agent ID",
      type: "text",
      placeholder: "settings.platform.placeholder.dingtalkAgent",
    },
    { key: "dingtalk_robot_code", label: "Robot Code (Optional)", type: "text" },
  ],
};

export function GatewayConfigPanel() {
  const { t } = useTranslation();
  const config = usePlatformStore((s) => s.config);
  const saveConfig = usePlatformStore((s) => s.saveConfig);
  const debounceTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const pendingRef = useRef<Partial<PlatformConfig>>({});

  const handleChange = (key: keyof PlatformConfig, value: unknown) => {
    // Immediately update local store state for responsive UI
    usePlatformStore.setState((s) => ({ config: { ...s.config, [key]: value } }));
    // Debounce backend save to avoid excessive API calls on rapid input
    (pendingRef.current as Record<string, unknown>)[key] = value;
    if (debounceTimer.current) { clearTimeout(debounceTimer.current); }
    debounceTimer.current = setTimeout(() => {
      saveConfig(pendingRef.current);
      pendingRef.current = {};
    }, 300);
  };

  return (
    <div className="flex flex-col gap-3">
      {ALL_PLATFORMS.map((platform) => {
        if (platform.name === "api_server") { return null; }
        const fields = PLATFORM_FIELDS[platform.name];
        if (!fields) { return null; }

        const enabled = config[platform.enabledKey] as boolean;

        return (
          <Card key={platform.name} size="small" title={`${platform.icon} ${platform.label}`}>
            {fields.map((field) => {
              if (field.type === "switch") {
                return (
                  <div key={field.key} className="flex items-center justify-between py-1">
                    <span>{t(field.label)}</span>
                    <Switch
                      checked={enabled}
                      onChange={(v) => handleChange(field.key, v)}
                    />
                  </div>
                );
              }
              if (!enabled) { return null; }
              return (
                <div key={field.key} className="mt-3">
                  <Text type="secondary">{t(field.label)}</Text>
                  {field.type === "select"
                    ? (
                      <Select
                        value={(config[field.key] as string) ?? ""}
                        onChange={(v) => handleChange(field.key, v)}
                        options={field.options?.map((o) => ({ ...o, label: t(o.label) }))}
                        style={{ width: "100%" }}
                      />
                    )
                    : field.type === "password"
                    ? (
                      <Input.Password
                        value={(config[field.key] as string) ?? ""}
                        onChange={(e) => handleChange(field.key, e.target.value)}
                        placeholder={field.placeholder ? t(field.placeholder) : undefined}
                      />
                    )
                    : (
                      <Input
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
            checked={config.api_server_enabled}
            onChange={(v) => handleChange("api_server_enabled", v)}
          />
        </div>
        {config.api_server_enabled && (
          <div className="mt-3">
            <Text type="secondary">{t("settings.platform.apiServerPort")}</Text>
            <Input
              type="number"
              value={config.api_server_port ?? 8080}
              onChange={(e) => handleChange("api_server_port", Number.parseInt(e.target.value, 10) || 8080)}
              placeholder="8080"
            />
          </div>
        )}
        <div className="flex items-center justify-between py-1 mt-2">
          <span>{t("settings.platform.autoSyncMessages")}</span>
          <Switch
            checked={config.auto_sync_messages}
            onChange={(v) => handleChange("auto_sync_messages", v)}
          />
        </div>
        <div className="mt-3">
          <Text type="secondary">{t("settings.platform.maxHistoryPerSession")}</Text>
          <Input
            type="number"
            value={config.max_history_per_session}
            onChange={(e) => handleChange("max_history_per_session", Number.parseInt(e.target.value, 10) || 100)}
          />
        </div>
      </Card>
    </div>
  );
}

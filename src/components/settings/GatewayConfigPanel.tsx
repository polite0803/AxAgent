import { usePlatformStore } from "@/stores";
import { ALL_PLATFORMS, type PlatformConfig } from "@/types/platform";
import { Card, Input, Switch, Typography } from "antd";

const { Text } = Typography;

type PlatformFieldDef = {
  key: keyof PlatformConfig;
  label: string;
  type: "switch" | "password" | "text" | "number";
  placeholder?: string;
};

const PLATFORM_FIELDS: Record<string, PlatformFieldDef[]> = {
  telegram: [
    { key: "telegram_enabled", label: "启用", type: "switch" },
    { key: "telegram_bot_token", label: "Bot Token", type: "password", placeholder: "从 @BotFather 获取" },
    { key: "telegram_webhook_url", label: "Webhook URL (可选)", type: "text" },
    { key: "telegram_webhook_secret", label: "Webhook Secret (可选)", type: "password" },
  ],
  discord: [
    { key: "discord_enabled", label: "启用", type: "switch" },
    { key: "discord_bot_token", label: "Bot Token", type: "password", placeholder: "从 Discord Developer Portal 获取" },
    { key: "discord_webhook_url", label: "Webhook URL (可选)", type: "text" },
  ],
  slack: [
    { key: "slack_enabled", label: "启用", type: "switch" },
    { key: "slack_bot_token", label: "Bot Token", type: "password" },
    { key: "slack_app_token", label: "App Token (Socket Mode)", type: "password", placeholder: "xapp-... 从 Slack App → App-Level Tokens 获取" },
    { key: "slack_signing_secret", label: "Signing Secret", type: "password" },
    { key: "slack_workspace_id", label: "Workspace ID", type: "text" },
  ],
  whatsapp: [
    { key: "whatsapp_enabled", label: "启用", type: "switch" },
    { key: "whatsapp_phone_number_id", label: "Phone Number ID", type: "text" },
    { key: "whatsapp_access_token", label: "Access Token", type: "password" },
    { key: "whatsapp_business_account_id", label: "Business Account ID", type: "text" },
  ],
  wechat: [
    { key: "wechat_enabled", label: "启用", type: "switch" },
    { key: "wechat_app_id", label: "App ID", type: "text" },
    { key: "wechat_app_secret", label: "App Secret", type: "password" },
    { key: "wechat_token", label: "Token (可选)", type: "text" },
    { key: "wechat_encoding_aes_key", label: "Encoding AES Key (可选)", type: "password" },
    { key: "wechat_original_id", label: "Original ID (可选)", type: "text" },
  ],
  feishu: [
    { key: "feishu_enabled", label: "启用", type: "switch" },
    { key: "feishu_app_id", label: "App ID", type: "text" },
    { key: "feishu_app_secret", label: "App Secret", type: "password" },
    { key: "feishu_verification_token", label: "Verification Token (可选)", type: "password" },
    { key: "feishu_encrypt_key", label: "Encrypt Key (可选)", type: "password" },
  ],
  qq: [
    { key: "qq_enabled", label: "启用", type: "switch" },
    { key: "qq_bot_app_id", label: "App ID", type: "text" },
    { key: "qq_bot_token", label: "Token", type: "password" },
    { key: "qq_bot_secret", label: "Secret (可选)", type: "password" },
  ],
  dingtalk: [
    { key: "dingtalk_enabled", label: "启用", type: "switch" },
    { key: "dingtalk_app_key", label: "App Key", type: "text" },
    { key: "dingtalk_app_secret", label: "App Secret", type: "password" },
    { key: "dingtalk_robot_code", label: "Robot Code (可选)", type: "text" },
  ],
};

export function GatewayConfigPanel() {
  const config = usePlatformStore((s) => s.config);
  const saveConfig = usePlatformStore((s) => s.saveConfig);

  const handleChange = (key: keyof PlatformConfig, value: unknown) => {
    saveConfig({ [key]: value });
  };

  return (
    <div className="flex flex-col gap-3">
      {ALL_PLATFORMS.map((platform) => {
        if (platform.name === "api_server") return null;
        const fields = PLATFORM_FIELDS[platform.name];
        if (!fields) return null;

        const enabled = config[platform.enabledKey] as boolean;

        return (
          <Card key={platform.name} size="small" title={`${platform.icon} ${platform.label}`}>
            {fields.map((field) => {
              if (field.type === "switch") {
                return (
                  <div key={field.key} className="flex items-center justify-between py-1">
                    <span>{field.label}</span>
                    <Switch
                      checked={enabled}
                      onChange={(v) => handleChange(field.key, v)}
                    />
                  </div>
                );
              }
              if (!enabled) return null;
              return (
                <div key={field.key} className="mt-3">
                  <Text type="secondary">{field.label}</Text>
                  {field.type === "password" ? (
                    <Input.Password
                      value={(config[field.key] as string) ?? ""}
                      onChange={(e) => handleChange(field.key, e.target.value)}
                      placeholder={field.placeholder}
                    />
                  ) : (
                    <Input
                      value={(config[field.key] as string) ?? ""}
                      onChange={(e) => handleChange(field.key, e.target.value)}
                      placeholder={field.placeholder}
                    />
                  )}
                </div>
              );
            })}
          </Card>
        );
      })}

      <Card size="small" title="通用设置">
        <div className="flex items-center justify-between py-1">
          <span>启用 API Server</span>
          <Switch
            checked={config.api_server_enabled}
            onChange={(v) => handleChange("api_server_enabled", v)}
          />
        </div>
        <div className="flex items-center justify-between py-1 mt-2">
          <span>自动同步消息</span>
          <Switch
            checked={config.auto_sync_messages}
            onChange={(v) => handleChange("auto_sync_messages", v)}
          />
        </div>
        <div className="mt-3">
          <Text type="secondary">单会话最大历史消息数</Text>
          <Input
            type="number"
            value={config.max_history_per_session}
            onChange={(e) =>
              handleChange("max_history_per_session", Number.parseInt(e.target.value, 10) || 100)
            }
          />
        </div>
      </Card>
    </div>
  );
}

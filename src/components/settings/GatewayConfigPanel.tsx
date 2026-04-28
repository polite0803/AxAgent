import { usePlatformStore } from "@/stores";
import { Button, Input, Switch, Typography, App } from "antd";
import { SettingsGroup } from "./SettingsGroup";

const { Text } = Typography;

export function GatewayConfigPanel() {
  const config = usePlatformStore((s) => s.config);
  const saveConfig = usePlatformStore((s) => s.saveConfig);
  const { message } = App.useApp();

  const handleSave = async () => {
    try {
      await saveConfig(config);
      message.success("Platform configuration saved");
    } catch {
      message.error("Failed to save configuration");
    }
  };

  return (
    <div className="p-6 pb-12">
      <SettingsGroup title="Telegram">
        <div className="flex items-center justify-between">
          <span>Enable Telegram</span>
          <Switch
            checked={config.telegram_enabled}
            onChange={(v) => saveConfig({ telegram_enabled: v })}
          />
        </div>
        {config.telegram_enabled && (
          <>
            <div className="mt-3">
              <Text type="secondary">Bot Token</Text>
              <Input.Password
                value={config.telegram_bot_token ?? ""}
                onChange={(e) => saveConfig({ telegram_bot_token: e.target.value })}
                placeholder="Enter bot token from @BotFather"
              />
            </div>
            <div className="mt-3">
              <Text type="secondary">Webhook URL (optional)</Text>
              <Input
                value={config.telegram_webhook_url ?? ""}
                onChange={(e) => saveConfig({ telegram_webhook_url: e.target.value })}
                placeholder="https://your-domain.com/telegram/webhook"
              />
            </div>
            <div className="mt-3">
              <Text type="secondary">Webhook Secret (optional)</Text>
              <Input.Password
                value={config.telegram_webhook_secret ?? ""}
                onChange={(e) => saveConfig({ telegram_webhook_secret: e.target.value })}
                placeholder="Secret for webhook validation"
              />
            </div>
          </>
        )}
      </SettingsGroup>

      <SettingsGroup title="Discord">
        <div className="flex items-center justify-between">
          <span>Enable Discord</span>
          <Switch
            checked={config.discord_enabled}
            onChange={(v) => saveConfig({ discord_enabled: v })}
          />
        </div>
        {config.discord_enabled && (
          <>
            <div className="mt-3">
              <Text type="secondary">Bot Token</Text>
              <Input.Password
                value={config.discord_bot_token ?? ""}
                onChange={(e) => saveConfig({ discord_bot_token: e.target.value })}
                placeholder="Enter bot token from Discord Developer Portal"
              />
            </div>
            <div className="mt-3">
              <Text type="secondary">Webhook URL (optional)</Text>
              <Input
                value={config.discord_webhook_url ?? ""}
                onChange={(e) => saveConfig({ discord_webhook_url: e.target.value })}
                placeholder="https://discord.com/api/webhooks/..."
              />
            </div>
          </>
        )}
      </SettingsGroup>

      <SettingsGroup title="API Server">
        <div className="flex items-center justify-between">
          <span>Enable REST API Server</span>
          <Switch
            checked={config.api_server_enabled}
            onChange={(v) => saveConfig({ api_server_enabled: v })}
          />
        </div>
        {config.api_server_enabled && (
          <div className="mt-3">
            <Text type="secondary">Port</Text>
            <Input
              type="number"
              value={config.api_server_port ?? 8080}
              onChange={(e) =>
                saveConfig({ api_server_port: Number.parseInt(e.target.value, 10) || 8080 })
              }
              placeholder="8080"
            />
          </div>
        )}
      </SettingsGroup>

      <SettingsGroup title="General">
        <div className="flex items-center justify-between">
          <span>Auto-sync Messages</span>
          <Switch
            checked={config.auto_sync_messages}
            onChange={(v) => saveConfig({ auto_sync_messages: v })}
          />
        </div>
        <div className="mt-3">
          <Text type="secondary">Max History Per Session</Text>
          <Input
            type="number"
            value={config.max_history_per_session}
            onChange={(e) =>
              saveConfig({
                max_history_per_session: Number.parseInt(e.target.value, 10) || 100,
              })
            }
          />
        </div>
      </SettingsGroup>

      <div className="mt-6">
        <Button type="primary" onClick={handleSave}>
          Save Configuration
        </Button>
      </div>
    </div>
  );
}

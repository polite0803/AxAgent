export interface PlatformConfig {
  telegram_enabled: boolean;
  telegram_bot_token: string | null;
  telegram_webhook_url: string | null;
  telegram_webhook_secret: string | null;
  telegram_allowed_users: number[] | null;

  discord_enabled: boolean;
  discord_bot_token: string | null;
  discord_webhook_url: string | null;
  discord_allowed_channels: string[] | null;

  slack_enabled: boolean;
  slack_bot_token: string | null;
  slack_signing_secret: string | null;
  slack_workspace_id: string | null;
  slack_app_token: string | null;

  whatsapp_enabled: boolean;
  whatsapp_phone_number_id: string | null;
  whatsapp_access_token: string | null;
  whatsapp_business_account_id: string | null;
  whatsapp_webhook_verify_token: string | null;
  whatsapp_api_version: string | null;

  wechat_enabled: boolean;
  wechat_app_id: string | null;
  wechat_app_secret: string | null;
  wechat_token: string | null;
  wechat_encoding_aes_key: string | null;
  wechat_original_id: string | null;
  wechat_mode: string | null;

  feishu_enabled: boolean;
  feishu_app_id: string | null;
  feishu_app_secret: string | null;
  feishu_verification_token: string | null;
  feishu_encrypt_key: string | null;

  qq_enabled: boolean;
  qq_bot_app_id: string | null;
  qq_bot_token: string | null;
  qq_bot_secret: string | null;

  dingtalk_enabled: boolean;
  dingtalk_app_key: string | null;
  dingtalk_app_secret: string | null;
  dingtalk_agent_id: string | null;
  dingtalk_robot_code: string | null;

  api_server_enabled: boolean;
  api_server_port: number | null;

  auto_sync_messages: boolean;
  max_history_per_session: number;
}

export interface PlatformMeta {
  name: string;
  label: string;
  icon: string;
  enabledKey: keyof PlatformConfig;
}

export const ALL_PLATFORMS: PlatformMeta[] = [
  { name: "telegram", label: "Telegram", icon: "✈️", enabledKey: "telegram_enabled" },
  { name: "discord", label: "Discord", icon: "💬", enabledKey: "discord_enabled" },
  { name: "slack", label: "Slack", icon: "💼", enabledKey: "slack_enabled" },
  { name: "whatsapp", label: "WhatsApp", icon: "📱", enabledKey: "whatsapp_enabled" },
  { name: "wechat", label: "WeChat", icon: "💚", enabledKey: "wechat_enabled" },
  { name: "feishu", label: "Feishu", icon: "🐦", enabledKey: "feishu_enabled" },
  { name: "qq", label: "QQ", icon: "🐧", enabledKey: "qq_enabled" },
  { name: "dingtalk", label: "DingTalk", icon: "🔷", enabledKey: "dingtalk_enabled" },
  { name: "api_server", label: "API Server", icon: "🔌", enabledKey: "api_server_enabled" },
];

export interface PlatformStatus {
  name: string;
  enabled: boolean;
  connected: boolean;
  last_activity: number | null;
  active_sessions: number;
}

export interface PlatformSession {
  session_id: string;
  platform: string;
  user_id: string;
  username: string | null;
  is_active: boolean;
  last_activity: number;
}

export interface OutgoingMessage {
  platform: string;
  chat_id: string;
  content: string;
  parse_mode: string | null;
}

export interface PlatformReconcileReport {
  started: string[];
  stopped: string[];
  errors: [string, string][];
}

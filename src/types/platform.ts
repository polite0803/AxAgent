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
  api_server_enabled: boolean;
  api_server_port: number | null;
  auto_sync_messages: boolean;
  max_history_per_session: number;
}

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

// SPDX-License-Identifier: AGPL-3.0-only

export interface PlatformConfig {
  telegramEnabled: boolean;
  telegramBotToken: string | null;
  telegramWebhookUrl: string | null;
  telegramWebhookSecret: string | null;
  telegramAllowedUsers: number[] | null;

  discordEnabled: boolean;
  discordBotToken: string | null;
  discordWebhookUrl: string | null;
  discordAllowedChannels: string[] | null;

  slackEnabled: boolean;
  slackBotToken: string | null;
  slackSigningSecret: string | null;
  slackWorkspaceId: string | null;
  slackAppToken: string | null;

  whatsappEnabled: boolean;
  whatsappPhoneNumberId: string | null;
  whatsappAccessToken: string | null;
  whatsappBusinessAccountId: string | null;
  whatsappWebhookVerifyToken: string | null;
  whatsappApiVersion: string | null;

  wechatEnabled: boolean;
  wechatAppId: string | null;
  wechatAppSecret: string | null;
  wechatToken: string | null;
  wechatEncodingAesKey: string | null;
  wechatOriginalId: string | null;
  wechatMode: string | null;

  feishuEnabled: boolean;
  feishuAppId: string | null;
  feishuAppSecret: string | null;
  feishuVerificationToken: string | null;
  feishuEncryptKey: string | null;

  qqEnabled: boolean;
  qqBotAppId: string | null;
  qqBotToken: string | null;
  qqBotSecret: string | null;

  dingtalkEnabled: boolean;
  dingtalkAppKey: string | null;
  dingtalkAppSecret: string | null;
  dingtalkAgentId: string | null;
  dingtalkRobotCode: string | null;

  apiServerEnabled: boolean;
  apiServerPort: number | null;

  autoSyncMessages: boolean;
  maxHistoryPerSession: number;
}

export interface PlatformMeta {
  name: string;
  label: string;
  icon: string;
  enabledKey: keyof PlatformConfig;
}

export const ALL_PLATFORMS: PlatformMeta[] = [
  {
    name: "telegram",
    label: "Telegram",
    icon: "✈️",
    enabledKey: "telegramEnabled",
  },
  {
    name: "discord",
    label: "Discord",
    icon: "💬",
    enabledKey: "discordEnabled",
  },
  { name: "slack", label: "Slack", icon: "💼", enabledKey: "slackEnabled" },
  {
    name: "whatsapp",
    label: "WhatsApp",
    icon: "📱",
    enabledKey: "whatsappEnabled",
  },
  { name: "wechat", label: "WeChat", icon: "💚", enabledKey: "wechatEnabled" },
  { name: "feishu", label: "Feishu", icon: "🐦", enabledKey: "feishuEnabled" },
  { name: "qq", label: "QQ", icon: "🐧", enabledKey: "qqEnabled" },
  {
    name: "dingtalk",
    label: "DingTalk",
    icon: "🔷",
    enabledKey: "dingtalkEnabled",
  },
  {
    name: "api_server",
    label: "API Server",
    icon: "🔌",
    enabledKey: "apiServerEnabled",
  },
];

export interface PlatformStatus {
  name: string;
  enabled: boolean;
  connected: boolean;
  lastActivity: number | null;
  activeSessions: number;
}

export interface PlatformSession {
  sessionId: string;
  platform: string;
  userId: string;
  username: string | null;
  isActive: boolean;
  lastActivity: number;
}

export interface OutgoingMessage {
  platform: string;
  chatId: string;
  content: string;
  parseMode: string | null;
}

export interface PlatformReconcileReport {
  started: string[];
  stopped: string[];
  errors: [string, string][];
}

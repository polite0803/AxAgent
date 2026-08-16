// SPDX-License-Identifier: AGPL-3.0-only

import { invoke } from "@/lib/invoke";
import type { PlatformConfig, PlatformReconcileReport, PlatformSession, PlatformStatus } from "@/types";
import { create } from "zustand";

interface PlatformState {
  config: PlatformConfig;
  statuses: PlatformStatus[];
  sessions: PlatformSession[];
  loading: boolean;
  error: string | null;

  loadConfig: () => Promise<void>;
  saveConfig: (
    config: Partial<PlatformConfig>,
  ) => Promise<PlatformReconcileReport>;
  loadStatuses: () => Promise<void>;
  loadSessions: () => Promise<void>;
  reconcile: () => Promise<PlatformReconcileReport>;
  deactivateSession: (sessionId: string) => Promise<void>;
  sendMessage: (
    platform: string,
    chatId: string,
    text: string,
  ) => Promise<void>;
}

const defaultConfig: PlatformConfig = {
  telegramEnabled: false,
  telegramBotToken: null,
  telegramWebhookUrl: null,
  telegramWebhookSecret: null,
  telegramAllowedUsers: null,
  discordEnabled: false,
  discordBotToken: null,
  discordWebhookUrl: null,
  discordAllowedChannels: null,
  slackEnabled: false,
  slackBotToken: null,
  slackSigningSecret: null,
  slackWorkspaceId: null,
  slackAppToken: null,
  whatsappEnabled: false,
  whatsappPhoneNumberId: null,
  whatsappAccessToken: null,
  whatsappBusinessAccountId: null,
  whatsappWebhookVerifyToken: null,
  whatsappApiVersion: null,
  wechatEnabled: false,
  wechatAppId: null,
  wechatAppSecret: null,
  wechatToken: null,
  wechatEncodingAesKey: null,
  wechatOriginalId: null,
  wechatMode: null,
  feishuEnabled: false,
  feishuAppId: null,
  feishuAppSecret: null,
  feishuVerificationToken: null,
  feishuEncryptKey: null,
  qqEnabled: false,
  qqBotAppId: null,
  qqBotToken: null,
  qqBotSecret: null,
  dingtalkEnabled: false,
  dingtalkAppKey: null,
  dingtalkAppSecret: null,
  dingtalkAgentId: null,
  dingtalkRobotCode: null,
  apiServerEnabled: false,
  apiServerPort: null,
  autoSyncMessages: true,
  maxHistoryPerSession: 100,
};

export const usePlatformStore = create<PlatformState>((set, get) => ({
  config: defaultConfig,
  statuses: [],
  sessions: [],
  loading: false,
  error: null,

  loadConfig: async () => {
    set({ loading: true });
    try {
      const config = await invoke<PlatformConfig>("get_platform_config");
      set({ config, loading: false, error: null });
    } catch (e) {
      set({ error: String(e), loading: false });
    }
  },

  saveConfig: async (partial: Partial<PlatformConfig>) => {
    set({ loading: true });
    try {
      const current = await invoke<PlatformConfig>("get_platform_config");
      const merged: PlatformConfig = { ...current, ...partial };
      const report = await invoke<PlatformReconcileReport>(
        "update_platform_config",
        {
          config: merged,
        },
      );
      set({ config: merged, loading: false, error: null });
      return report;
    } catch (e) {
      set({ error: String(e), loading: false });
      throw e;
    }
  },

  loadStatuses: async () => {
    try {
      const statuses = await invoke<PlatformStatus[]>("get_platform_statuses");
      set({ statuses, error: null });
    } catch (e) {
      set({ error: String(e) });
    }
  },

  loadSessions: async () => {
    try {
      const sessions = await invoke<PlatformSession[]>("get_active_sessions");
      set({ sessions, error: null });
    } catch (e) {
      set({ error: String(e) });
    }
  },

  reconcile: async () => {
    try {
      const report = await invoke<PlatformReconcileReport>(
        "reconcile_platforms",
      );
      await get().loadStatuses();
      return report;
    } catch (e) {
      set({ error: String(e) });
      throw e;
    }
  },

  deactivateSession: async (sessionId: string) => {
    try {
      await invoke("deactivate_platform_session", { session_id: sessionId });
      set((s) => ({
        sessions: s.sessions.map((ses) => ses.sessionId === sessionId ? { ...ses, isActive: false } : ses),
      }));
    } catch (e) {
      set({ error: String(e) });
    }
  },

  sendMessage: async (platform: string, chatId: string, text: string) => {
    await invoke("send_platform_message", { platform, chat_id: chatId, text });
  },

  createSession: async (platform: string, chatId: string) => {
    const session = await invoke<PlatformSession>("create_platform_session", {
      platform,
      chatId,
    });
    set((s) => ({ sessions: [...s.sessions, session] }));
    return session;
  },

  processMessage: async (platform: string, payload: unknown) => {
    const cmdMap: Record<string, string> = {
      telegram: "process_telegram_message",
      discord: "process_discord_message",
      wechat: "process_wechat_message",
      feishu: "process_feishu_message",
      qq: "process_qq_message",
      dingtalk: "process_dingtalk_message",
      slack: "process_slack_message",
      whatsapp: "process_whatsapp_message",
    };
    const cmd = cmdMap[platform];
    if (cmd) {
      return await invoke(cmd, { payload });
    }
    return await invoke("process_platform_message", { platform, payload });
  },
}));

import { invoke } from "@/lib/invoke";
import type { PlatformConfig, PlatformReconcileReport, PlatformSession, PlatformStatus } from "@/types/platform";
import { create } from "zustand";

interface PlatformState {
  config: PlatformConfig;
  statuses: PlatformStatus[];
  sessions: PlatformSession[];
  loading: boolean;
  error: string | null;

  loadConfig: () => Promise<void>;
  saveConfig: (config: Partial<PlatformConfig>) => Promise<PlatformReconcileReport>;
  loadStatuses: () => Promise<void>;
  loadSessions: () => Promise<void>;
  reconcile: () => Promise<PlatformReconcileReport>;
  deactivateSession: (sessionId: string) => Promise<void>;
  sendMessage: (platform: string, chatId: string, text: string) => Promise<void>;
}

const defaultConfig: PlatformConfig = {
  telegram_enabled: false,
  telegram_bot_token: null,
  telegram_webhook_url: null,
  telegram_webhook_secret: null,
  telegram_allowed_users: null,
  discord_enabled: false,
  discord_bot_token: null,
  discord_webhook_url: null,
  discord_allowed_channels: null,
  slack_enabled: false,
  slack_bot_token: null,
  slack_signing_secret: null,
  slack_workspace_id: null,
  slack_app_token: null,
  whatsapp_enabled: false,
  whatsapp_phone_number_id: null,
  whatsapp_access_token: null,
  whatsapp_business_account_id: null,
  whatsapp_webhook_verify_token: null,
  whatsapp_api_version: null,
  wechat_enabled: false,
  wechat_app_id: null,
  wechat_app_secret: null,
  wechat_token: null,
  wechat_encoding_aes_key: null,
  wechat_original_id: null,
  wechat_mode: null,
  feishu_enabled: false,
  feishu_app_id: null,
  feishu_app_secret: null,
  feishu_verification_token: null,
  feishu_encrypt_key: null,
  qq_enabled: false,
  qq_bot_app_id: null,
  qq_bot_token: null,
  qq_bot_secret: null,
  dingtalk_enabled: false,
  dingtalk_app_key: null,
  dingtalk_app_secret: null,
  dingtalk_agent_id: null,
  dingtalk_robot_code: null,
  api_server_enabled: false,
  api_server_port: null,
  auto_sync_messages: true,
  max_history_per_session: 100,
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
      const report = await invoke<PlatformReconcileReport>("update_platform_config", {
        config: merged,
      });
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
      const report = await invoke<PlatformReconcileReport>("reconcile_platforms");
      await get().loadStatuses();
      return report;
    } catch (e) {
      set({ error: String(e) });
      throw e;
    }
  },

  deactivateSession: async (sessionId: string) => {
    try {
      await invoke("deactivate_platform_session", { sessionId });
      set((s) => ({
        sessions: s.sessions.map((ses) =>
          ses.session_id === sessionId ? { ...ses, is_active: false } : ses
        ),
      }));
    } catch (e) {
      set({ error: String(e) });
    }
  },

  sendMessage: async (platform: string, chatId: string, text: string) => {
    await invoke("send_platform_message", { platform, chatId, text });
  },

  createSession: async (platform: string, chatId: string) => {
    const session = await invoke<PlatformSession>("create_platform_session", { platform, chatId });
    set((s) => ({ sessions: [...s.sessions, session] }));
    return session;
  },

  processMessage: async (platform: string, payload: unknown) => {
    const cmd = platform === "telegram" ? "process_telegram_message" : "process_discord_message";
    return await invoke(cmd, { payload });
  },
}));

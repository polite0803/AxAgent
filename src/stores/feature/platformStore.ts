import { invoke } from "@/lib/invoke";
import type { PlatformConfig, PlatformSession, PlatformStatus } from "@/types/platform";
import { create } from "zustand";

interface PlatformState {
  config: PlatformConfig;
  statuses: PlatformStatus[];
  sessions: PlatformSession[];
  loading: boolean;
  error: string | null;

  loadConfig: () => Promise<void>;
  saveConfig: (config: Partial<PlatformConfig>) => Promise<void>;
  loadStatuses: () => Promise<void>;
  loadSessions: () => Promise<void>;
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
  api_server_enabled: false,
  api_server_port: null,
  auto_sync_messages: true,
  max_history_per_session: 100,
};

export const usePlatformStore = create<PlatformState>((set) => ({
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
      await invoke("update_platform_config", { config: merged });
      set({ config: merged, loading: false, error: null });
    } catch (e) {
      set({ error: String(e), loading: false });
      throw e;
    }
  },

  loadStatuses: async () => {
    try {
      const config = await invoke<PlatformConfig>("get_platform_config");
      const statuses: PlatformStatus[] = [
        {
          name: "telegram",
          enabled: config.telegram_enabled,
          connected: false,
          last_activity: null,
          active_sessions: 0,
        },
        {
          name: "discord",
          enabled: config.discord_enabled,
          connected: false,
          last_activity: null,
          active_sessions: 0,
        },
        {
          name: "api_server",
          enabled: config.api_server_enabled,
          connected: false,
          last_activity: null,
          active_sessions: 0,
        },
      ];
      set({ statuses });
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
    if (platform === "telegram") {
      const chatIdNum = Number.parseInt(chatId, 10);
      if (Number.isNaN(chatIdNum)) throw new Error("Invalid chat ID");
      await invoke("send_telegram_message", { chatId: chatIdNum, text });
    } else if (platform === "discord") {
      await invoke("send_discord_message", { content: text });
    } else {
      throw new Error(`Unknown platform: ${platform}`);
    }
  },
}));

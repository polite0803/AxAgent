// SPDX-License-Identifier: AGPL-3.0-only

import { invoke } from "@/lib/invoke";
import { create } from "zustand";

export type TTSChannel = "commentary" | "final";

export interface TTSMessage {
  id: string;
  channel: TTSChannel;
  text: string;
  timestamp: number;
  duration_ms?: number;
  status: "queued" | "speaking" | "completed" | "cancelled";
}

interface TTSChannelStore {
  messages: TTSMessage[];
  isSpeaking: boolean;
  activeChannel: TTSChannel | null;
  enabled: boolean;
  error: string | null;

  setEnabled: (enabled: boolean) => void;
  enqueueMessage: (channel: TTSChannel, text: string) => Promise<void>;
  speak: (text: string, channel?: TTSChannel) => Promise<void>;
  cancelAll: () => void;
  clearMessages: () => void;
  handleProgressBrief: (brief: {
    brief_type: string;
    description: string;
  }) => void;
}

/**
 * Web Speech API 降级播报（Tauri WebView2 / 浏览器均支持）。
 * 后端 tts_speak 命令未注册时使用；final 通道会打断 commentary 播报（双通道语义）。
 */
function speakViaWebSpeech(text: string, channel: TTSChannel): Promise<void> {
  return new Promise((resolve) => {
    if (typeof window === "undefined" || !("speechSynthesis" in window)) {
      resolve();
      return;
    }
    const synth = window.speechSynthesis;
    if (channel === "final") {
      synth.cancel();
    }
    const utterance = new SpeechSynthesisUtterance(text);
    utterance.lang = "zh-CN";
    utterance.rate = 1.05;
    const zhVoice = synth
      .getVoices()
      .find((v) => v.lang.toLowerCase().startsWith("zh"));
    if (zhVoice) {
      utterance.voice = zhVoice;
    }
    utterance.onend = () => resolve();
    utterance.onerror = () => resolve();
    synth.speak(utterance);
  });
}

export const useTTSChannelStore = create<TTSChannelStore>((set, get) => ({
  messages: [],
  isSpeaking: false,
  activeChannel: null,
  enabled: true,
  error: null,

  setEnabled: (enabled) => {
    if (!enabled) {
      // 关闭时打断正在进行的播报
      if (typeof window !== "undefined" && "speechSynthesis" in window) {
        window.speechSynthesis.cancel();
      }
    }
    set({ enabled });
  },

  enqueueMessage: async (channel, text) => {
    const msg: TTSMessage = {
      id: `tts-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`,
      channel,
      text,
      timestamp: Date.now(),
      status: "queued",
    };
    set((s) => ({ messages: [...s.messages, msg] }));

    // 标记为播报中
    set((s) => ({
      messages: s.messages.map((m) => m.id === msg.id ? { ...m, status: "speaking" as const } : m),
      isSpeaking: true,
      activeChannel: channel,
    }));

    try {
      // 1) 优先后端 tts_speak（注册后走系统 TTS）
      await invoke("tts_speak", { text, channel });
    } catch {
      // 2) 后端未注册 → Web Speech API 降级
      try {
        await speakViaWebSpeech(text, channel);
      } catch {
        // 3) 环境完全不可用：静默完成，不卡队列
      }
    }

    set((s) => {
      const stillSpeaking = s.messages.some(
        (m) => m.id !== msg.id && m.status === "speaking",
      );
      return {
        messages: s.messages.map((m) => m.id === msg.id ? { ...m, status: "completed" as const } : m),
        isSpeaking: stillSpeaking,
        activeChannel: stillSpeaking ? s.activeChannel : null,
      };
    });
  },

  speak: async (text, channel = "commentary") => {
    if (!get().enabled) { return; }
    await get().enqueueMessage(channel, text);
  },

  cancelAll: () => {
    if (typeof window !== "undefined" && "speechSynthesis" in window) {
      window.speechSynthesis.cancel();
    }
    set((s) => ({
      messages: s.messages.map((m) =>
        m.status === "speaking" || m.status === "queued"
          ? { ...m, status: "cancelled" as const }
          : m
      ),
      isSpeaking: false,
      activeChannel: null,
    }));
  },

  clearMessages: () => set({ messages: [] }),

  handleProgressBrief: (brief) => {
    const { enabled } = get();
    if (!enabled) { return; }

    if (brief.brief_type === "workflow_complete") {
      void get().enqueueMessage("final", brief.description);
    } else {
      void get().enqueueMessage("commentary", brief.description);
    }
  },
}));

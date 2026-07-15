// SPDX-License-Identifier: AGPL-3.0-only

import { create } from "zustand";
import { persist } from "zustand/middleware";

export type TtsVoice = "alloy" | "echo" | "fable" | "onyx" | "nova" | "shimmer";

export const TTS_VOICES: TtsVoice[] = [
  "alloy",
  "echo",
  "fable",
  "onyx",
  "nova",
  "shimmer",
];

interface VoicePreferenceState {
  /** TTS 音色（OpenAI 标准），null 表示服务端默认 */
  ttsVoice: TtsVoice;
  /** STT 提供商 ID（空 = 使用 LLM 同一家） */
  sttProviderId: string;
  /** TTS 提供商 ID（空 = 使用 LLM 同一家） */
  ttsProviderId: string;

  setTtsVoice: (voice: TtsVoice) => void;
  setSttProviderId: (id: string) => void;
  setTtsProviderId: (id: string) => void;
}

export const useVoicePreferenceStore = create<VoicePreferenceState>()(
  persist(
    (set) => ({
      ttsVoice: "nova",
      sttProviderId: "",
      ttsProviderId: "",

      setTtsVoice: (ttsVoice) => set({ ttsVoice }),
      setSttProviderId: (sttProviderId) => set({ sttProviderId }),
      setTtsProviderId: (ttsProviderId) => set({ ttsProviderId }),
    }),
    {
      name: "axagent-voice-preference",
    },
  ),
);

// SPDX-License-Identifier: AGPL-3.0-only

// 新用户引导状态管理
import { invoke } from "@/lib/invoke";
import { useSettingsStore } from "@/stores/feature/settingsStore";
import { create } from "zustand";

interface DetectedKey {
  providerType: string;
  prefix: string;
  envVar: string;
}

interface OllamaModelInfo {
  name: string;
  sizeMb?: number;
  family?: string;
}

interface OnboardingStore {
  wizardCompleted: boolean;
  wizardDismissed: boolean;
  currentStep: number;
  ollamaAvailable: boolean;
  ollamaModels: OllamaModelInfo[];
  detectedKeys: DetectedKey[];
  selectedPreset: string | null;
  tutorialCompleted: boolean;
  tutorialActive: boolean;
  tutorialStep: number;

  detectOllama: () => Promise<void>;
  detectKeys: () => Promise<void>;
  applyPreset: (preset: string) => Promise<string>;
  setStep: (step: number) => void;
  dismissWizard: () => void;
  completeWizard: () => Promise<void>;
  startTutorial: () => void;
  nextTutorialStep: () => void;
  skipTutorial: () => Promise<void>;
  completeTutorial: () => Promise<void>;
  loadFromSettings: () => void;
}

export const useOnboardingStore = create<OnboardingStore>((set, get) => ({
  wizardCompleted: false,
  wizardDismissed: false,
  currentStep: 0,
  ollamaAvailable: false,
  ollamaModels: [],
  detectedKeys: [],
  selectedPreset: null,
  tutorialCompleted: false,
  tutorialActive: false,
  tutorialStep: 0,

  detectOllama: async () => {
    try {
      const result = await invoke<{
        available: boolean;
        models: OllamaModelInfo[];
        error?: string;
      }>("detect_ollama_availability");
      set({
        ollamaAvailable: result.available,
        ollamaModels: result.models,
      });
    } catch {
      set({ ollamaAvailable: false });
    }
  },

  detectKeys: async () => {
    try {
      const result = await invoke<DetectedKey[]>("detect_api_keys");
      set({ detectedKeys: result });
    } catch {
      set({ detectedKeys: [] });
    }
  },

  applyPreset: async (preset: string) => {
    set({ selectedPreset: preset });
    try {
      const result = await invoke<{ success: boolean; message: string }>(
        "apply_quick_start_preset",
        { preset },
      );
      return result.message;
    } catch (e) {
      return `预设应用失败: ${e}`;
    }
  },

  setStep: (step) => set({ currentStep: step }),

  dismissWizard: () => {
    set({ wizardDismissed: true });
    const s = useSettingsStore.getState();
    s.saveSettings({ onboarding_wizard_dismissed: true });
  },

  completeWizard: async () => {
    set({ wizardCompleted: true });
    const s = useSettingsStore.getState();
    await s.saveSettings({
      onboarding_completed: true,
      onboarding_selected_preset: get().selectedPreset,
    });
  },

  startTutorial: () => set({ tutorialActive: true, tutorialStep: 0 }),

  nextTutorialStep: () => set((s) => ({ tutorialStep: s.tutorialStep + 1 })),

  skipTutorial: async () => {
    set({ tutorialActive: false, tutorialCompleted: true });
    await useSettingsStore
      .getState()
      .saveSettings({ onboarding_tutorial_completed: true });
  },

  completeTutorial: async () => {
    set({ tutorialActive: false, tutorialCompleted: true });
    await useSettingsStore
      .getState()
      .saveSettings({ onboarding_tutorial_completed: true });
  },

  loadFromSettings: () => {
    const s = useSettingsStore.getState().settings;
    set({
      wizardCompleted: s.onboarding_completed ?? false,
      wizardDismissed: s.onboarding_wizard_dismissed ?? false,
      tutorialCompleted: s.onboarding_tutorial_completed ?? false,
      selectedPreset: s.onboarding_selected_preset ?? null,
    });
  },
}));

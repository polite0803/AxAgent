import { invoke } from "@/lib/invoke";
import type {
  ProactiveSuggestion,
  ContextPrediction,
  Reminder,
  ProactiveConfig,
  PredictionResult,
  PrefetchResults,
} from "@/types/proactive";
import { create } from "zustand";

interface ProactiveState {
  suggestions: ProactiveSuggestion[];
  predictions: ContextPrediction[];
  reminders: Reminder[];
  config: ProactiveConfig | null;
  isEnabled: boolean;
  isLoading: boolean;
  error: string | null;

  fetchSuggestions: () => Promise<void>;
  fetchPredictions: (context: Record<string, unknown>) => Promise<void>;
  fetchReminders: () => Promise<void>;
  dismissSuggestion: (id: string) => Promise<void>;
  acceptSuggestion: (id: string) => Promise<void>;
  snoozeSuggestion: (id: string, durationMinutes: number) => Promise<void>;
  addReminder: (reminder: ReminderInput) => Promise<void>;
  removeReminder: (id: string) => Promise<void>;
  completeReminder: (id: string) => Promise<void>;
  setEnabled: (enabled: boolean) => Promise<void>;
  updateConfig: (config: Partial<ProactiveConfig>) => Promise<void>;
  prefetch: (predictions: ContextPrediction[]) => Promise<PrefetchResults>;
  clearAll: () => void;
}

export interface ReminderInput {
  title: string;
  description: string;
  scheduled_at: string;
  recurrence?: {
    frequency: "daily" | "weekly" | "monthly";
    interval: number;
  };
}

export const useProactiveStore = create<ProactiveState>((set, get) => ({
  suggestions: [],
  predictions: [],
  reminders: [],
  config: null,
  isEnabled: true,
  isLoading: false,
  error: null,

  fetchSuggestions: async () => {
    set({ isLoading: true, error: null });
    try {
      const suggestions = await invoke<ProactiveSuggestion[]>("proactive_list_suggestions");
      set({ suggestions, isLoading: false });
    } catch (error) {
      set({
        error: error instanceof Error ? error.message : "Failed to fetch suggestions",
        isLoading: false,
      });
    }
  },

  fetchPredictions: async (context: Record<string, unknown>) => {
    set({ isLoading: true, error: null });
    try {
      const result = await invoke<PredictionResult>("proactive_predict", { context });
      set({
        predictions: result.predictions,
        isLoading: false,
      });
    } catch (error) {
      set({
        error: error instanceof Error ? error.message : "Failed to fetch predictions",
        isLoading: false,
      });
    }
  },

  fetchReminders: async () => {
    set({ isLoading: true, error: null });
    try {
      const reminders = await invoke<Reminder[]>("proactive_list_reminders");
      set({ reminders, isLoading: false });
    } catch (error) {
      set({
        error: error instanceof Error ? error.message : "Failed to fetch reminders",
        isLoading: false,
      });
    }
  },

  dismissSuggestion: async (id: string) => {
    try {
      await invoke("proactive_dismiss_suggestion", { id });
      set((state) => ({
        suggestions: state.suggestions.filter((s) => s.id !== id),
      }));
    } catch (error) {
      set({
        error: error instanceof Error ? error.message : "Failed to dismiss suggestion",
      });
    }
  },

  acceptSuggestion: async (id: string) => {
    try {
      await invoke("proactive_accept_suggestion", { id });
      set((state) => ({
        suggestions: state.suggestions.map((s) =>
          s.id === id ? { ...s, accepted: true } : s
        ),
      }));
    } catch (error) {
      set({
        error: error instanceof Error ? error.message : "Failed to accept suggestion",
      });
    }
  },

  snoozeSuggestion: async (id: string, durationMinutes: number) => {
    try {
      await invoke("proactive_snooze_suggestion", { id, duration: durationMinutes });
      set((state) => ({
        suggestions: state.suggestions.filter((s) => s.id !== id),
      }));
    } catch (error) {
      set({
        error: error instanceof Error ? error.message : "Failed to snooze suggestion",
      });
    }
  },

  addReminder: async (input: ReminderInput) => {
    set({ isLoading: true, error: null });
    try {
      const reminder = await invoke<Reminder>("proactive_add_reminder", { reminder: input });
      set((state) => ({
        reminders: [...state.reminders, reminder],
        isLoading: false,
      }));
    } catch (error) {
      set({
        error: error instanceof Error ? error.message : "Failed to add reminder",
        isLoading: false,
      });
    }
  },

  removeReminder: async (id: string) => {
    try {
      await invoke("proactive_delete_reminder", { id });
      set((state) => ({
        reminders: state.reminders.filter((r) => r.id !== id),
      }));
    } catch (error) {
      set({
        error: error instanceof Error ? error.message : "Failed to remove reminder",
      });
    }
  },

  completeReminder: async (id: string) => {
    try {
      await invoke("proactive_complete_reminder", { id });
      set((state) => ({
        reminders: state.reminders.map((r) =>
          r.id === id ? { ...r, completed: true } : r
        ),
      }));
    } catch (error) {
      set({
        error: error instanceof Error ? error.message : "Failed to complete reminder",
      });
    }
  },

  setEnabled: async (enabled: boolean) => {
    try {
      await invoke("proactive_set_enabled", { enabled });
      set({ isEnabled: enabled });
    } catch (error) {
      set({
        error: error instanceof Error ? error.message : "Failed to set enabled state",
      });
    }
  },

  updateConfig: async (configUpdate: Partial<ProactiveConfig>) => {
    try {
      const defaultConfig: ProactiveConfig = {
        enabled: true,
        max_suggestions: 10,
        suggestion_ttl_minutes: 60,
        prediction_confidence_threshold: 0.5,
        prefetch_enabled: true,
        reminder_enabled: true,
      };
      const currentConfig = get().config || defaultConfig;
      const newConfig = { ...currentConfig, ...configUpdate };
      await invoke("proactive_update_config", { config: newConfig });
      set({ config: newConfig });
    } catch (error) {
      set({
        error: error instanceof Error ? error.message : "Failed to update config",
      });
    }
  },

  prefetch: async (predictions: ContextPrediction[]) => {
    try {
      const results = await invoke<PrefetchResults>("proactive_prefetch", {
        predictions,
      });
      return results;
    } catch (error) {
      set({
        error: error instanceof Error ? error.message : "Failed to prefetch",
      });
      return { results: [], total_estimated_time_ms: 0, critical_path: [] };
    }
  },

  clearAll: () => {
    set({
      suggestions: [],
      predictions: [],
      reminders: [],
      error: null,
    });
  },
}));

import { invoke } from "@/lib/invoke";
import type {
  ContextPrediction,
  PredictionResult,
  PrefetchResults,
  ProactiveConfig,
  ProactiveSuggestion,
  Reminder,
} from "@/types/proactive";
import { create } from "zustand";

// ─── Prefetch state: tracks what's been prefetched to avoid duplicates ───

interface PrefetchState {
  /** Set of already-prefetched resource IDs to prevent redundant work */
  prefetchedIds: Set<string>;
  /** Timestamps of last prefetch per type */
  lastPrefetchTime: Record<string, number>;
}

const _prefetchState: PrefetchState = {
  prefetchedIds: new Set(),
  lastPrefetchTime: {},
};

/** Minimum interval between prefetches of the same type (ms) */
const PREFETCH_COOLDOWN_MS = 30_000; // 30 seconds
/** Maximum prefetched items to track */
const MAX_PREFETCH_IDS = 200;

// ─── Input intent prediction (local, no backend call) ───

interface IntentPrediction {
  intent: string;
  confidence: number;
}

/** Lightweight local intent prediction from user typing patterns.
 *  Used to trigger prefetching before the user hits send. */
function predictIntentFromInput(text: string): IntentPrediction[] {
  if (!text || text.length < 3) { return []; }
  const lower = text.toLowerCase();
  const intents: IntentPrediction[] = [];

  // Code generation
  if (
    lower.includes("write") || lower.includes("create") || lower.includes("build")
    || lower.includes("写") || lower.includes("创建") || lower.includes("生成")
    || lower.startsWith("```")
  ) {
    intents.push({ intent: "codeGeneration", confidence: 0.85 });
  }

  // Search / research
  if (
    lower.includes("search") || lower.includes("find") || lower.includes("look up")
    || lower.includes("搜索") || lower.includes("查找") || lower.includes("what is")
    || lower.includes("how to") || lower.includes("什么是") || lower.includes("怎么")
  ) {
    intents.push({ intent: "search", confidence: 0.8 });
  }

  // Refactoring
  if (
    lower.includes("refactor") || lower.includes("optimize") || lower.includes("improve")
    || lower.includes("重构") || lower.includes("优化") || lower.includes("改进")
  ) {
    intents.push({ intent: "refactoring", confidence: 0.75 });
  }

  // Translation
  if (
    lower.includes("translate") || lower.includes("翻译") || lower.includes("译为")
  ) {
    intents.push({ intent: "translation", confidence: 0.9 });
  }

  // Debug
  if (
    lower.includes("debug") || lower.includes("fix") || lower.includes("error")
    || lower.includes("broken") || lower.includes("not working")
    || lower.includes("修复") || lower.includes("错误") || lower.includes("bug")
  ) {
    intents.push({ intent: "debug", confidence: 0.8 });
  }

  return intents;
}

/** Check if a prefetch type has been recently prefetched (cooldown). */
function canPrefetchNow(type: string): boolean {
  const last = _prefetchState.lastPrefetchTime[type] || 0;
  return Date.now() - last > PREFETCH_COOLDOWN_MS;
}

/** Mark a prefetch type as done. */
function markPrefetched(type: string, ids: string[]) {
  _prefetchState.lastPrefetchTime[type] = Date.now();
  for (const id of ids) {
    if (_prefetchState.prefetchedIds.size >= MAX_PREFETCH_IDS) {
      // Evict oldest (simple FIFO via clear-and-rebuild)
      const entries = [..._prefetchState.prefetchedIds].slice(-MAX_PREFETCH_IDS / 2);
      _prefetchState.prefetchedIds = new Set(entries);
    }
    _prefetchState.prefetchedIds.add(id);
  }
}

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

  // ── P2 Smart Prefetch ──

  /** Triggered on conversation switch — prefetch context, token counts */
  prefetchOnConversationSwitch: (conversationId: string) => void;

  /** Prefetch model cost and capability info when model selector opens */
  prefetchModelCosts: (providerId: string, modelId: string) => void;

  /** Analyze user input and trigger prefetch based on predicted intent */
  predictAndPrefetch: (inputText: string) => IntentPrediction[];

  /** Clear the internal prefetch dedup state */
  resetPrefetchState: () => void;
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
        suggestions: state.suggestions.map((s) => s.id === id ? { ...s, accepted: true } : s),
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
        reminders: state.reminders.map((r) => r.id === id ? { ...r, completed: true } : r),
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

  // ── P2 Smart Prefetch ──

  prefetchOnConversationSwitch: (conversationId: string) => {
    if (!get().isEnabled || !canPrefetchNow("conversationSwitch")) { return; }

    // Prefetch token counts for the conversation in background
    const type = "conversationSwitch";
    invoke("list_messages_page", {
      conversationId,
      limit: 1,
      beforeMessageId: null,
    })
      .then(() => {
        markPrefetched(type, [conversationId]);
      })
      .catch(() => {
        // Silent — prefetch is best-effort
      });

    // Also prefetch compression summary if available
    invoke("get_compression_summary", { conversationId })
      .then(() => {
        markPrefetched("compressionSummary", [conversationId]);
      })
      .catch((e: unknown) => {
        console.warn("[IPC]", e);
      });
  },

  prefetchModelCosts: (_providerId: string, _modelId: string) => {
    if (!get().isEnabled || !canPrefetchNow("modelCosts")) { return; }

    // Cost estimation is now config-based (pricing.toml) and computed locally.
    // The backend has fast O(1) lookup via lookup_pricing_from_config().
    // We still call it asynchronously to warm any cold caches.
    const type = "modelCosts";
    invoke("get_invoke_metrics", {})
      .then(() => {
        markPrefetched(type, ["metrics"]);
      })
      .catch((e: unknown) => {
        console.warn("[IPC]", e);
      });
  },

  predictAndPrefetch: (inputText: string): IntentPrediction[] => {
    const intents = predictIntentFromInput(inputText);
    if (intents.length === 0 || !get().isEnabled) { return intents; }

    // Trigger prefetch based on predicted intent
    for (const intent of intents) {
      const type = `intent:${intent.intent}`;
      if (!canPrefetchNow(type)) { continue; }

      // Fire-and-forget prefetch based on intent type
      switch (intent.intent) {
        case "search":
          // Warm up search provider
          invoke("list_search_providers", {})
            .then(() => markPrefetched(type, ["searchProviders"]))
            .catch((e: unknown) => {
              console.warn("[IPC]", e);
            });
          break;
        case "codeGeneration":
          // Pre-warm code executor
          invoke("list_local_tools", {})
            .then(() => markPrefetched(type, ["localTools"]))
            .catch((e: unknown) => {
              console.warn("[IPC]", e);
            });
          break;
        case "translation":
          // Pre-warm language models list
          invoke("list_providers", {})
            .then(() => markPrefetched(type, ["providers"]))
            .catch((e: unknown) => {
              console.warn("[IPC]", e);
            });
          break;
        default:
          break;
      }
    }

    return intents;
  },

  resetPrefetchState: () => {
    _prefetchState.prefetchedIds.clear();
    _prefetchState.lastPrefetchTime = {};
  },
}));

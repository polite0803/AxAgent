import { invoke } from "@/lib/invoke";
import { create } from "zustand";

// ─── Types ───

export type ModelTier = "budget" | "balanced" | "premium";

export interface RouteDecision {
  tier: ModelTier;
  min_tokens: number;
  cacheable: boolean;
  cache_ttl_secs: number | null;
  reason: string;
}

// ─── Tier-to-model mapping (model IDs by provider, configured per user) ───

interface TierModels {
  budget: Array<{ providerId: string; model_id: string }>;
  balanced: Array<{ providerId: string; model_id: string }>;
  premium: Array<{ providerId: string; model_id: string }>;
}

interface RouterState {
  /** Last routing decision for the current prompt */
  lastDecision: RouteDecision | null;
  /** Whether routing is enabled */
  enabled: boolean;
  /** User-configured tier model mappings */
  tierModels: TierModels;
  /** Loading state for backend call */
  loading: boolean;

  /** Classify the current prompt and return a routing decision */
  classifyRoute: (prompt: string) => Promise<RouteDecision>;

  /** Get the best model for a given tier */
  getModelForTier: (
    tier: ModelTier,
    currentProviderId: string,
  ) => { providerId: string; model_id: string } | null;

  /** Toggle smart routing on/off */
  setEnabled: (enabled: boolean) => void;

  /** Update the model mapping for a specific tier */
  setTierModels: (tier: keyof TierModels, models: TierModels[keyof TierModels]) => void;

  /** Clear the last decision */
  clearDecision: () => void;
}

const DEFAULT_TIER_MODELS: TierModels = {
  budget: [],
  balanced: [],
  premium: [],
};

export const useRouterStore = create<RouterState>((set, get) => ({
  lastDecision: null,
  enabled: true,
  tierModels: DEFAULT_TIER_MODELS,
  loading: false,

  classifyRoute: async (prompt: string) => {
    set({ loading: true });
    try {
      const decision = await invoke<RouteDecision>("classify_route", {
        prompt,
      });
      set({ lastDecision: decision, loading: false });
      return decision;
    } catch (e) {
      console.error("[routerStore] classifyRoute failed:", e);
      set({ loading: false });
      // Fallback: treat everything as balanced
      return {
        tier: "balanced" as ModelTier,
        min_tokens: 2048,
        cacheable: false,
        cache_ttl_secs: null,
        reason: "router unavailable, defaulting to balanced",
      };
    }
  },

  getModelForTier: (tier: ModelTier, currentProviderId: string) => {
    const models = get().tierModels[tier];
    if (models.length === 0) return null;

    // Prefer same-provider model, then first available
    const sameProvider = models.find((m) => m.providerId === currentProviderId);
    return sameProvider ?? models[0];
  },

  setEnabled: (enabled: boolean) => set({ enabled }),

  setTierModels: (tier: keyof TierModels, models: TierModels[keyof TierModels]) => {
    set((s) => ({
      tierModels: { ...s.tierModels, [tier]: models },
    }));
  },

  clearDecision: () => set({ lastDecision: null }),
}));

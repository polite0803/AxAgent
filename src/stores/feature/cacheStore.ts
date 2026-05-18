import { invoke } from "@/lib/invoke";
import { create } from "zustand";

export interface PromptCacheState {
  cacheValid: boolean;
  hasPendingChanges: boolean;
  tokensSaved: number;
  cacheHits: number;
}

interface CacheStore extends PromptCacheState {
  loading: boolean;
  error: string | null;
  fetchCacheState: () => Promise<void>;
  reset: () => void;
}

const initialState: PromptCacheState = {
  cacheValid: false,
  hasPendingChanges: false,
  tokensSaved: 0,
  cacheHits: 0,
};

export const useCacheStore = create<CacheStore>((set) => ({
  ...initialState,
  loading: false,
  error: null,

  fetchCacheState: async () => {
    set({ loading: true, error: null });
    try {
      const state = await invoke<PromptCacheState>("get_prompt_cache_state");
      set({ ...state, loading: false });
    } catch (e) {
      set({ loading: false, error: String(e) });
    }
  },

  reset: () => {
    set({ ...initialState, loading: false, error: null });
  },
}));

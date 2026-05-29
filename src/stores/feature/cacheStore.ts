import { invoke, isTauri, listen, logIpcError } from "@/lib/invoke";
import { create } from "zustand";

export interface PromptCacheState {
  cacheValid: boolean;
  hasPendingChanges: boolean;
  tokensSaved: number;
  cacheHits: number;
}

interface PromptCacheEvent {
  conversationId: string;
  assistantMessageId: string;
  unexpected: boolean;
  reason: string;
  cacheReadTokens: number;
  tokenDrop: number;
}

interface CacheStore extends PromptCacheState {
  loading: boolean;
  error: string | null;
  lastCacheEvent: PromptCacheEvent | null;
  fetchCacheState: () => Promise<void>;
  reset: () => void;
}

const initialState: PromptCacheState = {
  cacheValid: false,
  hasPendingChanges: false,
  tokensSaved: 0,
  cacheHits: 0,
};

let _listenerInitialized = false;

function initCacheEventListener() {
  if (_listenerInitialized || !isTauri()) { return; }
  _listenerInitialized = true;

  listen<PromptCacheEvent>("prompt-cache-event", (event) => {
    const evt = event.payload;
    useCacheStore.setState((state) => ({
      cacheHits: state.cacheHits + 1,
      tokensSaved: state.tokensSaved + evt.cacheReadTokens,
      cacheValid: !evt.unexpected,
      lastCacheEvent: evt,
    }));
  }).catch(logIpcError("listen:prompt-cache-event"));
}

export const useCacheStore = create<CacheStore>((set) => ({
  ...initialState,
  loading: false,
  error: null,
  lastCacheEvent: null,

  fetchCacheState: async () => {
    set({ loading: true, error: null });
    try {
      const state = await invoke<PromptCacheState>("get_prompt_cache_state");
      set({ ...state, loading: false });
      initCacheEventListener();
    } catch (e) {
      set({ loading: false, error: String(e) });
    }
  },

  reset: () => {
    set({ ...initialState, loading: false, error: null, lastCacheEvent: null });
  },
}));

initCacheEventListener();

import { invoke } from "@tauri-apps/api/core";
import { create } from "zustand";

export interface GeneratedImage {
  url?: string;
  base64?: string;
  width: number;
  height: number;
  seed?: number;
}

export interface ImageGenResult {
  images: GeneratedImage[];
  model_used: string;
  elapsed_ms: number;
}

interface ImageGenHistoryEntry {
  prompt: string;
  images: GeneratedImage[];
  model: string;
  timestamp: number;
}

interface ImageGenState {
  generating: boolean;
  results: GeneratedImage[];
  history: ImageGenHistoryEntry[];
  currentProvider: string;
  currentApiKey: string | null;

  setProvider: (provider: string) => void;
  setApiKey: (apiKey: string) => void;
  generate: (params: {
    prompt: string;
    negative_prompt?: string;
    width?: number;
    height?: number;
    steps?: number;
  }) => Promise<GeneratedImage[]>;
  clearResults: () => void;
}

export const useImageGenStore = create<ImageGenState>((set, get) => ({
  generating: false,
  results: [],
  history: [],
  currentProvider: "flux",
  currentApiKey: null,

  setProvider: (provider) => set({ currentProvider: provider }),

  setApiKey: (apiKey) => set({ currentApiKey: apiKey }),

  generate: async (params) => {
    const { currentProvider, currentApiKey } = get();

    if (!currentApiKey) {
      throw new Error("API key not configured");
    }

    set({ generating: true });

    try {
      const res = await invoke<ImageGenResult>("generate_image", {
        prompt: params.prompt,
        negativePrompt: params.negative_prompt,
        width: params.width,
        height: params.height,
        steps: params.steps,
        provider: currentProvider,
        apiKey: currentApiKey,
      });

      set((s) => ({
        results: res.images,
        history: [
          {
            prompt: params.prompt,
            images: res.images,
            model: res.model_used,
            timestamp: Date.now(),
          },
          ...s.history,
        ].slice(0, 50),
      }));

      return res.images;
    } finally {
      set({ generating: false });
    }
  },

  clearResults: () => set({ results: [] }),
}));

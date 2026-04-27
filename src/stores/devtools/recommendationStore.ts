import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";

export interface ToolScore {
  tool_id: string;
  tool_name: string;
  score: number;
  reasons: string[];
}

export interface AlternativeSet {
  description: string;
  tools: string[];
  tradeoffs: string[];
}

export interface RecommendationResult {
  tools: ToolScore[];
  reasoning: string;
  confidence: number;
  alternatives: AlternativeSet[];
}

export interface ToolInfo {
  id: string;
  name: string;
  description: string;
  categories: string[];
}

interface RecommendationState {
  currentTask: string;
  recommendations: RecommendationResult | null;
  availableTools: ToolInfo[];
  isLoading: boolean;
  error: string | null;
  setCurrentTask: (task: string) => void;
  getRecommendations: (taskDescription: string) => Promise<void>;
  fetchAvailableTools: () => Promise<void>;
  getToolsByCategory: (category: string) => Promise<ToolInfo[]>;
  recordToolUsage: (
    taskSignature: string,
    toolsUsed: string[],
    success: boolean,
    durationMs: number
  ) => Promise<void>;
  clearRecommendations: () => void;
}

export const useRecommendationStore = create<RecommendationState>((set) => ({
  currentTask: "",
  recommendations: null,
  availableTools: [],
  isLoading: false,
  error: null,

  setCurrentTask: (task: string) => {
    set({ currentTask: task });
  },

  getRecommendations: async (taskDescription: string) => {
    set({ isLoading: true, error: null });
    try {
      const result = await invoke<RecommendationResult>("analyze_task", {
        taskDescription,
      });
      set({ recommendations: result, isLoading: false });
    } catch (error) {
      set({ error: String(error), isLoading: false });
    }
  },

  fetchAvailableTools: async () => {
    set({ isLoading: true, error: null });
    try {
      const tools = await invoke<ToolInfo[]>("get_available_tools", {});
      set({ availableTools: tools, isLoading: false });
    } catch (error) {
      set({ error: String(error), isLoading: false });
    }
  },

  getToolsByCategory: async (category: string) => {
    try {
      const tools = await invoke<ToolInfo[]>("get_tools_by_category", { category });
      return tools;
    } catch (error) {
      set({ error: String(error) });
      return [];
    }
  },

  recordToolUsage: async (
    taskSignature: string,
    toolsUsed: string[],
    success: boolean,
    durationMs: number
  ) => {
    try {
      await invoke("record_tool_usage", {
        userId: "default",
        taskSignature,
        toolsUsed,
        success,
        durationMs,
      });
    } catch (error) {
      set({ error: String(error) });
    }
  },

  clearRecommendations: () => {
    set({ recommendations: null, currentTask: "" });
  },
}));

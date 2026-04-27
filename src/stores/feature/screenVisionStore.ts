import { invoke } from "@/lib/invoke";
import { create } from "zustand";

export interface UIElementInfo {
  element_type: string;
  name: string;
  description: string;
  bounds: {
    x: number;
    y: number;
    width: number;
    height: number;
  };
  clickable: boolean;
  editable: boolean;
  confidence: number;
}

export interface SuggestedActionInfo {
  action_type: string;
  target_element: string;
  description: string;
  reasoning: string;
  x: number;
  y: number;
}

export interface ScreenAnalysisResult {
  elements: UIElementInfo[];
  suggested_actions: SuggestedActionInfo[];
  reasoning: string;
  confidence: number;
}

export type VisionProvider = "anthropic" | "openai" | "gemini";

interface ScreenVisionState {
  lastAnalysis: ScreenAnalysisResult | null;
  selectedElement: UIElementInfo | null;
  suggestedActions: SuggestedActionInfo[];
  isAnalyzing: boolean;
  error: string | null;
  provider: VisionProvider;
  monitorIndex: number;

  analyzeScreen: (taskDescription: string, monitorIndex?: number) => Promise<ScreenAnalysisResult | null>;
  findElement: (description: string, monitorIndex?: number) => Promise<UIElementInfo | null>;
  suggestAction: (currentTask: string, monitorIndex?: number) => Promise<SuggestedActionInfo[]>;
  clickAtPosition: (x: number, y: number, button?: string) => Promise<void>;
  executeAction: (actionType: string, x: number, y: number, text?: string) => Promise<void>;
  setProvider: (provider: VisionProvider) => void;
  setMonitorIndex: (index: number) => void;
  clearError: () => void;
}

export const useScreenVisionStore = create<ScreenVisionState>((set, get) => ({
  lastAnalysis: null,
  selectedElement: null,
  suggestedActions: [],
  isAnalyzing: false,
  error: null,
  provider: "anthropic",
  monitorIndex: 0,

  analyzeScreen: async (taskDescription: string, monitorIndex?: number) => {
    set({ isAnalyzing: true, error: null });
    try {
      const monIdx = monitorIndex ?? get().monitorIndex;
      const result = await invoke<ScreenAnalysisResult>("analyze_screen", {
        taskDescription,
        monitorIndex: monIdx,
      });
      set({ lastAnalysis: result, isAnalyzing: false });
      return result;
    } catch (e) {
      const errorMsg = e instanceof Error ? e.message : String(e);
      set({ error: errorMsg, isAnalyzing: false });
      return null;
    }
  },

  findElement: async (description: string, monitorIndex?: number) => {
    set({ isAnalyzing: true, error: null });
    try {
      const monIdx = monitorIndex ?? get().monitorIndex;
      const element = await invoke<UIElementInfo | null>("find_element_on_screen", {
        elementDescription: description,
        monitorIndex: monIdx,
      });
      set({ selectedElement: element, isAnalyzing: false });
      return element;
    } catch (e) {
      const errorMsg = e instanceof Error ? e.message : String(e);
      set({ error: errorMsg, isAnalyzing: false });
      return null;
    }
  },

  suggestAction: async (currentTask: string, monitorIndex?: number) => {
    set({ isAnalyzing: true, error: null });
    try {
      const monIdx = monitorIndex ?? get().monitorIndex;
      const actions = await invoke<SuggestedActionInfo[]>("suggest_screen_action", {
        currentTask,
        monitorIndex: monIdx,
      });
      set({ suggestedActions: actions, isAnalyzing: false });
      return actions;
    } catch (e) {
      const errorMsg = e instanceof Error ? e.message : String(e);
      set({ error: errorMsg, isAnalyzing: false });
      return [];
    }
  },

  clickAtPosition: async (x: number, y: number, button?: string) => {
    try {
      await invoke<void>("click_element_at_position", { x, y, button });
    } catch (e) {
      const errorMsg = e instanceof Error ? e.message : String(e);
      set({ error: errorMsg });
      throw e;
    }
  },

  executeAction: async (actionType: string, x: number, y: number, text?: string) => {
    try {
      await invoke<void>("execute_vision_action", { actionType, x, y, text });
    } catch (e) {
      const errorMsg = e instanceof Error ? e.message : String(e);
      set({ error: errorMsg });
      throw e;
    }
  },

  setProvider: (provider: VisionProvider) => {
    set({ provider });
  },

  setMonitorIndex: (index: number) => {
    set({ monitorIndex: index });
  },

  clearError: () => {
    set({ error: null });
  },
}));

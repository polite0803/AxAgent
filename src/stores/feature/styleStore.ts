import { invoke } from "@/lib/invoke";
import { create } from "zustand";

export interface StyleDimensions {
  naming_score: number;
  density_score: number;
  comment_ratio: number;
  abstraction_level: number;
  formality_score: number;
  structure_score: number;
  technical_depth: number;
  explanation_length: number;
}

export interface StyleVector {
  dimensions: StyleDimensions;
  source_confidence: number;
  learned_at: string;
  sample_count: number;
}

export interface CodeStyleTemplate {
  name: string;
  patterns: StylePattern[];
  templates: CodeTemplate[];
}

export interface StylePattern {
  pattern_type: PatternType;
  original: string;
  transformed: string;
  context: string;
  usage_count: number;
}

export type PatternType = "Naming" | "Formatting" | "Structure" | "Comment";

export interface CodeTemplate {
  name: string;
  template: string;
  description: string;
}

export interface DocumentStyleProfile {
  formality_level: number;
  structure_level: number;
  technical_vocabulary_ratio: number;
  explanation_detail_level: number;
  preferred_format: DocumentFormat;
}

export type DocumentFormat = "PlainText" | "Markdown" | "Structured";

export interface UserStyleProfile {
  id: string;
  user_id: string;
  code_style_vector: StyleVector;
  document_style_profile: DocumentStyleProfile;
  code_templates: CodeStyleTemplate[];
  learned_patterns: LearnedPattern[];
  created_at: string;
  updated_at: string;
  total_samples: number;
  confidence: number;
}

export interface LearnedPattern {
  id: string;
  pattern_type: LearnedPatternType;
  original: string;
  transformed: string;
  context: string;
  usage_count: number;
  last_used: string;
}

export type LearnedPatternType =
  | "Naming"
  | "Formatting"
  | "Comment"
  | "Structure"
  | "Document";

export interface StyleMigratorStats {
  total_profiles: number;
  total_samples: number;
  average_confidence: number;
}

interface StyleStore {
  currentProfile: UserStyleProfile | null;
  appliedStyle: StyleVector | null;
  isApplying: boolean;
  isLoading: boolean;
  error: string | null;

  loadStyleProfile: (userId: string) => Promise<void>;
  applyStyleToCode: (code: string, userId?: string) => Promise<string>;
  applyStyleToDocument: (content: string, userId?: string) => Promise<string>;
  adjustStyleDimension: (dimension: keyof StyleDimensions, value: number) => void;
  resetToDefaults: () => void;
  learnFromCodeSamples: (userId: string, samples: CodeSample[]) => Promise<void>;
  learnFromMessages: (userId: string, messages: MessageSample[]) => Promise<void>;
  exportProfile: (userId: string) => Promise<string | null>;
  importProfile: (userId: string, json: string) => Promise<void>;
  getStats: () => Promise<StyleMigratorStats | null>;
}

export interface CodeSample {
  code: string;
  language: string;
  timestamp: string;
}

export interface MessageSample {
  content: string;
  role: string;
  timestamp: string;
}

export const useStyleStore = create<StyleStore>((set, get) => ({
  currentProfile: null,
  appliedStyle: null,
  isApplying: false,
  isLoading: false,
  error: null,

  loadStyleProfile: async (userId: string) => {
    set({ isLoading: true, error: null });
    try {
      const profile = await invoke<UserStyleProfile | null>("style_get_profile", {
        userId,
      });
      set({ currentProfile: profile, isLoading: false });
    } catch (error) {
      set({
        error: error instanceof Error ? error.message : "Failed to load style profile",
        isLoading: false,
      });
    }
  },

  applyStyleToCode: async (code: string, userId?: string) => {
    set({ isApplying: true, error: null });
    try {
      const result = await invoke<string>("style_apply_code", {
        code,
        userId: userId || "default",
      });
      set({ isApplying: false });
      return result;
    } catch (error) {
      set({
        error: error instanceof Error ? error.message : "Failed to apply style",
        isApplying: false,
      });
      return code;
    }
  },

  applyStyleToDocument: async (content: string, userId?: string) => {
    set({ isApplying: true, error: null });
    try {
      const result = await invoke<string>("style_apply_document", {
        content,
        userId: userId || "default",
      });
      set({ isApplying: false });
      return result;
    } catch (error) {
      set({
        error:
          error instanceof Error ? error.message : "Failed to apply document style",
        isApplying: false,
      });
      return content;
    }
  },

  adjustStyleDimension: (dimension: keyof StyleDimensions, value: number) => {
    const { currentProfile } = get();
    if (!currentProfile) return;

    const updatedDimensions = {
      ...currentProfile.code_style_vector.dimensions,
      [dimension]: Math.max(0, Math.min(1, value)),
    };

    set({
      currentProfile: {
        ...currentProfile,
        code_style_vector: {
          ...currentProfile.code_style_vector,
          dimensions: updatedDimensions,
        },
      },
      appliedStyle: {
        ...currentProfile.code_style_vector,
        dimensions: updatedDimensions,
      },
    });
  },

  resetToDefaults: () => {
    const defaultDimensions: StyleDimensions = {
      naming_score: 0.5,
      density_score: 0.5,
      comment_ratio: 0.5,
      abstraction_level: 0.5,
      formality_score: 0.5,
      structure_score: 0.5,
      technical_depth: 0.5,
      explanation_length: 0.5,
    };

    const defaultVector: StyleVector = {
      dimensions: defaultDimensions,
      source_confidence: 0,
      learned_at: new Date().toISOString(),
      sample_count: 0,
    };

    set({
      currentProfile: null,
      appliedStyle: defaultVector,
    });
  },

  learnFromCodeSamples: async (userId: string, samples: CodeSample[]) => {
    set({ isLoading: true, error: null });
    try {
      const profile = await invoke<UserStyleProfile>("style_learn_code", {
        userId,
        samples,
      });
      set({ currentProfile: profile, isLoading: false });
    } catch (error) {
      set({
        error:
          error instanceof Error ? error.message : "Failed to learn from samples",
        isLoading: false,
      });
    }
  },

  learnFromMessages: async (userId: string, messages: MessageSample[]) => {
    set({ isLoading: true, error: null });
    try {
      const profile = await invoke<DocumentStyleProfile>("style_learn_messages", {
        userId,
        messages,
      });
      const { currentProfile } = get();
      if (currentProfile) {
        set({
          currentProfile: {
            ...currentProfile,
            document_style_profile: profile,
          },
          isLoading: false,
        });
      }
    } catch (error) {
      set({
        error:
          error instanceof Error ? error.message : "Failed to learn from messages",
        isLoading: false,
      });
    }
  },

  exportProfile: async (userId: string) => {
    try {
      const json = await invoke<string>("style_export_profile", { userId });
      return json;
    } catch (error) {
      set({
        error:
          error instanceof Error ? error.message : "Failed to export profile",
      });
      return null;
    }
  },

  importProfile: async (userId: string, json: string) => {
    set({ isLoading: true, error: null });
    try {
      await invoke("style_import_profile", { userId, json });
      await get().loadStyleProfile(userId);
    } catch (error) {
      set({
        error:
          error instanceof Error ? error.message : "Failed to import profile",
        isLoading: false,
      });
    }
  },

  getStats: async () => {
    try {
      const stats = await invoke<StyleMigratorStats>("style_get_stats");
      return stats;
    } catch (error) {
      set({
        error: error instanceof Error ? error.message : "Failed to get stats",
      });
      return null;
    }
  },
}));

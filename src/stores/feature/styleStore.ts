// SPDX-License-Identifier: AGPL-3.0-only

import { invoke, isTauri } from "@/lib/invoke";
import type {
  CodeStyleTemplate,
  CodeTemplate,
  DocumentFormat,
  DocumentStyleProfile,
  LearnedPattern,
  LearnedPatternType,
  PatternType,
  StyleDimensions,
  StylePattern,
  StyleVector,
  UserStyleProfile,
} from "@/types";
import { create } from "zustand";

export type {
  CodeStyleTemplate,
  CodeTemplate,
  DocumentFormat,
  DocumentStyleProfile,
  LearnedPattern,
  LearnedPatternType,
  PatternType,
  StyleDimensions,
  StylePattern,
  StyleVector,
  UserStyleProfile,
};

export interface StyleMigratorStats {
  totalProfiles: number;
  totalSamples: number;
  averageConfidence: number;
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
  adjustStyleDimension: (
    dimension: keyof StyleDimensions,
    value: number,
  ) => void;
  resetToDefaults: () => void;
  learnFromCodeSamples: (
    userId: string,
    samples: CodeSample[],
  ) => Promise<void>;
  learnFromMessages: (
    userId: string,
    messages: MessageSample[],
  ) => Promise<void>;
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
    if (!isTauri()) {
      const defaultDimensions: StyleDimensions = {
        namingScore: 0.5,
        densityScore: 0.5,
        commentRatio: 0.5,
        abstractionLevel: 0.5,
        formalityScore: 0.5,
        structureScore: 0.5,
        technicalDepth: 0.5,
        explanationLength: 0.5,
      };
      const defaultVector: StyleVector = {
        dimensions: defaultDimensions,
        sourceConfidence: 0.5,
        learnedAt: new Date().toISOString(),
        sampleCount: 0,
      };
      const defaultDocumentProfile: DocumentStyleProfile = {
        formalityLevel: 0.5,
        structureLevel: 0.5,
        technicalVocabularyRatio: 0.5,
        explanationDetailLevel: 0.5,
        preferredFormat: "Markdown",
      };
      const defaultProfile: UserStyleProfile = {
        id: "default",
        userId: userId,
        codeStyleVector: defaultVector,
        documentStyleProfile: defaultDocumentProfile,
        codeTemplates: [],
        learnedPatterns: [],
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
        totalSamples: 0,
        confidence: 0.5,
      };
      set({ currentProfile: defaultProfile, isLoading: false });
      return;
    }
    set({ isLoading: true, error: null });
    try {
      const profile = await invoke<UserStyleProfile | null>(
        "style_get_profile",
        {
          userId,
        },
      );
      set({ currentProfile: profile, isLoading: false });
    } catch (error) {
      set({
        error: error instanceof Error
          ? error.message
          : "Failed to load style profile",
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
        error: error instanceof Error
          ? error.message
          : "Failed to apply document style",
        isApplying: false,
      });
      return content;
    }
  },

  adjustStyleDimension: (dimension: keyof StyleDimensions, value: number) => {
    const { currentProfile } = get();
    if (!currentProfile) {
      return;
    }

    const updatedDimensions = {
      ...currentProfile.codeStyleVector.dimensions,
      [dimension]: Math.max(0, Math.min(1, value)),
    };

    set({
      currentProfile: {
        ...currentProfile,
        codeStyleVector: {
          ...currentProfile.codeStyleVector,
          dimensions: updatedDimensions,
        },
      },
      appliedStyle: {
        ...currentProfile.codeStyleVector,
        dimensions: updatedDimensions,
      },
    });
  },

  resetToDefaults: () => {
    const defaultDimensions: StyleDimensions = {
      namingScore: 0.5,
      densityScore: 0.5,
      commentRatio: 0.5,
      abstractionLevel: 0.5,
      formalityScore: 0.5,
      structureScore: 0.5,
      technicalDepth: 0.5,
      explanationLength: 0.5,
    };

    const defaultVector: StyleVector = {
      dimensions: defaultDimensions,
      sourceConfidence: 0,
      learnedAt: new Date().toISOString(),
      sampleCount: 0,
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
        error: error instanceof Error
          ? error.message
          : "Failed to learn from samples",
        isLoading: false,
      });
    }
  },

  learnFromMessages: async (userId: string, messages: MessageSample[]) => {
    set({ isLoading: true, error: null });
    try {
      const profile = await invoke<DocumentStyleProfile>(
        "style_learn_messages",
        {
          userId,
          messages,
        },
      );
      const { currentProfile } = get();
      if (currentProfile) {
        set({
          currentProfile: {
            ...currentProfile,
            documentStyleProfile: profile,
          },
          isLoading: false,
        });
      }
    } catch (error) {
      set({
        error: error instanceof Error
          ? error.message
          : "Failed to learn from messages",
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
        error: error instanceof Error ? error.message : "Failed to export profile",
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
        error: error instanceof Error ? error.message : "Failed to import profile",
        isLoading: false,
      });
    }
  },

  getStats: async () => {
    if (!isTauri()) {
      return { totalProfiles: 1, totalSamples: 0, averageConfidence: 0.5 };
    }
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

// SPDX-License-Identifier: AGPL-3.0-only

import { invoke } from "@/lib/invoke";
import type { ConversationCategory, NullableModelRef } from "@/types";
import { create } from "zustand";

interface CategoryState {
  categories: ConversationCategory[];
  loading: boolean;
  fetchCategories: () => Promise<void>;
  createCategory: (input: {
    name: string;
    iconType?: string | null;
    iconValue?: string | null;
    systemPrompt?: string | null;
    defaultModel?: NullableModelRef;
    defaultTemperature?: number | null;
    defaultMaxTokens?: number | null;
    defaultTopP?: number | null;
    defaultFrequencyPenalty?: number | null;
  }) => Promise<ConversationCategory>;
  updateCategory: (
    id: string,
    input: {
      name?: string;
      iconType?: string | null;
      iconValue?: string | null;
      systemPrompt?: string | null;
      defaultModel?: NullableModelRef;
      defaultTemperature?: number | null;
      defaultMaxTokens?: number | null;
      defaultTopP?: number | null;
      defaultFrequencyPenalty?: number | null;
    },
  ) => Promise<void>;
  deleteCategory: (id: string) => Promise<void>;
  reorderCategories: (categoryIds: string[]) => Promise<void>;
  setCollapsed: (id: string, collapsed: boolean) => Promise<void>;
}

export const useCategoryStore = create<CategoryState>((set) => ({
  categories: [],
  loading: false,

  fetchCategories: async () => {
    set({ loading: true });
    try {
      const categories = await invoke<ConversationCategory[]>(
        "list_conversation_categories",
      );
      set({ categories, loading: false });
    } catch {
      set({ loading: false });
    }
  },

  createCategory: async (input) => {
    const category = await invoke<ConversationCategory>(
      "create_conversation_category",
      {
        input: {
          name: input.name,
          icon_type: input.iconType,
          icon_value: input.iconValue,
          system_prompt: input.systemPrompt,
          default_provider_id: input.defaultModel?.a,
          default_model_id: input.defaultModel?.b,
          default_temperature: input.defaultTemperature,
          default_max_tokens: input.defaultMaxTokens,
          default_top_p: input.defaultTopP,
          default_frequency_penalty: input.defaultFrequencyPenalty,
        },
      },
    );
    set((s) => ({ categories: [...s.categories, category] }));
    return category;
  },

  updateCategory: async (id, input) => {
    const updated = await invoke<ConversationCategory>(
      "update_conversation_category",
      {
        id,
        input: {
          name: input.name,
          icon_type: input.iconType,
          icon_value: input.iconValue,
          system_prompt: input.systemPrompt,
          default_provider_id: input.defaultModel?.a,
          default_model_id: input.defaultModel?.b,
          default_temperature: input.defaultTemperature,
          default_max_tokens: input.defaultMaxTokens,
          default_top_p: input.defaultTopP,
          default_frequency_penalty: input.defaultFrequencyPenalty,
        },
      },
    );
    set((s) => ({
      categories: s.categories.map((c) => (c.id === id ? updated : c)),
    }));
  },

  deleteCategory: async (id) => {
    await invoke("delete_conversation_category", { id });
    set((s) => ({
      categories: s.categories.filter((c) => c.id !== id),
    }));
  },

  reorderCategories: async (categoryIds) => {
    await invoke("reorder_conversation_categories", { categoryIds });
    set((s) => {
      const ordered = categoryIds.flatMap((id, i) => {
        const c = s.categories.find((c) => c.id === id);
        return c ? [{ ...c, sortOrder: i }] : [];
      }) as ConversationCategory[];
      return { categories: ordered };
    });
  },

  setCollapsed: async (id, collapsed) => {
    set((s) => ({
      categories: s.categories.map((c) => c.id === id ? { ...c, isCollapsed: collapsed } : c),
    }));
    await invoke("set_conversation_category_collapsed", { id, collapsed });
  },
}));

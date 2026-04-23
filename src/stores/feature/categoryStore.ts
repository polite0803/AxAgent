import { create } from 'zustand';
import { invoke } from '@/lib/invoke';
import type { ConversationCategory } from '@/types';

interface CategoryState {
  categories: ConversationCategory[];
  loading: boolean;
  fetchCategories: () => Promise<void>;
  createCategory: (input: {
    name: string;
    icon_type?: string | null;
    icon_value?: string | null;
    system_prompt?: string | null;
    default_provider_id?: string | null;
    default_model_id?: string | null;
    default_temperature?: number | null;
    default_max_tokens?: number | null;
    default_top_p?: number | null;
    default_frequency_penalty?: number | null;
  }) => Promise<ConversationCategory>;
  updateCategory: (
    id: string,
    input: {
      name?: string;
      icon_type?: string | null;
      icon_value?: string | null;
      system_prompt?: string | null;
      default_provider_id?: string | null;
      default_model_id?: string | null;
      default_temperature?: number | null;
      default_max_tokens?: number | null;
      default_top_p?: number | null;
      default_frequency_penalty?: number | null;
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
        'list_conversation_categories',
      );
      set({ categories, loading: false });
    } catch {
      set({ loading: false });
    }
  },

  createCategory: async (input) => {
    const category = await invoke<ConversationCategory>(
      'create_conversation_category',
      { input },
    );
    set((s) => ({ categories: [...s.categories, category] }));
    return category;
  },

  updateCategory: async (id, input) => {
    const updated = await invoke<ConversationCategory>(
      'update_conversation_category',
      { id, input },
    );
    set((s) => ({
      categories: s.categories.map((c) => (c.id === id ? updated : c)),
    }));
  },

  deleteCategory: async (id) => {
    await invoke('delete_conversation_category', { id });
    set((s) => ({
      categories: s.categories.filter((c) => c.id !== id),
    }));
  },

  reorderCategories: async (categoryIds) => {
    await invoke('reorder_conversation_categories', { categoryIds });
    set((s) => {
      const ordered = categoryIds
        .map((id, i) => {
          const c = s.categories.find((c) => c.id === id);
          return c ? { ...c, sort_order: i } : null;
        })
        .filter(Boolean) as ConversationCategory[];
      return { categories: ordered };
    });
  },

  setCollapsed: async (id, collapsed) => {
    set((s) => ({
      categories: s.categories.map((c) =>
        c.id === id ? { ...c, is_collapsed: collapsed } : c,
      ),
    }));
    await invoke('set_conversation_category_collapsed', { id, collapsed });
  },
}));

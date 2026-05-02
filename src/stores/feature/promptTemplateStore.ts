import { invoke } from "@/lib/invoke";
import type { CreatePromptTemplateInput, PromptTemplate, PromptTemplateVersion, UpdatePromptTemplateInput } from "@/types";
import { create } from "zustand";

interface PromptTemplateState {
  templates: PromptTemplate[];
  versions: PromptTemplateVersion[];
  loading: boolean;
  error: string | null;

  loadTemplates: () => Promise<void>;
  createTemplate: (input: CreatePromptTemplateInput) => Promise<PromptTemplate | null>;
  updateTemplate: (id: string, input: UpdatePromptTemplateInput) => Promise<void>;
  deleteTemplate: (id: string) => Promise<void>;
  loadVersions: (templateId: string) => Promise<void>;
}

export const usePromptTemplateStore = create<PromptTemplateState>((set, _get) => ({
  templates: [],
  versions: [],
  loading: false,
  error: null,

  loadTemplates: async () => {
    set({ loading: true });
    try {
      const templates = await invoke<PromptTemplate[]>("list_prompt_templates");
      set({ templates, loading: false, error: null });
    } catch (e) {
      set({ error: String(e), loading: false });
    }
  },

  createTemplate: async (input) => {
    try {
      const template = await invoke<PromptTemplate>("create_prompt_template", { input });
      set((s) => ({ templates: [template, ...s.templates], error: null }));
      return template;
    } catch (e) {
      set({ error: String(e) });
      return null;
    }
  },

  updateTemplate: async (id, input) => {
    try {
      const updated = await invoke<PromptTemplate>("update_prompt_template", { id, input });
      set((s) => ({
        templates: s.templates.map((t) => (t.id === id ? updated : t)),
        error: null,
      }));
    } catch (e) {
      set({ error: String(e) });
      throw e;
    }
  },

  deleteTemplate: async (id) => {
    try {
      await invoke("delete_prompt_template", { id });
      set((s) => ({
        templates: s.templates.filter((t) => t.id !== id),
        error: null,
      }));
    } catch (e) {
      set({ error: String(e) });
      throw e;
    }
  },

  loadVersions: async (templateId) => {
    try {
      const versions = await invoke<PromptTemplateVersion[]>("get_prompt_template_versions", { templateId });
      set({ versions, error: null });
    } catch (e) {
      set({ error: String(e) });
    }
  },
}));
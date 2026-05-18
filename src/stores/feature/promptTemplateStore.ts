import { invoke } from "@/lib/invoke";
import type {
  CreatePromptTemplateInput,
  ExportPromptFormat,
  ImportFromUrlInput,
  ImportPromptResult,
  ImportPromptTemplateInput,
  PromptTemplate,
  PromptTemplateVersion,
  UpdatePromptTemplateInput,
} from "@/types";
import { create } from "zustand";

interface PromptTemplateState {
  templates: PromptTemplate[];
  versions: PromptTemplateVersion[];
  loading: boolean;
  error: string | null;

  loadTemplates: () => Promise<void>;
  createTemplate: (
    input: CreatePromptTemplateInput,
  ) => Promise<PromptTemplate | null>;
  updateTemplate: (
    id: string,
    input: UpdatePromptTemplateInput,
  ) => Promise<void>;
  deleteTemplate: (id: string) => Promise<void>;
  loadVersions: (templateId: string) => Promise<void>;
  rollbackTemplate: (
    id: string,
    targetVersion: number,
  ) => Promise<PromptTemplate | null>;
  importTemplates: (
    inputs: ImportPromptTemplateInput[],
  ) => Promise<ImportPromptResult | null>;
  importFromUrl: (
    input: ImportFromUrlInput,
  ) => Promise<ImportPromptResult | null>;
  importFromFolder: (
    folderPath: string,
    categoryFilter?: string,
  ) => Promise<ImportPromptResult | null>;
  exportTemplates: (
    ids: string[],
    format: ExportPromptFormat,
  ) => Promise<string | null>;
  incrementUsage: (id: string) => Promise<void>;
  toggleFavorite: (id: string) => Promise<void>;
}

export const usePromptTemplateStore = create<PromptTemplateState>(
  (set, get) => ({
    templates: [],
    versions: [],
    loading: false,
    error: null,

    loadTemplates: async () => {
      set({ loading: true });
      try {
        const templates = await invoke<PromptTemplate[]>(
          "list_prompt_templates",
        );
        set({ templates, loading: false, error: null });
      } catch (e) {
        set({ error: String(e), loading: false });
      }
    },

    createTemplate: async (input) => {
      try {
        const template = await invoke<PromptTemplate>(
          "create_prompt_template",
          { input },
        );
        set((s) => ({ templates: [template, ...s.templates], error: null }));
        return template;
      } catch (e) {
        set({ error: String(e) });
        return null;
      }
    },

    updateTemplate: async (id, input) => {
      try {
        const updated = await invoke<PromptTemplate>("update_prompt_template", {
          id,
          input,
        });
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
        const versions = await invoke<PromptTemplateVersion[]>(
          "get_prompt_template_versions",
          { templateId },
        );
        set({ versions, error: null });
      } catch (e) {
        set({ error: String(e) });
      }
    },

    rollbackTemplate: async (id, targetVersion) => {
      try {
        const template = await invoke<PromptTemplate>(
          "rollback_prompt_template",
          {
            id,
            targetVersion,
          },
        );
        set((s) => ({
          templates: s.templates.map((t) => (t.id === id ? template : t)),
          error: null,
        }));
        return template;
      } catch (e) {
        set({ error: String(e) });
        return null;
      }
    },

    importTemplates: async (inputs) => {
      try {
        const result = await invoke<ImportPromptResult>(
          "import_prompt_templates",
          { inputs },
        );
        if (result.imported.length > 0) {
          await get().loadTemplates();
        }
        set({ error: null });
        return result;
      } catch (e) {
        set({ error: String(e) });
        return null;
      }
    },

    importFromUrl: async (input) => {
      try {
        const result = await invoke<ImportPromptResult>(
          "import_prompt_from_url",
          { input },
        );
        if (result.imported.length > 0) {
          await get().loadTemplates();
        }
        set({ error: null });
        return result;
      } catch (e) {
        set({ error: String(e) });
        return null;
      }
    },

    importFromFolder: async (folderPath: string, categoryFilter?: string) => {
      try {
        const result = await invoke<ImportPromptResult>(
          "import_prompt_from_folder",
          {
            folderPath,
            categoryFilter: categoryFilter || null,
          },
        );
        if (result.imported.length > 0) {
          await get().loadTemplates();
        }
        set({ error: null });
        return result;
      } catch (e) {
        set({ error: String(e) });
        return null;
      }
    },

    exportTemplates: async (ids, format) => {
      try {
        const result = await invoke<string>("export_prompt_templates", {
          ids,
          format,
        });
        set({ error: null });
        return result;
      } catch (e) {
        set({ error: String(e) });
        return null;
      }
    },

    incrementUsage: async (id) => {
      try {
        const template = await invoke<PromptTemplate>(
          "increment_prompt_usage",
          { id },
        );
        set((s) => ({
          templates: s.templates.map((t) => (t.id === id ? template : t)),
        }));
      } catch {
        // 静默失败，使用计数不影响核心功能
      }
    },

    toggleFavorite: async (id) => {
      const t = get().templates.find((tmpl) => tmpl.id === id);
      if (!t) {
        return;
      }
      try {
        const updated = await invoke<PromptTemplate>("update_prompt_template", {
          id,
          input: { isFavorite: !t.isFavorite } as UpdatePromptTemplateInput,
        });
        set((s) => ({
          templates: s.templates.map((tmpl) => tmpl.id === id ? updated : tmpl),
        }));
      } catch {
        // 静默失败
      }
    },
  }),
);

// SPDX-License-Identifier: AGPL-3.0-only

import { translateBackendError } from "@/lib/errorI18n";
import { invoke } from "@/lib/invoke";
import type {
  CompiledPage,
  CompileResult,
  FolderImportPreviewItem,
  FolderImportResult,
  IngestResult,
  LintIssue,
  LintResult,
  PageResult,
  QueryResult,
  SchemaVersion,
  Wiki,
  WikiOperation,
  WikiPage,
  WikiSource,
} from "@/types";
import { create } from "zustand";

// 从 @/types 重导出，保持向后兼容
export type {
  CompiledPage,
  CompileResult,
  FolderImportPreviewItem,
  FolderImportResult,
  IngestResult,
  LintIssue,
  LintResult,
  PageResult,
  QueryResult,
  SchemaVersion,
  Wiki,
  WikiOperation,
  WikiPage,
  WikiSource,
};

interface LlmWikiState {
  wikis: Wiki[];
  selectedWikiId: string | null;
  sources: WikiSource[];
  pages: WikiPage[];
  operations: WikiOperation[];
  loading: boolean;
  error: string | null;

  loadWikis: () => Promise<void>;
  selectWiki: (wikiId: string | null) => void;
  createWiki: (
    name: string,
    rootPath: string,
    description?: string,
  ) => Promise<Wiki | null>;
  deleteWiki: (wikiId: string) => Promise<void>;

  ingestSource: (
    wikiId: string,
    sourceType: string,
    path: string,
    url?: string,
    title?: string,
  ) => Promise<IngestResult | null>;
  deleteSource: (sourceId: string) => Promise<boolean>;
  compileWiki: (
    wikiId: string,
    sourceIds: string[],
  ) => Promise<CompileResult | null>;
  queryWiki: (
    wikiId: string,
    query: string,
    limit?: number,
    offset?: number,
  ) => Promise<QueryResult | null>;

  lintNote: (noteId: string) => Promise<LintResult | null>;
  updateLintScore: (noteId: string) => Promise<number | null>;

  getSchema: (wikiId: string) => Promise<string | null>;
  validateFrontmatter: (
    wikiId: string,
    frontmatter: Record<string, unknown>,
  ) => Promise<string[] | null>;
  createSchemaVersion: (
    wikiId: string,
    version: string,
    description?: string,
  ) => Promise<SchemaVersion | null>;

  loadOperations: (wikiId: string) => Promise<void>;

  updateSchema: (wikiId: string, content: string) => Promise<void>;
  deleteSchema: (wikiId: string) => Promise<void>;
  lintVault: (wikiId: string) => Promise<LintResult[] | null>;
  autoFix: (wikiId: string, noteId?: string) => Promise<string[] | null>;
  askQuestion: (wikiId: string, question: string) => Promise<string | null>;
  processSyncPending: (wikiId: string) => Promise<number | null>;

  importFolderPreview: (
    folderPath: string,
  ) => Promise<FolderImportPreviewItem[]>;
  importFolder: (
    wikiId: string,
    folderPath: string,
  ) => Promise<FolderImportResult | null>;
}

export const useLlmWikiStore = create<LlmWikiState>((set) => ({
  wikis: [],
  selectedWikiId: null,
  sources: [],
  pages: [],
  operations: [],
  loading: false,
  error: null,

  loadWikis: async () => {
    set({ loading: true, error: null });
    try {
      const wikis = await invoke<Wiki[]>("llm_wiki_list", {});
      set({ wikis: Array.isArray(wikis) ? wikis : [], loading: false });
    } catch (e) {
      set({ error: translateBackendError(e), loading: false });
    }
  },

  selectWiki: (wikiId) => {
    set({ selectedWikiId: wikiId, sources: [], pages: [] });
  },

  createWiki: async (name, rootPath, description) => {
    try {
      const wiki = await invoke<Wiki>("llm_wiki_create", {
        input: {
          name,
          rootPath,
          description,
        },
      });
      set((s) => ({ wikis: [...s.wikis, wiki] }));
      return wiki;
    } catch (e) {
      set({ error: translateBackendError(e) });
      return null;
    }
  },

  deleteWiki: async (wikiId) => {
    try {
      await invoke("llm_wiki_delete", { wikiId });
      set((s) => ({
        wikis: s.wikis.filter((w) => w.id !== wikiId),
        selectedWikiId: s.selectedWikiId === wikiId ? null : s.selectedWikiId,
      }));
    } catch (e) {
      set({ error: translateBackendError(e) });
    }
  },

  ingestSource: async (wikiId, sourceType, path, url, title) => {
    try {
      const result = await invoke<IngestResult>("llm_wiki_ingest", {
        input: {
          wikiId,
          sourceType,
          path,
          url,
          title,
        },
      });
      return result;
    } catch (e) {
      set({ error: translateBackendError(e) });
      return null;
    }
  },

  deleteSource: async (sourceId) => {
    try {
      await invoke("llm_wiki_delete_source", { sourceId });
      set((s) => ({ sources: s.sources.filter((src) => src.id !== sourceId) }));
      return true;
    } catch (e) {
      set({ error: translateBackendError(e) });
      return false;
    }
  },

  compileWiki: async (wikiId, sourceIds) => {
    try {
      const result = await invoke<CompileResult>("llm_wiki_compile", {
        input: {
          wikiId,
          sourceIds,
        },
      });
      return result;
    } catch (e) {
      set({ error: translateBackendError(e) });
      return null;
    }
  },

  queryWiki: async (wikiId, query, limit, offset) => {
    try {
      const result = await invoke<QueryResult>("llm_wiki_query", {
        input: {
          wikiId,
          query,
          limit,
          offset,
        },
      });
      return result;
    } catch (e) {
      set({ error: translateBackendError(e) });
      return null;
    }
  },

  lintNote: async (noteId) => {
    try {
      return await invoke<LintResult>("llm_wiki_lint", { noteId });
    } catch (e) {
      set({ error: translateBackendError(e) });
      return null;
    }
  },

  updateLintScore: async (noteId) => {
    try {
      return await invoke<number>("llm_wiki_lint_update_score", { noteId });
    } catch (e) {
      set({ error: translateBackendError(e) });
      return null;
    }
  },

  getSchema: async (wikiId) => {
    try {
      return await invoke<string>("llm_wiki_get_schema", { wikiId });
    } catch (e) {
      set({ error: translateBackendError(e) });
      return null;
    }
  },

  validateFrontmatter: async (wikiId, frontmatter) => {
    try {
      return await invoke<string[]>("llm_wiki_validate_frontmatter", {
        input: {
          wikiId,
          frontmatter,
        },
      });
    } catch (e) {
      set({ error: translateBackendError(e) });
      return null;
    }
  },

  createSchemaVersion: async (wikiId, version, description) => {
    try {
      return await invoke<SchemaVersion>("llm_wiki_create_schema_version", {
        wikiId,
        version,
        description,
      });
    } catch (e) {
      set({ error: translateBackendError(e) });
      return null;
    }
  },

  loadOperations: async (wikiId) => {
    try {
      const operations = await invoke<WikiOperation[]>(
        "llm_wiki_operations_list",
        { wikiId },
      );
      set({ operations });
    } catch (e) {
      set({ error: translateBackendError(e) });
    }
  },

  updateSchema: async (wikiId, content) => {
    try {
      await invoke("llm_wiki_update_schema", { input: { wikiId, content } });
    } catch (e) {
      set({ error: translateBackendError(e) });
    }
  },

  deleteSchema: async (wikiId) => {
    try {
      await invoke("llm_wiki_delete_schema", { wikiId });
    } catch (e) {
      set({ error: translateBackendError(e) });
    }
  },

  lintVault: async (wikiId) => {
    try {
      return await invoke<LintResult[]>("llm_wiki_lint_vault", { wikiId });
    } catch (e) {
      set({ error: translateBackendError(e) });
      return null;
    }
  },

  autoFix: async (wikiId, noteId) => {
    try {
      return await invoke<string[]>("llm_wiki_auto_fix", { wikiId, noteId });
    } catch (e) {
      set({ error: translateBackendError(e) });
      return null;
    }
  },

  askQuestion: async (wikiId, question) => {
    try {
      return await invoke<string>("llm_wiki_ask", { wikiId, question });
    } catch (e) {
      set({ error: translateBackendError(e) });
      return null;
    }
  },

  processSyncPending: async (wikiId) => {
    try {
      return await invoke<number>("wiki_sync_process_pending", { wikiId });
    } catch (e) {
      set({ error: translateBackendError(e) });
      return null;
    }
  },

  importFolderPreview: async (folderPath) => {
    try {
      const items = await invoke<FolderImportPreviewItem[]>(
        "llm_wiki_import_folder_preview",
        { folderPath },
      );
      return items;
    } catch (e) {
      set({ error: translateBackendError(e) });
      return [];
    }
  },

  importFolder: async (wikiId, folderPath) => {
    try {
      const result = await invoke<FolderImportResult>(
        "llm_wiki_import_folder",
        { input: { wikiId, folderPath } },
      );
      return result;
    } catch (e) {
      set({ error: translateBackendError(e) });
      return null;
    }
  },
}));

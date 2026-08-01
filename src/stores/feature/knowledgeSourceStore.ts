// SPDX-License-Identifier: AGPL-3.0-only

import { invoke } from "@/lib/invoke";
import type {
  CreateKnowledgeSourceInput,
  FetchSourceResult,
  FetchUrlResult,
  KnowledgeSource,
  UpdateKnowledgeSourceInput,
} from "@/types";
import { create } from "zustand";

interface KnowledgeSourceState {
  sources: KnowledgeSource[];
  loading: boolean;
  error: string | null;

  loadSources: () => Promise<void>;
  createSource: (input: CreateKnowledgeSourceInput) => Promise<KnowledgeSource | null>;
  updateSource: (input: UpdateKnowledgeSourceInput) => Promise<KnowledgeSource | null>;
  deleteSource: (id: string) => Promise<boolean>;
  fetchNow: (sourceId: string) => Promise<FetchSourceResult | null>;
  fetchAll: () => Promise<FetchSourceResult[]>;
  fetchUrlToWiki: (
    url: string,
    title?: string,
    wikiId?: string,
  ) => Promise<FetchUrlResult | null>;
  scheduleSync: (cronExpression: string) => Promise<string | null>;
  githubRepoImport: (
    repo: string,
    pathFilter?: string,
    wikiId?: string,
  ) => Promise<FetchSourceResult | null>;
  sitemapCrawl: (
    baseUrl: string,
    wikiId?: string,
  ) => Promise<FetchSourceResult[] | null>;
}

export const useKnowledgeSourceStore = create<KnowledgeSourceState>((set, get) => ({
  sources: [],
  loading: false,
  error: null,

  loadSources: async () => {
    set({ loading: true, error: null });
    try {
      const sources = await invoke<KnowledgeSource[]>("knowledge_source_list");
      set({ sources, loading: false });
    } catch (e) {
      set({ error: String(e), loading: false });
    }
  },

  createSource: async (input) => {
    try {
      const source = await invoke<KnowledgeSource>("knowledge_source_create", { input });
      await get().loadSources();
      return source;
    } catch (e) {
      set({ error: String(e) });
      return null;
    }
  },

  updateSource: async (input) => {
    try {
      const source = await invoke<KnowledgeSource>("knowledge_source_update", { input });
      await get().loadSources();
      return source;
    } catch (e) {
      set({ error: String(e) });
      return null;
    }
  },

  deleteSource: async (id) => {
    try {
      const ok = await invoke<boolean>("knowledge_source_delete", { id });
      await get().loadSources();
      return ok;
    } catch (e) {
      set({ error: String(e) });
      return false;
    }
  },

  fetchNow: async (sourceId) => {
    try {
      const result = await invoke<FetchSourceResult>("knowledge_source_fetch_now", { sourceId });
      await get().loadSources();
      return result;
    } catch (e) {
      set({ error: String(e) });
      return null;
    }
  },

  fetchAll: async () => {
    try {
      const results = await invoke<FetchSourceResult[]>("knowledge_source_fetch_all");
      await get().loadSources();
      return results;
    } catch (e) {
      set({ error: String(e) });
      return [];
    }
  },

  fetchUrlToWiki: async (url, title, wikiId) => {
    try {
      return await invoke<FetchUrlResult>("fetch_url_to_wiki", { url, title, wikiId });
    } catch (e) {
      set({ error: String(e) });
      return null;
    }
  },

  scheduleSync: async (cronExpression) => {
    try {
      return await invoke<string>("knowledge_source_schedule_sync", { cronExpression });
    } catch (e) {
      set({ error: String(e) });
      return null;
    }
  },

  githubRepoImport: async (repo, pathFilter, wikiId) => {
    try {
      return await invoke<FetchSourceResult>("github_repo_import", {
        repo,
        pathFilter,
        wikiId,
      });
    } catch (e) {
      set({ error: String(e) });
      return null;
    }
  },

  sitemapCrawl: async (baseUrl, wikiId) => {
    try {
      return await invoke<FetchSourceResult[]>("sitemap_crawl", { baseUrl, wikiId });
    } catch (e) {
      set({ error: String(e) });
      return null;
    }
  },
}));

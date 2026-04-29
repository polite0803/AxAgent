import { create } from "zustand";
import { persist } from "zustand/middleware";
import { invoke } from "@/lib/invoke";
import type {
  CreateSearchProviderInput,
  SearchProvider,
  UpdateSearchProviderInput,
} from "@/types/search";

export interface SearchResult {
  session_id: string;
  message_index: number;
  content: string;
  highlight_ranges: [number, number][];
  timestamp: string;
  agent_name?: string;
  score: number;
}

export interface SearchQuery {
  query: string;
  regex?: boolean;
  case_sensitive?: boolean;
  session_filter?: string[];
  date_from?: string;
  date_to?: string;
  limit?: number;
  offset?: number;
}

interface SearchState {
  query: string;
  results: SearchResult[];
  isSearching: boolean;
  error: string | null;
  recentSearches: string[];
  savedFilters: SavedFilter[];
  searchOptions: SearchOptions;
  providers: SearchProvider[];

  setQuery: (query: string) => void;
  setSearchOptions: (options: Partial<SearchOptions>) => void;
  search: (searchQuery?: Partial<SearchQuery>) => Promise<void>;
  clearResults: () => void;
  addRecentSearch: (query: string) => void;
  clearRecentSearches: () => void;
  saveFilter: (filter: SavedFilter) => void;
  deleteFilter: (name: string) => void;
  loadRecentSearches: () => void;
  loadProviders: () => Promise<void>;
  createProvider: (input: CreateSearchProviderInput) => Promise<void>;
  updateProvider: (id: string, input: UpdateSearchProviderInput) => Promise<void>;
  deleteProvider: (id: string) => Promise<void>;
  executeSearch: (providerId: string, query: string) => Promise<{ ok: boolean; results: SearchResultItem[] }>;
}

export interface SearchOptions {
  useRegex: boolean;
  caseSensitive: boolean;
  limit: number;
}

export interface SavedFilter {
  name: string;
  query: string;
  options: SearchOptions;
}

type SearchResultItem = {
  title: string;
  content: string;
  url: string;
};

export const useSearchStore = create<SearchState>()(
  persist(
    (set, get) => ({
      query: "",
      results: [],
      isSearching: false,
      error: null,
      recentSearches: [],
      savedFilters: [],
      searchOptions: {
        useRegex: false,
        caseSensitive: false,
        limit: 50,
      },

      setQuery: (query: string) => set({ query }),

      setSearchOptions: (options: Partial<SearchOptions>) =>
        set((state) => ({
          searchOptions: { ...state.searchOptions, ...options },
        })),

      search: async (searchQuery?: Partial<SearchQuery>) => {
        const { query, searchOptions } = get();
        if (!query.trim()) {
          set({ results: [], error: null });
          return;
        }

        set({ isSearching: true, error: null });

        try {
          const fullQuery: SearchQuery = {
            query,
            regex: searchQuery?.regex ?? searchOptions.useRegex,
            case_sensitive:
              searchQuery?.case_sensitive ?? searchOptions.caseSensitive,
            session_filter: searchQuery?.session_filter,
            date_from: searchQuery?.date_from,
            date_to: searchQuery?.date_to,
            limit: searchQuery?.limit ?? searchOptions.limit,
            offset: searchQuery?.offset ?? 0,
          };

          const results = await invoke<SearchResult[]>("session_search", {
            query: fullQuery,
          });

          set({ results, isSearching: false });
          get().addRecentSearch(query);
        } catch (e) {
          set({ error: String(e), isSearching: false, results: [] });
        }
      },

      clearResults: () => set({ results: [], error: null }),

      addRecentSearch: (query: string) => {
        const { recentSearches } = get();
        const filtered = recentSearches.filter((q) => q !== query);
        const updated = [query, ...filtered].slice(0, 10);
        set({ recentSearches: updated });
      },

      clearRecentSearches: () => set({ recentSearches: [] }),

      saveFilter: (filter: SavedFilter) => {
        const { savedFilters } = get();
        const existing = savedFilters.findIndex((f) => f.name === filter.name);
        if (existing >= 0) {
          const updated = [...savedFilters];
          updated[existing] = filter;
          set({ savedFilters: updated });
        } else {
          set({ savedFilters: [...savedFilters, filter] });
        }
      },

      deleteFilter: (name: string) => {
        const { savedFilters } = get();
        set({ savedFilters: savedFilters.filter((f) => f.name !== name) });
      },

      loadRecentSearches: () => {
        const stored = localStorage.getItem("axagent-recent-searches");
        if (stored) {
          try {
            set({ recentSearches: JSON.parse(stored) });
          } catch {
            localStorage.removeItem("axagent-recent-searches");
          }
        }
      },

      providers: [],

      loadProviders: async () => {
        try {
          const providers = await invoke<SearchProvider[]>("list_search_providers");
          set({ providers });
        } catch (e) {
          console.error("Failed to load search providers:", e);
        }
      },

      createProvider: async (input: CreateSearchProviderInput) => {
        const provider = await invoke<SearchProvider>("create_search_provider", { input });
        set((state) => ({ providers: [...state.providers, provider] }));
      },

      updateProvider: async (id: string, input: UpdateSearchProviderInput) => {
        const updated = await invoke<SearchProvider>("update_search_provider", { id, input });
        set((state) => ({
          providers: state.providers.map((p) => (p.id === id ? updated : p)),
        }));
      },

      deleteProvider: async (id: string) => {
        await invoke("delete_search_provider", { id });
        set((state) => ({
          providers: state.providers.filter((p) => p.id !== id),
        }));
      },

      executeSearch: async (providerId: string, query: string) => {
        const result = await invoke<{ ok: boolean; results: SearchResultItem[] }>("execute_search", {
          providerId,
          query,
        });
        return result;
      },
    }),
    {
      name: "axagent-search-storage",
      partialize: (state) => ({
        recentSearches: state.recentSearches,
        savedFilters: state.savedFilters,
        searchOptions: state.searchOptions,
      }),
    }
  )
);

import { invoke } from "@/lib/invoke";
import { create } from "zustand";

export interface SourceConfig {
  embeddingProvider?: string;
  embeddingDimensions?: number;
  retrievalThreshold?: number;
  retrievalTopK?: number;
}

export interface UnifiedSource {
  id: string;
  name: string;
  description?: string;
  containerType: string;
  embeddingProvider?: string;
  embeddingDimensions?: number;
  retrievalThreshold?: number;
  retrievalTopK?: number;
  iconType?: string;
  iconValue?: string;
  sortOrder: number;
  enabled: boolean;
}

export interface SourceRef {
  containerType: string;
  id: string;
}

export interface RagContextResult {
  context: string;
  totalResults: number;
  sources: Array<{
    sourceType: string;
    containerId: string;
    containerName: string;
    content: string;
    score: number;
  }>;
}

interface SourceState {
  sources: UnifiedSource[];
  loading: boolean;
  error: string | null;

  fetchSources: (containerTypes?: string[]) => Promise<void>;
  getSourceConfig: (containerType: string, containerId: string) => Promise<SourceConfig>;
  searchAllSources: (query: string, topK?: number) => Promise<RagContextResult>;
  getSourceName: (sourceRef: SourceRef) => string;
  getSourcesByType: (containerType: string) => UnifiedSource[];
  knowledgeSources: () => UnifiedSource[];
  memorySources: () => UnifiedSource[];
  wikiSources: () => UnifiedSource[];
  configuredSources: () => UnifiedSource[];
  sourceById: () => Map<string, UnifiedSource>;
}

export const useSourceStore = create<SourceState>((set, get) => ({
  sources: [],
  loading: false,
  error: null,

  fetchSources: async (containerTypes) => {
    set({ loading: true, error: null });
    try {
      const sources = await invoke<UnifiedSource[]>("list_all_sources", {
        containerTypes: containerTypes ?? null,
      });
      set({ sources, loading: false });
    } catch (e) {
      set({ error: String(e), loading: false });
    }
  },

  getSourceConfig: async (containerType, containerId) => {
    return invoke<SourceConfig>("get_source_config", { containerType, containerId });
  },

  searchAllSources: async (query, topK) => {
    return invoke<RagContextResult>("search_all_sources", { query, topK: topK ?? null });
  },

  getSourceName: (sourceRef) => {
    const source = get().sources.find((s) => s.id === sourceRef.id);
    return source?.name ?? sourceRef.id;
  },

  getSourcesByType: (containerType) => {
    return get().sources.filter((s) => s.containerType === containerType);
  },

  knowledgeSources: () => get().sources.filter((s) => s.containerType === "knowledge"),
  memorySources: () => get().sources.filter((s) => s.containerType === "memory"),
  wikiSources: () => get().sources.filter((s) => s.containerType === "wiki"),
  configuredSources: () => get().sources.filter((s) => s.embeddingProvider != null),
  sourceById: () => {
    const map = new Map<string, UnifiedSource>();
    for (const s of get().sources) {
      map.set(s.id, s);
    }
    return map;
  },
}));

// SPDX-License-Identifier: AGPL-3.0-only

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
  getSourceConfig: (
    containerType: string,
    containerId: string,
  ) => Promise<SourceConfig>;
  searchAllSources: (query: string, topK?: number) => Promise<RagContextResult>;
  getSourceName: (sourceRef: SourceRef) => string;
  getSourcesByType: (containerType: string) => UnifiedSource[];
  knowledgeSources: () => UnifiedSource[];
  memorySources: () => UnifiedSource[];
  wikiSources: () => UnifiedSource[];
  configuredSources: () => UnifiedSource[];
  sourceById: () => Map<string, UnifiedSource>;
  /** 修改数据源的向量模型；按 containerType 路由到对应后端命令 */
  updateSourceEmbedding: (
    source: Pick<UnifiedSource, "id" | "containerType" | "embeddingProvider">,
    newProvider: string | undefined,
  ) => Promise<{ embeddingChanged: boolean }>;
  /** 触发对应数据源类型的向量索引重建 */
  rebuildSourceIndex: (
    containerType: string,
    containerId: string,
  ) => Promise<void>;
  /** 删除数据源容器（含向量集合）；按 containerType 路由到对应后端命令 */
  deleteSource: (source: Pick<UnifiedSource, "id" | "containerType">) => Promise<void>;
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
      set({ sources: Array.isArray(sources) ? sources : [], loading: false });
    } catch (e) {
      set({ error: String(e), loading: false });
    }
  },

  getSourceConfig: async (containerType, containerId) => {
    return invoke<SourceConfig>("get_source_config", {
      containerType,
      containerId,
    });
  },

  searchAllSources: async (query, topK) => {
    return invoke<RagContextResult>("search_all_sources", {
      query,
      topK: topK ?? null,
    });
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

  updateSourceEmbedding: async (source, newProvider) => {
    const oldProvider = source.embeddingProvider;
    const embeddingChanged = (oldProvider ?? "") !== (newProvider ?? "");

    switch (source.containerType) {
      case "knowledge": {
        await invoke("update_knowledge_base", {
          id: source.id,
          input: {
            embeddingProvider: newProvider,
            updateEmbeddingProvider: true,
          },
        });
        break;
      }
      case "memory": {
        await invoke("update_memory_namespace", {
          id: source.id,
          input: {
            embeddingProvider: newProvider,
            updateEmbeddingProvider: true,
          },
        });
        break;
      }
      case "wiki": {
        await invoke("update_wiki", {
          id: source.id,
          // dao::repo::wiki::update_wiki 仅在 Some 时更新；传 undefined 保持原值
          embeddingProvider: newProvider,
        });
        break;
      }
      default:
        throw new Error(`Unsupported container type: ${source.containerType}`);
    }

    // 同步本地 sources 列表中的 embeddingProvider，避免下次读取仍是旧值
    set((state) => ({
      sources: state.sources.map((s) =>
        s.id === source.id
          ? { ...s, embeddingProvider: newProvider }
          : s
      ),
    }));

    return { embeddingChanged };
  },

  rebuildSourceIndex: async (containerType, containerId) => {
    switch (containerType) {
      case "knowledge":
        await invoke("rebuild_knowledge_index", { baseId: containerId });
        break;
      case "memory":
        await invoke("rebuild_memory_index", { namespaceId: containerId });
        break;
      case "wiki":
        await invoke("rebuild_wiki_index", { wikiId: containerId });
        break;
      default:
        throw new Error(`Unsupported container type: ${containerType}`);
    }
  },

  deleteSource: async (source) => {
    switch (source.containerType) {
      case "knowledge":
        await invoke("delete_knowledge_base", { id: source.id });
        break;
      case "memory":
        await invoke("delete_memory_namespace", { id: source.id });
        break;
      case "wiki":
        await invoke("delete_wiki", { id: source.id });
        break;
      default:
        throw new Error(`Unsupported container type: ${source.containerType}`);
    }
    // 同步本地 sources 缓存
    set((state) => ({
      sources: state.sources.filter((s) => s.id !== source.id),
    }));
  },
}));

// SPDX-License-Identifier: AGPL-3.0-only

// LightRAG 知识图谱 Store
// 对接后端 commands::knowledge_graph 模块

import { invoke } from "@/lib/invoke";
import type {
  ExtractedEntity,
  ExtractedRelation,
  ExtractEntitiesResult,
  GraphEnhancedSearchInput,
  GraphEnhancedSearchResult,
} from "@/types";
import { create } from "zustand";

interface KnowledgeGraphState {
  /** 最近一次图查询增强结果 */
  searchResult: GraphEnhancedSearchResult | null;
  /** 最近一次跨文档抽取结果 */
  extractResult: ExtractEntitiesResult | null;
  loading: boolean;
  error: string | null;

  /** 图查询增强搜索 */
  graphEnhancedSearch: (input: GraphEnhancedSearchInput) => Promise<GraphEnhancedSearchResult | null>;
  /** 跨文档抽取实体与关系并写入知识图谱 */
  extractEntitiesFromDocuments: (
    knowledgeBaseId: string,
    documentIds: string[],
  ) => Promise<ExtractEntitiesResult | null>;
  /** 批量 upsert 实体与关系（保留入口，便于后续 UI 调用） */
  batchUpsertEntitiesAndRelations: (
    knowledgeBaseId: string,
    entities: ExtractedEntity[],
    relations: ExtractedRelation[],
  ) => Promise<ExtractEntitiesResult | null>;
  /** 清空搜索结果 */
  clearResults: () => void;
  /** 清空错误状态 */
  clearError: () => void;
}

export const useKnowledgeGraphStore = create<KnowledgeGraphState>((set) => ({
  searchResult: null,
  extractResult: null,
  loading: false,
  error: null,

  graphEnhancedSearch: async (input) => {
    set({ loading: true, error: null });
    try {
      const result = await invoke<GraphEnhancedSearchResult>("graph_enhanced_search", { input });
      set({ searchResult: result, loading: false });
      return result;
    } catch (e) {
      set({ error: String(e), loading: false });
      return null;
    }
  },

  extractEntitiesFromDocuments: async (knowledgeBaseId, documentIds) => {
    set({ loading: true, error: null });
    try {
      const result = await invoke<ExtractEntitiesResult>("extract_entities_from_documents", {
        knowledgeBaseId,
        documentIds,
      });
      set({ extractResult: result, loading: false });
      return result;
    } catch (e) {
      // 后端尚未实现，预期返回错误
      set({ error: String(e), loading: false });
      throw e;
    }
  },

  batchUpsertEntitiesAndRelations: async (knowledgeBaseId, entities, relations) => {
    set({ loading: true, error: null });
    try {
      const result = await invoke<ExtractEntitiesResult>("batch_upsert_entities_and_relations", {
        knowledgeBaseId,
        entities,
        relations,
      });
      set({ extractResult: result, loading: false });
      return result;
    } catch (e) {
      set({ error: String(e), loading: false });
      return null;
    }
  },

  clearResults: () => {
    set({ searchResult: null, extractResult: null, error: null });
  },

  clearError: () => {
    set({ error: null });
  },
}));

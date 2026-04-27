import type { MemoryScope, MemorySource } from "./knowledge";

export type MemoryNamespace = {
  id: string;
  name: string;
  scope: MemoryScope;
  embeddingProvider?: string;
  embeddingDimensions?: number;
  retrievalThreshold?: number;
  retrievalTopK?: number;
  iconType?: string;
  iconValue?: string;
  sortOrder: number;
};

export type MemoryItem = {
  id: string;
  namespaceId: string;
  title: string;
  content: string;
  source: MemorySource;
  indexStatus: string; // pending | indexing | ready | failed | skipped
  indexError?: string;
  updatedAt: string;
};

export type CreateMemoryNamespaceInput = {
  name: string;
  scope: MemoryScope;
  embeddingProvider?: string;
  embeddingDimensions?: number;
  retrievalThreshold?: number;
  retrievalTopK?: number;
};

export type CreateMemoryItemInput = {
  namespaceId: string;
  title: string;
  content: string;
  source?: MemorySource;
};

export type UpdateMemoryItemInput = {
  title?: string;
  content?: string;
};

export type UpdateMemoryNamespaceInput = {
  name?: string;
  embeddingProvider?: string;
  updateEmbeddingProvider?: boolean;
  embeddingDimensions?: number;
  updateEmbeddingDimensions?: boolean;
  retrievalThreshold?: number;
  updateRetrievalThreshold?: boolean;
  retrievalTopK?: number;
  updateRetrievalTopK?: boolean;
  iconType?: string;
  iconValue?: string;
  updateIcon?: boolean;
  sortOrder?: number;
};

// SPDX-License-Identifier: AGPL-3.0-only

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

export type MemoryTier = "short_term" | "working" | "long_term" | "core";
export type MemoryNature = "episodic" | "semantic";

export type MemoryItem = {
  id: string;
  namespaceId: string;
  title: string;
  content: string;
  source: MemorySource;
  indexStatus: string;
  indexError?: string;
  // 以下字段后端 memory_items 表当前未持久化，可能不返回（见 memory_items 实体）。
  // 标记为可选以如实反映契约，UI 需对 undefined 做降级渲染。
  tier?: MemoryTier;
  importance?: number;
  nature?: MemoryNature;
  tags?: string[];
  accessCount?: number;
  expiresAt?: string;
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

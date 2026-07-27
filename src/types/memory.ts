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
  // 三层记忆系统：v101 已持久化，DTO 同步暴露（必填，后端总有默认值）
  tier: MemoryTier;
  importance: number;
  accessCount: number;
  lastAccessed?: number;
  decayRate: number;
  expiresAt?: number;
  memoryNature: MemoryNature;
  tags: string[];
  sourceConversationId?: string;
  sourceMessageId?: string;
  updatedAt: string;
  // v108: 自进化闭环 — 记忆适用范围边界 + 人工确认门
  applicabilityTags: string[];
  confirmed: number;
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
  // 三层记忆系统：创建时可选指定
  tier?: MemoryTier;
  importance?: number;
  memoryNature?: MemoryNature;
  tags?: string[];
  decayRate?: number;
  expiresAt?: number;
  // v108: 自进化闭环 — 创建时可选指定适用范围与确认状态
  applicabilityTags?: string[];
  confirmed?: number;
};

export type UpdateMemoryItemInput = {
  title?: string;
  content?: string;
  // 三层记忆系统：更新时可选调整
  tier?: MemoryTier;
  importance?: number;
  memoryNature?: MemoryNature;
  tags?: string[];
  // v108: 自进化闭环 — 更新时可选调整适用范围
  applicabilityTags?: string[];
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

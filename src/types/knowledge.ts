// SPDX-License-Identifier: AGPL-3.0-only

export type IndexingStatus = "pending" | "indexing" | "ready" | "failed";
export type MemoryScope = "global" | "project";
export type MemorySource = "manual" | "auto_extract";

/**
 * 知识库类型：
 * - `indexed`: 默认，KB 内容存于本地 data 目录，走 RAG 索引
 * - `connected_vault`: 指针型，指向外部 Obsidian vault，agent 通过 9 个 obsidian_* 工具直接读写 live 文件
 * - `connected_linked` / `connected_subagent`: 保留枚举位，本期不实现
 */
export type KbKind =
  | "indexed"
  | "connected_vault"
  | "connected_linked"
  | "connected_subagent";

export type KnowledgeBase = {
  id: string;
  name: string;
  description?: string;
  embeddingProvider?: string;
  enabled: boolean;
  iconType?: string;
  iconValue?: string;
  sortOrder: number;
  embeddingDimensions?: number;
  retrievalThreshold?: number;
  retrievalTopK?: number;
  chunkSize?: number;
  chunkOverlap?: number;
  separator?: string;
  /** 知识库类型，默认 `indexed`；`connected_vault` 时通过 obsidian_* 工具直接读写 vault */
  kind?: KbKind;
  /** ConnectedVault 类型时的 vault 根路径（绝对路径），其他类型为 undefined */
  vaultPath?: string;
};

export type KnowledgeDocument = {
  id: string;
  knowledgeBaseId: string;
  title: string;
  sourcePath: string;
  mimeType: string;
  sizeBytes: number;
  indexingStatus: IndexingStatus;
  docType: string;
  indexError?: string;
  sourceConversationId?: string;
  // 后端 KnowledgeDocumentDto 实际返回（repo_dtos.rs），此前缺失
  createdAt?: number;
  updatedAt?: number;
};

export type ImportDirectoryError = {
  path: string;
  error: string;
};

export type ImportDirectoryResult = {
  baseId: string;
  importedCount: number;
  skippedCount: number;
  errorCount: number;
  imported: KnowledgeDocument[];
  skipped: string[];
  errors: ImportDirectoryError[];
};

export type RetrievalHit = {
  id: string;
  conversationId: string;
  messageId: string;
  knowledgeBaseId: string;
  documentId: string;
  chunkRef: string;
  score: number;
  preview: string;
};

export type CreateKnowledgeBaseInput = {
  name: string;
  description?: string;
  embeddingProvider?: string;
  enabled?: boolean;
  /** KB 类型，默认 `indexed` */
  kind?: KbKind;
  /** ConnectedVault 类型时的 vault 根路径（绝对路径） */
  vaultPath?: string;
};

export type UpdateKnowledgeBaseInput = Partial<CreateKnowledgeBaseInput> & {
  iconType?: string | null;
  iconValue?: string | null;
  updateIcon?: boolean;
  embeddingDimensions?: number;
  updateEmbeddingDimensions?: boolean;
  retrievalThreshold?: number;
  updateRetrievalThreshold?: boolean;
  retrievalTopK?: number;
  updateRetrievalTopK?: boolean;
  chunkSize?: number;
  updateChunkSize?: boolean;
  chunkOverlap?: number;
  updateChunkOverlap?: boolean;
  separator?: string;
  updateSeparator?: boolean;
};

// ── RAG Pipeline Config ───────────────────────────────────

export type EnhancementConfig = {
  enabled: boolean;
  strategy: "none" | "hyde" | "multi_query" | "decomposition" | "auto";
  maxVariants: number;
  combinedCall: boolean;
};

export type RerankConfig = {
  enabled: boolean;
  backend: "rule" | "cross_encoder" | "pipeline";
  crossEncoderModel: string | null;
  topN: number;
  candidateK: number;
  ruleFilterKeep: number;
  scoreThreshold: number | null;
  ollamaEndpoint: string | null;
};

export type SelfRagConfig = {
  enabled: boolean;
  judgeModel: string;
  ollamaEndpoint: string;
  relevanceThreshold: number;
  qualityThreshold: number;
  maxRetryRounds: number;
};

export type RAGPipelineConfig = {
  queryEnhancement: EnhancementConfig;
  rerank: RerankConfig;
  selfRag: SelfRagConfig;
};

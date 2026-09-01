// SPDX-License-Identifier: AGPL-3.0-only

export type Wiki = {
  id: string;
  name: string;
  rootPath: string;
  schemaVersion: string;
  description?: string;
  noteCount: number;
  sourceCount: number;
  createdAt: number;
  updatedAt: number;
  // 后端 Wiki 实际返回（rag_voice_etc.rs），此前缺失导致读取恒为 undefined
  embeddingProvider?: string;
  embeddingDimensions?: number;
  retrievalThreshold?: number;
  retrievalTopK?: number;
  // v118: 关联的知识库 ID，与后端 Wiki DTO 的 knowledge_base_id 对齐
  knowledgeBaseId?: string;
};

export type WikiSource = {
  id: string;
  wikiId: string;
  sourceType: string;
  sourcePath: string;
  title: string;
  mimeType: string;
  sizeBytes: number;
  contentHash: string;
  metadataJson?: Record<string, unknown>;
  createdAt: number;
  updatedAt: number;
};

// ── Ingest / Query 结果类型（此前定义在 store 内，统一收归 @/types） ──

export interface IngestResult {
  sourceId: string;
  rawPath: string;
  title: string;
  /** 后台批量索引的笔记总数，结合 wiki-note-indexed 事件计算真实进度 */
  generatedNoteCount: number;
}

export interface QueryResult {
  pages: PageResult[];
  total: number;
}

export interface PageResult {
  noteId: string;
  title: string;
  contentSnippet: string;
  relevanceScore: number;
  linkPaths: string[];
}

export type WikiPage = {
  id: string;
  wikiId: string;
  noteId: string;
  pageType: string;
  title: string;
  sourceIds?: string[];
  qualityScore?: number;
  lastLintedAt?: number;
  lastCompiledAt?: number;
  compiledSourceHash?: string;
  createdAt: number;
  updatedAt: number;
};

export type WikiOperation = {
  id: number;
  wikiId: string;
  operationType: string;
  targetType: string;
  targetId: string;
  status: string;
  detailsJson?: Record<string, unknown>;
  errorMessage?: string;
  createdAt: number;
  completedAt?: number;
};

export type CompileResult = {
  newPages: CompiledPage[];
  updatedPages: CompiledPage[];
  errors: string[];
};

export type CompiledPage = {
  title: string;
  content: string;
  pageType: string;
  sourceIds: string[];
};

export type LintResult = {
  noteId: string;
  issues: LintIssue[];
  score: number;
};

export type LintIssue = {
  severity: "Error" | "Warning" | "Info";
  code: string;
  message: string;
  line?: number;
};

export type IngestSourceInput = {
  wikiId: string;
  sourcePath: string;
  sourceType: string;
};

export type CompileInput = {
  wikiId: string;
  sourceIds: string[];
};

export type QueryInput = {
  wikiId: string;
  query: string;
  limit?: number;
};

export type SchemaVersion = {
  id: string;
  wikiId: string;
  version: string;
  schema: Record<string, unknown>;
  description?: string;
  createdAt: number;
};

export type ValidationReport = {
  wikiId: string;
  totalNotes: number;
  consistentNotes: number;
  issues: ValidationIssue[];
  checkedAt: number;
};

export type ValidationIssue = {
  noteId: string;
  title: string;
  issueType:
    | "HashMismatch"
    | "MissingInDatabase"
    | "MissingInFilesystem"
    | "OrphanInVectorStore";
  message: string;
};

export type SyncQueueItem = {
  id: string;
  wikiId: string;
  eventType: string;
  payload: Record<string, unknown>;
  status: "pending" | "processing" | "completed" | "failed";
  retryCount: number;
  createdAt: number;
  processedAt?: number;
};

export type CapacityInfo = {
  totalChunks: number;
  maxChunks: number;
  usagePercent: number;
  wikiChunkCounts: Record<string, number>;
};

export type FolderImportPreviewItem = {
  fileName: string;
  filePath: string;
  folderContext: string;
  fileType: string;
  estimatedSize: number;
};

export type FolderImportResult = {
  taskIds: string[];
  importedCount: number;
  failedFiles: string[];
};

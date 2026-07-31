// SPDX-License-Identifier: AGPL-3.0-only

// ── 统一知识源类型 ─────────────────────────────────────────

/** 知识源类型 */
export type KnowledgeSourceType =
  | "knowledge_base"
  | "wiki"
  | "memory"
  | "obsidian_vault";

/** 统一搜索结果 */
export type UnifiedSearchResult = {
  sourceType: KnowledgeSourceType;
  sourceId: string;
  contentId: string;
  title: string;
  snippet: string;
  score: number;
  contentType: string;
};

/** 统一搜索请求 */
export type UnifiedSearchRequest = {
  /** 源类型过滤（不填 = 全部源） */
  sourceType?: KnowledgeSourceType;
  /** 源 ID（kb_id / wiki_id / namespace_id / vault_id） */
  sourceId?: string;
  /** 查询文本 */
  query: string;
  /** 最多返回结果数（默认 10） */
  topK?: number;
};

/** 知识源元数据 */
export type KnowledgeSourceMeta = {
  sourceType: KnowledgeSourceType;
  sourceId: string;
  name: string;
  itemCount: number;
  lastUpdatedAt?: number;
};

// ── 反馈数据湖类型 ─────────────────────────────────────────

/** 反馈事件类型 */
export type FeedbackEventType =
  | "retrieval_hit"
  | "tool_call"
  | "memory_access"
  | "wiki_edit";

/** 统一反馈事件 */
export type FeedbackEvent = {
  id: string;
  eventType: FeedbackEventType;
  conversationId?: string;
  messageId?: string;
  userId?: string;
  sessionId?: string;
  sourceId?: string;
  sourceType?: string;
  payload: Record<string, unknown>;
  createdAt: number;
};

/** 反馈查询请求 */
export type FeedbackQueryRequest = {
  eventTypes?: FeedbackEventType[];
  conversationId?: string;
  sourceId?: string;
  sourceType?: string;
  startTime?: number;
  endTime?: number;
  limit?: number;
  offset?: number;
};

/** 检索命中记录 */
export type RetrievalHitRecord = {
  id: string;
  conversationId: string;
  messageId: string;
  knowledgeBaseId: string;
  documentId: string;
  chunkRef: string;
  score: number;
  preview: string;
  feedback?: string;
  feedbackAt?: number;
  usedInResponse: boolean;
  scoreAfterRerank?: number;
  createdAt: number;
};

/** 工具调用记录 */
export type ToolCallRecord = {
  id: string;
  conversationId?: string;
  trajectoryId?: string;
  stepIndex: number;
  toolName: string;
  arguments: Record<string, unknown>;
  result?: Record<string, unknown>;
  success: boolean;
  durationMs: number;
  relatedSourceId?: string;
  createdAt: number;
};

/** 记忆访问记录 */
export type MemoryAccessRecord = {
  id: string;
  conversationId?: string;
  namespaceId: string;
  memoryId: string;
  accessType: string;
  query?: string;
  contentSnippet?: string;
  hit: boolean;
  createdAt: number;
};

/** Wiki 编辑记录 */
export type WikiEditRecord = {
  id: string;
  conversationId?: string;
  wikiId: string;
  noteId: string;
  operation: string;
  beforeSnippet?: string;
  afterSnippet?: string;
  reason?: string;
  qualityScore?: number;
  createdAt: number;
};

/** 反馈统计 */
export type FeedbackStats = {
  /** 总反馈数 */
  totalCount: number;
  /** 按类型分组的数量 */
  byType: Record<string, number>;
  /** 正反馈率 (0.0 - 1.0) */
  positiveRate: number;
};

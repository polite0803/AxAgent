// SPDX-License-Identifier: AGPL-3.0-only

// 论文概览 / 阅读列表 / 论文问答 / LightRAG 知识图谱 前端类型定义
// 与后端 commands::paper_overview / commands::reading_list / commands::paper_qa / commands::knowledge_graph 对齐

// ── Paper Overview ────────────────────────────────────────────────────────

/** 论文概览章节结构 */
export interface PaperOverviewSection {
  title: string;
  summary: string;
}

/** 论文概览 */
export interface PaperOverview {
  id: string;
  documentId: string;
  knowledgeBaseId: string;
  overviewType: string;
  abstractText?: string;
  keyConcepts: string[];
  methods: string[];
  contributions: string[];
  limitations: string[];
  tlDr?: string;
  sections: PaperOverviewSection[];
  metadata: unknown;
  generatedBy?: string;
  createdAt: number;
  updatedAt: number;
}

/** 创建论文概览入参 */
export interface CreatePaperOverviewInput {
  documentId: string;
  knowledgeBaseId: string;
  overviewType?: string;
  abstractText?: string;
  keyConcepts: string[];
  methods: string[];
  contributions: string[];
  limitations: string[];
  tlDr?: string;
  sections: PaperOverviewSection[];
  metadata?: unknown;
  generatedBy?: string;
}

/** 更新论文概览入参（字段可选，null 表示显式清空） */
export interface UpdatePaperOverviewInput {
  abstractText?: string | null;
  keyConcepts?: string[];
  methods?: string[];
  contributions?: string[];
  limitations?: string[];
  tlDr?: string | null;
  sections?: PaperOverviewSection[];
  metadata?: unknown;
}

// ── Reading List ───────────────────────────────────────────────────────────

/** 阅读列表 */
export interface ReadingList {
  id: string;
  name: string;
  description?: string;
  ownerUserId?: string;
  status: string;
  sortOrder: number;
  createdAt: number;
  updatedAt: number;
}

/** 创建阅读列表入参 */
export interface CreateReadingListInput {
  name: string;
  description?: string;
  ownerUserId?: string;
}

/** 更新阅读列表入参 */
export interface UpdateReadingListInput {
  name?: string;
  description?: string | null;
  status?: string;
  sortOrder?: number;
}

/** 阅读列表条目 */
export interface ReadingListItem {
  id: string;
  readingListId: string;
  documentId?: string;
  externalUrl?: string;
  title: string;
  notes?: string;
  /** unread / reading / read / skipped */
  status: string;
  priority: number;
  position: number;
  metadata: unknown;
  addedAt: number;
  updatedAt: number;
}

/** 创建阅读列表条目入参 */
export interface CreateReadingListItemInput {
  readingListId: string;
  documentId?: string;
  externalUrl?: string;
  title: string;
  notes?: string;
  priority?: number;
  position?: number;
  metadata?: unknown;
}

/** 更新阅读列表条目入参 */
export interface UpdateReadingListItemInput {
  title?: string;
  notes?: string | null;
  status?: string;
  priority?: number;
  position?: number;
  metadata?: unknown;
}

// ── Paper QA Pipeline ──────────────────────────────────────────────────────

/** 论文 QA 准备上下文中的检索分块 */
export interface PaperQAChunk {
  id: string;
  documentId: string;
  chunkIndex: number;
  content: string;
  score: number;
  hasEmbedding: boolean;
}

/** 论文 QA 准备好的上下文 */
export interface PaperQAPreparedContext {
  overview?: PaperOverview;
  chunks: PaperQAChunk[];
  contextText: string;
  suggestedPrompt: string;
  knowledgeBaseId: string;
  documentTitle: string;
}

// ── LightRAG 知识图谱 ──────────────────────────────────────────────────────

/** 图查询增强入参 */
export interface GraphEnhancedSearchInput {
  knowledgeBaseId: string;
  query: string;
  topK?: number;
  includeNeighbors?: boolean;
}

/** 图查询关系边 */
export interface GraphRelationEdge {
  targetEntityName: string;
  relationType: string;
  description?: string;
  weight: number;
}

/** 图查询返回的实体 */
export interface GraphEnhancedContextChunk {
  entityName: string;
  entityType: string;
  description?: string;
  relations: GraphRelationEdge[];
  knowledgeBaseId: string;
}

/** 图查询增强结果 */
export interface GraphEnhancedSearchResult {
  entities: GraphEnhancedContextChunk[];
  contextText: string;
  totalHits: number;
}

/** 抽取得到的实体 */
export interface ExtractedEntity {
  entityName: string;
  entityType: string;
  description?: string;
  knowledgeBaseId?: string;
}

/** 抽取得到的关系 */
export interface ExtractedRelation {
  sourceEntityName: string;
  targetEntityName: string;
  relationType: string;
  description?: string;
  weight?: number;
  knowledgeBaseId?: string;
}

/** 已写入知识库的实体（对应后端 `KnowledgeEntity`，camelCase 序列化） */
export interface KnowledgeEntity {
  id: string;
  knowledgeBaseId: string;
  name: string;
  entityType: string;
  description?: string | null;
  sourcePath: string;
  sourceLanguage?: string | null;
  properties: Record<string, unknown>;
  lifecycle?: Record<string, unknown> | null;
  behaviors?: Record<string, unknown> | null;
  metadata?: Record<string, unknown> | null;
  createdAt: number;
  updatedAt: number;
  aliases: string;
  mentionCount: number;
  confidence: number;
  firstSeenAt?: string | null;
  lastSeenAt?: string | null;
}

/** 已写入知识库的关系（对应后端 `KnowledgeRelation`，camelCase 序列化） */
export interface KnowledgeRelation {
  id: string;
  knowledgeBaseId: string;
  sourceEntityId: string;
  targetEntityId: string;
  relationType: string;
  description?: string | null;
  properties?: Record<string, unknown> | null;
  metadata?: Record<string, unknown> | null;
  createdAt: number;
  updatedAt: number;
  weight: number;
}

/** 实体关系抽取结果（对应后端 `ExtractEntitiesResult`，camelCase 序列化） */
export interface ExtractEntitiesResult {
  newEntities: KnowledgeEntity[];
  updatedEntities: KnowledgeEntity[];
  newRelations: KnowledgeRelation[];
  skippedChunks: number;
  elapsedMs: number;
}

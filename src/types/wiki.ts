// SPDX-License-Identifier: AGPL-3.0-only

export type Note = {
  id: string;
  vaultId: string;
  title: string;
  filePath: string;
  content: string;
  contentHash: string;
  author: string;
  pageType?: string;
  sourceRefs?: string[];
  relatedPages?: string[];
  qualityScore?: number;
  lastLintedAt?: number;
  lastCompiledAt?: number;
  compiledSourceHash?: string;
  userEdited: boolean;
  userEditedAt?: number;
  createdAt: number;
  updatedAt: number;
  isDeleted: boolean;
};

export type CreateNoteInput = {
  vaultId: string;
  title: string;
  filePath: string;
  content: string;
  author: string;
  pageType?: string;
  sourceRefs?: string[];
};

export type UpdateNoteInput = {
  title?: string;
  content?: string;
  pageType?: string;
  relatedPages?: string[];
};

export type NoteLink = {
  id: number;
  vaultId: string;
  sourceNoteId: string;
  targetNoteId: string;
  linkText: string;
  linkType: string;
  createdAt: number;
};

export type BacklinkInfo = {
  noteId: string;
  title: string;
  snippets: string[];
};

export type NoteSearchResult = {
  note: Note;
  snippet: string;
  score: number;
};

export type NoteVersion = {
  id: number;
  wikiId: string;
  noteId: string;
  title: string;
  content: string;
  contentHash: string;
  author: string;
  createdAt: number;
};

export type WikiTemplate = {
  id: string;
  wikiId: string;
  name: string;
  description?: string;
  content: string;
  pageType?: string;
  isBuiltin: boolean;
  createdAt: number;
  updatedAt: number;
};

export type CreateWikiTemplateInput = {
  wikiId: string;
  name: string;
  description?: string;
  content: string;
  pageType?: string;
  isBuiltin: boolean;
};

export type ImportStats = {
  imported: number;
  failed: number;
  skipped: number;
};

export type KnowledgeMdImportStats = {
  imported: number;
  failed: number;
  skipped: number;
  total: number;
};

export type ExportStats = {
  exported: number;
  failed: number;
};

// ── 知识源（Knowledge Source）— 知识库增长更新入口 ────────────────

export type KnowledgeSource = {
  id: string;
  wikiId: string;
  sourceType: string;
  sourcePath: string;
  title: string;
  mimeType: string;
  sizeBytes: number;
  contentHash: string;
  metadataJson?: Record<string, unknown>;
  scheduleCron?: string;
  lastFetchedAt?: number;
  status: string;
  createdAt: number;
  updatedAt: number;
};

export type CreateKnowledgeSourceInput = {
  wikiId: string;
  sourceType: string;
  sourcePath: string;
  title: string;
  mimeType?: string;
  scheduleCron?: string;
  status?: string;
  metadataJson?: Record<string, unknown>;
};

export type UpdateKnowledgeSourceInput = {
  id: string;
  title?: string;
  sourceType?: string;
  sourcePath?: string;
  scheduleCron?: string;
  clearSchedule?: boolean;
  status?: string;
  metadataJson?: Record<string, unknown>;
};

export type FetchUrlResult = {
  noteId: string;
  title: string;
  /** created | updated | skipped */
  action: string;
  sourceId?: string;
};

export type FetchSourceResult = {
  sourceId: string;
  sourceTitle: string;
  /** created | updated | skipped | error */
  action: string;
  detail: string;
};

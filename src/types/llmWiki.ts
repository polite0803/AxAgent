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
  new_pages: CompiledPage[];
  updated_pages: CompiledPage[];
  errors: string[];
};

export type CompiledPage = {
  title: string;
  content: string;
  page_type: string;
  source_ids: string[];
};

export type LintResult = {
  note_id: string;
  issues: LintIssue[];
  score: number;
};

export type LintIssue = {
  severity: 'Error' | 'Warning' | 'Info';
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
  issueType: 'HashMismatch' | 'MissingInDatabase' | 'MissingInFilesystem' | 'OrphanInVectorStore';
  message: string;
};

export type SyncQueueItem = {
  id: string;
  wikiId: string;
  eventType: string;
  payload: Record<string, unknown>;
  status: 'pending' | 'processing' | 'completed' | 'failed';
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
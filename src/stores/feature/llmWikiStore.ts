import { invoke } from '@/lib/invoke';
import { create } from 'zustand';

export interface Wiki {
  id: string;
  name: string;
  description?: string;
  rootPath: string;
  schemaVersion: string;
  noteCount: number;
  sourceCount: number;
  createdAt: number;
  updatedAt: number;
}

export interface WikiSource {
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
}

export interface WikiPage {
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
}

export interface WikiOperation {
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
}

export interface IngestResult {
  source_id: string;
  raw_path: string;
  title: string;
}

export interface CompiledPage {
  title: string;
  content: string;
  page_type: string;
  source_ids: string[];
}

export interface CompileResult {
  new_pages: CompiledPage[];
  updated_pages: CompiledPage[];
  errors: string[];
}

export interface QueryResult {
  pages: PageResult[];
  total: number;
}

export interface PageResult {
  note_id: string;
  title: string;
  content_snippet: string;
  relevance_score: number;
  link_paths: string[];
}

export interface LintIssue {
  severity: 'Error' | 'Warning' | 'Info';
  code: string;
  message: string;
  line?: number;
}

export interface LintResult {
  note_id: string;
  issues: LintIssue[];
  score: number;
}

export interface SchemaVersion {
  version: string;
  created_at: number;
  content_hash: string;
}

interface LlmWikiState {
  wikis: Wiki[];
  selectedWikiId: string | null;
  sources: WikiSource[];
  pages: WikiPage[];
  operations: WikiOperation[];
  loading: boolean;
  error: string | null;

  loadWikis: () => Promise<void>;
  selectWiki: (wikiId: string | null) => void;
  createWiki: (name: string, rootPath: string, description?: string) => Promise<Wiki | null>;
  deleteWiki: (wikiId: string) => Promise<void>;

  ingestSource: (
    wikiId: string,
    sourceType: string,
    path: string,
    url?: string,
    title?: string
  ) => Promise<IngestResult | null>;
  compileWiki: (wikiId: string, sourceIds: string[]) => Promise<CompileResult | null>;
  queryWiki: (
    wikiId: string,
    query: string,
    limit?: number,
    offset?: number
  ) => Promise<QueryResult | null>;

  lintNote: (noteId: string) => Promise<LintResult | null>;
  updateLintScore: (noteId: string) => Promise<number | null>;

  getSchema: (wikiId: string) => Promise<string | null>;
  validateFrontmatter: (
    wikiId: string,
    frontmatter: Record<string, unknown>
  ) => Promise<string[] | null>;
  createSchemaVersion: (
    wikiId: string,
    version: string,
    description?: string
  ) => Promise<SchemaVersion | null>;

  loadOperations: (wikiId: string) => Promise<void>;

  updateSchema: (wikiId: string, content: string) => Promise<void>;
  deleteSchema: (wikiId: string) => Promise<void>;
  lintVault: (wikiId: string) => Promise<LintResult[] | null>;
  autoFix: (wikiId: string, noteId?: string) => Promise<string[] | null>;
  askQuestion: (wikiId: string, question: string) => Promise<string | null>;
  processSyncPending: (wikiId: string) => Promise<number | null>;
}

export const useLlmWikiStore = create<LlmWikiState>((set) => ({
  wikis: [],
  selectedWikiId: null,
  sources: [],
  pages: [],
  operations: [],
  loading: false,
  error: null,

  loadWikis: async () => {
    set({ loading: true, error: null });
    try {
      const wikis = await invoke<Wiki[]>('llm_wiki_list', {});
      set({ wikis, loading: false });
    } catch (e) {
      set({ error: String(e), loading: false });
    }
  },

  selectWiki: (wikiId) => {
    set({ selectedWikiId: wikiId, sources: [], pages: [] });
  },

  createWiki: async (name, rootPath, description) => {
    try {
      const wiki = await invoke<Wiki>('llm_wiki_create', { name, rootPath, description });
      set((s) => ({ wikis: [...s.wikis, wiki] }));
      return wiki;
    } catch (e) {
      set({ error: String(e) });
      return null;
    }
  },

  deleteWiki: async (wikiId) => {
    try {
      await invoke('llm_wiki_delete', { wikiId });
      set((s) => ({
        wikis: s.wikis.filter((w) => w.id !== wikiId),
        selectedWikiId: s.selectedWikiId === wikiId ? null : s.selectedWikiId,
      }));
    } catch (e) {
      set({ error: String(e) });
    }
  },

  ingestSource: async (wikiId, sourceType, path, url, title) => {
    try {
      const result = await invoke<IngestResult>('llm_wiki_ingest', {
        input: { wiki_id: wikiId, source_type: sourceType, path, url, title },
      });
      return result;
    } catch (e) {
      set({ error: String(e) });
      return null;
    }
  },

  compileWiki: async (wikiId, sourceIds) => {
    try {
      const result = await invoke<CompileResult>('llm_wiki_compile', {
        input: { wiki_id: wikiId, source_ids: sourceIds },
      });
      return result;
    } catch (e) {
      set({ error: String(e) });
      return null;
    }
  },

  queryWiki: async (wikiId, query, limit, offset) => {
    try {
      const result = await invoke<QueryResult>('llm_wiki_query', {
        input: { wiki_id: wikiId, query, limit, offset },
      });
      return result;
    } catch (e) {
      set({ error: String(e) });
      return null;
    }
  },

  lintNote: async (noteId) => {
    try {
      return await invoke<LintResult>('llm_wiki_lint', { noteId });
    } catch (e) {
      set({ error: String(e) });
      return null;
    }
  },

  updateLintScore: async (noteId) => {
    try {
      return await invoke<number>('llm_wiki_lint_update_score', { noteId });
    } catch (e) {
      set({ error: String(e) });
      return null;
    }
  },

  getSchema: async (wikiId) => {
    try {
      return await invoke<string>('llm_wiki_get_schema', { wikiId });
    } catch (e) {
      set({ error: String(e) });
      return null;
    }
  },

  validateFrontmatter: async (wikiId, frontmatter) => {
    try {
      return await invoke<string[]>('llm_wiki_validate_frontmatter', {
        input: { wiki_id: wikiId, frontmatter },
      });
    } catch (e) {
      set({ error: String(e) });
      return null;
    }
  },

  createSchemaVersion: async (wikiId, version, description) => {
    try {
      return await invoke<SchemaVersion>('llm_wiki_create_schema_version', {
        wikiId,
        version,
        description,
      });
    } catch (e) {
      set({ error: String(e) });
      return null;
    }
  },

  loadOperations: async (wikiId) => {
    try {
      const operations = await invoke<WikiOperation[]>('llm_wiki_operations_list', { wikiId });
      set({ operations });
    } catch (e) {
      set({ error: String(e) });
    }
  },

  updateSchema: async (wikiId, content) => {
    try {
      await invoke('llm_wiki_update_schema', { wikiId, content });
    } catch (e) {
      set({ error: String(e) });
    }
  },

  deleteSchema: async (wikiId) => {
    try {
      await invoke('llm_wiki_delete_schema', { wikiId });
    } catch (e) {
      set({ error: String(e) });
    }
  },

  lintVault: async (wikiId) => {
    try {
      return await invoke<LintResult[]>('llm_wiki_lint_vault', { wikiId });
    } catch (e) {
      set({ error: String(e) });
      return null;
    }
  },

  autoFix: async (wikiId, noteId) => {
    try {
      return await invoke<string[]>('llm_wiki_auto_fix', { wikiId, noteId });
    } catch (e) {
      set({ error: String(e) });
      return null;
    }
  },

  askQuestion: async (wikiId, question) => {
    try {
      return await invoke<string>('llm_wiki_ask', { wikiId, question });
    } catch (e) {
      set({ error: String(e) });
      return null;
    }
  },

  processSyncPending: async (wikiId) => {
    try {
      return await invoke<number>('wiki_sync_process_pending', { wikiId });
    } catch (e) {
      set({ error: String(e) });
      return null;
    }
  },
}));
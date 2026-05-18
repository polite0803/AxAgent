import { invoke } from "@/lib/invoke";
import type {
  BacklinkInfo,
  CreateNoteInput,
  CreateWikiTemplateInput,
  ExportStats,
  ImportStats,
  Note,
  NoteLink,
  NoteSearchResult,
  NoteVersion,
  UpdateNoteInput,
  WikiTemplate,
} from "@/types";
import { create } from "zustand";

interface WikiState {
  notes: Note[];
  selectedNoteId: string | null;
  selectedVaultId: string | null;
  loading: boolean;
  error: string | null;
  templates: WikiTemplate[];

  setSelectedVaultId: (vaultId: string | null) => void;
  loadNotes: (vaultId: string) => Promise<void>;
  getNote: (id: string) => Promise<Note | null>;
  getNoteByPath: (vaultId: string, filePath: string) => Promise<Note | null>;
  createNote: (input: CreateNoteInput) => Promise<Note | null>;
  updateNote: (id: string, input: UpdateNoteInput) => Promise<Note | null>;
  deleteNote: (id: string) => Promise<void>;
  searchNotes: (
    vaultId: string,
    query: string,
    topK?: number,
  ) => Promise<NoteSearchResult[]>;
  getNoteLinks: (noteId: string) => Promise<NoteLink[]>;
  getNoteBacklinks: (noteId: string) => Promise<BacklinkInfo[]>;
  syncNoteLinks: (
    vaultId: string,
    sourceNoteId: string,
    links: [string, string, string][],
  ) => Promise<void>;
  setSelectedNoteId: (id: string | null) => void;
  loadVersions: (noteId: string) => Promise<NoteVersion[]>;
  getVersion: (versionId: number) => Promise<NoteVersion | null>;
  restoreVersion: (noteId: string, versionId: number) => Promise<Note | null>;
  loadTemplates: (wikiId: string) => Promise<void>;
  createTemplate: (
    input: CreateWikiTemplateInput,
  ) => Promise<WikiTemplate | null>;
  deleteTemplate: (id: string) => Promise<void>;
  createNoteFromTemplate: (
    vaultId: string,
    templateId: string,
    title?: string,
  ) => Promise<Note | null>;
  createDailyNote: (vaultId: string) => Promise<Note | null>;
  importObsidianVault: (
    wikiId: string,
    vaultPath: string,
  ) => Promise<ImportStats | null>;
  exportMarkdown: (
    wikiId: string,
    outputPath: string,
  ) => Promise<ExportStats | null>;
  exportHtml: (
    wikiId: string,
    outputPath: string,
  ) => Promise<ExportStats | null>;
  exportNotePdf: (noteId: string, outputPath: string) => Promise<string | null>;
}

export const useWikiStore = create<WikiState>((set) => ({
  notes: [],
  selectedNoteId: null,
  selectedVaultId: null,
  loading: false,
  error: null,
  templates: [],

  setSelectedVaultId: (vaultId) => {
    set({
      selectedVaultId: vaultId,
      selectedNoteId: null,
      notes: [],
      templates: [],
    });
  },

  loadNotes: async (vaultId) => {
    set({ loading: true, error: null });
    try {
      const notes = await invoke<Note[]>("wiki_notes_list", { vaultId });
      set({ notes, loading: false });
    } catch (e) {
      set({ error: String(e), loading: false });
    }
  },

  getNote: async (id) => {
    try {
      return await invoke<Note>("wiki_notes_get", { id });
    } catch (e) {
      set({ error: String(e) });
      return null;
    }
  },

  getNoteByPath: async (vaultId, filePath) => {
    try {
      return await invoke<Note>("wiki_notes_get_by_path", {
        vaultId,
        filePath,
      });
    } catch (e) {
      set({ error: String(e) });
      return null;
    }
  },

  createNote: async (input) => {
    try {
      const note = await invoke<Note>("wiki_notes_create", { input });
      set((s) => ({ notes: [...s.notes, note], error: null }));
      return note;
    } catch (e) {
      set({ error: String(e) });
      return null;
    }
  },

  updateNote: async (id, input) => {
    try {
      const updated = await invoke<Note>("wiki_notes_update", { id, input });
      set((s) => ({
        notes: s.notes.map((n) => (n.id === id ? updated : n)),
        error: null,
      }));
      return updated;
    } catch (e) {
      set({ error: String(e) });
      return null;
    }
  },

  deleteNote: async (id) => {
    try {
      await invoke("wiki_notes_delete", { id });
      set((s) => ({
        notes: s.notes.filter((n) => n.id !== id),
        selectedNoteId: s.selectedNoteId === id ? null : s.selectedNoteId,
        error: null,
      }));
    } catch (e) {
      set({ error: String(e) });
      throw e;
    }
  },

  searchNotes: async (vaultId, query, topK) => {
    try {
      return await invoke<NoteSearchResult[]>("wiki_notes_search", {
        vaultId,
        query,
        topK,
      });
    } catch (e) {
      set({ error: String(e) });
      return [];
    }
  },

  getNoteLinks: async (noteId) => {
    try {
      return await invoke<NoteLink[]>("wiki_notes_get_links", { noteId });
    } catch (e) {
      set({ error: String(e) });
      return [];
    }
  },

  getNoteBacklinks: async (noteId) => {
    try {
      return await invoke<BacklinkInfo[]>("wiki_notes_get_backlinks", {
        noteId,
      });
    } catch (e) {
      set({ error: String(e) });
      return [];
    }
  },

  syncNoteLinks: async (vaultId, sourceNoteId, links) => {
    try {
      await invoke("wiki_notes_sync_links", { vaultId, sourceNoteId, links });
    } catch (e) {
      set({ error: String(e) });
      throw e;
    }
  },

  setSelectedNoteId: (id) => {
    set({ selectedNoteId: id });
  },

  loadVersions: async (noteId) => {
    try {
      return await invoke<NoteVersion[]>("wiki_note_versions", { noteId });
    } catch (e) {
      set({ error: String(e) });
      return [];
    }
  },

  getVersion: async (versionId) => {
    try {
      return await invoke<NoteVersion>("wiki_note_get_version", { versionId });
    } catch (e) {
      set({ error: String(e) });
      return null;
    }
  },

  restoreVersion: async (noteId, versionId) => {
    try {
      const updated = await invoke<Note>("wiki_note_restore_version", {
        noteId,
        versionId,
      });
      set((s) => ({
        notes: s.notes.map((n) => (n.id === noteId ? updated : n)),
        error: null,
      }));
      return updated;
    } catch (e) {
      set({ error: String(e) });
      return null;
    }
  },

  loadTemplates: async (wikiId) => {
    try {
      const templates = await invoke<WikiTemplate[]>("wiki_template_list", {
        wikiId,
      });
      set({ templates });
    } catch (e) {
      set({ error: String(e) });
    }
  },

  createTemplate: async (input) => {
    try {
      const template = await invoke<WikiTemplate>("wiki_template_create", {
        input,
      });
      set((s) => ({ templates: [...s.templates, template], error: null }));
      return template;
    } catch (e) {
      set({ error: String(e) });
      return null;
    }
  },

  deleteTemplate: async (id) => {
    try {
      await invoke("wiki_template_delete", { id });
      set((s) => ({
        templates: s.templates.filter((t) => t.id !== id),
        error: null,
      }));
    } catch (e) {
      set({ error: String(e) });
    }
  },

  createNoteFromTemplate: async (vaultId, templateId, title) => {
    try {
      const note = await invoke<Note>("wiki_note_create_from_template", {
        vaultId,
        templateId,
        title,
      });
      set((s) => ({ notes: [...s.notes, note], error: null }));
      return note;
    } catch (e) {
      set({ error: String(e) });
      return null;
    }
  },

  createDailyNote: async (vaultId) => {
    try {
      const note = await invoke<Note>("wiki_create_daily_note", { vaultId });
      set((s) => {
        const exists = s.notes.some((n) => n.id === note.id);
        return {
          notes: exists ? s.notes : [...s.notes, note],
          error: null,
        };
      });
      return note;
    } catch (e) {
      set({ error: String(e) });
      return null;
    }
  },

  importObsidianVault: async (wikiId, vaultPath) => {
    try {
      const stats = await invoke<ImportStats>("wiki_import_obsidian_vault", {
        wikiId,
        vaultPath,
      });
      set({ error: null });
      return stats;
    } catch (e) {
      set({ error: String(e) });
      return null;
    }
  },

  exportMarkdown: async (wikiId, outputPath) => {
    try {
      const stats = await invoke<ExportStats>("wiki_export_markdown", {
        wikiId,
        outputPath,
      });
      set({ error: null });
      return stats;
    } catch (e) {
      set({ error: String(e) });
      return null;
    }
  },

  exportHtml: async (wikiId, outputPath) => {
    try {
      const stats = await invoke<ExportStats>("wiki_export_html", {
        wikiId,
        outputPath,
      });
      set({ error: null });
      return stats;
    } catch (e) {
      set({ error: String(e) });
      return null;
    }
  },

  exportNotePdf: async (noteId, outputPath) => {
    try {
      const htmlPath = await invoke<string>("wiki_note_export_pdf", {
        noteId,
        outputPath,
      });
      set({ error: null });
      return htmlPath;
    } catch (e) {
      set({ error: String(e) });
      return null;
    }
  },
}));

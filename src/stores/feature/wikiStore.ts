import { invoke } from "@/lib/invoke";
import type { CreateNoteInput, Note, NoteLink, NoteSearchResult, UpdateNoteInput } from "@/types";
import { create } from "zustand";

interface WikiState {
  notes: Note[];
  selectedNoteId: string | null;
  selectedVaultId: string | null;
  loading: boolean;
  error: string | null;

  setSelectedVaultId: (vaultId: string | null) => void;
  loadNotes: (vaultId: string) => Promise<void>;
  getNote: (id: string) => Promise<Note | null>;
  getNoteByPath: (vaultId: string, filePath: string) => Promise<Note | null>;
  createNote: (input: CreateNoteInput) => Promise<Note | null>;
  updateNote: (id: string, input: UpdateNoteInput) => Promise<Note | null>;
  deleteNote: (id: string) => Promise<void>;
  searchNotes: (vaultId: string, query: string, topK?: number) => Promise<NoteSearchResult[]>;
  getNoteLinks: (noteId: string) => Promise<NoteLink[]>;
  getNoteBacklinks: (noteId: string) => Promise<NoteLink[]>;
  syncNoteLinks: (vaultId: string, sourceNoteId: string, links: [string, string, string][]) => Promise<void>;
  setSelectedNoteId: (id: string | null) => void;
}

export const useWikiStore = create<WikiState>((set) => ({
  notes: [],
  selectedNoteId: null,
  selectedVaultId: null,
  loading: false,
  error: null,

  setSelectedVaultId: (vaultId) => {
    set({ selectedVaultId: vaultId, selectedNoteId: null, notes: [] });
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
      return await invoke<Note>("wiki_notes_get_by_path", { vaultId, filePath });
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
      return await invoke<NoteLink[]>("wiki_notes_get_backlinks", { noteId });
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
}));

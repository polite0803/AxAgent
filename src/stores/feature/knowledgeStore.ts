import { invoke, listen } from "@/lib/invoke";
import type { CreateKnowledgeBaseInput, KnowledgeBase, KnowledgeDocument, UpdateKnowledgeBaseInput } from "@/types";
import { create } from "zustand";

interface DocumentIndexedEvent {
  documentId: string;
  success: boolean;
  error?: string;
}

interface KnowledgeState {
  bases: KnowledgeBase[];
  documents: KnowledgeDocument[];
  loading: boolean;
  error: string | null;
  selectedBaseId: string | null;

  loadBases: () => Promise<void>;
  createBase: (input: CreateKnowledgeBaseInput) => Promise<KnowledgeBase | null>;
  updateBase: (id: string, input: UpdateKnowledgeBaseInput) => Promise<void>;
  deleteBase: (id: string) => Promise<void>;
  reorderBases: (baseIds: string[]) => Promise<void>;
  loadDocuments: (baseId: string) => Promise<void>;
  addDocument: (baseId: string, title: string, sourcePath: string, mimeType: string) => Promise<void>;
  deleteDocument: (knowledgeBaseId: string, documentId: string) => Promise<void>;
  setSelectedBaseId: (id: string | null) => void;
  setupEventListeners: () => Promise<() => void>;
}

export const useKnowledgeStore = create<KnowledgeState>((set, get) => ({
  bases: [],
  documents: [],
  loading: false,
  error: null,
  selectedBaseId: null,

  loadBases: async () => {
    set({ loading: true });
    try {
      const bases = await invoke<KnowledgeBase[]>("list_knowledge_bases");
      set({ bases, loading: false, error: null });
    } catch (e) {
      set({ error: String(e), loading: false });
    }
  },

  createBase: async (input) => {
    try {
      const base = await invoke<KnowledgeBase>("create_knowledge_base", { input });
      set((s) => ({ bases: [...s.bases, base], error: null }));
      return base;
    } catch (e) {
      set({ error: String(e) });
      return null;
    }
  },

  updateBase: async (id, input) => {
    try {
      const updated = await invoke<KnowledgeBase>("update_knowledge_base", { id, input });
      set((s) => ({ bases: s.bases.map((b) => (b.id === id ? updated : b)), error: null }));
    } catch (e) {
      set({ error: String(e) });
      throw e;
    }
  },

  deleteBase: async (id) => {
    try {
      await invoke("delete_knowledge_base", { id });
      set((s) => ({ bases: s.bases.filter((b) => b.id !== id), error: null }));
    } catch (e) {
      set({ error: String(e) });
      throw e;
    }
  },

  reorderBases: async (baseIds) => {
    const prev = get().bases;
    const reordered = baseIds
      .map((id) => prev.find((b) => b.id === id))
      .filter(Boolean) as KnowledgeBase[];
    set({ bases: reordered });
    try {
      await invoke("reorder_knowledge_bases", { baseIds });
    } catch (e) {
      set({ bases: prev, error: String(e) });
    }
  },

  loadDocuments: async (baseId) => {
    set({ loading: true });
    try {
      const documents = await invoke<KnowledgeDocument[]>("list_knowledge_documents", { baseId });
      set({ documents, loading: false, error: null });
    } catch (e) {
      set({ error: String(e), loading: false });
    }
  },

  addDocument: async (baseId, title, sourcePath, mimeType) => {
    try {
      await invoke("add_knowledge_document", { baseId, title, sourcePath, mimeType });
      await get().loadDocuments(baseId);
    } catch (e) {
      set({ error: String(e) });
      throw e;
    }
  },

  deleteDocument: async (knowledgeBaseId, documentId) => {
    try {
      await invoke("delete_knowledge_document", { baseId: knowledgeBaseId, id: documentId });
      await get().loadDocuments(knowledgeBaseId);
    } catch (e) {
      set({ error: String(e) });
      throw e;
    }
  },

  setSelectedBaseId: (id) => {
    set({ selectedBaseId: id });
  },

  setupEventListeners: async () => {
    const unlistenIndexed = await listen<DocumentIndexedEvent>(
      "knowledge-document-indexed",
      (event) => {
        const { documentId, success, error } = event.payload;
        set((state) => ({
          documents: state.documents.map((doc) =>
            doc.id === documentId
              ? {
                ...doc,
                indexingStatus: success ? "ready" : "failed",
                indexError: success ? undefined : error,
              }
              : doc
          ),
        }));
      },
    );

    const unlistenReindexed = await listen<{ chunkId: string; success: boolean }>(
      "knowledge-chunk-reindexed",
      () => {
        // Chunk reindexed — refresh documents if a base is selected
        const selectedBaseId = get().selectedBaseId;
        if (selectedBaseId) {
          get().loadDocuments(selectedBaseId);
        }
      },
    );

    const unlistenRebuild = await listen<{ baseId: string }>(
      "knowledge-rebuild-complete",
      (event) => {
        const selectedBaseId = get().selectedBaseId;
        if (selectedBaseId === event.payload.baseId) {
          get().loadDocuments(selectedBaseId);
        }
      },
    );

    // Return cleanup function
    return () => {
      unlistenIndexed();
      unlistenReindexed();
      unlistenRebuild();
    };
  },
}));

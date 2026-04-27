import { invoke } from "@/lib/invoke";
import type { Artifact, ArtifactPreviewMode, CreateArtifactInput, UpdateArtifactInput } from "@/types";
import { create } from "zustand";

export interface ExecutionResult {
  stdout: string;
  stderr: string;
  exit_code: number;
  duration_ms: number;
}

interface ArtifactState {
  artifacts: Artifact[];
  loading: boolean;
  error: string | null;
  previewArtifact: Artifact | null;
  previewMode: ArtifactPreviewMode;
  executionResults: Record<string, ExecutionResult>;

  loadArtifacts: (conversationId: string) => Promise<void>;
  createArtifact: (input: CreateArtifactInput) => Promise<Artifact | null>;
  updateArtifact: (id: string, input: UpdateArtifactInput) => Promise<Artifact | null>;
  deleteArtifact: (id: string) => Promise<void>;
  pinArtifact: (id: string, pinned: boolean) => Promise<void>;
  clearArtifacts: () => void;
  setPreviewArtifact: (artifact: Artifact | null) => void;
  setPreviewMode: (mode: ArtifactPreviewMode) => void;
  executeCode: (artifactId: string) => Promise<ExecutionResult | null>;
  duplicateArtifact: (id: string) => Promise<Artifact | null>;
}

export const useArtifactStore = create<ArtifactState>((set, get) => ({
  artifacts: [],
  loading: false,
  error: null,
  previewArtifact: null,
  previewMode: "code",
  executionResults: {},

  loadArtifacts: async (conversationId) => {
    set({ loading: true, error: null });
    try {
      const artifacts = await invoke<Artifact[]>("list_artifacts", {
        conversation_id: conversationId,
      });
      set({ artifacts, loading: false });
    } catch (e) {
      set({ error: String(e), loading: false });
    }
  },

  createArtifact: async (input) => {
    try {
      const artifact = await invoke<Artifact>("create_artifact", input);
      set((s) => ({ artifacts: [...s.artifacts, artifact] }));
      return artifact;
    } catch (e) {
      set({ error: String(e) });
      return null;
    }
  },

  updateArtifact: async (id, input) => {
    try {
      const updated = await invoke<Artifact>("update_artifact", { id, ...input });
      set((s) => ({
        artifacts: s.artifacts.map((a) => (a.id === id ? updated : a)),
        previewArtifact: s.previewArtifact?.id === id ? updated : s.previewArtifact,
      }));
      return updated;
    } catch (e) {
      set({ error: String(e) });
      return null;
    }
  },

  deleteArtifact: async (id) => {
    try {
      await invoke("delete_artifact", { id });
      set((s) => ({
        artifacts: s.artifacts.filter((a) => a.id !== id),
        previewArtifact: s.previewArtifact?.id === id ? null : s.previewArtifact,
      }));
    } catch (e) {
      set({ error: String(e) });
    }
  },

  pinArtifact: async (id, pinned) => {
    const { updateArtifact } = get();
    await updateArtifact(id, { pinned });
  },

  clearArtifacts: () => set({ artifacts: [], error: null, previewArtifact: null }),

  setPreviewArtifact: (artifact) => set({ previewArtifact: artifact }),

  setPreviewMode: (mode) => set({ previewMode: mode }),

  executeCode: async (artifactId) => {
    const { artifacts } = get();
    const artifact = artifacts.find((a) => a.id === artifactId);
    if (!artifact) { return null; }

    const startTime = performance.now();
    try {
      const result = await invoke<ExecutionResult>("execute_sandbox", {
        code: artifact.content,
        language: artifact.language || artifact.format,
      });
      const executionResult = { ...result, duration_ms: performance.now() - startTime };
      set((s) => ({
        executionResults: { ...s.executionResults, [artifactId]: executionResult },
      }));
      return executionResult;
    } catch (e) {
      const errorResult: ExecutionResult = {
        stdout: "",
        stderr: String(e),
        exit_code: -1,
        duration_ms: performance.now() - startTime,
      };
      set((s) => ({
        executionResults: { ...s.executionResults, [artifactId]: errorResult },
      }));
      return errorResult;
    }
  },

  duplicateArtifact: async (id) => {
    const { artifacts, createArtifact } = get();
    const original = artifacts.find((a) => a.id === id);
    if (!original) { return null; }

    return createArtifact({
      conversationId: original.conversationId,
      kind: original.kind,
      title: `${original.title} (copy)`,
      content: original.content,
      format: original.format,
    });
  },
}));

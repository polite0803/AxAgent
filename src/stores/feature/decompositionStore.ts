import { create } from "zustand";
import { invoke } from "../../lib/invoke";
import type { DecompositionPreview } from "../../types";

type ToolAction = "auto_install" | "manual_install" | "generate" | "skip";

interface DecompositionRequest {
  name: string;
  description: string;
  content: string;
  source: string;
  version?: string;
  repo?: string;
}

interface DecompositionState {
  preview: DecompositionPreview | null;
  originalRequest: DecompositionRequest | null;
  loading: boolean;
  toolActions: Record<string, ToolAction>;

  previewDecomposition: (request: DecompositionRequest) => Promise<void>;
  setToolAction: (toolName: string, action: ToolAction) => void;
  generateMissingTool: (
    name: string,
    description: string,
    inputSchema: Record<string, unknown>,
    outputSchema: Record<string, unknown>,
  ) => Promise<boolean>;
  confirmDecomposition: (
    workflowName: string,
    workflowDescription?: string,
  ) => Promise<{ workflow_id: string; saved_skills: number }>;
  reset: () => void;
}

export const useDecompositionStore = create<DecompositionState>((set, get) => ({
  preview: null,
  originalRequest: null,
  loading: false,
  toolActions: {},

  previewDecomposition: async (request) => {
    set({ loading: true });
    try {
      const preview = await invoke<DecompositionPreview & { cache_id: string }>("preview_decomposition", { request });
      set({ preview, originalRequest: request });
    } finally {
      set({ loading: false });
    }
  },

  setToolAction: (toolName: string, action: ToolAction) => {
    set((state) => ({
      toolActions: { ...state.toolActions, [toolName]: action },
    }));
  },

  generateMissingTool: async (
    name: string,
    description: string,
    inputSchema: Record<string, unknown>,
    outputSchema: Record<string, unknown>,
  ) => {
    try {
      const result = await invoke<{ tool_name: string; success: boolean }>("generate_missing_tool", {
        name,
        description,
        input_schema: inputSchema,
        output_schema: outputSchema,
      });
      return result.success;
    } catch {
      return false;
    }
  },

  confirmDecomposition: async (workflowName: string, workflowDescription?: string) => {
    const { preview, originalRequest } = get();
    if (!preview) { throw new Error("No preview available"); }
    if (!originalRequest) { throw new Error("No original request available"); }

    const result = await invoke<{ workflow_id: string; saved_skills: number }>("confirm_decomposition", {
      request: {
        preview: {
          name: originalRequest.name,
          description: originalRequest.description,
          content: originalRequest.content,
          source: originalRequest.source,
          version: originalRequest.version,
          repo: originalRequest.repo,
        },
        cache_id: preview.cache_id,
        workflow_name: workflowName,
        workflow_description: workflowDescription,
      },
    });

    set({ preview: null, originalRequest: null, toolActions: {} });
    return result;
  },

  reset: () => {
    set({ preview: null, originalRequest: null, toolActions: {}, loading: false });
  },
}));

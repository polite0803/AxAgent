import type {
  ErrorConfig,
  JsonSchema,
  SemanticCheckResult,
  SkillReplacementAction,
  TemplateFilter,
  TriggerConfig,
  ValidationResult,
  Variable,
  WorkflowEdge,
  WorkflowNode,
  WorkflowTemplateInput,
  WorkflowTemplateResponse,
} from "@/components/workflow/types";
import { invoke } from "@/lib/invoke";
import { create } from "zustand";
import { immer } from "zustand/middleware/immer";

export interface SimilarWorkflow {
  workflow_id: string;
  name: string;
  skill_ids: string[];
  similarity: number;
}

export interface SaveSkillWorkflowResponse {
  needs_review: boolean;
  workflow_id: string | null;
  similar_workflows: SimilarWorkflow[];
}

interface PendingWorkflowData {
  workflowName: string;
  workflowDescription?: string;
}

interface WorkflowEditorState {
  currentTemplate: WorkflowTemplateResponse | null;
  templates: WorkflowTemplateResponse[];
  selectedNodeId: string | null;
  selectedEdgeId: string | null;
  isLoading: boolean;
  isSaving: boolean;
  isDirty: boolean;
  validationResult: ValidationResult | null;
  filter: TemplateFilter;
  error: string | null;
  past: Array<{ nodes: WorkflowNode[]; edges: WorkflowEdge[] }>;
  future: Array<{ nodes: WorkflowNode[]; edges: WorkflowEdge[] }>;
  undo: () => void;
  redo: () => void;
  canUndo: () => boolean;
  canRedo: () => boolean;
  importedWorkflowData: {
    nodes: WorkflowNode[];
    edges: WorkflowEdge[];
    name?: string;
    description?: string;
    isDecompositionWorkflow: boolean;
    decompositionSource?: {
      market: string;
      repo?: string;
      version?: string;
      content: string;
    };
  } | null;
  isDecompositionTemplate: boolean;
  pendingDecompositionSource: {
    market: string;
    repo?: string;
    version?: string;
    content: string;
  } | null;
  similarWorkflowsForReview: SimilarWorkflow[];
  pendingWorkflowData: PendingWorkflowData | null;

  nodes: WorkflowNode[];
  edges: WorkflowEdge[];

  loadTemplates: () => Promise<void>;
  loadTemplate: (id: string) => Promise<void>;
  createTemplate: (input: WorkflowTemplateInput) => Promise<string | null>;
  updateTemplate: (id: string, input: WorkflowTemplateInput) => Promise<boolean>;
  deleteTemplate: (id: string) => Promise<boolean>;
  duplicateTemplate: (id: string) => Promise<string | null>;
  validateTemplate: () => Promise<ValidationResult | null>;
  exportTemplate: (id: string) => Promise<string | null>;
  importTemplate: (jsonData: string) => Promise<string | null>;
  loadTemplateVersions: (id: string) => Promise<number[]>;
  loadTemplateByVersion: (id: string, version: number) => Promise<void>;

  setFilter: (filter: TemplateFilter) => void;
  setSelectedNode: (nodeId: string | null) => void;
  setSelectedEdge: (edgeId: string | null) => void;

  addNode: (node: WorkflowNode) => void;
  updateNode: (nodeId: string, updates: Partial<WorkflowNode>) => void;
  deleteNode: (nodeId: string) => void;

  addEdge: (edge: WorkflowEdge) => void;
  updateEdge: (edgeId: string, updates: Partial<WorkflowEdge>) => void;
  deleteEdge: (edgeId: string) => void;

  setNodes: (nodes: WorkflowNode[]) => void;
  setEdges: (edges: WorkflowEdge[]) => void;

  updateTemplateMetadata: (metadata: {
    name?: string;
    description?: string;
    icon?: string;
    tags?: string[];
    triggerConfig?: TriggerConfig;
    inputSchema?: JsonSchema;
    outputSchema?: JsonSchema;
    variables?: Variable[];
    errorConfig?: ErrorConfig;
  }) => void;

  initNewTemplate: () => void;
  markClean: () => void;
  setError: (error: string | null) => void;
  setImportedWorkflowData: (data: {
    nodes: WorkflowNode[];
    edges: WorkflowEdge[];
    name?: string;
    description?: string;
    isDecompositionWorkflow?: boolean;
    decompositionSource?: {
      market: string;
      repo?: string;
      version?: string;
      content: string;
    };
  }) => void;
  clearImportedWorkflowData: () => void;
  saveDecompositionWorkflow: (
    workflowName: string,
    workflowDescription?: string,
  ) => Promise<{ workflow_id: string; saved_skills: number }>;
  saveSkillWorkflowFromLlm: (
    workflowName: string,
    workflowDescription?: string,
  ) => Promise<SaveSkillWorkflowResponse>;
  forceSaveSkillWorkflow: (
    targetWorkflowId: string,
    workflowName: string,
    workflowDescription?: string,
  ) => Promise<string>;
  setSimilarWorkflowsForReview: (workflows: SimilarWorkflow[], pendingData: PendingWorkflowData) => void;
  clearSimilarWorkflowsForReview: () => void;

  generateWorkflowFromPrompt: (prompt: string) => Promise<{ nodes: WorkflowNode[]; edges: WorkflowEdge[] } | null>;
  optimizeAgentPrompt: (prompt: string) => Promise<string | null>;
  recommendNodes: (context: string) => Promise<string[] | null>;

  semanticCheckResult: SemanticCheckResult | null;
  pendingReplacements: Map<string, { existingSkillId: string; action: SkillReplacementAction }>;
  checkSkillSemanticMatches: (nodes: WorkflowNode[]) => Promise<SemanticCheckResult | null>;
  applySkillReplacement: (nodeId: string, existingSkillId: string, action: SkillReplacementAction) => void;
  applySemanticAction: (nodeId: string, action: "replace" | "keep" | "upgrade_existing") => void;
  clearSemanticCheckResult: () => void;

  loadConversationWorkflowPreview: (conversationId: string) => Promise<void>;
}

interface ConversationWorkflowPreviewResponse {
  nodes: unknown[];
  edges: unknown[];
  skill_execution_order: string[];
  skill_count: number;
}

const createEmptyTemplate = (): Omit<WorkflowTemplateResponse, "id" | "created_at" | "updated_at"> => ({
  name: "Unnamed Workflow",
  description: "",
  icon: "Bot",
  tags: [],
  version: 1,
  is_preset: false,
  is_editable: true,
  is_public: false,
  trigger_config: { type: "manual", config: {} },
  nodes: [],
  edges: [],
  input_schema: undefined,
  output_schema: undefined,
  variables: [],
  error_config: undefined,
});

export const useWorkflowEditorStore = create<WorkflowEditorState>()(
  immer((set, get) => ({
    currentTemplate: null,
    templates: [],
    selectedNodeId: null,
    selectedEdgeId: null,
    isLoading: false,
    isSaving: false,
    isDirty: false,
    validationResult: null,
    filter: {},
    error: null,
    importedWorkflowData: null,
    isDecompositionTemplate: false,
    pendingDecompositionSource: null,
    similarWorkflowsForReview: [],
    pendingWorkflowData: null,
    nodes: [],
    edges: [],
    past: [],
    future: [],

    undo: () => {
      const { past } = get();
      if (past.length === 0) { return; }

      const previous = past[past.length - 1];
      set((state) => {
        state.future.push({ nodes: [...state.nodes], edges: [...state.edges] });
        state.nodes = previous.nodes;
        state.edges = previous.edges;
        state.past = state.past.slice(0, -1);
        state.isDirty = true;
      });
    },

    redo: () => {
      const { future } = get();
      if (future.length === 0) { return; }

      const next = future[future.length - 1];
      set((state) => {
        state.past.push({ nodes: [...state.nodes], edges: [...state.edges] });
        state.nodes = next.nodes;
        state.edges = next.edges;
        state.future = state.future.slice(0, -1);
        state.isDirty = true;
      });
    },

    canUndo: () => get().past.length > 0,
    canRedo: () => get().future.length > 0,

    loadTemplates: async () => {
      set((state) => {
        state.isLoading = true;
        state.error = null;
      });
      try {
        const filter = get().filter;
        const is_preset = filter.is_preset;
        const params = is_preset !== undefined ? { is_preset } : {};
        const templates = await invoke<WorkflowTemplateResponse[]>("list_workflow_templates", params);
        set((state) => {
          state.templates = Array.isArray(templates) ? templates : [];
          state.isLoading = false;
        });
      } catch (error) {
        set((state) => {
          state.error = String(error);
          state.isLoading = false;
        });
      }
    },

    loadTemplate: async (id: string) => {
      set((state) => {
        state.isLoading = true;
        state.error = null;
      });
      try {
        const template = await invoke<WorkflowTemplateResponse>("get_workflow_template", { id });
        set((state) => {
          state.currentTemplate = template;
          state.nodes = template.nodes;
          state.edges = template.edges;
          state.isLoading = false;
          state.isDirty = false;
          state.past = [];
          state.future = [];
        });
      } catch (error) {
        set((state) => {
          state.error = String(error);
          state.isLoading = false;
        });
      }
    },

    createTemplate: async (input: WorkflowTemplateInput) => {
      set((state) => {
        state.isSaving = true;
        state.error = null;
      });
      try {
        const id = await invoke<string>("create_workflow_template", { input });
        await get().loadTemplates();
        set((state) => {
          state.isSaving = false;
        });
        return id;
      } catch (error) {
        set((state) => {
          state.error = String(error);
          state.isSaving = false;
        });
        return null;
      }
    },

    updateTemplate: async (id: string, input: WorkflowTemplateInput) => {
      set((state) => {
        state.isSaving = true;
        state.error = null;
      });
      try {
        await invoke<boolean>("update_workflow_template", { id, input });
        await get().loadTemplates();
        await get().loadTemplate(id);
        set((state) => {
          state.isSaving = false;
          state.isDirty = false;
        });
        return true;
      } catch (error) {
        set((state) => {
          state.error = String(error);
          state.isSaving = false;
        });
        return false;
      }
    },

    deleteTemplate: async (id: string) => {
      set((state) => {
        state.isLoading = true;
        state.error = null;
      });
      try {
        await invoke<void>("delete_workflow_template", { id });
        set((state) => {
          if (state.currentTemplate?.id === id) {
            state.currentTemplate = null;
            state.nodes = [];
            state.edges = [];
          }
          state.templates = state.templates.filter((t) => t.id !== id);
          state.isLoading = false;
        });
        return true;
      } catch (error) {
        set((state) => {
          state.error = String(error);
          state.isLoading = false;
        });
        return false;
      }
    },

    duplicateTemplate: async (id: string) => {
      set((state) => {
        state.isSaving = true;
        state.error = null;
      });
      try {
        const newId = await invoke<string>("duplicate_workflow_template", { id });
        await get().loadTemplates();
        set((state) => {
          state.isSaving = false;
        });
        return newId;
      } catch (error) {
        set((state) => {
          state.error = String(error);
          state.isSaving = false;
        });
        return null;
      }
    },

    validateTemplate: async () => {
      const { currentTemplate, nodes, edges } = get();
      if (!currentTemplate) { return null; }

      const input: WorkflowTemplateInput = {
        name: currentTemplate.name,
        description: currentTemplate.description,
        icon: currentTemplate.icon,
        tags: currentTemplate.tags,
        trigger_config: currentTemplate.trigger_config,
        nodes,
        edges,
        input_schema: currentTemplate.input_schema,
        output_schema: currentTemplate.output_schema,
        variables: currentTemplate.variables,
        error_config: currentTemplate.error_config,
      };

      try {
        const result = await invoke<ValidationResult>("validate_workflow_template", { input });
        set((state) => {
          state.validationResult = result;
        });
        return result;
      } catch (error) {
        set((state) => {
          state.error = String(error);
        });
        return null;
      }
    },

    exportTemplate: async (id: string) => {
      try {
        const json = await invoke<string>("export_workflow_template", { id });
        return json;
      } catch (error) {
        set((state) => {
          state.error = String(error);
        });
        return null;
      }
    },

    importTemplate: async (jsonData: string) => {
      set((state) => {
        state.isSaving = true;
        state.error = null;
      });
      try {
        const id = await invoke<string>("import_workflow_template", { jsonData });
        await get().loadTemplates();
        set((state) => {
          state.isSaving = false;
        });
        return id;
      } catch (error) {
        set((state) => {
          state.error = String(error);
          state.isSaving = false;
        });
        return null;
      }
    },

    loadTemplateVersions: async (id: string) => {
      try {
        const versions = await invoke<number[]>("get_template_versions", { id });
        return versions;
      } catch (error) {
        set((state) => {
          state.error = String(error);
        });
        return [];
      }
    },

    loadTemplateByVersion: async (id: string, version: number) => {
      set((state) => {
        state.isLoading = true;
        state.error = null;
      });
      try {
        const template = await invoke<WorkflowTemplateResponse | null>("get_template_by_version", { id, version });
        if (template) {
          set((state) => {
            state.currentTemplate = template;
            state.nodes = template.nodes || [];
            state.edges = template.edges || [];
            state.isLoading = false;
            state.isDirty = false;
          });
        } else {
          set((state) => {
            state.error = "Version not found";
            state.isLoading = false;
          });
        }
      } catch (error) {
        set((state) => {
          state.error = String(error);
          state.isLoading = false;
        });
      }
    },

    setFilter: (filter: TemplateFilter) => {
      set((state) => {
        state.filter = filter;
      });
    },

    setSelectedNode: (nodeId: string | null) => {
      set((state) => {
        state.selectedNodeId = nodeId;
        state.selectedEdgeId = null;
      });
    },

    setSelectedEdge: (edgeId: string | null) => {
      set((state) => {
        state.selectedEdgeId = edgeId;
        state.selectedNodeId = null;
      });
    },

    addNode: (node: WorkflowNode) => {
      set((state) => {
        state.past.push({ nodes: [...state.nodes], edges: [...state.edges] });
        state.future = [];
        if (state.past.length > 50) {
          state.past = state.past.slice(-50);
        }
        state.nodes.push(node);
        state.isDirty = true;
      });
    },

    updateNode: (nodeId: string, updates: Partial<WorkflowNode>) => {
      set((state) => {
        state.past.push({ nodes: [...state.nodes], edges: [...state.edges] });
        state.future = [];
        if (state.past.length > 50) {
          state.past = state.past.slice(-50);
        }
        const index = state.nodes.findIndex((n) => n.id === nodeId);
        if (index !== -1) {
          state.nodes[index] = { ...state.nodes[index], ...updates } as WorkflowNode;
          state.isDirty = true;
        }
      });
    },

    deleteNode: (nodeId: string) => {
      set((state) => {
        state.past.push({ nodes: [...state.nodes], edges: [...state.edges] });
        state.future = [];
        if (state.past.length > 50) {
          state.past = state.past.slice(-50);
        }
        state.nodes = state.nodes.filter((n) => n.id !== nodeId);
        state.edges = state.edges.filter((e) => e.source !== nodeId && e.target !== nodeId);
        if (state.selectedNodeId === nodeId) {
          state.selectedNodeId = null;
        }
        state.isDirty = true;
      });
    },

    addEdge: (edge: WorkflowEdge) => {
      set((state) => {
        state.past.push({ nodes: [...state.nodes], edges: [...state.edges] });
        state.future = [];
        if (state.past.length > 50) {
          state.past = state.past.slice(-50);
        }
        state.edges.push(edge);
        state.isDirty = true;
      });
    },

    updateEdge: (edgeId: string, updates: Partial<WorkflowEdge>) => {
      set((state) => {
        state.past.push({ nodes: [...state.nodes], edges: [...state.edges] });
        state.future = [];
        if (state.past.length > 50) {
          state.past = state.past.slice(-50);
        }
        const index = state.edges.findIndex((e) => e.id === edgeId);
        if (index !== -1) {
          state.edges[index] = { ...state.edges[index], ...updates };
          state.isDirty = true;
        }
      });
    },

    deleteEdge: (edgeId: string) => {
      set((state) => {
        state.past.push({ nodes: [...state.nodes], edges: [...state.edges] });
        state.future = [];
        if (state.past.length > 50) {
          state.past = state.past.slice(-50);
        }
        state.edges = state.edges.filter((e) => e.id !== edgeId);
        if (state.selectedEdgeId === edgeId) {
          state.selectedEdgeId = null;
        }
        state.isDirty = true;
      });
    },

    setNodes: (nodes: WorkflowNode[]) => {
      set((state) => {
        state.past.push({ nodes: [...state.nodes], edges: [...state.edges] });
        state.future = [];
        if (state.past.length > 50) {
          state.past = state.past.slice(-50);
        }
        state.nodes = nodes;
        state.isDirty = true;
      });
    },

    setEdges: (edges: WorkflowEdge[]) => {
      set((state) => {
        state.past.push({ nodes: [...state.nodes], edges: [...state.edges] });
        state.future = [];
        if (state.past.length > 50) {
          state.past = state.past.slice(-50);
        }
        state.edges = edges;
        state.isDirty = true;
      });
    },

    updateTemplateMetadata: (metadata) => {
      set((state) => {
        if (state.currentTemplate) {
          if (metadata.name !== undefined) { state.currentTemplate.name = metadata.name; }
          if (metadata.description !== undefined) { state.currentTemplate.description = metadata.description; }
          if (metadata.icon !== undefined) { state.currentTemplate.icon = metadata.icon; }
          if (metadata.tags !== undefined) { state.currentTemplate.tags = metadata.tags; }
          if (metadata.triggerConfig !== undefined) { state.currentTemplate.trigger_config = metadata.triggerConfig; }
          if (metadata.inputSchema !== undefined) { state.currentTemplate.input_schema = metadata.inputSchema; }
          if (metadata.outputSchema !== undefined) { state.currentTemplate.output_schema = metadata.outputSchema; }
          if (metadata.variables !== undefined) { state.currentTemplate.variables = metadata.variables; }
          if (metadata.errorConfig !== undefined) { state.currentTemplate.error_config = metadata.errorConfig; }
          state.isDirty = true;
        }
      });
    },

    initNewTemplate: () => {
      const importedData = get().importedWorkflowData;
      const empty = createEmptyTemplate();
      set((state) => {
        state.currentTemplate = {
          ...empty,
          ...(importedData?.name && { name: importedData.name }),
          ...(importedData?.description && { description: importedData.description }),
          id: "",
          created_at: Date.now(),
          updated_at: Date.now(),
        } as WorkflowTemplateResponse;
        state.nodes = importedData?.nodes || [];
        state.edges = importedData?.edges || [];
        state.isDirty = !!(importedData?.nodes && importedData.nodes.length > 0);
        state.isDecompositionTemplate = importedData?.isDecompositionWorkflow || false;
        state.pendingDecompositionSource = importedData?.decompositionSource || null;
        state.selectedNodeId = null;
        state.selectedEdgeId = null;
        state.importedWorkflowData = null;
        state.past = [];
        state.future = [];
      });
    },

    setImportedWorkflowData: (data) => {
      set((state) => {
        state.importedWorkflowData = {
          ...data,
          isDecompositionWorkflow: data.isDecompositionWorkflow || false,
        };
      });
    },

    clearImportedWorkflowData: () => {
      set((state) => {
        state.importedWorkflowData = null;
        state.isDecompositionTemplate = false;
        state.pendingDecompositionSource = null;
      });
    },

    saveDecompositionWorkflow: async (workflowName: string, workflowDescription?: string) => {
      const { isDecompositionTemplate, pendingDecompositionSource } = get();
      if (!isDecompositionTemplate || !pendingDecompositionSource) {
        throw new Error("Not a decomposition workflow or missing source data");
      }

      set((state) => {
        state.isSaving = true;
        state.error = null;
      });

      try {
        const result = await invoke<{ workflow_id: string; saved_skills: number }>("confirm_decomposition", {
          request: {
            preview: {
              name: pendingDecompositionSource.market,
              description: workflowDescription || "",
              content: pendingDecompositionSource.content,
              source: pendingDecompositionSource.market,
              version: pendingDecompositionSource.version,
              repo: pendingDecompositionSource.repo,
            },
            workflow_name: workflowName,
            workflow_description: workflowDescription,
          },
        });

        set((state) => {
          state.isSaving = false;
          state.isDirty = false;
          state.isDecompositionTemplate = false;
          state.pendingDecompositionSource = null;
        });

        await get().loadTemplates();
        return result;
      } catch (error) {
        set((state) => {
          state.error = String(error);
          state.isSaving = false;
        });
        throw error;
      }
    },

    saveSkillWorkflowFromLlm: async (workflowName: string, workflowDescription?: string) => {
      const { isDecompositionTemplate, pendingDecompositionSource, nodes, edges } = get();
      if (!isDecompositionTemplate || !pendingDecompositionSource) {
        throw new Error("Not a decomposition workflow or missing source data");
      }

      set((state) => {
        state.isSaving = true;
        state.error = null;
      });

      try {
        const response = await invoke<SaveSkillWorkflowResponse>("save_skill_workflow_from_llm", {
          request: {
            skill_id: pendingDecompositionSource.market,
            skill_name: pendingDecompositionSource.repo || pendingDecompositionSource.market,
            workflow_name: workflowName,
            description: workflowDescription,
            nodes,
            edges,
          },
        });

        set((state) => {
          state.isSaving = false;
        });

        if (response.needs_review) {
          set((state) => {
            state.similarWorkflowsForReview = response.similar_workflows;
            state.pendingWorkflowData = { workflowName, workflowDescription };
          });
          return response;
        }

        set((state) => {
          state.isDirty = false;
          state.isDecompositionTemplate = false;
          state.pendingDecompositionSource = null;
        });

        await get().loadTemplates();
        return response;
      } catch (error) {
        set((state) => {
          state.error = String(error);
          state.isSaving = false;
        });
        throw error;
      }
    },

    forceSaveSkillWorkflow: async (targetWorkflowId: string, workflowName: string, workflowDescription?: string) => {
      const { isDecompositionTemplate, pendingDecompositionSource, nodes, edges } = get();
      if (!isDecompositionTemplate || !pendingDecompositionSource) {
        throw new Error("Not a decomposition workflow or missing source data");
      }

      set((state) => {
        state.isSaving = true;
        state.error = null;
      });

      try {
        const workflowId = await invoke<string>("force_save_skill_workflow", {
          request: {
            skill_id: pendingDecompositionSource.market,
            skill_name: pendingDecompositionSource.repo || pendingDecompositionSource.market,
            workflow_name: workflowName,
            description: workflowDescription,
            nodes,
            edges,
            target_workflow_id: targetWorkflowId,
          },
        });

        set((state) => {
          state.isSaving = false;
          state.isDirty = false;
          state.isDecompositionTemplate = false;
          state.pendingDecompositionSource = null;
          state.similarWorkflowsForReview = [];
          state.pendingWorkflowData = null;
        });

        await get().loadTemplates();
        return workflowId;
      } catch (error) {
        set((state) => {
          state.error = String(error);
          state.isSaving = false;
        });
        throw error;
      }
    },

    setSimilarWorkflowsForReview: (workflows, pendingData) => {
      set((state) => {
        state.similarWorkflowsForReview = workflows;
        state.pendingWorkflowData = pendingData;
      });
    },

    clearSimilarWorkflowsForReview: () => {
      set((state) => {
        state.similarWorkflowsForReview = [];
        state.pendingWorkflowData = null;
      });
    },

    markClean: () => {
      set((state) => {
        state.isDirty = false;
      });
    },

    setError: (error: string | null) => {
      set((state) => {
        state.error = error;
      });
    },

    generateWorkflowFromPrompt: async (prompt: string) => {
      set((state) => {
        state.isLoading = true;
        state.error = null;
      });
      try {
        const result = await invoke<{ nodes: WorkflowNode[]; edges: WorkflowEdge[]; explanation?: string }>(
          "generate_workflow_from_prompt",
          { prompt },
        );
        if (result) {
          set((state) => {
            state.nodes = result.nodes;
            state.edges = result.edges;
            state.isLoading = false;
          });
          return { nodes: result.nodes, edges: result.edges };
        }
        return null;
      } catch (error) {
        set((state) => {
          state.error = String(error);
          state.isLoading = false;
        });
        return null;
      }
    },

    optimizeAgentPrompt: async (prompt: string) => {
      set((state) => {
        state.isLoading = true;
        state.error = null;
      });
      try {
        const result = await invoke<string>("optimize_agent_prompt", { prompt });
        set((state) => {
          state.isLoading = false;
        });
        return result;
      } catch (error) {
        set((state) => {
          state.error = String(error);
          state.isLoading = false;
        });
        return null;
      }
    },

    recommendNodes: async (context: string) => {
      set((state) => {
        state.isLoading = true;
        state.error = null;
      });
      try {
        const result = await invoke<
          Array<{ node_type: string; label: string; description: string; confidence: number }>
        >(
          "recommend_nodes",
          { context },
        );
        set((state) => {
          state.isLoading = false;
        });
        return result?.map((r) => r.label) ?? null;
      } catch (error) {
        set((state) => {
          state.error = String(error);
          state.isLoading = false;
        });
        return null;
      }
    },

    semanticCheckResult: null,
    pendingReplacements: new Map(),

    checkSkillSemanticMatches: async (nodes: WorkflowNode[]) => {
      const atomicSkillNodes = nodes
        .filter((n) => n.type === "atomicSkill" && (n as any).config?.skill_name)
        .map((n) =>
          n as WorkflowNode & {
            type: "atomicSkill";
            config: { skill_name?: string; entry_type?: string; entry_ref?: string; category?: string };
          }
        );

      if (atomicSkillNodes.length === 0) {
        return null;
      }

      const skillsToCheck = atomicSkillNodes.map((node) => ({
        name: node.config.skill_name || "",
        description: node.title || "",
        entry_type: node.config.entry_type || "local",
        entry_ref: node.config.entry_ref || "",
        category: node.config.category || "other",
        node_id: node.id,
      }));

      try {
        const result = await invoke<SemanticCheckResult>("check_skill_semantic_matches", {
          request: { skills: skillsToCheck },
          minSimilarity: 0.5,
        });

        set((state) => {
          state.semanticCheckResult = result;
        });

        return result;
      } catch (error) {
        console.error("Failed to check semantic matches:", error);
        return null;
      }
    },

    applySkillReplacement: (nodeId: string, existingSkillId: string, action: SkillReplacementAction) => {
      const { semanticCheckResult } = get();
      if (!semanticCheckResult) { return; }

      const match = semanticCheckResult.matches.find((m) => m.node_id === nodeId);
      if (!match) { return; }

      const replacement = match.matches.find((m) => m.existing_skill.id === existingSkillId);
      if (!replacement) { return; }

      set((state) => {
        state.pendingReplacements.set(nodeId, {
          existingSkillId,
          action,
        });

        if (action === "replace" || action === "upgrade_existing") {
          const nodeIndex = state.nodes.findIndex((n) => n.id === nodeId);
          if (nodeIndex !== -1) {
            const node = state.nodes[nodeIndex] as any;
            node.config.skill_id = existingSkillId;
            node.config.entry_ref = replacement.existing_skill.entry_ref;
          }
        }
      });
    },

    applySemanticAction: (nodeId: string, action: "replace" | "keep" | "upgrade_existing") => {
      const { semanticCheckResult } = get();
      if (!semanticCheckResult) { return; }

      const match = semanticCheckResult.matches.find((m) => m.node_id === nodeId);
      if (!match || !match.matches || match.matches.length === 0) { return; }

      const bestMatch = match.matches[0];

      set((state) => {
        state.pendingReplacements.set(nodeId, {
          existingSkillId: bestMatch.existing_skill.id,
          action,
        });

        if (action === "replace" || action === "upgrade_existing") {
          const nodeIndex = state.nodes.findIndex((n) => n.id === nodeId);
          if (nodeIndex !== -1) {
            const node = state.nodes[nodeIndex] as any;
            node.config.skill_id = bestMatch.existing_skill.id;
            node.config.skill_name = bestMatch.existing_skill.name;
            node.config.entry_ref = bestMatch.existing_skill.entry_ref;
            node.config.entry_type = bestMatch.existing_skill.entry_type;
            node.config.category = bestMatch.existing_skill.category;
            node.data.skillId = bestMatch.existing_skill.id;
            node.data.skillName = bestMatch.existing_skill.name;
          }
        }

        if (action === "keep") {
          const nodeIndex = state.nodes.findIndex((n) => n.id === nodeId);
          if (nodeIndex !== -1) {
            const node = state.nodes[nodeIndex] as any;
            node.data.semanticMatch = undefined;
          }
        }

        const remainingMatches = state.semanticCheckResult?.matches.filter((m) => m.node_id !== nodeId) || [];
        if (remainingMatches.length === 0) {
          state.semanticCheckResult = null;
        } else if (state.semanticCheckResult) {
          state.semanticCheckResult.matches = remainingMatches;
        }
      });
    },

    clearSemanticCheckResult: () => {
      set((state) => {
        state.semanticCheckResult = null;
        state.pendingReplacements = new Map();
      });
    },

    loadConversationWorkflowPreview: async (conversationId: string) => {
      try {
        const response = await invoke<ConversationWorkflowPreviewResponse>(
          "get_conversation_workflow_preview",
          { conversation_id: conversationId },
        );

        if (response.skill_count === 0) {
          throw new Error("No skill executions found in this conversation");
        }

        // D7: runtime validation — verify nodes have required 'type' and 'id' fields
        const validNodes = response.nodes.filter((n: any) => n?.type && n?.id) as WorkflowNode[];
        const validEdges = response.edges.filter((e: any) => e?.source && e?.target) as WorkflowEdge[];
        if (validNodes.length === 0) {
          throw new Error("Workflow preview contains no valid nodes");
        }

        set((state) => {
          state.importedWorkflowData = {
            nodes: validNodes,
            edges: validEdges,
            name: `Workflow from Conversation`,
            description: `Converted from conversation with ${response.skill_count} skill(s)`,
            isDecompositionWorkflow: true,
            decompositionSource: {
              market: conversationId,
              repo: response.skill_execution_order.join(", "),
              content: "",
            },
          };
          state.isDecompositionTemplate = true;
        });
      } catch (error) {
        set((state) => {
          state.error = String(error);
        });
        throw error;
      }
    },
  })),
);

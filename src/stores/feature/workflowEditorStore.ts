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
import { invoke, logIpcError } from "@/lib/invoke";
import { create } from "zustand";
import { immer } from "zustand/middleware/immer";

export interface AiChatMessage {
  role: "user" | "assistant";
  content: string;
  timestamp: number;
  id: string;
  isStreaming?: boolean;
  actions?: AiChatAction[];
  rawContent?: string;
}

export interface AiChatAction {
  action_type: string;
  data: Record<string, unknown>;
}

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

type HistoryEntry = {
  nodes: WorkflowNode[];
  edges: WorkflowEdge[];
  name: string;
  description?: string;
  icon: string;
  tags: string[];
  input_schema?: JsonSchema;
  output_schema?: JsonSchema;
  variables?: Variable[];
  error_config?: ErrorConfig;
  trigger_config?: TriggerConfig;
};

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
  past: Array<HistoryEntry>;
  future: Array<HistoryEntry>;
  _lastUndoRecordTime: number;
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
  updateTemplate: (
    id: string,
    input: WorkflowTemplateInput,
  ) => Promise<boolean>;
  deleteTemplate: (id: string) => Promise<boolean>;
  duplicateTemplate: (id: string) => Promise<string | null>;
  validateTemplate: () => Promise<ValidationResult | null>;
  exportTemplate: (id: string) => Promise<string | null>;
  importTemplate: (
    jsonData: string,
  ) => Promise<{ id: string; warnings: string[]; errors: string[] } | null>;
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
  setSimilarWorkflowsForReview: (
    workflows: SimilarWorkflow[],
    pendingData: PendingWorkflowData,
  ) => void;
  clearSimilarWorkflowsForReview: () => void;

  generateWorkflowFromPrompt: (
    prompt: string,
    mergeMode?: boolean,
  ) => Promise<
    {
      nodes: WorkflowNode[];
      edges: WorkflowEdge[];
      explanation?: string;
    } | null
  >;
  optimizeAgentPrompt: (prompt: string) => Promise<string | null>;
  recommendNodes: (
    context: string,
  ) => Promise<
    Array<{
      node_type: string;
      label: string;
      description: string;
      confidence: number;
    }> | null
  >;
  applyOptimizedPromptToNode: (nodeId: string, optimizedPrompt: string) => void;

  aiChatMessages: AiChatMessage[];
  aiChatSessionId: string;
  aiChatStreaming: boolean;
  aiChatStreamingMessageId: string | null;
  aiChatSend: (message: string) => Promise<void>;
  aiChatCancel: () => void;
  aiChatClear: () => void;
  applyAiChatAction: (action: AiChatAction) => void;

  semanticCheckResult: SemanticCheckResult | null;
  pendingReplacements: Map<
    string,
    { existingSkillId: string; action: SkillReplacementAction }
  >;
  checkSkillSemanticMatches: (
    nodes: WorkflowNode[],
  ) => Promise<SemanticCheckResult | null>;
  applySkillReplacement: (
    nodeId: string,
    existingSkillId: string,
    action: SkillReplacementAction,
  ) => void;
  applySemanticAction: (
    nodeId: string,
    action: "replace" | "keep" | "upgrade_existing",
  ) => void;
  clearSemanticCheckResult: () => void;

  loadConversationWorkflowPreview: (conversationId: string) => Promise<void>;
}

interface ConversationWorkflowPreviewResponse {
  nodes: unknown[];
  edges: unknown[];
  skill_execution_order: string[];
  skill_count: number;
}

const createEmptyTemplate = (): Omit<
  WorkflowTemplateResponse,
  "id" | "created_at" | "updated_at"
> => ({
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

const buildHistoryEntry = (state: WorkflowEditorState): HistoryEntry => ({
  nodes: [...state.nodes],
  edges: [...state.edges],
  name: state.currentTemplate?.name || "",
  description: state.currentTemplate?.description,
  icon: state.currentTemplate?.icon || "Bot",
  tags: state.currentTemplate?.tags || [],
  input_schema: state.currentTemplate?.input_schema,
  output_schema: state.currentTemplate?.output_schema,
  variables: state.currentTemplate?.variables,
  error_config: state.currentTemplate?.error_config,
  trigger_config: state.currentTemplate?.trigger_config,
});

function parseActionsFromContent(content: string): AiChatAction[] {
  const actions: AiChatAction[] = [];
  const regex = /:::action\s*\n([\s\S]*?)\n:::/g;
  let match;
  while ((match = regex.exec(content)) !== null) {
    try {
      const parsed = JSON.parse(match[1].trim());
      actions.push({
        action_type: parsed.action_type || "unknown",
        data: parsed.data || {},
      });
    } catch {
      // skip invalid JSON
    }
  }
  return actions;
}

function stripActionBlocks(content: string): string {
  return content.replace(/:::action\s*\n[\s\S]*?\n:::/g, "").trim();
}

function stripPartialActionBlocks(content: string): string {
  let result = content.replace(/:::action\s*\n[\s\S]*?\n:::/g, "");
  const partialMatch = result.match(/:::action\s*\n[\s\S]*$/);
  if (partialMatch) {
    result = result.slice(0, partialMatch.index);
  }
  return result.trim();
}

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
    aiChatMessages: [],
    aiChatSessionId: `ai-session-${Date.now()}`,
    aiChatStreaming: false,
    aiChatStreamingMessageId: null,
    past: [],
    future: [],
    _lastUndoRecordTime: 0,

    undo: () => {
      const { past } = get();
      if (past.length === 0) {
        return;
      }

      const previous = past[past.length - 1];
      set((state) => {
        state.future.push(buildHistoryEntry(state));
        state.nodes = previous.nodes;
        state.edges = previous.edges;
        if (state.currentTemplate) {
          state.currentTemplate.name = previous.name;
          state.currentTemplate.description = previous.description;
          state.currentTemplate.icon = previous.icon;
          state.currentTemplate.tags = previous.tags;
          state.currentTemplate.input_schema = previous.input_schema;
          state.currentTemplate.output_schema = previous.output_schema;
          state.currentTemplate.variables = previous.variables ?? [];
          state.currentTemplate.error_config = previous.error_config;
          state.currentTemplate.trigger_config = previous.trigger_config;
        }
        state.past = state.past.slice(0, -1);
        state.isDirty = true;
      });
    },

    redo: () => {
      const { future } = get();
      if (future.length === 0) {
        return;
      }

      const next = future[future.length - 1];
      set((state) => {
        state.past.push(buildHistoryEntry(state));
        state.nodes = next.nodes;
        state.edges = next.edges;
        if (state.currentTemplate) {
          state.currentTemplate.name = next.name;
          state.currentTemplate.description = next.description;
          state.currentTemplate.icon = next.icon;
          state.currentTemplate.tags = next.tags;
          state.currentTemplate.input_schema = next.input_schema;
          state.currentTemplate.output_schema = next.output_schema;
          state.currentTemplate.variables = next.variables ?? [];
          state.currentTemplate.error_config = next.error_config;
          state.currentTemplate.trigger_config = next.trigger_config;
        }
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
        const templates = await invoke<WorkflowTemplateResponse[]>(
          "list_workflow_templates",
          params,
        );
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
        const template = await invoke<WorkflowTemplateResponse>(
          "get_workflow_template",
          { id },
        );
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
        // 仅刷新侧栏模板列表，不重新加载当前模板（避免覆盖本地编辑中的位置/配置）
        await get().loadTemplates();
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
        const newId = await invoke<string>("duplicate_workflow_template", {
          id,
        });
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
      if (!currentTemplate) {
        return null;
      }

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
        const result = await invoke<ValidationResult>(
          "validate_workflow_template",
          { input },
        );
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
        const result = await invoke<{
          id: string;
          warnings: string[];
          errors: string[];
        }>("import_workflow_template", {
          json_data: jsonData,
        });
        await get().loadTemplates();
        set((state) => {
          state.isSaving = false;
        });
        return result;
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
        const versions = await invoke<number[]>("get_template_versions", {
          id,
        });
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
        const template = await invoke<WorkflowTemplateResponse | null>(
          "get_template_by_version",
          { id, version },
        );
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
        state.past.push(buildHistoryEntry(state));
        state.future = [];
        if (state.past.length > 50) {
          state.past = state.past.slice(-50);
        }
        state._lastUndoRecordTime = Date.now();
        state.nodes.push(node);
        state.isDirty = true;
      });
    },

    updateNode: (nodeId: string, updates: Partial<WorkflowNode>) => {
      set((state) => {
        const now = Date.now();
        if (now - state._lastUndoRecordTime >= 1000) {
          state.past.push(buildHistoryEntry(state));
          state.future = [];
          if (state.past.length > 50) {
            state.past = state.past.slice(-50);
          }
          state._lastUndoRecordTime = now;
        }
        const index = state.nodes.findIndex((n) => n.id === nodeId);
        if (index !== -1) {
          const existing = state.nodes[index];
          // 深合并嵌套对象（config / position / retry），避免浅合并覆盖未传入的字段
          const merged = {
            ...existing,
            ...updates,
            position: updates.position
              ? { ...existing.position, ...updates.position }
              : existing.position,
            config: updates.config
              ? { ...(existing as any).config, ...(updates as any).config }
              : (existing as any).config,
            retry: (updates as any).retry
              ? { ...(existing as any).retry, ...(updates as any).retry }
              : (existing as any).retry,
          } as WorkflowNode;
          state.nodes[index] = merged;
          state.isDirty = true;
        }
      });
    },

    deleteNode: (nodeId: string) => {
      set((state) => {
        state.past.push(buildHistoryEntry(state));
        state.future = [];
        if (state.past.length > 50) {
          state.past = state.past.slice(-50);
        }
        state._lastUndoRecordTime = Date.now();
        state.nodes = state.nodes.filter((n) => n.id !== nodeId);
        state.edges = state.edges.filter(
          (e) => e.source !== nodeId && e.target !== nodeId,
        );
        if (state.selectedNodeId === nodeId) {
          state.selectedNodeId = null;
        }
        state.isDirty = true;
      });
    },

    addEdge: (edge: WorkflowEdge) => {
      set((state) => {
        state.past.push(buildHistoryEntry(state));
        state.future = [];
        if (state.past.length > 50) {
          state.past = state.past.slice(-50);
        }
        state._lastUndoRecordTime = Date.now();
        state.edges.push(edge);
        state.isDirty = true;
      });
    },

    updateEdge: (edgeId: string, updates: Partial<WorkflowEdge>) => {
      set((state) => {
        const now = Date.now();
        if (now - state._lastUndoRecordTime >= 1000) {
          state.past.push(buildHistoryEntry(state));
          state.future = [];
          if (state.past.length > 50) {
            state.past = state.past.slice(-50);
          }
          state._lastUndoRecordTime = now;
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
        state.past.push(buildHistoryEntry(state));
        state.future = [];
        if (state.past.length > 50) {
          state.past = state.past.slice(-50);
        }
        state._lastUndoRecordTime = Date.now();
        state.edges = state.edges.filter((e) => e.id !== edgeId);
        if (state.selectedEdgeId === edgeId) {
          state.selectedEdgeId = null;
        }
        state.isDirty = true;
      });
    },

    setNodes: (nodes: WorkflowNode[]) => {
      set((state) => {
        state.past.push(buildHistoryEntry(state));
        state.future = [];
        if (state.past.length > 50) {
          state.past = state.past.slice(-50);
        }
        state._lastUndoRecordTime = Date.now();
        state.nodes = nodes;
        state.isDirty = true;
      });
    },

    setEdges: (edges: WorkflowEdge[]) => {
      set((state) => {
        state.past.push(buildHistoryEntry(state));
        state.future = [];
        if (state.past.length > 50) {
          state.past = state.past.slice(-50);
        }
        state._lastUndoRecordTime = Date.now();
        state.edges = edges;
        state.isDirty = true;
      });
    },

    updateTemplateMetadata: (metadata) => {
      set((state) => {
        if (state.currentTemplate) {
          const now = Date.now();
          if (now - state._lastUndoRecordTime >= 1000) {
            state.past.push(buildHistoryEntry(state));
            state.future = [];
            if (state.past.length > 50) {
              state.past = state.past.slice(-50);
            }
            state._lastUndoRecordTime = now;
          }
          if (metadata.name !== undefined) {
            state.currentTemplate.name = metadata.name;
          }
          if (metadata.description !== undefined) {
            state.currentTemplate.description = metadata.description;
          }
          if (metadata.icon !== undefined) {
            state.currentTemplate.icon = metadata.icon;
          }
          if (metadata.tags !== undefined) {
            state.currentTemplate.tags = metadata.tags;
          }
          if (metadata.triggerConfig !== undefined) {
            state.currentTemplate.trigger_config = metadata.triggerConfig;
          }
          if ("inputSchema" in metadata) {
            state.currentTemplate.input_schema = metadata.inputSchema;
          }
          if ("outputSchema" in metadata) {
            state.currentTemplate.output_schema = metadata.outputSchema;
          }
          if (metadata.variables !== undefined) {
            state.currentTemplate.variables = metadata.variables;
          }
          if (metadata.errorConfig !== undefined) {
            state.currentTemplate.error_config = metadata.errorConfig;
          }
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
          ...(importedData?.description && {
            description: importedData.description,
          }),
          id: "",
          created_at: Date.now(),
          updated_at: Date.now(),
        } as WorkflowTemplateResponse;
        state.nodes = importedData?.nodes || [];
        state.edges = importedData?.edges || [];
        state.isDirty = !!(
          importedData?.nodes && importedData.nodes.length > 0
        );
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

    saveDecompositionWorkflow: async (
      workflowName: string,
      workflowDescription?: string,
    ) => {
      const { isDecompositionTemplate, pendingDecompositionSource } = get();
      if (!isDecompositionTemplate || !pendingDecompositionSource) {
        throw new Error("Not a decomposition workflow or missing source data");
      }

      set((state) => {
        state.isSaving = true;
        state.error = null;
      });

      try {
        const result = await invoke<{
          workflow_id: string;
          saved_skills: number;
        }>("confirm_decomposition", {
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

    saveSkillWorkflowFromLlm: async (
      workflowName: string,
      workflowDescription?: string,
    ) => {
      const {
        isDecompositionTemplate,
        pendingDecompositionSource,
        nodes,
        edges,
      } = get();
      if (!isDecompositionTemplate || !pendingDecompositionSource) {
        throw new Error("Not a decomposition workflow or missing source data");
      }

      set((state) => {
        state.isSaving = true;
        state.error = null;
      });

      try {
        const response = await invoke<SaveSkillWorkflowResponse>(
          "save_skill_workflow_from_llm",
          {
            request: {
              skill_id: pendingDecompositionSource.market,
              skill_name: pendingDecompositionSource.repo
                || pendingDecompositionSource.market,
              workflow_name: workflowName,
              description: workflowDescription,
              nodes,
              edges,
            },
          },
        );

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

    forceSaveSkillWorkflow: async (
      targetWorkflowId: string,
      workflowName: string,
      workflowDescription?: string,
    ) => {
      const {
        isDecompositionTemplate,
        pendingDecompositionSource,
        nodes,
        edges,
      } = get();
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
            skill_name: pendingDecompositionSource.repo
              || pendingDecompositionSource.market,
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

    generateWorkflowFromPrompt: async (prompt: string, mergeMode?: boolean) => {
      set((state) => {
        state.isLoading = true;
        state.error = null;
      });
      try {
        const { nodes, edges } = get();
        const result = await invoke<{
          nodes: WorkflowNode[];
          edges: WorkflowEdge[];
          explanation?: string;
        }>("generate_workflow_from_prompt", {
          prompt,
          current_nodes: nodes.length > 0 ? nodes : undefined,
          current_edges: edges.length > 0 ? edges : undefined,
        });
        if (result) {
          set((state) => {
            if (mergeMode && state.nodes.length > 0) {
              const existingIds = new Set(state.nodes.map(n => n.id));
              const prefix = `ai-${Date.now()}`;
              const newNodes = result.nodes.map(n => ({
                ...n,
                id: existingIds.has(n.id) ? `${prefix}-${n.id}` : n.id,
                position: { x: n.position.x + 50, y: n.position.y + 50 },
              }));
              const nodeIdMap = new Map<string, string>();
              result.nodes.forEach((orig, i) => {
                if (newNodes[i].id !== orig.id) {
                  nodeIdMap.set(orig.id, newNodes[i].id);
                }
              });
              const newEdges = result.edges.map(e => ({
                ...e,
                id: `ai-edge-${Date.now()}-${e.id}`,
                source: nodeIdMap.get(e.source) || e.source,
                target: nodeIdMap.get(e.target) || e.target,
              }));
              state.nodes = [...state.nodes, ...newNodes];
              state.edges = [...state.edges, ...newEdges];
            } else {
              state.nodes = result.nodes;
              state.edges = result.edges;
            }
            state.isLoading = false;
          });
          return {
            nodes: get().nodes,
            edges: get().edges,
            explanation: result.explanation,
          };
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
        const result = await invoke<string>("optimize_agent_prompt", {
          prompt,
        });
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
        const { nodes } = get();
        const currentNodeTypes = nodes.map(n => n.type).filter(Boolean) as string[];
        const result = await invoke<
          Array<{
            node_type: string;
            label: string;
            description: string;
            confidence: number;
          }>
        >("recommend_nodes", {
          context,
          current_node_types: currentNodeTypes.length > 0 ? currentNodeTypes : undefined,
        });
        set((state) => {
          state.isLoading = false;
        });
        return result ?? null;
      } catch (error) {
        set((state) => {
          state.error = String(error);
          state.isLoading = false;
        });
        return null;
      }
    },

    applyOptimizedPromptToNode: (nodeId: string, optimizedPrompt: string) => {
      const { nodes } = get();
      const node = nodes.find(n => n.id === nodeId);
      if (!node) { return; }
      if (node.type === "agent") {
        const agentNode = node as import("@/components/workflow/types").AgentNode;
        get().updateNode(nodeId, {
          ...agentNode,
          config: { ...agentNode.config, system_prompt: optimizedPrompt },
        });
      }
    },

    aiChatSend: async (message: string) => {
      const { aiChatMessages, aiChatSessionId } = get();
      const msgId = `user-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
      const assistantId = `assistant-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
      const userMsg: AiChatMessage = {
        role: "user",
        content: message,
        timestamp: Date.now(),
        id: msgId,
      };
      const assistantMsg: AiChatMessage = {
        role: "assistant",
        content: "",
        timestamp: Date.now(),
        id: assistantId,
        isStreaming: true,
        actions: [],
        rawContent: "",
      };
      set((state) => {
        state.aiChatMessages = [...state.aiChatMessages, userMsg, assistantMsg];
        state.aiChatStreaming = true;
        state.aiChatStreamingMessageId = assistantMsg.id;
      });
      let chunkUnlisten: (() => void) | null = null;
      let errorUnlisten: (() => void) | null = null;
      const cleanupListeners = () => {
        chunkUnlisten?.();
        errorUnlisten?.();
        chunkUnlisten = null;
        errorUnlisten = null;
      };
      try {
        const history = aiChatMessages.map((m) => ({
          role: m.role,
          content: (m as any).rawContent || m.content,
        }));
        const { listen } = await import("@/lib/invoke");
        let accumulatedContent = "";
        chunkUnlisten = await listen<
          { conversation_id: string; message_id: string; chunk: { content: string | null; done: boolean } }
        >(
          "workflow-ai-chat-chunk",
          (event) => {
            if (event.payload.conversation_id !== aiChatSessionId) { return; }
            const chunk = event.payload.chunk;
            if (chunk.content) {
              accumulatedContent += chunk.content;
            }
            if (chunk.done) {
              const actions = parseActionsFromContent(accumulatedContent);
              const cleanContent = stripActionBlocks(accumulatedContent);
              set((state) => {
                state.aiChatMessages = state.aiChatMessages.map((m) =>
                  m.id === assistantMsg.id
                    ? { ...m, content: cleanContent, isStreaming: false, actions, rawContent: accumulatedContent }
                    : m
                );
                state.aiChatStreaming = false;
                state.aiChatStreamingMessageId = null;
              });
              cleanupListeners();
            } else {
              const displayContent = stripPartialActionBlocks(accumulatedContent);
              set((state) => {
                state.aiChatMessages = state.aiChatMessages.map((m) =>
                  m.id === assistantMsg.id
                    ? { ...m, content: displayContent + "▍", isStreaming: true, rawContent: accumulatedContent }
                    : m
                );
              });
            }
          },
        );
        errorUnlisten = await listen<{ conversation_id: string; error: string }>(
          "workflow-ai-chat-error",
          (event) => {
            if (event.payload.conversation_id !== aiChatSessionId) { return; }
            set((state) => {
              state.aiChatMessages = state.aiChatMessages.map((m) =>
                m.id === assistantMsg.id
                  ? { ...m, content: m.content + `\n\n❌ Error: ${event.payload.error}`, isStreaming: false }
                  : m
              );
              state.aiChatStreaming = false;
              state.aiChatStreamingMessageId = null;
            });
            cleanupListeners();
          },
        );
        await invoke("workflow_ai_chat_stream", {
          message,
          history,
          current_nodes: get().nodes.length > 0 ? get().nodes : undefined,
          current_edges: get().edges.length > 0 ? get().edges : undefined,
          session_id: aiChatSessionId,
        });
      } catch (error) {
        logIpcError("AI Chat")(error);
        cleanupListeners();
        set((state) => {
          state.aiChatMessages = state.aiChatMessages.map((m) =>
            m.id === assistantMsg.id
              ? { ...m, content: `❌ ${String(error)}`, isStreaming: false }
              : m
          );
          state.aiChatStreaming = false;
          state.aiChatStreamingMessageId = null;
        });
      }
    },

    aiChatCancel: () => {
      const { aiChatSessionId, aiChatStreamingMessageId } = get();
      invoke("workflow_ai_chat_cancel", { session_id: aiChatSessionId }).catch(logIpcError("AI Chat Cancel"));
      set((state) => {
        state.aiChatMessages = state.aiChatMessages.map((m) =>
          m.id === aiChatStreamingMessageId
            ? { ...m, isStreaming: false }
            : m
        );
        state.aiChatStreaming = false;
        state.aiChatStreamingMessageId = null;
      });
    },

    aiChatClear: () => {
      set((state) => {
        state.aiChatMessages = [];
        state.aiChatSessionId = `ai-session-${Date.now()}`;
      });
    },

    applyAiChatAction: (action: AiChatAction) => {
      const { nodes } = get();
      switch (action.action_type) {
        case "generate_workflow": {
          const data = action.data as { nodes: WorkflowNode[]; edges: WorkflowEdge[] };
          if (data.nodes && data.edges) {
            set((state) => {
              state.nodes = data.nodes;
              state.edges = data.edges;
            });
          }
          break;
        }
        case "add_nodes": {
          const data = action.data as { nodes: WorkflowNode[] };
          if (data.nodes) {
            const existingIds = new Set(nodes.map(n => n.id));
            const newNodes = data.nodes.map(n => ({
              ...n,
              id: existingIds.has(n.id) ? `ai-${Date.now()}-${n.id}` : n.id,
              position: { x: n.position.x + 50, y: n.position.y + 50 },
            }));
            set((state) => {
              state.nodes = [...state.nodes, ...newNodes];
            });
          }
          break;
        }
        case "modify_node": {
          const data = action.data as { node_id: string; changes: Record<string, unknown> };
          if (data.node_id) {
            set((state) => {
              state.nodes = state.nodes.map(n => {
                if (n.id !== data.node_id) { return n; }
                const changes = { ...data.changes };
                if (changes.config && typeof changes.config === "object" && n.config) {
                  changes.config = { ...n.config, ...changes.config };
                }
                return { ...n, ...changes };
              });
            });
          }
          break;
        }
        case "optimize_prompt": {
          const data = action.data as { node_id: string; optimized_prompt: string };
          if (data.node_id && data.optimized_prompt) {
            get().applyOptimizedPromptToNode(data.node_id, data.optimized_prompt);
          }
          break;
        }
        case "delete_nodes": {
          const data = action.data as { node_ids: string[] };
          if (data.node_ids) {
            const idsToDelete = new Set(data.node_ids);
            set((state) => {
              state.nodes = state.nodes.filter(n => !idsToDelete.has(n.id));
              state.edges = state.edges.filter(e => !idsToDelete.has(e.source) && !idsToDelete.has(e.target));
            });
          }
          break;
        }
      }
    },

    semanticCheckResult: null,
    pendingReplacements: new Map(),

    checkSkillSemanticMatches: async (_nodes: WorkflowNode[]) => {
      // atomicSkill nodes removed — no matching needed
      return null;
    },

    applySkillReplacement: (
      _nodeId: string,
      _existingSkillId: string,
      _action: SkillReplacementAction,
    ) => {
      // atomicSkill nodes removed — no replacement needed
    },

    applySemanticAction: (
      nodeId: string,
      _action: "replace" | "keep" | "upgrade_existing",
    ) => {
      const { semanticCheckResult } = get();
      if (!semanticCheckResult) {
        return;
      }

      const match = semanticCheckResult.matches.find(
        (m) => m.node_id === nodeId,
      );
      if (!match || !match.matches || match.matches.length === 0) {
        return;
      }

      // atomicSkill removed — noop
      set((state) => {
        const remainingMatches = state.semanticCheckResult?.matches.filter(
          (m) => m.node_id !== nodeId,
        ) || [];
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
          { conversationId: conversationId },
        );

        if (response.skill_count === 0) {
          throw new Error(
            "WORKFLOW_NO_SKILL_EXECUTIONS: No skill executions found in this conversation",
          );
        }

        // D7: runtime validation — verify nodes have required 'type' and 'id' fields
        const validNodes = response.nodes.filter(
          (n: any) => n?.type && n?.id,
        ) as WorkflowNode[];
        const validEdges = response.edges.filter(
          (e: any) => e?.source && e?.target,
        ) as WorkflowEdge[];
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

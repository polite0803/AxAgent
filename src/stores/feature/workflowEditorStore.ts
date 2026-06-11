// SPDX-License-Identifier: AGPL-3.0-only

import type {
  AiChatAction,
  DiagnosticFix,
  DiagnosticIssue,
  DiagnosticReport,
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

export interface ExpandedSubWorkflowData {
  /** 子工作流内部节点（ID 已 prefixed 避免冲突） */
  nodes: WorkflowNode[];
  /** 子工作流内部边（ID 已 prefixed 避免冲突） */
  edges: WorkflowEdge[];
  /** 是否正在加载 */
  isLoading: boolean;
}
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
  diagnoseReport: DiagnosticReport | null;
  diagnoseLoading: boolean;
  diagnoseApplying: boolean;
  diagnoseDrawerVisible: boolean;
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
  // 容器父子关系（childId → parentId），独立于 nodes 数组以避免污染 WorkflowNode 联合类型。
  // 渲染时反查此表为 ReactFlow 节点注入 parentId，保存时摊平到 nodes.parentId 字段。
  parentRefs: Record<string, string>;
  setParentRef: (childId: string, parentId: string | null) => void;
  clearParentRefs: () => void;

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

  llmDiagnoseWorkflow: (
    nodes: WorkflowNode[],
    workflowName: string,
    description?: string,
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
  ) => Promise<any>;

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
  /**
   * 将 AI 生成结果应用到节点的指定字段。
   * 用于 Phase 1 节点级 AI 辅助（如 LLM.prompt、Agent.system_prompt、HttpRequest.url、Email.body 等）。
   * - kind = "string" 时，value 必须是字符串，写入 config[field]
   * - kind = "object" 时，value 是任意 JSON 兼容对象，写入 config[field]
   */
  applyAIAssistToNodeField: (
    nodeId: string,
    field: string,
    value: unknown,
    kind?: "string" | "object",
  ) => boolean;

  runWorkflowDiagnose: () => Promise<DiagnosticReport | null>;
  clearDiagnoseReport: () => void;
  setDiagnoseDrawerVisible: (visible: boolean) => void;
  applyDiagnoseFix: (issueId: string) => boolean;

  aiChatMessages: AiChatMessage[];
  aiChatSessionId: string;
  aiChatStreaming: boolean;
  aiChatStreamingMessageId: string | null;
  /** AI 聊天 listener 清理函数，由 aiChatSend 设置、aiChatCancel 调用 */
  _aiChatCleanup: (() => void) | null;
  aiChatSend: (message: string) => Promise<void>;
  aiChatCancel: () => void;
  aiChatClear: () => void;
  applyAiChatAction: (action: AiChatAction) => void;
  /**
   * 事务性 AI action 批处理：一组 actions 要么全部应用、要么一键回滚。
   * - beginAiActionTransaction  拍快照（保存当前 nodes/edges 副本）
   * - applyAiChatAction         在事务内逐个应用
   * - commitAiActionTransaction 成功完成，丢弃快照
   * - rollbackAiActionTransaction 回滚到事务开始前的状态
   */
  aiActionTransactions: Array<{
    id: string;
    timestamp: number;
    appliedCount: number;
    beforeNodes: WorkflowNode[];
    beforeEdges: WorkflowEdge[];
  }>;
  beginAiActionTransaction: () => string;
  applyAiChatActionInTransaction: (txId: string, action: AiChatAction) => void;
  commitAiActionTransaction: (txId: string) => void;
  rollbackAiActionTransaction: (txId: string) => void;
  rollbackLastAiActionTransaction: () => void;

  /**
   * 待用户在 Diff 预览中确认的 AI action 队列。
   * 为 null 表示 DiffPreview 弹窗关闭；非空时显示在 ActionDiffPreview 中。
   * 用户确认 apply 走 applyAiChatAction；cancel 走 clearPendingAiChatActions。
   */
  pendingAiChatActions: AiChatAction[] | null;
  pendingAiChatMessageId: string | null;
  setPendingAiChatActions: (messageId: string, actions: AiChatAction[]) => void;
  clearPendingAiChatActions: () => void;

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

  /** 已展开的子工作流（keyed by 子工作流节点 ID），null = 未展开/已折叠 */
  expandedSubWorkflows: Record<string, ExpandedSubWorkflowData | null>;
  /** 切换子工作流节点的展开/折叠状态 */
  toggleExpandSubWorkflow: (nodeId: string, subWorkflowId: string | undefined) => Promise<void>;

  /** 已折叠的容器 ID 集合（会话内 UI 状态，不持久化到后端） */
  collapsedContainers: Set<string>;
  /** 切换容器的展开/折叠状态 */
  toggleContainerCollapse: (parallelId: string) => void;
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

// 从 nodes 中已有的 (as any).parentId 字段重建父子关系映射。
// 后端目前不感知 parentRefs，所以老工作流的父子关系以 nodes 字段为准持久化。
function rebuildParentRefsFromNodes(nodes: WorkflowNode[]): Record<string, string> {
  const refs: Record<string, string> = {};
  for (const n of nodes) {
    const pid = (n as { parentId?: string }).parentId;
    if (typeof pid === "string" && pid.length > 0) {
      refs[n.id] = pid;
    }
  }
  return refs;
}

function parseActionsFromContent(content: string): AiChatAction[] {
  const actions: AiChatAction[] = [];
  const regex = /:::action\s*\n([\s\S]*?)\n:::/g;
  let match;
  while ((match = regex.exec(content)) !== null) {
    try {
      const parsed = JSON.parse(match[1].trim());
      const actionType = parsed.action_type;
      const data = parsed.data ?? {};
      // 只接受已知的 action_type；未知类型丢弃（避免下游 switch 出现"幽灵分支"）
      const known: ReadonlyArray<AiChatAction["action_type"]> = [
        "generate_workflow",
        "add_node",
        "add_nodes",
        "update_node",
        "modify_node",
        "delete_node",
        "delete_nodes",
        "add_edge",
        "update_edge",
        "delete_edge",
        "optimize_prompt",
      ];
      if (known.includes(actionType)) {
        actions.push({ action_type: actionType, data } as AiChatAction);
      }
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

function mergeReports(ruleReport: DiagnosticReport, llmReport: DiagnosticReport): DiagnosticReport {
  const seen = new Set<string>();
  const issues: DiagnosticIssue[] = [];
  for (const iss of ruleReport.issues) {
    const key = `${iss.id}:${iss.node_ids.join(",")}`;
    if (!seen.has(key)) {
      seen.add(key);
      issues.push(iss);
    }
  }
  for (const iss of llmReport.issues) {
    const key = `${iss.id}:${iss.node_ids?.join(",") ?? ""}`;
    if (!seen.has(key)) {
      seen.add(key);
      issues.push(iss);
    }
  }
  const summary = { error: 0, warning: 0, info: 0 };
  for (const iss of issues) { summary[iss.severity]++; }
  return {
    issues,
    summary,
    generated_at: Date.now(),
    duration_ms: ruleReport.duration_ms + (llmReport.duration_ms ?? 0),
  };
}

interface LlmDiagnoseRaw {
  summary: string;
  issues: Array<{
    severity: string;
    category: string;
    node_id: string | null;
    title: string;
    detail: string;
    suggestion: string;
  }>;
  suggestions: string[];
}

function transformLlmResult(raw: LlmDiagnoseRaw): DiagnosticReport {
  const validSeverities = new Set(["error", "warning", "info"]);
  const issues: DiagnosticIssue[] = (raw.issues ?? []).map((iss, idx) => ({
    id: `llm_${idx}_${iss.category}`,
    severity: (validSeverities.has(iss.severity) ? iss.severity : "info") as DiagnosticIssue["severity"],
    category: iss.category as DiagnosticIssue["category"],
    title_key: "",
    message_key: "",
    node_ids: iss.node_id ? [iss.node_id] : [],
    auto_fixable: false,
    title_override: iss.title,
    detail_override: iss.detail,
    suggestion_override: iss.suggestion,
  }));
  const summary = { error: 0, warning: 0, info: 0 };
  for (const iss of issues) { summary[iss.severity]++; }
  return {
    issues,
    summary,
    generated_at: Date.now(),
    duration_ms: 0,
  };
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
    diagnoseReport: null,
    diagnoseLoading: false,
    diagnoseApplying: false,
    diagnoseDrawerVisible: false,
    filter: {},
    error: null,
    importedWorkflowData: null,
    isDecompositionTemplate: false,
    pendingDecompositionSource: null,
    similarWorkflowsForReview: [],
    pendingWorkflowData: null,
    nodes: [],
    edges: [],
    parentRefs: {},
    aiChatMessages: [],
    aiChatSessionId: `ai-session-${Date.now()}`,
    aiChatStreaming: false,
    aiChatStreamingMessageId: null,
    _aiChatCleanup: null,
    pendingAiChatActions: null,
    pendingAiChatMessageId: null,
    expandedSubWorkflows: {},
    collapsedContainers: new Set<string>(
      (() => {
        try {
          const v = localStorage.getItem("workflow_collapsed_containers");
          return v ? JSON.parse(v) as string[] : [];
        } catch {
          return [];
        }
      })(),
    ),
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
          state.parentRefs = rebuildParentRefsFromNodes(template.nodes);
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
        // 刷新侧栏列表，同时刷新当前模板（确保 version 等元数据同步）
        const { currentTemplate } = get();
        if (currentTemplate?.id === id) {
          await get().loadTemplate(id);
        } else {
          await get().loadTemplates();
        }
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
            state.parentRefs = rebuildParentRefsFromNodes(state.nodes);
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

    /** 从联合类型 WorkflowNode 中无损提取 config/retry 做深合并。
     *  各变体 config 类型不同，通过 'unknown' 中转避免 'as any' 扩散。 */
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
          // 深合并嵌套对象：联合类型各变体 config/retry 类型不同，
          // 通过 unknown 中转精确读取共有字段，避免 as any 扩散到整行
          const ext = existing as unknown as { config: Record<string, unknown>; retry: Record<string, unknown> };
          const upd = updates as unknown as { config?: Record<string, unknown>; retry?: Record<string, unknown> };
          const merged = {
            ...existing,
            ...updates,
            position: updates.position
              ? { ...existing.position, ...updates.position }
              : existing.position,
            config: upd.config
              ? { ...ext.config, ...upd.config, conditions: upd.config.conditions ?? ext.config.conditions }
              : ext.config,
            retry: upd.retry
              ? { ...ext.retry, ...upd.retry }
              : ext.retry,
          } as unknown as WorkflowNode;
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

        // 级联删除：若被删节点是 parallel 容器，需一并删除所有 parentRefs 登记为它的子节点
        const toDelete = new Set<string>([nodeId]);
        for (const [cid, pid] of Object.entries(state.parentRefs)) {
          if (pid === nodeId) { toDelete.add(cid); }
        }

        state.nodes = state.nodes.filter((n) => !toDelete.has(n.id));
        state.edges = state.edges.filter(
          (e) => !toDelete.has(e.source) && !toDelete.has(e.target),
        );

        // 清理 parentRefs 中被删节点作为子或作为父的登记项
        const nextParentRefs: Record<string, string> = {};
        for (const [k, v] of Object.entries(state.parentRefs)) {
          if (!toDelete.has(k) && !toDelete.has(v)) {
            nextParentRefs[k] = v;
          }
        }
        state.parentRefs = nextParentRefs;

        // 清理被删节点的折叠状态（含级联删除的子节点）
        if (toDelete.size === 1 && toDelete.has(nodeId)) {
          if (state.collapsedContainers.has(nodeId)) {
            const next = new Set(state.collapsedContainers);
            next.delete(nodeId);
            state.collapsedContainers = next;
            try {
              localStorage.setItem("workflow_collapsed_containers", JSON.stringify([...next]));
            } catch { /* localStorage may be full */ }
          }
        } else if (toDelete.size > 0) {
          const next = new Set(state.collapsedContainers);
          let changed = false;
          for (const id of toDelete) {
            if (next.delete(id)) { changed = true; }
          }
          if (changed) {
            state.collapsedContainers = next;
            try {
              localStorage.setItem("workflow_collapsed_containers", JSON.stringify([...next]));
            } catch { /* localStorage may be full */ }
          }
        }

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

    // 写入/清除容器父子关系。不进撤销栈（避免每次回填都被用户撤销一次）。
    setParentRef: (childId: string, parentId: string | null) => {
      set((state) => {
        if (parentId === null) {
          delete state.parentRefs[childId];
        } else {
          state.parentRefs[childId] = parentId;
        }
        state.isDirty = true;
      });
    },

    clearParentRefs: () => {
      set((state) => {
        state.parentRefs = {};
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
        state.parentRefs = rebuildParentRefsFromNodes(state.nodes);
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

    llmDiagnoseWorkflow: async (nodes: WorkflowNode[], workflowName: string, description?: string) => {
      try {
        const { invoke } = await import("@/lib/invoke");
        return await invoke("llm_diagnose_workflow", {
          request: { nodes, workflow_name: workflowName, workflow_description: description || null },
        });
      } catch {
        return null;
      }
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
      } else if (node.type === "llm") {
        const llmNode = node as import("@/components/workflow/types").LLMNode;
        get().updateNode(nodeId, {
          ...llmNode,
          config: { ...llmNode.config, prompt: optimizedPrompt },
        });
      } else if (node.type === "email") {
        const emailNode = node as import("@/components/workflow/types").EmailNode;
        get().updateNode(nodeId, {
          ...emailNode,
          config: { ...emailNode.config, body: optimizedPrompt },
        });
      }
    },

    applyAIAssistToNodeField: (
      nodeId: string,
      field: string,
      value: unknown,
      kind: "string" | "object" = "string",
    ) => {
      const { nodes } = get();
      const node = nodes.find((n) => n.id === nodeId);
      if (!node) { return false; }
      const currentConfig = (node as unknown as { config?: Record<string, unknown> }).config ?? {};
      const sanitized = kind === "string" && typeof value === "string"
        ? value
        : kind === "string"
        ? String(value ?? "")
        : value;
      get().updateNode(nodeId, {
        ...node,
        config: { ...currentConfig, [field]: sanitized },
      } as unknown as Partial<import("@/components/workflow/types").WorkflowNode>);
      return true;
    },

    runWorkflowDiagnose: async () => {
      const { nodes, edges } = get();
      if (nodes.length === 0) {
        set((s) => {
          s.diagnoseReport = {
            issues: [],
            summary: { error: 0, warning: 0, info: 0 },
            generated_at: Date.now(),
            duration_ms: 0,
          };
          s.diagnoseLoading = false;
        });
        return null;
      }
      set((s) => {
        s.diagnoseLoading = true;
        s.diagnoseDrawerVisible = true;
      });

      const { runDiagnosticRules } = await import(
        "@/components/workflow/Diagnostic/diagnosticRules"
      );
      const ruleReport = runDiagnosticRules(nodes, edges);

      try {
        const workflowName = get().currentTemplate?.name ?? "Untitled";
        const llmRaw = await invoke<{
          summary: string;
          issues: Array<{
            severity: string;
            category: string;
            node_id: string | null;
            title: string;
            detail: string;
            suggestion: string;
          }>;
          suggestions: string[];
        }>("llm_diagnose_workflow", {
          request: {
            nodes,
            workflow_name: workflowName,
            workflow_description: null,
          },
        });
        const llmReport = transformLlmResult(llmRaw);
        const merged = mergeReports(ruleReport, llmReport);
        set((s) => {
          s.diagnoseReport = merged;
          s.diagnoseLoading = false;
        });
        return merged;
      } catch {
        set((s) => {
          s.diagnoseReport = ruleReport;
          s.diagnoseLoading = false;
        });
        return ruleReport;
      }
    },

    clearDiagnoseReport: () => {
      set((s) => {
        s.diagnoseReport = null;
        s.diagnoseDrawerVisible = false;
      });
    },

    setDiagnoseDrawerVisible: (visible: boolean) => {
      set((s) => {
        s.diagnoseDrawerVisible = visible;
      });
    },

    applyDiagnoseFix: (issueId: string) => {
      const { diagnoseReport, nodes, edges } = get();
      if (!diagnoseReport) { return false; }
      const issue = diagnoseReport.issues.find((i) => i.id === issueId);
      if (!issue || !issue.auto_fixable || !issue.fix) { return false; }
      const fix: DiagnosticFix = issue.fix;
      set((s) => {
        s.diagnoseApplying = true;
      });
      let success = false;
      try {
        switch (fix.action_type) {
          case "delete_node": {
            if (!nodes.find((n) => n.id === fix.node_id)) { break; }
            get().deleteNode(fix.node_id);
            success = true;
            break;
          }
          case "delete_edge": {
            if (!edges.find((e) => e.id === fix.edge_id)) { break; }
            get().deleteEdge(fix.edge_id);
            success = true;
            break;
          }
          case "set_node_field": {
            success = get().applyAIAssistToNodeField(fix.node_id, fix.field, fix.value, "string");
            break;
          }
          case "set_timeout": {
            get().updateNode(
              fix.node_id,
              { timeout: fix.timeout_ms } as unknown as Partial<WorkflowNode>,
            );
            success = true;
            break;
          }
          case "enable_retry": {
            get().updateNode(
              fix.node_id,
              {
                retry: {
                  max_retries: fix.max_retries,
                  backoff: "exponential",
                  initial_interval_ms: 1000,
                },
              } as unknown as Record<string, unknown>,
            );
            success = true;
            break;
          }
          case "remove_debater_step": {
            const debate = nodes.find((n) => n.id === fix.node_id);
            if (!debate || debate.type !== "debate") { break; }
            const cfg = (debate as unknown as {
              config: {
                debater_steps: string[];
                subGraph?: { nodes: Array<{ id: string }>; edges: Array<{ source: string; target: string }> };
              };
            }).config;
            if (!cfg.debater_steps.includes(fix.step_id)) { break; }
            const newSteps = cfg.debater_steps.filter((s) => s !== fix.step_id);
            const newSubNodes = cfg.subGraph?.nodes.filter((n) => n.id !== fix.step_id) ?? [];
            const newSubEdges = cfg.subGraph?.edges.filter(
              (e) => e.source !== fix.step_id && e.target !== fix.step_id,
            ) ?? [];
            get().updateNode(fix.node_id, {
              ...(debate as object),
              config: {
                ...cfg,
                debater_steps: newSteps,
                subGraph: { nodes: newSubNodes, edges: newSubEdges },
              },
            } as unknown as Partial<WorkflowNode>);
            success = true;
            break;
          }
          default:
            break;
        }
      } finally {
        set((s) => {
          s.diagnoseApplying = false;
        });
      }
      return success;
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
          content: m.rawContent || m.content,
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
        // 将 cleanup 挂到 store 上，供 aiChatCancel 调用
        set((state) => {
          state._aiChatCleanup = cleanupListeners;
        });

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
      const { aiChatSessionId, aiChatStreamingMessageId, _aiChatCleanup } = get();
      // 先取消后端流，再清理 listener
      invoke("workflow_ai_chat_cancel", { session_id: aiChatSessionId }).catch(logIpcError("AI Chat Cancel"));
      _aiChatCleanup?.();
      set((state) => {
        state._aiChatCleanup = null;
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
      const { nodes, edges } = get();
      switch (action.action_type) {
        case "generate_workflow": {
          set((state) => {
            state.nodes = action.data.nodes;
            state.edges = action.data.edges;
          });
          break;
        }
        case "add_node": {
          const newNode = action.data.node;
          const existingIds = new Set(nodes.map(n => n.id));
          const finalId = existingIds.has(newNode.id) ? `ai-${Date.now()}-${newNode.id}` : newNode.id;
          const offset = action.data.position ?? { x: 50, y: 50 };
          set((state) => {
            state.nodes = [...state.nodes, {
              ...newNode,
              id: finalId,
              position: { x: newNode.position.x + offset.x, y: newNode.position.y + offset.y },
            }];
          });
          break;
        }
        case "add_nodes": {
          const existingIds = new Set(nodes.map(n => n.id));
          const newNodes = action.data.nodes.map(n => ({
            ...n,
            id: existingIds.has(n.id) ? `ai-${Date.now()}-${n.id}` : n.id,
            position: { x: n.position.x + 50, y: n.position.y + 50 },
          }));
          set((state) => {
            state.nodes = [...state.nodes, ...newNodes];
          });
          break;
        }
        case "update_node":
        case "modify_node": {
          const { node_id, changes } = action.data;
          if (node_id) {
            set((state) => {
              state.nodes = state.nodes.map(n => {
                if (n.id !== node_id) { return n; }
                const merged: Record<string, unknown> = { ...changes };
                if (merged.config && typeof merged.config === "object" && n.config) {
                  merged.config = { ...n.config, ...merged.config };
                }
                return { ...n, ...merged } as WorkflowNode;
              });
            });
          }
          break;
        }
        case "delete_node": {
          const id = action.data.node_id;
          if (id) {
            set((state) => {
              state.nodes = state.nodes.filter(n => n.id !== id);
              state.edges = state.edges.filter(e => e.source !== id && e.target !== id);
            });
          }
          break;
        }
        case "delete_nodes": {
          const idsToDelete = new Set(action.data.node_ids);
          if (idsToDelete.size > 0) {
            set((state) => {
              state.nodes = state.nodes.filter(n => !idsToDelete.has(n.id));
              state.edges = state.edges.filter(e => !idsToDelete.has(e.source) && !idsToDelete.has(e.target));
            });
          }
          break;
        }
        case "add_edge": {
          const newEdge = action.data.edge;
          const exists = edges.some(e => e.id === newEdge.id);
          if (!exists) {
            set((state) => {
              state.edges = [...state.edges, newEdge];
            });
          }
          break;
        }
        case "update_edge": {
          const { edge_id, changes } = action.data;
          if (edge_id) {
            set((state) => {
              state.edges = state.edges.map(e => (e.id === edge_id ? { ...e, ...changes } : e));
            });
          }
          break;
        }
        case "delete_edge": {
          const id = action.data.edge_id;
          if (id) {
            set((state) => {
              state.edges = state.edges.filter(e => e.id !== id);
            });
          }
          break;
        }
        case "optimize_prompt": {
          const { node_id, optimized_prompt } = action.data;
          if (node_id && optimized_prompt) {
            get().applyOptimizedPromptToNode(node_id, optimized_prompt);
          }
          break;
        }
      }
    },

    setPendingAiChatActions: (messageId: string, actions: AiChatAction[]) => {
      set((state) => {
        state.pendingAiChatActions = actions;
        state.pendingAiChatMessageId = messageId;
      });
    },

    clearPendingAiChatActions: () => {
      set((state) => {
        state.pendingAiChatActions = null;
        state.pendingAiChatMessageId = null;
      });
    },

    aiActionTransactions: [],

    beginAiActionTransaction: () => {
      const { nodes, edges } = get();
      const txId = `tx-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
      set((state) => {
        state.aiActionTransactions = [
          ...state.aiActionTransactions,
          {
            id: txId,
            timestamp: Date.now(),
            appliedCount: 0,
            beforeNodes: JSON.parse(JSON.stringify(nodes)) as WorkflowNode[],
            beforeEdges: JSON.parse(JSON.stringify(edges)) as WorkflowEdge[],
          },
        ];
      });
      return txId;
    },

    applyAiChatActionInTransaction: (txId: string, action: AiChatAction) => {
      const tx = get().aiActionTransactions.find((t) => t.id === txId);
      if (!tx) {
        get().applyAiChatAction(action);
        return;
      }
      get().applyAiChatAction(action);
      set((state) => {
        state.aiActionTransactions = state.aiActionTransactions.map((t) =>
          t.id === txId ? { ...t, appliedCount: t.appliedCount + 1 } : t
        );
      });
    },

    commitAiActionTransaction: (txId: string) => {
      set((state) => {
        state.aiActionTransactions = state.aiActionTransactions.filter((t) => t.id !== txId);
      });
    },

    rollbackAiActionTransaction: (txId: string) => {
      const tx = get().aiActionTransactions.find((t) => t.id === txId);
      if (!tx) { return; }
      const snapshotNodes = JSON.parse(JSON.stringify(tx.beforeNodes)) as WorkflowNode[];
      const snapshotEdges = JSON.parse(JSON.stringify(tx.beforeEdges)) as WorkflowEdge[];
      set((state) => {
        state.nodes = snapshotNodes;
        state.edges = snapshotEdges;
        state.aiActionTransactions = state.aiActionTransactions.filter((t) => t.id !== txId);
      });
    },

    rollbackLastAiActionTransaction: () => {
      const last = get().aiActionTransactions[get().aiActionTransactions.length - 1];
      if (last) {
        get().rollbackAiActionTransaction(last.id);
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
        const nodes = (response.nodes ?? []) as unknown as WorkflowNode[];
        const validNodes = nodes.filter(
          (n: WorkflowNode) => n?.type && n?.id,
        );
        const edges = (response.edges ?? []) as unknown as WorkflowEdge[];
        const validEdges = edges.filter(
          (e: WorkflowEdge) => e?.source && e?.target,
        );
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

    toggleExpandSubWorkflow: async (nodeId: string, subWorkflowId: string | undefined) => {
      const { expandedSubWorkflows } = get();

      // 已展开 → 折叠
      if (expandedSubWorkflows[nodeId]) {
        set((state) => {
          // 清理 parentRefs 中子工作流内部节点的引用
          const sub = state.expandedSubWorkflows[nodeId];
          if (sub?.nodes) {
            for (const n of sub.nodes) {
              delete state.parentRefs[n.id];
            }
            // 清理子节点与主画布的连接边
            const subNodeIds = new Set(sub.nodes.map((n) => n.id));
            state.edges = state.edges.filter(
              (e) => !subNodeIds.has(e.source) && !subNodeIds.has(e.target),
            );
          }
          delete state.expandedSubWorkflows[nodeId];
        });
        return;
      }

      // 折叠 → 展开
      if (!subWorkflowId) { return; }

      set((state) => {
        state.expandedSubWorkflows[nodeId] = { nodes: [], edges: [], isLoading: true };
      });

      try {
        const template = await invoke<WorkflowTemplateResponse>(
          "get_workflow_template",
          { id: subWorkflowId },
        );
        if (!template) {
          set((state) => {
            delete state.expandedSubWorkflows[nodeId];
          });
          return;
        }

        // 为内部节点 IDs 添加前缀避免与主画布冲突
        const prefix = `sw_${nodeId}_`;
        const idMap = new Map<string, string>();
        const subNodes: WorkflowNode[] = (template.nodes || []).map((n: WorkflowNode) => {
          const oldId = n.id || "";
          const newId = `${prefix}${oldId}`;
          idMap.set(oldId, newId);
          return { ...n, id: newId } as unknown as WorkflowNode;
        });
        const subEdges: WorkflowEdge[] = (template.edges || []).map((e: WorkflowEdge) => ({
          ...e,
          id: `${prefix}${e.id}`,
          source: idMap.get(e.source) || e.source,
          target: idMap.get(e.target) || e.target,
        }));

        set((state) => {
          state.expandedSubWorkflows[nodeId] = { nodes: subNodes, edges: subEdges, isLoading: false };
          // 将子节点注册到 parentRefs
          for (const n of subNodes) {
            state.parentRefs[n.id] = nodeId;
          }
          // 展开的子工作流内部边也加入主边列表（带 sw_ 前缀）
          for (const e of subEdges) {
            state.edges.push(e);
          }
        });
      } catch {
        set((state) => {
          delete state.expandedSubWorkflows[nodeId];
        });
      }
    },

    /**
     * 切换容器的折叠状态。折叠时容器内的子节点会从画布上隐藏（hidden=true），
     * 边会随子节点隐藏。仅会话内 UI 状态，不写入后端模板，不进撤销栈。
     * 重新生成 Set 引用以触发订阅方基于引用的依赖比较。
     */
    toggleContainerCollapse: (containerId: string) => {
      set((state) => {
        const next = new Set(state.collapsedContainers);
        if (next.has(containerId)) {
          next.delete(containerId);
        } else {
          next.add(containerId);
        }
        state.collapsedContainers = next;
        try {
          localStorage.setItem("workflow_collapsed_containers", JSON.stringify([...next]));
        } catch {
          // localStorage may be full or unavailable
        }
      });
    },
  })),
);

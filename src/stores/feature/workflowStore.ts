// SPDX-License-Identifier: AGPL-3.0-only
// Phase 4: NL2Workflow Store — 工作流状态管理（已接入真实后端命令）

import { t } from "@/lib/i18nStoreHelper";
import { invoke, listen, logIpcError } from "@/lib/invoke";
import type {
  ExecutionLogEntry,
  NL2SkillRequest,
  NL2SkillResult,
  NL2UIRequest,
  NL2UIResult,
  NLParseRequest,
  NLParseResult,
  NodeExecutionState,
  WorkflowDefinition,
  WorkflowEdge,
  WorkflowExecution,
  WorkflowFilter,
  WorkflowNode,
  WorkflowTemplate,
  WorkflowVersion,
} from "@/types";
import { create } from "zustand";

// ============================================================
// 后端 DTO 类型（snake_case，与 Rust serde 输出一致）
// ============================================================

/** 后端 WorkflowTemplateResponse（仅声明用到的字段） */
interface BackendTemplateResponse {
  id: string;
  name: string;
  description?: string;
  icon: string;
  tags: string[];
  version: number;
  is_preset: boolean;
  is_editable: boolean;
  is_public: boolean;
  trigger_config?: unknown;
  nodes: BackendWorkflowNode[];
  edges: BackendWorkflowEdge[];
  input_schema?: unknown;
  output_schema?: unknown;
  variables: BackendVariable[];
  error_config?: unknown;
  tool_defs?: unknown[];
  created_at: number;
  updated_at: number;
}

/** 后端 WorkflowNode（serde tag = "type"，base 被 flatten） */
interface BackendWorkflowNode {
  type: string;
  // base 字段被 flatten 进来
  id: string;
  title: string;
  description?: string;
  position: { x: number; y: number };
  enabled: boolean;
  config: Record<string, unknown>;
}

/** 后端 WorkflowEdge */
interface BackendWorkflowEdge {
  id: string;
  source: string;
  target: string;
  edge_type: string;
  label?: string;
}

/** 后端 Variable */
interface BackendVariable {
  name: string;
  var_type: string;
  value: unknown;
  description?: string;
  is_secret: boolean;
}

/** 后端 WorkflowTemplateInput */
interface BackendTemplateInput {
  name: string;
  description?: string;
  icon: string;
  tags: string[];
  trigger_config?: unknown;
  nodes: BackendWorkflowNode[];
  edges: BackendWorkflowEdge[];
  input_schema?: unknown;
  output_schema?: unknown;
  variables: BackendVariable[];
  error_config?: unknown;
  tool_defs?: unknown[];
}

/** 后端 ExecutionStatusResponse */
interface BackendExecutionStatus {
  execution_id: string;
  workflow_id: string;
  status: string;
  current_node_id?: string;
  total_time_ms: number;
  node_count: number;
  node_records: BackendNodeRecord[];
  variables: Record<string, unknown>;
}

interface BackendNodeRecord {
  node_id: string;
  node_type: string;
  node_name?: string;
  status: string;
  input?: unknown;
  output?: unknown;
  execution_time_ms?: number;
  error?: string;
  started_at: number;
  completed_at?: number;
}

// ============================================================
// 类型转换工具
// ============================================================

/** 后端模板响应 → 前端简化 WorkflowDefinition */
function templateResponseToWorkflowDefinition(
  resp: BackendTemplateResponse,
): WorkflowDefinition {
  const nodes: WorkflowNode[] = (resp.nodes || []).map((n) => ({
    id: n.id,
    type: mapBackendNodeTypeToFrontend(n.type),
    label: n.title,
    description: n.description,
    config: n.config ?? {},
    position: n.position ?? { x: 0, y: 0 },
  }));

  const edges: WorkflowEdge[] = (resp.edges || []).map((e) => ({
    id: e.id,
    source: e.source,
    target: e.target,
    label: e.label,
  }));

  const variables: Record<string, unknown> = {};
  for (const v of resp.variables || []) {
    variables[v.name] = v.value;
  }

  return {
    id: resp.id,
    name: resp.name,
    description: resp.description ?? "",
    version: resp.version,
    nodes,
    edges,
    variables,
    createdAt: resp.created_at,
    updatedAt: resp.updated_at,
    status: resp.is_preset ? "active" : "draft",
  };
}

/** 后端节点类型字符串 → 前端简化 NodeType */
function mapBackendNodeTypeToFrontend(backendType: string): WorkflowNode["type"] {
  const mapping: Record<string, WorkflowNode["type"]> = {
    trigger: "trigger",
    end: "output",
    agent: "action",
    llm: "action",
    tool: "action",
    code: "action",
    httpRequest: "action",
    databaseQuery: "action",
    email: "action",
    notification: "action",
    webhookSend: "action",
    fileOperation: "action",
    dataTransformer: "action",
    documentParser: "action",
    vectorRetrieve: "action",
    storage: "action",
    logging: "action",
    aggregator: "action",
    condition: "condition",
    switch: "condition",
    loop: "loop",
    parallel: "parallel",
    merge: "parallel",
    delay: "action",
    validation: "action",
    subWorkflow: "subflow",
    workflowRef: "subflow",
    approval: "action",
    llmClassifier: "action",
    debate: "action",
    swarm: "action",
  };
  return mapping[backendType] ?? "action";
}

/** 前端简化 WorkflowDefinition → 后端 WorkflowTemplateInput */
function workflowDefinitionToTemplateInput(
  wf: Partial<WorkflowDefinition>,
): BackendTemplateInput {
  const nodes: BackendWorkflowNode[] = (wf.nodes ?? []).map((n) => ({
    type: "end", // 简化节点无法精确反推后端联合类型，统一用 end 占位
    id: n.id,
    title: n.label,
    description: n.description,
    position: n.position,
    enabled: true,
    config: n.config ?? {},
  }));

  const edges: BackendWorkflowEdge[] = (wf.edges ?? []).map((e) => ({
    id: e.id,
    source: e.source,
    target: e.target,
    edge_type: "direct",
    label: e.label,
  }));

  const variables: BackendVariable[] = Object.entries(wf.variables ?? {}).map(
    ([name, value]) => ({
      name,
      var_type: "string",
      value,
      is_secret: false,
    }),
  );

  return {
    name: wf.name ?? t("workflow.defaultWorkflowName"),
    description: wf.description,
    icon: "🔧",
    tags: [],
    nodes,
    edges,
    variables,
  };
}

/** 后端执行状态 → 前端 WorkflowExecution */
function executionStatusToWorkflowExecution(
  status: BackendExecutionStatus,
  inputs: Record<string, unknown>,
): WorkflowExecution {
  const now = Date.now();
  const startedAt = now - status.total_time_ms;

  const nodeStates: NodeExecutionState[] = status.node_records.map((r) => ({
    nodeId: r.node_id,
    status: mapBackendNodeStatusToFrontend(r.status),
    startedAt: r.started_at,
    finishedAt: r.completed_at,
    output: r.output,
    error: r.error,
  }));

  const logs: ExecutionLogEntry[] = [];
  for (const r of status.node_records) {
    if (r.error) {
      logs.push({
        timestamp: r.completed_at ?? r.started_at,
        nodeId: r.node_id,
        nodeName: r.node_name ?? r.node_id,
        level: "error",
        message: r.error,
      });
    } else {
      logs.push({
        timestamp: r.completed_at ?? r.started_at,
        nodeId: r.node_id,
        nodeName: r.node_name ?? r.node_id,
        level: "info",
        message: t("workflow.execution.nodeCompleted", { label: r.node_name ?? r.node_id }),
      });
    }
  }

  const isCompleted = status.status === "completed";
  const isFailed = status.status === "failed";

  return {
    id: status.execution_id,
    workflowId: status.workflow_id,
    status: isCompleted ? "completed" : isFailed ? "failed" : "running",
    startedAt,
    finishedAt: isCompleted || isFailed ? now : undefined,
    nodeStates,
    inputs,
    outputs: status.variables,
    logs,
  };
}

/** 后端节点状态字符串 → 前端 NodeExecutionState.status */
function mapBackendNodeStatusToFrontend(
  backendStatus: string,
): NodeExecutionState["status"] {
  const mapping: Record<string, NodeExecutionState["status"]> = {
    waiting: "waiting",
    pending: "waiting",
    ready: "waiting",
    running: "running",
    in_progress: "running",
    success: "success",
    completed: "success",
    failed: "failed",
    error: "failed",
    skipped: "failed",
  };
  return mapping[backendStatus.toLowerCase()] ?? "waiting";
}

/** 后端模板响应 → 前端 WorkflowTemplate（简化） */
function templateResponseToWorkflowTemplate(
  resp: BackendTemplateResponse,
): WorkflowTemplate {
  return {
    id: resp.id,
    name: resp.name,
    description: resp.description ?? "",
    category: "integration", // 后端无 category 字段，使用默认值
    nodeCount: resp.nodes?.length ?? 0,
    tags: resp.tags ?? [],
    workflow: templateResponseToWorkflowDefinition(resp),
    isBuiltIn: resp.is_preset,
    createdAt: resp.created_at,
    updatedAt: resp.updated_at,
  };
}

// ============================================================
// Store Types
// ============================================================

interface WorkflowStoreState {
  // 工作流列表
  workflows: WorkflowDefinition[];
  // 当前编辑的工作流 ID
  currentWorkflowId: string | null;
  // 模板库
  templates: WorkflowTemplate[];
  // NL 解析历史
  parseHistory: NLParseResult[];
  // 执行记录
  executions: WorkflowExecution[];
  // 版本历史 (key: workflowId)
  versionHistories: Record<string, WorkflowVersion[]>;

  // 加载状态
  loading: boolean;
  error: string | null;

  // NL 解析状态
  isParsing: boolean;
  parseProgress: string;

  // 执行状态
  isExecuting: boolean;

  // 筛选
  filter: WorkflowFilter;

  // ========== Actions ==========
  fetchWorkflows: () => Promise<void>;
  getWorkflow: (id: string) => Promise<WorkflowDefinition | null>;
  createWorkflow: (workflow: Partial<WorkflowDefinition>) => Promise<WorkflowDefinition>;
  updateWorkflow: (id: string, updates: Partial<WorkflowDefinition>) => Promise<void>;
  deleteWorkflow: (id: string) => Promise<void>;
  duplicateWorkflow: (id: string) => Promise<WorkflowDefinition>;

  // NL 解析
  parseNaturalLanguage: (request: NLParseRequest) => Promise<NLParseResult>;

  // 模板
  fetchTemplates: () => Promise<void>;
  createFromTemplate: (templateId: string) => Promise<WorkflowDefinition>;

  // 执行
  executeWorkflow: (id: string, inputs: Record<string, unknown>) => Promise<WorkflowExecution>;
  getExecutionStatus: (executionId: string) => Promise<WorkflowExecution | null>;

  // 版本管理
  getVersionHistory: (workflowId: string) => Promise<WorkflowVersion[]>;
  restoreVersion: (workflowId: string, version: number) => Promise<void>;

  // 筛选
  setFilter: (filter: Partial<WorkflowFilter>) => void;
  getFilteredWorkflows: () => WorkflowDefinition[];

  // 编辑器专用
  setCurrentWorkflow: (id: string | null) => void;
  addNode: (node: WorkflowNode) => void;
  updateNode: (nodeId: string, updates: Partial<WorkflowNode>) => void;
  removeNode: (nodeId: string) => void;
  addEdge: (edge: WorkflowEdge) => void;
  removeEdge: (edgeId: string) => void;
  setParsingProgress: (progress: string) => void;

  // NL2Skill
  parseSkillFromNaturalLanguage: (request: NL2SkillRequest) => Promise<NL2SkillResult>;

  // NL2UI
  parseUIFromNaturalLanguage: (request: NL2UIRequest) => Promise<NL2UIResult>;
}

// ============================================================
// Store Implementation
// ============================================================

export const useWorkflowStore = create<WorkflowStoreState>((set, get) => ({
  workflows: [],
  currentWorkflowId: null,
  templates: [],
  parseHistory: [],
  executions: [],
  versionHistories: {},
  loading: false,
  error: null,
  isParsing: false,
  parseProgress: "",
  isExecuting: false,
  filter: { status: "all" },

  // ========== 工作流 CRUD ==========

  fetchWorkflows: async () => {
    set({ loading: true, error: null });
    try {
      const templates = await invoke<BackendTemplateResponse[]>(
        "list_workflow_templates",
        { isPreset: false },
      );
      const workflows = templates.map(templateResponseToWorkflowDefinition);
      set({ workflows });
    } catch (e) {
      logIpcError("fetchWorkflows: list_workflow_templates")(e);
      set({ error: String(e) });
    } finally {
      set({ loading: false });
    }
  },

  getWorkflow: async (id: string) => {
    // 优先从缓存取
    const cached = get().workflows.find((w) => w.id === id);
    if (cached) { return cached; }

    set({ loading: true, error: null });
    try {
      const resp = await invoke<BackendTemplateResponse | null>(
        "get_workflow_template",
        { id },
      );
      if (!resp) { return null; }
      const wf = templateResponseToWorkflowDefinition(resp);
      set((s) => ({
        workflows: s.workflows.some((w) => w.id === wf.id)
          ? s.workflows.map((w) => (w.id === wf.id ? wf : w))
          : [wf, ...s.workflows],
      }));
      return wf;
    } catch (e) {
      logIpcError("getWorkflow: get_workflow_template")(e);
      set({ error: String(e) });
      return null;
    } finally {
      set({ loading: false });
    }
  },

  createWorkflow: async (workflow: Partial<WorkflowDefinition>) => {
    set({ loading: true, error: null });
    try {
      const input = workflowDefinitionToTemplateInput(workflow);
      const newId = await invoke<string>("create_workflow_template", { input });
      // 拉取完整记录（后端会补充 version/created_at 等字段）
      const resp = await invoke<BackendTemplateResponse | null>(
        "get_workflow_template",
        { id: newId },
      );
      const newWf = resp
        ? templateResponseToWorkflowDefinition(resp)
        : {
          id: newId,
          name: workflow.name ?? t("workflow.defaultWorkflowName"),
          description: workflow.description ?? "",
          version: 1,
          nodes: workflow.nodes ?? [],
          edges: workflow.edges ?? [],
          variables: workflow.variables ?? {},
          createdAt: Date.now(),
          updatedAt: Date.now(),
          status: "draft" as const,
        };
      set((s) => ({ workflows: [newWf, ...s.workflows] }));
      return newWf;
    } catch (e) {
      logIpcError("createWorkflow: create_workflow_template")(e);
      set({ error: String(e) });
      throw e;
    } finally {
      set({ loading: false });
    }
  },

  updateWorkflow: async (id: string, updates: Partial<WorkflowDefinition>) => {
    set({ loading: true, error: null });
    try {
      // 先获取当前完整状态，合并 updates 后提交
      const existing = get().workflows.find((w) => w.id === id);
      const merged = { ...existing, ...updates };
      const input = workflowDefinitionToTemplateInput(merged);
      await invoke<boolean>("update_workflow_template", { id, input });
      set((s) => ({
        workflows: s.workflows.map((w) =>
          w.id === id
            ? { ...w, ...updates, updatedAt: Date.now() }
            : w
        ),
      }));
    } catch (e) {
      logIpcError("updateWorkflow: update_workflow_template")(e);
      set({ error: String(e) });
    } finally {
      set({ loading: false });
    }
  },

  deleteWorkflow: async (id: string) => {
    set({ loading: true, error: null });
    try {
      await invoke<boolean>("delete_workflow_template", { id });
      set((s) => ({
        workflows: s.workflows.filter((w) => w.id !== id),
        currentWorkflowId: s.currentWorkflowId === id ? null : s.currentWorkflowId,
      }));
    } catch (e) {
      logIpcError("deleteWorkflow: delete_workflow_template")(e);
      set({ error: String(e) });
    } finally {
      set({ loading: false });
    }
  },

  duplicateWorkflow: async (id: string) => {
    set({ loading: true, error: null });
    try {
      const newId = await invoke<string>("duplicate_workflow_template", { id });
      const resp = await invoke<BackendTemplateResponse | null>(
        "get_workflow_template",
        { id: newId },
      );
      if (!resp) {
        throw new Error(t("workflow.duplicateSuffix", { name: id }));
      }
      const dup = templateResponseToWorkflowDefinition(resp);
      set((s) => ({ workflows: [dup, ...s.workflows] }));
      return dup;
    } catch (e) {
      logIpcError("duplicateWorkflow: duplicate_workflow_template")(e);
      set({ error: String(e) });
      throw e;
    } finally {
      set({ loading: false });
    }
  },

  // ========== NL 解析 ==========

  parseNaturalLanguage: async (request: NLParseRequest) => {
    set({ isParsing: true, parseProgress: t("workflow.parse.analyzingIntent") });
    try {
      set({ parseProgress: t("workflow.parse.matchingNodes") });

      type GenerationResult = {
        nodes: unknown[];
        edges: unknown[];
        explanation: string;
      };

      const result = await invoke<GenerationResult>("generate_workflow_from_prompt", {
        prompt: request.prompt,
        currentNodes: null,
        currentEdges: null,
      });

      const nlResult: NLParseResult = {
        workflow: {
          id: "",
          name: `NL-${Date.now()}`,
          description: result.explanation,
          version: 1,
          nodes: result.nodes as WorkflowNode[],
          edges: result.edges as WorkflowEdge[],
          variables: {},
          createdAt: Date.now(),
          updatedAt: Date.now(),
          status: "draft",
        },
        confidence: 0.85,
        suggestions: [result.explanation],
      };

      set((s) => ({ parseHistory: [nlResult, ...s.parseHistory] }));
      return nlResult;
    } catch (e) {
      logIpcError("parseNaturalLanguage: generate_workflow_from_prompt")(e);
      set({ error: String(e) });
      throw e;
    } finally {
      set({ isParsing: false, parseProgress: "" });
    }
  },

  // ========== 模板 ==========

  fetchTemplates: async () => {
    set({ loading: true, error: null });
    try {
      const templates = await invoke<BackendTemplateResponse[]>(
        "list_workflow_templates",
        { isPreset: true },
      );
      set({ templates: templates.map(templateResponseToWorkflowTemplate) });
    } catch (e) {
      logIpcError("fetchTemplates: list_workflow_templates")(e);
      set({ error: String(e) });
    } finally {
      set({ loading: false });
    }
  },

  createFromTemplate: async (templateId: string) => {
    set({ loading: true, error: null });
    try {
      const newId = await invoke<string>("duplicate_workflow_template", { id: templateId });
      const resp = await invoke<BackendTemplateResponse | null>(
        "get_workflow_template",
        { id: newId },
      );
      if (!resp) {
        throw new Error(t("workflow.fromTemplate", { name: templateId }));
      }
      const newWf = templateResponseToWorkflowDefinition(resp);
      set((s) => ({ workflows: [newWf, ...s.workflows] }));
      return newWf;
    } catch (e) {
      logIpcError("createFromTemplate: duplicate_workflow_template")(e);
      set({ error: String(e) });
      throw e;
    } finally {
      set({ loading: false });
    }
  },

  // ========== 执行 ==========

  executeWorkflow: async (id: string, inputs: Record<string, unknown>) => {
    set({ isExecuting: true, error: null });
    try {
      // 调用后端启动执行
      const executionId = await invoke<string>("start_workflow_execution", {
        workflowId: id,
        input: inputs,
      });

      // 监听完成事件 + 轮询状态
      return await new Promise<WorkflowExecution>((resolve, reject) => {
        let resolved = false;
        const timeoutMs = 5 * 60 * 1000; // 5 分钟超时
        const startTime = Date.now();

        // 超时保护
        const timeoutHandle = setTimeout(() => {
          if (!resolved) {
            resolved = true;
            cleanup();
            reject(new Error(`execution timed out (${timeoutMs / 1000}s)`));
          }
        }, timeoutMs);

        // 监听完成事件
        let unlisten: (() => void) | null = null;
        const setupListener = async () => {
          try {
            unlisten = await listen<{
              workflow_id: string;
              execution_id: string | null;
              status: string;
              total_time_ms: number;
              error?: string;
            }>("workflow:execution-completed", async (event) => {
              const payload = event.payload;
              if (!payload || payload.execution_id !== executionId) { return; }
              if (resolved) { return; }
              resolved = true;
              cleanup();

              try {
                const status = await invoke<BackendExecutionStatus>(
                  "get_workflow_execution_status",
                  { executionId },
                );
                resolve(executionStatusToWorkflowExecution(status, inputs));
              } catch (e) {
                // 状态获取失败时用事件数据兜底
                resolve({
                  id: executionId,
                  workflowId: id,
                  status: payload.status === "completed" ? "completed" : "failed",
                  startedAt: Date.now() - (payload.total_time_ms ?? 0),
                  finishedAt: Date.now(),
                  nodeStates: [],
                  inputs,
                  outputs: {},
                  logs: payload.error
                    ? [{
                      timestamp: Date.now(),
                      nodeId: "",
                      nodeName: "",
                      level: "error",
                      message: payload.error,
                    }]
                    : [],
                });
              }
            });
          } catch (e) {
            logIpcError("executeWorkflow: listen execution-completed")(e);
          }
        };
        setupListener();

        // 轮询兜底（每 1.5s 拉取一次状态，防止事件丢失）
        const pollInterval = setInterval(async () => {
          if (resolved) { return; }
          if (Date.now() - startTime > timeoutMs) { return; }
          try {
            const status = await invoke<BackendExecutionStatus>(
              "get_workflow_execution_status",
              { executionId },
            );
            const isDone = ["completed", "failed", "cancelled"].includes(status.status);
            if (isDone && !resolved) {
              resolved = true;
              cleanup();
              resolve(executionStatusToWorkflowExecution(status, inputs));
            }
          } catch (e) {
            // 轮询错误不中断，继续等待事件
          }
        }, 1500);

        function cleanup() {
          clearTimeout(timeoutHandle);
          clearInterval(pollInterval);
          unlisten?.();
        }
      });
    } catch (e) {
      logIpcError("executeWorkflow: start_workflow_execution")(e);
      const execution: WorkflowExecution = {
        id: `exec_${Date.now()}`,
        workflowId: id,
        status: "failed",
        startedAt: Date.now(),
        finishedAt: Date.now(),
        nodeStates: [],
        inputs,
        logs: [{
          timestamp: Date.now(),
          nodeId: "",
          nodeName: "",
          level: "error",
          message: String(e),
        }],
      };
      set((s) => ({ executions: [execution, ...s.executions], error: String(e) }));
      return execution;
    } finally {
      set({ isExecuting: false });
    }
  },

  getExecutionStatus: async (executionId: string) => {
    try {
      const status = await invoke<BackendExecutionStatus>(
        "get_workflow_execution_status",
        { executionId },
      );
      return executionStatusToWorkflowExecution(status, {});
    } catch (e) {
      logIpcError("getExecutionStatus: get_workflow_execution_status")(e);
      const exec = get().executions.find((e) => e.id === executionId);
      return exec ?? null;
    }
  },

  // ========== 版本管理 ==========

  getVersionHistory: async (workflowId: string) => {
    try {
      const versions = await invoke<number[]>("get_template_versions", { id: workflowId });
      if (versions.length === 0) { return []; }

      // 拉取当前模板作为快照来源
      const currentResp = await invoke<BackendTemplateResponse | null>(
        "get_workflow_template",
        { id: workflowId },
      );

      const result: WorkflowVersion[] = [];
      for (const ver of versions) {
        let snapshot: WorkflowDefinition | null = null;
        // 当前版本直接用 currentResp，历史版本尝试拉取
        if (currentResp && currentResp.version === ver) {
          snapshot = templateResponseToWorkflowDefinition(currentResp);
        } else {
          try {
            const histResp = await invoke<BackendTemplateResponse | null>(
              "get_template_by_version",
              { id: workflowId, version: ver },
            );
            if (histResp) {
              snapshot = templateResponseToWorkflowDefinition(histResp);
            }
          } catch {
            // 历史版本拉取失败，跳过
          }
        }

        if (snapshot) {
          result.push({
            version: ver,
            updatedAt: snapshot.updatedAt,
            summary: ver === (currentResp?.version ?? 0)
              ? t("workflow.version.current")
              : t("workflow.version.optimizedConfig"),
            status: snapshot.status,
            snapshot,
          });
        }
      }
      set((s) => ({ versionHistories: { ...s.versionHistories, [workflowId]: result } }));
      return result;
    } catch (e) {
      logIpcError("getVersionHistory: get_template_versions")(e);
      set({ error: String(e) });
      return [];
    }
  },

  restoreVersion: async (workflowId: string, version: number) => {
    set({ loading: true, error: null });
    try {
      // 拉取历史版本快照
      const histResp = await invoke<BackendTemplateResponse | null>(
        "get_template_by_version",
        { id: workflowId, version },
      );
      if (!histResp) {
        throw new Error(`version ${version} not found`);
      }

      // 用历史版本的 nodes/edges 覆盖当前模板
      const input: BackendTemplateInput = {
        name: histResp.name,
        description: histResp.description,
        icon: histResp.icon,
        tags: histResp.tags,
        trigger_config: histResp.trigger_config,
        nodes: histResp.nodes,
        edges: histResp.edges,
        input_schema: histResp.input_schema,
        output_schema: histResp.output_schema,
        variables: histResp.variables,
        error_config: histResp.error_config,
        tool_defs: histResp.tool_defs,
      };
      await invoke<boolean>("update_workflow_template", { id: workflowId, input });

      // 刷新本地缓存
      const updated = templateResponseToWorkflowDefinition(histResp);
      set((s) => ({
        workflows: s.workflows.map((w) => (w.id === workflowId ? updated : w)),
      }));
    } catch (e) {
      logIpcError("restoreVersion: get_template_by_version + update_workflow_template")(e);
      set({ error: String(e) });
    } finally {
      set({ loading: false });
    }
  },

  // ========== 筛选 ==========

  setFilter: (filter: Partial<WorkflowFilter>) => {
    set((s) => ({ filter: { ...s.filter, ...filter } }));
  },

  getFilteredWorkflows: () => {
    const { workflows, filter } = get();
    return workflows.filter((wf) => {
      if (filter.status && filter.status !== "all" && wf.status !== filter.status) { return false; }
      if (filter.search) {
        const q = filter.search.toLowerCase();
        if (!wf.name.toLowerCase().includes(q) && !wf.description.toLowerCase().includes(q)) { return false; }
      }
      return true;
    });
  },

  // ========== 编辑器 ==========

  setCurrentWorkflow: (id: string | null) => {
    set({ currentWorkflowId: id });
  },

  addNode: (node: WorkflowNode) => {
    set((s) => ({
      workflows: s.workflows.map((w) =>
        w.id === s.currentWorkflowId
          ? { ...w, nodes: [...w.nodes, node], updatedAt: Date.now() }
          : w
      ),
    }));
  },

  updateNode: (nodeId: string, updates: Partial<WorkflowNode>) => {
    set((s) => ({
      workflows: s.workflows.map((w) =>
        w.id === s.currentWorkflowId
          ? {
            ...w,
            nodes: w.nodes.map((n) => (n.id === nodeId ? { ...n, ...updates } : n)),
            updatedAt: Date.now(),
          }
          : w
      ),
    }));
  },

  removeNode: (nodeId: string) => {
    set((s) => ({
      workflows: s.workflows.map((w) =>
        w.id === s.currentWorkflowId
          ? {
            ...w,
            nodes: w.nodes.filter((n) => n.id !== nodeId),
            edges: w.edges.filter((e) => e.source !== nodeId && e.target !== nodeId),
            updatedAt: Date.now(),
          }
          : w
      ),
    }));
  },

  addEdge: (edge: WorkflowEdge) => {
    set((s) => ({
      workflows: s.workflows.map((w) =>
        w.id === s.currentWorkflowId
          ? { ...w, edges: [...w.edges, edge], updatedAt: Date.now() }
          : w
      ),
    }));
  },

  removeEdge: (edgeId: string) => {
    set((s) => ({
      workflows: s.workflows.map((w) =>
        w.id === s.currentWorkflowId
          ? { ...w, edges: w.edges.filter((e) => e.id !== edgeId), updatedAt: Date.now() }
          : w
      ),
    }));
  },

  setParsingProgress: (progress: string) => {
    set({ parseProgress: progress });
  },

  // ========== NL2Skill ==========

  parseSkillFromNaturalLanguage: async (request: NL2SkillRequest) => {
    set({ isParsing: true, parseProgress: t("workflow.parse.intentAnalysis") });
    try {
      set({ parseProgress: t("workflow.parse.generating") });
      const backendResult = await invoke<{
        skill: import("@/types").SkillDefinition;
        confidence: number;
        suggestions: string[];
        alternatives?: import("@/types").SkillDefinition[];
      }>("generate_skill_from_prompt", {
        prompt: request.prompt,
        skillType: request.skillType ?? "chat",
      });
      set({ isParsing: false, parseProgress: t("workflow.parse.complete") });
      return {
        skill: backendResult.skill,
        confidence: backendResult.confidence,
        phases: [
          { phase: "intent", status: "done", detail: t("workflow.parse.intentAnalysis") },
          { phase: "generate", status: "done", detail: t("workflow.parse.generating") },
          { phase: "validate", status: "done", detail: t("workflow.parse.complete") },
        ],
        suggestions: backendResult.suggestions,
        alternatives: backendResult.alternatives,
      } satisfies NL2SkillResult;
    } catch (e) {
      logIpcError("parseSkillFromNaturalLanguage: generate_skill_from_prompt")(e);
      set({ isParsing: false, parseProgress: t("workflow.parse.complete") });
      throw e;
    }
  },

  // ========== NL2UI ==========

  parseUIFromNaturalLanguage: async (request: NL2UIRequest) => {
    set({ isParsing: true, parseProgress: t("workflow.parse.intentAnalysis") });
    try {
      set({ parseProgress: t("workflow.parse.generating") });
      const backendResult = await invoke<{
        schema: import("@/types/dynamicUI").UISchema;
        confidence: number;
        suggestions: string[];
        alternatives?: { schema: import("@/types/dynamicUI").UISchema; description: string }[];
      }>("generate_ui_from_prompt", {
        prompt: request.prompt,
        uiType: request.uiType ?? "form",
      });
      set({ isParsing: false, parseProgress: t("workflow.parse.complete") });
      return {
        schema: backendResult.schema,
        confidence: backendResult.confidence,
        phases: [
          { phase: "intent", status: "done", detail: t("workflow.parse.intentAnalysis") },
          { phase: "generate", status: "done", detail: t("workflow.parse.generating") },
          { phase: "validate", status: "done", detail: t("workflow.parse.complete") },
        ],
        suggestions: backendResult.suggestions,
        alternatives: backendResult.alternatives,
      } satisfies NL2UIResult;
    } catch (e) {
      logIpcError("parseUIFromNaturalLanguage: generate_ui_from_prompt")(e);
      set({ isParsing: false, parseProgress: t("workflow.parse.complete") });
      throw e;
    }
  },
}));

// Selector for working with current workflow
export function useCurrentWorkflow(): WorkflowDefinition | null {
  return useWorkflowStore((s) => {
    if (!s.currentWorkflowId) { return null; }
    return s.workflows.find((w) => w.id === s.currentWorkflowId) ?? null;
  });
}

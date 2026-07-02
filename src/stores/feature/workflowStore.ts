// SPDX-License-Identifier: AGPL-3.0-only
// Phase 4: NL2Workflow Store — 工作流状态管理

import { t } from "@/lib/i18nStoreHelper";
import type {
  ExecutionLogEntry,
  NL2SkillRequest,
  NL2SkillResult,
  NL2UIRequest,
  NL2UIResult,
  NLParseRequest,
  NLParseResult,
  NodeExecutionState,
  VersionDiff,
  WorkflowDefinition,
  WorkflowEdge,
  WorkflowExecution,
  WorkflowFilter,
  WorkflowNode,
  WorkflowTemplate,
  WorkflowVersion,
} from "@/types/workflow";
import { create } from "zustand";

// Dev-only mock imports — Vite tree-shakes these in production build.
import { getMocks, makeId } from "./workflowStoreUtils";

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
  compareVersions: (workflowId: string, v1: number, v2: number) => Promise<VersionDiff>;

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
      // 尝试调用后端，失败时使用 mock
      // await invoke("get_all_workflows");
      await new Promise((r) => setTimeout(r, 300));
      // keep mock data, already loaded
    } catch (e) {
      console.warn("[workflowStore] fetchWorkflows fallback to mock:", e);
    } finally {
      set({ loading: false });
    }
  },

  getWorkflow: async (id: string) => {
    set({ loading: true, error: null });
    try {
      await new Promise((r) => setTimeout(r, 100));
      const wf = get().workflows.find((w) => w.id === id) ?? null;
      return wf;
    } catch (e) {
      console.warn("[workflowStore] getWorkflow fallback:", e);
      set({ error: String(e) });
      return null;
    } finally {
      set({ loading: false });
    }
  },

  createWorkflow: async (workflow: Partial<WorkflowDefinition>) => {
    set({ loading: true, error: null });
    try {
      await new Promise((r) => setTimeout(r, 200));
      const newWf: WorkflowDefinition = {
        id: makeId(),
        name: workflow.name ?? t("workflow.defaultWorkflowName"),
        description: workflow.description ?? "",
        version: 1,
        nodes: workflow.nodes ?? [],
        edges: workflow.edges ?? [],
        variables: workflow.variables ?? {},
        createdAt: Date.now(),
        updatedAt: Date.now(),
        status: "draft",
      };
      set((s) => ({ workflows: [newWf, ...s.workflows] }));
      return newWf;
    } catch (e) {
      console.warn("[workflowStore] createWorkflow fallback:", e);
      set({ error: String(e) });
      throw e;
    } finally {
      set({ loading: false });
    }
  },

  updateWorkflow: async (id: string, updates: Partial<WorkflowDefinition>) => {
    set({ loading: true, error: null });
    try {
      await new Promise((r) => setTimeout(r, 200));
      set((s) => ({
        workflows: s.workflows.map((w) =>
          w.id === id
            ? { ...w, ...updates, updatedAt: Date.now(), version: w.version + 1 }
            : w
        ),
      }));
    } catch (e) {
      console.warn("[workflowStore] updateWorkflow fallback:", e);
      set({ error: String(e) });
    } finally {
      set({ loading: false });
    }
  },

  deleteWorkflow: async (id: string) => {
    set({ loading: true, error: null });
    try {
      await new Promise((r) => setTimeout(r, 200));
      set((s) => ({
        workflows: s.workflows.filter((w) => w.id !== id),
        currentWorkflowId: s.currentWorkflowId === id ? null : s.currentWorkflowId,
      }));
    } catch (e) {
      console.warn("[workflowStore] deleteWorkflow fallback:", e);
      set({ error: String(e) });
    } finally {
      set({ loading: false });
    }
  },

  duplicateWorkflow: async (id: string) => {
    set({ loading: true, error: null });
    try {
      await new Promise((r) => setTimeout(r, 200));
      const original = get().workflows.find((w) => w.id === id);
      if (!original) { throw new Error(`Workflow ${id} not found`); }
      const dup: WorkflowDefinition = {
        ...original,
        id: makeId(),
        name: t("workflow.duplicateSuffix", { name: original.name }),
        version: 1,
        createdAt: Date.now(),
        updatedAt: Date.now(),
        status: "draft",
      };
      set((s) => ({ workflows: [dup, ...s.workflows] }));
      return dup;
    } catch (e) {
      console.warn("[workflowStore] duplicateWorkflow fallback:", e);
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
      // Production: invoke backend generate_workflow_from_prompt
      set({ parseProgress: t("workflow.parse.matchingNodes") });
      const { invoke } = await import("@/lib/invoke");

      type GenerationResult = {
        nodes: unknown[];
        edges: unknown[];
        explanation: string;
      };

      const result = await invoke<GenerationResult>("generate_workflow_from_prompt", {
        prompt: request.prompt,
        current_nodes: null,
        current_edges: null,
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
      console.warn("[workflowStore] parseNaturalLanguage failed:", e);
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
      await new Promise((r) => setTimeout(r, 200));
    } catch (e) {
      console.warn("[workflowStore] fetchTemplates fallback:", e);
    } finally {
      set({ loading: false });
    }
  },

  createFromTemplate: async (templateId: string) => {
    set({ loading: true, error: null });
    try {
      await new Promise((r) => setTimeout(r, 300));
      const template = get().templates.find((t) => t.id === templateId);
      if (!template) { throw new Error(`Template ${templateId} not found`); }
      const newWf: WorkflowDefinition = {
        ...template.workflow,
        id: makeId(),
        name: t("workflow.fromTemplate", { name: template.name }),
        version: 1,
        createdAt: Date.now(),
        updatedAt: Date.now(),
        status: "draft",
      };
      set((s) => ({ workflows: [newWf, ...s.workflows] }));
      return newWf;
    } catch (e) {
      console.warn("[workflowStore] createFromTemplate fallback:", e);
      set({ error: String(e) });
      throw e;
    } finally {
      set({ loading: false });
    }
  },

  // ========== 执行 ==========

  executeWorkflow: async (id: string, inputs: Record<string, unknown>) => {
    set({ isExecuting: true, error: null });
    const executionId = `exec_${Date.now()}`;
    const wf = get().workflows.find((w) => w.id === id);
    const logs: ExecutionLogEntry[] = [];

    const addLog = (nodeId: string, nodeName: string, level: ExecutionLogEntry["level"], message: string) => {
      logs.push({ timestamp: Date.now(), nodeId, nodeName, level, message });
    };

    const nodeStates: NodeExecutionState[] = wf
      ? wf.nodes.map((n) => ({ nodeId: n.id, status: "waiting" as const }))
      : [];

    try {
      if (wf) {
        for (const node of wf.nodes) {
          addLog(node.id, node.label, "info", t("workflow.execution.startingNode", { label: node.label }));
          set({ isExecuting: true });
          await new Promise((r) => setTimeout(r, 300 + Math.random() * 400));
          const idx = nodeStates.findIndex((ns) => ns.nodeId === node.id);
          if (idx >= 0) {
            nodeStates[idx] = { nodeId: node.id, status: "success", startedAt: Date.now(), finishedAt: Date.now() };
          }
          addLog(node.id, node.label, "info", t("workflow.execution.nodeCompleted", { label: node.label }));
        }
      }

      const execution: WorkflowExecution = {
        id: executionId,
        workflowId: id,
        status: "completed",
        startedAt: Date.now(),
        finishedAt: Date.now(),
        nodeStates,
        inputs,
        outputs: { message: t("workflow.execution.success") },
        logs,
      };

      set((s) => ({ executions: [execution, ...s.executions] }));
      return execution;
    } catch (e) {
      const execution: WorkflowExecution = {
        id: executionId,
        workflowId: id,
        status: "failed",
        startedAt: Date.now(),
        finishedAt: Date.now(),
        nodeStates,
        inputs,
        logs,
      };
      set((s) => ({ executions: [execution, ...s.executions], error: String(e) }));
      return execution;
    } finally {
      set({ isExecuting: false });
    }
  },

  getExecutionStatus: async (executionId: string) => {
    const exec = get().executions.find((e) => e.id === executionId) ?? null;
    return exec;
  },

  // ========== 版本管理 ==========

  getVersionHistory: async (workflowId: string) => {
    const wf = get().workflows.find((w) => w.id === workflowId);
    if (!wf) { return []; }
    const versions: WorkflowVersion[] = [
      {
        version: wf.version,
        updatedAt: wf.updatedAt,
        summary: t("workflow.version.current"),
        status: wf.status,
        snapshot: wf,
      },
      {
        version: wf.version - 1,
        updatedAt: wf.updatedAt - 86400000,
        summary: t("workflow.version.optimizedConfig"),
        status: "active",
        snapshot: wf,
      },
      {
        version: wf.version - 2,
        updatedAt: wf.updatedAt - 86400000 * 2,
        summary: t("workflow.version.addedBranch"),
        status: "active",
        snapshot: wf,
      },
      {
        version: 1,
        updatedAt: wf.createdAt,
        summary: t("workflow.version.initialCreation"),
        status: "draft",
        snapshot: wf,
      },
    ];
    return versions;
  },

  restoreVersion: async (workflowId: string, _version: number) => {
    await new Promise((r) => setTimeout(r, 300));
    set((s) => ({
      workflows: s.workflows.map((w) =>
        w.id === workflowId ? { ...w, version: w.version + 1, updatedAt: Date.now() } : w
      ),
    }));
  },

  compareVersions: async (_workflowId: string, _v1: number, _v2: number) => {
    await new Promise((r) => setTimeout(r, 200));
    return {
      addedNodes: [],
      removedNodes: [],
      modifiedNodes: [],
      addedEdges: [],
      removedEdges: [],
      modifiedEdges: [],
    };
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
      const mocks = await getMocks();
      if (mocks) {
        const result = await mocks.mockParseSkill(
          request.prompt,
          request.skillType || "chat",
          (p) => set({ parseProgress: p }),
        );
        set({ isParsing: false, parseProgress: t("workflow.parse.complete") });
        return result;
      }
      // Production: invoke backend
      throw new Error("NL2Skill backend not yet connected");
    } catch (e) {
      set({ isParsing: false, parseProgress: t("workflow.parse.complete") });
      throw e;
    }
  },

  // ========== NL2UI ==========

  parseUIFromNaturalLanguage: async (request: NL2UIRequest) => {
    set({ isParsing: true, parseProgress: t("workflow.parse.intentAnalysis") });
    try {
      const mocks = await getMocks();
      if (mocks) {
        const result = await mocks.mockParseUI(
          request,
          (p) => set({ parseProgress: p }),
        );
        set({ isParsing: false, parseProgress: t("workflow.parse.complete") });
        return result;
      }
      // Production: invoke backend
      throw new Error("NL2UI backend not yet connected");
    } catch (e) {
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

// ============================================================
// All mock data moved to __mocks__/workflowStoreMocks.ts
// Dev-only: imported lazily via getMocks() above
// Production: tree-shaken by Vite
// ============================================================

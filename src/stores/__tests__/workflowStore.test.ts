// SPDX-License-Identifier: AGPL-3.0-only

import { beforeEach, describe, expect, it, vi } from "vitest";

const { invokeMock, listenMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
  listenMock: vi.fn(() => Promise.resolve(() => {})),
}));

vi.mock("@/lib/invoke", () => ({
  invoke: invokeMock,
  listen: listenMock,
  isTauri: () => false,
  logIpcError: vi.fn(() => vi.fn()),
}));

vi.mock("@/lib/i18nStoreHelper", () => ({
  t: (key: string, params?: Record<string, unknown>) => {
    if (!params) { return key; }
    // 简单替换 {name} 占位符
    return key.replace(/\{(\w+)\}/g, (_, k: string) => String(params[k] ?? ""));
  },
}));

import { useWorkflowStore } from "@/stores/feature/workflowStore";
import type { WorkflowDefinition, WorkflowExecution } from "@/types";

const baseWorkflow = (
  overrides?: Partial<WorkflowDefinition>,
): WorkflowDefinition => ({
  id: "wf-1",
  name: "Test Workflow",
  description: "A test workflow",
  nodes: [],
  edges: [],
  variables: {},
  createdAt: 1700000000000,
  updatedAt: 1700000000000,
  version: 1,
  status: "draft" as const,
  ...overrides,
});

const mockTemplate = {
  id: "tpl-1",
  name: "Template 1",
  description: "A template",
  category: "general",
  steps: [{ id: "s-1", goal: "Step", agentRole: "coordinator" }],
  tags: [],
  usageCount: 0,
};

const mockExecution: WorkflowExecution = {
  id: "exec-1",
  workflowId: "wf-1",
  status: "running",
  startedAt: 1700000000000,
  nodeStates: [],
  inputs: {},
  logs: [],
};

// 后端模板响应的默认 mock
const mockBackendTemplate = {
  id: "tpl-new",
  name: "New Workflow",
  description: "",
  icon: "🔧",
  tags: [],
  version: 1,
  is_preset: false,
  is_editable: true,
  is_public: false,
  nodes: [],
  edges: [],
  variables: [],
  created_at: 1700000000000,
  updated_at: 1700000000000,
};

describe("workflowStore", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useWorkflowStore.setState({
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
      filter: {},
    });
  });

  describe("fetchWorkflows", () => {
    it("calls list_workflow_templates with isPreset=false", async () => {
      invokeMock.mockResolvedValueOnce([]);
      await useWorkflowStore.getState().fetchWorkflows();
      expect(invokeMock).toHaveBeenCalledWith("list_workflow_templates", {
        isPreset: false,
      });
      expect(useWorkflowStore.getState().loading).toBe(false);
    });

    it("stores fetched workflows in state", async () => {
      invokeMock.mockResolvedValueOnce([mockBackendTemplate]);
      await useWorkflowStore.getState().fetchWorkflows();
      expect(useWorkflowStore.getState().workflows).toHaveLength(1);
      expect(useWorkflowStore.getState().workflows[0].id).toBe("tpl-new");
    });

    it("sets error on invoke failure", async () => {
      invokeMock.mockRejectedValueOnce(new Error("Network error"));
      await useWorkflowStore.getState().fetchWorkflows();
      expect(useWorkflowStore.getState().error).toBeTruthy();
    });
  });

  describe("getWorkflow", () => {
    it("returns workflow from local cache if available", async () => {
      useWorkflowStore.setState({ workflows: [baseWorkflow()] });
      const result = await useWorkflowStore.getState().getWorkflow("wf-1");
      expect(result?.id).toBe("wf-1");
      expect(invokeMock).not.toHaveBeenCalled();
    });

    it("fetches from backend when not in cache", async () => {
      invokeMock.mockResolvedValueOnce(mockBackendTemplate);
      const result = await useWorkflowStore.getState().getWorkflow("tpl-new");
      expect(invokeMock).toHaveBeenCalledWith("get_workflow_template", {
        id: "tpl-new",
      });
      expect(result?.id).toBe("tpl-new");
    });

    it("returns null when backend returns null", async () => {
      invokeMock.mockResolvedValueOnce(null);
      const result = await useWorkflowStore.getState().getWorkflow("missing");
      expect(result).toBeNull();
    });
  });

  describe("createWorkflow", () => {
    it("creates a workflow via backend and adds to list", async () => {
      invokeMock
        .mockResolvedValueOnce("tpl-new") // create_workflow_template
        .mockResolvedValueOnce(mockBackendTemplate); // get_workflow_template

      const result = await useWorkflowStore
        .getState()
        .createWorkflow({ name: "New Workflow" });

      expect(invokeMock).toHaveBeenCalledWith(
        "create_workflow_template",
        expect.objectContaining({ input: expect.any(Object) }),
      );
      expect(result.id).toBe("tpl-new");
      expect(useWorkflowStore.getState().workflows).toContainEqual(result);
    });

    it("throws on backend failure", async () => {
      invokeMock.mockRejectedValueOnce(new Error("Create failed"));
      await expect(
        useWorkflowStore.getState().createWorkflow({ name: "X" }),
      ).rejects.toThrow("Create failed");
    });
  });

  describe("updateWorkflow", () => {
    it("updates a workflow in the list", async () => {
      useWorkflowStore.setState({ workflows: [baseWorkflow()] });
      invokeMock.mockResolvedValueOnce(true);
      await useWorkflowStore.getState().updateWorkflow("wf-1", { name: "Updated Name" });
      expect(useWorkflowStore.getState().workflows[0].name).toBe("Updated Name");
    });
  });

  describe("deleteWorkflow", () => {
    it("removes workflow from list", async () => {
      useWorkflowStore.setState({
        workflows: [baseWorkflow(), baseWorkflow({ id: "wf-2", name: "Second" })],
      });
      invokeMock.mockResolvedValueOnce(true);
      await useWorkflowStore.getState().deleteWorkflow("wf-1");
      expect(useWorkflowStore.getState().workflows).toHaveLength(1);
      expect(useWorkflowStore.getState().workflows[0].id).toBe("wf-2");
    });

    it("resets currentWorkflowId when deleting the current workflow", async () => {
      useWorkflowStore.setState({
        workflows: [baseWorkflow()],
        currentWorkflowId: "wf-1",
      });
      invokeMock.mockResolvedValueOnce(true);
      await useWorkflowStore.getState().deleteWorkflow("wf-1");
      expect(useWorkflowStore.getState().currentWorkflowId).toBeNull();
    });
  });

  describe("duplicateWorkflow", () => {
    it("duplicates a workflow via backend and adds to list", async () => {
      useWorkflowStore.setState({ workflows: [baseWorkflow()] });
      invokeMock
        .mockResolvedValueOnce("tpl-dup") // duplicate_workflow_template
        .mockResolvedValueOnce({ ...mockBackendTemplate, id: "tpl-dup" }); // get_workflow_template

      const result = await useWorkflowStore.getState().duplicateWorkflow("wf-1");
      expect(result.id).toBe("tpl-dup");
      expect(useWorkflowStore.getState().workflows).toHaveLength(2);
    });

    it("throws when backend returns null", async () => {
      invokeMock
        .mockResolvedValueOnce("missing-dup")
        .mockResolvedValueOnce(null);
      await expect(
        useWorkflowStore.getState().duplicateWorkflow("missing"),
      ).rejects.toThrow();
    });
  });

  describe("setCurrentWorkflow", () => {
    it("sets the current workflow id", () => {
      useWorkflowStore.getState().setCurrentWorkflow("wf-1");
      expect(useWorkflowStore.getState().currentWorkflowId).toBe("wf-1");
    });

    it("clears the current workflow id", () => {
      useWorkflowStore.getState().setCurrentWorkflow("wf-1");
      useWorkflowStore.getState().setCurrentWorkflow(null);
      expect(useWorkflowStore.getState().currentWorkflowId).toBeNull();
    });
  });

  describe("setFilter / getFilteredWorkflows", () => {
    it("sets and merges filter", () => {
      useWorkflowStore.getState().setFilter({ search: "test" });
      useWorkflowStore.getState().setFilter({ status: "active" });
      expect(useWorkflowStore.getState().filter).toEqual({
        search: "test",
        status: "active",
      });
    });

    it("filters workflows by search", () => {
      useWorkflowStore.setState({
        workflows: [
          baseWorkflow({ id: "wf-1", name: "Alpha", status: "draft" }),
          baseWorkflow({ id: "wf-2", name: "Beta", status: "active" }),
        ],
      });
      useWorkflowStore.getState().setFilter({ search: "beta" });
      expect(
        useWorkflowStore.getState().getFilteredWorkflows().map((w) => w.id),
      ).toEqual(["wf-2"]);
    });
  });

  describe("fetchTemplates", () => {
    it("calls list_workflow_templates with isPreset=true", async () => {
      invokeMock.mockResolvedValueOnce([]);
      await useWorkflowStore.getState().fetchTemplates();
      expect(invokeMock).toHaveBeenCalledWith("list_workflow_templates", {
        isPreset: true,
      });
      expect(useWorkflowStore.getState().loading).toBe(false);
    });
  });

  describe("getVersionHistory", () => {
    it("returns version history from backend", async () => {
      invokeMock
        .mockResolvedValueOnce([1, 2, 3]) // get_template_versions
        .mockResolvedValueOnce(mockBackendTemplate); // get_workflow_template (current)

      const versions = await useWorkflowStore.getState().getVersionHistory("tpl-new");
      expect(versions.length).toBeGreaterThan(0);
      expect(versions[0].version).toBe(1);
    });

    it("returns empty array for unknown workflow", async () => {
      invokeMock.mockRejectedValueOnce(new Error("Not found"));
      const versions = await useWorkflowStore.getState().getVersionHistory("missing");
      expect(versions).toEqual([]);
    });
  });

  describe("parseNaturalLanguage", () => {
    it("calls generate_workflow_from_prompt with camelCase params", async () => {
      invokeMock.mockResolvedValueOnce({ nodes: [], edges: [], explanation: "Generated" });

      const result = await useWorkflowStore
        .getState()
        .parseNaturalLanguage({ prompt: "Build a CI/CD pipeline" });

      expect(invokeMock).toHaveBeenCalledWith("generate_workflow_from_prompt", {
        prompt: "Build a CI/CD pipeline",
        currentNodes: null,
        currentEdges: null,
      });
      expect(result.workflow).toBeDefined();
      expect(useWorkflowStore.getState().parseHistory).toHaveLength(1);
    });

    it("sets error and throws on parse failure", async () => {
      invokeMock.mockRejectedValueOnce(new Error("Parse error"));

      await expect(
        useWorkflowStore.getState().parseNaturalLanguage({ prompt: "bad" }),
      ).rejects.toThrow("Parse error");
      expect(useWorkflowStore.getState().error).toBe("Error: Parse error");
    });
  });

  describe("executeWorkflow", () => {
    it("records failed execution when start fails", async () => {
      invokeMock.mockRejectedValueOnce(new Error("Start failed"));
      const exec = await useWorkflowStore.getState().executeWorkflow("wf-1", {});
      expect(exec.workflowId).toBe("wf-1");
      expect(exec.status).toBe("failed");
      expect(useWorkflowStore.getState().executions).toContainEqual(exec);
    });

    it("resolves with completed execution when backend completes", async () => {
      invokeMock
        .mockResolvedValueOnce("exec-1") // workflow_execute（P0-2 单一入口）
        .mockResolvedValueOnce({ // get_workflow_execution_status (poll)
          execution_id: "exec-1",
          workflow_id: "wf-1",
          status: "completed",
          total_time_ms: 1000,
          node_count: 0,
          node_records: [],
          variables: {},
        });

      const exec = await useWorkflowStore.getState().executeWorkflow("wf-1", {});
      expect(exec.workflowId).toBe("wf-1");
      expect(exec.status).toBe("completed");
    });
  });

  describe("getExecutionStatus", () => {
    it("returns a local execution by id when backend fails", async () => {
      useWorkflowStore.setState({ executions: [mockExecution] });
      invokeMock.mockRejectedValueOnce(new Error("Not found"));
      const exec = await useWorkflowStore.getState().getExecutionStatus("exec-1");
      expect(exec?.id).toBe("exec-1");
    });
  });

  describe("templates data", () => {
    it("template shape is usable by createFromTemplate", () => {
      expect(mockTemplate.id).toBe("tpl-1");
      expect(Array.isArray(mockTemplate.steps)).toBe(true);
    });
  });
});

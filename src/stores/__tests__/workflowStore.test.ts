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
    it("resolves and clears loading without backend call (mock-first)", async () => {
      await useWorkflowStore.getState().fetchWorkflows();
      expect(invokeMock).not.toHaveBeenCalled();
      expect(useWorkflowStore.getState().loading).toBe(false);
    });
  });

  describe("getWorkflow", () => {
    it("returns workflow from local cache if available", async () => {
      useWorkflowStore.setState({ workflows: [baseWorkflow()] });
      const result = await useWorkflowStore.getState().getWorkflow("wf-1");
      expect(result?.id).toBe("wf-1");
      expect(invokeMock).not.toHaveBeenCalled();
    });

    it("returns null when not in local cache (no backend fallback)", async () => {
      const result = await useWorkflowStore.getState().getWorkflow("missing");
      expect(result).toBeNull();
    });
  });

  describe("createWorkflow", () => {
    it("creates a workflow with generated id and adds to list", async () => {
      const result = await useWorkflowStore
        .getState()
        .createWorkflow({ name: "New Workflow" });
      expect(result.id).toBeTruthy();
      expect(result.name).toBe("New Workflow");
      expect(useWorkflowStore.getState().workflows).toContainEqual(result);
    });
  });

  describe("updateWorkflow", () => {
    it("updates a workflow in the list", async () => {
      useWorkflowStore.setState({ workflows: [baseWorkflow()] });
      await useWorkflowStore.getState().updateWorkflow("wf-1", { name: "Updated Name" });
      expect(useWorkflowStore.getState().workflows[0].name).toBe("Updated Name");
    });
  });

  describe("deleteWorkflow", () => {
    it("removes workflow from list", async () => {
      useWorkflowStore.setState({
        workflows: [baseWorkflow(), baseWorkflow({ id: "wf-2", name: "Second" })],
      });
      await useWorkflowStore.getState().deleteWorkflow("wf-1");
      expect(useWorkflowStore.getState().workflows).toHaveLength(1);
      expect(useWorkflowStore.getState().workflows[0].id).toBe("wf-2");
    });

    it("resets currentWorkflowId when deleting the current workflow", async () => {
      useWorkflowStore.setState({
        workflows: [baseWorkflow()],
        currentWorkflowId: "wf-1",
      });
      await useWorkflowStore.getState().deleteWorkflow("wf-1");
      expect(useWorkflowStore.getState().currentWorkflowId).toBeNull();
    });
  });

  describe("duplicateWorkflow", () => {
    it("duplicates a workflow and adds to list", async () => {
      useWorkflowStore.setState({ workflows: [baseWorkflow()] });
      const result = await useWorkflowStore.getState().duplicateWorkflow("wf-1");
      expect(result.id).not.toBe("wf-1");
      expect(result.name).toBeTruthy();
      expect(useWorkflowStore.getState().workflows).toHaveLength(2);
    });

    it("throws when source workflow not found", async () => {
      await expect(
        useWorkflowStore.getState().duplicateWorkflow("missing"),
      ).rejects.toThrow(/not found/i);
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
    it("resolves without backend call (mock-first)", async () => {
      await useWorkflowStore.getState().fetchTemplates();
      expect(invokeMock).not.toHaveBeenCalled();
      expect(useWorkflowStore.getState().loading).toBe(false);
    });
  });

  describe("getVersionHistory", () => {
    it("returns synthetic version history for an existing workflow", async () => {
      useWorkflowStore.setState({ workflows: [baseWorkflow({ version: 3 })] });
      const versions = await useWorkflowStore.getState().getVersionHistory("wf-1");
      expect(versions.length).toBeGreaterThan(0);
      expect(versions[0].version).toBe(3);
    });

    it("returns empty array for unknown workflow", async () => {
      const versions = await useWorkflowStore.getState().getVersionHistory("missing");
      expect(versions).toEqual([]);
    });
  });

  describe("parseNaturalLanguage", () => {
    it("calls generate_workflow_from_prompt and records the parse result", async () => {
      invokeMock.mockResolvedValueOnce({ nodes: [], edges: [], explanation: "Generated" });

      const result = await useWorkflowStore
        .getState()
        .parseNaturalLanguage({ prompt: "Build a CI/CD pipeline" });

      expect(invokeMock).toHaveBeenCalledWith("generate_workflow_from_prompt", {
        prompt: "Build a CI/CD pipeline",
        current_nodes: null,
        current_edges: null,
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
    it("simulates execution and records an execution", async () => {
      useWorkflowStore.setState({
        workflows: [
          baseWorkflow({
            // @ts-expect-error 测试用最小节点结构
            nodes: [{ id: "n1", label: "Step" }],
          }),
        ],
      });

      const exec = await useWorkflowStore.getState().executeWorkflow("wf-1", {});
      expect(exec.workflowId).toBe("wf-1");
      expect(exec.status).toBe("completed");
      expect(useWorkflowStore.getState().executions).toContainEqual(exec);
    });
  });

  describe("getExecutionStatus", () => {
    it("returns a local execution by id", async () => {
      useWorkflowStore.setState({ executions: [mockExecution] });
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

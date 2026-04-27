import type { WorkflowEdge, WorkflowNode, WorkflowTemplateResponse } from "@/components/workflow/types";
import { beforeEach, describe, expect, it, vi } from "vitest";

const invokeMock = vi.fn();
const listenMock = vi.fn();

vi.mock("@/lib/invoke", () => ({
  invoke: invokeMock,
  listen: listenMock,
  isTauri: () => false,
}));

vi.mock("zustand/middleware/immer", () => ({
  immer: (config: any) => (set: any, get: any, api: any) =>
    config(
      (partial: any) => {
        if (typeof partial === "function") {
          partial(get());
          set(get());
        } else {
          set(partial);
        }
      },
      get,
      api,
    ),
}));

function makeMockWorkflowNode(id: string, _nodeType: string = "trigger"): WorkflowNode {
  const base = {
    id,
    title: `Node ${id}`,
    description: `Description for ${id}`,
    position: { x: 0, y: 0 },
    retry: { enabled: false, max_retries: 0, backoff_type: "Fixed" as const, base_delay_ms: 0, max_delay_ms: 0 },
    enabled: true,
  };
  return {
    ...base,
    type: "trigger" as const,
    config: { trigger_type: "manual", config: {} },
  } as unknown as WorkflowNode;
}

function makeMockWorkflowEdge(id: string, source: string, target: string): WorkflowEdge {
  return {
    id,
    source,
    target,
    edge_type: "direct",
  };
}

function makeMockTemplate(id: string, overrides: Partial<WorkflowTemplateResponse> = {}): WorkflowTemplateResponse {
  return {
    id,
    name: `Template ${id}`,
    description: `Description for template ${id}`,
    icon: "📋",
    tags: ["test"],
    version: 1,
    is_preset: false,
    is_editable: true,
    is_public: false,
    trigger_config: undefined,
    nodes: [],
    edges: [],
    input_schema: undefined,
    output_schema: undefined,
    variables: [],
    error_config: undefined,
    created_at: Date.now(),
    updated_at: Date.now(),
    ...overrides,
  };
}

/** Reset store to initial state between tests */
async function resetStore() {
  const { useWorkflowEditorStore } = await import("@/stores/feature/workflowEditorStore");
  useWorkflowEditorStore.setState({
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
    nodes: [],
    edges: [],
  });
}

describe("WorkflowEditorStore", () => {
  beforeEach(async () => {
    vi.clearAllMocks();
    invokeMock.mockReset();
    await resetStore();
  });

  describe("Initial State", () => {
    it("should have correct initial state structure", async () => {
      const { useWorkflowEditorStore } = await import("@/stores/feature/workflowEditorStore");
      const state = useWorkflowEditorStore.getState();

      expect(state.currentTemplate).toBeNull();
      expect(state.templates).toEqual([]);
      expect(state.selectedNodeId).toBeNull();
      expect(state.selectedEdgeId).toBeNull();
      expect(state.isLoading).toBe(false);
      expect(state.isSaving).toBe(false);
      expect(state.isDirty).toBe(false);
      expect(state.nodes).toEqual([]);
      expect(state.edges).toEqual([]);
      expect(state.error).toBeNull();
    });
  });

  describe("Node Operations", () => {
    it("should add a node to the canvas", async () => {
      const { useWorkflowEditorStore } = await import("@/stores/feature/workflowEditorStore");
      const store = useWorkflowEditorStore.getState();

      const node = makeMockWorkflowNode("node-1", "trigger");
      store.addNode(node);

      const state = useWorkflowEditorStore.getState();
      expect(state.nodes).toContain(node);
      expect(state.isDirty).toBe(true);
    });

    it("should update an existing node", async () => {
      const { useWorkflowEditorStore } = await import("@/stores/feature/workflowEditorStore");
      const store = useWorkflowEditorStore.getState();

      const node = makeMockWorkflowNode("node-1", "trigger");
      store.addNode(node);

      store.updateNode("node-1", {
        title: "Updated Node",
        description: "Updated description",
      });

      const state = useWorkflowEditorStore.getState();
      const updatedNode = state.nodes.find((n: WorkflowNode) => n.id === "node-1");
      expect(updatedNode?.title).toBe("Updated Node");
    });

    it("should delete a node from the canvas", async () => {
      const { useWorkflowEditorStore } = await import("@/stores/feature/workflowEditorStore");
      const store = useWorkflowEditorStore.getState();

      const node = makeMockWorkflowNode("node-1", "trigger");
      store.addNode(node);
      expect(store.nodes.length).toBe(1);

      store.deleteNode("node-1");

      const state = useWorkflowEditorStore.getState();
      expect(state.nodes.find((n: WorkflowNode) => n.id === "node-1")).toBeUndefined();
    });

    it("should select a node", async () => {
      const { useWorkflowEditorStore } = await import("@/stores/feature/workflowEditorStore");
      const store = useWorkflowEditorStore.getState();

      store.setSelectedNode("node-1");

      const state = useWorkflowEditorStore.getState();
      expect(state.selectedNodeId).toBe("node-1");
    });

    it("should clear node selection when setting null", async () => {
      const { useWorkflowEditorStore } = await import("@/stores/feature/workflowEditorStore");
      const store = useWorkflowEditorStore.getState();

      store.setSelectedNode("node-1");
      store.setSelectedNode(null);

      const state = useWorkflowEditorStore.getState();
      expect(state.selectedNodeId).toBeNull();
    });
  });

  describe("Edge Operations", () => {
    it("should add an edge to the canvas", async () => {
      const { useWorkflowEditorStore } = await import("@/stores/feature/workflowEditorStore");
      const store = useWorkflowEditorStore.getState();

      const edge = makeMockWorkflowEdge("edge-1", "node-1", "node-2");
      store.addEdge(edge);

      const state = useWorkflowEditorStore.getState();
      expect(state.edges).toContain(edge);
      expect(state.isDirty).toBe(true);
    });

    it("should update an existing edge", async () => {
      const { useWorkflowEditorStore } = await import("@/stores/feature/workflowEditorStore");
      const store = useWorkflowEditorStore.getState();

      const edge = makeMockWorkflowEdge("edge-1", "node-1", "node-2");
      store.addEdge(edge);

      store.updateEdge("edge-1", { label: "Updated Edge" });

      const state = useWorkflowEditorStore.getState();
      const updatedEdge = state.edges.find((e: WorkflowEdge) => e.id === "edge-1");
      expect(updatedEdge?.label).toBe("Updated Edge");
    });

    it("should delete an edge from the canvas", async () => {
      const { useWorkflowEditorStore } = await import("@/stores/feature/workflowEditorStore");
      const store = useWorkflowEditorStore.getState();

      const edge = makeMockWorkflowEdge("edge-1", "node-1", "node-2");
      store.addEdge(edge);
      expect(store.edges.length).toBe(1);

      store.deleteEdge("edge-1");

      const state = useWorkflowEditorStore.getState();
      expect(state.edges.find((e: WorkflowEdge) => e.id === "edge-1")).toBeUndefined();
    });

    it("should select an edge", async () => {
      const { useWorkflowEditorStore } = await import("@/stores/feature/workflowEditorStore");
      const store = useWorkflowEditorStore.getState();

      store.setSelectedEdge("edge-1");

      const state = useWorkflowEditorStore.getState();
      expect(state.selectedEdgeId).toBe("edge-1");
    });
  });

  describe("Template Operations", () => {
    it("should load templates from backend", async () => {
      const mockTemplates = [
        makeMockTemplate("template-1"),
        makeMockTemplate("template-2"),
      ];

      invokeMock.mockResolvedValueOnce(mockTemplates);

      const { useWorkflowEditorStore } = await import("@/stores/feature/workflowEditorStore");
      const store = useWorkflowEditorStore.getState();

      await store.loadTemplates();

      expect(invokeMock).toHaveBeenCalledWith("list_workflow_templates", { is_preset: undefined });
      const state = useWorkflowEditorStore.getState();
      expect(state.templates).toEqual(mockTemplates);
      expect(state.isLoading).toBe(false);
    });

    it("should load a specific template by id", async () => {
      const mockTemplate = makeMockTemplate("template-1");
      invokeMock.mockResolvedValueOnce(mockTemplate);

      const { useWorkflowEditorStore } = await import("@/stores/feature/workflowEditorStore");
      const store = useWorkflowEditorStore.getState();

      await store.loadTemplate("template-1");

      expect(invokeMock).toHaveBeenCalledWith("get_workflow_template", { id: "template-1" });
      const state = useWorkflowEditorStore.getState();
      expect(state.currentTemplate).toEqual(mockTemplate);
      expect(state.nodes).toEqual(mockTemplate.nodes);
      expect(state.edges).toEqual(mockTemplate.edges);
    });

    it("should create a new template", async () => {
      invokeMock
        .mockResolvedValueOnce("new-template-id") // create_workflow_template
        .mockResolvedValueOnce([]); // loadTemplates → list_workflow_templates

      const { useWorkflowEditorStore } = await import("@/stores/feature/workflowEditorStore");
      const store = useWorkflowEditorStore.getState();

      const input = {
        name: "New Template",
        description: "A new template",
        icon: "📋",
        tags: ["new"],
        trigger_config: undefined,
        nodes: [],
        edges: [],
        input_schema: undefined,
        output_schema: undefined,
        variables: [],
        error_config: undefined,
      };

      const result = await store.createTemplate(input);

      expect(invokeMock).toHaveBeenCalledWith("create_workflow_template", { input });
      expect(result).toBe("new-template-id");
    });

    it("should update an existing template", async () => {
      const updatedTemplate = makeMockTemplate("template-1", { name: "Updated Template" });
      invokeMock
        .mockResolvedValueOnce(true) // update_workflow_template
        .mockResolvedValueOnce([]) // loadTemplates → list_workflow_templates
        .mockResolvedValueOnce(updatedTemplate); // loadTemplate → get_workflow_template

      const { useWorkflowEditorStore } = await import("@/stores/feature/workflowEditorStore");
      const store = useWorkflowEditorStore.getState();

      const input = {
        name: "Updated Template",
        description: "Updated description",
        icon: "📝",
        tags: ["updated"],
        trigger_config: undefined,
        nodes: [],
        edges: [],
        input_schema: undefined,
        output_schema: undefined,
        variables: [],
        error_config: undefined,
      };

      const result = await store.updateTemplate("template-1", input);

      expect(invokeMock).toHaveBeenCalledWith("update_workflow_template", { id: "template-1", input });
      expect(result).toBe(true);
    });

    it("should delete a template", async () => {
      invokeMock.mockResolvedValueOnce(true);

      const { useWorkflowEditorStore } = await import("@/stores/feature/workflowEditorStore");
      const store = useWorkflowEditorStore.getState();

      const result = await store.deleteTemplate("template-1");

      expect(invokeMock).toHaveBeenCalledWith("delete_workflow_template", { id: "template-1" });
      expect(result).toBe(true);
    });

    it("should duplicate a template", async () => {
      invokeMock
        .mockResolvedValueOnce("duplicated-template-id") // duplicate_workflow_template
        .mockResolvedValueOnce([]); // loadTemplates → list_workflow_templates

      const { useWorkflowEditorStore } = await import("@/stores/feature/workflowEditorStore");
      const store = useWorkflowEditorStore.getState();

      const result = await store.duplicateTemplate("template-1");

      expect(invokeMock).toHaveBeenCalledWith("duplicate_workflow_template", { id: "template-1" });
      expect(result).toBe("duplicated-template-id");
    });

    it("should export a template to JSON", async () => {
      const mockJson = JSON.stringify(makeMockTemplate("template-1"));
      invokeMock.mockResolvedValueOnce(mockJson);

      const { useWorkflowEditorStore } = await import("@/stores/feature/workflowEditorStore");
      const store = useWorkflowEditorStore.getState();

      const result = await store.exportTemplate("template-1");

      expect(invokeMock).toHaveBeenCalledWith("export_workflow_template", { id: "template-1" });
      expect(result).toBe(mockJson);
    });

    it("should import a template from JSON", async () => {
      invokeMock
        .mockResolvedValueOnce("imported-template-id") // import_workflow_template
        .mockResolvedValueOnce([]); // loadTemplates → list_workflow_templates
      const jsonData = JSON.stringify(makeMockTemplate("imported"));

      const { useWorkflowEditorStore } = await import("@/stores/feature/workflowEditorStore");
      const store = useWorkflowEditorStore.getState();

      const result = await store.importTemplate(jsonData);

      expect(invokeMock).toHaveBeenCalledWith("import_workflow_template", { jsonData });
      expect(result).toBe("imported-template-id");
    });
  });

  describe("Validation", () => {
    it("should validate a template", async () => {
      const mockValidation = {
        isValid: true,
        errors: [],
        warnings: [{ nodeId: "node-1", message: "Warning" }],
      };
      const mockTemplate = makeMockTemplate("template-1");
      invokeMock
        .mockResolvedValueOnce(mockTemplate) // get_workflow_template (for loadTemplate)
        .mockResolvedValueOnce(mockValidation); // validate_workflow_template

      const { useWorkflowEditorStore } = await import("@/stores/feature/workflowEditorStore");
      // First load a template so currentTemplate is set
      await useWorkflowEditorStore.getState().loadTemplate("template-1");

      const result = await useWorkflowEditorStore.getState().validateTemplate();

      expect(result).toEqual(mockValidation);
    });
  });

  describe("Dirty State", () => {
    it("should mark state as dirty after adding node", async () => {
      const { useWorkflowEditorStore } = await import("@/stores/feature/workflowEditorStore");
      const store = useWorkflowEditorStore.getState();

      expect(store.isDirty).toBe(false);

      store.addNode(makeMockWorkflowNode("node-1"));

      const state = useWorkflowEditorStore.getState();
      expect(state.isDirty).toBe(true);
    });

    it("should mark state as clean after saving", async () => {
      const { useWorkflowEditorStore } = await import("@/stores/feature/workflowEditorStore");
      const store = useWorkflowEditorStore.getState();

      store.addNode(makeMockWorkflowNode("node-1"));
      expect(store.isDirty).toBe(true);

      store.markClean();

      const state = useWorkflowEditorStore.getState();
      expect(state.isDirty).toBe(false);
    });
  });

  describe("AI Features", () => {
    it("should generate workflow from prompt", async () => {
      const mockResult = {
        nodes: [makeMockWorkflowNode("node-1")],
        edges: [makeMockWorkflowEdge("edge-1", "node-1", "node-2")],
        explanation: "Generated workflow",
      };
      invokeMock.mockResolvedValueOnce(mockResult);

      const { useWorkflowEditorStore } = await import("@/stores/feature/workflowEditorStore");
      const store = useWorkflowEditorStore.getState();

      const result = await store.generateWorkflowFromPrompt("Create a workflow");

      expect(invokeMock).toHaveBeenCalledWith("generate_workflow_from_prompt", { prompt: "Create a workflow" });
      expect(result).toEqual({
        nodes: mockResult.nodes,
        edges: mockResult.edges,
      });
    });

    it("should optimize agent prompt", async () => {
      const mockOptimized = "Optimized prompt text";
      invokeMock.mockResolvedValueOnce(mockOptimized);

      const { useWorkflowEditorStore } = await import("@/stores/feature/workflowEditorStore");
      const store = useWorkflowEditorStore.getState();

      const result = await store.optimizeAgentPrompt("Original prompt");

      expect(invokeMock).toHaveBeenCalledWith("optimize_agent_prompt", { prompt: "Original prompt" });
      expect(result).toBe(mockOptimized);
    });

    it("should recommend nodes based on context", async () => {
      const mockRecommendations = [
        { node_type: "agent", label: "Agent 节点", description: "AI Agent", confidence: 0.9 },
        { node_type: "llm", label: "LLM 节点", description: "LLM", confidence: 0.85 },
      ];
      invokeMock.mockResolvedValueOnce(mockRecommendations);

      const { useWorkflowEditorStore } = await import("@/stores/feature/workflowEditorStore");
      const store = useWorkflowEditorStore.getState();

      const result = await store.recommendNodes("I need an AI workflow");

      expect(invokeMock).toHaveBeenCalledWith("recommend_nodes", { context: "I need an AI workflow" });
      expect(result).toEqual(["Agent 节点", "LLM 节点"]);
    });
  });

  describe("Error Handling", () => {
    it("should handle API errors gracefully", async () => {
      invokeMock.mockRejectedValueOnce(new Error("API Error"));

      const { useWorkflowEditorStore } = await import("@/stores/feature/workflowEditorStore");
      const store = useWorkflowEditorStore.getState();

      await store.loadTemplates();

      const state = useWorkflowEditorStore.getState();
      expect(state.error).toBe("Error: API Error");
      expect(state.isLoading).toBe(false);
    });

    it("should set error manually", async () => {
      const { useWorkflowEditorStore } = await import("@/stores/feature/workflowEditorStore");
      const store = useWorkflowEditorStore.getState();

      store.setError("Test error");

      const state = useWorkflowEditorStore.getState();
      expect(state.error).toBe("Test error");
    });

    it("should clear error when setting null", async () => {
      const { useWorkflowEditorStore } = await import("@/stores/feature/workflowEditorStore");
      const store = useWorkflowEditorStore.getState();

      store.setError("Test error");
      store.setError(null);

      const state = useWorkflowEditorStore.getState();
      expect(state.error).toBeNull();
    });
  });

  describe("Batch Operations", () => {
    it("should set multiple nodes at once", async () => {
      const { useWorkflowEditorStore } = await import("@/stores/feature/workflowEditorStore");
      const store = useWorkflowEditorStore.getState();

      const nodes = [
        makeMockWorkflowNode("node-1"),
        makeMockWorkflowNode("node-2"),
      ];
      store.setNodes(nodes);

      const state = useWorkflowEditorStore.getState();
      expect(state.nodes).toEqual(nodes);
    });

    it("should set multiple edges at once", async () => {
      const { useWorkflowEditorStore } = await import("@/stores/feature/workflowEditorStore");
      const store = useWorkflowEditorStore.getState();

      const edges = [
        makeMockWorkflowEdge("edge-1", "node-1", "node-2"),
        makeMockWorkflowEdge("edge-2", "node-2", "node-3"),
      ];
      store.setEdges(edges);

      const state = useWorkflowEditorStore.getState();
      expect(state.edges).toEqual(edges);
    });
  });
});

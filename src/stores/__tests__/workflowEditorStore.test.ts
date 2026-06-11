// SPDX-License-Identifier: AGPL-3.0-only

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

function makeMockWorkflowNode(
  id: string,
  _nodeType: string = "trigger",
): WorkflowNode {
  const base = {
    id,
    title: `Node ${id}`,
    description: `Description for ${id}`,
    position: { x: 0, y: 0 },
    retry: {
      enabled: false,
      max_retries: 0,
      backoff_type: "Fixed" as const,
      base_delay_ms: 0,
      max_delay_ms: 0,
    },
    enabled: true,
  };
  return {
    ...base,
    type: "trigger" as const,
    config: { trigger_type: "manual", config: {} },
  } as unknown as WorkflowNode;
}

function makeMockWorkflowEdge(
  id: string,
  source: string,
  target: string,
): WorkflowEdge {
  return {
    id,
    source,
    target,
    edge_type: "direct",
  };
}

function makeMockTemplate(
  id: string,
  overrides: Partial<WorkflowTemplateResponse> = {},
): WorkflowTemplateResponse {
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
  (useWorkflowEditorStore as any).setState({
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
    parentRefs: {},
    past: [],
    future: [],
    _lastUndoRecordTime: 0,
    aiChatMessages: [],
    aiChatStreaming: false,
    aiChatStreamingMessageId: null,
    collapsedContainers: new Set<string>(),
    diagnoseApplying: false,
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
      const state = useWorkflowEditorStore.getState() as any;

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
      const store = useWorkflowEditorStore.getState() as any;

      const node = makeMockWorkflowNode("node-1", "trigger");
      store.addNode(node);

      const state = useWorkflowEditorStore.getState() as any;
      expect(state.nodes).toContain(node);
      expect(state.isDirty).toBe(true);
    });

    it("should update an existing node", async () => {
      const { useWorkflowEditorStore } = await import("@/stores/feature/workflowEditorStore");
      const store = useWorkflowEditorStore.getState() as any;

      const node = makeMockWorkflowNode("node-1", "trigger");
      store.addNode(node);

      store.updateNode("node-1", {
        title: "Updated Node",
        description: "Updated description",
      });

      const state = useWorkflowEditorStore.getState() as any;
      const updatedNode = state.nodes.find(
        (n: WorkflowNode) => n.id === "node-1",
      );
      expect(updatedNode?.title).toBe("Updated Node");
    });

    it("should delete a node from the canvas", async () => {
      const { useWorkflowEditorStore } = await import("@/stores/feature/workflowEditorStore");
      const store = useWorkflowEditorStore.getState() as any;

      const node = makeMockWorkflowNode("node-1", "trigger");
      store.addNode(node);
      expect(store.nodes.length).toBe(1);

      store.deleteNode("node-1");

      const state = useWorkflowEditorStore.getState() as any;
      expect(
        state.nodes.find((n: WorkflowNode) => n.id === "node-1"),
      ).toBeUndefined();
    });

    it("should select a node", async () => {
      const { useWorkflowEditorStore } = await import("@/stores/feature/workflowEditorStore");
      const store = useWorkflowEditorStore.getState() as any;

      store.setSelectedNode("node-1");

      const state = useWorkflowEditorStore.getState() as any;
      expect(state.selectedNodeId).toBe("node-1");
    });

    it("should clear node selection when setting null", async () => {
      const { useWorkflowEditorStore } = await import("@/stores/feature/workflowEditorStore");
      const store = useWorkflowEditorStore.getState() as any;

      store.setSelectedNode("node-1");
      store.setSelectedNode(null);

      const state = useWorkflowEditorStore.getState() as any;
      expect(state.selectedNodeId).toBeNull();
    });
  });

  describe("Edge Operations", () => {
    it("should add an edge to the canvas", async () => {
      const { useWorkflowEditorStore } = await import("@/stores/feature/workflowEditorStore");
      const store = useWorkflowEditorStore.getState() as any;

      const edge = makeMockWorkflowEdge("edge-1", "node-1", "node-2");
      store.addEdge(edge);

      const state = useWorkflowEditorStore.getState() as any;
      expect(state.edges).toContain(edge);
      expect(state.isDirty).toBe(true);
    });

    it("should update an existing edge", async () => {
      const { useWorkflowEditorStore } = await import("@/stores/feature/workflowEditorStore");
      const store = useWorkflowEditorStore.getState() as any;

      const edge = makeMockWorkflowEdge("edge-1", "node-1", "node-2");
      store.addEdge(edge);

      store.updateEdge("edge-1", { label: "Updated Edge" });

      const state = useWorkflowEditorStore.getState() as any;
      const updatedEdge = state.edges.find(
        (e: WorkflowEdge) => e.id === "edge-1",
      );
      expect(updatedEdge?.label).toBe("Updated Edge");
    });

    it("should delete an edge from the canvas", async () => {
      const { useWorkflowEditorStore } = await import("@/stores/feature/workflowEditorStore");
      const store = useWorkflowEditorStore.getState() as any;

      const edge = makeMockWorkflowEdge("edge-1", "node-1", "node-2");
      store.addEdge(edge);
      expect(store.edges.length).toBe(1);

      store.deleteEdge("edge-1");

      const state = useWorkflowEditorStore.getState() as any;
      expect(
        state.edges.find((e: WorkflowEdge) => e.id === "edge-1"),
      ).toBeUndefined();
    });

    it("should select an edge", async () => {
      const { useWorkflowEditorStore } = await import("@/stores/feature/workflowEditorStore");
      const store = useWorkflowEditorStore.getState() as any;

      store.setSelectedEdge("edge-1");

      const state = useWorkflowEditorStore.getState() as any;
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
      const store = useWorkflowEditorStore.getState() as any;

      await store.loadTemplates();

      expect(invokeMock).toHaveBeenCalledWith("list_workflow_templates", {
        is_preset: undefined,
      });
      const state = useWorkflowEditorStore.getState() as any;
      expect(state.templates).toEqual(mockTemplates);
      expect(state.isLoading).toBe(false);
    });

    it("should load a specific template by id", async () => {
      const mockTemplate = makeMockTemplate("template-1");
      invokeMock.mockResolvedValueOnce(mockTemplate);

      const { useWorkflowEditorStore } = await import("@/stores/feature/workflowEditorStore");
      const store = useWorkflowEditorStore.getState() as any;

      await store.loadTemplate("template-1");

      expect(invokeMock).toHaveBeenCalledWith("get_workflow_template", {
        id: "template-1",
      });
      const state = useWorkflowEditorStore.getState() as any;
      expect(state.currentTemplate).toEqual(mockTemplate);
      expect(state.nodes).toEqual(mockTemplate.nodes);
      expect(state.edges).toEqual(mockTemplate.edges);
    });

    it("should create a new template", async () => {
      invokeMock
        .mockResolvedValueOnce("new-template-id") // create_workflow_template
        .mockResolvedValueOnce([]); // loadTemplates → list_workflow_templates

      const { useWorkflowEditorStore } = await import("@/stores/feature/workflowEditorStore");
      const store = useWorkflowEditorStore.getState() as any;

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

      expect(invokeMock).toHaveBeenCalledWith("create_workflow_template", {
        input,
      });
      expect(result).toBe("new-template-id");
    });

    it("should update an existing template", async () => {
      const updatedTemplate = makeMockTemplate("template-1", {
        name: "Updated Template",
      });
      invokeMock
        .mockResolvedValueOnce(true) // update_workflow_template
        .mockResolvedValueOnce([]) // loadTemplates → list_workflow_templates
        .mockResolvedValueOnce(updatedTemplate); // loadTemplate → get_workflow_template

      const { useWorkflowEditorStore } = await import("@/stores/feature/workflowEditorStore");
      const store = useWorkflowEditorStore.getState() as any;

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

      expect(invokeMock).toHaveBeenCalledWith("update_workflow_template", {
        id: "template-1",
        input,
      });
      expect(result).toBe(true);
    });

    it("should delete a template", async () => {
      invokeMock.mockResolvedValueOnce(true);

      const { useWorkflowEditorStore } = await import("@/stores/feature/workflowEditorStore");
      const store = useWorkflowEditorStore.getState() as any;

      const result = await store.deleteTemplate("template-1");

      expect(invokeMock).toHaveBeenCalledWith("delete_workflow_template", {
        id: "template-1",
      });
      expect(result).toBe(true);
    });

    it("should duplicate a template", async () => {
      invokeMock
        .mockResolvedValueOnce("duplicated-template-id") // duplicate_workflow_template
        .mockResolvedValueOnce([]); // loadTemplates → list_workflow_templates

      const { useWorkflowEditorStore } = await import("@/stores/feature/workflowEditorStore");
      const store = useWorkflowEditorStore.getState() as any;

      const result = await store.duplicateTemplate("template-1");

      expect(invokeMock).toHaveBeenCalledWith("duplicate_workflow_template", {
        id: "template-1",
      });
      expect(result).toBe("duplicated-template-id");
    });

    it("should export a template to JSON", async () => {
      const mockJson = JSON.stringify(makeMockTemplate("template-1"));
      invokeMock.mockResolvedValueOnce(mockJson);

      const { useWorkflowEditorStore } = await import("@/stores/feature/workflowEditorStore");
      const store = useWorkflowEditorStore.getState() as any;

      const result = await store.exportTemplate("template-1");

      expect(invokeMock).toHaveBeenCalledWith("export_workflow_template", {
        id: "template-1",
      });
      expect(result).toBe(mockJson);
    });

    it("should import a template from JSON", async () => {
      invokeMock
        .mockResolvedValueOnce({
          id: "imported-template-id",
          warnings: [],
          errors: [],
        }) // import_workflow_template
        .mockResolvedValueOnce([]); // loadTemplates → list_workflow_templates
      const jsonData = JSON.stringify(makeMockTemplate("imported"));

      const { useWorkflowEditorStore } = await import("@/stores/feature/workflowEditorStore");
      const store = useWorkflowEditorStore.getState() as any;

      const result = await store.importTemplate(jsonData);

      expect(invokeMock).toHaveBeenCalledWith("import_workflow_template", {
        json_data: jsonData,
      });
      expect(result).toEqual({
        id: "imported-template-id",
        warnings: [],
        errors: [],
      });
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
      const store = useWorkflowEditorStore.getState() as any;

      expect(store.isDirty).toBe(false);

      store.addNode(makeMockWorkflowNode("node-1"));

      const state = useWorkflowEditorStore.getState() as any;
      expect(state.isDirty).toBe(true);
    });

    it("should mark state as clean after saving", async () => {
      const { useWorkflowEditorStore } = await import("@/stores/feature/workflowEditorStore");
      const store = useWorkflowEditorStore.getState() as any;

      store.addNode(makeMockWorkflowNode("node-1"));
      expect(store.isDirty).toBe(true);

      store.markClean();

      const state = useWorkflowEditorStore.getState() as any;
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
      const store = useWorkflowEditorStore.getState() as any;

      const result = await store.generateWorkflowFromPrompt("Create a workflow");

      expect(invokeMock).toHaveBeenCalledWith("generate_workflow_from_prompt", {
        prompt: "Create a workflow",
      });
      expect(result).toEqual({
        nodes: mockResult.nodes,
        edges: mockResult.edges,
        explanation: "Generated workflow",
      });
    });

    it("should optimize agent prompt", async () => {
      const mockOptimized = "Optimized prompt text";
      invokeMock.mockResolvedValueOnce(mockOptimized);

      const { useWorkflowEditorStore } = await import("@/stores/feature/workflowEditorStore");
      const store = useWorkflowEditorStore.getState() as any;

      const result = await store.optimizeAgentPrompt("Original prompt");

      expect(invokeMock).toHaveBeenCalledWith("optimize_agent_prompt", {
        prompt: "Original prompt",
      });
      expect(result).toBe(mockOptimized);
    });

    it("should recommend nodes based on context", async () => {
      const mockRecommendations = [
        {
          node_type: "agent",
          label: "Agent 节点",
          description: "AI Agent",
          confidence: 0.9,
        },
        {
          node_type: "llm",
          label: "LLM 节点",
          description: "LLM",
          confidence: 0.85,
        },
      ];
      invokeMock.mockResolvedValueOnce(mockRecommendations);

      const { useWorkflowEditorStore } = await import("@/stores/feature/workflowEditorStore");
      const store = useWorkflowEditorStore.getState() as any;

      const result = await store.recommendNodes("I need an AI workflow");

      expect(invokeMock).toHaveBeenCalledWith("recommend_nodes", {
        context: "I need an AI workflow",
      });
      expect(result).toEqual(mockRecommendations);
    });
  });

  describe("Error Handling", () => {
    it("should handle API errors gracefully", async () => {
      invokeMock.mockRejectedValueOnce(new Error("API Error"));

      const { useWorkflowEditorStore } = await import("@/stores/feature/workflowEditorStore");
      const store = useWorkflowEditorStore.getState() as any;

      await store.loadTemplates();

      const state = useWorkflowEditorStore.getState() as any;
      expect(state.error).toBe("Error: API Error");
      expect(state.isLoading).toBe(false);
    });

    it("should set error manually", async () => {
      const { useWorkflowEditorStore } = await import("@/stores/feature/workflowEditorStore");
      const store = useWorkflowEditorStore.getState() as any;

      store.setError("Test error");

      const state = useWorkflowEditorStore.getState() as any;
      expect(state.error).toBe("Test error");
    });

    it("should clear error when setting null", async () => {
      const { useWorkflowEditorStore } = await import("@/stores/feature/workflowEditorStore");
      const store = useWorkflowEditorStore.getState() as any;

      store.setError("Test error");
      store.setError(null);

      const state = useWorkflowEditorStore.getState() as any;
      expect(state.error).toBeNull();
    });
  });

  describe("Container Parent Refs", () => {
    it("setParentRef registers child→parent mapping", async () => {
      const { useWorkflowEditorStore } = await import("@/stores/feature/workflowEditorStore");
      const store = useWorkflowEditorStore.getState() as any;

      store.setParentRef("child-1", "parent-1");

      const state = useWorkflowEditorStore.getState() as any;
      expect(state.parentRefs).toEqual({ "child-1": "parent-1" });
      expect(state.isDirty).toBe(true);
    });

    it("setParentRef with null removes existing entry", async () => {
      const { useWorkflowEditorStore } = await import("@/stores/feature/workflowEditorStore");
      const store = useWorkflowEditorStore.getState() as any;

      store.setParentRef("child-1", "parent-1");
      store.setParentRef("child-2", "parent-1");
      store.setParentRef("child-1", null);

      const state = useWorkflowEditorStore.getState() as any;
      expect(state.parentRefs).toEqual({ "child-2": "parent-1" });
    });

    it("setParentRef with null is a no-op on missing key", async () => {
      const { useWorkflowEditorStore } = await import("@/stores/feature/workflowEditorStore");
      const store = useWorkflowEditorStore.getState() as any;

      store.setParentRef("ghost-child", null);

      const state = useWorkflowEditorStore.getState() as any;
      expect(state.parentRefs).toEqual({});
    });

    it("clearParentRefs wipes all entries", async () => {
      const { useWorkflowEditorStore } = await import("@/stores/feature/workflowEditorStore");
      const store = useWorkflowEditorStore.getState() as any;

      store.setParentRef("child-1", "parent-1");
      store.setParentRef("child-2", "parent-1");
      store.clearParentRefs();

      const state = useWorkflowEditorStore.getState() as any;
      expect(state.parentRefs).toEqual({});
    });

    it("deleteNode cascades to children of deleted container", async () => {
      const { useWorkflowEditorStore } = await import("@/stores/feature/workflowEditorStore");
      const store = useWorkflowEditorStore.getState() as any;

      const container = makeMockWorkflowNode("container-1", "parallel");
      const childA = makeMockWorkflowNode("child-a");
      const childB = makeMockWorkflowNode("child-b");
      const sibling = makeMockWorkflowNode("sibling-1");

      store.addNode(container);
      store.addNode(childA);
      store.addNode(childB);
      store.addNode(sibling);
      store.setParentRef("child-a", "container-1");
      store.setParentRef("child-b", "container-1");

      store.deleteNode("container-1");

      const state = useWorkflowEditorStore.getState() as any;
      expect(state.nodes.map((n: any) => n.id)).toEqual(["sibling-1"]);
      expect(state.parentRefs).toEqual({});
    });

    it("deleteNode preserves parentRefs for unrelated subtrees", async () => {
      const { useWorkflowEditorStore } = await import("@/stores/feature/workflowEditorStore");
      const store = useWorkflowEditorStore.getState() as any;

      const containerA = makeMockWorkflowNode("container-a", "parallel");
      const containerB = makeMockWorkflowNode("container-b", "parallel");
      const childA = makeMockWorkflowNode("child-a");
      const childB = makeMockWorkflowNode("child-b");

      store.addNode(containerA);
      store.addNode(containerB);
      store.addNode(childA);
      store.addNode(childB);
      store.setParentRef("child-a", "container-a");
      store.setParentRef("child-b", "container-b");

      store.deleteNode("container-a");

      const state = useWorkflowEditorStore.getState() as any;
      expect(state.nodes.map((n: any) => n.id).sort()).toEqual(["child-b", "container-b"]);
      expect(state.parentRefs).toEqual({ "child-b": "container-b" });
    });

    it("deleteNode cleans parentRefs where deleted node was the parent", async () => {
      const { useWorkflowEditorStore } = await import("@/stores/feature/workflowEditorStore");
      const store = useWorkflowEditorStore.getState() as any;

      // 模拟"节点被删但 parentRefs 仍残留旧记录"的脏数据场景
      store.setNodes([makeMockWorkflowNode("ghost-parent")]);
      store.setParentRef("ghost-parent", null); // 仍可能残留，先手动写入绕过
      // 手动注入孤儿登记
      useWorkflowEditorStore.setState({
        parentRefs: { "child-x": "ghost-parent", "child-y": "real-parent" },
        nodes: [makeMockWorkflowNode("ghost-parent"), makeMockWorkflowNode("real-parent")],
      });

      store.deleteNode("ghost-parent");

      const state = useWorkflowEditorStore.getState() as any;
      expect(state.parentRefs).toEqual({ "child-y": "real-parent" });
    });

    it("loadTemplate rebuilds parentRefs from nodes[].parentId", async () => {
      const childNode = {
        ...makeMockWorkflowNode("child-1"),
        parentId: "parent-1",
      } as WorkflowNode;
      const mockTemplate = makeMockTemplate("template-1", {
        nodes: [makeMockWorkflowNode("parent-1", "parallel"), childNode],
        edges: [],
      });
      invokeMock.mockResolvedValueOnce(mockTemplate);

      const { useWorkflowEditorStore } = await import("@/stores/feature/workflowEditorStore");
      await useWorkflowEditorStore.getState().loadTemplate("template-1");

      const state = useWorkflowEditorStore.getState() as any;
      expect(state.parentRefs).toEqual({ "child-1": "parent-1" });
    });

    it("loadTemplate with no parentId leaves parentRefs empty", async () => {
      const mockTemplate = makeMockTemplate("template-1", {
        nodes: [makeMockWorkflowNode("node-1")],
        edges: [],
      });
      invokeMock.mockResolvedValueOnce(mockTemplate);

      const { useWorkflowEditorStore } = await import("@/stores/feature/workflowEditorStore");
      await useWorkflowEditorStore.getState().loadTemplate("template-1");

      const state = useWorkflowEditorStore.getState() as any;
      expect(state.parentRefs).toEqual({});
    });
  });

  describe("Container Collapse", () => {
    it("should start with an empty collapsed set", async () => {
      const { useWorkflowEditorStore } = await import("@/stores/feature/workflowEditorStore");
      const state = useWorkflowEditorStore.getState() as any;
      expect(state.collapsedContainers).toBeInstanceOf(Set);
      expect(state.collapsedContainers.size).toBe(0);
    });

    it("should add a parallel id when toggled on", async () => {
      const { useWorkflowEditorStore } = await import("@/stores/feature/workflowEditorStore");
      const store = useWorkflowEditorStore.getState() as any;
      store.toggleContainerCollapse("parallel-1");
      const state = useWorkflowEditorStore.getState() as any;
      expect(state.collapsedContainers.has("parallel-1")).toBe(true);
    });

    it("should remove a parallel id when toggled off", async () => {
      const { useWorkflowEditorStore } = await import("@/stores/feature/workflowEditorStore");
      const store = useWorkflowEditorStore.getState() as any;
      store.toggleContainerCollapse("parallel-1");
      store.toggleContainerCollapse("parallel-1");
      const state = useWorkflowEditorStore.getState() as any;
      expect(state.collapsedContainers.has("parallel-1")).toBe(false);
      expect(state.collapsedContainers.size).toBe(0);
    });

    it("should track multiple collapsed parallels independently", async () => {
      const { useWorkflowEditorStore } = await import("@/stores/feature/workflowEditorStore");
      const store = useWorkflowEditorStore.getState() as any;
      store.toggleContainerCollapse("parallel-1");
      store.toggleContainerCollapse("parallel-2");
      const state = useWorkflowEditorStore.getState() as any;
      expect(state.collapsedContainers.has("parallel-1")).toBe(true);
      expect(state.collapsedContainers.has("parallel-2")).toBe(true);
      expect(state.collapsedContainers.size).toBe(2);
    });

    it("should clean up collapse state when a node is deleted", async () => {
      const { useWorkflowEditorStore } = await import("@/stores/feature/workflowEditorStore");
      const store = useWorkflowEditorStore.getState() as any;
      store.toggleContainerCollapse("parallel-1");
      store.toggleContainerCollapse("parallel-2");
      expect(
        (useWorkflowEditorStore.getState() as any).collapsedContainers.size,
      ).toBe(2);
      // Set up a template with parallel-1 and a child, so delete cascades
      const tpl: WorkflowTemplateResponse = makeMockTemplate("template-1", {
        name: "t",
        nodes: [
          makeMockWorkflowNode("parallel-1", "parallel"),
          makeMockWorkflowNode("child-1"),
        ],
        edges: [],
      });
      invokeMock.mockResolvedValueOnce(tpl);
      await store.loadTemplate("template-1");
      await store.deleteNode("parallel-1");
      const state = useWorkflowEditorStore.getState() as any;
      expect(state.collapsedContainers.has("parallel-1")).toBe(false);
      expect(state.collapsedContainers.has("parallel-2")).toBe(true);
    });
  });

  describe("Batch Operations", () => {
    it("should set multiple nodes at once", async () => {
      const { useWorkflowEditorStore } = await import("@/stores/feature/workflowEditorStore");
      const store = useWorkflowEditorStore.getState() as any;

      const nodes = [
        makeMockWorkflowNode("node-1"),
        makeMockWorkflowNode("node-2"),
      ];
      store.setNodes(nodes);

      const state = useWorkflowEditorStore.getState() as any;
      expect(state.nodes).toEqual(nodes);
    });

    it("should set multiple edges at once", async () => {
      const { useWorkflowEditorStore } = await import("@/stores/feature/workflowEditorStore");
      const store = useWorkflowEditorStore.getState() as any;

      const edges = [
        makeMockWorkflowEdge("edge-1", "node-1", "node-2"),
        makeMockWorkflowEdge("edge-2", "node-2", "node-3"),
      ];
      store.setEdges(edges);

      const state = useWorkflowEditorStore.getState() as any;
      expect(state.edges).toEqual(edges);
    });
  });

  describe("Diagnose Fix - #6.1 remove_debater_step", () => {
    function makeDebateNode(id: string, stepIds: string[], subNodeIds: string[] = []) {
      const subNodes = subNodeIds.map((sid) => ({
        id: sid,
        type: "agent" as const,
        title: sid,
        description: "",
        position: { x: 0, y: 0 },
        config: { agent_name: "a", system_prompt: "", output_var: "x" },
        retry: { enabled: false, max_retries: 0, backoff_type: "Fixed" as const, base_delay_ms: 0, max_delay_ms: 0 },
        enabled: true,
      }));
      return {
        id,
        type: "debate" as const,
        title: "Debate",
        description: "",
        position: { x: 0, y: 0 },
        config: {
          debater_steps: stepIds,
          max_rounds: 3,
          topic_var: "topic",
          output_var: "result",
          subGraph: { nodes: subNodes, edges: [] },
        },
        retry: { enabled: false, max_retries: 0, backoff_type: "Fixed" as const, base_delay_ms: 0, max_delay_ms: 0 },
        enabled: true,
      };
    }

    async function seedReport(issueId: string, fix: any) {
      const { useWorkflowEditorStore } = await import("@/stores/feature/workflowEditorStore");
      useWorkflowEditorStore.setState({
        diagnoseReport: {
          issues: [{
            id: issueId,
            severity: "warning",
            category: "structure",
            title_key: "debate_dangling_step",
            message_key: "debate_dangling_step",
            title_override: "Test issue",
            detail_override: "Test",
            suggestion_override: "Test",
            auto_fixable: true,
            fix,
            node_ids: ["d-1"],
          }],
          summary: { error: 0, warning: 1, info: 0 },
          generated_at: 0,
          duration_ms: 0,
        },
      });
    }

    it("removes step from debater_steps and drops subGraph node + its edges", async () => {
      const { useWorkflowEditorStore } = await import("@/stores/feature/workflowEditorStore");
      const store = useWorkflowEditorStore.getState() as any;
      const debate = makeDebateNode("d-1", ["s-1", "s-2"], ["s-1", "s-2"]);
      store.addNode(debate);
      store.addEdge({ id: "e-1", source: "d-1.s-1", target: "d-1.s-2", edge_type: "direct" } as any);
      store.addEdge({ id: "e-2", source: "d-1.s-1", target: "d-1.s-2", edge_type: "direct" } as any);

      // 在 d-1 的 subGraph 内补一条边让 e-2 出现在 subGraph.edges
      const state = useWorkflowEditorStore.getState() as any;
      const updatedDebate = state.nodes.find((n: any) => n.id === "d-1");
      updatedDebate.config.subGraph.edges = [{ id: "e-2", source: "s-1", target: "s-2", edge_type: "direct" }];
      state.nodes = [...state.nodes];

      await seedReport("issue-1", { action_type: "remove_debater_step", node_id: "d-1", step_id: "s-2" });

      const result = store.applyDiagnoseFix("issue-1");
      expect(result).toBe(true);

      const after = useWorkflowEditorStore.getState() as any;
      const node = after.nodes.find((n: any) => n.id === "d-1");
      expect(node.config.debater_steps).toEqual(["s-1"]);
      expect(node.config.subGraph.nodes.map((n: any) => n.id)).toEqual(["s-1"]);
      expect(node.config.subGraph.edges).toEqual([]);
    });

    it("returns false when the debate node does not exist", async () => {
      const { useWorkflowEditorStore } = await import("@/stores/feature/workflowEditorStore");
      const store = useWorkflowEditorStore.getState() as any;
      await seedReport("issue-2", { action_type: "remove_debater_step", node_id: "missing", step_id: "s-1" });
      expect(store.applyDiagnoseFix("issue-2")).toBe(false);
    });

    it("returns false when step_id is not in debater_steps", async () => {
      const { useWorkflowEditorStore } = await import("@/stores/feature/workflowEditorStore");
      const store = useWorkflowEditorStore.getState() as any;
      store.addNode(makeDebateNode("d-1", ["s-1"], []));
      await seedReport("issue-3", { action_type: "remove_debater_step", node_id: "d-1", step_id: "s-99" });
      expect(store.applyDiagnoseFix("issue-3")).toBe(false);
    });
  });

  describe("Diagnose Applying state - #6.13", () => {
    it("toggles diagnoseApplying around applyDiagnoseFix and resets on success", async () => {
      const { useWorkflowEditorStore } = await import("@/stores/feature/workflowEditorStore");
      const store = useWorkflowEditorStore.getState() as any;
      store.addNode(makeMockWorkflowNode("n-1"));
      useWorkflowEditorStore.setState({
        diagnoseReport: {
          issues: [{
            id: "i-1",
            severity: "warning",
            category: "structure",
            title_key: "r",
            message_key: "r",
            title_override: "t",
            detail_override: "d",
            suggestion_override: "s",
            auto_fixable: true,
            fix: { action_type: "delete_node", node_id: "n-1" },
            node_ids: ["n-1"],
          }],
          summary: { error: 0, warning: 1, info: 0 },
          generated_at: 0,
          duration_ms: 0,
        },
      });

      expect((useWorkflowEditorStore.getState() as any).diagnoseApplying).toBe(false);
      const ok = store.applyDiagnoseFix("i-1");
      expect(ok).toBe(true);
      expect((useWorkflowEditorStore.getState() as any).diagnoseApplying).toBe(false);
    });
  });
});

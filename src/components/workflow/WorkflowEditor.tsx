import React, { useCallback, useEffect, useMemo, useState } from "react";
import {
  Background,
  Connection,
  Controls,
  type Edge,
  MiniMap,
  type Node,
  Panel,
  ReactFlow,
  useEdgesState,
  useNodesState,
  useReactFlow,
} from "reactflow";
import "reactflow/dist/style.css";
import { useWorkflowEditorStore, SimilarWorkflow } from "@/stores";
import { Button, message, Modal, Spin } from "antd";
import { useTranslation } from "react-i18next";
import { BaseEdge } from "./Edges/BaseEdge";
import { EditorHeader } from "./Header/EditorHeader";
import { AIPanel } from "./AIPanel/AIPanel";
import {
  AgentNode,
  AtomicSkillNode,
  BaseNode,
  CodeNode,
  ConditionNode,
  DelayNode,
  DocumentParserNode,
  EndNode,
  LLMNode,
  LoopNode,
  MergeNode,
  ParallelNode,
  SubWorkflowNode,
  ToolNode,
  TriggerNode,
  VectorRetrieveNode,
  ValidationNode,
} from "./Nodes";
import { LeftPanel } from "./Panels/LeftPanel";
import { RightPanel } from "./Panels/RightPanel";
import { SkillUpgradeModal } from "./SkillUpgradeModal";
import { StatusBar } from "./StatusBar/EditorStatusBar";
import { DebugPanel } from "./DebugPanel";
import { ImportExportModal } from "./Templates/ImportExportModal";
import { type AtomicSkillInfo, NODE_TYPE_MAP, type WorkflowEdge, type WorkflowNode } from "./types";
import { getDragPayload, clearDragPayload } from "./dndState";

const nodeTypes = {
  base: BaseNode,
  trigger: TriggerNode,
  agent: AgentNode,
  llm: LLMNode,
  condition: ConditionNode,
  parallel: ParallelNode,
  loop: LoopNode,
  merge: MergeNode,
  delay: DelayNode,
  tool: ToolNode,
  code: CodeNode,
  atomicSkill: AtomicSkillNode,
  subWorkflow: SubWorkflowNode,
  documentParser: DocumentParserNode,
  vectorRetrieve: VectorRetrieveNode,
  validation: ValidationNode,
  end: EndNode,
};

const edgeTypes = {
  base: BaseEdge,
};

const defaultEdgeOptions = {
  type: "base",
  animated: false,
};

interface WorkflowEditorProps {
  templateId?: string;
  onClose?: () => void;
}

export const WorkflowEditor: React.FC<WorkflowEditorProps> = ({ templateId, onClose }) => {
  const { t } = useTranslation("chat");
  const {
    currentTemplate,
    nodes,
    edges,
    isLoading,
    isSaving,
    isDirty,
    validationResult,
    loadTemplate,
    initNewTemplate,
    updateNode,
    deleteNode,
    deleteEdge,
    addEdge: storeAddEdge,
    setSelectedNode,
    setSelectedEdge,
    selectedNodeId,
    selectedEdgeId,
    updateTemplate,
    updateTemplateMetadata,
    createTemplate,
    validateTemplate,
    error,
    undo,
    redo,
    addNode,
  } = useWorkflowEditorStore();

  const [reactFlowNodes, setRNodes, onNodesChange] = useNodesState([]);
  const [reactFlowEdges, setREdges, onEdgesChange] = useEdgesState([]);
  const [isInitialized, setIsInitialized] = React.useState(false);
  const clipboardRef = React.useRef<WorkflowNode[]>([]);
  const [upgradeModalState, setUpgradeModalState] = React.useState<{
    visible: boolean;
    existingSkill: AtomicSkillInfo | null;
    generatedSkillName: string;
    generatedSkillDescription: string;
    nodeId: string;
  }>({
    visible: false,
    existingSkill: null,
    generatedSkillName: "",
    generatedSkillDescription: "",
    nodeId: "",
  });

  const [similarWorkflowsModal, setSimilarWorkflowsModal] = useState<{
    visible: boolean;
    workflows: SimilarWorkflow[];
    pendingWorkflowName: string;
    pendingWorkflowDescription: string;
  }>({
    visible: false,
    workflows: [],
    pendingWorkflowName: "",
    pendingWorkflowDescription: "",
  });

  const [aiPanelVisible, setAiPanelVisible] = useState(false);
  const [debugPanelVisible, setDebugPanelVisible] = useState(false);
  const [importExportModalVisible, setImportExportModalVisible] = useState(false);

  const {
    isDecompositionTemplate,
    checkSkillSemanticMatches,
    similarWorkflowsForReview,
    pendingWorkflowData,
    forceSaveSkillWorkflow,
    clearSimilarWorkflowsForReview,
    saveSkillWorkflowFromLlm,
    semanticCheckResult,
    applySemanticAction,
    generateWorkflowFromPrompt,
    optimizeAgentPrompt,
    recommendNodes,
    exportTemplate,
    importTemplate,
  } = useWorkflowEditorStore();

  useEffect(() => {
    if (templateId) {
      loadTemplate(templateId);
    } else {
      initNewTemplate();
    }
  }, [templateId]);

  useEffect(() => {
    if (isInitialized && isDecompositionTemplate && nodes.length > 0) {
      checkSkillSemanticMatches(nodes);
    }
  }, [isInitialized, isDecompositionTemplate, nodes, checkSkillSemanticMatches]);

  useEffect(() => {
    if (similarWorkflowsForReview.length > 0 && pendingWorkflowData) {
      setSimilarWorkflowsModal({
        visible: true,
        workflows: similarWorkflowsForReview,
        pendingWorkflowName: pendingWorkflowData.workflowName,
        pendingWorkflowDescription: pendingWorkflowData.workflowDescription || "",
      });
    }
  }, [similarWorkflowsForReview, pendingWorkflowData]);

  useEffect(() => {
    if (currentTemplate) {
      const flowNodes: Node[] = nodes.map((node: WorkflowNode) => {
        const typeInfo = NODE_TYPE_MAP[node.type] || { label: node.type, color: "#999" };
        const nodeType = NODE_TYPE_MAP[node.type] ? node.type : "base";

        let semanticMatch = undefined;
        if (semanticCheckResult?.matches && node.type === "atomicSkill") {
          const match = semanticCheckResult.matches.find((m) => m.node_id === node.id);
          if (match?.matches && match.matches.length > 0) {
            const bestMatch = match.matches[0];
            semanticMatch = {
              existing_skill_id: bestMatch.existing_skill.id,
              existing_skill_name: bestMatch.existing_skill.name,
              similarity_score: bestMatch.similarity_score,
              match_reasons: bestMatch.match_reasons,
            };
          }
        }

        return {
          id: node.id,
          type: nodeType,
          position: node.position,
          data: {
            ...node,
            label: node.title,
            color: typeInfo.color,
            nodeType: node.type,
            semanticMatch,
            onSemanticAction: applySemanticAction,
            onUpgradeRequest: handleUpgradeRequest,
          },
        };
      });
      setRNodes(flowNodes);

      const flowEdges: Edge[] = edges.map((edge: WorkflowEdge) => ({
        id: edge.id,
        source: edge.source,
        sourceHandle: edge.sourceHandle,
        target: edge.target,
        targetHandle: edge.targetHandle,
        type: "base",
        animated: edge.edge_type === "loopBack",
        label: edge.label,
        data: { edgeType: edge.edge_type },
      }));
      setREdges(flowEdges);
      setIsInitialized(true);
    }
  }, [currentTemplate, nodes, edges]);

  const handleUpgradeRequest = useCallback(
    (
      nodeId: string,
      existingSkillId: string,
      _existingSkillName: string,
      generatedSkillName: string,
      generatedSkillDescription: string,
    ) => {
      if (!semanticCheckResult) { return; }

      const match = semanticCheckResult.matches.find((m) => m.node_id === nodeId);
      if (!match || !match.matches || match.matches.length === 0) { return; }

      const bestMatch = match.matches.find((m) => m.existing_skill.id === existingSkillId);
      if (!bestMatch) { return; }

      setUpgradeModalState({
        visible: true,
        existingSkill: bestMatch.existing_skill,
        generatedSkillName,
        generatedSkillDescription,
        nodeId,
      });
    },
    [semanticCheckResult],
  );

  const handleUpgradeConfirm = useCallback(
    (
      suggestion: {
        name: string;
        description: string;
        input_schema: Record<string, unknown> | null;
        output_schema: Record<string, unknown> | null;
        reasoning: string;
      },
    ) => {
      const { nodeId, existingSkill } = upgradeModalState;
      if (!existingSkill) { return; }

      useWorkflowEditorStore.setState((state) => {
        const nodeIndex = state.nodes.findIndex((n) => n.id === nodeId);
        if (nodeIndex !== -1) {
          const node = state.nodes[nodeIndex] as any;
          node.config.skill_id = existingSkill.id;
          node.config.skill_name = suggestion.name;
          node.config.entry_ref = existingSkill.entry_ref;
          node.config.entry_type = existingSkill.entry_type;
          node.config.category = existingSkill.category;
          node.title = suggestion.name;
          node.description = suggestion.description;
          node.data.skillId = existingSkill.id;
          node.data.skillName = suggestion.name;
        }

        state.pendingReplacements.set(nodeId, {
          existingSkillId: existingSkill.id,
          action: "upgrade_existing",
        });

        const remainingMatches = state.semanticCheckResult?.matches.filter((m) => m.node_id !== nodeId) || [];
        if (remainingMatches.length === 0) {
          state.semanticCheckResult = null;
        }
      });

      setUpgradeModalState((prev) => ({ ...prev, visible: false }));
      message.success(t("workflow.semanticCheckApplied"));
    },
    [upgradeModalState, t],
  );

  const onConnect = useCallback(
    (params: Connection) => {
      if (params.source && params.target) {
        // Determine edge type based on sourceHandle
        let edgeType: WorkflowEdge["edge_type"] = "direct";
        const sourceHandle = params.sourceHandle;
        if (sourceHandle === "true") {
          edgeType = "conditionTrue";
        } else if (sourceHandle === "false") {
          edgeType = "conditionFalse";
        } else if (sourceHandle === "loopBack") {
          edgeType = "loopBack";
        } else if (sourceHandle?.startsWith("branch-")) {
          edgeType = "parallelBranch";
        } else if (sourceHandle === "fail") {
          edgeType = "error";
        }

        const newEdge: WorkflowEdge = {
          id: `edge-${Date.now()}`,
          source: params.source,
          sourceHandle: sourceHandle ?? undefined,
          target: params.target,
          targetHandle: params.targetHandle ?? undefined,
          edge_type: edgeType,
        };
        storeAddEdge(newEdge);
      }
    },
    [storeAddEdge],
  );

  const onNodeClick = useCallback(
    (_: React.MouseEvent, node: Node) => {
      setSelectedNode(node.id);
    },
    [setSelectedNode],
  );

  const onEdgeClick = useCallback(
    (_: React.MouseEvent, edge: Edge) => {
      setSelectedEdge(edge.id);
    },
    [setSelectedEdge],
  );

  const onPaneClick = useCallback(() => {
    setSelectedNode(null);
    setSelectedEdge(null);
  }, [setSelectedNode, setSelectedEdge]);

  const reactFlowInstance = useReactFlow();

  // Custom DnD: handle mouse-up on the canvas to place a node.
  // We listen on the window so the drop works even if the cursor
  // is slightly outside the ReactFlow pane.
  useEffect(() => {
    const handleGlobalMouseUp = (e: MouseEvent) => {
      const payload = getDragPayload();
      if (!payload) return;

      try {
        const typeInfo = NODE_TYPE_MAP[payload.type] || { label: payload.type, color: "#999" };

        // Check if the mouse is within the canvas area
        const canvasEl = document.querySelector(".react-flow");
        if (!canvasEl) return;

        const rect = canvasEl.getBoundingClientRect();
        if (
          e.clientX < rect.left ||
          e.clientX > rect.right ||
          e.clientY < rect.top ||
          e.clientY > rect.bottom
        ) {
          return;
        }

        const position = reactFlowInstance.screenToFlowPosition({
          x: e.clientX,
          y: e.clientY,
        });

        const id = `node-${Date.now()}-${Math.random().toString(36).slice(2, 9)}`;
        const actualNodeType = NODE_TYPE_MAP[payload.type] ? payload.type : "base";

        const newNode: Node = {
          id,
          type: actualNodeType,
          position,
          data: {
            id,
            type: payload.type,
            title: t("workflow.newNode", { type: typeInfo.label }),
            description: "",
            color: typeInfo.color,
            nodeType: payload.type,
            enabled: true,
            ...getDefaultNodeConfig(payload.type),
          },
        };

        setRNodes((nds) => [...nds, newNode]);

        const workflowNode = createWorkflowNode(id, payload.type, position, t("workflow.newNode", { type: typeInfo.label }));
        useWorkflowEditorStore.getState().addNode(workflowNode);
      } catch (error) {
        console.error("Failed to drop node:", error);
      } finally {
        clearDragPayload();
      }
    };

    window.addEventListener("mouseup", handleGlobalMouseUp);
    return () => window.removeEventListener("mouseup", handleGlobalMouseUp);
  }, [reactFlowInstance, setRNodes]);

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      const isCtrlOrCmd = e.ctrlKey || e.metaKey;

      if (isCtrlOrCmd && e.key === "z" && !e.shiftKey) {
        e.preventDefault();
        undo();
        return;
      }

      if (isCtrlOrCmd && e.key === "z" && e.shiftKey) {
        e.preventDefault();
        redo();
        return;
      }

      if (isCtrlOrCmd && e.key === "y") {
        e.preventDefault();
        redo();
        return;
      }

      if ((e.key === "Delete" || e.key === "Backspace") && selectedNodeId) {
        const target = e.target as HTMLElement;
        if (target.tagName === "INPUT" || target.tagName === "TEXTAREA" || target.isContentEditable) {
          return;
        }
        e.preventDefault();
        deleteNode(selectedNodeId);
        setSelectedNode(null);
        return;
      }

      if (isCtrlOrCmd && e.key === "c" && selectedNodeId) {
        const nodeToCopy = nodes.find((n) => n.id === selectedNodeId);
        if (nodeToCopy) {
          clipboardRef.current = [nodeToCopy];
          message.success(t("workflow.nodeCopied"));
        }
        return;
      }

      if (isCtrlOrCmd && e.key === "v") {
        if (clipboardRef.current.length === 0) return;
        const newNodes: WorkflowNode[] = [];
        const offset = { x: 50, y: 50 };

        clipboardRef.current.forEach((node) => {
          const id = `node-${Date.now()}-${Math.random().toString(36).slice(2, 9)}`;
          const newNode: WorkflowNode = {
            ...node,
            id,
            position: {
              x: node.position.x + offset.x,
              y: node.position.y + offset.y,
            },
          };
          newNodes.push(newNode);
          addNode(newNode);
        });
        if (newNodes.length > 0) {
          message.success(t("workflow.nodesPasted", { count: newNodes.length }));
        }
        return;
      }

      if (isCtrlOrCmd && e.key === "a") {
        const target = e.target as HTMLElement;
        if (target.tagName === "INPUT" || target.tagName === "TEXTAREA" || target.isContentEditable) {
          return;
        }
        e.preventDefault();
        return;
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [undo, redo, selectedNodeId, deleteNode, setSelectedNode, nodes, addNode]);

  const handleNodesChange = useCallback(
    (changes: any) => {
      onNodesChange(changes);

      changes.forEach((change: any) => {
        if (change.type === "position" && change.position && currentTemplate) {
          const nodeId = change.id;
          const position = change.position;
          updateNode(nodeId, { position } as Partial<WorkflowNode>);
        }
        if (change.type === "remove" && change.id) {
          deleteNode(change.id);
        }
      });
    },
    [onNodesChange, currentTemplate, updateNode, deleteNode],
  );

  const handleEdgesChange = useCallback(
    (changes: any) => {
      onEdgesChange(changes);

      changes.forEach((change: any) => {
        if (change.type === "remove" && change.id) {
          deleteEdge(change.id);
        }
      });
    },
    [onEdgesChange, deleteEdge],
  );

  const handleSave = useCallback(async () => {
    if (!currentTemplate) { return; }

    if (isDecompositionTemplate) {
      try {
        await saveSkillWorkflowFromLlm(currentTemplate.name, currentTemplate.description);
        message.success(t("workflow.decompositionSaved"));
        onClose?.();
      } catch (e) {
        message.error(String(e));
      }
      return;
    }

    const validation = await validateTemplate();
    if (validation && !validation.is_valid) {
      message.error(t("workflow.validationFailed", { count: validation.errors.length }));
      return;
    }

    const input = {
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

    if (currentTemplate.id) {
      await updateTemplate(currentTemplate.id, input);
    } else {
      const newId = await createTemplate(input);
      if (newId) {
        await loadTemplate(newId);
        message.success(t("workflow.saved"));
      }
    }
  }, [currentTemplate, nodes, edges, createTemplate, updateTemplate, validateTemplate, t, onClose]);

  const handleNameChange = useCallback(
    (name: string) => {
      updateTemplateMetadata({ name });
    },
    [updateTemplateMetadata],
  );

  const selectedNode = useMemo(() => {
    if (!selectedNodeId) { return null; }
    return nodes.find((n) => n.id === selectedNodeId) || null;
  }, [selectedNodeId, nodes]);

  const selectedEdge = useMemo(() => {
    if (!selectedEdgeId) { return null; }
    return edges.find((e) => e.id === selectedEdgeId) || null;
  }, [selectedEdgeId, edges]);

  if (isLoading) {
    return (
      <div style={{ display: "flex", alignItems: "center", justifyContent: "center", height: "100%" }}>
        <Spin size="large" />
      </div>
    );
  }

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%", background: "#1a1a1a" }}>
      <EditorHeader
        templateName={currentTemplate?.name || t("workflow.newWorkflow")}
        isDirty={isDirty}
        isSaving={isSaving}
        onSave={handleSave}
        onNameChange={handleNameChange}
        onClose={onClose}
        onToggleAIPanel={() => setAiPanelVisible(!aiPanelVisible)}
        onToggleDebugPanel={() => setDebugPanelVisible(!debugPanelVisible)}
        onOpenImportExport={() => setImportExportModalVisible(true)}
        aiPanelVisible={aiPanelVisible}
        debugPanelVisible={debugPanelVisible}
      />

      <div style={{ display: "flex", flex: 1, overflow: "hidden" }}>
        <LeftPanel />

        <div style={{ flex: 1, position: "relative" }}>
          {isInitialized ? (
            <ReactFlow
              nodes={reactFlowNodes}
              edges={reactFlowEdges}
              onNodesChange={handleNodesChange}
              onEdgesChange={handleEdgesChange}
              onConnect={onConnect}
              onNodeClick={onNodeClick}
              onEdgeClick={onEdgeClick}
              onPaneClick={onPaneClick}
              nodeTypes={nodeTypes}
              edgeTypes={edgeTypes}
              defaultEdgeOptions={defaultEdgeOptions}
              fitView
              snapToGrid
              snapGrid={[16, 16]}
            >
              <Background color="#333" gap={16} />
              <Controls />
              <MiniMap
                nodeColor={(node: Node) => (node.data as any)?.color || "#999"}
                maskColor="rgba(0, 0, 0, 0.8)"
              />
              {nodes.length === 0 && (
                <Panel position="top-center" style={{ textAlign: "center", color: "#666" }}>
                  {t("workflow.dragToStart")}
                </Panel>
              )}
            </ReactFlow>
          ) : (
            <div
              className="react-flow"
              style={{
                width: "100%",
                height: "100%",
                display: "flex",
                alignItems: "center",
                justifyContent: "center",
                background: "#1a1a1a",
                color: "#666",
              }}
            >
              <Spin />
            </div>
          )}
        </div>

        <RightPanel selectedNode={selectedNode} selectedEdge={selectedEdge} />
      </div>

      {aiPanelVisible && (
        <div
          style={{
            height: 300,
            background: "#252525",
            borderTop: "1px solid #333",
            display: "flex",
            flexDirection: "column",
          }}
        >
          <AIPanel
            onGenerateWorkflow={generateWorkflowFromPrompt}
            onOptimizePrompt={optimizeAgentPrompt}
            onRecommendNodes={recommendNodes}
            onClose={() => setAiPanelVisible(false)}
          />
        </div>
      )}

      {debugPanelVisible && (
        <div
          style={{
            height: 300,
            background: "#252525",
            borderTop: "1px solid #333",
            display: "flex",
            flexDirection: "column",
            overflow: "hidden",
          }}
        >
          <DebugPanel />
        </div>
      )}

      <StatusBar
        nodeCount={nodes.length}
        edgeCount={edges.length}
        validationResult={validationResult}
        isDirty={isDirty}
      />

      {error && (
        <div style={{ position: "fixed", bottom: 60, left: "50%", transform: "translateX(-50%)", color: "red" }}>
          {error}
        </div>
      )}

      {upgradeModalState.visible && upgradeModalState.existingSkill && (
        <SkillUpgradeModal
          open={upgradeModalState.visible}
          onClose={() => setUpgradeModalState((prev) => ({ ...prev, visible: false }))}
          existingSkill={upgradeModalState.existingSkill}
          generatedSkillName={upgradeModalState.generatedSkillName}
          generatedSkillDescription={upgradeModalState.generatedSkillDescription}
          onConfirm={handleUpgradeConfirm}
        />
      )}

      <Modal
        title={t("workflow.similarWorkflowsFound", { count: similarWorkflowsModal.workflows.length })}
        open={similarWorkflowsModal.visible}
        onCancel={() => {
          setSimilarWorkflowsModal((prev) => ({ ...prev, visible: false }));
          clearSimilarWorkflowsForReview();
        }}
        footer={[
          <Button
            key="new"
            onClick={async () => {
              setSimilarWorkflowsModal((prev) => ({ ...prev, visible: false }));
              message.success(t("workflow.workflowSavedAsNew"));
              onClose?.();
            }}
          >
            {t("workflow.saveAsNew")}
          </Button>,
          ...similarWorkflowsModal.workflows.map((wf) => (
            <Button
              key={wf.workflow_id}
              type="primary"
              onClick={async () => {
                try {
                  await forceSaveSkillWorkflow(
                    wf.workflow_id,
                    similarWorkflowsModal.pendingWorkflowName,
                    similarWorkflowsModal.pendingWorkflowDescription,
                  );
                  message.success(t("workflow.workflowUpdated", { name: wf.name }));
                  setSimilarWorkflowsModal((prev) => ({ ...prev, visible: false }));
                  onClose?.();
                } catch (e) {
                  message.error(String(e));
                }
              }}
            >
              {t("workflow.replaceExisting", { name: wf.name, similarity: Math.round(wf.similarity * 100) })}
            </Button>
          )),
        ]}
      >
        <p>{t("workflow.similarWorkflowsExplanation")}</p>
        <ul>
          {similarWorkflowsModal.workflows.map((wf) => (
            <li key={wf.workflow_id}>
              <strong>{wf.name}</strong> - {Math.round(wf.similarity * 100)}% {t("workflow.similarity")}
              <br />
              <small>{t("workflow.skills")}: {wf.skill_ids.join(", ")}</small>
            </li>
          ))}
        </ul>
      </Modal>

      <ImportExportModal
        open={importExportModalVisible}
        onClose={() => setImportExportModalVisible(false)}
        onExport={exportTemplate}
        onImport={importTemplate}
      />
    </div>
  );
};

function getDefaultNodeConfig(nodeType: string): Record<string, unknown> {
  switch (nodeType) {
    case "trigger":
      return { type: "manual", config: {} };
    case "agent":
      return { role: "developer", system_prompt: "", tools: [], context_sources: [], output_var: "", output_mode: "text" };
    case "llm":
      return { model: "", prompt: "", temperature: 0.7, max_tokens: 2048 };
    case "condition":
      return { conditions: [], logical_op: "and" };
    case "parallel":
      return { branches: [], wait_for_all: true };
    case "loop":
      return { loop_type: "forEach", max_iterations: 100, continue_on_error: false, body_steps: [] };
    case "tool":
      return { tool_name: "", input_mapping: {}, output_var: "" };
    case "code":
      return { language: "javascript", code: "", output_var: "" };
    case "atomicSkill":
      return { skill_id: "", skill_name: "", entry_type: "builtin", input_mapping: {}, output_var: "" };
    case "end":
      return { output_var: "" };
    case "validation":
      return { assertions: [], on_fail: "stop" as const, max_retries: 0 };
    default:
      return {};
  }
}

function createWorkflowNode(id: string, type: string, position: { x: number; y: number }, title: string): WorkflowNode {
  const baseNode = {
    id,
    title,
    description: "",
    position,
    retry: {
      enabled: false,
      max_retries: 3,
      backoff_type: "Exponential" as const,
      base_delay_ms: 1000,
      max_delay_ms: 60000,
    },
    timeout: undefined,
    enabled: true,
  };

  switch (type) {
    case "trigger":
      return { ...baseNode, type: "trigger", config: { type: "manual", config: {} } };
    case "agent":
      return {
        ...baseNode,
        type: "agent",
        config: {
          role: "developer",
          system_prompt: "",
          context_sources: [],
          output_var: "",
          tools: [],
          output_mode: "text",
        },
      };
    case "llm":
      return { ...baseNode, type: "llm", config: { model: "", prompt: "", temperature: 0.7, max_tokens: 2048 } };
    case "condition":
      return { ...baseNode, type: "condition", config: { conditions: [], logical_op: "and" } };
    case "parallel":
      return { ...baseNode, type: "parallel", config: { branches: [], wait_for_all: true } };
    case "loop":
      return {
        ...baseNode,
        type: "loop",
        config: { loop_type: "forEach", max_iterations: 100, continue_on_error: false, body_steps: [] },
      };
    case "merge":
      return { ...baseNode, type: "merge", config: { merge_type: "all", inputs: [] } };
    case "delay":
      return { ...baseNode, type: "delay", config: { delay_type: "seconds", seconds: 5 } };
    case "tool":
      return { ...baseNode, type: "tool", config: { tool_name: "", input_mapping: {}, output_var: "" } };
    case "code":
      return { ...baseNode, type: "code", config: { language: "javascript", code: "", output_var: "" } };
    case "subWorkflow":
      return {
        ...baseNode,
        type: "subWorkflow",
        config: { sub_workflow_id: "", input_mapping: {}, output_var: "", is_async: false },
      };
    case "documentParser":
      return { ...baseNode, type: "documentParser", config: { input_var: "", parser_type: "", output_var: "" } };
    case "vectorRetrieve":
      return {
        ...baseNode,
        type: "vectorRetrieve",
        config: { query: "", knowledge_base_id: "", top_k: 5, output_var: "" },
      };
    case "atomicSkill":
      return {
        ...baseNode,
        type: "atomicSkill",
        config: { skill_id: "", skill_name: "", entry_type: "builtin", input_mapping: {}, output_var: "" },
      };
    case "end":
      return { ...baseNode, type: "end", config: {} };
    case "validation":
      return {
        ...baseNode,
        type: "validation",
        config: { assertions: [], on_fail: "stop" as const, max_retries: 0 },
      };
    default:
      return {
        ...baseNode,
        type: "agent",
        config: {
          role: "developer",
          system_prompt: "",
          context_sources: [],
          output_var: "",
          tools: [],
          output_mode: "text",
        },
      };
  }
}

export default WorkflowEditor;
